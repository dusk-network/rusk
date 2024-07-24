// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::{mpsc, Arc, LazyLock};
use std::time::{Duration, Instant};
use std::{fs, io};

use parking_lot::RwLock;
use sha3::{Digest, Sha3_256};
use tokio::task;
use tracing::{debug, info, warn};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_consensus::config::{
    ratification_committee_quorum, ratification_extra,
    validation_committee_quorum, validation_extra,
    RATIFICATION_COMMITTEE_CREDITS, VALIDATION_COMMITTEE_CREDITS,
};
use dusk_consensus::operations::{
    CallParams, VerificationOutput, VoterWithCredits,
};
use execution_core::bytecode::Bytecode;
use execution_core::transfer::ContractDeploy;
use execution_core::{
    stake::StakeData,
    transfer::{AccountData, Transaction as ProtocolTransaction},
    BlsPublicKey, BlsScalar,
};
use node_data::ledger::{Slash, SpentTransaction, Transaction};
use rusk_abi::dusk::Dusk;
use rusk_abi::ContractError::{OutOfGas, Panic};
use rusk_abi::{
    CallReceipt, ContractError, Event, PiecrustError, Session, STAKE_CONTRACT,
    TRANSFER_CONTRACT, VM,
};
use rusk_profile::to_rusk_state_id_path;
use tokio::sync::broadcast;

use super::{coinbase_value, Rusk, RuskTip};
use crate::gen_id::gen_contract_id;
use crate::http::RuesEvent;
use crate::Error::InvalidCreditsCount;
use crate::{Error, Result};

pub static DUSK_KEY: LazyLock<BlsPublicKey> = LazyLock::new(|| {
    let dusk_cpk_bytes = include_bytes!("../../assets/dusk.cpk");
    BlsPublicKey::from_slice(dusk_cpk_bytes)
        .expect("Dusk consensus public key to be valid")
});

const DEFAULT_GAS_PER_DEPLOY_BYTE: u64 = 100;

