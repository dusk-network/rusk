// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::vec::Vec;
use blake2b_simd::Params;
use canonical::{Canon, CanonError, Source};
use dusk_abi::{ContractId, HostModule, Query, ReturnValue};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicKey;
use dusk_plonk::circuit::{self, VerifierData};
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;

use crate::{PublicInput, RuskModule};

/// Generate a [`ContractId`] address from the given slice of bytes, that is
/// also a valid [`BlsScalar`]
pub fn gen_contract_id(bytes: &[u8]) -> ContractId {
    let mut state = Params::new().hash_length(64).to_state();
    state.update(bytes);

    let mut buf = [0u8; 64];
    buf.copy_from_slice(state.finalize().as_ref());

    ContractId::from_raw(BlsScalar::from_bytes_wide(&buf).to_bytes())
}

/// Hashes a vector of [`BlsScalar`] using Poseidon's sponge function
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    dusk_poseidon::sponge::hash(&scalars)
}

impl RuskModule {
    pub fn new(pp: &'static PublicParameters) -> Self {
        RuskModule { pp }
    }
}

impl HostModule for RuskModule {
    fn execute(&self, query: Query) -> Result<ReturnValue, CanonError> {
        let mut source = Source::new(query.as_bytes());

        let qid = u8::decode(&mut source)?;

        match qid {
            Self::POSEIDON_HASH => {
                let scalars = Vec::<BlsScalar>::decode(&mut source)?;
                let ret = dusk_poseidon::sponge::hash(&scalars);

                Ok(ReturnValue::from_canon(&ret))
            }

            Self::VERIFY_PROOF => {
                let proof = Vec::<u8>::decode(&mut source)?;
                let verifier_data = Vec::<u8>::decode(&mut source)?;
                let pi = Vec::<PublicInput>::decode(&mut source)?;

                let proof = Proof::from_slice(&proof)
                    .map_err(|_| CanonError::InvalidEncoding)?;

                let verifier_data =
                    VerifierData::from_slice(verifier_data.as_slice())
                        .map_err(|_| CanonError::InvalidEncoding)?;

                let pi: Vec<PublicInputValue> =
                    pi.into_iter().map(|pi| pi.into()).collect();

                let ret = circuit::verify_proof(
                    self.pp,
                    verifier_data.key(),
                    &proof,
                    pi.as_slice(),
                    verifier_data.pi_pos().as_slice(),
                    b"dusk-network",
                )
                .is_ok();

                Ok(ReturnValue::from_canon(&ret))
            }

            Self::VERIFY_SCHNORR_SIGN => {
                let sign = Signature::decode(&mut source)?;
                let pk = PublicKey::decode(&mut source)?;
                let message = BlsScalar::decode(&mut source)?;
                let ret = sign.verify(&pk, message);

                Ok(ReturnValue::from_canon(&ret))
            }
            _ => todo!(),
        }
    }
}
