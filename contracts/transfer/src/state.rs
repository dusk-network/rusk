// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::tree::Tree;
use crate::verifier_data::*;

use alloc::collections::btree_map::Entry;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use poseidon_merkle::Opening as PoseidonOpening;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rusk_abi::{
    ContractError, ContractId, EconomicMode, PublicInput, RawResult,
    STAKE_CONTRACT,
};

use execution_core::{
    transfer::{
        EconomicEvent, EconomicResult, Fee, Mint, SenderAccount, Transaction,
        TreeLeaf, TRANSFER_TREE_DEPTH,
    },
    BlsScalar, Note,
};

/// Number of roots stored
pub const MAX_ROOTS: usize = 5000;

pub struct TransferState {
    tree: Tree,
    nullifiers: BTreeSet<BlsScalar>,
    roots: ConstGenericRingBuffer<BlsScalar, MAX_ROOTS>,
    balances: BTreeMap<ContractId, u64>,
    deposit: Option<(ContractId, u64)>,
    gas_price: Option<u64>,
}

impl TransferState {
    pub const fn new() -> TransferState {
        TransferState {
            tree: Tree::new(),
            nullifiers: BTreeSet::new(),
            roots: ConstGenericRingBuffer::new(),
            balances: BTreeMap::new(),
            deposit: None,
            gas_price: None,
        }
    }

    /// Mint a new phoenix note.
    ///
    /// This can only be called by the transfer- and stake-contracts.
    /// If called by the `stake-contract`, this method will increase the total
    /// amount of circulating dusk. This happens when the reward for staking
    /// and participating in the consensus is withdrawn.
    /// If called by the transfer-contract itself, it is important to make sure
    /// that the minted value is subtracted from a contracts balance before
    /// creating a phoenix-note.
    pub fn mint(&mut self, mint: Mint) -> bool {
        // why return bool?
        let caller = rusk_abi::caller();
        if caller != STAKE_CONTRACT && !rusk_abi::caller().is_uninitialized() {
            panic!("Can only be called by the stake contract!")
        }
        let sender = SenderAccount {
            contract: caller.to_bytes(),
            account: mint.sender,
        };

        let note = Note::transparent_stealth(mint.address, mint.value, sender);

        self.push_note_current_height(note);

        true
    }

    /// Withdraw from a contract's balance into a phoenix-note.
    ///
    /// Even though a new phoenix-note is minted, the funds are only moved there
    /// from the contract's balance. This means that, unlike [`mint`], calling
    /// this function will not increase the total amount of circulating dusk.
    ///
    /// # Panics
    /// This can only be called by a contract that with sufficient balance.
    pub fn withdraw(&mut self, withdraw: Mint) {
        // check if the request comes from a contract
        let contract = rusk_abi::caller();
        if contract.is_uninitialized() {
            panic!("The \"withdraw\" method can only be called by another contract.")
        }

        // check if the contract has enough balance
        if self.balance(&contract) < withdraw.value {
            panic!("The contract doesn't have enough balance.");
        }

        // subtract the withdraw-value from the contract's balance
        self.sub_balance(&contract, withdraw.value)
            .expect("The contract should have enough balance");

        // push a new phoenix-note with the given data to the tree
        let sender = SenderAccount {
            contract: contract.to_bytes(),
            account: withdraw.sender,
        };
        let note =
            Note::transparent_stealth(withdraw.address, withdraw.value, sender);
        self.push_note_current_height(note);
    }

    /// Deposit funds to a contract's balance.
    ///
    /// This function checks whether a deposit has been placed earlier on the
    /// state. If so and the contract-id matches the caller, the deposit will be
    /// added to the contract's balance.
    ///
    /// # Panics
    /// This function will panic if there is no deposit on the state or the
    /// caller-id doesn't match the contract-id stored for the deposit.
    pub fn deposit(&mut self, value: u64) {
        // check is the request comes from a contract
        let caller = rusk_abi::caller();
        if caller.is_uninitialized() {
            panic!("Only a contract is authorized to claim a deposit.")
        }

        let deposit = self.deposit.take();
        match deposit {
            Some((deposit_contract, deposit_value)) => {
                if deposit_value != value {
                    panic!(
                        "The value to deposit doesn't match the previously deposited value"
                        );
                } else if deposit_contract != caller {
                    panic!(
                        "The caller is not authorized to claim the deposit."
                    );
                } else {
                    self.add_balance(deposit_contract, deposit_value);
                }
            }
            None => {
                panic!("There is no deposit on the state.");
            }
        }
    }

    /// Spends the inputs and creates the given UTXO, and executes the contract
    /// call if present. It performs all checks necessary to ensure the
    /// transaction is valid - hash matches, anchor has been a root of the
    /// tree, proof checks out, etc...
    ///
    /// This will emplace the deposit in the state, if it exists - making it
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
        let tx_skeleton = tx.payload().tx_skeleton();

        // panic if the root is invalid
        if !self.root_exists(&tx_skeleton.root) {
            panic!("Root not found in the state!");
        }

