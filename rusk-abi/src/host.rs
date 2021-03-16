// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

use alloc::vec::Vec;
use canonical::{ByteSource, Canon, Store};
use dusk_abi::{HostModule, Query, ReturnValue};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;
use dusk_pki::PublicKey;
use dusk_plonk::circuit;
use dusk_plonk::prelude::*;
use schnorr::Signature;

use crate::PublicInput;
use crate::RuskModule;

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
                let vk: Vec<u8> = Canon::<S>::read(&mut source)?;
                let pi_values: Vec<PublicInput> =
                    Canon::<S>::read(&mut source)?;
                let pi_positions: Vec<u32> = Canon::<S>::read(&mut source)?;

                let pi_positions: Vec<_> =
                    pi_positions.iter().map(|i| *i as usize).collect();

                let vk = VerifierKey::from_slice(&vk[..]).expect("a Key");

                let proof = Proof::from_slice(&proof).expect("a Proof");

                let pi_values: Vec<PublicInputValue> = pi_values
                    .iter()
                    .map(|pi| match *pi {
                        PublicInput::BlsScalar(v) => PublicInputValue::from(v),
                        PublicInput::JubJubScalar(v) => {
                            PublicInputValue::from(v)
                        }
                        PublicInput::Point(v) => PublicInputValue::from(v),
                    })
                    .collect();

                let ret = circuit::verify_proof(
                    &self.pp,
                    &vk,
                    &proof,
                    &pi_values[..],
                    &pi_positions[..],
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
