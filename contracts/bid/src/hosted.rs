// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Hosted interface for the Bid Contract.
//!
//! Here the interface of the contract that will run inside the hosted
//! envoirnoment (WASM instance) is defined and implemented.

mod bridge;
mod transaction;

//TODO: Move this to `dusk-abi` or other repo that exports the
// signatures and implementation of the host functions and the
// unsafe usage.
/// Host function collection used by this contract.
pub(crate) mod host_functions {
    use dusk_plonk::bls12_381::BlsScalar;
    use dusk_plonk::proof_system::Proof;
    use schnorr::single_key::{PublicKey, Signature};
    // Host function definitions implemented in the Externals of WASMI.
    extern "C" {
        fn _verify_schnorr_sig(pk: &u8, sig: &u8, msg: &u8) -> i32;
        fn _verify_proof(
            pub_inputs_len: usize,
            pub_inputs: &u8,
            proof: &u8,
            verif_key: &u8,
        ) -> i32;
        fn _p_hash(ofs: &u8, len: u32, ret_addr: &mut [u8; 32]);
    }

    /// Verifies a PLONK proof returning `true` if the verification suceeded
    /// or `false` if it didn't.
    pub(crate) fn verify_proof(
        pub_inputs: [u8; 33],
        proof: Proof,
        verifier_key: &'static [u8],
    ) -> bool {
        // TODO: We should avoid that.
        let proof_bytes = proof.to_bytes();
        unsafe {
            match _verify_proof(
                1usize,
                &pub_inputs[0],
                &proof_bytes[0],
                &verifier_key[0],
            ) {
                1i32 => true,
                0i32 => false,
                _ => panic!("Malformed result from Proof verification"),
            }
        }
    }

    /// Verifies a Schnorr Signature returning `true` if the verification
    /// suceeded or `false` if it didn't.
    pub(crate) fn verify_schnorr_sig(
        pk: PublicKey,
        sig: Signature,
        msg: BlsScalar,
    ) -> bool {
        let pk_bytes = pk.to_bytes();
        let sig_bytes = sig.to_bytes();
        let message_bytes = msg.to_bytes();
        unsafe {
            match _verify_schnorr_sig(
                &pk_bytes[0],
                &sig_bytes[0],
                &message_bytes[0],
            ) {
                1i32 => true,
                _ => false,
            }
        }
    }

    /// Executes a poseidon sponge hash in the host envoironment returning
    /// the result as a `BlsScalar`.
    ///
    /// TODO: Use `const_generics` once it's possible to operate with
    /// the const parameter. On that way we can create a `[u8; inputs.len() *
    /// 32]`.
    /// This will become possible once the feature `const_evaluatable_checked`
    /// gets stabilized (at least is not incomplete).
    /// See: https://github.com/rust-lang/rust/issues/76560
    pub(crate) fn p_hash(inputs: &[BlsScalar]) -> BlsScalar {
        // Transform the BlsScalars into bytes.
        // For now we allow the hash of 16 BlsScalars which will
        // occupy 512 bytes. It's difficult to think about an example
        // that will need more. But we can always increase it here.
        let mut inp_as_bytes = [0u8; 512];
        inputs.iter().enumerate().for_each(|(idx, scalar)| {
            inp_as_bytes[idx * 32..(idx * 32) + 32]
                .copy_from_slice(&scalar.to_bytes()[..])
        });

        let mut result_ffi = [0u8; 32];
        unsafe {
            _p_hash(
                &inp_as_bytes[0],
                (inputs.len() * 32) as u32,
                &mut result_ffi,
            );
        }
        Option::from(BlsScalar::from_bytes(&result_ffi))
            .expect("Malformed hash result")
    }
}
