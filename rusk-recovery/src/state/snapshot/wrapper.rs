// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::ops::Deref;

use dusk_bytes::{BadLength, DeserializableSlice, InvalidChar, Serializable};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq)]
pub(crate) struct Wrapper<C, const N: usize>(C);

impl<C, const N: usize> Deref for Wrapper<C, N> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<C, const N: usize> From<C> for Wrapper<C, N> {
    fn from(inner: C) -> Self {
        Self(inner)
    }
}

impl<T, const N: usize> Serialize for Wrapper<T, N>
where
    T: Serializable<N>,
    T::Error: BadLength + InvalidChar,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&bs58::encode(&self.to_bytes()).into_string())
    }
}

impl<'de, T, const N: usize> Deserialize<'de> for Wrapper<T, N>
where
    T: Serializable<N>,
    T::Error: BadLength + InvalidChar,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let data = bs58::decode(s)
            .into_vec()
            .map_err(|_| serde::de::Error::custom("invalid base58"))?;
        let data = T::from_slice(&data[..])
            .map_err(|_| serde::de::Error::custom("invalid address"))?;
        Ok(Self(data))
    }
}
