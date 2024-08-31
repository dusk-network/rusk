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

extern crate alloc;

pub mod keys;
pub mod transaction;

mod error;
pub use error::Error;

// The maximum amount of input notes that can be spend in one
// phoenix-transaction
const MAX_INPUT_NOTES: usize = 4;

use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Serializable, Write};

use execution_core::{
    transfer::phoenix::{
        Note, SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
    },
    BlsScalar,
};

/// Tuple containing Note and block height
pub type EnrichedNote = (Note, u64);

/// Filter all notes and their block height that are owned by the given keys,
/// mapped to their nullifiers.
pub fn map_owned(
    keys: impl AsRef<[PhoenixSecretKey]>,
    notes: impl AsRef<[EnrichedNote]>,
) -> BTreeMap<BlsScalar, EnrichedNote> {
    notes.as_ref().iter().fold(
        BTreeMap::new(),
        |mut notes_map, enriched_note| {
            for sk in keys.as_ref() {
                if sk.owns(enriched_note.0.stealth_address()) {
                    let nullifier = enriched_note.0.gen_nullifier(sk);
                    notes_map.insert(nullifier, enriched_note.clone());
                }
            }
            notes_map
        },
    )
}

/// Calculate the sum for all the given [`Note`]s that belong to the given
/// [`PhoenixViewKey`].
pub fn phoenix_balance(
    phoenix_vk: &PhoenixViewKey,
    unspent_notes: impl AsRef<[Note]>,
) -> BalanceInfo {
    let mut values: Vec<u64> = Vec::new();
    let unspent_notes = unspent_notes.as_ref();
    for note in unspent_notes {
        values.push(note.value(Some(phoenix_vk)).unwrap_or_default());
    }

    values.sort_by(|a, b| b.cmp(a));

    let spendable = values.iter().take(MAX_INPUT_NOTES).sum();
    let value = spendable + values.iter().skip(MAX_INPUT_NOTES).sum::<u64>();

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