impl Rusk {
    pub fn new<P: AsRef<Path>>(
        dir: P,
        generation_timeout: Option<Duration>,
        gas_per_deploy_byte: Option<u64>,
        feeder_gas_limit: u64,
        event_sender: broadcast::Sender<RuesEvent>,
    ) -> Result<Self> {
        let dir = dir.as_ref();
        let commit_id_path = to_rusk_state_id_path(dir);

        let base_commit_bytes = fs::read(commit_id_path)?;
        if base_commit_bytes.len() != 32 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Expected commit id to have 32 bytes, got {}",
                    base_commit_bytes.len()
                ),
            )
            .into());
        }
        let mut base_commit = [0u8; 32];
        base_commit.copy_from_slice(&base_commit_bytes);

        let vm = Arc::new(rusk_abi::new_vm(dir)?);

        let tip = Arc::new(RwLock::new(RuskTip {
            current: base_commit,
            base: base_commit,
        }));

        Ok(Self {
            tip,
            vm,
            dir: dir.into(),
            generation_timeout,
            gas_per_deploy_byte,
            feeder_gas_limit,
            event_sender,
        })
    }

    pub fn execute_transactions<I: Iterator<Item = Transaction>>(
        &self,
        params: &CallParams,
        txs: I,
    ) -> Result<(Vec<SpentTransaction>, Vec<Transaction>, VerificationOutput)>
    {
        let started = Instant::now();

        let block_height = params.round;
        let block_gas_limit = params.block_gas_limit;
        let generator = params.generator_pubkey.inner();
        let to_slash = params.to_slash.clone();

        let voters = params.voters_pubkey.as_ref().map(|voters| &voters[..]);

        let mut session = self.session(block_height, None)?;

        let mut block_gas_left = block_gas_limit;

        let mut spent_txs = Vec::<SpentTransaction>::new();
        let mut discarded_txs = vec![];

        let mut dusk_spent = 0;

        let mut event_hasher = Sha3_256::new();

        for unspent_tx in txs {
            if let Some(timeout) = self.generation_timeout {
                if started.elapsed() > timeout {
                    info!("execute_transactions timeout triggered {timeout:?}");
                    break;
                }
            }
            let tx_id = hex::encode(unspent_tx.id());
            if unspent_tx.inner.gas_limit() > block_gas_left {
                info!("Skipping {tx_id} due gas_limit greater than left: {block_gas_left}");
                continue;
            }

            match execute(
                &mut session,
                &unspent_tx.inner,
                self.gas_per_deploy_byte,
            ) {
                Ok(receipt) => {
                    let gas_spent = receipt.gas_spent;

                    // If the transaction went over the block gas limit we
                    // re-execute all spent transactions. We don't discard the
                    // transaction, since it is technically valid.
                    if gas_spent > block_gas_left {
                        warn!("This is not supposed to happen with conservative tx inclusion");
                        session = self.session(block_height, None)?;

                        for spent_tx in &spent_txs {
                            // We know these transactions were correctly
                            // executed before, so we don't bother checking.
                            let _ = execute(
                                &mut session,
                                &spent_tx.inner.inner,
                                self.gas_per_deploy_byte,
                            );
                        }

                        continue;
                    }

                    // We're currently ignoring the result of successful calls
                    let err = receipt.data.err().map(|e| format!("{e}"));
                    info!("Tx {tx_id} executed with {gas_spent} gas and err {err:?}");

                    update_hasher(&mut event_hasher, &receipt.events);

                    block_gas_left -= gas_spent;
                    let gas_price = unspent_tx.inner.gas_price();
                    dusk_spent += gas_spent * gas_price;
                    spent_txs.push(SpentTransaction {
                        inner: unspent_tx,
                        gas_spent,
                        block_height,
                        err,
                    });
                }
                Err(e) => {
                    info!("discard tx {tx_id} due to {e:?}");
                    // An unspendable transaction should be discarded
                    discarded_txs.push(unspent_tx);
                    continue;
                }
            }
        }

        let coinbase_events = reward_slash_and_update_root(
            &mut session,
            block_height,
            dusk_spent,
            generator,
            to_slash,
            voters,
        )?;
        update_hasher(&mut event_hasher, &coinbase_events);

        let state_root = session.root();
        let event_hash = event_hasher.finalize().into();

        Ok((
            spent_txs,
            discarded_txs,
            VerificationOutput {
                state_root,
                event_hash,
            },
        ))
    }

    /// Verify the given transactions are ok.
    pub fn verify_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: &BlsPublicKey,
        txs: &[Transaction],
        slashing: Vec<Slash>,
        voters: Option<&[VoterWithCredits]>,
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let session = self.session(block_height, None)?;

        accept(
            session,
            block_height,
            block_gas_limit,
            generator,
            txs,
            slashing,
            voters,
            self.gas_per_deploy_byte,
        )
        .map(|(a, b, _, _)| (a, b))
    }

    /// Accept the given transactions.
    ///
    ///   * `consistency_check` - represents a state_root, the caller expects to
    ///   be returned on successful transactions execution. Passing a None
    ///   value disables the check.
    #[allow(clippy::too_many_arguments)]
    pub fn accept_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
        consistency_check: Option<VerificationOutput>,
        slashing: Vec<Slash>,
        voters: Option<&[VoterWithCredits]>,
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let session = self.session(block_height, None)?;

        let (spent_txs, verification_output, session, events) = accept(
            session,
            block_height,
            block_gas_limit,
            &generator,
            &txs[..],
            slashing,
            voters,
            self.gas_per_deploy_byte,
        )?;

        if let Some(expected_verification) = consistency_check {
            if expected_verification != verification_output {
                // Drop the session if the resulting is inconsistent
                // with the callers one.
                return Err(Error::InconsistentState(verification_output));
            }
        }

        self.set_current_commit(session.commit()?);

        for event in events {
            let _ = self.event_sender.send(event.into());
        }

        Ok((spent_txs, verification_output))
    }

    pub fn finalize_state(
        &self,
        commit: [u8; 32],
        to_delete: Vec<[u8; 32]>,
    ) -> Result<()> {
        let commit_id_path = to_rusk_state_id_path(&self.dir);
        fs::write(commit_id_path, commit)?;

        self.set_base_and_delete(commit, to_delete);
        Ok(())
    }

    pub fn revert(&self, state_hash: [u8; 32]) -> Result<[u8; 32]> {
        let mut tip = self.tip.write();

        let commits = self.vm.commits();
        if !commits.contains(&state_hash) {
            return Err(Error::CommitNotFound(state_hash));
        }

        tip.current = state_hash;
        Ok(tip.current)
    }

    pub fn revert_to_base_root(&self) -> Result<[u8; 32]> {
        self.revert(self.base_root())
    }

    /// Get the base root.
    pub fn base_root(&self) -> [u8; 32] {
        self.tip.read().base
    }

    /// Get the current state root.
    pub fn state_root(&self) -> [u8; 32] {
        self.tip.read().current
    }

    /// Returns the nullifiers that already exist from a list of given
    /// `nullifiers`.
    pub fn existing_nullifiers(
        &self,
        nullifiers: &Vec<BlsScalar>,
    ) -> Result<Vec<BlsScalar>> {
        self.query(TRANSFER_CONTRACT, "existing_nullifiers", nullifiers)
    }

    /// Returns the stakes.
    pub fn provisioners(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> Result<impl Iterator<Item = (BlsPublicKey, StakeData)>> {
        let (sender, receiver) = mpsc::channel();
        self.feeder_query(STAKE_CONTRACT, "stakes", &(), sender, base_commit)?;
        Ok(receiver.into_iter().map(|bytes| {
            rkyv::from_bytes::<(BlsPublicKey, StakeData)>(&bytes).expect(
                "The contract should only return (pk, stake_data) tuples",
            )
        }))
    }

    /// Returns an account's information.
    pub fn account(&self, pk: &BlsPublicKey) -> Result<AccountData> {
        self.query(TRANSFER_CONTRACT, "account", pk)
    }

    /// Fetches the previous state data for stake changes in the contract.
    ///
    /// Communicates with the stake contract to obtain information about the
    /// state data before the last changes. Optionally takes a base commit
    /// hash to query changes since a specific point in time.
    ///
    /// # Arguments
    ///
    /// - `base_commit`: An optional base commit hash indicating the starting
    ///   point for querying changes.
    ///
    /// # Returns
    ///
    /// Returns a Result containing an iterator over tuples. Each tuple consists
    /// of a `BlsPublicKey` and an optional `StakeData`, representing the
    /// state data before the last changes in the stake contract.
    pub fn last_provisioners_change(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> Result<Vec<(BlsPublicKey, Option<StakeData>)>> {
        let (sender, receiver) = mpsc::channel();
        self.feeder_query(
            STAKE_CONTRACT,
            "prev_state_changes",
            &(),
            sender,
            base_commit,
        )?;
        Ok(receiver.into_iter().map(|bytes| {
            rkyv::from_bytes::<(BlsPublicKey, Option<StakeData>)>(&bytes).expect(
                "The contract should only return (pk, Option<stake_data>) tuples",
            )
        }).collect())
    }

    pub fn provisioner(&self, pk: &BlsPublicKey) -> Result<Option<StakeData>> {
        self.query(STAKE_CONTRACT, "get_stake", pk)
    }

    pub(crate) fn session(
        &self,
        block_height: u64,
        commit: Option<[u8; 32]>,
    ) -> Result<Session> {
        let commit = commit.unwrap_or_else(|| {
            let tip = self.tip.read();
            tip.current
        });

        let session = rusk_abi::new_session(&self.vm, commit, block_height)?;

        Ok(session)
    }

    pub(crate) fn set_current_commit(&self, commit: [u8; 32]) {
        let mut tip = self.tip.write();
        tip.current = commit;
    }

    pub(crate) fn set_base_and_delete(
        &self,
        base: [u8; 32],
        to_delete: Vec<[u8; 32]>,
    ) {
        self.tip.write().base = base;

        // Deleting commits is blocking, meaning it will wait until any process
        // using the commit is done. This includes any queries that are
        // currently executing.
        // Since we do want commits to be deleted, but don't want block
        // finalization to wait, we spawn a new task to delete the commits.
        task::spawn(delete_commits(self.vm.clone(), to_delete));
    }
}

async fn delete_commits(vm: Arc<VM>, commits: Vec<[u8; 32]>) {
    for commit in commits {
        if let Err(err) = vm.delete_commit(commit) {
            debug!("failed deleting commit {}: {err}", hex::encode(commit));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn accept(
    session: Session,
    block_height: u64,
    block_gas_limit: u64,
    generator: &BlsPublicKey,
    txs: &[Transaction],
    slashing: Vec<Slash>,
    voters: Option<&[VoterWithCredits]>,
    gas_per_deploy_byte: Option<u64>,
) -> Result<(
    Vec<SpentTransaction>,
    VerificationOutput,
    Session,
    Vec<Event>,
)> {
    let mut session = session;

    let mut block_gas_left = block_gas_limit;

    let mut spent_txs = Vec::with_capacity(txs.len());
    let mut dusk_spent = 0;

    let mut events = Vec::new();
    let mut event_hasher = Sha3_256::new();

    for unspent_tx in txs {
        let tx = &unspent_tx.inner;
        let receipt = execute(&mut session, tx, gas_per_deploy_byte)?;

        update_hasher(&mut event_hasher, &receipt.events);
        events.extend(receipt.events);

        let gas_spent = receipt.gas_spent;

        dusk_spent += gas_spent * tx.gas_price();
        block_gas_left = block_gas_left
            .checked_sub(gas_spent)
            .ok_or(Error::OutOfGas)?;

        spent_txs.push(SpentTransaction {
            inner: unspent_tx.clone(),
            gas_spent,
            block_height,
            // We're currently ignoring the result of successful calls
            err: receipt.data.err().map(|e| format!("{e}")),
        });
    }

    let coinbase_events = reward_slash_and_update_root(
        &mut session,
        block_height,
        dusk_spent,
        generator,
        slashing,
        voters,
    )?;

    update_hasher(&mut event_hasher, &coinbase_events);
    events.extend(coinbase_events);

    let state_root = session.root();
    let event_hash = event_hasher.finalize().into();

    Ok((
        spent_txs,
        VerificationOutput {
            state_root,
            event_hash,
        },
        session,
        events,
    ))
}

// Returns gas charge for bytecode deployment.
fn bytecode_charge(
    bytecode: &Bytecode,
    gas_per_deploy_byte: &Option<u64>,
) -> u64 {
    bytecode.bytes.len() as u64
        * gas_per_deploy_byte.unwrap_or(DEFAULT_GAS_PER_DEPLOY_BYTE)
}

// Contract deployment will fail and charge full gas limit in the
// following cases:
// 1) Transaction gas limit is smaller than deploy charge plus gas used for
//    spending funds.
// 2) Transaction's bytecode's bytes are not consistent with bytecode's hash.
// 3) Deployment fails for deploy-specific reasons like e.g.:
//      - contract already deployed
//      - corrupted bytecode
//      - sufficient gas to spend funds yet insufficient for deployment
fn contract_deploy(
    session: &mut Session,
    deploy: &ContractDeploy,
    gas_limit: u64,
    gas_per_deploy_byte: Option<u64>,
    receipt: &mut CallReceipt<Result<Vec<u8>, ContractError>>,
) {
    let deploy_charge = bytecode_charge(&deploy.bytecode, &gas_per_deploy_byte);
    let min_gas_limit = receipt.gas_spent + deploy_charge;
    let hash = blake3::hash(deploy.bytecode.bytes.as_slice());
    if gas_limit < min_gas_limit {
        receipt.data = Err(OutOfGas);
    } else if hash != deploy.bytecode.hash {
        receipt.data = Err(Panic("failed bytecode hash check".into()))
    } else {
        let result = session.deploy_raw(
            Some(gen_contract_id(
                &deploy.bytecode.bytes,
                deploy.nonce,
                &deploy.owner,
            )),
            deploy.bytecode.bytes.as_slice(),
            deploy.constructor_args.clone(),
            deploy.owner.clone(),
            gas_limit - receipt.gas_spent,
        );
        match result {
            Ok(_) => receipt.gas_spent += deploy_charge,
            Err(err) => {
                info!("Tx caused deployment error {err:?}");
                receipt.data = Err(Panic("failed deployment".into()))
            }
        }
    }
}

/// Executes a transaction, returning the receipt of the call and the gas spent.
/// The following steps are performed:
///
/// 1. Check if the transaction contains contract deployment data, and if so,
///    verifies if gas limit is enough for deployment. If gas limit is not
///    sufficient for deployment, transaction is discarded.
///
/// 2. Call the "spend_and_execute" function on the transfer contract with
///    unlimited gas. If this fails, an error is returned. If an error is
///    returned the transaction should be considered unspendable/invalid, but no
///    re-execution of previous transactions is required.
///
/// 3. If the transaction contains contract deployment data, additional checks
///    are performed and if they pass, deployment is executed. The following
///    checks are performed:
///    - gas limit should be is smaller than deploy charge plus gas used for
///      spending funds
///    - transaction's bytecode's bytes are consistent with bytecode's hash
///    Deployment execution may fail for deployment-specific reasons, such as
///    for example:
///    - contract already deployed
///    - corrupted bytecode
///    If deployment execution fails, the entire gas limit is consumed and error
///    is returned.
///
/// 4. Call the "refund" function on the transfer contract with unlimited gas.
///    The amount charged depends on the gas spent by the transaction, and the
///    optional contract call in steps 2 or 3.
///
/// Note that deployment transaction will never be re-executed for reasons
/// related to deployment, as it is either discarded or it charges the
/// full gas limit. It might be re-executed only if some other transaction
/// failed to fit the block.
fn execute(
    session: &mut Session,
    tx: &ProtocolTransaction,
    gas_per_deploy_byte: Option<u64>,
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, PiecrustError> {
    // Transaction will be discarded if it is a deployment transaction
    // with gas limit smaller than deploy charge.
    if let Some(deploy) = tx.deploy() {
        let deploy_charge =
            bytecode_charge(&deploy.bytecode, &gas_per_deploy_byte);
        if tx.gas_limit() < deploy_charge {
            return Err(PiecrustError::Panic(
                "not enough gas to deploy".into(),
            ));
        }
    }

    let tx_stripped = tx.strip_off_bytecode();
    // Spend the inputs and execute the call. If this errors the transaction is
    // unspendable.
    let mut receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        tx_stripped.as_ref().unwrap_or(tx),
        tx.gas_limit(),
    )?;

    // Deploy if this is a deployment transaction and spend part is successful.
    if let Some(deploy) = tx.deploy() {
        if receipt.data.is_ok() {
            contract_deploy(
                session,
                deploy,
                tx.gas_limit(),
                gas_per_deploy_byte,
                &mut receipt,
            );
        }
    };

    // Ensure all gas is consumed if there's an error in the contract call
    if receipt.data.is_err() {
        receipt.gas_spent = receipt.gas_limit;
    }

    // Refund the appropriate amount to the transaction. This call is guaranteed
    // to never error. If it does, then a programming error has occurred. As
    // such, the call to `Result::expect` is warranted.
    let refund_receipt = session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &receipt.gas_spent,
            u64::MAX,
        )
        .expect("Refunding must succeed");

    receipt.events.extend(refund_receipt.events);

    Ok(receipt)
}

fn update_hasher(hasher: &mut Sha3_256, events: &[Event]) {
    for event in events {
        hasher.update(event.source.as_bytes());
        hasher.update(event.topic.as_bytes());
        hasher.update(&event.data);
    }
}

fn reward_slash_and_update_root(
    session: &mut Session,
    block_height: u64,
    dusk_spent: Dusk,
    generator: &BlsPublicKey,
    slashing: Vec<Slash>,
    voters: Option<&[(BlsPublicKey, usize)]>,
) -> Result<Vec<Event>> {
    let (
        dusk_value,
        generator_fixed_reward,
        generator_extra_reward,
        voters_reward,
    ) = coinbase_value(block_height, dusk_spent);

    let credits = voters
        .unwrap_or_default()
        .iter()
        .map(|(_, credits)| *credits as u64)
        .sum::<u64>();

    if voters.is_some() && credits == 0 && block_height > 1 {
        return Err(InvalidCreditsCount(block_height, 0));
    }

    let credit_reward = voters_reward / 64 * 2;

    let r = session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(*DUSK_KEY, dusk_value),
        u64::MAX,
    )?;

    let mut events = r.events;

    debug!(
        event = "Dusk rewarded",
        voter = to_bs58(&DUSK_KEY),
        reward = dusk_value
    );

    let generator_curr_extra_reward =
        calc_generator_extra_reward(generator_extra_reward, credits);

    let generator_reward = generator_fixed_reward + generator_curr_extra_reward;
    let r = session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(*generator, generator_reward),
        u64::MAX,
    )?;
    events.extend(r.events);

    debug!(
        event = "generator rewarded",
        voter = to_bs58(generator),
        total_reward = generator_reward,
        extra_reward = generator_curr_extra_reward,
        credits,
    );

    for (to_voter, credits) in voters.unwrap_or_default() {
        let voter_reward = *credits as u64 * credit_reward;
        let r = session.call::<_, ()>(
            STAKE_CONTRACT,
            "reward",
            &(*to_voter, voter_reward),
            u64::MAX,
        )?;
        events.extend(r.events);

        debug!(
            event = "validator of prev block rewarded",
            voter = to_bs58(to_voter),
            credits = *credits,
            reward = voter_reward
        )
    }

    events.extend(slash(session, slashing)?);

    let r = session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "update_root",
        &(),
        u64::MAX,
    )?;
    events.extend(r.events);

    Ok(events)
}

