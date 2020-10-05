// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]
use super::services::rusk_proto;
<<<<<<< HEAD
use crate::transaction::{Transaction, TransactionPayload};
use core::convert::{TryFrom, TryInto};
use dusk_pki::{
    Ownable, PublicSpendKey, SecretSpendKey, StealthAddress, ViewKey,
};
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::{
    AffinePoint as JubJubAffine, ExtendedPoint as JubJubExtended,
    Scalar as JubJubScalar,
};
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Note};
use poseidon252::cipher::{PoseidonCipher, CIPHER_BYTES_SIZE};
use std::io::{Read, Write};
use tonic::{Code, Status};

/// Generic function used to retrieve parameters that are optional from a
/// GRPC request.
pub(crate) fn decode_request_param<T, U>(
    possible_param: Option<&T>,
) -> Result<U, Status>
where
    T: Clone,
    U: TryFrom<T, Error = Status>,
{
    Ok(U::try_from(
        possible_param
            .ok_or(Status::new(Code::Unknown, "Missing required fields."))?
            .clone(),
    )?)
}

/// Generic function used to encore parameters that are optional in a
/// GRPC response.
pub(crate) fn encode_request_param<T, U>(param: T) -> Option<U>
where
    U: From<T>,
{
    Some(U::from(param))
}

// ---- Basic Types -> Protobuf types ---- //
impl From<JubJubAffine> for rusk_proto::JubJubCompressed {
    fn from(value: JubJubAffine) -> Self {
        rusk_proto::JubJubCompressed {
            data: Vec::from(&value.to_bytes()[..]),
        }
    }
}

impl From<JubJubExtended> for rusk_proto::JubJubCompressed {
    fn from(value: JubJubExtended) -> Self {
        JubJubAffine::from(value).into()
    }
}

impl From<&JubJubExtended> for rusk_proto::JubJubCompressed {
    fn from(value: &JubJubExtended) -> Self {
        (*value).into()
    }
}

impl From<&JubJubScalar> for rusk_proto::JubJubScalar {
    fn from(value: &JubJubScalar) -> Self {
        rusk_proto::JubJubScalar {
            data: Vec::from(&value.to_bytes()[..]),
        }
    }
}

impl From<JubJubScalar> for rusk_proto::JubJubScalar {
    fn from(value: JubJubScalar) -> Self {
        (&value).into()
    }
}

impl From<&BlsScalar> for rusk_proto::BlsScalar {
    fn from(value: &BlsScalar) -> Self {
        rusk_proto::BlsScalar {
            data: Vec::from(&value.to_bytes()[..]),
        }
    }
}
=======
use core::convert::TryFrom;
use dusk_pki::{
    jubjub_decode, PublicSpendKey, SecretSpendKey, StealthAddress, ViewKey,
};
use dusk_plonk::jubjub::AffinePoint as JubJubAffine;
use dusk_plonk::prelude::*;
use tonic::Status;

/// Wrapper over `jubjub_decode` fn
pub(crate) fn decode_affine(bytes: &[u8]) -> Result<JubJubAffine, Status> {
    jubjub_decode::<JubJubAffine>(bytes).map_err(|_| {
        Status::failed_precondition("Point was improperly encoded")
    })
}

/// Wrapper over `jubjub_decode` fn
pub(crate) fn decode_jubjub_scalar(
    bytes: &[u8],
) -> Result<JubJubScalar, Status> {
    jubjub_decode::<JubJubScalar>(bytes).map_err(|_| {
        Status::failed_precondition("JubjubScalar was improperly encoded")
    })
}

/// Decoder fn used for `BlsScalar`
pub(crate) fn decode_bls_scalar(bytes: &[u8]) -> Result<BlsScalar, Status> {
    if bytes.len() < 32 {
        Err(Status::failed_precondition(
            "Not enough bytes to decode a BlsScalar",
        ))
    } else {
        let bytes = <&[u8; 32]>::try_from(bytes).map_err(|_| {
            Status::failed_precondition(
                "Expecting 32 bytes to decode a BlsScalar",
            )
        })?;
>>>>>>> master

        Option::from(BlsScalar::from_bytes(&bytes)).ok_or_else(|| {
            Status::failed_precondition("Point was improperly encoded")
        })
    }
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
            pk_r: JubJubAffine::from(value.pk_r()).to_bytes().to_vec(),
        }
    }
}

