// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Provides functions and types for interacting with notes.

use alloc::vec::Vec;
use core::ops::Index;
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use execution_core::transfer::phoenix::{Note, ViewKey as PhoenixViewKey};
use execution_core::{
    transfer::phoenix::{NoteLeaf, SecretKey as PhoenixSecretKey},
    BlsScalar,
};

use rkyv::{Archive, Deserialize, Serialize};

// The maximum amount of input notes that can be spend in one
// phoenix-transaction
const MAX_INPUT_NOTES: usize = 4;

/// A collection of notes stored as key-value pairs.
/// The key is a `BlsScalar` and the value is a `NoteLeaf`.
/// Duplicates are allowed.
#[derive(Default, Archive, Serialize, Deserialize, Debug)]
pub struct OwnedList {
    /// The underlying storage of key-value pairs where
    /// `BlsScalar` is the key and `NoteLeaf` is the value.
    entries: Vec<(BlsScalar, NoteLeaf)>,
}

impl OwnedList {
    /// Inserts a new key-value pair into the collection.
    pub fn insert(&mut self, key: BlsScalar, value: NoteLeaf) {
        self.entries.push((key, value));
    }

    /// Returns the number of entries (key-value pairs) in the collection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Checks if the collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retrieves the value (`NoteLeaf`) associated with a given key
    #[must_use]
    pub fn get(&self, key: &BlsScalar) -> Option<&NoteLeaf> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Retrieves all keys in the collection.
    #[must_use]
    pub fn keys(&self) -> Vec<BlsScalar> {
        self.entries.iter().map(|(k, _)| *k).collect()
    }
}

impl Index<&BlsScalar> for OwnedList {
    type Output = NoteLeaf;

    /// Retrieves the value (`NoteLeaf`) associated with a given key
    /// (`BlsScalar`).
    ///
    /// Panics if the key is not found in the collection.
    fn index(&self, index: &BlsScalar) -> &Self::Output {
        self.get(index).expect("key not found")
    }
}

/// Filter all notes and their block height that are owned by the given keys,
/// mapped to their nullifiers.
pub fn map_owned(
    keys: impl AsRef<[PhoenixSecretKey]>,
    notes: impl AsRef<[NoteLeaf]>,
) -> OwnedList {
    notes.as_ref().iter().fold(
        OwnedList::default(),
        |mut notes_map, note_leaf| {
            for sk in keys.as_ref() {
                if sk.owns(note_leaf.note.stealth_address()) {
                    let nullifier = note_leaf.note.gen_nullifier(sk);
                    notes_map.insert(nullifier, note_leaf.clone());
                    break;
                }
            }
            notes_map
        },
    )
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
