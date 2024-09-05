// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities.

#![cfg_attr(target_family = "wasm", no_std)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#[cfg(target_family = "wasm")]
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

extern crate alloc;

#[cfg(target_family = "wasm")]
mod ffi;

pub mod input;
pub mod keys;
pub mod transaction;

pub mod prelude {
    //! Re-export of the most commonly used types and traits.
    pub use crate::keys;
    pub use crate::{input::MAX_INPUT_NOTES, keys::RNG_SEED};
}

use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Serializable, Write};

use execution_core::{
    transfer::phoenix::{
        Note, NoteLeaf, SecretKey as PhoenixSecretKey,
        ViewKey as PhoenixViewKey,
    },
    BlsScalar,
};

/// Filter all notes and their block height that are owned by the given keys,
/// mapped to their nullifiers.
pub fn map_owned(
    keys: impl AsRef<[PhoenixSecretKey]>,
    notes: impl AsRef<[NoteLeaf]>,
) -> BTreeMap<BlsScalar, NoteLeaf> {
    notes
        .as_ref()
        .iter()
        .fold(BTreeMap::new(), |mut notes_map, note_leaf| {
            for sk in keys.as_ref() {
                if sk.owns(note_leaf.note.stealth_address()) {
                    let nullifier = note_leaf.note.gen_nullifier(sk);
                    notes_map.insert(nullifier, note_leaf.clone());
                }
            }
            notes_map
        })
}

/// Calculate the sum for all the given [`Note`]s that belong to the given
/// [`PhoenixViewKey`].
pub fn phoenix_balance<T>(
    phoenix_vk: &PhoenixViewKey,
    notes: impl Iterator<Item = T>,
) -> BalanceInfo
where
    T: AsRef<Note>,
{
    let mut values: Vec<u64> = notes
        .filter_map(|note| note.as_ref().value(Some(phoenix_vk)).ok())
        .collect();

    values.sort_by(|a, b| b.cmp(a));

    let spendable = values.iter().take(input::MAX_INPUT_NOTES).sum();
    let value =
        spendable + values.iter().skip(input::MAX_INPUT_NOTES).sum::<u64>();

    BalanceInfo { value, spendable }
}

/// Information about the balance of a particular key.
#[derive(Debug, Default, Hash, Clone, Copy, PartialEq, Eq)]
pub struct BalanceInfo {
    /// The total value of the balance.
    pub value: u64,
    /// The maximum _spendable_ value in a single transaction. This is
    /// different from `value` since there is a maximum number of notes one can
    /// spend.
    pub spendable: u64,
}

impl Serializable<{ 2 * u64::SIZE }> for BalanceInfo {
    type Error = dusk_bytes::Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut reader = &buf[..];

        let value = u64::from_reader(&mut reader)?;
        let spendable = u64::from_reader(&mut reader)?;

        Ok(Self { value, spendable })
    }

    #[allow(unused_must_use)]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        let mut writer = &mut buf[..];

        writer.write(&self.value.to_bytes());
        writer.write(&self.spendable.to_bytes());

        buf
    }
}
