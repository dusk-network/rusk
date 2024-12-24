// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

use base64 as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use dusk_bytes::Serializable;
use hex as _;
use serde::de::{
    EnumAccess, Error as SerdeError, MapAccess, Unexpected, VariantAccess,
    Visitor,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json as _;

use crate::stake::{Reward, RewardReason, SlashEvent, StakeEvent, StakeKeys};
use crate::transfer::withdraw::WithdrawReceiver;
use crate::transfer::{
    ContractToAccountEvent, ContractToContractEvent, ConvertEvent,
    DepositEvent, MoonlightTransactionEvent, PhoenixTransactionEvent,
    WithdrawEvent,
};
use crate::{
    signatures::bls::PublicKey as AccountPublicKey, stake::StakeFundOwner,
};
use crate::{BlsScalar, String};

// To serialize and deserialize u64s as big ints:
// https://github.com/dusk-network/rusk/issues/2773#issuecomment-2519791322.
struct Bigint(u64);

impl Serialize for Bigint {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let s: String = format!("{}n", self.0);
        s.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bigint {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let mut s = String::deserialize(deserializer)?;
        let last_char = s.pop().ok_or_else(|| {
            SerdeError::invalid_value(
                Unexpected::Str(&s),
                &"a non-empty string",
            )
        })?;
        if last_char != 'n' {
            return Err(SerdeError::invalid_value(
                Unexpected::Str(&s),
                &"a bigint ending with character 'n'",
            ));
        }
        let parsed_number = u64::from_str_radix(&s, 10).map_err(|e| {
            SerdeError::custom(format!("failed to deserialize u64: {e}"))
        })?;
        Ok(Self(parsed_number))
    }
}

impl Serialize for StakeFundOwner {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::Account(pk) => serializer.serialize_newtype_variant(
                "StakeFundOwner",
                0,
                "Account",
                pk,
            ),
            Self::Contract(contract_id) => serializer
                .serialize_newtype_variant(
                    "StakeFundOwner",
                    1,
                    "ContractId",
                    contract_id,
                ),
        }
    }
}

impl<'de> Deserialize<'de> for StakeFundOwner {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct StakeFundOwnerVisitor;

        static VARIANTS: [&'static str; 2] = ["Account", "ContractId"];

        impl<'de> Visitor<'de> for StakeFundOwnerVisitor {
            type Value = StakeFundOwner;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter
                    .write_str("an enum with variants Account and ContractId")
            }

            fn visit_enum<A: EnumAccess<'de>>(
                self,
                data: A,
            ) -> Result<Self::Value, A::Error> {
                match data.variant()? {
                    ("Account", variant) => {
                        Ok(StakeFundOwner::Account(variant.newtype_variant()?))
                    }
                    ("ContractId", variant) => {
                        Ok(StakeFundOwner::Contract(variant.newtype_variant()?))
                    }
                    (variant_name, _) => Err(SerdeError::unknown_variant(
                        variant_name,
                        &VARIANTS,
                    )),
                }
            }
        }

        deserializer.deserialize_enum(
            "StakeFundOwner",
            &VARIANTS,
            StakeFundOwnerVisitor,
        )
    }
}

impl Serialize for StakeKeys {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("StakeKeys", 2)?;
        ser_struct.serialize_field("account", &self.account)?;
        ser_struct.serialize_field("owner", &self.owner)?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for StakeKeys {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct StakeKeysVisitor;

        static FIELDS: [&'static str; 2] = ["account", "owner"];

        impl<'de> Visitor<'de> for StakeKeysVisitor {
            type Value = StakeKeys;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields account and owner",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut account = None;
                let mut owner = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "account" => {
                            if account.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "account",
                                ));
                            }
                            account = Some(map.next_value()?);
                        }
                        "owner" => {
                            if owner.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "owner",
                                ));
                            }
                            owner = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(StakeKeys {
                    account: account
                        .ok_or_else(|| SerdeError::missing_field("account"))?,
                    owner: owner
                        .ok_or_else(|| SerdeError::missing_field("owner"))?,
                })
            }
        }

        deserializer.deserialize_struct("StakeKeys", &FIELDS, StakeKeysVisitor)
    }
}

