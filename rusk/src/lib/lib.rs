// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::services::prover::RuskProver;
use crate::transaction::SpentTransaction;

use std::collections::BTreeSet;
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
use dusk_pki::PublicKey;
use dusk_plonk::prelude::PublicParameters;
use dusk_poseidon::tree::PoseidonBranch;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use phoenix_core::transaction::*;
use phoenix_core::Message;
use piecrust::{Session, VM};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use rusk_abi::dusk::{dusk, Dusk};
use rusk_abi::{ModuleError, ModuleId, RawResult, StandardBufSerializer};
use rusk_profile::to_rusk_state_id_path;
use rusk_recovery_tools::provisioners::DUSK_KEY;

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

        let mut vm = VM::new(dir)?;
        rusk_abi::register_host_queries(&mut vm);

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

        let mut session = inner.vm.session(inner.current_commit)?;
        rusk_abi::set_block_height(&mut session, block_height);

        let mut block_gas_left = block_gas_limit;

        let mut spent_txs = Vec::with_capacity(txs.len());
        let mut discarded_txs = Vec::with_capacity(txs.len());

        let mut dusk_spent = 0;

        let mut nullifiers = BTreeSet::new();

        // Here we discard transactions that:
        // - Use nullifiers that are already use by previous TXs
        // - Fail for any reason other than out of gas due to hitting the block
        //   gas limit
        'tx_loop: for tx in txs {
            for nullifier in &tx.nullifiers {
                if !nullifiers.insert(*nullifier) {
                    discarded_txs.push(tx);
                    continue 'tx_loop;
                }
            }

            // The gas limit set for a transaction is either the limit it sets,
            // or the gas left in the block, whichever is smallest.
            let gas_limit = cmp::min(tx.fee.gas_limit, block_gas_left);
            session.set_point_limit(gas_limit);

            let call_result: Option<Result<RawResult, ModuleError>> =
                match session.transact(
                    rusk_abi::transfer_module(),
                    "execute",
                    &tx,
                ) {
                    Ok(call_result) => call_result,
                    Err(err) => match err {
                        piecrust::Error::OutOfPoints => {
                            // If the transaction would have been out of points
                            // with its own gas limit, it is invalid and should
                            // be discarded.
                            if gas_limit == tx.fee.gas_limit {
                                discarded_txs.push(tx);
                            }
                            continue;
                        }
                        _ => {
                            discarded_txs.push(tx);
                            continue;
                        }
                    },
                };

            let gas_spent = session.spent();

            block_gas_left -= gas_spent;
            dusk_spent += gas_spent * tx.fee.gas_price;

            spent_txs.push(SpentTransaction(
                tx,
                gas_spent,
                call_result.and_then(|result| result.err()),
            ));

            // No need to keep executing if there is no gas left in the
            // block
            if block_gas_left == 0 {
                break;
            }
        }

        reward(&mut session, block_height, dusk_spent, generator)?;
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
        let mut session = inner.vm.session(inner.current_commit)?;

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
        let mut session = inner.vm.session(inner.current_commit)?;

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
        let mut session = inner.vm.session(inner.current_commit)?;

        let (spent_txs, state_root) = accept(
            &mut session,
            block_height,
            block_gas_limit,
            generator,
            txs,
        )?;

        let commit_id = session.commit()?;
        inner.current_commit = commit_id;

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
        self.query(rusk_abi::transfer_module(), "leaves_in_range", &range)
    }

    /// Returns the nullifiers that already exist from a list of given
    /// `nullifiers`.
    pub fn existing_nullifiers(
        &self,
        nullifiers: &Vec<BlsScalar>,
    ) -> Result<Vec<BlsScalar>> {
        self.query(
            rusk_abi::transfer_module(),
            "existing_nullifiers",
            nullifiers,
        )
    }

    /// Returns the root of the transfer tree.
    pub fn tree_root(&self) -> Result<BlsScalar> {
        self.query(rusk_abi::transfer_module(), "root", &())
    }

    /// Returns the opening of the transfer tree at the given position.
    pub fn tree_opening(
        &self,
        pos: u64,
    ) -> Result<Option<PoseidonBranch<TRANSFER_TREE_DEPTH>>> {
        self.query(rusk_abi::transfer_module(), "opening", &pos)
    }

    /// Returns the "transparent" balance of the given module.
    pub fn module_balance(&self, module: ModuleId) -> Result<u64> {
        self.query(rusk_abi::transfer_module(), "module_balance", &module)
    }

    /// Returns the message mapped to the given module and public key.
    pub fn module_message(
        &self,
        module: ModuleId,
        pk: PublicKey,
    ) -> Result<Option<Message>> {
        self.query(rusk_abi::transfer_module(), "message", &(module, pk))
    }

    /// Returns data about the stake of the given key.
    pub fn stake(&self, pk: BlsPublicKey) -> Result<Option<StakeData>> {
        self.query(rusk_abi::stake_module(), "get_stake", &pk)
    }

    /// Returns the stakes.
    pub fn provisioners(&self) -> Result<Vec<(BlsPublicKey, StakeData)>> {
        self.query(rusk_abi::stake_module(), "stakes", &())
    }

    /// Returns the keys allowed to stake.
    pub fn stake_allowlist(&self) -> Result<Vec<BlsPublicKey>> {
        self.query(rusk_abi::stake_module(), "allowlist", &())
    }

    /// Returns the keys that own the stake contract.
    pub fn stake_owners(&self) -> Result<Vec<BlsPublicKey>> {
        self.query(rusk_abi::stake_module(), "owners", &())
    }

    fn query<A, R>(
        &self,
        module_id: ModuleId,
        call_name: &str,
        call_arg: &A,
    ) -> Result<R>
    where
        A: for<'b> Serialize<StandardBufSerializer<'b>>,
        R: Archive,
        R::Archived: Deserialize<R, Infallible>
            + for<'b> CheckBytes<DefaultValidator<'b>>,
    {
        let inner = self.inner.lock();

        let mut session = inner.vm.session(inner.current_commit)?;

        // For queries we set a point limit of effectively infinite and a block
        // height of zero since this doesn't affect the result.
        session.set_point_limit(u64::MAX);
        rusk_abi::set_block_height(&mut session, 0);

        Ok(session.query(module_id, call_name, call_arg)?)
    }
}

