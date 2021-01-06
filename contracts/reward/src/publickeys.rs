// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Sink, Source, Store};
use core::marker::PhantomData;
use dusk_bls12_381_sign::APK;

const SIZE: usize = 128;

// TODO: determine size
#[derive(Debug, Clone)]
pub struct PublicKeys<S> {
    pub(crate) pks: [APK; SIZE],
    _marker: PhantomData<S>,
}

impl<S> From<[APK; SIZE]> for PublicKeys<S>
where
    S: Store,
{
    fn from(v: [APK; SIZE]) -> Self {
        PublicKeys {
            pks: v,
            _marker: PhantomData,
        }
    }
}

impl<S> Default for PublicKeys<S>
where
    S: Store,
{
    fn default() -> Self {
        Self {
            pks: [APK::default(); SIZE],
            _marker: PhantomData,
        }
    }
}

impl<S> Canon<S> for PublicKeys<S>
where
    S: Store,
{
    fn read(source: &mut impl Source<S>) -> Result<Self, S::Error> {
        let mut keys = [APK::default(); SIZE];
        for i in 0..keys.len() {
            let pk = Canon::<S>::read(source)?;
            keys[i] = pk
        }

        Ok(PublicKeys {
            pks: keys,
            _marker: PhantomData,
        })
    }

    fn write(&self, sink: &mut impl Sink<S>) -> Result<(), S::Error> {
        self.pks
            .iter()
            .map(|pk| pk.write(sink))
            .collect::<Result<(), S::Error>>()?;
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        APK::serialized_size() * SIZE
    }
}
