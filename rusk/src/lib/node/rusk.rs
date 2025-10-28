// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::Instant;
use std::{fs, io};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_consensus::config::{
    ratification_extra, ratification_quorum, validation_extra,
    validation_quorum, MAX_NUMBER_OF_TRANSACTIONS, TOTAL_COMMITTEES_CREDITS,
};
use dusk_consensus::errors::StateTransitionError;
use dusk_consensus::operations::{
    StateTransitionData, StateTransitionResult, Voter,
};
use dusk_core::abi::{ContractId, Event};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{
    Reward, RewardReason, StakeData, StakeKeys, STAKE_CONTRACT,
};
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::{
    Transaction as ProtocolTransaction, PANIC_NONCE_NOT_READY,
    TRANSFER_CONTRACT,
};
use dusk_core::{BlsScalar, Dusk};
use dusk_vm::{execute, CallReceipt, Error as VMError, Session, VM};
#[cfg(feature = "archive")]
use node::archive::Archive;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::{to_str, Block, Slash, SpentTransaction, Transaction};
use parking_lot::RwLock;
use rusk_profile::to_rusk_state_id_path;
use tokio::sync::broadcast;
use tracing::info;

use super::RuskVmConfig;
use crate::bloom::Bloom;
use crate::node::driverstore::DriverStore;
use crate::node::{get_block_rewards, RuesEvent, Rusk, RuskTip};
use crate::{Error as RuskError, Result, DUSK_CONSENSUS_KEY};

