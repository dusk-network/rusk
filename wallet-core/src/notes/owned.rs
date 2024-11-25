// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Provides functions and types to handle notes' ownership.

use alloc::vec;
use alloc::vec::Vec;
use core::ops::Index;
use core::slice::Iter;

use bytecheck::CheckBytes;
use execution_core::transfer::phoenix::{
    NoteLeaf, SecretKey as PhoenixSecretKey,
};
use execution_core::BlsScalar;
use rkyv::{Archive, Deserialize, Serialize};

/// A collection of notes stored as key-value pairs.
/// The key is a `BlsScalar` and the value is a `NoteLeaf`.
/// Duplicates are allowed.
#[derive(Default, Archive, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[archive_attr(derive(CheckBytes))]
pub struct NoteList {
    /// The underlying storage of key-value pairs where
    /// `BlsScalar` is the key and `NoteLeaf` is the value.
    entries: Vec<(BlsScalar, NoteLeaf)>,
}

impl NoteList {
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

    /// Returns an iterator over the key-value pairs.
    pub fn iter(&self) -> Iter<'_, (BlsScalar, NoteLeaf)> {
        self.entries.iter()
    }
}

impl Index<&BlsScalar> for NoteList {
    type Output = NoteLeaf;

    /// Retrieves the value (`NoteLeaf`) associated with a given key
    /// (`BlsScalar`).
    ///
    /// Panics if the key is not found in the collection.
    fn index(&self, index: &BlsScalar) -> &Self::Output {
        self.get(index).expect("key not found")
    }
}

impl<'a> IntoIterator for &'a NoteList {
    type IntoIter = core::slice::Iter<'a, (BlsScalar, NoteLeaf)>;
    type Item = &'a (BlsScalar, NoteLeaf);
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl From<Vec<(BlsScalar, NoteLeaf)>> for NoteList {
    fn from(entries: Vec<(BlsScalar, NoteLeaf)>) -> Self {
        NoteList { entries }
    }
}

/// Filter all notes and their block height that are owned by the given keys,
/// mapped to their nullifiers.
pub fn map(
    keys: impl AsRef<[PhoenixSecretKey]>,
    notes: impl AsRef<[NoteLeaf]>,
) -> Vec<NoteList> {
    notes.as_ref().iter().fold(
        vec![NoteList::default(); keys.as_ref().len()],
        |mut notes_maps, note_leaf| {
            for (i, sk) in keys.as_ref().iter().enumerate() {
                if sk.owns(note_leaf.note.stealth_address()) {
                    let nullifier = note_leaf.note.gen_nullifier(sk);
                    notes_maps[i].insert(nullifier, note_leaf.clone());
                    break;
                }
            }
            notes_maps
        },
    )
}
