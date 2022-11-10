// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use crate::hash::Hasher;
use crate::PublicInput;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature as BlsSignature, APK};
use dusk_pki::PublicKey;
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;

/// Compute the blake2b hash of the given scalars, returning the resulting
/// scalar. The output of the hasher is truncated (last nibble) to fit onto a
/// scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    Hasher::digest(bytes)
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    dusk_poseidon::sponge::hash(&scalars)
}

/// Verify a proof is valid for a given circuit type and public inputs
///
/// # Panics
/// This will panic if `verifier_data` is not valid.
pub fn verify_proof(
    verifier_data: Vec<u8>,
    proof: Proof,
    public_inputs: Vec<PublicInput>,
) -> bool {
    let verifier =
        Verifier::<DummyCircuit>::try_from_bytes(verifier_data).expect("");

    let n_pi = public_inputs.iter().fold(0, |num, pi| {
        num + match pi {
            PublicInput::Point(_) => 2,
            PublicInput::BlsScalar(_) => 1,
            PublicInput::JubJubScalar(_) => 1,
        }
    });

    let mut pis = Vec::with_capacity(n_pi);

    public_inputs.into_iter().for_each(|pi| match pi {
        PublicInput::Point(p) => pis.extend([p.get_x(), p.get_y()]),
        PublicInput::BlsScalar(s) => pis.push(s),
        PublicInput::JubJubScalar(s) => pis.push(s.into()),
    });

    verifier.verify(&proof, &pis).is_ok()
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(msg: BlsScalar, pk: PublicKey, sig: Signature) -> bool {
    sig.verify(&pk, msg)
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, apk: APK, sig: BlsSignature) -> bool {
    apk.verify(&sig, &msg).is_ok()
}

#[derive(Default)]
struct DummyCircuit;

impl Circuit for DummyCircuit {
    fn circuit<C>(&self, _: &mut C) -> Result<(), Error>
    where
        C: Composer,
    {
        unreachable!(
            "This circuit should never be compiled or proven, only verified"
        )
    }
}