impl Rusk {
    #[allow(clippy::too_many_arguments)]
    pub fn new<P: AsRef<Path>>(
        dir: P,
        chain_id: u8,
        vm_config: RuskVmConfig,
        min_gas_limit: u64,
        feeder_gas_limit: u64,
        event_sender: broadcast::Sender<RuesEvent>,
        #[cfg(feature = "archive")] archive: Archive,
        driver_store: DriverStore,
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

        let mut vm = VM::new(dir)?;
        for (feat, activation) in vm_config.features() {
            let feat = feat.to_ascii_lowercase();
            if let Some(hq_name) = feat.strip_prefix("hq_") {
                vm.with_hq_activation(hq_name, *activation);
            }
        }

        let vm = Arc::new(vm);

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
            driver_store: Arc::new(RwLock::new(driver_store)),
            instance_cache: Arc::new(RwLock::new(BTreeMap::new())),
        })
    }

    pub fn create_state_transition<I: Iterator<Item = Transaction>>(
        &self,
        transition_data: &StateTransitionData,
        mut mempool_txs: I,
    ) -> Result<
        (
            Vec<SpentTransaction>,
            Vec<Transaction>,
            StateTransitionResult,
        ),
        StateTransitionError,
    > {
        let started = Instant::now();

        let block_height = transition_data.round;
        let gas_limit = self
            .vm_config
            .block_gas_limit
            .expect("vm block-gas-limit configuration item should exist");
        let generator = transition_data.generator.inner();
        let slashes = transition_data.slashes.clone();
        let prev_state = transition_data.prev_state_root;

        let cert_voters = &transition_data.cert_voters[..];

        info!(
            event = "Creating state transition",
            height = block_height,
            prev_state = to_str(&prev_state),
            gas_limit,
            ?slashes
        );

        let mut session = self.new_block_session(block_height, prev_state)?;

        let mut gas_left = gas_limit;

        let mut spent_txs = Vec::<SpentTransaction>::new();
        let mut discarded_txs = vec![];

        let mut dusk_spent = 0;

        let mut event_bloom = Bloom::new();

        let execution_config = self.vm_config.to_execution_config(block_height);

        // We always write the faults len in a u32
        let mut space_left = transition_data.max_txs_bytes - u32::SIZE;

        // We use the pending list to keep track of transactions whose nonce is
        // not yet valid but may become valid when the transactions using the
        // missing nonces are executed.
        // When a transaction in the pending list becomes valid (wrt the nonce)
        // it is added to the unblocked list to be processed immediately.
        // Unblocked transactions have priority over other transactions in the
        // mempool.
        let mut pending_txs: BTreeMap<[u8; 193], BTreeMap<u64, Transaction>> =
            BTreeMap::new();

        let mut unblocked_txs = VecDeque::new();

        while let Some(unspent_tx) =
            unblocked_txs.pop_front().or_else(|| mempool_txs.next())
        {
            if let Some(timeout) = self.vm_config.generation_timeout {
                if started.elapsed() > timeout {
                    info!(
                        event = "Stop creating state transition",
                        reason = "timeout expired",
                        ?timeout
                    );
                    break;
                }
            }

            // Limit execution to the block transactions limit
            if spent_txs.len() >= MAX_NUMBER_OF_TRANSACTIONS {
                info!(
                    event = "Stop creating state transition",
                    reason = "maximum number of transactions reached"
                );
                break;
            }

            let tx_id = hex::encode(unspent_tx.id());
            let tx_size = unspent_tx.size();

            if tx_size > space_left {
                info!(
                    event = "Skipping transaction",
                    reason = "not enough space in block",
                    tx_id,
                    tx_size,
                    space_left
                );
                continue;
            }

            match execute(&mut session, &unspent_tx.inner, &execution_config) {
                Ok(receipt) => {
                    let gas_spent = receipt.gas_spent;

                    // If the transaction went over the block gas limit we
                    // re-execute all spent transactions. We don't discard the
                    // transaction, since it is technically valid.
                    if gas_spent > gas_left {
                        info!(
                            event = "Skipping transaction",
                            reason = "exceeding block gas limit",
                            tx_id,
                            gas_spent,
                            gas_left
                        );

                        session =
                            self.new_block_session(block_height, prev_state)?;

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

                    space_left -= tx_size;

                    // We're currently ignoring the result of successful calls
                    let error = receipt.data.err().map(|e| format!("{e}"));
                    info!(event = "Tx executed", tx_id, gas_spent, error);

                    event_bloom.add_events(&receipt.events);

                    gas_left -= gas_spent;
                    let gas_price = unspent_tx.inner.gas_price();
                    dusk_spent += gas_spent * gas_price;

                    if let ProtocolTransaction::Moonlight(tx) =
                        &unspent_tx.inner
                    {
                        // Check if the current transaction unblocks any
                        // transaction from the same in the pending list.
                        // All transactions with valid subsequent nonces are
                        // added to the unblocked list to be processed
                        // immediately.
                        let sender = tx.sender().to_raw_bytes();
                        if let Some(pendings) = pending_txs.get_mut(&sender) {
                            let mut next_nonce = tx.nonce() + 1;

                            while let Some(next_tx) =
                                pendings.remove(&next_nonce)
                            {
                                let tx_id = hex::encode(next_tx.id());
                                unblocked_txs.push_back(next_tx);
                                info!(
                                    event = "Reinserting transaction",
                                    reason = "Nonce ready",
                                    tx_id,
                                    nonce = next_nonce,
                                );
                                next_nonce += 1;
                            }

                            // Clean up empty map for sender
                            if pendings.is_empty() {
                                pending_txs.remove(&sender);
                            }
                        }
                    }

                    spent_txs.push(SpentTransaction {
                        inner: unspent_tx,
                        gas_spent,
                        block_height,
                        err: error,
                    });
                }
                Err(VMError::Panic(val)) if val == PANIC_NONCE_NOT_READY => {
                    // If the transaction panics due to a not yet valid nonce,
                    // we do not discard it.
                    // Instead, we add it to a list of pending transactions so
                    // it can be processed immediately when the nonce become
                    // valid (i.e., all transactions with
                    // the missing nonces are executed in this loop).
                    if let ProtocolTransaction::Moonlight(tx) =
                        &unspent_tx.inner
                    {
                        let nonce = tx.nonce();
                        pending_txs
                            .entry(tx.sender().to_raw_bytes())
                            .or_default()
                            .insert(tx.nonce(), unspent_tx);
                        info!(
                            event = "Skipping transaction",
                            reason = "Future Nonce",
                            tx_id,
                            nonce
                        );
                    }

                    continue;
                }
                Err(error) => {
                    info!(event = "Tx discarded", tx_id, ?error);
                    // An unspendable transaction should be discarded
                    discarded_txs.push(unspent_tx);
                    continue;
                }
            }
        }

        let coinbase_events = reward_and_slash(
            &mut session,
            block_height,
            generator,
            cert_voters,
            dusk_spent,
            slashes,
        )
        .map_err(|err| {
            StateTransitionError::ExecutionError(format!("{err}"))
        })?;

        event_bloom.add_events(&coinbase_events);

        let state_root = session.root();

        Ok((
            spent_txs,
            discarded_txs,
            StateTransitionResult {
                state_root,
                event_bloom: event_bloom.into(),
            },
        ))
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
            return Err(RuskError::CommitNotFound(state_hash));
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

    /// Return the active moonlight accounts
    pub fn moonlight_accounts(
        &self,
        base_commit: Option<[u8; 32]>,
    ) -> Result<impl Iterator<Item = (AccountData, BlsPublicKey)>> {
        let (sender, receiver) = mpsc::channel();
        let sync_range = (0u64, u64::MAX);
        self.feeder_query(
            TRANSFER_CONTRACT,
            "sync_accounts",
            &sync_range,
            sender,
            base_commit,
        )?;

        Ok(receiver.into_iter().map(|bytes| {
            let from_bytes = rkyv::from_bytes::<(AccountData, [u8; 193])>(&bytes).expect(
                "The contract should only return (AccountData, [u8; 193]) tuples",
            );
            unsafe {
            (from_bytes.0, BlsPublicKey::from_slice_unchecked(&from_bytes.1))
            }
        }))
    }

    /// Returns an account's information.
    pub fn account(&self, pk: &BlsPublicKey) -> Result<AccountData> {
        self.query(TRANSFER_CONTRACT, "account", pk)
    }

    /// Returns the balance held by a smart contract by its `ContractId`.
    pub fn contract_balance(&self, id: &ContractId) -> Result<u64> {
        self.query(TRANSFER_CONTRACT, "contract_balance", id)
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
    pub fn new_block_session(
        &self,
        block_height: u64,
        commit: [u8; 32],
    ) -> Result<Session, StateTransitionError> {
        let mut session = self._session(block_height, None).map_err(|err| {
            StateTransitionError::SessionError(format!("{err}"))
        })?;

        if session.root() != commit {
            return Err(StateTransitionError::TipChanged);
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

    pub fn set_current_commit(&self, commit: [u8; 32]) {
        let mut tip = self.tip.write();
        tip.current = commit;
    }

    pub fn commit_session(&self, session: Session) -> Result<()> {
        let commit = session.commit()?;
        self.set_current_commit(commit);

        Ok(())
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

    /// Computes the state transition for a given block by executing
    /// transactions and applying rewards and slashes
    #[allow(clippy::too_many_arguments)]
    pub fn execute_state_transition(
        &self,
        prev_state: [u8; 32],
        blk: &Block,
        cert_voters: &[Voter],
    ) -> Result<
        (
            Vec<SpentTransaction>,
            StateTransitionResult,
            Vec<ContractTxEvent>,
            Session,
        ),
        StateTransitionError,
    > {
        let block_height = blk.header().height;
        let block_hash = blk.header().hash;
        let gas_limit = blk.header().gas_limit;
        let txs = blk.txs();

        let generator_bytes = blk.header().generator_bls_pubkey;
        let generator = BlsPublicKey::from_slice(&generator_bytes.0)
            .map_err(StateTransitionError::InvalidGenerator)?;

        let slashes = Slash::from_block(blk)
            .map_err(StateTransitionError::InvalidSlash)?;

        info!(
            event = "Executing state transition",
            height = block_height,
            block_hash = to_str(&block_hash),
            prev_state = to_str(&prev_state),
            gas_limit,
            ?slashes
        );

        // Start a VM session on top of prev_state
        let mut session =
            self.new_block_session(blk.header().height, prev_state)?;
        let execution_config = self.vm_config.to_execution_config(block_height);

        let mut gas_left = gas_limit;

        let mut spent_txs = Vec::with_capacity(txs.len());
        let mut dusk_spent = 0;

        let mut events = Vec::new();
        let mut event_bloom = Bloom::new();

        // Execute transactions
        for unspent_tx in txs {
            let tx = &unspent_tx.inner;
            let tx_id = unspent_tx.id();
            let receipt = execute(&mut session, tx, &execution_config)
                .map_err(|err| {
                    StateTransitionError::ExecutionError(format!("{err}"))
                })?;

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
            gas_left = gas_left
                .checked_sub(gas_spent)
                .ok_or(RuskError::OutOfGas)
                .map_err(|err| {
                    StateTransitionError::ExecutionError(format!("{err}"))
                })?;

            spent_txs.push(SpentTransaction {
                inner: unspent_tx.clone(),
                gas_spent,
                block_height,
                // We're currently ignoring the result of successful calls
                err: receipt.data.err().map(|e| format!("{e}")),
            });
        }

        // Apply rewards and slashes
        let coinbase_events = reward_and_slash(
            &mut session,
            block_height,
            &generator,
            cert_voters,
            dusk_spent,
            slashes,
        )
        .map_err(|err| {
            StateTransitionError::ExecutionError(format!("{err}"))
        })?;

        event_bloom.add_events(&coinbase_events);

        let coinbase_events: Vec<_> = coinbase_events
            .into_iter()
            .map(|event| ContractTxEvent {
                event: event.into(),
                origin: block_hash,
            })
            .collect();
        events.extend(coinbase_events);

        // Get new state root
        let state_root = session.root();

        Ok((
            spent_txs,
            StateTransitionResult {
                state_root,
                event_bloom: event_bloom.into(),
            },
            events,
            session,
        ))
    }
}

/// Execute rewards and slashes in a VM session.
///
/// The Trasnfer contract's note tree root is updated accordingly.
fn reward_and_slash(
    session: &mut Session,
    block_height: u64,
    generator: &BlsPublicKey,
    voters: &[Voter],
    spent_amount: Dusk,
    slashes: Vec<Slash>,
) -> Result<Vec<Event>> {
    let mut events = vec![];

    // Apply rewards
    events.extend(reward(
        session,
        block_height,
        generator,
        voters,
        spent_amount,
    )?);

    // Apply slashes
    events.extend(slash(session, slashes)?);

    // Update the note tree root in the Transfer contract
    let r = session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "update_root",
        &(),
        u64::MAX,
    )?;
    events.extend(r.events);

    Ok(events)
}

/// Apply rewards by calling the `reward` method in the Stake Contract
fn reward(
    session: &mut Session,
    block_height: u64,
    generator: &BlsPublicKey,
    voters: &[Voter],
    spent_amount: Dusk,
) -> Result<Vec<Event>> {
    // Compute base rewards
    let (dusk_reward, generator_reward, generator_extra_reward, voters_reward) =
        get_block_rewards(block_height, spent_amount);

    let voters_credits = voters
        .iter()
        .map(|(_, credits)| *credits as u64)
        .sum::<u64>();

    // Except for the genesis block, there should always be some voters
    if block_height > 1 && (voters.is_empty() || voters_credits == 0) {
        return Err(RuskError::InvalidCreditsCount(block_height, 0));
    }

    let generator_extra_reward =
        calc_generator_extra_reward(generator_extra_reward, voters_credits);

    // Split voters reward in credit quotas.
    // Each voter will get as many quotas as its credits in the committee.
    let credit_reward = voters_reward / TOTAL_COMMITTEES_CREDITS as u64;

    // Compute the number of rewards
    let mut num_rewards = 2;
    if generator_extra_reward != 0 {
        num_rewards += 1;
    }
    num_rewards += voters.len();

    // Collect individual rewards into a `rewards` vector
    let mut rewards = Vec::with_capacity(num_rewards);

    rewards.push(Reward {
        account: *generator,
        value: generator_reward,
        reason: RewardReason::GeneratorFixed,
    });

    rewards.push(Reward {
        account: *DUSK_CONSENSUS_KEY,
        value: dusk_reward,
        reason: RewardReason::Other,
    });

    if generator_extra_reward != 0 {
        rewards.push(Reward {
            account: *generator,
            value: generator_extra_reward,
            reason: RewardReason::GeneratorExtra,
        });
    }

    for (voter, voter_credits) in voters {
        let voter_pk = voter.inner();
        let voter_reward = *voter_credits as u64 * credit_reward;

        rewards.push(Reward {
            account: *voter_pk,
            value: voter_reward,
            reason: RewardReason::Voter,
        });
    }

    // Apply rewards
    let r =
        session.call::<_, ()>(STAKE_CONTRACT, "reward", &rewards, u64::MAX)?;

    Ok(r.events)
}

/// Calculates the extra reward for the block generator.
/// This reward depends on the number of extra credits (i.e., credit beyond the
/// minimum quorum threshold) included in the block attestation.
///
/// # Arguments
///
/// * `full_extra_reward` - Total available extra reward for the generator (as
///   percentage of the total block reward)
/// * `att_credits` - Total number of credits included in the block attestation
fn calc_generator_extra_reward(
    full_extra_reward: Dusk,
    att_credits: u64,
) -> u64 {
    // If all votes are included, reward the whole amount.
    // We do this check to avoid assigning less than `full_extra_reward` due
    // to loss of precision when fractioning the total reward into quotas.
    if att_credits == TOTAL_COMMITTEES_CREDITS as u64 {
        return full_extra_reward;
    }

    // The calculate the extra reward, we divide the whole amount in quotas,
    // with each quota corresponding to reward value for a single extra credit.
    let max_extra_credits = validation_extra() + ratification_extra();
    let reward_quota = full_extra_reward / max_extra_credits as u64;

    let quorum_credits = validation_quorum() + ratification_quorum();
    reward_quota * att_credits.saturating_sub(quorum_credits as u64)
}

/// Apply slashes by calling the `slash` method in the Stake Contract
fn slash(session: &mut Session, slashes: Vec<Slash>) -> Result<Vec<Event>> {
    let mut events = vec![];
    for s in slashes {
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
