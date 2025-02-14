// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::Instant;
use std::{fs, io};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_consensus::config::{
    ratification_extra, ratification_quorum, validation_extra,
    validation_quorum, MAX_NUMBER_OF_TRANSACTIONS,
    RATIFICATION_COMMITTEE_CREDITS, VALIDATION_COMMITTEE_CREDITS,
};
use dusk_consensus::operations::{CallParams, VerificationOutput, Voter};
use dusk_core::abi::Event;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{
    Reward, RewardReason, StakeData, StakeKeys, STAKE_CONTRACT,
};
use dusk_core::transfer::{
    moonlight::AccountData, PANIC_NONCE_NOT_READY, TRANSFER_CONTRACT,
};
use dusk_core::{BlsScalar, Dusk};
use dusk_vm::{
    execute, CallReceipt, Error as VMError, ExecutionConfig, Session, VM,
};
#[cfg(feature = "archive")]
use node::archive::Archive;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::{Header, Slash, SpentTransaction, Transaction};
use parking_lot::RwLock;
use rusk_profile::to_rusk_state_id_path;
use tokio::sync::broadcast;
use tracing::info;

use super::RuskVmConfig;
use crate::bloom::Bloom;
use crate::http::RuesEvent;
use crate::node::{coinbase_value, Rusk, RuskTip};
use crate::Error::InvalidCreditsCount;
use crate::{Error, Result, DUSK_CONSENSUS_KEY};