impl Serialize for StakeEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("StakeEvent", 3)?;
        ser_struct.serialize_field("keys", &self.keys)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.serialize_field("locked", &Bigint(self.locked))?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for StakeEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct StakeEventVisitor;

        static FIELDS: [&'static str; 3] = ["keys", "value", "locked"];

        impl<'de> Visitor<'de> for StakeEventVisitor {
            type Value = StakeEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields keys, value and locked",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut keys = None;
                let mut value: Option<Bigint> = None;
                let mut locked: Option<Bigint> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "keys" => {
                            if keys.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "keys",
                                ));
                            }
                            keys = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        "locked" => {
                            if locked.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "locked",
                                ));
                            }
                            locked = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(StakeEvent {
                    keys: keys
                        .ok_or_else(|| SerdeError::missing_field("keys"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                    locked: locked
                        .ok_or_else(|| SerdeError::missing_field("locked"))?
                        .0,
                })
            }
        }

        deserializer.deserialize_struct(
            "StakeEvent",
            &FIELDS,
            StakeEventVisitor,
        )
    }
}

impl Serialize for SlashEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("SlashEvent", 3)?;
        ser_struct.serialize_field("account", &self.account)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.serialize_field(
            "next_eligibility",
            &Bigint(self.next_eligibility),
        )?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for SlashEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct SlashEventVisitor;

        static FIELDS: [&'static str; 3] =
            ["account", "value", "next_eligibility"];

        impl<'de> Visitor<'de> for SlashEventVisitor {
            type Value = SlashEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str("expecting a struct with fields account, value and next_eligibility")
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut account = None;
                let mut value: Option<Bigint> = None;
                let mut next_eligibility: Option<Bigint> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "account" => {
                            if account.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "account",
                                ));
                            }
                            account = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        "next_eligibility" => {
                            if next_eligibility.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "next_eligibility",
                                ));
                            }
                            next_eligibility = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(SlashEvent {
                    account: account
                        .ok_or_else(|| SerdeError::missing_field("account"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                    next_eligibility: next_eligibility
                        .ok_or_else(|| {
                            SerdeError::missing_field("next_eligibility")
                        })?
                        .0,
                })
            }
        }

        deserializer.deserialize_struct(
            "SlashEvent",
            &FIELDS,
            SlashEventVisitor,
        )
    }
}

impl Serialize for RewardReason {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::GeneratorFixed => serializer.serialize_unit_variant(
                "RewardReason",
                0,
                "GeneratorFixed",
            ),
            Self::GeneratorExtra => serializer.serialize_unit_variant(
                "RewardReason",
                1,
                "GeneratorExtra",
            ),
            Self::Voter => {
                serializer.serialize_unit_variant("RewardReason", 2, "Voter")
            }
            Self::Other => {
                serializer.serialize_unit_variant("RewardReason", 3, "Other")
            }
        }
    }
}

impl<'de> Deserialize<'de> for RewardReason {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct RewardReasonVisitor;

        static VARIANTS: [&'static str; 4] =
            ["GeneratorFixed", "GeneratorExtra", "Voter", "Other"];

        impl<'de> Visitor<'de> for RewardReasonVisitor {
            type Value = RewardReason;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "an enum with variants GeneratorFixed, GeneratorExtra, Voter and Other",
                )
            }

            fn visit_enum<A: EnumAccess<'de>>(
                self,
                data: A,
            ) -> Result<Self::Value, A::Error> {
                match data.variant()? {
                    ("GeneratorExtra", variant) => {
                        variant.unit_variant()?;
                        Ok(RewardReason::GeneratorExtra)
                    }
                    ("GeneratorFixed", variant) => {
                        variant.unit_variant()?;
                        Ok(RewardReason::GeneratorFixed)
                    }
                    ("Voter", variant) => {
                        variant.unit_variant()?;
                        Ok(RewardReason::Voter)
                    }
                    ("Other", variant) => {
                        variant.unit_variant()?;
                        Ok(RewardReason::Other)
                    }
                    (variant_name, _) => Err(SerdeError::unknown_variant(
                        variant_name,
                        &VARIANTS,
                    )),
                }
            }
        }

        deserializer.deserialize_enum(
            "RewardReason",
            &VARIANTS,
            RewardReasonVisitor,
        )
    }
}

