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

pub struct Tree {
    tree: NotesTree,
    // Since `dusk-merkle` does not include data blocks with the tree, we do it
    // here.
    leaves: Vec<NoteLeaf>,
}

impl Tree {
    pub const fn new() -> Self {
        Self {
            tree: NotesTree::new(),
            leaves: Vec::new(),
        }
    }

    pub fn push(&mut self, mut leaf: NoteLeaf) -> Note {
        // update the position before computing the hash
        let pos = self.leaves.len() as u64;
        leaf.note.set_pos(pos);

        // compute the item that goes in the leaf of the tree
        let hash = rusk_abi::poseidon_hash(leaf.note.hash_inputs().to_vec());
        let item = NoteTreeItem { hash, data: () };

        self.tree.insert(pos, item);
        self.leaves.push(leaf.clone());

        leaf.note
    }

    pub fn extend_notes<I: IntoIterator<Item = Note>>(
        &mut self,
        block_height: u64,
        notes: I,
    ) -> Vec<Note> {
        let mut n = Vec::new();

        for note in notes {
            // skip transparent notes with a value of 0
            if !note.value(None).is_ok_and(|value| value == 0) {
                let note = self.push(NoteLeaf { block_height, note });
                n.push(note);
            }
        }

        n
    }

    pub fn root(&self) -> BlsScalar {
        self.tree.root().hash
    }

    /// Return an iterator through the leaves in the tree, starting from a given
    /// `height`.
    pub fn leaves(&self, height: u64) -> impl Iterator<Item = &NoteLeaf> {
        // We can do this since we know the leaves are strictly increasing in
        // block height. If this ever changes - such as in the case of a
        // sparsely populated tree - we should annotate the tree and use
        // `Tree::walk` instead.
        self.leaves
            .iter()
            .skip_while(move |leaf| leaf.block_height < height)
    }

    /// Return an iterator through the leaves in the tree, starting from a given
    /// `position`.
    pub fn leaves_pos(&self, pos: u64) -> impl Iterator<Item = &NoteLeaf> {
        // We can do this since we know the leaves are strictly increasing in
        // block height. If this ever changes - such as in the case of a
        // sparsely populated tree - we should annotate the tree and use
        // `Tree::walk` instead.
        let pos = pos as usize;
        if self.leaves.len() < pos {
            return self.leaves[..0].iter();
        }
        self.leaves[pos..].iter()
    }

    pub fn opening(&self, pos: u64) -> Option<NoteOpening> {
        self.tree.opening(pos)
    }

    pub fn leaves_len(&self) -> u64 {
        self.leaves.len() as u64
    }
}