impl Rusk {
    pub fn new<P: AsRef<Path>>(
        dir: P,
        chain_id: u8,
        vm_config: RuskVmConfig,
        min_gas_limit: u64,
        feeder_gas_limit: u64,
        event_sender: broadcast::Sender<RuesEvent>,
        #[cfg(feature = "archive")] archive: Archive,
    ) -> Result<Self> {
        let dir = dir.as_ref();
        info!("Using state from {dir:?}");

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

        let vm = Arc::new(VM::new(dir)?);

        let tip = Arc::new(RwLock::new(RuskTip {
            current: base_commit,
            base: base_commit,
        }));

        Ok(Self {
            tip,
            vm,
            dir: dir.into(),
            chain_id,
            vm_config,
            min_gas_limit,
            feeder_gas_limit,
            event_sender,
            #[cfg(feature = "archive")]
            archive,
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
        let block_gas_limit = self.vm_config.block_gas_limit;
        let generator = params.generator_pubkey.inner();
        let to_slash = params.to_slash.clone();
        let prev_state_root = params.prev_state_root;

        let voters = &params.voters_pubkey[..];

        let mut session =
            self.new_block_session(block_height, prev_state_root)?;

        let mut block_gas_left = block_gas_limit;

        let mut spent_txs = Vec::<SpentTransaction>::new();
        let mut discarded_txs = vec![];

        let mut dusk_spent = 0;

        let mut event_bloom = Bloom::new();

        let execution_config = self.vm_config.to_execution_config(block_height);

        // We always write the faults len in a u32
        let mut size_left = params.max_txs_bytes - u32::SIZE;

        for unspent_tx in txs {
            if let Some(timeout) = self.vm_config.generation_timeout {
                if started.elapsed() > timeout {
                    info!("execute_transactions timeout triggered {timeout:?}");
                    break;
                }
            }

            // Limit execution to the block transactions limit
            if spent_txs.len() >= MAX_NUMBER_OF_TRANSACTIONS {
                info!("Maximum number of transactions reached");
                break;
            }

            let tx_id_hex = hex::encode(unspent_tx.id());
            let tx_len = unspent_tx.size().unwrap_or_default();

            if tx_len == 0 {
                info!("Skipping {tx_id_hex} due to error while calculating the len");
                continue;
            }

            if tx_len > size_left {
                info!("Skipping {tx_id_hex} due size greater than bytes left: {size_left}");
                continue;
            }

            match execute(&mut session, &unspent_tx.inner, &execution_config) {
                Ok(receipt) => {
                    let gas_spent = receipt.gas_spent;

                    // If the transaction went over the block gas limit we
                    // re-execute all spent transactions. We don't discard the
                    // transaction, since it is technically valid.
                    if gas_spent > block_gas_left {
                        info!("Skipping {tx_id_hex} due gas_spent {gas_spent} greater than left: {block_gas_left}");
                        session = self
                            .new_block_session(block_height, prev_state_root)?;

                        for spent_tx in &spent_txs {
                            // We know these transactions were correctly
                            // executed before, so we don't bother checking.
                            let _ = execute(
                                &mut session,
                                &spent_tx.inner.inner,
                                &execution_config,
                            );
                        }

                        continue;
                    }

                    size_left -= tx_len;

                    // We're currently ignoring the result of successful calls
                    let err = receipt.data.err().map(|e| format!("{e}"));
                    info!("Tx {tx_id_hex} executed with {gas_spent} gas and err {err:?}");

                    event_bloom.add_events(&receipt.events);

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
                Err(VMError::Panic(val)) if val == PANIC_NONCE_NOT_READY => {
                    // If the transaction panic due to a not yet valid nonce,
                    // we should not discard the transactions since it can be
                    // included in future.

                    // TODO: Try to process the transaction as soon as the
                    // nonce is unlocked
                }
                Err(e) => {
                    info!("discard tx {tx_id_hex} due to {e:?}");
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

        event_bloom.add_events(&coinbase_events);

        let state_root = session.root();

        Ok((
            spent_txs,
            discarded_txs,
            VerificationOutput {
                state_root,
                event_bloom: event_bloom.into(),
            },
        ))
    }

    /// Verify the given transactions are ok.
    pub fn verify_transactions(
        &self,
        prev_commit: [u8; 32],
        header: &Header,
        generator: &BlsPublicKey,
        txs: &[Transaction],
        slashing: Vec<Slash>,
        voters: &[Voter],
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let block_height = header.height;
        let session = self.new_block_session(block_height, prev_commit)?;
        let execution_config = self.vm_config.to_execution_config(block_height);

        accept(
            session,
            header,
            generator,
            txs,
            slashing,
            voters,
            &execution_config,
        )
        .map(|(a, b, _, _)| (a, b))
    }

    /// Accept the given transactions.
    ///
    ///   * `consistency_check` - represents a state_root, the caller expects to
    ///   be returned on successful transactions execution. Passing a None
    ///   value disables the check.
    ///
    /// # Returns
    ///  - Vec<SpentTransaction> - The transactions that were spent.
    /// - VerificationOutput - The verification output.
    /// - Vec<ContractTxEvent> - All contract events that were emitted from the
    ///   given transactions.
    pub fn accept_transactions(
        &self,
        prev_commit: [u8; 32],
        header: &Header,
        txs: &[Transaction],
        consistency_check: Option<VerificationOutput>,
        slashing: Vec<Slash>,
        voters: &[Voter],
    ) -> Result<(
        Vec<SpentTransaction>,
        VerificationOutput,
        Vec<ContractTxEvent>,
    )> {
        let generator = header.generator_bls_pubkey.inner();
        let generator = BlsPublicKey::from_slice(generator).map_err(|e| {
            Error::Other(anyhow::anyhow!("Error in from_slice {e:?}").into())
        })?;
        let block_height = header.height;
        let session = self.new_block_session(block_height, prev_commit)?;

        let execution_config = self.vm_config.to_execution_config(block_height);

        let (spent_txs, verification_output, session, events) = accept(
            session,
            header,
            &generator,
            txs,
            slashing,
            voters,
            &execution_config,
        )?;

        if let Some(expected_verification) = consistency_check {
            if expected_verification != verification_output {
                // Drop the session if the resulting is inconsistent
                // with the callers one.
                return Err(Error::InconsistentState(Box::new(
                    verification_output,
                )));
            }
        }

        self.set_current_commit(session.commit()?);

        let contract_events = events.clone();
        for event in events {
            // Send VM event to RUES
            let event = RuesEvent::from(event);
            let _ = self.event_sender.send(event);
        } // TODO: move this also in acceptor (async fn try_accept_block) where
          // stake events are filtered, to avoid looping twice?

        Ok((spent_txs, verification_output, contract_events))
    }

    pub fn finalize_state(
        &self,
        commit: [u8; 32],
        to_merge: Vec<[u8; 32]>,
    ) -> Result<()> {
        self.set_base_and_merge(commit, to_merge)?;

        let commit_id_path = to_rusk_state_id_path(&self.dir);
        fs::write(commit_id_path, commit)?;
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
    ) -> Result<impl Iterator<Item = (StakeKeys, StakeData)>> {
        let (sender, receiver) = mpsc::channel();
        self.feeder_query(STAKE_CONTRACT, "stakes", &(), sender, base_commit)?;
        Ok(receiver.into_iter().map(|bytes| {
            rkyv::from_bytes::<(StakeKeys, StakeData)>(&bytes).expect(
                "The contract should only return (StakeKeys, StakeData) tuples",
            )
        }))
    }

    /// Returns an account's information.
    pub fn account(&self, pk: &BlsPublicKey) -> Result<AccountData> {
        self.query(TRANSFER_CONTRACT, "account", pk)
    }

    /// Returns an account's information.
    pub fn chain_id(&self) -> Result<u8> {
        self.query(TRANSFER_CONTRACT, "chain_id", &())
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

    /// Opens a session for a new block proposal/verification.
    ///
    /// Before returning the session, "before_state_transition" of Stake
    /// Contract is called
    pub(crate) fn new_block_session(
        &self,
        block_height: u64,
        commit: [u8; 32],
    ) -> Result<Session> {
        let mut session = self._session(block_height, None)?;
        if session.root() != commit {
            return Err(Error::TipChanged);
        }
        let _: CallReceipt<()> = session
            .call(STAKE_CONTRACT, "before_state_transition", &(), u64::MAX)
            .expect("before_state_transition to success");
        Ok(session)
    }

    /// Opens a session for query, setting a block height of zero since this
    /// doesn't affect the result.
    pub(crate) fn query_session(
        &self,
        commit: Option<[u8; 32]>,
    ) -> Result<Session> {
        self._session(0, commit)
    }

    /// Opens a new session with the specified block height and commit hash.
    ///
    /// # Warning
    /// This is a low-level function intended for internal use only.
    /// Directly invoking `_session` bypasses critical preconditions, such as
    /// the "before_state_transition" call to the Stake Contract, which are
    /// enforced by higher-level functions like `new_block_session`.
    ///
    /// Instead, use the public-facing functions like `new_block_session` or
    /// `query_session` to ensure correct behavior and consistency.
    ///
    /// # Parameters
    /// - `block_height`: The height of the block for which the session is
    ///   created.
    /// - `commit`: The optional commit hash. If not provided, the current tip
    ///   is used.
    ///
    /// # Returns
    /// - A `Result` containing a `Session` if successful, or an error if the
    ///   session could not be created.
    ///
    /// # Errors
    /// - Returns an error if the session could not be initialized with the
    ///   given parameters.
    fn _session(
        &self,
        block_height: u64,
        commit: Option<[u8; 32]>,
    ) -> Result<Session> {
        let commit = commit.unwrap_or_else(|| {
            let tip = self.tip.read();
            tip.current
        });

        let session = self.vm.session(commit, self.chain_id, block_height)?;

        Ok(session)
    }

    pub(crate) fn set_current_commit(&self, commit: [u8; 32]) {
        let mut tip = self.tip.write();
        tip.current = commit;
    }

    pub(crate) fn set_base_and_merge(
        &self,
        base: [u8; 32],
        to_merge: Vec<[u8; 32]>,
    ) -> Result<()> {
        self.tip.write().base = base;
        for d in to_merge {
            if d == base {
                // Don't finalize the new tip, otherwise it will not be
                // accessible anymore
                continue;
            };
            self.vm.finalize_commit(d)?;
        }
        Ok(())
    }
}

fn accept(
    session: Session,
    header: &Header,
    generator: &BlsPublicKey,
    txs: &[Transaction],
    slashing: Vec<Slash>,
    voters: &[Voter],
    execution_config: &ExecutionConfig,
) -> Result<(
    Vec<SpentTransaction>,
    VerificationOutput,
    Session,
    Vec<ContractTxEvent>,
)> {
    let mut session = session;

    let mut block_gas_left = header.gas_limit;
    let block_height = header.height;

    let mut spent_txs = Vec::with_capacity(txs.len());
    let mut dusk_spent = 0;

    let mut events = Vec::new();
    let mut event_bloom = Bloom::new();

    for unspent_tx in txs {
        let tx = &unspent_tx.inner;
        let tx_id = unspent_tx.id();
        let receipt = execute(&mut session, tx, execution_config)?;

        event_bloom.add_events(&receipt.events);

        let tx_events: Vec<_> = receipt
            .events
            .into_iter()
            .map(|event| ContractTxEvent {
                event: event.into(),
                origin: tx_id,
            })
            .collect();

        events.extend(tx_events);

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

    event_bloom.add_events(&coinbase_events);

    let coinbase_events =
        coinbase_events.into_iter().map(|event| ContractTxEvent {
            event: event.into(),
            origin: header.hash,
        });
    events.extend(coinbase_events);

    let state_root = session.root();

    Ok((
        spent_txs,
        VerificationOutput {
            state_root,
            event_bloom: event_bloom.into(),
        },
        session,
        events,
    ))
}

fn reward_slash_and_update_root(
    session: &mut Session,
    block_height: u64,
    dusk_spent: Dusk,
    generator: &BlsPublicKey,
    slashing: Vec<Slash>,
    voters: &[Voter],
) -> Result<Vec<Event>> {
    let (dusk_value, generator_reward, generator_extra_reward, voters_reward) =
        coinbase_value(block_height, dusk_spent);

    let credits = voters
        .iter()
        .map(|(_, credits)| *credits as u64)
        .sum::<u64>();

    if !voters.is_empty() && credits == 0 && block_height > 1 {
        return Err(InvalidCreditsCount(block_height, 0));
    }

    let generator_extra_reward =
        calc_generator_extra_reward(generator_extra_reward, credits);

    // We first start with only the generator (fixed) and Dusk
    let mut num_rewards = 2;

    // If there is an extra reward we add it.
    if generator_extra_reward != 0 {
        num_rewards += 1;
    }

    // Additionally we also reward the voters.
    num_rewards += voters.len();

    let mut rewards = Vec::with_capacity(num_rewards);

    rewards.push(Reward {
        account: *generator,
        value: generator_reward,
        reason: RewardReason::GeneratorFixed,
    });

    rewards.push(Reward {
        account: *DUSK_CONSENSUS_KEY,
        value: dusk_value,
        reason: RewardReason::Other,
    });

    if generator_extra_reward != 0 {
        rewards.push(Reward {
            account: *generator,
            value: generator_extra_reward,
            reason: RewardReason::GeneratorExtra,
        });
    }

    let credit_reward = voters_reward
        / (VALIDATION_COMMITTEE_CREDITS + RATIFICATION_COMMITTEE_CREDITS)
            as u64;

    for (to_voter, credits) in voters {
        let voter = to_voter.inner();
        let voter_reward = *credits as u64 * credit_reward;
        rewards.push(Reward {
            account: *voter,
            value: voter_reward,
            reason: RewardReason::Voter,
        });
    }

    let r =
        session.call::<_, ()>(STAKE_CONTRACT, "reward", &rewards, u64::MAX)?;

    let mut events = r.events;

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

    let sum = ratification_quorum() + validation_quorum();
    credits.saturating_sub(sum as u64) * reward_per_quota
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
            // INFO: Hard Slashing is currently "relaxed" to Soft Slashing as a
            // safety measure for the initial period after mainnet launch.
            // Proper behavior should be restored in the future
            node_data::ledger::SlashType::Hard => session.call::<_, ()>(
                STAKE_CONTRACT,
                "slash",
                &(provisioner, None::<u64>),
                u64::MAX,
            ),
            node_data::ledger::SlashType::HardWithSeverity(_severity) => {
                session.call::<_, ()>(
                    STAKE_CONTRACT,
                    "slash",
                    &(provisioner, None::<u64>),
                    u64::MAX,
                )
            }
        }?;
        events.extend(r.events);
    }
    Ok(events)
}