impl Serialize for Reward {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("Reward", 3)?;
        ser_struct.serialize_field("account", &self.account)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.serialize_field("reason", &self.reason)?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for Reward {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct RewardVisitor;

        static FIELDS: [&'static str; 3] = ["account", "value", "reason"];

        impl<'de> Visitor<'de> for RewardVisitor {
            type Value = Reward;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields account, value and reason",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut account = None;
                let mut value: Option<Bigint> = None;
                let mut reason = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "account" => {
                            if account.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "account",
                                ));
                            }
                            account = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        "reason" => {
                            if reason.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "reason",
                                ));
                            }
                            reason = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(Reward {
                    account: account
                        .ok_or_else(|| SerdeError::missing_field("account"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                    reason: reason
                        .ok_or_else(|| SerdeError::missing_field("reason"))?,
                })
            }
        }

        deserializer.deserialize_struct("Reward", &FIELDS, RewardVisitor)
    }
}

impl Serialize for WithdrawReceiver {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::Phoenix(address) => serializer.serialize_newtype_variant(
                "WithdrawReceiver",
                0,
                "Phoenix",
                address,
            ),
            Self::Moonlight(pk) => serializer.serialize_newtype_variant(
                "WithdrawReceiver",
                1,
                "Moonlight",
                pk,
            ),
        }
    }
}

impl<'de> Deserialize<'de> for WithdrawReceiver {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct WithdrawReceiverVisitor;

        static VARIANTS: [&'static str; 2] = ["Moonlight", "Phoenix"];

        impl<'de> Visitor<'de> for WithdrawReceiverVisitor {
            type Value = WithdrawReceiver;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter
                    .write_str("an enum with variants Moonlight and Phoenix")
            }

            fn visit_enum<A: EnumAccess<'de>>(
                self,
                data: A,
            ) -> Result<Self::Value, A::Error> {
                match data.variant()? {
                    ("Moonlight", variant) => Ok(WithdrawReceiver::Moonlight(
                        variant.newtype_variant()?,
                    )),
                    ("Phoenix", variant) => Ok(WithdrawReceiver::Phoenix(
                        variant.newtype_variant()?,
                    )),
                    (variant_name, _) => Err(SerdeError::unknown_variant(
                        variant_name,
                        &VARIANTS,
                    )),
                }
            }
        }

        deserializer.deserialize_enum(
            "WithdrawReceiver",
            &VARIANTS,
            WithdrawReceiverVisitor,
        )
    }
}

impl Serialize for WithdrawEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("WithdrawEvent", 3)?;
        ser_struct.serialize_field("sender", &self.sender)?;
        ser_struct.serialize_field("receiver", &self.receiver)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for WithdrawEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct WithdrawEventVisitor;

        static FIELDS: [&'static str; 3] = ["sender", "receiver", "value"];

        impl<'de> Visitor<'de> for WithdrawEventVisitor {
            type Value = WithdrawEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields sender, receiver and value",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sender = None;
                let mut value: Option<Bigint> = None;
                let mut receiver = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "sender" => {
                            if sender.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "sender",
                                ));
                            }
                            sender = Some(map.next_value()?);
                        }
                        "receiver" => {
                            if receiver.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "receiver",
                                ));
                            }
                            receiver = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(WithdrawEvent {
                    sender: sender
                        .ok_or_else(|| SerdeError::missing_field("sender"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                    receiver: receiver
                        .ok_or_else(|| SerdeError::missing_field("receiver"))?,
                })
            }
        }

        deserializer.deserialize_struct(
            "WithdrawEvent",
            &FIELDS,
            WithdrawEventVisitor,
        )
    }
}

