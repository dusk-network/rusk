// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::services::prover::RuskProver;
use crate::transaction::SpentTransaction;

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{cmp, fs, io};

pub mod error;
pub mod services;
pub mod transaction;

use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_merkle::poseidon::Opening as PoseidonOpening;
use dusk_pki::PublicKey;
use dusk_plonk::prelude::PublicParameters;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use phoenix_core::transaction::*;
use phoenix_core::Message;
use piecrust::{Error as PiecrustError, Session, VM};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use rusk_abi::dusk::{dusk, Dusk};
use rusk_abi::{
    ContractId, StandardBufSerializer, STAKE_CONTRACT, TRANSFER_CONTRACT,
};
use rusk_profile::to_rusk_state_id_path;
use rusk_recovery_tools::provisioners::DUSK_KEY;

const A: usize = 4;

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub static PUB_PARAMS: Lazy<PublicParameters> = Lazy::new(|| unsafe {
    let pp = rusk_profile::get_common_reference_string()
        .expect("Failed to get common reference string");

    PublicParameters::from_slice_unchecked(pp.as_slice())
});

const STREAM_BUF_SIZE: usize = 64;

pub struct RuskInner {
    pub current_commit: [u8; 32],
    pub base_commit: [u8; 32],
    pub vm: VM,
}

