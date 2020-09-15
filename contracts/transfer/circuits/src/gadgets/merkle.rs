// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details. 

use phoenix-core::note::{Note}
use dusk_plonk::constraint_system::ecc::scalar_mul::fixed_base::scalar_mul;
use dusk_plonk::jubjub::{
    Fr, AffinePoint, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_plonk::prelude::*;
use poseidon252::merkle_proof::{merkle_opening_gadget, PoseidonBranch};

pub fn merkle(composer: &mut StandardComposer, branch: PoseidonBranch, note: &Note) {

    let leaf = composer.add_input(Note.note_type.hash());
    let root = branch.root;

    merkle_opening_gadget(composer, branch, leaf, root);
}