impl Serialize for ConvertEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("ConvertEvent", 3)?;
        ser_struct.serialize_field("sender", &self.sender)?;
        ser_struct.serialize_field("receiver", &self.receiver)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for ConvertEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct ConvertEventVisitor;

        static FIELDS: [&'static str; 3] = ["sender", "receiver", "value"];

        impl<'de> Visitor<'de> for ConvertEventVisitor {
            type Value = ConvertEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields sender, receiver and value",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sender = None;
                let mut value: Option<Bigint> = None;
                let mut receiver = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "sender" => {
                            if sender.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "sender",
                                ));
                            }
                            sender = Some(map.next_value()?);
                        }
                        "receiver" => {
                            if receiver.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "receiver",
                                ));
                            }
                            receiver = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(ConvertEvent {
                    sender: sender
                        .ok_or_else(|| SerdeError::missing_field("sender"))?,
                    receiver: receiver
                        .ok_or_else(|| SerdeError::missing_field("receiver"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                })
            }
        }

        deserializer.deserialize_struct(
            "ConvertEvent",
            &FIELDS,
            ConvertEventVisitor,
        )
    }
}

impl Serialize for DepositEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct = serializer.serialize_struct("DepositEvent", 3)?;
        ser_struct.serialize_field("sender", &self.sender)?;
        ser_struct.serialize_field("receiver", &self.receiver)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for DepositEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct DepositEventVisitor;

        static FIELDS: [&'static str; 3] = ["sender", "receiver", "value"];

        impl<'de> Visitor<'de> for DepositEventVisitor {
            type Value = DepositEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields sender, receiver and value",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sender = None;
                let mut receiver = None;
                let mut value: Option<Bigint> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "sender" => {
                            if sender.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "sender",
                                ));
                            }
                            sender = Some(map.next_value()?);
                        }
                        "receiver" => {
                            if receiver.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "receiver",
                                ));
                            }
                            receiver = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(DepositEvent {
                    sender: sender
                        .ok_or_else(|| SerdeError::missing_field("sender"))?,
                    receiver: receiver
                        .ok_or_else(|| SerdeError::missing_field("receiver"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                })
            }
        }

        deserializer.deserialize_struct(
            "DepositEvent",
            &FIELDS,
            DepositEventVisitor,
        )
    }
}

impl Serialize for ContractToContractEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct =
            serializer.serialize_struct("ContractToContractEvent", 3)?;
        ser_struct.serialize_field("sender", &self.sender)?;
        ser_struct.serialize_field("receiver", &self.receiver)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for ContractToContractEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct ContractToContractEventVisitor;

        static FIELDS: [&'static str; 3] = ["sender", "receiver", "value"];

        impl<'de> Visitor<'de> for ContractToContractEventVisitor {
            type Value = ContractToContractEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields sender, receiver and value",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sender = None;
                let mut receiver = None;
                let mut value: Option<Bigint> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "sender" => {
                            if sender.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "sender",
                                ));
                            }
                            sender = Some(map.next_value()?);
                        }
                        "receiver" => {
                            if receiver.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "receiver",
                                ));
                            }
                            receiver = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(ContractToContractEvent {
                    sender: sender
                        .ok_or_else(|| SerdeError::missing_field("sender"))?,
                    receiver: receiver
                        .ok_or_else(|| SerdeError::missing_field("receiver"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                })
            }
        }

        deserializer.deserialize_struct(
            "ContractToContractEvent",
            &FIELDS,
            ContractToContractEventVisitor,
        )
    }
}

impl Serialize for ContractToAccountEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct =
            serializer.serialize_struct("ContractToAccountEvent", 3)?;
        ser_struct.serialize_field("sender", &self.sender)?;
        ser_struct.serialize_field("receiver", &self.receiver)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for ContractToAccountEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct ContractToAccountEventVisitor;

        static FIELDS: [&'static str; 3] = ["sender", "receiver", "value"];

        impl<'de> Visitor<'de> for ContractToAccountEventVisitor {
            type Value = ContractToAccountEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str(
                    "expecting a struct with fields sender, receiver and value",
                )
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sender = None;
                let mut receiver = None;
                let mut value: Option<Bigint> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "sender" => {
                            if sender.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "sender",
                                ));
                            }
                            sender = Some(map.next_value()?);
                        }
                        "receiver" => {
                            if receiver.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "receiver",
                                ));
                            }
                            receiver = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                Ok(ContractToAccountEvent {
                    sender: sender
                        .ok_or_else(|| SerdeError::missing_field("sender"))?,
                    receiver: receiver
                        .ok_or_else(|| SerdeError::missing_field("receiver"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                })
            }
        }

        deserializer.deserialize_struct(
            "ContractToAccountEvent",
            &FIELDS,
            ContractToAccountEventVisitor,
        )
    }
}