#[derive(Clone)]
pub struct Rusk {
    inner: Arc<Mutex<RuskInner>>,
    dir: PathBuf,
    stream_buffer_size: usize,
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
            stream_buffer_size: STREAM_BUF_SIZE,
        })
    }

    pub fn execute_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
    ) -> Result<(Vec<SpentTransaction>, Vec<Transaction>, [u8; 32])> {
        let inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        let mut block_gas_left = block_gas_limit;

        let mut spent_txs = Vec::<SpentTransaction>::with_capacity(txs.len());
        let mut discarded_txs = Vec::with_capacity(txs.len());

        let mut dusk_spent = 0;

        for tx in txs {
            let (call_result, gas_spent) = match execute(
                &mut session,
                &tx,
                block_gas_left,
            ) {
                // We're currently ignoring the result of a call.
                Ok((_ret, gas_spent)) => (None, gas_spent),
                Err(err) => match err {
                    // An unspendable transaction should be discarded
                    TxError::Unspendable(_) => {
                        discarded_txs.push(tx);
                        continue;
                    }
                    // This transaction was given its own gas limit, so it
                    // should be included with the error.
                    TxError::TxLimit { err, gas_spent } => {
                        (Some(err), gas_spent)
                    }
                    // A transaction that errors due to hitting the block
                    // gas limit is not included,
                    // but not dicarded either.
                    TxError::BlockLimit(_) => continue,
                    // A transaction that hit the block gas limit after
                    // execution leaves the transaction in a spent state,
                    // therefore re-execution is required. It also is not
                    // discarded.
                    TxError::BlockLimitAfter(_) => {
                        session = rusk_abi::new_session(
                            &inner.vm,
                            current_commit,
                            block_height,
                        )?;

                        let mut block_gas_left = block_gas_limit;

                        for spent_tx in &spent_txs {
                            let gas_spent = execute(
                                    &mut session,
                                    &spent_tx.0,
                                    block_gas_left,
                                )
                                .map(|(_, gas_spent)| gas_spent)
                                .unwrap_or_else(|err| match err {
                                    TxError::TxLimit { gas_spent, .. } => {
                                        gas_spent
                                    }
                                    _ => unreachable!("Spent transactions are either succeeding or TxError"),
                                });

                            block_gas_left -= gas_spent;
                        }

                        continue;
                    }
                },
            };

            block_gas_left -= gas_spent;
            dusk_spent += gas_spent * tx.fee.gas_price;

            spent_txs.push(SpentTransaction(tx, gas_spent, call_result));

            // No need to keep executing if there is no gas left in the
            // block
            if block_gas_left == 0 {
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

        Ok((spent_txs, discarded_txs, state_root))
    }

    /// Verify the given transactions are ok.
    pub fn verify_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
    ) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
        let inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        accept(&mut session, block_height, block_gas_limit, generator, txs)
    }

    /// Accept the given transactions.
    pub fn accept_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
    ) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
        let mut inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        let (spent_txs, state_root) = accept(
            &mut session,
            block_height,
            block_gas_limit,
            generator,
            txs,
        )?;

        let commit_id = session.commit()?;
        inner.current_commit = commit_id;

        Ok((spent_txs, state_root))
    }

    /// Finalize the given transactions.
    pub fn finalize_transactions(
        &self,
        block_height: u64,
        block_gas_limit: u64,
        generator: BlsPublicKey,
        txs: Vec<Transaction>,
    ) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
        let mut inner = self.inner.lock();

        let current_commit = inner.current_commit;
        let mut session =
            rusk_abi::new_session(&inner.vm, current_commit, block_height)?;

        let (spent_txs, state_root) = accept(
            &mut session,
            block_height,
            block_gas_limit,
            generator,
            txs,
        )?;

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

        // Squash the current commit
        inner.vm.squash_commit(inner.current_commit)?;

        let commit_id_path = to_rusk_state_id_path(&self.dir);
        fs::write(commit_id_path, commit_id)?;

        inner.base_commit = commit_id;

        Ok((spent_txs, state_root))
    }

    pub fn persist_state(&self) -> Result<()> {
        Ok(())
    }

    pub fn revert(&self) -> Result<[u8; 32]> {
        let mut inner = self.inner.lock();
        inner.current_commit = inner.base_commit;
        Ok(inner.current_commit)
    }

    pub fn pre_verify(&self, tx: &Transaction) -> Result<()> {
        let existing_nullifiers = self.existing_nullifiers(&tx.nullifiers)?;

        if !existing_nullifiers.is_empty() {
            return Err(Error::RepeatingNullifiers(existing_nullifiers));
        }

        if !RuskProver::preverify(tx)? {
            return Err(Error::ProofVerification);
        }

        Ok(())
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

    /// Returns the leaves of the transfer tree in the given range.
    pub fn leaves_in_range(&self, range: Range<u64>) -> Result<Vec<TreeLeaf>> {
        self.query(TRANSFER_CONTRACT, "leaves_in_range", &range)
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
        const MAX: usize = 8; // maximum number of stakes per call
        let mut skip = 0;

        let mut provisioners = Vec::new();

        self.query_seq(
            STAKE_CONTRACT,
            "stakes",
            &(MAX, skip),
            |r: Vec<(BlsPublicKey, StakeData)>| {
                let n_stakes = r.len();
                provisioners.extend(r);

                skip += n_stakes;
                if n_stakes == 0 {
                    None
                } else {
                    Some((MAX, skip))
                }
            },
        )?;

        Ok(provisioners)
    }

    /// Returns the keys allowed to stake.
    pub fn stake_allowlist(&self) -> Result<Vec<BlsPublicKey>> {
        self.query(STAKE_CONTRACT, "allowlist", &())
    }

    /// Returns the keys that own the stake contract.
    pub fn stake_owners(&self) -> Result<Vec<BlsPublicKey>> {
        self.query(STAKE_CONTRACT, "owners", &())
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
        session.set_point_limit(u64::MAX);

        let mut result = session.call(contract_id, call_name, call_arg)?;

        while let Some(call_arg) = closure(result) {
            result = session.call(contract_id, call_name, &call_arg)?;
        }

        Ok(session.call(contract_id, call_name, call_arg)?)
    }
}

