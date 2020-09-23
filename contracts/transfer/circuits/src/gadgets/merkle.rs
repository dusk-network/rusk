// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use phoenix_core::note::Note;
use dusk_plonk::prelude::*;
use poseidon252::merkle_proof::{merkle_opening_gadget, PoseidonBranch};
use plonk_gadgets::AllocatedScalar;


pub fn merkle(composer: &mut StandardComposer, branch: PoseidonBranch, note_hash: AllocatedScalar) {

    let leaf = note_hash.var;
    let root = branch.root;

    merkle_opening_gadget(composer, branch, leaf, root);
}

#[cfg(test)]
mod commitment_tests {
    use super::*;
    use phoenix_core::note::NoteType;
    use dusk_plonk::commitment_scheme::kzg10::PublicParameters;
    use dusk_plonk::proof_system::{Prover, Verifier};
    use poseidon252::{PoseidonTree, StorageScalar};
    use kelvin::Blake2b;
    use dusk_pki::PublicSpendKey;
    use rand::Rng;
    use dusk_plonk::jubjub::GENERATOR_EXTENDED;

    #[test]
    fn merkle_gadget() {
        let mut tree: PoseidonTree<_, Blake2b> = PoseidonTree::new(17usize);

        let a = GENERATOR_EXTENDED * JubJubScalar::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * JubJubScalar::random(&mut rand::thread_rng());
        let psk = PublicSpendKey::new(a, b);
        let note = Note::new(NoteType::Transparent, &psk, 100);
        tree.push(StorageScalar{0: note.hash()}).unwrap();

        
        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut prover = Prover::new(b"test");

        let note_hash = AllocatedScalar::allocate(prover.mut_cs(), note.hash());

        merkle(prover.mut_cs(), tree.poseidon_branch(0u64).unwrap().unwrap(), note_hash);

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");

        let note_hash = AllocatedScalar::allocate(verifier.mut_cs(), note.hash());

        merkle(verifier.mut_cs(), tree.poseidon_branch(0u64).unwrap().unwrap(), note_hash);
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}