// The current serde implementation for `BlsScalar` is not what it's expected to
// be, so this is needed at the moment: https://github.com/dusk-network/bls12_381/issues/145.
struct BlsScalarSerde(BlsScalar);

impl Serialize for BlsScalarSerde {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let s = hex::encode(self.0.to_bytes());
        s.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BlsScalarSerde {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let decoded = hex::decode(&s).map_err(SerdeError::custom)?;
        let decoded_len = decoded.len();
        let bytes: [u8; BlsScalar::SIZE] =
            decoded.try_into().map_err(|_| {
                SerdeError::invalid_length(
                    decoded_len,
                    &BlsScalar::SIZE.to_string().as_str(),
                )
            })?;
        let bls_scalar = BlsScalar::from_bytes(&bytes).into_option().ok_or(
            SerdeError::custom(
                "Failed to deserialize BlsScalar: invalid BlsScalar",
            ),
        )?;
        Ok(BlsScalarSerde(bls_scalar))
    }
}

impl Serialize for PhoenixTransactionEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct =
            serializer.serialize_struct("PhoenixTransactionEvent", 5)?;
        let nullifiers: Vec<BlsScalarSerde> = self
            .nullifiers
            .iter()
            .map(|scalar| BlsScalarSerde(scalar.clone()))
            .collect();
        ser_struct.serialize_field("nullifiers", &nullifiers)?;
        ser_struct.serialize_field("notes", &self.notes)?;
        ser_struct
            .serialize_field("memo", &BASE64_STANDARD.encode(&self.memo))?;
        ser_struct.serialize_field("gas_spent", &Bigint(self.gas_spent))?;
        ser_struct.serialize_field("refund_note", &self.refund_note)?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for PhoenixTransactionEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct PhoenixTransactionEventVisitor;

        static FIELDS: [&'static str; 5] =
            ["nullifiers", "notes", "memo", "gas_spent", "refund_note"];

        impl<'de> Visitor<'de> for PhoenixTransactionEventVisitor {
            type Value = PhoenixTransactionEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str("expecting a struct with fields nullifiers, notes, memo, gas_spent and refund_note")
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut nullifiers: Option<Vec<BlsScalarSerde>> = None;
                let mut notes = None;
                let mut memo: Option<String> = None;
                let mut gas_spent: Option<Bigint> = None;
                let mut refund_note = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "nullifiers" => {
                            if nullifiers.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "nullifiers",
                                ));
                            }
                            nullifiers = Some(map.next_value()?);
                        }
                        "notes" => {
                            if notes.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "notes",
                                ));
                            }
                            notes = Some(map.next_value()?);
                        }
                        "memo" => {
                            if memo.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "memo",
                                ));
                            }
                            memo = Some(map.next_value()?);
                        }
                        "gas_spent" => {
                            if gas_spent.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "gas_spent",
                                ));
                            }
                            gas_spent = Some(map.next_value()?);
                        }
                        "refund_note" => {
                            if refund_note.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "refund_note",
                                ));
                            }
                            refund_note = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                let nullifiers = nullifiers
                    .ok_or_else(|| SerdeError::missing_field("nullifiers"))?;
                let nullifiers: Vec<BlsScalar> = nullifiers
                    .into_iter()
                    .map(|scalar_serde| scalar_serde.0)
                    .collect();
                let memo =
                    memo.ok_or_else(|| SerdeError::missing_field("memo"))?;
                let memo = BASE64_STANDARD
                    .decode(memo)
                    .map_err(|err| SerdeError::custom(err))?;

                Ok(PhoenixTransactionEvent {
                    nullifiers,
                    notes: notes
                        .ok_or_else(|| SerdeError::missing_field("memo"))?,
                    memo,
                    gas_spent: gas_spent
                        .ok_or_else(|| SerdeError::missing_field("gas_spent"))?
                        .0,
                    refund_note: refund_note.ok_or_else(|| {
                        SerdeError::missing_field("refund_note")
                    })?,
                })
            }
        }

        deserializer.deserialize_struct(
            "PhoenixTransactionEvent",
            &FIELDS,
            PhoenixTransactionEventVisitor,
        )
    }
}

