// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/*
#![allow(dead_code)]
use core::convert::TryFrom;
use dusk_pki::jubjub_decode;
use dusk_plonk::jubjub::JubJubAffine;
use dusk_plonk::prelude::*;
use tonic::{Code, Status};

/// Generic function used to retrieve parameters that are optional from a
/// GRPC request.
pub fn decode_request_param<T, U>(
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
pub fn encode_optional_request_param<T, U>(param: T) -> Option<U>
where
    U: From<T>,
{
    Some(U::from(param))
}

/// Wrapper over `jubjub_decode` fn
pub fn decode_affine(bytes: &[u8]) -> Result<JubJubAffine, Status> {
    jubjub_decode::<JubJubAffine>(bytes).map_err(|_| {
        Status::failed_precondition("Point was improperly encoded")
    })
}

/// Wrapper over `jubjub_decode` fn
pub fn decode_jubjub_scalar(bytes: &[u8]) -> Result<JubJubScalar, Status> {
    jubjub_decode::<JubJubScalar>(bytes).map_err(|_| {
        Status::failed_precondition("JubjubScalar was improperly encoded")
    })
}

/// Decoder fn used for `BlsScalar`
pub fn decode_bls_scalar(bytes: &[u8]) -> Result<BlsScalar, Status> {
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
*/
