// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(non_snake_case)]
use super::services::rusk_proto;
use core::convert::TryFrom;
use dusk_pki::{
    jubjub_decode, PublicSpendKey, SecretSpendKey, StealthAddress, ViewKey,
};
use dusk_plonk::jubjub::AffinePoint as JubJubAffine;
use dusk_plonk::prelude::*;
use tonic::{Code, Status};

/// Generic function used to retrieve parameters that are optional from a
/// GRPC request.
pub(crate) fn decode_optional_request_param<T, U>(
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
pub(crate) fn encode_optional_request_param<T, U>(param: T) -> Option<U>
where
    U: From<T>,
{
    Some(U::from(param))
}

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
        let mut buff = [0u8; 32];
        buff.copy_from_slice(&bytes[0..32]);
        Option::from(BlsScalar::from_bytes(&buff)).ok_or_else(|| {
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
            decode_jubjub_scalar(&value.a)?.into(),
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
