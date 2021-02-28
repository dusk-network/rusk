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
use dusk_pki::PublicKey;
use schnorr::Signature;

use crate::RuskModule;

impl<S> RuskModule<S>
where
    S: Store,
{
    pub fn new(store: S) -> Self {
        RuskModule { store }
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
