// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::convert::TryFrom;

use alloc::vec::Vec;
use canonical::{Canon, InvalidEncoding, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubAffine;
use dusk_kelvin_map::Map;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Message, Note};

mod call;
mod tree;

use tree::Tree;

pub use call::Call;

pub type PublicKeyBytes = [u8; PublicKey::SIZE];

#[derive(Debug, Default, Clone, Canon)]
pub struct TransferContract<S: Store> {
    pub(crate) notes: Tree<S>,
    pub(crate) notes_mapping: Map<u64, Vec<Note>, S>,
    pub(crate) nullifiers: Map<BlsScalar, (), S>,
    pub(crate) roots: Map<BlsScalar, (), S>,
    pub(crate) balances: Map<BlsScalar, u64, S>,
    pub(crate) message_mapping:
        Map<BlsScalar, Map<PublicKeyBytes, Message, S>, S>,
    pub(crate) message_mapping_set:
        Map<BlsScalar, (PublicKey, JubJubAffine), S>,

    // FIXME Variable space
    // https://github.com/dusk-network/rusk/issues/213
    pub(crate) var_crossover: Option<Crossover>,
    pub(crate) var_crossover_pk: Option<PublicKey>,
    // TODO not implemented
    pub(crate) circulating_supply: Option<u64>,
}

impl<S: Store> TransferContract<S> {
    pub fn get_note(&self, pos: u64) -> Result<Option<Note>, S::Error> {
        self.notes
            .get(pos)
            .map(|l| l.map(|l| l.note.clone().into()))
            .map_err(|_| InvalidEncoding.into())
    }

    pub(crate) fn push_note(
        &mut self,
        block_height: u64,
        note: Note,
    ) -> Result<Note, S::Error> {
        let pos = self
            .notes
            .push((block_height, note).into())
            .map_err(|_| InvalidEncoding.into())?;

        let note = self.get_note(pos)?.ok_or(InvalidEncoding.into())?;

        let mut create = false;
        match self.notes_mapping.get_mut(&block_height)? {
            // TODO evaluate options for efficient dedup
            // We can't call dedup here because the note `PartialEq` relies on
            // poseidon hash, that is supposed to be a host function
            // https://github.com/dusk-network/rusk/issues/196
            Some(mut mapped) => mapped.push(note.clone()),

            None => create = true,
        }

        if create {
            self.notes_mapping.insert(block_height, [note].to_vec())?;
        }

        Ok(note)
    }

    pub fn notes(&self) -> &Tree<S> {
        &self.notes
    }

    pub fn notes_mapping(&self) -> &Map<u64, Vec<Note>, S> {
        &self.notes_mapping
    }

    pub fn balances(&self) -> &Map<BlsScalar, u64, S> {
        &self.balances
    }

    pub(crate) fn update_root(&mut self) -> Result<(), S::Error> {
        let root = self.notes.root().map_err(|_| InvalidEncoding.into())?;

        self.roots.insert(root, ())?;

        Ok(())
    }
}

impl<S: Store> TryFrom<Note> for TransferContract<S> {
    type Error = S::Error;

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
