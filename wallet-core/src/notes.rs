// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities to create transactions.

use alloc::vec::Vec;
use core::ops::Index;
use execution_core::{
    transfer::phoenix::{NoteLeaf, SecretKey as PhoenixSecretKey},
    BlsScalar,
};

use rkyv::{Archive, Deserialize, Serialize};

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
            eprintln!("Printing note...");
            dbg!(note_leaf);
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
