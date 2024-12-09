// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::signatures::bls::PublicKey as AccountPublicKey;
use crate::String;
use alloc::format;
use dusk_bytes::Serializable;
use serde::{de, Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use serde_json as _;

struct SerializablePublicKey(AccountPublicKey);

impl Serialize for SerializablePublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = bs58::encode(self.0.to_bytes()).into_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for SerializablePublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut bytes: [u8; AccountPublicKey::SIZE] =
            [0; AccountPublicKey::SIZE];
        match bs58::decode(&s).into(&mut bytes) {
            Ok(n) => {
                if n != AccountPublicKey::SIZE {
                    return Err(de::Error::custom(
                        "failed to deserialize AccountPublicKey",
                    ));
                }
            }
            Err(err) => return Err(de::Error::custom(format!("{err:?}"))),
        }
        let pubk = AccountPublicKey::from_bytes(&bytes)
            .map_err(|err| de::Error::custom(format!("{err:?}")))?;
        Ok(SerializablePublicKey(pubk))
    }
}

pub mod pubk {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &AccountPublicKey,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let s = SerializablePublicKey(value.clone());
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<AccountPublicKey, D::Error> {
        SerializablePublicKey::deserialize(deserializer)
            .map(|ser_pubk| ser_pubk.0)
    }
}

pub mod optional_pubk {
    use super::*;

    pub fn serialize<S: Serializer>(
        value: &Option<AccountPublicKey>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let s = value
            .as_ref()
            .map(|pubk| SerializablePublicKey(pubk.clone()));
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<AccountPublicKey>, D::Error> {
        let s = Option::<SerializablePublicKey>::deserialize(deserializer)?;
        Ok(s.map(|ser_pubk| ser_pubk.0))
    }
}

pub mod pubk_u64_tuple {
    use super::*;
    use de::Visitor;

    pub fn serialize<S: Serializer>(
        value: &Option<(AccountPublicKey, u64)>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some((pubk, n)) => serializer
                .serialize_some(&(SerializablePublicKey(pubk.clone()), n)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<(AccountPublicKey, u64)>, D::Error> {
        struct OptionalTupleVisitor;

        impl<'de> Visitor<'de> for OptionalTupleVisitor {
            type Value = Option<(AccountPublicKey, u64)>;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str("an Option<(PublicKey, u64)>")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(None)
            }

            fn visit_some<D>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_seq(TupleVisitor).map(|t| Some(t))
            }
        }

        struct TupleVisitor;

        impl<'de> Visitor<'de> for TupleVisitor {
            type Value = (AccountPublicKey, u64);

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str("a (PublicKey, u64)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let ser_pubk: SerializablePublicKey = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let n = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok((ser_pubk.0, n))
            }
        }
        deserializer.deserialize_option(OptionalTupleVisitor)
    }
}

pub mod hex_serde {
    use super::*;
    use alloc::vec::Vec;

    pub fn serialize<S: Serializer>(
        value: &[u8],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        hex::encode(value).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(|err| de::Error::custom(format!("{err:?}")))
    }
}
