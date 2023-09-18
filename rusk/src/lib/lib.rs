// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![feature(lazy_cell)]

use crate::error::Error;

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{mpsc, Arc, LazyLock};
use std::{cmp, fs, io};

pub mod chain;
pub mod error;
pub mod http;
pub mod prover;
mod vm;

use dusk_bytes::DeserializableSlice;
use futures::Stream;
use tokio::spawn;
use tracing::{error, info};

use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_consensus::contract_state::VerificationOutput;
use dusk_pki::{PublicKey, ViewKey};
use node_data::ledger::{SpentTransaction, Transaction};
use parking_lot::{Mutex, MutexGuard};
use phoenix_core::transaction::{StakeData, TreeLeaf, TRANSFER_TREE_DEPTH};
use phoenix_core::{Message, Note, Transaction as PhoenixTransaction};
use poseidon_merkle::Opening as PoseidonOpening;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use rusk_abi::dusk::{dusk, Dusk};
use rusk_abi::{
    CallReceipt, ContractId, Error as PiecrustError, Event, Session,
    StandardBufSerializer, STAKE_CONTRACT, TRANSFER_CONTRACT, VM,
};
use rusk_profile::to_rusk_state_id_path;
use sha3::{Digest, Sha3_256};

const A: usize = 4;

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub type StoredNote = (Note, u64);

pub type GetNotesStream = Pin<Box<dyn Stream<Item = StoredNote> + Send>>;

pub static DUSK_KEY: LazyLock<BlsPublicKey> = LazyLock::new(|| {
    let dusk_cpk_bytes = include_bytes!("../assets/dusk.cpk");
    BlsPublicKey::from_slice(dusk_cpk_bytes)
        .expect("Dusk consensus public key to be valid")
});

pub struct RuskInner {
    pub current_commit: [u8; 32],
    pub base_commit: [u8; 32],
    pub vm: VM,
}

#[derive(Clone)]
pub struct Rusk {
    inner: Arc<Mutex<RuskInner>>,
    dir: PathBuf,
}

impl Rusk {
    pub fn new<P: AsRef<Path>>(dir: P) -> Result<Self> {
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

        let vm = rusk_abi::new_vm(dir)?;

        let inner = Arc::new(Mutex::new(RuskInner {
            current_commit: base_commit,
            base_commit,
            vm,
        }));

        Ok(Self {
            inner,
            dir: dir.into(),
        })
    }

    pub fn execute_transactions<I: Iterator<Item = Transaction>>(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: &BlsPublicKey,
        txs: I,
    ) -> Result<(Vec<SpentTransaction>, Vec<Transaction>, VerificationOutput)>
    {
        let inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        let mut block_gas_left = block_gas_limit;

        let mut spent_txs = Vec::<SpentTransaction>::new();
        let mut discarded_txs = vec![];

        let mut dusk_spent = 0;

        let mut event_hasher = Sha3_256::new();

        for unspent_tx in txs {
            let tx = unspent_tx.inner.clone();
            let receipt = execute(&mut session, &tx, block_gas_left);

            let (call_result, gas_spent) = match receipt.data {
                // We're currently ignoring the successful result of a call
                Ok(_) => {
                    for event in receipt.events {
                        update_hasher(&mut event_hasher, event);
                    }
                    (None, receipt.points_spent)
                }
                // This transaction was given its own gas limit, so it should be
                // included with the error.
                Err(TxError::TxLimit { err, gas_spent }) => {
                    for event in receipt.events {
                        update_hasher(&mut event_hasher, event);
                    }
                    (Some(err), gas_spent)
                }
                // An unspendable transaction should be discarded
                Err(TxError::Unspendable(_)) => {
                    discarded_txs.push(unspent_tx);
                    continue;
                }
                // A transaction that errors due to hitting the block gas limit
                // is not included, but not dicarded either.
                Err(TxError::BlockLimit(_)) => continue,
                // A transaction that hit the block gas limit after execution
                // leaves the transaction in a spent state, therefore
                // re-execution is required. It also is not discarded.
                Err(TxError::BlockLimitAfter(_)) => {
                    session = rusk_abi::new_session(
                        &inner.vm,
                        current_commit,
                        block_height,
                    )?;

                    let mut block_gas_left = block_gas_limit;

                    for spent_tx in &spent_txs {
                        let receipt = execute(
                            &mut session,
                            &spent_tx.inner.inner,
                            block_gas_left,
                        );

                        // We know these transactions were either spent or
                        // erroring with `TxLimit` so we don't need to check.
                        block_gas_left -= receipt.points_spent;
                    }

                    continue;
                }
            };

            block_gas_left -= gas_spent;
            dusk_spent += gas_spent * tx.fee.gas_price;

            spent_txs.push(SpentTransaction {
                inner: unspent_tx.clone(),
                gas_spent,
                block_height,
                err: call_result.map(|e| format!("{e:?}")),
            });

            // Stop executing if there is no gas left for a normal transfer
            if block_gas_left < GAS_PER_INPUT {
                break;
            }
        }

        reward_and_update_root(
            &mut session,
            block_height,
            dusk_spent,
            generator,
        )?;

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
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        accept(&mut session, block_height, block_gas_limit, generator, txs)
    }

