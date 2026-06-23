// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::Instant;
use std::{fs, io};

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_consensus::config::{
    MAX_NUMBER_OF_TRANSACTIONS, TOTAL_COMMITTEES_CREDITS, ratification_extra,
    ratification_quorum, validation_extra, validation_quorum,
};
use dusk_consensus::errors::StateTransitionError;
use dusk_consensus::operations::{
    StateTransitionData, StateTransitionResult, Voter,
};
use dusk_core::abi::{ContractId, Event};
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{
    Reward, RewardReason, STAKE_CONTRACT, StakeData, StakeKeys,
};
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::{
    TRANSFER_CONTRACT, Transaction as ProtocolTransaction,
};
use dusk_core::{BlsScalar, Dusk};
use dusk_vm::{
    CallReceipt, Error as VMError, ExecutionError, Session, VM, execute,
};
#[cfg(feature = "archive")]
use node::archive::Archive;
use node_data::events::contract::ContractTxEvent;
use node_data::ledger::{
    Block, Header, LedgerTransaction, Slash, SpentTransaction, to_str,
};
use parking_lot::RwLock;
use rkyv::Deserialize;
use rusk_profile::to_rusk_state_id_path;
use tokio::sync::broadcast;
use tracing::{info, warn};

use super::fork_policy::set_hard_fork_activations;
use super::{FEATURE_HARDFORK_BOREAS, RuskVmConfig};
use crate::bloom::Bloom;
use crate::node::driverstore::DriverStore;
use crate::node::{
    RuesEvent, Rusk, RuskTip, get_block_rewards, set_vm_host_context,
};
use crate::{DUSK_CONSENSUS_KEY, Error as RuskError, Result};

fn boreas_active(vm_config: &RuskVmConfig, block_height: u64) -> bool {
    vm_config.feature_active_at(FEATURE_HARDFORK_BOREAS, block_height)
}