        // panic if any of the given nullifiers already exist
        if self.any_nullifier_exists(&tx_skeleton.nullifiers) {
            panic!("A provided nullifier already exists!");
        }

        // append the nullifiers to the nullifiers set
        self.nullifiers.extend(&tx_skeleton.nullifiers);

        // verify the phoenix-circuit
        if !verify_tx_proof(&tx) {
            panic!("Invalid transaction proof!");
        }

        // append the output notes to the phoenix-notes tree
        let block_height = rusk_abi::block_height();
        self.tree
            .extend_notes(block_height, tx_skeleton.outputs.clone());

        // place the contract deposit on the state
        if tx.payload().deposit {
            let contract = match tx.payload().contract_call() {
                Some(call) => ContractId::from_bytes(call.contract),
                None => {
                    panic!("There needs to be a contract call when depositing funds");
                }
            };
            self.deposit = Some((contract, tx.payload().tx_skeleton.deposit));
        }

        // perform contract call if present
        let mut result = Ok(rusk_abi::RawResult::empty());
        if let Some(call) = tx.payload().contract_call() {
            self.gas_price = Some(tx.payload().fee.gas_price);
            result = rusk_abi::call_raw(
                ContractId::from_bytes(call.contract),
                &call.fn_name,
                &call.fn_args,
            );
            self.gas_price = None;
            if let Ok(RawResult {
                data: _,
                economic_mode,
            }) = result.clone()
            {
                match economic_mode {
                    EconomicMode::Allowance(allowance) if allowance != 0 => {
                        rusk_abi::set_allowance(allowance)
                    }
                    _ => (),
                }
            }
        }

        result.map(|r| r.data)
    }

    // Applies contract's allowance. Caller of the contract's method
    // won't pay a fee and all the cost will be covered by the contract.
    // Allowance has no effect if contract does not have enough funds or
    // if the actual cost of the call is greater than allowance.
    // Returns economic gas spent
    fn apply_allowance(
        &mut self,
        contract_id: &ContractId,
        allowance: u64,
        gas_spent: u64,
        gas_price: u64,
    ) -> u64 {
        let spent = gas_spent * gas_price;
        if allowance * gas_price < spent {
            rusk_abi::emit(
                "sponsoring",
                EconomicEvent {
                    contract: contract_id.to_bytes(),
                    value: allowance * gas_price,
                    result: EconomicResult::AllowanceNotSufficient,
                },
            );
            gas_spent
        } else {
            let contract_balance = self.balance(contract_id);
            if spent > contract_balance {
                rusk_abi::emit(
                    "sponsoring",
                    EconomicEvent {
                        contract: contract_id.to_bytes(),
                        value: allowance,
                        result: EconomicResult::BalanceNotSufficient,
                    },
                );
                gas_spent
            } else {
                self.sub_balance(contract_id, spent).expect(
                    "Subtracting callee contract balance should succeed",
                );
                self.add_balance(rusk_abi::self_id(), spent);
                rusk_abi::emit(
                    "sponsoring",
                    EconomicEvent {
                        contract: contract_id.to_bytes(),
                        value: spent,
                        result: EconomicResult::AllowanceApplied,
                    },
                );
                0u64
            }
        }
    }

    /// Refund the previously performed transaction, taking into account the
    /// given gas spent. The notes produced will be refunded to the address
    /// present in the fee structure.
    /// If contract id is present, it applies economic mode to the the contract
    /// and refund is based on the economic calculation.
    ///
    /// This function guarantees that it will not panic.
    pub fn refund(
        &mut self,
        fee: Fee,
        gas_spent: u64,
        economic_mode: EconomicMode,
        contract_id: Option<ContractId>,
    ) {
        let economic_gas_spent = if let Some(contract_id) = contract_id {
            match economic_mode {
                EconomicMode::Allowance(allowance) if allowance != 0 => self
                    .apply_allowance(
                        &contract_id,
                        allowance,
                        gas_spent,
                        fee.gas_price,
                    ),
                _ => gas_spent,
            }
        } else {
            gas_spent
        };

        let remainder_note = fee.gen_remainder_note(economic_gas_spent);

        let remainder_value = remainder_note
            .value(None)
            .expect("Should always succeed for a transparent note");

        if remainder_value > 0 {
            self.push_note_current_height(remainder_note);
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
    ) -> Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH>> {
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

    /// Return the current gas price as set by the execute method.
    /// Returns none outside of the lifetime of the execute method.
    pub fn gas_price(&self) -> u64 {
        self.gas_price.expect(
            "During transaction execution host should always set the gas price",
        )
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
}

fn verify_tx_proof(tx: &Transaction) -> bool {
    let pis: Vec<PublicInput> =
        tx.public_inputs().iter().map(|pi| pi.into()).collect();

    // fetch the verifier data
    let num_inputs = tx.payload().tx_skeleton.nullifiers.len();
    let vd = verifier_data_execute(num_inputs)
        .expect("No circuit available for given number of inputs!")
        .to_vec();

    // verify the proof
    rusk_abi::verify_proof(vd, tx.proof().clone(), pis)
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