    /// Accept the given transactions.
    ///
    ///   * `consistency_check` - represents a state_root, the caller expects to
    ///   be returned on successful transactions execution. Passing a None
    ///   value disables the check.
    pub fn accept_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
        consistency_check: Option<VerificationOutput>,
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let mut inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        let (spent_txs, verification_output) = accept(
            &mut session,
            block_height,
            block_gas_limit,
            &generator,
            &txs[..],
        )?;

        if let Some(expected_verification) = consistency_check {
            if expected_verification != verification_output {
                // Drop the session if the resulting is inconsistent
                // with the callers one.
                return Err(Error::InconsistentState(verification_output));
            }
        }

        let commit_id = session.commit()?;
        inner.current_commit = commit_id;

        Ok((spent_txs, verification_output))
    }

    /// Finalize the given transactions.
    ///
    /// * `consistency_check` - represents a state_root, the caller expects to
    ///   be returned on successful transactions execution. Passing None value
    ///   disables the check.
    pub fn finalize_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
        consistency_check: Option<VerificationOutput>,
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let mut inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        let (spent_txs, verification_output) = accept(
            &mut session,
            block_height,
            block_gas_limit,
            &generator,
            &txs[..],
        )?;

        if let Some(expected_verification) = consistency_check {
            if expected_verification != verification_output {
                // Drop the session if the result state root is inconsistent
                // with the callers one.
                return Err(Error::InconsistentState(verification_output));
            }
        }

        let commit_id = session.commit()?;
        inner.current_commit = commit_id;

        // Delete all commits except the previous base commit, and the current
        // commit
        let mut delete_commits = inner.vm.commits();
        delete_commits
            .retain(|c| c != &inner.current_commit && c != &inner.base_commit);
        for commit in delete_commits {
            inner.vm.delete_commit(commit)?;
        }

        let commit_id_path = to_rusk_state_id_path(&self.dir);
        fs::write(commit_id_path, commit_id)?;

        inner.base_commit = commit_id;

        Ok((spent_txs, verification_output))
    }

    pub fn persist_state(&self) -> Result<()> {
        Ok(())
    }

    pub fn revert(&self) -> Result<[u8; 32]> {
        let mut inner = self.inner.lock();
        inner.current_commit = inner.base_commit;
        Ok(inner.current_commit)
    }

    /// Perform an action with the underlying data structure.
    pub fn with_inner<'a, F, T>(&'a self, closure: F) -> T
    where
        F: FnOnce(MutexGuard<'a, RuskInner>) -> T,
    {
        let inner = self.inner.lock();
        closure(inner)
    }

    /// Get the base root.
    pub fn base_root(&self) -> [u8; 32] {
        let inner = self.inner.lock();
        inner.base_commit
    }

    /// Get the current state root.
    pub fn state_root(&self) -> [u8; 32] {
        let inner = self.inner.lock();
        inner.current_commit
    }

    /// Performs a feeder query returning the leaves of the transfer tree
    /// starting from the given height. The function will block while executing,
    /// and the results of the query will be passed through the `receiver`
    /// counterpart of the given `sender`.
    ///
    /// The receiver of the leaves is responsible for deserializing the leaves
    /// appropriately - i.e. using `rkyv`.
    pub fn leaves_from_height(
        &self,
        height: u64,
        sender: mpsc::Sender<Vec<u8>>,
    ) -> Result<()> {
        self.feeder_query(
            TRANSFER_CONTRACT,
            "leaves_from_height",
            &height,
            sender,
        )
    }

    /// Returns the nullifiers that already exist from a list of given
    /// `nullifiers`.
    pub fn existing_nullifiers(
        &self,
        nullifiers: &Vec<BlsScalar>,
    ) -> Result<Vec<BlsScalar>> {
        self.query(TRANSFER_CONTRACT, "existing_nullifiers", nullifiers)
    }

    /// Returns the root of the transfer tree.
    pub fn tree_root(&self) -> Result<BlsScalar> {
        info!("Received tree_root request");
        self.query(TRANSFER_CONTRACT, "root", &())
    }

    /// Returns the opening of the transfer tree at the given position.
    pub fn tree_opening(
        &self,
        pos: u64,
    ) -> Result<Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>>> {
        self.query(TRANSFER_CONTRACT, "opening", &pos)
    }

    /// Returns the "transparent" balance of the given module.
    pub fn module_balance(&self, contract: ContractId) -> Result<u64> {
        self.query(TRANSFER_CONTRACT, "module_balance", &contract)
    }

    /// Returns the message mapped to the given module and public key.
    pub fn module_message(
        &self,
        contract: ContractId,
        pk: PublicKey,
    ) -> Result<Option<Message>> {
        self.query(TRANSFER_CONTRACT, "message", &(contract, pk))
    }

    /// Returns data about the stake of the given key.
    pub fn stake(&self, pk: BlsPublicKey) -> Result<Option<StakeData>> {
        self.query(STAKE_CONTRACT, "get_stake", &pk)
    }

    /// Returns the stakes.
    pub fn provisioners(&self) -> Result<Vec<(BlsPublicKey, StakeData)>> {
        let (sender, receiver) = mpsc::channel();
        self.feeder_query(STAKE_CONTRACT, "stakes", &(), sender)?;
        Ok(receiver
            .into_iter()
            .map(|bytes| {
                rkyv::from_bytes::<(BlsPublicKey, StakeData)>(&bytes).expect(
                    "The contract should only return (pk, stake_data) tuples",
                )
            })
            .collect())
    }

    /// Returns the keys allowed to stake.
    pub fn stake_allowlist(&self) -> Result<Vec<BlsPublicKey>> {
        self.query(STAKE_CONTRACT, "allowlist", &())
    }

    /// Returns the keys that own the stake contract.
    pub fn stake_owners(&self) -> Result<Vec<BlsPublicKey>> {
        self.query(STAKE_CONTRACT, "owners", &())
    }

    pub fn query_raw<S, V>(
        &self,
        contract_id: ContractId,
        fn_name: S,
        fn_arg: V,
    ) -> Result<Vec<u8>>
    where
        S: AsRef<str>,
        V: Into<Vec<u8>>,
    {
        let inner = self.inner.lock();

        // For queries we set a point limit of effectively infinite and a block
        // height of zero since this doesn't affect the result.
        let current_commit = inner.current_commit;
        let mut session = rusk_abi::new_session(&inner.vm, current_commit, 0)?;

        session
            .call_raw(contract_id, fn_name.as_ref(), fn_arg, u64::MAX)
            .map(|receipt| receipt.data)
            .map_err(Into::into)
    }

    fn query<A, R>(
        &self,
        contract_id: ContractId,
        call_name: &str,
        call_arg: &A,
    ) -> Result<R>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let mut results = Vec::with_capacity(1);
        self.query_seq(contract_id, call_name, call_arg, |r| {
            results.push(r);
            None
        })?;
        Ok(results.pop().unwrap())
    }

    fn query_seq<A, R, F>(
        &self,
        contract_id: ContractId,
        call_name: &str,
        call_arg: &A,
        mut closure: F,
    ) -> Result<()>
    where
        F: FnMut(R) -> Option<A>,
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let inner = self.inner.lock();

        // For queries we set a point limit of effectively infinite and a block
        // height of zero since this doesn't affect the result.
        let current_commit = inner.current_commit;
        let mut session = rusk_abi::new_session(&inner.vm, current_commit, 0)?;

        let mut result = session
            .call(contract_id, call_name, call_arg, u64::MAX)?
            .data;

        while let Some(call_arg) = closure(result) {
            result = session
                .call(contract_id, call_name, &call_arg, u64::MAX)?
                .data;
        }

        session.call::<_, ()>(contract_id, call_name, call_arg, u64::MAX)?;

        Ok(())
    }

    pub fn feeder_query<A>(
        &self,
        contract_id: ContractId,
        call_name: &str,
        call_arg: &A,
        feeder: mpsc::Sender<Vec<u8>>,
    ) -> Result<()>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
    {
        let inner = self.inner.lock();

        // For queries we set a point limit of effectively infinite and a block
        // height of zero since this doesn't affect the result.
        let current_commit = inner.current_commit;
        let mut session = rusk_abi::new_session(&inner.vm, current_commit, 0)?;

        session.feeder_call::<_, ()>(
            contract_id,
            call_name,
            call_arg,
            feeder,
        )?;

        Ok(())
    }

    pub fn feeder_query_raw<S, V>(
        &self,
        contract_id: ContractId,
        call_name: S,
        call_arg: V,
        feeder: mpsc::Sender<Vec<u8>>,
    ) -> Result<()>
    where
        S: AsRef<str>,
        V: Into<Vec<u8>>,
    {
        let inner = self.inner.lock();

        // For queries we set a point limit of effectively infinite and a block
        // height of zero since this doesn't affect the result.
        let current_commit = inner.current_commit;
        let mut session = rusk_abi::new_session(&inner.vm, current_commit, 0)?;

        session.feeder_call_raw(
            contract_id,
            call_name.as_ref(),
            call_arg,
            feeder,
        )?;

        Ok(())
    }

    pub async fn get_notes(
        &self,
        vk: &[u8],
        height: u64,
    ) -> Result<GetNotesStream, Error> {
        info!("Received GetNotes request");

        let vk = match vk.is_empty() {
            false => {
                let vk =
                    ViewKey::from_slice(vk).map_err(Error::Serialization)?;
                Some(vk)
            }
            true => None,
        };

        let (sender, receiver) = mpsc::channel();

        // Clone rusk and move it to the thread
        let rusk = self.clone();

        // Spawn a task responsible for running the feeder query.
        spawn(async move {
            if let Err(err) = rusk.leaves_from_height(height, sender) {
                error!("GetNotes errored: {err}");
            }
        });

        // Make a stream from the receiver and map the elements to be the
        // expected output
        let stream =
            tokio_stream::iter(receiver.into_iter().filter_map(move |bytes| {
                let leaf = rkyv::from_bytes::<TreeLeaf>(&bytes)
                    .expect("The contract should always return valid leaves");
                match &vk {
                    Some(vk) => vk
                        .owns(&leaf.note)
                        .then_some((leaf.note, leaf.block_height)),
                    None => Some((leaf.note, leaf.block_height)),
                }
            }));

        Ok(Box::pin(stream) as GetNotesStream)
    }
}

