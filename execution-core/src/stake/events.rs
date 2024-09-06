// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by Dusk's stake contract.

use alloc::{
    fmt,
    string::{String, ToString},
};
use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Serializable};
use rkyv::{Archive, Deserialize, Serialize};
use serde::{
    de::{self, Visitor},
    Deserialize as SerdeDeserialize, Deserializer, Serialize as SerdeSerialize,
    Serializer,
};

use crate::{
    signatures::bls::PublicKey as BlsPublicKey,
    transfer::withdraw::WithdrawReceiver,
};

/// Event emitted after a stake contract operation is performed.
#[derive(
    Debug,
    Clone,
    Archive,
    Deserialize,
    Serialize,
    SerdeDeserialize,
    SerdeSerialize,
)]
#[archive_attr(derive(CheckBytes))]
pub struct StakeEvent {
    #[serde(
        serialize_with = "serialize_bls_public_key",
        deserialize_with = "deserialize_bls_public_key"
    )]
    /// Account associated to the event.
    pub account: BlsPublicKey,
    /// Value of the relevant operation, be it `stake`, `reward` or `slash`.
    ///
    /// In case of `suspended` the amount refers to the next eligibility
    pub value: u64,
}

impl StakeEvent {
    /// Return the JSON representation of the event
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

// Custom serializer for the `BlsPublicKey` field
fn serialize_bls_public_key<S>(
    key: &BlsPublicKey,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let bytes = key.to_bytes();
    let encoded = bs58::encode(bytes).into_string();
    serializer.serialize_str(&encoded)
}

// Custom deserializer for the `BlsPublicKey` field
fn deserialize_bls_public_key<'de, D>(
    deserializer: D,
) -> Result<BlsPublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    struct BlsPublicKeyVisitor;

    impl<'de> Visitor<'de> for BlsPublicKeyVisitor {
        type Value = BlsPublicKey;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str(
                "a Base58 encoded string representing a BlsPublicKey",
            )
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let bytes = bs58::decode(value).into_vec().map_err(|_| {
                de::Error::custom("Not a valid bs58".to_string())
            })?;
            let ret = BlsPublicKey::from_slice(&bytes).map_err(|_| {
                de::Error::custom("Not a valid BlsPublicKey".to_string())
            })?;
            Ok(ret)
        }
    }

    deserializer.deserialize_str(BlsPublicKeyVisitor)
}

/// Event emitted after a stake contract operation is performed.
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct StakeWithReceiverEvent {
    /// Account associated to the event.
    pub account: BlsPublicKey,
    /// Value of the relevant operation, be it `unstake` or `withdraw`.
    pub value: u64,
    /// The receiver of the action
    pub receiver: Option<WithdrawReceiver>,
}