impl From<&StealthAddress> for rusk_proto::StealthAddress {
    fn from(value: &StealthAddress) -> Self {
        (*value).into()
    }
}

impl TryFrom<&mut Note> for rusk_proto::Note {
    type Error = Status;

    fn try_from(value: &mut Note) -> Result<Self, Status> {
        let mut bytes = [0u8; 233];
        value.read(&mut bytes)?;
        Ok(rusk_proto::Note {
            note_type: value.note() as i32,
            value_commitment: Some(value.value_commitment().into()),
            nonce: Some(value.nonce().into()),
            address: Some(value.stealth_address().into()),
            pos: value.pos(),
            encrypted_data: Some(rusk_proto::PoseidonCipher {
                data: bytes[137..233].to_vec(),
            }),
        })
    }
}

impl From<Fee> for rusk_proto::Fee {
    fn from(value: Fee) -> Self {
        rusk_proto::Fee {
            gas_limit: value.gas_limit,
            gas_price: value.gas_price,
            address: Some(value.stealth_address().into()),
        }
    }
}

impl From<Crossover> for rusk_proto::Crossover {
    fn from(value: Crossover) -> Self {
        rusk_proto::Crossover {
            value_comm: Some(
                JubJubAffine::from(value.value_commitment()).into(),
            ),
            nonce: Some((*value.nonce()).into()),
            /// XXX: fix this typo in rusk-schema
            encypted_data: Some((*value.encrypted_data()).into()),
        }
    }
}

impl TryFrom<&TransactionPayload> for rusk_proto::TransactionPayload {
    type Error = Status;

    fn try_from(value: &TransactionPayload) -> Result<Self, Status> {
        Ok(rusk_proto::TransactionPayload {
            anchor: Some(value.anchor.into()),
            nullifier: value
                .nullifiers
                .to_vec()
                .into_iter()
                .map(|n| n.into())
                .collect(),
            crossover: value.crossover.map(|c| c.into()),
            notes: value
                .notes
                .to_vec()
                .into_iter()
                .map(|mut n| (&mut n).try_into())
                .collect::<Result<Vec<rusk_proto::Note>, _>>()?,
            fee: Some(value.fee.into()),
            spending_proof: Some((&value.spending_proof).into()),
            call_data: value.call_data.to_vec(),
        })
    }
}

impl TryFrom<TransactionPayload> for rusk_proto::TransactionPayload {
    type Error = Status;

    fn try_from(value: TransactionPayload) -> Result<Self, Status> {
        (&value).try_into()
    }
}

impl TryFrom<Transaction> for rusk_proto::Transaction {
    type Error = Status;

    fn try_from(value: Transaction) -> Result<Self, Status> {
        Ok(rusk_proto::Transaction {
            version: value.version.into(),
            r#type: value.tx_type.into(),
            tx_payload: Some(value.payload.try_into()?),
        })
    }
}

// ----- Protobuf types -> Basic types ----- //
impl TryFrom<&rusk_proto::PublicKey> for PublicSpendKey {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::PublicKey,
    ) -> Result<PublicSpendKey, Status> {
        Ok(PublicSpendKey::new(
            decode_affine(&value.a_g)?.into(),
            decode_affine(&value.b_g)?.into(),
        ))
    }
}

impl TryFrom<&rusk_proto::ViewKey> for ViewKey {
    type Error = Status;

    fn try_from(value: &rusk_proto::ViewKey) -> Result<ViewKey, Status> {
        Ok(ViewKey::new(
            decode_jubjub_scalar(&value.a)?,
            decode_affine(&value.b_g)?.into(),
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

impl TryFrom<&rusk_proto::Transaction> for Transaction {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::Transaction,
    ) -> Result<Transaction, Status> {
        Ok(Transaction {
            version: value
                .version
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            tx_type: value
                .r#type
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            payload: value
                .tx_payload
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No transaction payload present",
                ))?
                .try_into()?,
        })
    }
}
