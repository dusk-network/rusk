// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, Map};

use canonical_derive::Canon;
use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::{Crossover, Fee, Message, Note};
use rusk_abi::hash::Hasher;

use core::convert::TryFrom;

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

    pub fn update_root(&mut self) -> Result<(), Error> {
        let root = self.notes.root()?;

        self.roots.insert(root, ())?;

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
