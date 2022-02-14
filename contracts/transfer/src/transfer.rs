// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, Map};

use alloc::vec::Vec;
use canonical_derive::Canon;
use core::convert::TryFrom;
use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use microkelvin::Link;
use phoenix_core::{Crossover, Fee, Message, Note};
use rusk_abi::hash::Hasher;

mod call;
#[cfg(feature = "circuits")]
mod circuits;
#[cfg(not(target_arch = "wasm32"))]
mod host;
mod tree;

use tree::Tree;

pub use call::Call;

pub type PublicKeyBytes = [u8; PublicKey::SIZE];

#[derive(Debug, Default, Clone, Canon)]
pub struct TransferContract {
    pub(crate) notes: Tree,
    pub(crate) nullifiers: Link<Map<BlsScalar, ()>, ()>,
    pub(crate) roots: Link<Map<BlsScalar, ()>, ()>,
    pub(crate) balances: Link<Map<ContractId, u64>, ()>,
    pub(crate) message_mapping:
        Link<Map<ContractId, Map<PublicKeyBytes, Message>>, ()>,
    pub(crate) message_mapping_set: Link<Map<ContractId, StealthAddress>, ()>,
    pub(crate) var_crossover: Option<Crossover>,
    pub(crate) var_crossover_pk: Option<PublicKey>,
}

impl TransferContract {
    pub fn get_note(&self, pos: u64) -> Result<Option<Note>, Error> {
        self.notes.get(pos).map(|l| l.map(|l| l.into()))
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
        let pos = self.notes.push((block_height, note).into())?;
        let note = self.get_note(pos)?.ok_or(Error::NoteNotFound)?;

        Ok(note)
    }

    pub fn notes(&self) -> &Tree {
        &self.notes
    }

    pub fn message(
        &self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Result<Message, Error> {
        let inner = self.message_mapping.inner()?;
        let map = inner.get(contract)?.ok_or(Error::ContractNotFound)?;
        let message = map.get(&pk.to_bytes())?.ok_or(Error::MessageNotFound)?;

        Ok(*message)
    }

    pub fn notes_from_height(
        &self,
        block_height: u64,
    ) -> Result<impl Iterator<Item = Result<&Note, Error>>, Error> {
        self.notes.notes(block_height)
    }

    pub fn balances(&self) -> Result<Map<ContractId, u64>, Error> {
        Ok(self.balances.inner()?.clone())
    }

    pub fn update_root(&mut self) -> Result<(), Error> {
        let mut inner = self.roots.inner_mut()?;
        let root = self.notes.root()?;
        inner.insert(root, ())?;

        Ok(())
    }

    pub fn tx_hash(
        nullifiers: &[BlsScalar],
        outputs: &[Note],
        anchor: &BlsScalar,
        fee: &Fee,
        crossover: Option<&Crossover>,
        call: Option<&(ContractId, Transaction)>,
    ) -> BlsScalar {
        let mut hasher = Hasher::new();

        nullifiers.iter().for_each(|n| hasher.update(n.to_bytes()));
        outputs.iter().for_each(|o| hasher.update(o.to_bytes()));

        hasher.update(anchor.to_bytes());
        hasher.update(fee.to_bytes());

        if let Some(c) = crossover {
            hasher.update(c.to_bytes());
        };

        if let Some((cid, txdata)) = call {
            hasher.update(cid.as_bytes());
            hasher.update(txdata.as_bytes());
        };

        hasher.finalize()
    }

    pub fn any_nullifier_exists(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<bool, Error> {
        nullifiers.iter().try_fold(false, |t, n| {
            Ok(t || self.nullifiers.inner()?.get(n).map(|n| n.is_some())?)
        })
    }

    /// Takes a slice of nullifiers and returns a vector containing the ones
    /// that already exists in the contract
    pub fn find_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Error> {
        let self_nullifiers = self.nullifiers.inner()?;
        nullifiers
            .iter()
            .copied()
            .filter_map(|n| {
                self_nullifiers
                    .get(&n)
                    .map(|v| v.and(Some(n)))
                    .map_err(|e| e.into())
                    .transpose()
            })
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
        let mut transfer = Self::default();

        let block_height = 0;
        transfer.push_note(block_height, note)?;
        transfer.update_root()?;

        Ok(transfer)
    }
}

#[cfg(test)]
mod test_transfer {
    use super::*;
    use canonical::Canon;

    #[test]
    fn find_existing_nullifiers() -> Result<(), Error> {
        let mut transfer = TransferContract::default();

        let (zero, one, two, three, ten, eleven) = (
            BlsScalar::from(0),
            BlsScalar::from(1),
            BlsScalar::from(2),
            BlsScalar::from(3),
            BlsScalar::from(10),
            BlsScalar::from(11),
        );

        let existing = transfer
            .find_existing_nullifiers(&[zero, one, two, three, ten, eleven])?;

        assert_eq!(existing.len(), 0);

        for i in 1..10 {
            transfer
                .nullifiers
                .inner_mut()
                .unwrap()
                .insert(BlsScalar::from(i), ())?;
        }

        let existing = transfer
            .find_existing_nullifiers(&[zero, one, two, three, ten, eleven])?;

        assert_eq!(existing.len(), 3);

        assert!(existing.contains(&one));
        assert!(existing.contains(&two));
        assert!(existing.contains(&three));

        Ok(())
    }
}
