// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use super::services::rusk_proto;
use core::convert::TryFrom;
use dusk_pki::{PublicSpendKey, SecretSpendKey, StealthAddress, ViewKey};
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::{AffinePoint as JubJubAffine, Scalar as JubJubScalar};
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

impl From<BlsScalar> for rusk_proto::BlsScalar {
    fn from(value: BlsScalar) -> Self {
        (&value).into()
    }
}

impl From<PublicSpendKey> for rusk_proto::PublicKey {
    fn from(value: PublicSpendKey) -> Self {
        rusk_proto::PublicKey {
            a_g: Some(JubJubAffine::from(value.A()).into()),
            b_g: Some(JubJubAffine::from(value.B()).into()),
        }
    }
}

impl From<SecretSpendKey> for rusk_proto::SecretKey {
    fn from(value: SecretSpendKey) -> Self {
        rusk_proto::SecretKey {
            a: Some(value.a().into()),
            b: Some(value.b().into()),
        }
    }
}

impl From<ViewKey> for rusk_proto::ViewKey {
    fn from(value: ViewKey) -> Self {
        rusk_proto::ViewKey {
            a: Some(value.a().into()),
            b_g: Some(JubJubAffine::from(value.B()).into()),
        }
    }
}

impl From<StealthAddress> for rusk_proto::StealthAddress {
    fn from(value: StealthAddress) -> Self {
        rusk_proto::StealthAddress {
            r_g: Some(JubJubAffine::from(value.R()).into()),
            pk_r: Some(JubJubAffine::from(value.pk_r()).into()),
        }
    }
}

// ----- Protobuf types -> Basic types ----- //
impl TryFrom<&rusk_proto::BlsScalar> for BlsScalar {
    type Error = Status;

    fn try_from(value: &rusk_proto::BlsScalar) -> Result<BlsScalar, Status> {
        let mut bytes = [0u8; 32];
        // Check if the data is 32 bytes exactly so we can
        // safely copy from the slice.
        if value.data.len() != 32 {
            return Err(Status::failed_precondition(
                "BlsScalar recieved is not 32 bytes long",
            ));
        };
        bytes[..].copy_from_slice(&value.data[..]);
        Option::from(BlsScalar::from_bytes(&bytes)).ok_or(
            Status::failed_precondition(
                "BlsScalar was not cannonically encoded",
            ),
        )
    }
}

impl TryFrom<&rusk_proto::JubJubScalar> for JubJubScalar {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::JubJubScalar,
    ) -> Result<JubJubScalar, Status> {
        let mut bytes = [0u8; 32];
        // Check if the data is 32 bytes exactly so we can
        // safely copy from the slice.
        if value.data.len() != 32 {
            return Err(Status::failed_precondition(
                "JubJubScalar recieved is not 32 bytes long",
            ));
        };
        bytes[..].copy_from_slice(&value.data[..]);
        Option::from(JubJubScalar::from_bytes(&bytes)).ok_or(
            Status::failed_precondition(
                "JubJubScalar was not cannonically encoded",
            ),
        )
    }
}

impl TryFrom<&rusk_proto::JubJubCompressed> for JubJubAffine {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::JubJubCompressed,
    ) -> Result<JubJubAffine, Status> {
        let mut bytes = [0u8; 32];
        // Check if the data is 32 bytes exactly so we can
        // safely copy from the slice.
        if value.data.len() != 32 {
            return Err(Status::failed_precondition(
                "JubJubAffine recieved is not 32 bytes long",
            ));
        };
        bytes[..].copy_from_slice(&value.data[..]);
        Option::from(JubJubAffine::from_bytes(bytes)).ok_or(
            Status::failed_precondition(
                "JubJubAffine was not cannonically encoded",
            ),
        )
    }
}

impl TryFrom<&rusk_proto::PublicKey> for PublicSpendKey {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::PublicKey,
    ) -> Result<PublicSpendKey, Status> {
        Ok(
            PublicSpendKey::new(
                decode_request_param::<
                    &rusk_proto::JubJubCompressed,
                    JubJubAffine,
                >(value.a_g.as_ref().as_ref())?
                .into(),
                decode_request_param::<
                    &rusk_proto::JubJubCompressed,
                    JubJubAffine,
                >(value.a_g.as_ref().as_ref())?
                .into(),
            ),
        )
    }
}

impl TryFrom<&rusk_proto::StealthAddress> for StealthAddress {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::StealthAddress,
    ) -> Result<StealthAddress, Status> {
        let mut bytes = [0u8; 64];
        // Ensure that both fields are not empty
        let r_g = value.r_g.as_ref().ok_or(Status::failed_precondition(
            "StealthAddress was missing r_g field",
        ))?;
        let pk_r = value.pk_r.as_ref().ok_or(Status::failed_precondition(
            "StealthAddress was missing pk_r field",
        ))?;

        // Ensure that both fields are 32 bytes long, so we can
        // safely copy from the slice.
        if r_g.data.len() != 32 || pk_r.data.len() != 32 {
            return Err(Status::failed_precondition(
                "StealthAddress fields are of improper length",
            ));
        };

        bytes[..32].copy_from_slice(&r_g.data[..]);
        bytes[32..].copy_from_slice(&pk_r.data[..]);

        Ok(StealthAddress::from_bytes(&bytes).map_err(|_| {
            Status::failed_precondition("StealthAdress was improperly encoded")
        })?)
    }
}
