// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::circuits::*;
use crate::error::Error;
use crate::tree::Tree;

use alloc::collections::btree_map::Entry;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;
use dusk_jubjub::JubJubAffine;
use phoenix_core::transaction::*;
use phoenix_core::{Crossover, Fee, Note, Ownable, StealthAddress};
use poseidon_merkle::Opening as PoseidonOpening;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rusk_abi::{
    ContractError, ContractId, PaymentInfo, PublicInput, STAKE_CONTRACT,
};
use transfer_contract_types::{Mint, Stct, Wfct, Wfctc};

/// Arity of the transfer tree.
pub const A: usize = 4;

/// Number of roots stored
pub const MAX_ROOTS: usize = 5000;

pub struct TransferState {
    tree: Tree,
    nullifiers: BTreeSet<BlsScalar>,
    roots: ConstGenericRingBuffer<BlsScalar, MAX_ROOTS>,
    balances: BTreeMap<ContractId, u64>,
    var_crossover: Option<Crossover>,
    var_crossover_addr: Option<StealthAddress>,
}

impl TransferState {
    pub const fn new() -> TransferState {
        TransferState {
            tree: Tree::new(),
            nullifiers: BTreeSet::new(),
            roots: ConstGenericRingBuffer::new(),
            balances: BTreeMap::new(),
            var_crossover: None,
            var_crossover_addr: None,
        }
    }

    pub fn mint(&mut self, mint: Mint) -> bool {
        // Only the stake contract can mint notes to a particular stealth
        // address. This happens when the reward for staking and participating
        // in the consensus is withdrawn.
        if rusk_abi::caller() != STAKE_CONTRACT
            && !rusk_abi::caller().is_uninitialized()
        {
            panic!("Can only be called by the stake contract!")
        }

        let note =
            Note::transparent_stealth(mint.address, mint.value, mint.nonce);

        self.push_note_current_height(note);

        true
    }

