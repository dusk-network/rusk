// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Error;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use core::convert::TryFrom;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::{Crossover, Message, Note};
use rusk_abi::ModuleId;

mod circuits;
mod tree;

pub use tree::Leaf;
use tree::Tree;

#[derive(Debug, Clone)]
pub struct TransferContract {
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

impl TransferContract {
    pub const fn new() -> TransferContract {
        TransferContract {
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
        self.tree.get(pos).map(|l| l.into())
    }

    /// Push a note to the contract's state with the given block height
    ///
    /// Note: the method `update_root` needs to be called after the last note is
    /// pushed.
    pub fn push_note(
        &mut self,
        block_height: u64,
        note: Note,
    ) -> Result<Note, Error> {
        let pos = self.tree.push((block_height, note).into());
        let note = self.get_note(pos).ok_or(Error::NoteNotFound)?;

        Ok(note)
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn message(
        &self,
        contract: &ModuleId,
        pk: &PublicKey,
    ) -> Result<Message, Error> {
        let map = self
            .message_mapping
            .get(contract)
            .ok_or(Error::ContractNotFound)?;

        let message = map.get(&pk.to_bytes()).ok_or(Error::MessageNotFound)?;

        Ok(*message)
    }

    pub fn leaves_from_height(
        &self,
        block_height: u64,
    ) -> Option<impl Iterator<Item = &Leaf>> {
        self.tree.leaves(block_height)
    }

    pub fn balances(&self) -> &BTreeMap<ModuleId, u64> {
        &self.balances
    }

    pub fn add_balance(
        &mut self,
        address: ModuleId,
        value: u64,
    ) -> Result<(), Error> {
        if let Some(balance) = self.balances.get_mut(&address) {
            *balance += value;
            return Ok(());
        }

        self.balances.insert(address, value);

        Ok(())
    }

    pub fn update_root(&mut self) -> Result<(), Error> {
        let root = self.tree.root();
        self.roots.insert(root);
        Ok(())
    }

    pub fn any_nullifier_exists(&self, nullifiers: &[BlsScalar]) -> bool {
        nullifiers
            .iter()
            .fold(false, |t, n| t || self.nullifiers.get(n).is_some())
    }

    /// Takes a slice of nullifiers and returns a vector containing the ones
    /// that already exists in the contract
    pub fn find_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Vec<BlsScalar> {
        nullifiers
            .iter()
            .copied()
            .filter_map(|n| self.nullifiers.get(&n).map(|_| n))
            .collect()
    }
}

impl TryFrom<Note> for TransferContract {
    type Error = Error;

    /// This implementation is intended for test purposes to initialize the
    /// state with the provided note
    ///
    /// To avoid abuse, the block_height will always be `0`
    fn try_from(note: Note) -> Result<Self, Self::Error> {
        let mut transfer = Self::new();

        let block_height = 0;
        transfer.push_note(block_height, note)?;
        transfer.update_root()?;

        Ok(transfer)
    }
}

#[cfg(test)]
mod test_transfer {
    use super::*;

    #[test]
    fn find_existing_nullifiers() -> Result<(), Error> {
        let mut transfer = TransferContract::new();

        let (zero, one, two, three, ten, eleven) = (
            BlsScalar::from(0),
            BlsScalar::from(1),
            BlsScalar::from(2),
            BlsScalar::from(3),
            BlsScalar::from(10),
            BlsScalar::from(11),
        );

        let existing = transfer
            .find_existing_nullifiers(&[zero, one, two, three, ten, eleven]);

        assert_eq!(existing.len(), 0);

        for i in 1..10 {
            transfer.nullifiers.insert(BlsScalar::from(i));
        }

        let existing = transfer
            .find_existing_nullifiers(&[zero, one, two, three, ten, eleven]);

        assert_eq!(existing.len(), 3);

        assert!(existing.contains(&one));
        assert!(existing.contains(&two));
        assert!(existing.contains(&three));

        Ok(())
    }
}
