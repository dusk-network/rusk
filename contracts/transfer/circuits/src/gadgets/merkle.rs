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
    use dusk_plonk::jubjub::{GENERATOR_EXTENDED, Fr};

    #[test]
    fn merkle_gadget() {
        let mut tree: PoseidonTree<_, Blake2b> = PoseidonTree::new(17usize);

        let a = GENERATOR_EXTENDED * Fr::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * Fr::random(&mut rand::thread_rng());
        let psk = PublicSpendKey::new(a, b);
        let note = Note::new(NoteType::Transparent, &psk, 100);
        tree.push(StorageScalar{0: note.hash()}).unwrap();

        // Generate Composer & Public Parameters
        let pub_params = PublicParameters::setup(1 << 17, &mut rand::thread_rng()).unwrap();
        let (ck, vk) = pub_params.trim(1 << 16).unwrap();
        let mut prover = Prover::new(b"test");

        merkle(prover.mut_cs(), tree.poseidon_branch(0u64).unwrap().unwrap(), &note);

        let circuit = prover.preprocess(&ck).unwrap();
        let proof = prover.prove(&ck).unwrap();

        let mut verifier = Verifier::new(b"test");
        merkle(verifier.mut_cs(), tree.poseidon_branch(0u64).unwrap().unwrap(), &note);
        verifier.preprocess(&ck).unwrap();
        
        let pi = verifier.mut_cs().public_inputs.clone();
        verifier.verify(&proof, &vk, &pi).unwrap();
    }
}