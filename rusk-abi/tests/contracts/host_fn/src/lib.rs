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
pub const SCHNORR_SIGNATURE: u8 = 2;

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

    use canonical::{BridgeStore, ByteSink, ByteSource, Canon, Id32, Store};
    use dusk_abi::ReturnValue;

    use dusk_bls12_381::BlsScalar;
    use dusk_pki::PublicKey;
    use schnorr::Signature;

    const PAGE_SIZE: usize = 1024 * 4;

    type BS = BridgeStore<Id32>;

    impl HostFnTest {
        pub fn hash(&self, scalars: Vec<BlsScalar>) -> BlsScalar {
            rusk_abi::poseidon_hash(scalars)
        }

        pub fn schnorr_signature(
            &self,
            sig: Signature,
            pk: PublicKey,
            message: BlsScalar,
        ) -> bool {
            rusk_abi::verify_schnorr_sign(sig, pk, message)
        }
    }

    fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), <BS as Store>::Error> {
        let bs = BS::default();
        let mut source = ByteSource::new(&bytes[..], &bs);

        // read self.
        let slf: HostFnTest = Canon::<BS>::read(&mut source)?;

        // read query id
        let qid: u8 = Canon::<BS>::read(&mut source)?;
        match qid {
            // read_value (&Self) -> i32
            HASH => {
                let arg: Vec<BlsScalar> = Canon::<BS>::read(&mut source)?;

                let ret = slf.hash(arg);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret, &bs)?;

                    let mut sink = ByteSink::new(&mut bytes[..], &bs);

                    Canon::<BS>::write(&wrapped_return, &mut sink)
                };

                r
            }
            SCHNORR_SIGNATURE => {
                let sig: Signature = Canon::<BS>::read(&mut source)?;
                let pk: PublicKey = Canon::<BS>::read(&mut source)?;
                let message: BlsScalar = Canon::<BS>::read(&mut source)?;

                let ret = slf.schnorr_signature(sig, pk, message);

                let r = {
                    // return value
                    let wrapped_return = ReturnValue::from_canon(&ret, &bs)?;

                    let mut sink = ByteSink::new(&mut bytes[..], &bs);

                    Canon::<BS>::write(&wrapped_return, &mut sink)
                };

                r
            }

            _ => panic!(""),
        }
    }

    #[no_mangle]
    fn q(bytes: &mut [u8; PAGE_SIZE]) {
        // todo, handle errors here
        let _ = query(bytes);
    }

    fn transaction(
        bytes: &mut [u8; PAGE_SIZE],
    ) -> Result<(), <BS as Store>::Error> {
        let bs = BS::default();
        let mut source = ByteSource::new(bytes, &bs);

        // read self.
        let mut _slf: HostFnTest = Canon::<BS>::read(&mut source)?;
        // read transaction id
        let tid: u8 = Canon::<BS>::read(&mut source)?;
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
