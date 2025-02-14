// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::de::{Error as SerdeError, Unexpected};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::abi::ContractId;
use crate::signatures::bls::PublicKey as AccountPublicKey;
use crate::stake::{Reward, RewardReason, SlashEvent, StakeEvent, StakeKeys};
use crate::transfer::phoenix::Note;
use crate::transfer::withdraw::WithdrawReceiver;
use crate::transfer::{
    ContractToAccountEvent, ContractToContractEvent, ConvertEvent,
    DepositEvent, MoonlightTransactionEvent, PhoenixTransactionEvent,
    WithdrawEvent,
};
use crate::BlsScalar;

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
        #[derive(Deserialize)]
        struct Intermediate {
            keys: StakeKeys,
            value: Bigint,
            locked: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(StakeEvent {
            keys: intermediate_event.keys,
            value: intermediate_event.value.0,
            locked: intermediate_event.locked.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            pub account: AccountPublicKey,
            pub value: Bigint,
            pub next_eligibility: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(SlashEvent {
            account: intermediate_event.account,
            value: intermediate_event.value.0,
            next_eligibility: intermediate_event.next_eligibility.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            account: AccountPublicKey,
            value: Bigint,
            reason: RewardReason,
        }
        let intermediate_reward = Intermediate::deserialize(deserializer)?;
        Ok(Reward {
            account: intermediate_reward.account,
            value: intermediate_reward.value.0,
            reason: intermediate_reward.reason,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            pub sender: ContractId,
            pub receiver: WithdrawReceiver,
            pub value: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(WithdrawEvent {
            sender: intermediate_event.sender,
            receiver: intermediate_event.receiver,
            value: intermediate_event.value.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            sender: Option<AccountPublicKey>,
            receiver: WithdrawReceiver,
            value: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(ConvertEvent {
            sender: intermediate_event.sender,
            receiver: intermediate_event.receiver,
            value: intermediate_event.value.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            sender: Option<AccountPublicKey>,
            receiver: ContractId,
            value: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(DepositEvent {
            sender: intermediate_event.sender,
            receiver: intermediate_event.receiver,
            value: intermediate_event.value.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            sender: ContractId,
            receiver: ContractId,
            value: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(ContractToContractEvent {
            sender: intermediate_event.sender,
            receiver: intermediate_event.receiver,
            value: intermediate_event.value.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            sender: ContractId,
            receiver: AccountPublicKey,
            value: Bigint,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        Ok(ContractToAccountEvent {
            sender: intermediate_event.sender,
            receiver: intermediate_event.receiver,
            value: intermediate_event.value.0,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            pub nullifiers: Vec<BlsScalar>,
            pub notes: Vec<Note>,
            pub memo: String,
            pub gas_spent: Bigint,
            pub refund_note: Option<Note>,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        let memo = BASE64_STANDARD
            .decode(intermediate_event.memo)
            .map_err(SerdeError::custom)?;
        Ok(PhoenixTransactionEvent {
            nullifiers: intermediate_event.nullifiers,
            notes: intermediate_event.notes,
            memo,
            gas_spent: intermediate_event.gas_spent.0,
            refund_note: intermediate_event.refund_note,
        })
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
        #[derive(Deserialize)]
        struct Intermediate {
            sender: AccountPublicKey,
            receiver: Option<AccountPublicKey>,
            value: Bigint,
            memo: String,
            gas_spent: Bigint,
            refund_info: Option<(AccountPublicKey, Bigint)>,
        }
        let intermediate_event = Intermediate::deserialize(deserializer)?;
        let memo = BASE64_STANDARD
            .decode(intermediate_event.memo)
            .map_err(SerdeError::custom)?;
        let refund_info = intermediate_event
            .refund_info
            .map(|(pk, bigint)| (pk, bigint.0));
        Ok(MoonlightTransactionEvent {
            sender: intermediate_event.sender,
            receiver: intermediate_event.receiver,
            value: intermediate_event.value.0,
            memo,
            gas_spent: intermediate_event.gas_spent.0,
            refund_info,
        })
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
