// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

use super::services::rusk_proto;
use core::convert::TryFrom;
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::{AffinePoint as JubJubAffine, Scalar as JubJubScalar};
use tonic::{Code, Status};

/// Generic function used to retrieve parameters that are optional from a
/// GRPC request.
pub(crate) fn decode_request_param<T>(
    possible_param: Option<&T>,
) -> Result<&T, Status>
where
    T: Clone,
{
    possible_param.ok_or(Status::new(Code::Unknown, "Missing required fields."))
}

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
        let possible_scalar = BlsScalar::from_bytes(&bytes);
        if possible_scalar.is_none().into() {
            return Err(Status::failed_precondition(
                "BlsScalar was not cannonically encoded",
            ));
        };
        Ok(possible_scalar.unwrap())
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
        let possible_point = JubJubAffine::from_bytes(bytes);
        if possible_point.is_none().into() {
            return Err(Status::failed_precondition(
                "JubJubAffine was not cannonically encoded",
            ));
        };
        Ok(possible_point.unwrap())
    }
}
