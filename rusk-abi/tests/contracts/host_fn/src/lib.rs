// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(target_arch = "wasm32", no_std)]
#![feature(core_intrinsics, lang_items, alloc_error_handler)]
#![deny(clippy::all)]

extern crate alloc;

use canonical_derive::Canon;

// query ids
pub const HASH: u8 = 0;
pub const VERIFY: u8 = 1;
pub const SCHNORR_SIGNATURE: u8 = 2;
pub const BLS_SIGNATURE: u8 = 3;
pub const GET_PAYMENT_INFO: u8 = 4;
pub const SPONGE_HASH: u8 = 5;

// transaction ids
pub const SOMETHING: u8 = 0;

#[derive(Clone, Canon, Debug, Default)]
pub struct HostFnTest {}

impl HostFnTest {
    pub fn new() -> Self {
        HostFnTest {}
    }
}

#[cfg(target_arch = "wasm32")]
mod hosted {
    use super::*;

    use alloc::vec::Vec;

    use canonical::{Canon, CanonError, Sink, Source};
    use dusk_abi::ReturnValue;

    use dusk_bls12_381::BlsScalar;
    use dusk_bls12_381_sign::{
        Signature as BlsSignature, APK as AggregatedBlsPublicKey,
    };
    use dusk_bytes::Serializable;
    use dusk_pki::{PublicKey, PublicSpendKey};
    use dusk_schnorr::Signature;
    use rusk_abi::{hash, PaymentInfo, PublicInput};

    const PAGE_SIZE: usize = 1024 * 4;

    impl HostFnTest {
        pub fn sponge_hash(&self, scalars: Vec<BlsScalar>) -> BlsScalar {
            rusk_abi::poseidon_hash(scalars)
        }

        pub fn hash(&self, scalars: &[BlsScalar]) -> BlsScalar {
            let mut hasher = hash::Hasher::new();
            for scalar in scalars {
                hasher.update(&scalar.to_bytes());
            }
            hasher.update(b"dusk network rocks");
            hasher.finalize()
        }

        pub fn verify(
            &self,
            proof: Vec<u8>,
            verifier_data: Vec<u8>,
            pi_values: Vec<PublicInput>,
        ) -> bool {
            rusk_abi::verify_proof(proof, verifier_data, pi_values)
        }

        pub fn schnorr_signature(
            &self,
            sig: Signature,
            pk: PublicKey,
            message: BlsScalar,
        ) -> bool {
            rusk_abi::verify_schnorr_sign(sig, pk, message)
        }

        pub fn bls_signature(
            &self,
            sig: BlsSignature,
            pk: AggregatedBlsPublicKey,
            message: Vec<u8>,
        ) -> bool {
            rusk_abi::verify_bls_sign(sig, pk, message)
        }

        pub fn get_payment_info(&self) -> rusk_abi::PaymentInfo {
            rusk_abi::payment_info(dusk_abi::callee())
        }
    }

    fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
        let mut source = Source::new(&bytes[..]);

        // decode self.
        let slf = HostFnTest::decode(&mut source)?;

        // decode query id
        let qid = u8::decode(&mut source)?;
        match qid {
            SPONGE_HASH => {
                let arg = Vec::<BlsScalar>::decode(&mut source)?;

                let ret = slf.sponge_hash(arg);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            HASH => {
                let arg = Vec::<BlsScalar>::decode(&mut source)?;

                let ret = slf.hash(&arg);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            VERIFY => {
                let proof = Vec::<u8>::decode(&mut source)?;
                let verifier_data = Vec::<u8>::decode(&mut source)?;
                let pi_values =
                    Vec::<rusk_abi::PublicInput>::decode(&mut source)?;

                let ret = slf.verify(proof, verifier_data, pi_values);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            SCHNORR_SIGNATURE => {
                let sig = Signature::decode(&mut source)?;
                let pk = PublicKey::decode(&mut source)?;
                let message = BlsScalar::decode(&mut source)?;

                let ret = slf.schnorr_signature(sig, pk, message);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            BLS_SIGNATURE => {
                let sig = BlsSignature::decode(&mut source)?;
                let pk = AggregatedBlsPublicKey::decode(&mut source)?;
                let message = Vec::<u8>::decode(&mut source)?;

                let ret = slf.bls_signature(sig, pk, message);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            GET_PAYMENT_INFO => {
                let ret = slf.get_payment_info();

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            rusk_abi::PAYMENT_INFO => {
                let ret = PaymentInfo::Any(Some(PublicSpendKey::new(
                    dusk_jubjub::JubJubExtended::default(),
                    dusk_jubjub::JubJubExtended::default(),
                )));

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret);

                    let mut sink = Sink::new(&mut bytes[..]);

                    wrapped_return.encode(&mut sink)
                };

                Ok(r)
            }

            _ => panic!(""),
        }
    }

    #[no_mangle]
    fn q(bytes: &mut [u8; PAGE_SIZE]) {
        // todo, handle errors here
        let _ = query(bytes);
    }

    fn transaction(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
        let mut source = Source::new(bytes);

        // decode self.
        let mut _slf = HostFnTest::decode(&mut source)?;
        // decode transaction id
        let tid = u8::decode(&mut source)?;
        match tid {
            _ => panic!(""),
        }
    }

    #[no_mangle]
    fn t(bytes: &mut [u8; PAGE_SIZE]) {
        // todo, handle errors here
        transaction(bytes).unwrap()
    }
}