/// Calculates current extra reward for Block generator.
fn calc_generator_extra_reward(
    generator_extra_reward: Dusk,
    credits: u64,
) -> u64 {
    if credits
        == (VALIDATION_COMMITTEE_CREDITS + RATIFICATION_COMMITTEE_CREDITS)
            as u64
    {
        return generator_extra_reward;
    }

    let reward_per_quota = generator_extra_reward
        / (validation_extra() + ratification_extra()) as u64;

    let sum = ratification_committee_quorum() + validation_committee_quorum();
    credits.saturating_sub(sum as u64) * reward_per_quota
}

fn to_bs58(pk: &BlsPublicKey) -> String {
    let mut pk = bs58::encode(&pk.to_bytes()).into_string();
    pk.truncate(16);
    pk
}

fn slash(session: &mut Session, slash: Vec<Slash>) -> Result<Vec<Event>> {
    let mut events = vec![];
    for s in slash {
        let provisioner = s.provisioner.into_inner();
        let r = match s.r#type {
            node_data::ledger::SlashType::Soft => session.call::<_, ()>(
                STAKE_CONTRACT,
                "slash",
                &(provisioner, None::<u64>),
                u64::MAX,
            ),
            node_data::ledger::SlashType::Hard => session.call::<_, ()>(
                STAKE_CONTRACT,
                "hard_slash",
                &(provisioner, None::<u64>, None::<u64>),
                u64::MAX,
            ),
            node_data::ledger::SlashType::HardWithSeverity(severity) => session
                .call::<_, ()>(
                    STAKE_CONTRACT,
                    "hard_slash",
                    &(provisioner, None::<u64>, Some(severity)),
                    u64::MAX,
                ),
        }?;
        events.extend(r.events);
    }
    Ok(events)
}
