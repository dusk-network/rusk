// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]
use super::services::rusk_proto;
use crate::transaction::{Transaction, TransactionPayload};
use core::convert::TryFrom;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_pki::{PublicSpendKey, SecretSpendKey, StealthAddress, ViewKey};
use std::convert::TryInto;
use tonic::{Code, Status};

pub fn as_status_err<T, U: std::fmt::Debug>(
    res: Result<T, U>,
) -> Result<T, Status> {
    res.map_err(|e| Status::new(Code::Unknown, format!("{:?}", e)))
}

impl From<PublicSpendKey> for rusk_proto::PublicKey {
    fn from(value: PublicSpendKey) -> Self {
        rusk_proto::PublicKey {
            a_g: JubJubAffine::from(value.A()).to_bytes().to_vec(),
            b_g: JubJubAffine::from(value.B()).to_bytes().to_vec(),
        }
    }
}

impl From<SecretSpendKey> for rusk_proto::SecretKey {
    fn from(value: SecretSpendKey) -> Self {
        rusk_proto::SecretKey {
            a: value.a().to_bytes().to_vec(),
            b: value.b().to_bytes().to_vec(),
        }
    }
}

impl From<ViewKey> for rusk_proto::ViewKey {
    fn from(value: ViewKey) -> Self {
        rusk_proto::ViewKey {
            a: value.a().to_bytes().to_vec(),
            b_g: JubJubAffine::from(value.B()).to_bytes().to_vec(),
        }
    }
}

impl From<StealthAddress> for rusk_proto::StealthAddress {
    fn from(value: StealthAddress) -> Self {
        rusk_proto::StealthAddress {
            r_g: JubJubAffine::from(value.R()).to_bytes().to_vec(),
            pk_r: JubJubAffine::from(value.pk_r().as_ref())
                .to_bytes()
                .to_vec(),
        }
    }
}

impl From<&StealthAddress> for rusk_proto::StealthAddress {
    fn from(value: &StealthAddress) -> Self {
        (*value).into()
    }
}

impl From<&Transaction> for rusk_proto::Transaction {
    fn from(value: &Transaction) -> Self {
        let buf = value.payload.to_bytes();

        rusk_proto::Transaction {
            version: value.version.into(),
            r#type: value.tx_type.into(),
            payload: buf,
        }
    }
}

impl TryFrom<&mut Transaction> for rusk_proto::Transaction {
    type Error = Status;

    fn try_from(value: &mut Transaction) -> Result<Self, Status> {
        Ok(rusk_proto::Transaction::from(&*value))
    }
}

// ----- Protobuf types -> Basic types ----- //
impl TryFrom<&rusk_proto::PublicKey> for PublicSpendKey {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::PublicKey,
    ) -> Result<PublicSpendKey, Status> {
        Ok(PublicSpendKey::new(
            as_status_err(JubJubAffine::from_slice(&value.a_g))?.into(),
            as_status_err(JubJubAffine::from_slice(&value.b_g))?.into(),
        ))
    }
}

impl TryFrom<&rusk_proto::ViewKey> for ViewKey {
    type Error = Status;

    fn try_from(value: &rusk_proto::ViewKey) -> Result<ViewKey, Status> {
        Ok(ViewKey::new(
            as_status_err(JubJubScalar::from_slice(&value.a))?,
            as_status_err(JubJubAffine::from_slice(&value.b_g))?.into(),
        ))
    }
}

impl TryFrom<&rusk_proto::StealthAddress> for StealthAddress {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::StealthAddress,
    ) -> Result<StealthAddress, Status> {
        // Ensure that both fields are not empty
        let r_g = &value.r_g;
        let pk_r = &value.pk_r;

        // Ensure that both fields are 32 bytes long, so we can
        // safely copy from the slice.
        if r_g.len() != 32 || pk_r.len() != 32 {
            return Err(Status::failed_precondition(
                "StealthAddress fields are of improper length",
            ));
        };

        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&r_g[..]);
        bytes[32..].copy_from_slice(&pk_r[..]);

        Ok(StealthAddress::from_bytes(&bytes).map_err(|_| {
            Status::failed_precondition("StealthAdress was improperly encoded")
        })?)
    }
}

impl TryFrom<&mut rusk_proto::Transaction> for Transaction {
    type Error = Status;

    fn try_from(
        value: &mut rusk_proto::Transaction,
    ) -> Result<Transaction, Status> {
        let payload = TransactionPayload::from_bytes(value.payload.as_slice())
            .map_err(|e| {
                Status::new(Code::InvalidArgument, format!("{}", e))
            })?;

        Ok(Transaction {
            version: value
                .version
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            tx_type: value
                .r#type
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            payload,
        })
    }
}
