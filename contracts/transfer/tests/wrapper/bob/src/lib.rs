// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![no_std]

use canonical_derive::Canon;

#[derive(Debug, Default, Clone, Canon)]
pub struct Bob {}

#[cfg(target_arch = "wasm32")]
mod hosted {
    use super::*;

    use canonical::{Canon, CanonError, Sink, Source};
    use dusk_abi::ReturnValue;
    use rusk_abi::PaymentInfo;

    const PAGE_SIZE: usize = 1024 * 4;

    impl Bob {
        pub fn identifier() -> &'static [u8; 3] {
            b"bob"
        }
    }

    fn query(bytes: &mut [u8; PAGE_SIZE]) -> Result<(), CanonError> {
        let mut source = Source::new(&bytes[..]);

        let _contract = Bob::decode(&mut source)?;
        let qid = u8::decode(&mut source)?;

        match qid {
            rusk_abi::PAYMENT_INFO => {
                let ret = PaymentInfo::Any(None);

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
        let mut _slf = Bob::decode(&mut source)?;
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
