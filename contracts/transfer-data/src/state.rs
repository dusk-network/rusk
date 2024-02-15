// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::tree::Tree;

use alloc::collections::btree_map::Entry;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::transaction::*;
use phoenix_core::{Crossover, Message, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use rusk_abi::ContractId;

/// Arity of the transfer tree.
pub const A: usize = 4;

pub struct TransferState {
    tree: Tree,
    nullifiers: BTreeSet<BlsScalar>,
    roots: BTreeSet<BlsScalar>,
    balances: BTreeMap<ContractId, u64>,
    message_mapping:
        BTreeMap<ContractId, BTreeMap<[u8; PublicKey::SIZE], Message>>,
    message_mapping_set: BTreeMap<ContractId, StealthAddress>,
    var_crossover: Option<Crossover>,
    var_crossover_addr: Option<StealthAddress>,
}

impl TransferState {
    pub const fn new() -> TransferState {
        TransferState {
            tree: Tree::new(),
            nullifiers: BTreeSet::new(),
            roots: BTreeSet::new(),
            balances: BTreeMap::new(),
            message_mapping: BTreeMap::new(),
            message_mapping_set: BTreeMap::new(),
            var_crossover: None,
            var_crossover_addr: None,
        }
    }

    /// Push a note to the contract's state with the given block height
    ///
    /// Note: the method `update_root` needs to be called after the last note is
    /// pushed.
    pub fn push_note(&mut self, block_height: u64, note: Note) -> u64 {
        let tree_leaf = TreeLeaf { block_height, note };
        self.tree.push(tree_leaf)
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

    /// Update the root of the tree.
    pub fn update_root(&mut self) {
        let root = self.tree.root();
        self.roots.insert(root);
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
    /// already exist in the contract
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

    pub fn message(
        &self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Option<Message> {
        let map = self.message_mapping.get(contract)?;
        let message = map.get(&pk.to_bytes())?;

        Some(*message)
    }

    pub fn any_nullifier_exists(&self, nullifiers: Vec<BlsScalar>) -> bool {
        for ref nullifier in nullifiers {
            if self.nullifiers.contains(nullifier) {
                return true;
            }
        }

        false
    }

    pub fn extend_nullifiers(&mut self, nullifiers: Vec<BlsScalar>) {
        self.nullifiers.extend(&nullifiers);
    }

    pub fn take_message_from_address_key(
        &mut self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Option<Message> {
        self.message_mapping
            .get_mut(contract)?
            .remove(&pk.to_bytes())
    }

    pub fn root_exists(&self, root: &BlsScalar) -> bool {
        self.roots.get(root).is_some()
    }

    pub fn get_note(&self, pos: u64) -> Option<Note> {
        self.tree.get(pos).map(|l| l.note)
    }

    pub fn sub_balance(
        &mut self,
        address: ContractId,
        value: u64,
    ) -> Option<()> {
        match self.balances.get_mut(&address) {
            Some(balance) => {
                let (bal, underflow) = balance.overflowing_sub(value);
                if underflow {
                    None
                } else {
                    *balance = bal;

                    Some(())
                }
            }
            _ => None,
        }
    }

    pub fn push_message(
        &mut self,
        address: ContractId,
        message_address: StealthAddress,
        message: Message,
    ) {
        let mut to_insert: Option<BTreeMap<[u8; PublicKey::SIZE], Message>> =
            None;

        match self.message_mapping.get_mut(&address) {
            Some(map) => {
                map.insert(message_address.pk_r().to_bytes(), message);
            }

            None => {
                let mut map: BTreeMap<[u8; PublicKey::SIZE], Message> =
                    BTreeMap::default();
                map.insert(message_address.pk_r().to_bytes(), message);
                to_insert.replace(map);
            }
        }

        if let Some(map) = to_insert {
            self.message_mapping.insert(address, map);
        }

        self.message_mapping_set.insert(address, message_address);
    }

    pub fn take_crossover(&mut self) -> Option<(Crossover, StealthAddress)> {
        let crossover = self.var_crossover.take()?;

        let sa = self.var_crossover_addr.take()?;

        Some((crossover, sa))
    }

    pub fn set_crossover(
        &mut self,
        crossover: Option<Crossover>,
        stealth_address: Option<StealthAddress>,
    ) {
        self.var_crossover = crossover;
        self.var_crossover_addr = stealth_address;
    }

    pub fn get_crossover(
        &mut self,
    ) -> (Option<Crossover>, Option<StealthAddress>) {
        (self.var_crossover, self.var_crossover_addr)
    }

    pub fn extend_notes(&mut self, block_height: u64, notes: Vec<Note>) {
        self.tree.extend_notes(block_height, notes);
    }
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