fn accept(
    session: &mut Session,
    block_height: u64,
    block_gas_limit: u64,
    generator: BlsPublicKey,
    txs: Vec<Transaction>,
) -> Result<(Vec<SpentTransaction>, [u8; 32])> {
    rusk_abi::set_block_height(session, block_height);

    let mut block_gas_left = block_gas_limit;

    let mut spent_txs = Vec::with_capacity(txs.len());
    let mut dusk_spent = 0;

    let mut nullifiers = BTreeSet::new();

    for tx in txs {
        for input in &tx.nullifiers {
            if !nullifiers.insert(*input) {
                return Err(Error::RepeatingNullifiers(vec![*input]));
            }
        }

        let gas_limit = cmp::min(tx.fee.gas_limit, block_gas_left);
        session.set_point_limit(gas_limit);

        let call_result: Option<Result<RawResult, ModuleError>> =
            session.transact(rusk_abi::transfer_module(), "execute", &tx)?;

        let gas_spent = session.spent();

        dusk_spent += gas_spent * tx.fee.gas_price;
        block_gas_left = block_gas_left
            .checked_sub(gas_spent)
            .ok_or(Error::OutOfGas)?;

        spent_txs.push(SpentTransaction(
            tx,
            gas_spent,
            call_result.and_then(|result| result.err()),
        ));
    }

    reward(session, block_height, dusk_spent, generator)?;
    let state_root = session.root();

    Ok((spent_txs, state_root))
}

fn reward(
    session: &mut Session,
    block_height: u64,
    dusk_spent: Dusk,
    generator: BlsPublicKey,
) -> Result<()> {
    let (dusk_value, generator_value) =
        coinbase_value(block_height, dusk_spent);

    session.transact(
        rusk_abi::stake_module(),
        "reward",
        &(*DUSK_KEY, dusk_value),
    )?;

    session.transact(
        rusk_abi::stake_module(),
        "reward",
        &(generator, generator_value),
    )?;

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
