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
use std::{fs, io};

pub mod chain;
pub mod error;
pub mod http;
pub mod verifier;
mod version;
mod vm;

use dusk_bytes::DeserializableSlice;
use futures::Stream;
use tokio::spawn;
use tracing::{error, info};

use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_consensus::operations::VerificationOutput;
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
    CallReceipt, ContractError, ContractId, Error as PiecrustError, Event,
    Session, StandardBufSerializer, STAKE_CONTRACT, TRANSFER_CONTRACT, VM,
};
use rusk_profile::to_rusk_state_id_path;
use sha3::{Digest, Sha3_256};

pub use version::{VERSION, VERSION_BUILD};
pub const MINIMUM_STAKE: Dusk = dusk(1000.0);

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
        missed_generators: &[BlsPublicKey],
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
            match execute(&mut session, &tx) {
                Ok(receipt) => {
                    let gas_spent = receipt.gas_spent;

                    // If the transaction went over the block gas limit we
                    // re-execute all spent transactions. We don't discard the
                    // transaction, since it is technically valid.
                    if gas_spent > block_gas_left {
                        session = rusk_abi::new_session(
                            &inner.vm,
                            current_commit,
                            block_height,
                        )?;

                        for spent_tx in &spent_txs {
                            // We know these transactions were correctly
                            // executed before, so we don't bother checking.
                            let _ =
                                execute(&mut session, &spent_tx.inner.inner);
                        }

                        continue;
                    }

                    for event in receipt.events {
                        update_hasher(&mut event_hasher, event);
                    }

                    block_gas_left -= gas_spent;
                    dusk_spent += gas_spent * tx.fee.gas_price;

                    spent_txs.push(SpentTransaction {
                        inner: unspent_tx.clone(),
                        gas_spent,
                        block_height,
                        // We're currently ignoring the result of successful
                        // calls
                        err: receipt.data.err().map(|e| format!("{e}")),
                    });
                }
                Err(_) => {
                    // An unspendable transaction should be discarded
                    discarded_txs.push(unspent_tx);
                    continue;
                }
            }
        }

        reward_slash_and_update_root(
            &mut session,
            block_height,
            dusk_spent,
            generator,
            missed_generators,
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
        missed_generators: &[BlsPublicKey],
    ) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
        let inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        accept(
            &mut session,
            block_height,
            block_gas_limit,
            generator,
            txs,
            missed_generators,
        )
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
        missed_generators: &[BlsPublicKey],
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
            missed_generators,
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
        missed_generators: &[BlsPublicKey],
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
            missed_generators,
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
        delete_commits.retain(|c| {
            c != &inner.current_commit
                && c != &inner.base_commit
                && c != &current_commit
        });
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
            None,
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
        base_commit: Option<[u8; 32]>,
    ) -> Result<()>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
    {
        let inner = self.inner.lock();

        // For queries we set a point limit of effectively infinite and a block
        // height of zero since this doesn't affect the result.
        let current_commit = base_commit.unwrap_or(inner.current_commit);
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
    missed_generators: &[BlsPublicKey],
) -> Result<(Vec<SpentTransaction>, VerificationOutput)> {
    let mut block_gas_left = block_gas_limit;

    let mut spent_txs = Vec::with_capacity(txs.len());
    let mut dusk_spent = 0;

    let mut event_hasher = Sha3_256::new();

    for unspent_tx in txs {
        let tx = &unspent_tx.inner;
        let receipt = execute(session, tx)?;

        for event in receipt.events {
            update_hasher(&mut event_hasher, event);
        }
        let gas_spent = receipt.gas_spent;

        dusk_spent += gas_spent * tx.fee.gas_price;
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

    reward_slash_and_update_root(
        session,
        block_height,
        dusk_spent,
        generator,
        missed_generators,
    )?;

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

/// Executes a transaction, returning the receipt of the call and the gas spent.
/// The following steps are performed:
///
/// 1. Call the "spend_and_execute" function on the transfer contract with
///    unlimited gas. If this fails, an error is returned. If an error is
///    returned the transaction should be considered unspendable/invalid, but no
///    re-execution of previous transactions is required.
///
/// 2. Call the "refund" function on the transfer contract with unlimited gas.
///    The amount charged depends on the gas spent by the transaction, and the
///    optional contract call in step 1.
fn execute(
    session: &mut Session,
    tx: &PhoenixTransaction,
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, PiecrustError> {
    // Spend the inputs and execute the call. If this errors the transaction is
    // unspendable.
    let mut receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        tx,
        tx.fee.gas_limit,
    )?;

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
            &(tx.fee, receipt.gas_spent),
            u64::MAX,
        )
        .expect("Refunding must succeed");

    receipt.events.extend(refund_receipt.events);

    Ok(receipt)
}

fn update_hasher(hasher: &mut Sha3_256, event: Event) {
    hasher.update(event.source.as_bytes());
    hasher.update(event.topic.as_bytes());
    hasher.update(event.data);
}

fn reward_slash_and_update_root(
    session: &mut Session,
    block_height: u64,
    dusk_spent: Dusk,
    generator: &BlsPublicKey,
    slashing: &[BlsPublicKey],
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
    let slash_amount = emission_amount(block_height);

    for to_slash in slashing {
        session.call::<_, ()>(
            STAKE_CONTRACT,
            "slash",
            &(*to_slash, slash_amount),
            u64::MAX,
        )?;
    }

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