fn accept(
    session: &mut Session,
    block_height: u64,
    block_gas_limit: u64,
    generator: BlsPublicKey,
    txs: Vec<Transaction>,
) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
    let mut block_gas_left = block_gas_limit;

    let mut spent_txs = Vec::with_capacity(txs.len());
    let mut dusk_spent = 0;

    for tx in txs {
        let (call_result, gas_spent) =
            match execute(session, &tx, block_gas_left) {
                // We're currently ignoring the result of a call.
                Ok((_ret, gas_spent)) => (None, gas_spent),
                Err(err) => match err {
                    TxError::TxLimit { err, gas_spent } => {
                        (Some(err), gas_spent)
                    }
                    TxError::Unspendable(err)
                    | TxError::BlockLimit(err)
                    | TxError::BlockLimitAfter(err) => {
                        return Err(err.into());
                    }
                },
            };

        dusk_spent += gas_spent * tx.fee.gas_price;
        block_gas_left = block_gas_left
            .checked_sub(gas_spent)
            .ok_or(Error::OutOfGas)?;

        spent_txs.push(SpentTransaction(tx, gas_spent, call_result));
    }

    reward_and_update_root(session, block_height, dusk_spent, generator)?;
    let state_root = session.root();

    Ok((spent_txs, state_root))
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
    tx: &Transaction,
    block_gas_left: u64,
) -> Result<(Vec<u8>, u64), TxError> {
    let gas_for_spend = spent_gas_per_input(tx.nullifiers.len());

    // If the gas given is less than the amount the node charges per input, then
    // the transaction is unspendable.
    if tx.fee.gas_limit < gas_for_spend {
        return Err(TxError::Unspendable(PiecrustError::OutOfPoints));
    }

    // If the gas to spend is more than the amount remaining in a block, then
    // the transaction can't be spent at this spot in the block.
    if block_gas_left < gas_for_spend {
        return Err(TxError::BlockLimit(PiecrustError::OutOfPoints));
    }

    // Spend the transaction. If this error the transaction is unspendable.
    session.set_point_limit(u64::MAX);
    session
        .call(TRANSFER_CONTRACT, "spend", tx)
        .map_err(TxError::Unspendable)?;

    let mut gas_spent = gas_for_spend;

    let block_gas_left = block_gas_left - gas_spent;
    let tx_gas_left = tx.fee.gas_limit - gas_spent;

    let res = tx
        .call
        .as_ref()
        .map(|(contract_id_bytes, fn_name, fn_data)| {
            let contract_id = ContractId::from_bytes(*contract_id_bytes);
            let gas_left = cmp::min(block_gas_left, tx_gas_left);

            session.set_point_limit(gas_left);

            match session.call_raw(contract_id, fn_name, fn_data.clone()) {
                Ok(vec) => {
                    gas_spent += session.spent();
                    Ok(vec)
                }
                Err(err) => match err {
                    err @ PiecrustError::OutOfPoints => {
                        // If the transaction failed with an OUT_OF_GAS, and
                        // we're using the block gas remaining as a limit, then
                        // we can't be sure that the transaction would fail if
                        // it was given the full gas it gave as a limit.
                        if gas_left == block_gas_left {
                            return Err(TxError::BlockLimitAfter(err));
                        }

                        // Otherwise we should spend the maximum available gas
                        gas_spent = tx.fee.gas_limit;
                        Err(TxError::TxLimit { gas_spent, err })
                    }
                    err => {
                        // On any other error we should spent the maximum
                        // available gas
                        gas_spent = tx.fee.gas_limit;
                        Err(TxError::TxLimit { gas_spent, err })
                    }
                },
            }
        });

    // Refund the appropriate amount to the transaction. This call is guaranteed
    // to never error. If it does, then a programming error has occurred. As
    // such, the call to `Result::expect` is warranted.
    session.set_point_limit(u64::MAX);
    let _: () = session
        .call(TRANSFER_CONTRACT, "refund", &(tx.fee, gas_spent))
        .expect("Refunding must succeed");

    res.map(|res| res.map(|data| (data, gas_spent)))
        .unwrap_or(Ok((vec![], gas_spent)))
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
    generator: BlsPublicKey,
) -> Result<()> {
    let (dusk_value, generator_value) =
        coinbase_value(block_height, dusk_spent);

    session.set_point_limit(u64::MAX);

    session.call(STAKE_CONTRACT, "reward", &(*DUSK_KEY, dusk_value))?;
    session.call(STAKE_CONTRACT, "reward", &(generator, generator_value))?;
    session.call(TRANSFER_CONTRACT, "update_root", &())?;

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
const fn emission_amount(block_height: u64) -> Dusk {
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