fn accept(
    session: &mut Session,
    block_height: u64,
    block_gas_limit: u64,
    generator: &BlsPublicKey,
    txs: &[Transaction],
) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
    let mut block_gas_left = block_gas_limit;

    let mut spent_txs = Vec::with_capacity(txs.len());
    let mut dusk_spent = 0;

    let mut event_hasher = Sha3_256::new();

    for unspent_tx in txs {
        let tx = &unspent_tx.inner;
        let receipt = execute(session, tx, block_gas_left);

        let (call_result, gas_spent) = match receipt.data {
            Ok(_) => {
                for event in receipt.events {
                    update_hasher(&mut event_hasher, event);
                }
                (None, receipt.points_spent)
            }
            Err(TxError::TxLimit { err, gas_spent }) => {
                for event in receipt.events {
                    update_hasher(&mut event_hasher, event);
                }
                (Some(err), gas_spent)
            }
            Err(
                TxError::Unspendable(err)
                | TxError::BlockLimit(err)
                | TxError::BlockLimitAfter(err),
            ) => {
                return Err(err.into());
            }
        };

        dusk_spent += gas_spent * tx.fee.gas_price;
        block_gas_left = block_gas_left
            .checked_sub(gas_spent)
            .ok_or(Error::OutOfGas)?;

        spent_txs.push(SpentTransaction {
            inner: unspent_tx.clone(),
            gas_spent,
            block_height,

            err: call_result.map(|e| format!("{e:?}")),
        });
    }

    reward_and_update_root(session, block_height, dusk_spent, generator)?;

    let state_root = session.root();
    let event_hash = event_hasher.finalize().into();

    Ok((
        spent_txs,
        VerificationOutput {
            state_root,
            event_hash,
        },
    ))
}

