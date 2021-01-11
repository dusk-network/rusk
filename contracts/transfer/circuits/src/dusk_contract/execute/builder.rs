// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::ExecuteCircuit;

use canonical::Store;
use dusk_pki::{Ownable, SecretSpendKey, ViewKey};
use dusk_plonk::bls12_381::BlsScalar;
use dusk_plonk::jubjub::{JubJubAffine, JubJubScalar};
use phoenix_core::Note;
use poseidon252::tree::{PoseidonLeaf, PoseidonTree, PoseidonTreeAnnotation};
use poseidon252::Error as PoseidonError;
use schnorr::double_key::{PublicKeyPair, SecretKey as SignatureSecretKey};

use std::marker::PhantomData;

/// Helper to create execute circuits
#[derive(Debug, Default, Clone)]
pub struct ExecuteCircuitBuilder<L, A, S, const DEPTH: usize>
where
    L: PoseidonLeaf<S>,
    A: PoseidonTreeAnnotation<L, S>,
    S: Store,
{
    circuit: ExecuteCircuit,
    tree: PhantomData<(L, A, S)>,
}

impl<L, A, S, const DEPTH: usize> ExecuteCircuitBuilder<L, A, S, DEPTH>
where
    L: PoseidonLeaf<S>,
    A: PoseidonTreeAnnotation<L, S>,
    S: Store,
{
    /// Appends an input note
    pub fn input(
        mut self,
        tree: &PoseidonTree<L, A, S, DEPTH>,
        note: &Note,
        ssk: &SecretSpendKey,
    ) -> Result<Self, PoseidonError<S::Error>> {
        let vk = ViewKey::from(ssk);

        let nullifier = note.gen_nullifier(ssk);

        let hash = note.hash();
        let pos = BlsScalar::from(note.pos());
        let note_type = BlsScalar::from(note.note() as u64);
        let value_commitment: JubJubAffine = note.value_commitment().into();
        let nonce = *note.nonce();
        let value =
            JubJubScalar::from(note.value(Some(&vk)).unwrap_or_default());
        let blinder = note.blinding_factor(Some(&vk)).unwrap_or_default();
        let R: JubJubAffine = note.stealth_address().R().into();
        let cipher = *note.cipher();

        let branch = tree.branch(note.pos() as usize)?.unwrap_or_default();

        let sk_r = ssk.sk_r(note.stealth_address());
        let pk_r: JubJubAffine = note.stealth_address().pk_r().into();

        let sig_sk: SignatureSecretKey = sk_r.into();
        let sig_pk = PublicKeyPair::from(&sig_sk);
        let sig_pk_prime: JubJubAffine = sig_pk.PK_prime().into();
        let sig_pk: JubJubAffine = sig_pk.PK().into();
        let sig = sig_sk.sign(&mut rand::thread_rng(), BlsScalar::one());

        Ok(self)
    }

    /// Finalize the circuit construction
    pub fn build(self) -> ExecuteCircuit {
        self.circuit
    }
}