impl Rusk {
    #[allow(clippy::too_many_arguments)]
    pub fn new<P, F>(
        dir: P,
        initial_header: F,
        chain_id: u8,
        vm_config: RuskVmConfig,
        min_gas_limit: u64,
        feeder_gas_limit: u64,
        event_sender: broadcast::Sender<RuesEvent>,
        #[cfg(feature = "archive")] archive: Archive,
        driver_store: DriverStore,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        F: FnOnce([u8; 32]) -> Result<Header>,
    {
        let dir = dir.as_ref();
        info!("Using state from {dir:?}");

        set_hard_fork_activations(&vm_config);

        let commit_id_path = to_rusk_state_id_path(dir);

        let base_commit_bytes = fs::read(commit_id_path)?;
        if base_commit_bytes.len() != 32 {
            return Err(io::Error::other(format!(
                "Expected commit id to have 32 bytes, got {}",
                base_commit_bytes.len()
            ))
            .into());
        }
        let mut base_commit = [0u8; 32];
        base_commit.copy_from_slice(&base_commit_bytes);
        let initial_tip = initial_header(base_commit)?;

        let mut vm = VM::new(dir)?;
        for (feat, activation) in vm_config.features() {
            let feat = feat.to_ascii_lowercase();
            if let Some(hq_name) = feat.strip_prefix("hq_") {
                vm.with_hq_activation(hq_name, activation.clone());
            }
        }

        let vm = Arc::new(vm);

        let tip = Arc::new(RwLock::new(RuskTip {
            current: initial_tip.clone(),
            base: initial_tip,
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

    pub fn create_state_transition<I: Iterator<Item = LedgerTransaction>>(
        &self,
        transition_data: &StateTransitionData,
        mut mempool_txs: I,
    ) -> Result<
        (
            Vec<SpentTransaction>,
            Vec<LedgerTransaction>,
            StateTransitionResult,
        ),
        StateTransitionError,
    > {
        let started = Instant::now();

        let block_height = transition_data.round;
        let gas_limit = self.vm_config.block_gas_limit;
        let generator = transition_data.generator.inner();
        let slashes = transition_data.slashes.clone();
        let prev_state = transition_data.prev_state_root;

        let cert_voters = &transition_data.cert_voters[..];

        let _host_query_policy_guard =
            set_vm_host_context(&self.vm_config, block_height);

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
        let boreas_active = boreas_active(&self.vm_config, block_height);
        let replay_spent_txs =
            |spent_txs: &[SpentTransaction]| -> Result<Session, StateTransitionError> {
                let mut session =
                    self.new_block_session(block_height, prev_state)?;

                if boreas_active {
                    slash(&mut session, slashes.clone()).map_err(|err| {
                        StateTransitionError::ExecutionError(format!(
                            "{err}"
                        ))
                    })?;
                }

                for spent_tx in spent_txs {
                    execute(
                        &mut session,
                        spent_tx.inner.protocol(),
                        &execution_config,
                    )
                    .map_err(|err| {
                        StateTransitionError::ExecutionError(format!(
                            "Failed replaying tx {}: {err}",
                            hex::encode(spent_tx.inner.id())
                        ))
                    })?;
                }

                Ok(session)
            };

        if boreas_active {
            // Apply slashes before transaction execution so in-block stake
            // operations cannot bypass slash accounting.
            let slash_events =
                slash(&mut session, slashes.clone()).map_err(|err| {
                    StateTransitionError::ExecutionError(format!("{err}"))
                })?;
            event_bloom.add_events(&slash_events);
        }

        // We always write the faults len in a u32
        let mut space_left = transition_data.max_txs_bytes - u32::SIZE;

        // We use the pending list to keep track of transactions whose nonce is
        // not yet valid but may become valid when the transactions using the
        // missing nonces are executed.
        // When a transaction in the pending list becomes valid (wrt the nonce)
        // it is added to the unblocked list to be processed immediately.
        // Unblocked transactions have priority over other transactions in the
        // mempool.
        let mut pending_txs: BTreeMap<
            [u8; 193],
            BTreeMap<u64, LedgerTransaction>,
        > = BTreeMap::new();

        let mut unblocked_txs = VecDeque::new();

        while let Some(unspent_tx) =
            unblocked_txs.pop_front().or_else(|| mempool_txs.next())
        {
            if let Some(timeout) = self.vm_config.generation_timeout
                && started.elapsed() > timeout
            {
                info!(
                    event = "Stop creating state transition",
                    reason = "timeout expired",
                    ?timeout
                );
                break;
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
            let ledger_tx = unspent_tx.reformat_for_ledger(block_height);
            let tx_size = ledger_tx.size();

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

            match execute(&mut session, ledger_tx.protocol(), &execution_config)
            {
                Ok(mut receipt) => {
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

                        session = replay_spent_txs(&spent_txs)?;
                        continue;
                    }

                    space_left -= tx_size;

                    // We're currently ignoring the result of successful calls
                    let error = receipt.data.err().map(|e| format!("{e}"));
                    info!(event = "Tx executed", tx_id, gas_spent, error);

                    if boreas_active {
                        receipt.events.retain(|event| !event.reverted);
                    }
                    event_bloom.add_events(&receipt.events);

                    gas_left -= gas_spent;
                    let gas_price = ledger_tx.gas_price();
                    dusk_spent += gas_spent * gas_price;

                    if let ProtocolTransaction::Moonlight(tx) =
                        ledger_tx.protocol()
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
                        inner: ledger_tx,
                        gas_spent,
                        block_height,
                        err: error,
                    });
                }
                Err(ExecutionError::NotReady) => {
                    // If the transaction panics due to a not yet valid nonce,
                    // we do not discard it.
                    // Instead, we add it to a list of pending transactions so
                    // it can be processed immediately when the nonce become
                    // valid (i.e., all transactions with
                    // the missing nonces are executed in this loop).
                    if let ProtocolTransaction::Moonlight(tx) =
                        unspent_tx.protocol()
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
                    // If the transaction panics due to a failed refund, we need
                    // to revert the state to before the
                    // transaction execution and re-execute
                    // all spent transactions
                    if let ExecutionError::FailedRefund(_) = &error {
                        session = replay_spent_txs(&spent_txs)?;
                    }

                    info!(event = "Tx discarded", tx_id, ?error);
                    // A transaction that fails as unspendable or precondition
                    // failure is discarded and does not affect the state.
                    discarded_txs.push(unspent_tx);
                    continue;
                }
            }
        }

        let reward_events = reward(
            &mut session,
            block_height,
            generator,
            cert_voters,
            dusk_spent,
        )
        .map_err(|err| {
            StateTransitionError::ExecutionError(format!("{err}"))
        })?;

        event_bloom.add_events(&reward_events);

        if !boreas_active {
            let slash_events = slash(&mut session, slashes).map_err(|err| {
                StateTransitionError::ExecutionError(format!("{err}"))
            })?;
            event_bloom.add_events(&slash_events);
        }

        let root_update_events =
            update_transfer_root(&mut session).map_err(|err| {
                StateTransitionError::ExecutionError(format!("{err}"))
            })?;
        event_bloom.add_events(&root_update_events);

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
        header: &Header,
        to_merge: Vec<[u8; 32]>,
    ) -> Result<()> {
        self.set_base_and_merge(header, to_merge)?;

        let commit_id_path = to_rusk_state_id_path(&self.dir);
        fs::write(commit_id_path, header.state_hash)?;
        Ok(())
    }

    pub fn revert(&self, header: &Header) -> Result<[u8; 32]> {
        let mut tip = self.tip.write();
        let state_hash = header.state_hash;

        let commits = self.vm.commits();
        if !commits.contains(&state_hash) {
            return Err(RuskError::CommitNotFound(state_hash));
        }

        tip.current = header.clone();
        Ok(tip.current.state_hash)
    }

    pub fn revert_to_base_root(&self) -> Result<[u8; 32]> {
        let header = self.tip.read().base.clone();
        self.revert(&header)
    }

    /// Get the base root.
    pub fn base_root(&self) -> [u8; 32] {
        self.tip.read().base.state_hash
    }

    /// Get the current state root.
    pub fn state_root(&self) -> [u8; 32] {
        self.tip.read().current.state_hash
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
        base_header: Option<&Header>,
    ) -> Result<impl Iterator<Item = (StakeKeys, StakeData)>> {
        let (sender, receiver) = mpsc::channel();
        self.feeder_query(STAKE_CONTRACT, "stakes", &(), sender, base_header)?;
        Ok(receiver.into_iter().map(|bytes| {
            let root = rkyv::check_archived_root::<(StakeKeys, StakeData)>(
                &bytes,
            )
            .expect(
                "The contract should only return (StakeKeys, StakeData) tuples",
            );
            root.deserialize(&mut rkyv::Infallible).unwrap()
        }))
    }

    /// Return the active moonlight accounts
    pub fn moonlight_accounts(
        &self,
        base_header: Option<&Header>,
    ) -> Result<impl Iterator<Item = (AccountData, BlsPublicKey)>> {
        let (sender, receiver) = mpsc::channel();
        let sync_range = (0u64, u64::MAX);
        self.feeder_query(
            TRANSFER_CONTRACT,
            "sync_accounts",
            &sync_range,
            sender,
            base_header,
        )?;

        Ok(receiver.into_iter().map(|bytes| {
            let root =
                rkyv::check_archived_root::<(AccountData, [u8; 193])>(&bytes)
                    .expect("The contract should only return (AccountData, [u8; 193]) tuples");
            let from_bytes: (AccountData, [u8; 193]) =
                root.deserialize(&mut rkyv::Infallible).unwrap();
            unsafe {
            (from_bytes.0, BlsPublicKey::from_slice_unchecked(&from_bytes.1))
            }
        }))
    }

    pub fn shade_3rd_party(&self, contract_id: ContractId) -> Result<()> {
        Ok(self.vm.remove_3rd_party(contract_id)?)
    }

    pub fn recompile_3rd_party(&self, contract_id: ContractId) -> Result<()> {
        Ok(self.vm.recompile_3rd_party(contract_id)?)
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
    /// state data before the last changes. Optionally takes a base header to
    /// query changes since a specific point in time.
    ///
    /// # Arguments
    ///
    /// - `base_header`: An optional base header indicating the starting point
    ///   for querying changes.
    ///
    /// # Returns
    ///
    /// Returns a Result containing an iterator over tuples. Each tuple consists
    /// of a `BlsPublicKey` and an optional `StakeData`, representing the
    /// state data before the last changes in the stake contract.
    pub fn last_provisioners_change(
        &self,
        base_header: Option<&Header>,
    ) -> Result<Vec<(BlsPublicKey, Option<StakeData>)>> {
        let (sender, receiver) = mpsc::channel();
        self.feeder_query(
            STAKE_CONTRACT,
            "prev_state_changes",
            &(),
            sender,
            base_header,
        )?;
        Ok(receiver.into_iter().map(|bytes| {
            let root =
                rkyv::check_archived_root::<(BlsPublicKey, Option<StakeData>)>(&bytes)
                    .expect("The contract should only return (pk, Option<stake_data>) tuples");
            root.deserialize(&mut rkyv::Infallible).unwrap()
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

    /// Opens a session for query at the provided header height.
    ///
    /// If no header is provided, the current tip header is used.
    pub(crate) fn query_session(
        &self,
        header: Option<&Header>,
    ) -> Result<Session> {
        let (block_height, state_hash) = match header {
            Some(header) => (header.height, header.state_hash),
            None => {
                let tip = self.tip.read();
                (tip.current.height, tip.current.state_hash)
            }
        };

        self._session(block_height, Some(state_hash))
    }

    /// Opens a new session with the specified block height and state root.
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
    /// - `commit`: The optional state root. If not provided, the current tip
    ///   header state root is used.
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
            tip.current.state_hash
        });

        let contain_hq_panics = /* determine if hq panics containment should be enabled */

        let session_builder = self
            .vm
            .session_builder(commit, self.chain_id, block_height)?
            .contain_hq_panics(contain_hq_panics);
        let session = self.vm.session_from_builder(session_builder)?;

        Ok(session)
    }

    pub fn set_current_header(&self, header: &Header) {
        let mut tip = self.tip.write();
        tip.current = header.clone();
    }

    pub fn commit_session(
        &self,
        session: Session,
        header: &Header,
    ) -> Result<()> {
        let commit = session.commit()?;
        if commit != header.state_hash {
            return Err(io::Error::other(format!(
                "Committed state root {} does not match header state root {}",
                to_str(&commit),
                to_str(&header.state_hash)
            ))
            .into());
        }
        self.set_current_header(header);
        Ok(())
    }

    pub(crate) fn set_base_and_merge(
        &self,
        header: &Header,
        to_merge: Vec<[u8; 32]>,
    ) -> Result<()> {
        self.tip.write().base = header.clone();
        let base = header.state_hash;
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

        let _host_query_policy_guard =
            set_vm_host_context(&self.vm_config, block_height);

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

        // Start a VM session on top of prev_state.
        let mut session =
            self.new_block_session(blk.header().height, prev_state)?;
        let execution_config = self.vm_config.to_execution_config(block_height);

        let mut gas_left = gas_limit;

        let mut spent_txs = Vec::with_capacity(txs.len());
        let mut dusk_spent = 0;

        let mut events = Vec::new();
        let mut event_bloom = Bloom::new();
        let boreas_active = boreas_active(&self.vm_config, block_height);

        if boreas_active {
            // Apply slashes before transaction execution so in-block stake
            // operations cannot bypass slash accounting.
            let slash_events =
                slash(&mut session, slashes.clone()).map_err(|err| {
                    StateTransitionError::ExecutionError(format!("{err}"))
                })?;
            event_bloom.add_events(&slash_events);

            let slash_events: Vec<_> = slash_events
                .into_iter()
                .map(|event| ContractTxEvent {
                    event: event.into(),
                    origin: block_hash,
                })
                .collect();
            events.extend(slash_events);
        }

        // Execute transactions
        for unspent_tx in txs {
            let tx = unspent_tx.protocol();
            let tx_id = unspent_tx.id();
            let mut receipt = execute(&mut session, tx, &execution_config)
                .map_err(|err| {
                    StateTransitionError::ExecutionError(format!(
                        "Tx {} is discarded {err}",
                        hex::encode(tx_id)
                    ))
                })?;

            if boreas_active {
                receipt.events.retain(|event| !event.reverted);
            }
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

            let spent = SpentTransaction {
                inner: unspent_tx.clone(),
                gas_spent,
                block_height,
                // We're currently ignoring the result of successful calls
                err: receipt.data.err().map(|e| format!("{e}")),
            };
            info!("Tx executed: gas_spent {gas_spent}, err: {:?}", spent.err);

            spent_txs.push(spent);
        }

        // Apply rewards.
        let reward_events = reward(
            &mut session,
            block_height,
            &generator,
            cert_voters,
            dusk_spent,
        )
        .map_err(|err| {
            StateTransitionError::ExecutionError(format!("{err}"))
        })?;

        event_bloom.add_events(&reward_events);

        let reward_events: Vec<_> = reward_events
            .into_iter()
            .map(|event| ContractTxEvent {
                event: event.into(),
                origin: block_hash,
            })
            .collect();
        events.extend(reward_events);

        if !boreas_active {
            // Pre-Boreas compatibility path.
            let slash_events = slash(&mut session, slashes).map_err(|err| {
                StateTransitionError::ExecutionError(format!("{err}"))
            })?;
            event_bloom.add_events(&slash_events);

            let slash_events: Vec<_> = slash_events
                .into_iter()
                .map(|event| ContractTxEvent {
                    event: event.into(),
                    origin: block_hash,
                })
                .collect();
            events.extend(slash_events);
        }

        let root_update_events =
            update_transfer_root(&mut session).map_err(|err| {
                StateTransitionError::ExecutionError(format!("{err}"))
            })?;
        event_bloom.add_events(&root_update_events);

        let root_update_events: Vec<_> = root_update_events
            .into_iter()
            .map(|event| ContractTxEvent {
                event: event.into(),
                origin: block_hash,
            })
            .collect();
        events.extend(root_update_events);

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

/// Updates the Transfer contract's note tree root.
fn update_transfer_root(session: &mut Session) -> Result<Vec<Event>> {
    let r = session.call::<_, ()>(
        TRANSFER_CONTRACT,
        "update_root",
        &(),
        u64::MAX,
    )?;
    Ok(r.events)
}

/// Apply rewards by calling the `reward` method in the Stake Contract
///
/// # Note on reward distribution and dust
///
/// The total block reward is split into a fixed generator reward, a Dusk
/// reward, a generator extra reward, and a voters reward. Due to integer
/// division when computing per-credit reward quotas, a small amount of dust
/// may be left undistributed and is effectively lost:
///
/// - **Voters reward**: divided by [`TOTAL_COMMITTEES_CREDITS`] to obtain a
///   per-credit quota. Any remainder from this division is lost, up to
///   [`TOTAL_COMMITTEES_CREDITS`] - 1 LUX (currently 127 LUX).
/// - **Generator extra reward**: divided by the maximum number of extra credits
///   to obtain a per-credit quota (see [`calc_generator_extra_reward`]). The
///   raw division remainder can be as high as `max_extra_credits - 1` LUX
///   (currently 41 LUX), but when all votes are included the generator gets the
///   full extra reward. On the proportional branch, at most 40 LUX can remain
///   undistributed.
///
/// While this dust amount is minimal, a more precise distribution mechanism
/// (e.g., assigning the remainder to the generator) could be considered in
/// the future.
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
    //
    // Note: Due to integer division, up to TOTAL_COMMITTEES_CREDITS - 1 LUX
    // (currently 127 LUX) can be lost as dust.
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

    // To calculate the extra reward, we divide the whole amount in quotas,
    // with each quota corresponding to reward value for a single extra credit.
    //
    // Note: The raw division remainder can be as high as max_extra_credits - 1
    // LUX (currently 41 LUX). However, that bound only occurs when all extra
    // credits are included, and that case returns `full_extra_reward` above.
    // On this branch, at most 40 LUX remain undistributed.
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
        let slash_type = format!("{:?}", s.r#type);
        let call = match s.r#type {
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
        };

        match call {
            Ok(r) => events.extend(r.events),
            Err(VMError::Panic(msg)) if is_missing_stake_slash_panic(&msg) => {
                warn!(
                    event = "Slash skipped",
                    reason = "missing stake for slash target",
                    slash_type = %slash_type,
                    panic = %msg
                );
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(events)
}

fn is_missing_stake_slash_panic(msg: &str) -> bool {
    msg.contains("The stake to slash should exist")
        || msg.contains("The stake to hard slash should exist")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use dusk_bytes::Serializable;
    use dusk_core::signatures::bls;
    use dusk_core::transfer::data::ContractCall;
    use dusk_core::transfer::{
        TRANSFER_CONTRACT, Transaction as ProtocolTransaction,
        TransactionFormat,
    };
    use dusk_rusk_test::common::state::{
        DEFAULT_MIN_GAS_LIMIT, header_from_root,
    };
    use rand::SeedableRng;
    use rusk_recovery_tools::state::restore_state;
    use tempfile::tempdir;
    use tokio::sync::broadcast;
    use wallet_core::transaction::moonlight;

    use super::*;
    use crate::node::{FEATURE_ABI_PUBLIC_SENDER, FEATURE_HARDFORK_AEGIS};

    const HISTORICAL_CHAIN_ID: u8 = 0x01;
    const HISTORICAL_BLOCK_HEIGHT: u64 = 2_710_377;
    const HISTORICAL_GAS_LIMIT: u64 = 0x10000000;

    fn resign_moonlight_insecure(
        tx: ProtocolTransaction,
        signer: &bls::SecretKey,
    ) -> ProtocolTransaction {
        let ProtocolTransaction::Moonlight(tx) = tx else {
            panic!("expected moonlight transaction");
        };
        let mut bytes = tx.to_var_bytes();
        let sig = signer.sign_insecure(&tx.signature_message()).to_bytes();
        let sig_start = bytes
            .len()
            .checked_sub(sig.len())
            .expect("moonlight tx must include signature bytes");
        bytes[sig_start..].copy_from_slice(&sig);

        ProtocolTransaction::Moonlight(
            dusk_core::transfer::moonlight::Transaction::from_slice(&bytes)
                .expect("re-signed moonlight transaction must deserialize"),
        )
    }

    async fn initial_state<P: AsRef<Path>>(dir: P) -> Rusk {
        let dir = dir.as_ref();
        let (_vm, _commit_id) =
            restore_state(dir).expect("historical state should restore");

        let (sender, _) = broadcast::channel(10);

        #[cfg(feature = "archive")]
        let archive_dir = tempdir().expect("archive tempdir should be created");
        #[cfg(feature = "archive")]
        let archive =
            node::archive::Archive::create_or_open(archive_dir.path()).await;

        let mut vm_config =
            RuskVmConfig::new().with_block_gas_limit(10_000_000_000);
        vm_config.with_feature(FEATURE_ABI_PUBLIC_SENDER, 1);
        vm_config.with_feature(FEATURE_HARDFORK_AEGIS, u64::MAX);
        Rusk::new(
            dir,
            |state_root| Ok(header_from_root(state_root)),
            HISTORICAL_CHAIN_ID,
            vm_config,
            DEFAULT_MIN_GAS_LIMIT,
            u64::MAX,
            sender,
            #[cfg(feature = "archive")]
            archive,
            DriverStore::new(None::<PathBuf>),
        )
        .expect("historical rusk should initialize")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn create_state_transition_rewraps_ingress_tx_to_ledger_format() {
        let tmpdir = tempdir().expect("tempdir should be created");
        let state_dir = tmpdir.path().join("state");
        let data = include_bytes!("../../../tests/assets/2710377_state.tar.gz");
        rusk_recovery_tools::state::tar::unarchive(
            &data[..],
            state_dir.as_path(),
        )
        .expect("historical state should unpack");

        let rusk = initial_state(&state_dir).await;
        let base = hex::decode(
            "53de818894cf665f1131edda3c5579ccb8736fd05c993ecb5cd16677974b088b",
        )
        .expect("historical base root should decode");
        let mut base_a = [0u8; 32];
        base_a.copy_from_slice(&base);
        rusk.set_current_header(&header_from_root(base_a));

        let mut rng = rand::rngs::StdRng::seed_from_u64(77);
        let sender_sk = bls::SecretKey::random(&mut rng);
        let sender_pk = bls::PublicKey::from(&sender_sk);
        let receiver_sk = bls::SecretKey::random(&mut rng);
        let receiver_pk = bls::PublicKey::from(&receiver_sk);

        let mut session = rusk
            .new_block_session(1, base_a)
            .expect("historical session should open");
        session
            .call::<(_, u64), ()>(
                TRANSFER_CONTRACT,
                "add_account_balance",
                &(sender_pk, 100_000_000_000_000),
                HISTORICAL_GAS_LIMIT,
            )
            .expect("sender balance should be injected");
        let header = header_from_root(session.root());
        rusk.commit_session(session, &header)
            .expect("historical funding session should commit");

        let protocol_tx = moonlight(
            &sender_sk,
            Some(receiver_pk),
            1,
            0,
            HISTORICAL_GAS_LIMIT,
            1,
            1,
            HISTORICAL_CHAIN_ID,
            None::<ContractCall>,
        )
        .expect("historical moonlight tx should build");
        let protocol_tx = resign_moonlight_insecure(protocol_tx, &sender_sk);
        let ingress_tx = LedgerTransaction::from_protocol_with_format(
            protocol_tx,
            TransactionFormat::Aegis,
        );

        let mut voters = vec![];
        for i in 0..10 {
            let sk = bls::SecretKey::random(&mut rng);
            let pk = bls::PublicKey::from(&sk);
            voters.push((node_data::bls::PublicKey::new(pk), i));
        }
        let transition_data = StateTransitionData {
            round: HISTORICAL_BLOCK_HEIGHT,
            generator: node_data::bls::PublicKey::new(*DUSK_CONSENSUS_KEY),
            slashes: vec![],
            cert_voters: voters,
            max_txs_bytes: 5_000,
            prev_state_root: rusk.state_root(),
        };

        let (spent, discarded, _) = rusk
            .create_state_transition(
                &transition_data,
                vec![ingress_tx.clone()].into_iter(),
            )
            .expect("state transition should execute");

        assert!(
            discarded.is_empty(),
            "ingress tx should execute successfully: spent={spent:?}, discarded={discarded:?}",
        );
        assert_eq!(spent.len(), 1, "exactly one tx should be sealed");
        assert_eq!(
            spent[0].inner.id(),
            ingress_tx.id(),
            "rewrapping must preserve transaction identity",
        );
        assert_eq!(
            spent[0].inner.format(),
            TransactionFormat::PreAegis,
            "pre-fork sealing must persist ledger-format bytes",
        );
        assert_ne!(
            spent[0].inner.format(),
            ingress_tx.format(),
            "pre-fork sealing must not reuse the ingress encoding as-is",
        );
    }
}
