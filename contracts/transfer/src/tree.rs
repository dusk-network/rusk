// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use execution_core::{
    transfer::phoenix::{Note, NoteLeaf, NoteOpening, NoteTreeItem, NotesTree},
    BlsScalar,
};

/// The merkle tree that holds all phoenix-notes.
///
/// This tree is append only. When a note is spend its `nullifier` will be
/// added to the nullifier-set in the transfer-contract's state.
/// To get all unspend phoenix-notes one needs to get all owned notes and
/// remove the ones who's nullifiers are already in the nullifier-set of the
/// transfer-contract.
/// To help with sync-time, we store the block-height at which a note has been
/// added to the tree together with the note itself.
pub struct Tree {
    // Merkle tree of the note-hashes.
    tree: NotesTree,
    // Since the merkle-tree only includes the note-hashes, we we store the
    // actual notes (and their respective block-height) here.
    // The index of a note in this vector corresponds to the position of its
    // hash in the merkle-tree.
    leaves: Vec<NoteLeaf>,
}

impl Tree {
    /// Create a new empty tree.
    pub const fn new() -> Self {
        Self {
            tree: NotesTree::new(),
            leaves: Vec::new(),
        }
    }

    /// Push one [`NoteLeaf`] onto the tree, filtering out notes that are
    /// transparent with a value of 0.
    pub fn push(&mut self, mut leaf: NoteLeaf) -> Option<Note> {
        // skip transparent notes with a value of 0
        if leaf.note.value(None).is_ok_and(|value| value == 0) {
            return None;
        }

        // update the position before computing the hash
        let pos = self.leaves.len() as u64;
        leaf.note.set_pos(pos);

        // compute the item that goes in the leaf of the tree
        let hash = rusk_abi::poseidon_hash(leaf.note.hash_inputs().to_vec());
        let item = NoteTreeItem { hash, data: () };

        self.tree.insert(pos, item);
        self.leaves.push(leaf.clone());

        Some(leaf.note)
    }

    /// Extend the tree with multiple [`NoteLeaf`] of the same block-height,
    /// filtering out notes that are transparent with a value of 0.
    pub fn extend_notes<I: IntoIterator<Item = Note>>(
        &mut self,
        block_height: u64,
        notes: I,
    ) -> Vec<Note> {
        let mut notes_vec = Vec::new();

        for note in notes {
            if let Some(note) = self.push(NoteLeaf { block_height, note }) {
                notes_vec.push(note);
            }
        }

        notes_vec
    }

    /// Return the root of the merkle tree of notes.
    pub fn root(&self) -> BlsScalar {
        self.tree.root().hash
    }

    /// Return an iterator through the leaves in the tree, starting from a given
    /// `block_height`.
    pub fn leaves(&self, block_height: u64) -> impl Iterator<Item = &NoteLeaf> {
        // We can do this since we know the leaves are strictly increasing in
        // block-height. If this ever changes - such as in the case of a
        // sparsely populated tree - we should annotate the tree and use
        // `Tree::walk` instead.
        self.leaves
            .iter()
            .skip_while(move |leaf| leaf.block_height < block_height)
    }

    /// Return an iterator through the leaves in the tree, starting from a given
    /// `position`.
    pub fn leaves_pos(&self, pos: u64) -> impl Iterator<Item = &NoteLeaf> {
        // We can do this since we know that, with increasing position, the
        // leaves are strictly increasing in block-height. If this ever changes
        // - such as in the case of a sparsely populated tree - we should
        // annotate the tree and use `Tree::walk` instead.
        let pos = pos as usize;
        if self.leaves.len() < pos {
            return self.leaves[..0].iter();
        }
        self.leaves[pos..].iter()
    }

    /// Return the merkle-opening for a note at a given position.
    pub fn opening(&self, pos: u64) -> Option<NoteOpening> {
        self.tree.opening(pos)
    }

    /// Return the amount of leaves, i.e. notes, that are stored in the tree.
    pub fn leaves_len(&self) -> u64 {
        self.leaves.len() as u64
    }
}
