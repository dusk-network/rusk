// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::convert::TryFrom;

use alloc::vec::Vec;
use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubAffine;
use dusk_kelvin_map::Map;
use dusk_pki::PublicKey;
use phoenix_core::{Message, Note};

mod call;
mod tree;

use tree::Tree;

pub use call::{Call, InternalCall, InternalCallResult, TransferExecute};

#[cfg(target_arch = "wasm32")]
pub(crate) use tree::TRANSFER_TREE_DEPTH;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Canon)]
pub struct PublicKeyBytes([u8; PublicKey::SIZE]);

impl From<PublicKey> for PublicKeyBytes {
    fn from(pk: PublicKey) -> Self {
        Self(pk.to_bytes())
    }
}

#[derive(Debug, Default, Clone, Canon)]
pub struct Transfer<S: Store> {
    pub(crate) notes: Tree<S>,
    pub(crate) notes_mapping: Map<u64, Vec<Note>, S>,
    pub(crate) nullifiers: Map<BlsScalar, (), S>,
    pub(crate) roots: Map<BlsScalar, (), S>,
    pub(crate) balance: Map<BlsScalar, u64, S>,
    pub(crate) message_mapping:
        Map<BlsScalar, Map<PublicKeyBytes, Message, S>, S>,
    pub(crate) message_mapping_set:
        Map<BlsScalar, (PublicKey, JubJubAffine), S>,
}

impl<S: Store> Transfer<S> {
    pub(crate) fn update_root(&mut self) -> Result<(), S::Error> {
        let root = self.notes.root()?;

        self.roots.insert(root, ())?;

        Ok(())
    }
}

impl<S: Store> TryFrom<Note> for Transfer<S> {
    type Error = S::Error;

    /// This implementation is intended for test purposes to initialize the
    /// state with the provided note
    ///
    /// To avoid abuse, the block_height will always be `0`
    fn try_from(note: Note) -> Result<Self, Self::Error> {
        use canonical::InvalidEncoding;

        let mut transfer = Self::default();

        let block_height = 0;
        transfer
            .notes_mapping
            .insert(block_height, [note].to_vec())?;

        transfer
            .notes
            .as_mut()
            .push((block_height, note).into())
            .map_err(|_| InvalidEncoding.into())?;

        transfer.update_root()?;

        Ok(transfer)
    }
}