impl Serialize for MoonlightTransactionEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct =
            serializer.serialize_struct("MoonlightTransactionEvent", 6)?;
        let refund_info =
            self.refund_info.map(|(pk, number)| (pk, Bigint(number)));
        ser_struct.serialize_field("sender", &self.sender)?;
        ser_struct.serialize_field("receiver", &self.receiver)?;
        ser_struct.serialize_field("value", &Bigint(self.value))?;
        ser_struct
            .serialize_field("memo", &BASE64_STANDARD.encode(&self.memo))?;
        ser_struct.serialize_field("gas_spent", &Bigint(self.gas_spent))?;
        ser_struct.serialize_field("refund_info", &refund_info)?;
        ser_struct.end()
    }
}

impl<'de> Deserialize<'de> for MoonlightTransactionEvent {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct MoonlightTransactionEventVisitor;

        static FIELDS: [&'static str; 6] = [
            "sender",
            "receiver",
            "value",
            "memo",
            "gas_spent",
            "refund_info",
        ];

        impl<'de> Visitor<'de> for MoonlightTransactionEventVisitor {
            type Value = MoonlightTransactionEvent;

            fn expecting(
                &self,
                formatter: &mut alloc::fmt::Formatter,
            ) -> alloc::fmt::Result {
                formatter.write_str("expecting a struct with fields sender, receiver, value, memo, gas_spent and refund_info")
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut sender = None;
                let mut receiver = None;
                let mut value: Option<Bigint> = None;
                let mut memo: Option<String> = None;
                let mut gas_spent: Option<Bigint> = None;
                let mut refund_info: Option<
                    Option<(AccountPublicKey, Bigint)>,
                > = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "sender" => {
                            if sender.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "sender",
                                ));
                            }
                            sender = Some(map.next_value()?);
                        }
                        "receiver" => {
                            if receiver.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "receiver",
                                ));
                            }
                            receiver = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "value",
                                ));
                            }
                            value = Some(map.next_value()?);
                        }
                        "memo" => {
                            if memo.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "memo",
                                ));
                            }
                            memo = Some(map.next_value()?);
                        }
                        "gas_spent" => {
                            if gas_spent.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "gas_spent",
                                ));
                            }
                            gas_spent = Some(map.next_value()?);
                        }
                        "refund_info" => {
                            if refund_info.is_some() {
                                return Err(SerdeError::duplicate_field(
                                    "refund_info",
                                ));
                            }
                            refund_info = Some(map.next_value()?);
                        }
                        field => {
                            return Err(SerdeError::unknown_field(
                                field, &FIELDS,
                            ))
                        }
                    }
                }

                let memo =
                    memo.ok_or_else(|| SerdeError::missing_field("memo"))?;
                let memo = BASE64_STANDARD
                    .decode(memo)
                    .map_err(|err| SerdeError::custom(err))?;
                let refund_info = refund_info
                    .ok_or_else(|| SerdeError::missing_field("refund_info"))?
                    .map(|(pk, bigint)| (pk, bigint.0));

                Ok(MoonlightTransactionEvent {
                    sender: sender
                        .ok_or_else(|| SerdeError::missing_field("sender"))?,
                    receiver: receiver
                        .ok_or_else(|| SerdeError::missing_field("receiver"))?,
                    value: value
                        .ok_or_else(|| SerdeError::missing_field("value"))?
                        .0,
                    memo,
                    gas_spent: gas_spent
                        .ok_or_else(|| SerdeError::missing_field("gas_spent"))?
                        .0,
                    refund_info,
                })
            }
        }

        deserializer.deserialize_struct(
            "MoonlightTransactionEvent",
            &FIELDS,
            MoonlightTransactionEventVisitor,
        )
    }
}
