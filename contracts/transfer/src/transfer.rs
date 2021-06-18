// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, Map};

use canonical_derive::Canon;
use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::{Crossover, Message, Note};

use core::convert::TryFrom;

mod call;
mod circuits;
mod tree;

use tree::Tree;

pub use call::Call;

pub type PublicKeyBytes = [u8; PublicKey::SIZE];

#[derive(Debug, Default, Clone, Canon)]
pub struct TransferContract {
    pub(crate) notes: Tree,
    pub(crate) nullifiers: Map<BlsScalar, ()>,
    pub(crate) roots: Map<BlsScalar, ()>,
    pub(crate) balances: Map<ContractId, u64>,
    pub(crate) message_mapping: Map<ContractId, Map<PublicKeyBytes, Message>>,
    pub(crate) message_mapping_set: Map<ContractId, StealthAddress>,
    pub(crate) var_crossover: Option<Crossover>,
    pub(crate) var_crossover_pk: Option<PublicKey>,
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

    pub fn message(
        &self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Result<Message, Error> {
        let map = self
            .message_mapping
            .get(contract)?
            .ok_or(Error::ContractNotFound)?;

        let message = map.get(&pk.to_bytes())?.ok_or(Error::MessageNotFound)?;

        Ok(*message)
    }

    pub fn notes_from_height(
        &self,
        block_height: u64,
    ) -> Result<impl Iterator<Item = Result<&Note, Error>>, Error> {
        self.notes.notes(block_height)
    }

    pub fn balances(&self) -> &Map<ContractId, u64> {
        &self.balances
    }

    pub(crate) fn update_root(&mut self) -> Result<(), Error> {
        let root = self.notes.root()?;

        self.roots.insert(root, ())?;

        Ok(())
    }

    pub fn contract_to_scalar(address: &ContractId) -> BlsScalar {
        // TODO provisory fn until native ContractId -> BlsScalar conversion is
        // implemented
        // https://github.com/dusk-network/cargo-bake/issues/1

        // ContractId don't have an API to extract internal bytes - so we
        // provisorily trust it is 32 bytes
        let mut scalar = [0u8; 32];
        scalar.copy_from_slice(address.as_bytes());

        // Truncate the contract id to fit bls
        scalar[31] &= 0x3f;

        BlsScalar::from_bytes(&scalar).unwrap_or_default()
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