/// Executes a transaction, returning the result of the call and the gas spent.
/// The following steps are executed:
///
/// 0. Pre-flight checks, i.e. the transaction gas limit must be at least the
///    same as what is minimally charged for a transaction of its type, and the
///    transaction must fit in the remaining block gas.
///
/// 1. Call the "spend" function on the transfer contract with unlimited gas. If
///    this fails, the transaction should be considered invalid, or unspendable,
///    and an error is returned.
///
/// 2. If the transaction includes a contract call, execute it with the gas
///    limit given in the transaction, or with the block gas remaining,
///    whichever is smallest. If this fails with an out of gas, two possible
///    things happen:
///        * We use the transaction gas limit and will treat this as any other
///          transaction.
///        * We used the block gas remaining and can't be sure of what to do. In
///          this case we return early with an [TxError::BlockLimitAfter], since
///          we are in a bad state, and can't be sure of what to do.
///    For any other transaction error we proceed to step 3.
///
/// 3. Call the "refund" function on the transfer contract with unlimited gas.
///    The amount charged depends on if the transaction has executed a call or
///    not. If it has there are two cases:
///        * The call succeeded and the transaction will be charged for gas used
///          plus the amount charged by a transaction of its type.
///        * The call errored and the transaction will be charged the full gas
///          given.
///    If the transaction has not executed a call only be the amount charged for
///    a transaction of its type.
fn execute(
    session: &mut Session,
    tx: &PhoenixTransaction,
    block_gas_left: u64,
) -> CallReceipt<Result<Option<Vec<u8>>, TxError>> {
    let gas_for_spend = spent_gas_per_input(tx.nullifiers.len());

    let mut receipt = CallReceipt {
        points_spent: 0,
        points_limit: tx.fee.gas_limit,
        events: vec![],
        data: Ok(None),
    };

    // If the gas given is less than the amount the node charges per input, then
    // the transaction is unspendable.
    if tx.fee.gas_limit < gas_for_spend {
        receipt.data = Err(TxError::Unspendable(PiecrustError::OutOfPoints));
        return receipt;
    }

    // If the gas to spend is more than the amount remaining in a block, then
    // the transaction can't be spent at this spot in the block.
    if block_gas_left < gas_for_spend {
        receipt.data = Err(TxError::BlockLimit(PiecrustError::OutOfPoints));
        return receipt;
    }

    // Spend the transaction. If this errors the transaction is unspendable.
    match session.call::<_, ()>(TRANSFER_CONTRACT, "spend", tx, u64::MAX) {
        Ok(spend_receipt) => {
            receipt.points_spent += gas_for_spend;
            receipt.events.extend(spend_receipt.events);
        }
        Err(err) => {
            receipt.data = Err(TxError::Unspendable(err));
            return receipt;
        }
    };

    let block_gas_left = block_gas_left - gas_for_spend;
    let tx_gas_left = tx.fee.gas_limit - gas_for_spend;

    if let Some((contract_id_bytes, fn_name, fn_data)) = &tx.call {
        let contract_id = ContractId::from_bytes(*contract_id_bytes);
        let gas_left = cmp::min(block_gas_left, tx_gas_left);

        match session.call_raw(contract_id, fn_name, fn_data.clone(), gas_left)
        {
            Ok(r) => {
                receipt.points_spent += r.points_spent;
                receipt.events.extend(r.events);
                receipt.data = Ok(Some(r.data));
            }
            Err(err) => match err {
                err @ PiecrustError::OutOfPoints => {
                    // If the transaction failed with an OUT_OF_GAS, and
                    // we're using the block gas remaining as a limit, then
                    // we can't be sure that the transaction would fail if
                    // it was given the full gas it gave as a limit.
                    if gas_left == block_gas_left {
                        receipt.data = Err(TxError::BlockLimitAfter(err));
                        return receipt;
                    } else {
                        // Otherwise we should spend the maximum available gas
                        receipt.points_spent = tx.fee.gas_limit;
                        receipt.data = Err(TxError::TxLimit {
                            gas_spent: receipt.points_spent,
                            err,
                        });
                    }
                }
                err => {
                    // On any other error we should spent the maximum
                    // available gas
                    receipt.points_spent = tx.fee.gas_limit;
                    receipt.data = Err(TxError::TxLimit {
                        gas_spent: receipt.points_spent,
                        err,
                    })
                }
            },
        }
    }

    // Refund the appropriate amount to the transaction. This call is guaranteed
    // to never error. If it does, then a programming error has occurred. As
    // such, the call to `Result::expect` is warranted.
    let refund_receipt = session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &(tx.fee, receipt.points_spent),
            u64::MAX,
        )
        .expect("Refunding must succeed");

    receipt.events.extend(refund_receipt.events);

    receipt
}

