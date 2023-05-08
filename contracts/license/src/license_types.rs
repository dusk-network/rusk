// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubAffine;
use dusk_pki::StealthAddress;
use dusk_plonk::prelude::*;
use dusk_schnorr::Signature;
use dusk_poseidon::cipher::PoseidonCipher;

/// SP Public Key.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SPPublicKey {
    pub sp_pk: u64,
}

/// User Public Key.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UserPublicKey {
    pub user_pk: JubJubAffine,
}

/// License Nullifier.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseNullifier {
    pub value: BlsScalar,
}

// License Request.
// #[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
// #[archive_attr(derive(CheckBytes))]
// pub struct LicenseRequest {
//     pub sp_public_key: SPPublicKey,
// }

/// License Session.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct LicenseSession {
    pub nullifier: LicenseNullifier,
}

/// License.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct ContractLicense {
    pub user_pk: UserPublicKey,
    pub sp_pk: SPPublicKey,
    pub sig_lic: Signature,
}

/// Use License Request.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UseLicenseArg {
    pub proof: Proof,
    pub public_inputs: Vec<BlsScalar>,
    pub license: ContractLicense,
}

#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Request {
    pub rsa: StealthAddress,   // request stealth address
    pub enc_1: PoseidonCipher, // encryption of the license stealth address and k_lic
    pub nonce_1: BlsScalar,    // IV for the encryption
    pub enc_2: PoseidonCipher, // encryption of the license stealth address and k_lic
    pub nonce_2: BlsScalar,    // IV for the encryption
    pub enc_3: PoseidonCipher, // encryption of the license stealth address and k_lic
    pub nonce_3: BlsScalar,    // IV for the encryption
}
