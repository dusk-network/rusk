// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

tonic::include_proto!("rusk");

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::Ownable;
use tonic::Status;

pub const TX_VERSION: u32 = 1;
pub const TX_TYPE_TRANSFER: u32 = 1;

impl From<dusk_pki::PublicSpendKey> for PublicKey {
    fn from(value: dusk_pki::PublicSpendKey) -> Self {
        PublicKey {
            payload: value.to_bytes().to_vec(),
        }
    }
}

impl From<dusk_pki::SecretSpendKey> for SecretKey {
    fn from(value: dusk_pki::SecretSpendKey) -> Self {
        SecretKey {
            payload: value.to_bytes().to_vec(),
        }
    }
}

impl From<dusk_pki::ViewKey> for ViewKey {
    fn from(value: dusk_pki::ViewKey) -> Self {
        ViewKey {
            payload: value.to_bytes().to_vec(),
        }
    }
}

impl From<dusk_pki::StealthAddress> for StealthAddress {
    fn from(value: dusk_pki::StealthAddress) -> Self {
        StealthAddress {
            payload: value.to_bytes().to_vec(),
        }
    }
}

impl From<&phoenix_core::Fee> for Fee {
    fn from(fee: &phoenix_core::Fee) -> Self {
        Self {
            gas_limit: fee.gas_limit,
            gas_price: fee.gas_price,
            stealth_address: Some(fee.stealth_address().into()),
        }

}

impl From<&dusk_pki::StealthAddress> for StealthAddress {
    fn from(value: &dusk_pki::StealthAddress) -> Self {
        (*value).into()
    }
}

impl TryFrom<&PublicKey> for dusk_pki::PublicSpendKey {
    type Error = Status;

    fn try_from(value: &PublicKey) -> Result<dusk_pki::PublicSpendKey, Status> {
        Self::from_slice(&value.payload)
            .map_err(|_| Status::invalid_argument("Invalid PublicKey"))
    }
}

impl TryFrom<&ViewKey> for dusk_pki::ViewKey {
    type Error = Status;

    fn try_from(value: &ViewKey) -> Result<Self, Status> {
        Self::from_slice(&value.payload)
            .map_err(|_| Status::invalid_argument("Invalid ViewKey"))
    }
}

impl TryFrom<&StealthAddress> for dusk_pki::StealthAddress {
    type Error = Status;

    fn try_from(value: &StealthAddress) -> Result<Self, Status> {
        Self::from_slice(&value.payload)
            .map_err(|_| Status::invalid_argument("Invalid ViewKey"))
    }
}
