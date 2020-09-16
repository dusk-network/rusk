// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

use phoenix_core::note::Note;
use dusk_plonk::prelude::*;
use poseidon252::merkle_proof::{merkle_opening_gadget, PoseidonBranch};

pub fn merkle(composer: &mut StandardComposer, branch: PoseidonBranch, note: &Note) {

    let leaf = composer.add_input(note.hash());
    let root = branch.root;

    merkle_opening_gadget(composer, branch, leaf, root);
}