    pub fn send_to_contract_transparent(&mut self, stct: Stct) -> bool {
        let (crossover, stealth_addr) =
            self.take_crossover().expect("Crossover not present");

        let address =
            rusk_abi::contract_to_scalar(&ContractId::from_bytes(stct.module));

        let message =
            stct_signature_message(&crossover, stct.value, address).to_vec();
        let message = rusk_abi::poseidon_hash(message);

        let mut pi = Vec::with_capacity(6);

        pi.push(crossover.value_commitment().into());
        pi.push(stct.value.into());
        pi.push(stealth_addr.pk_r().as_ref().into());
        pi.push(message.into());

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        let contract_id = ContractId::from_bytes(stct.module);
        self.add_balance(contract_id, stct.value);

        //  3. if a.isPayable() ↦ true then continue
        let contract_id = ContractId::from_bytes(stct.module);
        match rusk_abi::payment_info(contract_id)
            .expect("Querying the payment info should succeed")
        {
            PaymentInfo::Transparent(_) | PaymentInfo::Any(_) => (),
            _ => panic!("The caller doesn't accept transparent notes"),
        }

        //  4. verify(C.c, v, π)
        let vd = verifier_data_stct();
        Self::assert_proof(vd, stct.proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C ← C(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_contract_transparent(&mut self, wfct: Wfct) -> bool {
        let address = rusk_abi::caller();
        let mut pi = Vec::with_capacity(3);

        pi.push(wfct.value.into());
        pi.push(wfct.note.value_commitment().into());

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        self.sub_balance(&address, wfct.value)
            .expect("Failed to subtract the balance from the provided address");

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        self.push_note_current_height(wfct.note);

        //  6. verify(C.c, M, pk, π)
        let vd = verifier_data_wfct();
        Self::assert_proof(vd, wfct.proof, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn withdraw_from_contract_transparent_raw(
        &mut self,
        wfct_raw: transfer_contract_types::WfctRaw,
    ) -> bool {
        let note = Note::from_slice(wfct_raw.note.as_slice())
            .expect("Failed to deserialize note");
        self.withdraw_from_contract_transparent(Wfct {
            value: wfct_raw.value,
            note,
            proof: wfct_raw.proof,
        })
    }

    pub fn withdraw_from_contract_transparent_to_contract(
        &mut self,
        wfctc: Wfctc,
    ) -> bool {
        let from = rusk_abi::caller();

        //  1. from ∈ B↦
        //  2. B_from↦ ← B_from↦ − v
        self.sub_balance(&from, wfctc.value).expect(
            "Failed to subtract the balance from the provided address!",
        );

        //  3. B_to↦ = B_to↦ + v
        let module = ContractId::from_bytes(wfctc.module);
        self.add_balance(module, wfctc.value);

        true
    }

    /// Spends the inputs and creates the given UTXO, and executes the contract
    /// call if present. It performs all checks necessary to ensure the
    /// transaction is valid - hash matches, anchor has been a root of the
    /// tree, proof checks out, etc...
    ///
    /// This will emplace the crossover in the state, if it exists - making it
    /// available for any contracts called.
    ///
    /// [`refund`] **must** be called if this function succeeds, otherwise we
    /// will have an inconsistent state.
    ///
    /// # Panics
    /// Any failure in the checks performed in processing the transaction will
    /// result in a panic. The contract expects the environment to roll back any
    /// change in state.
    ///
    /// [`refund`]: [`TransferState::refund`]
    pub fn spend_and_execute(
        &mut self,
        tx: Transaction,
    ) -> Result<Vec<u8>, ContractError> {
        //  1. α ∈ R
        if !self.root_exists(&tx.anchor) {
            panic!("Anchor not found in the state!");
        }

        //  2. ν[] !∈ Nullifiers
        if self.any_nullifier_exists(&tx.nullifiers) {
            panic!("A provided nullifier already exists!");
        }

        //  3. Nullifiers.append(ν[])
        self.nullifiers.extend(&tx.nullifiers);

        //  4. if |C|=0 then set C ← (0,0,0)
        //  Crossover is received as option

        //  5. N↦.append((No.R[], No.pk[])
        //  6. Notes.append(No[])
        let block_height = rusk_abi::block_height();
        self.tree.extend_notes(block_height, tx.outputs.clone());

        //  7. g_l < 2^64
        //  8. g_pmin < g_p
        //  9. fee ← g_l ⋅ g_p
        // 10. verify(α, ν[], C.c, No.c[], fee)
        if !verify_tx_proof(&tx) {
            panic!("Invalid transaction proof!");
        }

        // 11. if ∣k∣≠0 then call(k)
        self.var_crossover = tx.crossover;
        self.var_crossover_addr.replace(*tx.fee.stealth_address());

        let mut result = Ok(Vec::new());

        if let Some((contract_id, fn_name, fn_args)) = tx.call {
            result = rusk_abi::call_raw(
                ContractId::from_bytes(contract_id),
                &fn_name,
                &fn_args,
            );
        }

        result
    }

    /// Refund the previously performed transaction, taking into account the
    /// given gas spent. The notes produced will be refunded to the address
    /// present in the fee structure.
    ///
    /// This function guarantees that it will not panic.
    pub fn refund(&mut self, fee: Fee, gas_spent: u64) {
        let block_height = rusk_abi::block_height();

        let remainder = fee.gen_remainder(gas_spent);
        let remainder = Note::from(remainder);

        let remainder_value = remainder
            .value(None)
            .expect("Should always succeed for a transparent note");

        if remainder_value > 0 {
            self.push_note(block_height, remainder);
        }

        if let Some(crossover) = self.var_crossover {
            let note = Note::from((fee, crossover));
            self.push_note(block_height, note);
        }
    }

    /// Push a note to the contract's state with the given block height
    ///
    /// Note: the method `update_root` needs to be called after the last note is
    /// pushed.
    pub fn push_note(&mut self, block_height: u64, note: Note) -> Note {
        let tree_leaf = TreeLeaf { block_height, note };
        let pos = self.tree.push(tree_leaf.clone());
        rusk_abi::emit("TREE_LEAF", (pos, tree_leaf));
        self.get_note(pos)
            .expect("There should be a note that was just inserted")
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// height.
    pub fn leaves_from_height(&self, height: u64) {
        for leaf in self.tree.leaves(height) {
            rusk_abi::feed(leaf.clone());
        }
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// position.
    pub fn leaves_from_pos(&self, pos: u64) {
        for leaf in self.tree.leaves_pos(pos) {
            rusk_abi::feed(leaf.clone());
        }
    }

    /// Update the root for of the tree.
    pub fn update_root(&mut self) {
        let root = self.tree.root();
        self.roots.push(root);
    }

    /// Get the root of the tree.
    pub fn root(&self) -> BlsScalar {
        self.tree.root()
    }

    /// Get the count of the notes in the tree.
    pub fn num_notes(&self) -> u64 {
        self.tree.leaves_len()
    }

    /// Get the opening
    pub fn opening(
        &self,
        pos: u64,
    ) -> Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>> {
        self.tree.opening(pos)
    }

    /// Takes some nullifiers and returns a vector containing the ones that
    /// already exists in the contract
    pub fn existing_nullifiers(
        &self,
        nullifiers: Vec<BlsScalar>,
    ) -> Vec<BlsScalar> {
        nullifiers
            .into_iter()
            .filter_map(|n| self.nullifiers.get(&n).map(|_| n))
            .collect()
    }

    /// Return the balance of a given contract.
    pub fn balance(&self, contract_id: &ContractId) -> u64 {
        self.balances.get(contract_id).copied().unwrap_or_default()
    }

    /// Add balance to the given contract
    pub fn add_balance(&mut self, contract: ContractId, value: u64) {
        match self.balances.entry(contract) {
            Entry::Vacant(ve) => {
                ve.insert(value);
            }
            Entry::Occupied(mut oe) => {
                let v = oe.get_mut();
                *v += value
            }
        }
    }

    fn get_note(&self, pos: u64) -> Option<Note> {
        self.tree.get(pos).map(|l| l.note)
    }

    fn any_nullifier_exists(&self, nullifiers: &[BlsScalar]) -> bool {
        for nullifier in nullifiers {
            if self.nullifiers.contains(nullifier) {
                return true;
            }
        }

        false
    }

    fn root_exists(&self, root: &BlsScalar) -> bool {
        self.roots.contains(root)
    }

    fn push_note_current_height(&mut self, note: Note) -> Note {
        let block_height = rusk_abi::block_height();
        self.push_note(block_height, note)
    }

    pub(crate) fn sub_balance(
        &mut self,
        address: &ContractId,
        value: u64,
    ) -> Result<(), Error> {
        match self.balances.get_mut(address) {
            Some(balance) => {
                let (bal, underflow) = balance.overflowing_sub(value);

                if underflow {
                    Err(Error::NotEnoughBalance)
                } else {
                    *balance = bal;

                    Ok(())
                }
            }

            _ => Err(Error::NotEnoughBalance),
        }
    }

    fn take_crossover(&mut self) -> Result<(Crossover, StealthAddress), Error> {
        let crossover =
            self.var_crossover.take().ok_or(Error::CrossoverNotFound)?;

        let sa = self
            .var_crossover_addr
            .take()
            .ok_or(Error::CrossoverNotFound)?;

        Ok((crossover, sa))
    }

    fn assert_proof(
        verifier_data: &[u8],
        proof: Vec<u8>,
        public_inputs: Vec<PublicInput>,
    ) -> Result<(), Error> {
        rusk_abi::verify_proof(verifier_data.to_vec(), proof, public_inputs)
            .then_some(())
            .ok_or(Error::ProofVerification)
    }
}

fn verify_tx_proof(tx: &Transaction) -> bool {
    // Constant for a pedersen commitment with zero value.
    // Calculated as `G^0 · G'^0`
    const ZERO_COMMITMENT: JubJubAffine =
        JubJubAffine::from_raw_unchecked(BlsScalar::zero(), BlsScalar::one());

    let n_nullifiers = tx.nullifiers.len();
    let n_outputs = tx.outputs.len();

    let tx_hash = rusk_abi::hash(tx.to_hash_input_bytes());
    let crossover_commitment = tx
        .crossover
        .map(|c| *c.value_commitment())
        .unwrap_or_default();
    let fee_value = tx.fee.gas_limit * tx.fee.gas_price;

    let mut pis =
        Vec::<PublicInput>::with_capacity(5 + n_nullifiers + 2 * n_outputs);

    pis.push(tx_hash.into());
    pis.push(tx.anchor.into());
    pis.extend(tx.nullifiers.iter().map(Into::into));
    pis.push(crossover_commitment.into());

    pis.push(fee_value.into());
    pis.extend(tx.outputs.iter().map(|n| n.value_commitment().into()));
    pis.extend(
        (0usize..2usize.saturating_sub(n_outputs))
            .map(|_| ZERO_COMMITMENT.into()),
    );

    let vd = verifier_data_execute(n_nullifiers)
        .expect("No circuit available for given number of inputs!")
        .to_vec();
    rusk_abi::verify_proof(vd, tx.proof.clone(), pis)
}

#[cfg(test)]
mod test_transfer {
    use super::*;

    #[test]
    fn find_existing_nullifiers() {
        let mut transfer = TransferState::new();

        let (zero, one, two, three, ten, eleven) = (
            BlsScalar::from(0),
            BlsScalar::from(1),
            BlsScalar::from(2),
            BlsScalar::from(3),
            BlsScalar::from(10),
            BlsScalar::from(11),
        );

        let existing = transfer
            .existing_nullifiers(vec![zero, one, two, three, ten, eleven]);

        assert_eq!(existing.len(), 0);

        for i in 1..10 {
            transfer.nullifiers.insert(BlsScalar::from(i));
        }

        let existing = transfer
            .existing_nullifiers(vec![zero, one, two, three, ten, eleven]);

        assert_eq!(existing.len(), 3);

        assert!(existing.contains(&one));
        assert!(existing.contains(&two));
        assert!(existing.contains(&three));
    }
}
