// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::vec::Vec;
use canonical::{ByteSource, Canon, InvalidEncoding, Store};
use dusk_abi::{HostModule, Query, ReturnValue};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;
use dusk_pki::PublicKey;
use dusk_plonk::circuit::{self, VerifierData};
use dusk_plonk::prelude::*;
use schnorr::Signature;

use crate::{PublicInput, RuskModule};

impl Into<PublicInputValue> for PublicInput {
    fn into(self) -> PublicInputValue {
        match self {
            PublicInput::BlsScalar(v) => PublicInputValue::from(v),
            PublicInput::JubJubScalar(v) => PublicInputValue::from(v),
            PublicInput::Point(v) => PublicInputValue::from(v),
        }
    }
}

impl<S> RuskModule<S>
where
    S: Store,
{
    pub fn new(store: S, pp: &'static PublicParameters) -> Self {
        RuskModule { store, pp }
    }
}

impl<S> HostModule<S> for RuskModule<S>
where
    S: Store,
{
    fn execute(&self, query: Query) -> Result<ReturnValue, S::Error> {
        let mut source = ByteSource::new(query.as_bytes(), &self.store);

        let qid: u8 = Canon::<S>::read(&mut source)?;

        match qid {
            Self::POSEIDON_HASH => {
                let scalars: Vec<BlsScalar> = Canon::<S>::read(&mut source)?;
                let ret = dusk_poseidon::sponge::hash(&scalars);

                ReturnValue::from_canon(&ret, &self.store)
            }

            Self::VERIFY_PROOF => {
                let proof: Vec<u8> = Canon::<S>::read(&mut source)?;
                let verifier_data: Vec<u8> = Canon::<S>::read(&mut source)?;
                let pi: Vec<PublicInput> = Canon::<S>::read(&mut source)?;

                let proof = Proof::from_slice(&proof)
                    .map_err(|_| InvalidEncoding.into())?;

                let verifier_data =
                    VerifierData::from_slice(verifier_data.as_slice())
                        .map_err(|_| InvalidEncoding.into())?;

                let pi: Vec<PublicInputValue> =
                    pi.into_iter().map(|pi| pi.into()).collect();

                let ret = circuit::verify_proof(
                    &self.pp,
                    verifier_data.key(),
                    &proof,
                    pi.as_slice(),
                    verifier_data.pi_pos().as_slice(),
                    b"dusk-network",
                )
                .is_ok();

                ReturnValue::from_canon(&ret, &self.store)
            }

            Self::VERIFY_SCHNORR_SIGN => {
                let sign: Signature = Canon::<S>::read(&mut source)?;
                let pk: PublicKey = Canon::<S>::read(&mut source)?;
                let message: BlsScalar = Canon::<S>::read(&mut source)?;
                let ret = sign.verify(&pk, message);

                ReturnValue::from_canon(&ret, &self.store)
            }
            _ => todo!(),
        }
    }
}
