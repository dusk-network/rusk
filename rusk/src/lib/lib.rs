// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::transaction::SpentTransaction;

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{cmp, fs, io};

pub mod error;
pub mod transaction;

use bytecheck::CheckBytes;
use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_merkle::poseidon::Opening as PoseidonOpening;
use dusk_pki::PublicKey;
use parking_lot::{Mutex, MutexGuard};
use phoenix_core::transaction::*;
use phoenix_core::Message;
use piecrust::{Session, VM};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Deserialize, Infallible, Serialize};
use rusk_abi::dusk::{dusk, Dusk};
use rusk_abi::{
    ContractError, ContractId, RawResult, StandardBufSerializer,
    STAKE_CONTRACT, TRANSFER_CONTRACT,
};
use rusk_profile::to_rusk_state_id_path;
use rusk_recovery_tools::provisioners::DUSK_KEY;
use transfer_circuits::ExecuteCircuit;

const A: usize = 4;

pub type Result<T, E = Error> = core::result::Result<T, E>;

/// The gas grace is a magic number denoting the amount of gas that is spent by
/// a transaction after the charging code has been executed.
const GAS_GRACE: u64 = 24_858_000;

/// Computes the gas limit to apply to a transaction, given the limit imposed by
/// the fee limit and the block gas limit.
///
/// The result will be the minimum between the block gas limit and the fee limit
/// plus the [`GAS_GRACE`].
fn compute_gas_limit(fee_limit: u64, block_limit: u64) -> u64 {
    let min = cmp::min(fee_limit, block_limit);
    min + GAS_GRACE
}

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
            let gas_limit = compute_gas_limit(tx.fee.gas_limit, block_gas_left);
            session.set_point_limit(gas_limit);

            let (gas_spent, call_result): (
                u64,
                Option<Result<RawResult, ContractError>>,
            ) = match session.call(TRANSFER_CONTRACT, "execute", &tx) {
                Ok(call_result) => call_result,
                Err(err) => match err {
                    piecrust::Error::OutOfPoints => {
                        let fee_limit =
                            compute_gas_limit(tx.fee.gas_limit, u64::MAX);

                        // If the transaction would have been out of points
                        // with its own gas limit, it is invalid and should
                        // be discarded.
                        if gas_limit == fee_limit {
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

            // If the gas spent is larger that the remaining block gas, then we
            // re-execute the transactions with a new session. We can ignore the
            // results since we know they will be valid.
            if gas_spent > block_gas_left {
                session = rusk_abi::new_session(
                    &inner.vm,
                    current_commit,
                    block_height,
                )?;

                for spent_tx in &spent_txs {
                    let _: (
                        u64,
                        Option<Result<RawResult, ContractError>>,
                    ) =
                    session.call(TRANSFER_CONTRACT, "execute", &spent_tx.0)
                        .expect("Re-execution of spent transactions should never fail");
                }

                continue;
            }

            block_gas_left -= gas_spent;
            dusk_spent += gas_spent * tx.fee.gas_price;

            spent_txs.push(SpentTransaction {
                tx,
                gas_spent,
                error: call_result.and_then(|result| result.err()),
            });

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

    pub fn preverify(&self, tx: &Transaction) -> Result<()> {
        let tx_hash = rusk_abi::hash(tx.to_hash_input_bytes());

        let inputs = &tx.nullifiers;
        let outputs = &tx.outputs;
        let proof = &tx.proof;

        let existing_nullifiers = self.existing_nullifiers(inputs)?;

        if !existing_nullifiers.is_empty() {
            return Err(Error::RepeatingNullifiers(existing_nullifiers));
        }

        // if !RuskProver::preverify(tx)? {
        //     return Err(Error::ProofVerification);
        // }

        let circuit = circuit_from_numbers(inputs.len(), outputs.len())
            .ok_or_else(|| {
                Error::InvalidCircuitArguments(inputs.len(), outputs.len())
            })?;

        let mut pi: Vec<rusk_abi::PublicInput> =
            Vec::with_capacity(9 + inputs.len());

        pi.push(tx_hash.into());
        pi.push(tx.anchor.into());
        pi.extend(inputs.iter().map(|n| n.into()));

        pi.push(
            tx.crossover()
                .copied()
                .unwrap_or_default()
                .value_commitment()
                .into(),
        );

        let fee_value = tx.fee().gas_limit * tx.fee().gas_price;

        pi.push(fee_value.into());
        pi.extend(outputs.iter().map(|n| n.value_commitment().into()));
        pi.extend(
            (0usize..2usize.saturating_sub(outputs.len())).map(|_| {
                transfer_circuits::CircuitOutput::ZERO_COMMITMENT.into()
            }),
        );

        let keys = rusk_profile::keys_for(circuit.circuit_id())?;
        let vd = keys.get_verifier()?;

        // Maybe we want to handle internal serialization error too, currently
        // they map to `false`.
        if !rusk_abi::verify_proof(vd, proof.clone(), pi) {
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

    let mut nullifiers = BTreeSet::new();

    for tx in txs {
        for input in &tx.nullifiers {
            if !nullifiers.insert(*input) {
                return Err(Error::RepeatingNullifiers(vec![*input]));
            }
        }

        let gas_limit = compute_gas_limit(tx.fee.gas_limit, block_gas_left);
        session.set_point_limit(gas_limit);

        let (gas_spent, call_result): (
            u64,
            Option<Result<RawResult, ContractError>>,
        ) = session.call(TRANSFER_CONTRACT, "execute", &tx)?;

        dusk_spent += gas_spent * tx.fee.gas_price;
        block_gas_left = block_gas_left
            .checked_sub(gas_spent)
            .ok_or(Error::OutOfGas)?;

        spent_txs.push(SpentTransaction {
            tx,
            gas_spent,
            error: call_result.and_then(|result| result.err()),
        });
    }

    reward_and_update_root(session, block_height, dusk_spent, generator)?;
    let state_root = session.root();

    Ok((spent_txs, state_root))
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

fn circuit_from_numbers(
    num_inputs: usize,
    num_outputs: usize,
) -> Option<ExecuteCircuit<(), TRANSFER_TREE_DEPTH, A>> {
    use ExecuteCircuit::*;

    match num_inputs {
        1 if num_outputs < 3 => Some(OneTwo(Default::default())),
        2 if num_outputs < 3 => Some(TwoTwo(Default::default())),
        3 if num_outputs < 3 => Some(ThreeTwo(Default::default())),
        4 if num_outputs < 3 => Some(FourTwo(Default::default())),
        _ => None,
    }
}
