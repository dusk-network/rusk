// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::format;
use alloc::string::String;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::de::{Error as SerdeError, MapAccess, Unexpected, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::signatures::bls::PublicKey as AccountPublicKey;
use crate::stake::{Reward, SlashEvent, StakeEvent};
use crate::transfer::{
    ContractToAccountEvent, ContractToContractEvent, ConvertEvent,
    DepositEvent, MoonlightTransactionEvent, PhoenixTransactionEvent,
    WithdrawEvent,
};

// To serialize and deserialize u64s as big ints:
#[derive(Debug)]
struct Bigint(u64);

impl Serialize for Bigint {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let s: String = format!("{}", self.0);
        s.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bigint {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() {
            return Err(SerdeError::invalid_value(
                Unexpected::Str(&s),
                &"a non-empty string",
            ));
        }
        let parsed_number = s.parse::<u64>().map_err(|e| {
            SerdeError::custom(format!("failed to deserialize u64: {e}"))
        })?;
        Ok(Self(parsed_number))
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

        const FIELDS: [&str; 3] = ["keys", "value", "locked"];

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

        const FIELDS: [&str; 3] = ["account", "value", "next_eligibility"];

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

        const FIELDS: [&str; 3] = ["account", "value", "reason"];

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

        const FIELDS: [&str; 3] = ["sender", "receiver", "value"];

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

        const FIELDS: [&str; 3] = ["sender", "receiver", "value"];

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

        const FIELDS: [&str; 3] = ["sender", "receiver", "value"];

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

        const FIELDS: [&str; 3] = ["sender", "receiver", "value"];

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

        const FIELDS: [&str; 3] = ["sender", "receiver", "value"];

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

impl Serialize for PhoenixTransactionEvent {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut ser_struct =
            serializer.serialize_struct("PhoenixTransactionEvent", 5)?;
        ser_struct.serialize_field("nullifiers", &self.nullifiers)?;
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

        const FIELDS: [&str; 5] =
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
                let mut nullifiers = None;
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
                let memo =
                    memo.ok_or_else(|| SerdeError::missing_field("memo"))?;
                let memo =
                    BASE64_STANDARD.decode(memo).map_err(SerdeError::custom)?;

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
        deserializer.deserialize_struct(
            "MoonlightTransactionEvent",
            &moonlight_transaction_event_helpers::FIELDS,
            moonlight_transaction_event_helpers::MoonlightTransactionEventVisitor,
        )
    }
}

// `MoonlightTransactionEvent`'s visitor for deserialization is in this module
// to satisy clippy.
mod moonlight_transaction_event_helpers {
    use super::{
        AccountPublicKey, Bigint, Engine, MapAccess, MoonlightTransactionEvent,
        SerdeError, String, Visitor, BASE64_STANDARD,
    };

    pub struct MoonlightTransactionEventVisitor;

    pub const FIELDS: [&str; 6] = [
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
            let mut refund_info: Option<Option<(AccountPublicKey, Bigint)>> =
                None;

            while let Some(key) = map.next_key()? {
                match key {
                    "sender" => {
                        if sender.is_some() {
                            return Err(SerdeError::duplicate_field("sender"));
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
                            return Err(SerdeError::duplicate_field("value"));
                        }
                        value = Some(map.next_value()?);
                    }
                    "memo" => {
                        if memo.is_some() {
                            return Err(SerdeError::duplicate_field("memo"));
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
                        return Err(SerdeError::unknown_field(field, &FIELDS))
                    }
                }
            }

            let memo = memo.ok_or_else(|| SerdeError::missing_field("memo"))?;
            let memo =
                BASE64_STANDARD.decode(memo).map_err(SerdeError::custom)?;
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
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    use super::*;

    #[test]
    fn serde_bigint() {
        let mut rng = StdRng::seed_from_u64(42);
        let n = Bigint(rng.next_u64());
        let ser = serde_json::to_string(&n).unwrap();
        let deser: Bigint = serde_json::from_str(&ser).unwrap();
        assert_eq!(n.0, deser.0);
    }

    #[test]
    fn serde_bigint_max() {
        let n = Bigint(u64::MAX);
        let ser = serde_json::to_string(&n).unwrap();
        let deser: Bigint = serde_json::from_str(&ser).unwrap();
        assert_eq!(n.0, deser.0);
    }

    #[test]
    fn serde_bigint_empty() {
        let deser: Result<Bigint, _> = serde_json::from_str("\"\"");
        assert!(deser.is_err());
    }
}
