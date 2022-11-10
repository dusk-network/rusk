// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::collections::btree_map::Entry;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::{Crossover, Message, Note};
use rusk_abi::ModuleId;
use transfer_contract_types::TreeLeaf;

mod circuits;
mod tree;

use tree::Tree;

#[derive(Debug, Clone)]
pub struct TransferState {
    pub(crate) tree: Tree,
    pub(crate) nullifiers: BTreeSet<BlsScalar>,
    pub(crate) roots: BTreeSet<BlsScalar>,
    pub(crate) balances: BTreeMap<ModuleId, u64>,
    pub(crate) message_mapping:
        BTreeMap<ModuleId, BTreeMap<[u8; PublicKey::SIZE], Message>>,
    pub(crate) message_mapping_set: BTreeMap<ModuleId, StealthAddress>,
    pub(crate) var_crossover: Option<Crossover>,
    pub(crate) var_crossover_pk: Option<PublicKey>,
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
            var_crossover_pk: None,
        }
    }

    pub fn get_note(&self, pos: u64) -> Option<Note> {
        self.tree.get(pos).map(|l| l.note)
    }

    /// Push a note to the contract's state with the given block height
    ///
    /// Note: the method `update_root` needs to be called after the last note is
    /// pushed.
    pub fn push_note(&mut self, block_height: u64, note: Note) -> Note {
        let pos = self.tree.push(TreeLeaf { block_height, note });
        self.get_note(pos)
            .expect("There should be a note that was just inserted")
    }

    pub fn leaves_in_range(&self, range: Range<u64>) -> Vec<TreeLeaf> {
        match self.tree.leaves(range) {
            Some(leaves) => leaves.cloned().collect(),
            None => vec![],
        }
    }

    pub fn add_balance(&mut self, module: ModuleId, value: u64) {
        match self.balances.entry(module) {
            Entry::Vacant(ve) => {
                ve.insert(value);
            }
            Entry::Occupied(mut oe) => {
                let v = oe.get_mut();
                *v += value
            }
        }
    }

    pub fn update_root(&mut self) -> BlsScalar {
        let root = self.tree.root();
        self.roots.insert(root);
        root
    }

    pub fn any_nullifier_exists(&self, nullifiers: &[BlsScalar]) -> bool {
        nullifiers
            .iter()
            .fold(false, |t, n| t || self.nullifiers.get(n).is_some())
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
}

impl From<Note> for TransferState {
    /// This implementation is intended for test purposes to initialize the
    /// state with the provided note
    ///
    /// To avoid abuse, the block_height will always be `0`
    fn from(note: Note) -> Self {
        let mut transfer = Self::new();

        let block_height = 0;
        transfer.push_note(block_height, note);
        transfer.update_root();

        transfer
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
