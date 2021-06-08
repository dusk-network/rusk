// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, Map};

use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubAffine;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Message, Note};

use core::convert::TryFrom;

mod call;
mod circuits;
mod tree;

use tree::Tree;

pub use call::Call;

pub type PublicKeyBytes = [u8; PublicKey::SIZE];

// TODO rename attributes
#[derive(Debug, Default, Clone, Canon)]
pub struct TransferContract {
    pub(crate) notes: Tree,
    pub(crate) nullifiers: Map<BlsScalar, ()>,
    pub(crate) roots: Map<BlsScalar, ()>,
    pub(crate) balances: Map<BlsScalar, u64>,
    pub(crate) message_mapping: Map<BlsScalar, Map<PublicKeyBytes, Message>>,
    pub(crate) message_mapping_set: Map<BlsScalar, (PublicKey, JubJubAffine)>,

    // FIXME Variable space
    // https://github.com/dusk-network/rusk/issues/213
    pub(crate) var_crossover: Option<Crossover>,
    pub(crate) var_crossover_pk: Option<PublicKey>,
    // TODO not implemented
    pub(crate) circulating_supply: Option<u64>,
}

impl TransferContract {
    pub fn get_note(&self, pos: u64) -> Result<Option<Note>, Error> {
        Ok(self.notes.get(pos).map(|l| l.map(|l| l.into()))?)
    }

    pub(crate) fn push_note(
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

    pub fn notes_from_height(
        &self,
        block_height: u64,
    ) -> Result<impl Iterator<Item = Result<&Note, Error>>, Error> {
        self.notes.notes(block_height)
    }

    pub fn balances(&self) -> &Map<BlsScalar, u64> {
        &self.balances
    }

    pub(crate) fn update_root(&mut self) -> Result<(), Error> {
        let root = self.notes.root()?;

        self.roots.insert(root, ())?;

        Ok(())
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
