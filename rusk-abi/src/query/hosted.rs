// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{MetadataType, PublicInput, QueryType};

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{Signature as BlsSignature, APK};
use dusk_pki::{PublicKey, PublicSpendKey};
use dusk_plonk::prelude::Proof;
use dusk_schnorr::Signature;
use piecrust_uplink::{ModuleError, ModuleId};

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

/// Enum representing all possible payment configurations.
#[derive(Debug, Clone, Copy, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
#[repr(C)]
pub enum PaymentInfo {
    /// Only transparent notes are accepted.
    Transparent(Option<PublicSpendKey>),
    /// Only obfuscated notes are accepted.
    Obfuscated(Option<PublicSpendKey>),
    /// Any type of note is accepted.
    Any(Option<PublicSpendKey>),
}

/// Compute the blake2b hash of the given bytes, returning the resulting scalar.
/// The output of the hasher is truncated (last nibble) to fit onto a scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    let str = QueryType::Hash.as_str();
    piecrust_uplink::host_query(str, bytes)
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    let str = QueryType::PoseidonHash.as_str();
    piecrust_uplink::host_query(str, scalars)
}

/// Verify a proof is valid for a given circuit type and public inputs
pub fn verify_proof(
    verifier_data: Vec<u8>,
    proof: Proof,
    public_inputs: Vec<PublicInput>,
) -> bool {
    let str = QueryType::VerifyProof.as_str();
    piecrust_uplink::host_query(str, (verifier_data, proof, public_inputs))
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(msg: BlsScalar, pk: PublicKey, sig: Signature) -> bool {
    let str = QueryType::VerifySchnorr.as_str();
    piecrust_uplink::host_query(str, (msg, pk, sig))
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, apk: APK, sig: BlsSignature) -> bool {
    let str = QueryType::VerifyBls.as_str();
    piecrust_uplink::host_query(str, (msg, apk, sig))
}

/// Get the current block height.
pub fn block_height() -> u64 {
    let str = MetadataType::BlockHeight.as_str();
    piecrust_uplink::host_data(str)
}

/// Query a contract for the types of payment it accepts.
pub fn payment_info(module: ModuleId) -> Result<PaymentInfo, ModuleError> {
    piecrust_uplink::query(module, "payment_info", ())
}

/// Define a payment info for the contract.
#[macro_export]
macro_rules! payment_info {
    ($info:expr) => {
        mod payment_info {
            use rusk_abi::PaymentInfo;

            const PAYMENT_INFO: rusk_abi::PaymentInfo = $info;

            #[no_mangle]
            fn payment_info(arg_len: u32) -> u32 {
                rusk_abi::wrap_query(arg_len, |_: ()| PAYMENT_INFO)
            }
        }
    };
}
