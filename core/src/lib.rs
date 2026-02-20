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
#![allow(clippy::used_underscore_binding)]
#![cfg_attr(not(target_family = "wasm"), deny(unused_crate_dependencies))]
#![deny(unused_extern_crates)]

extern crate alloc;

pub mod abi;

pub mod stake;
pub mod transfer;

mod error;
pub use error::{Error, TxPreconditionError};

mod dusk;
pub use dusk::{Dusk, LUX, dusk, from_dusk};

use blake2b_simd as _; // Required to satisfy unused_crate_dependencies

// elliptic curve types
pub use dusk_bls12_381::BlsScalar;
pub use dusk_jubjub::{
    GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED, JubJubAffine, JubJubExtended,
    JubJubScalar,
};

/// Signatures used in the Dusk protocol.
pub mod signatures {
    /// Types for the bls-signature scheme operating on the `bls12_381` curve.
    pub mod bls {
        pub use bls12_381_bls::{
            Error, MultisigPublicKey, MultisigSignature, PublicKey, SecretKey,
            Signature,
        };

        /// BLS signature scheme version.
        #[derive(Debug, Clone, Copy, Eq, PartialEq)]
        pub enum BlsVersion {
            /// Insecure v1 (pre-fork historical compatibility).
            V1,
            /// Secure v2 using RFC 9380 hash-to-curve.
            V2,
        }

        /// Verify a single BLS signature.
        ///
        /// # Errors
        /// Returns an error if the signature is invalid.
        pub fn verify(
            pk: &PublicKey,
            sig: &Signature,
            msg: &[u8],
            v: BlsVersion,
        ) -> Result<(), Error> {
            match v {
                BlsVersion::V2 => pk.verify(sig, msg),
                BlsVersion::V1 => pk.verify_insecure(sig, msg),
            }
        }

        /// Verify a BLS multi-signature.
        ///
        /// # Errors
        /// Returns an error if the signature is invalid.
        pub fn verify_multisig(
            apk: &MultisigPublicKey,
            sig: &MultisigSignature,
            msg: &[u8],
            v: BlsVersion,
        ) -> Result<(), Error> {
            match v {
                BlsVersion::V2 => apk.verify(sig, msg),
                BlsVersion::V1 => apk.verify_insecure(sig, msg),
            }
        }

        /// Aggregate public keys for multi-signature verification.
        ///
        /// # Errors
        /// Returns an error if the key list is empty.
        pub fn aggregate(
            pks: &[PublicKey],
            v: BlsVersion,
        ) -> Result<MultisigPublicKey, Error> {
            match v {
                BlsVersion::V2 => MultisigPublicKey::aggregate(pks),
                BlsVersion::V1 => MultisigPublicKey::aggregate_insecure(pks),
            }
        }

        /// Sign a message using the multi-signature scheme.
        #[must_use]
        pub fn sign_multisig(
            sk: &SecretKey,
            pk: &PublicKey,
            msg: &[u8],
            v: BlsVersion,
        ) -> MultisigSignature {
            match v {
                BlsVersion::V2 => sk.sign_multisig(pk, msg),
                BlsVersion::V1 => sk.sign_multisig_insecure(pk, msg),
            }
        }

        /// Sign a message using the single-signature scheme.
        #[must_use]
        pub fn sign(sk: &SecretKey, msg: &[u8], v: BlsVersion) -> Signature {
            match v {
                BlsVersion::V2 => sk.sign(msg),
                BlsVersion::V1 => sk.sign_insecure(msg),
            }
        }
    }

    /// Types for the schnorr-signature scheme operating on the `jubjub` curve.
    pub mod schnorr {
        pub use jubjub_schnorr::{
            PublicKey, SecretKey, Signature, SignatureDouble,
        };
    }
}

/// Types and traits to create plonk circuits and generate and verify plonk
/// proofs.
#[cfg(feature = "plonk")]
pub mod plonk {
    pub use dusk_plonk::prelude::{
        Circuit, Compiler, Composer, Constraint, Error, PlonkVersion, Proof,
        Prover, PublicParameters, Verifier, Witness, WitnessPoint,
    };
}

/// Groth16 circuitry
#[cfg(feature = "groth16")]
pub mod groth16 {
    pub use ark_bn254 as bn254;
    pub use ark_groth16::{
        Groth16, PreparedVerifyingKey, Proof, ProvingKey, VerifyingKey,
        data_structures, generator, prepare_verifying_key, prover, r1cs_to_qap,
        verifier,
    };
    pub use ark_relations as relations;
    pub use ark_serialize as serialize;
}

#[inline]
const fn reserved(b: u8) -> abi::ContractId {
    let mut bytes = [0u8; abi::CONTRACT_ID_BYTES];
    bytes[0] = b;
    abi::ContractId::from_bytes(bytes)
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

#[cfg(test)]
mod tests {
    // the `unused_crate_dependencies` lint complains for dev-dependencies that
    // are only used in integration tests, so adding this work-around here
    use serde_json as _;
}
