// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for interacting with Dusk's transfer and stake contracts.

#![no_std]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![feature(const_fn_floating_point_arithmetic)]

extern crate alloc;

pub mod license;
pub mod stake;
pub mod transfer;

mod error;
pub use error::Error;

mod dusk;
pub use dusk::{dusk, from_dusk, Dusk, LUX};

// elliptic curve types
pub use dusk_bls12_381::BlsScalar;
pub use dusk_jubjub::{
    JubJubAffine, JubJubExtended, JubJubScalar, GENERATOR_EXTENDED,
    GENERATOR_NUMS_EXTENDED,
};

/// Signatures used in the Dusk protocol.
pub mod signatures {
    /// Types for the bls-signature scheme operating on the `bls12_381` curve.
    pub mod bls {
        pub use bls12_381_bls::{
            Error, MultisigPublicKey, MultisigSignature, PublicKey, SecretKey,
            Signature,
        };
    }

    /// Types for the schnorr-signature scheme operating on the `jubjub` curve.
    pub mod schnorr {
        pub use jubjub_schnorr::{
            PublicKey, SecretKey, Signature, SignatureDouble,
        };
    }
}

pub use piecrust_uplink::{
    ContractError, ContractId, Event, StandardBufSerializer, ARGBUF_LEN,
    CONTRACT_ID_BYTES,
};

/// Types and traits to create plonk circuits and generate and verify plonk
/// proofs.
#[cfg(feature = "zk")]
pub mod plonk {
    pub use dusk_plonk::prelude::{
        Circuit, Compiler, Composer, Constraint, Error, Proof, Prover,
        PublicParameters, Verifier, Witness, WitnessPoint,
    };
}

#[inline]
const fn reserved(b: u8) -> ContractId {
    let mut bytes = [0u8; CONTRACT_ID_BYTES];
    bytes[0] = b;
    ContractId::from_bytes(bytes)
}

use alloc::string::String;
use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Error as BytesError};

/// Reads vector from a buffer.
/// Resets buffer to a position after the bytes read.
///
/// # Errors
/// When length or data could not be read.
fn read_vec(buf: &mut &[u8]) -> Result<Vec<u8>, BytesError> {
    let len = usize::try_from(u64::from_reader(buf)?)
        .map_err(|_| BytesError::InvalidData)?;
    if buf.len() < len {
        return Err(BytesError::InvalidData);
    }
    let bytes = buf[..len].into();
    *buf = &buf[len..];
    Ok(bytes)
}

/// Reads string from a buffer.
/// Resets buffer to a position after the bytes read.
///
/// # Errors
/// When length or data could not be read.
fn read_str(buf: &mut &[u8]) -> Result<String, BytesError> {
    let len = usize::try_from(u64::from_reader(buf)?)
        .map_err(|_| BytesError::InvalidData)?;
    if buf.len() < len {
        return Err(BytesError::InvalidData);
    }
    let str = String::from_utf8(buf[..len].into())
        .map_err(|_| BytesError::InvalidData)?;
    *buf = &buf[len..];
    Ok(str)
}

/// Reads array from a buffer.
/// Resets buffer to a position after the bytes read.
///
/// # Errors
/// When length or data could not be read.
fn read_arr<const N: usize>(buf: &mut &[u8]) -> Result<[u8; N], BytesError> {
    if buf.len() < N {
        return Err(BytesError::InvalidData);
    }
    let mut a = [0u8; N];
    a.copy_from_slice(&buf[..N]);
    *buf = &buf[N..];
    Ok(a)
}