fn update_hasher(hasher: &mut Sha3_256, event: Event) {
    hasher.update(event.source.as_bytes());
    hasher.update(event.topic.as_bytes());
    hasher.update(event.data);
}

/// The gas charged per input of a transaction.
pub const GAS_PER_INPUT: u64 = 1_000_000;

/// The gas charged given the number of inputs of a transaction.
const fn spent_gas_per_input(n_inputs: usize) -> u64 {
    n_inputs as u64 * GAS_PER_INPUT
}

/// The error returned when executing a transaction.
enum TxError {
    /// A transaction can't be spent.
    Unspendable(PiecrustError),
    /// The error was produced by executing the transaction's call with its own
    /// given gas limit.
    TxLimit { gas_spent: u64, err: PiecrustError },
    /// The error was produced by executing the transaction's call with the
    /// remaining block gas limit.
    BlockLimit(PiecrustError),
    /// The error was produced by executing the transaction's call with the
    /// remaining block gas limit, and after execution of a call.
    BlockLimitAfter(PiecrustError),
}

fn reward_and_update_root(
    session: &mut Session,
    block_height: u64,
    dusk_spent: Dusk,
    generator: &BlsPublicKey,
) -> Result<()> {
    let (dusk_value, generator_value) =
        coinbase_value(block_height, dusk_spent);

    session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(*DUSK_KEY, dusk_value),
        u64::MAX,
    )?;
    session.call::<_, ()>(
        STAKE_CONTRACT,
        "reward",
        &(*generator, generator_value),
        u64::MAX,
    )?;

    session.call::<_, ()>(TRANSFER_CONTRACT, "update_root", &(), u64::MAX)?;

    Ok(())
}

/// Calculates the value that the coinbase notes should contain.
///
/// 90% of the total value goes to the generator (rounded up).
/// 10% of the total value goes to the Dusk address (rounded down).
const fn coinbase_value(block_height: u64, dusk_spent: u64) -> (Dusk, Dusk) {
    let value = emission_amount(block_height) + dusk_spent;

    let dusk_value = value / 10;
    let generator_value = value - dusk_value;

    (dusk_value, generator_value)
}

/// This implements the emission schedule described in the economic paper.
pub const fn emission_amount(block_height: u64) -> Dusk {
    match block_height {
        1..=12_500_000 => dusk(16.0),
        12_500_001..=18_750_000 => dusk(12.8),
        18_750_001..=25_000_000 => dusk(9.6),
        25_000_001..=31_250_000 => dusk(8.0),
        31_250_001..=37_500_000 => dusk(6.4),
        37_500_001..=43_750_000 => dusk(4.8),
        43_750_001..=50_000_000 => dusk(3.2),
        50_000_001..=62_500_000 => dusk(1.6),
        _ => dusk(0.0),
    }
}
