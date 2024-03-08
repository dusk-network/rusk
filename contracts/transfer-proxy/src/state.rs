// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_pki::PublicKey;
use phoenix_core::transaction::*;
use phoenix_core::{Fee, Message, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use rusk_abi::{ContractError, ContractId};
use transfer_contract_types::{Mint, Stct, Wfco, WfcoRaw, Wfct, Wfctc};

/// Arity of the transfer tree.
pub const A: usize = 4;

pub struct TransferProxy {
    target: ContractId,
}

impl TransferProxy {
    pub const fn new() -> Self {
        Self {
            target: ContractId::uninitialized(),
        }
    }

    pub fn init(&mut self, contract_id: ContractId) {
        self.target = contract_id;
    }

    pub fn mint(&mut self, mint: Mint) -> bool {
        rusk_abi::call::<Mint, bool>(self.target, "mint", &mint)
            .expect("mint call should succeed")
    }

    pub fn send_to_contract_transparent(&mut self, stct: Stct) -> bool {
        rusk_abi::call::<Stct, bool>(self.target, "stct", &stct)
            .expect("send_to_contract_transparent call should succeed")
    }

    pub fn withdraw_from_contract_transparent(&mut self, wfct: Wfct) -> bool {
        let from_address = rusk_abi::caller();
        rusk_abi::call::<(Wfct, ContractId), bool>(
            self.target,
            "wfct",
            &(wfct, from_address),
        )
        .expect("withdraw_from_contract_transparent call should succeed")
    }

    pub fn withdraw_from_contract_transparent_raw(
        &mut self,
        wfct_raw: transfer_contract_types::WfctRaw,
    ) -> bool {
        let from_address = rusk_abi::caller();
        rusk_abi::call::<(transfer_contract_types::WfctRaw, ContractId), bool>(
            self.target,
            "wfct_raw",
            &(wfct_raw, from_address),
        )
        .expect("withdraw_from_contract_transparent_raw call should succeed")
    }

    pub fn send_to_contract_obfuscated(&mut self, stco: Stco) -> bool {
        rusk_abi::call::<Stco, bool>(self.target, "stco", &stco)
            .expect("send_to_contract_obfuscated call should succeed")
    }

    pub fn withdraw_from_contract_obfuscated(&mut self, wfco: Wfco) -> bool {
        let from_address = rusk_abi::caller();
        rusk_abi::call::<(Wfco, ContractId), bool>(
            self.target,
            "wfco",
            &(wfco, from_address),
        )
        .expect("withdraw_from_contract_obfuscated call should succeed")
    }

    pub fn withdraw_from_contract_obfuscated_raw(
        &mut self,
        wfco_raw: WfcoRaw,
    ) -> bool {
        let from_address = rusk_abi::caller();
        rusk_abi::call::<(WfcoRaw, ContractId), bool>(
            self.target,
            "wfco_raw",
            &(wfco_raw, from_address),
        )
        .expect("withdraw_from_contract_obfuscated_raw call should succeed")
    }

    pub fn withdraw_from_contract_transparent_to_contract(
        &mut self,
        wfctc: Wfctc,
    ) -> bool {
        let from_address = rusk_abi::caller();
        rusk_abi::call::<(Wfctc, ContractId), bool>(
            self.target,
            "wfctc",
            &(wfctc, from_address),
        )
        .expect("withdraw_from_contract_transparent_to_contract call should succeed")
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
        rusk_abi::call::<Transaction, Result<Vec<u8>, ContractError>>(
            self.target,
            "spend",
            &tx,
        )
        .expect("spend_and_execute call should succeed")?;
        match rusk_abi::call::<Transaction, Result<Vec<u8>, ContractError>>(
            self.target,
            "execute",
            &tx,
        ) {
            Ok(r) => r,
            Err(e) => Err(e),
        }
    }

    /// Refund the previously performed transaction, taking into account the
    /// given gas spent. The notes produced will be refunded to the address
    /// present in the fee structure.
    ///
    /// This function guarantees that it will not panic.
    pub fn refund(&mut self, fee: Fee, gas_spent: u64) {
        rusk_abi::call::<(Fee, u64), ()>(
            self.target,
            "refund",
            &(fee, gas_spent),
        )
        .expect("refund call should succeed")
    }

    /// Push a note to the contract's state with the given block height
    ///
    /// Note: the method `update_root` needs to be called after the last note is
    /// pushed.
    pub fn push_note(&mut self, block_height: u64, note: Note) -> Note {
        rusk_abi::call::<(u64, Note), Note>(
            self.target,
            "push_note",
            &(block_height, note),
        )
        .expect("push_note call should succeed")
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// height.
    pub fn leaves_from_height(&self, height: u64) {
        rusk_abi::call::<u64, ()>(self.target, "leaves_from_height", &height)
            .expect("leaves_from_height query should succeed");
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// position.
    pub fn leaves_from_pos(&self, pos: u64) {
        rusk_abi::call::<u64, ()>(self.target, "leaves_from_pos", &pos)
            .expect("leaves_from_pos query should succeed");
    }

    /// Update the root of the tree.
    pub fn update_root(&mut self) {
        rusk_abi::call::<(), ()>(self.target, "update_root", &())
            .expect("update_root call should succeed");
    }

    /// Get the root of the tree.
    pub fn root(&self) -> BlsScalar {
        rusk_abi::call::<(), BlsScalar>(self.target, "root", &())
            .expect("root query should succeed")
    }

    /// Get the count of the notes in the tree.
    pub fn num_notes(&self) -> u64 {
        rusk_abi::call::<(), u64>(self.target, "num_notes", &())
            .expect("num_notes query should succeed")
    }

    /// Get the opening
    pub fn opening(
        &self,
        pos: u64,
    ) -> Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>> {
        rusk_abi::call::<u64, Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>>>(
            self.target,
            "opening",
            &pos,
        )
        .expect("opening query should succeed")
    }

    /// Takes some nullifiers and returns a vector containing the ones that
    /// already exists in the contract
    pub fn existing_nullifiers(
        &self,
        nullifiers: &Vec<BlsScalar>,
    ) -> Vec<BlsScalar> {
        rusk_abi::call::<Vec<BlsScalar>, Vec<BlsScalar>>(
            self.target,
            "existing_nullifiers",
            nullifiers,
        )
        .expect("calling existing nullifiers should succeed")
    }

    /// Return the balance of a given contract.
    pub fn balance(&self, contract_id: &ContractId) -> u64 {
        rusk_abi::call(self.target, "module_balance", contract_id)
            .expect("balance query should succeed")
    }

    /// Add balance to the given contract
    pub fn add_balance(&mut self, contract: ContractId, value: u64) {
        rusk_abi::call::<(ContractId, u64), ()>(
            self.target,
            "add_module_balance",
            &(contract, value),
        )
        .expect("add_module_balance call should succeed")
    }

    pub fn message(
        &self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Option<Message> {
        rusk_abi::call::<(ContractId, PublicKey), Option<Message>>(
            self.target,
            "message",
            &(*contract, *pk),
        )
        .expect("message call should succeed")
    }

    pub fn sub_module_balance(&self, contract: ContractId, value: u64) {
        let caller = rusk_abi::caller();
        rusk_abi::call::<(ContractId, u64, ContractId), ()>(
            self.target,
            "sub_module_balance",
            &(contract, value, caller),
        )
        .expect("sub_module_balance call should succeed")
    }
}
