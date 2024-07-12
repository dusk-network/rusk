// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used for interacting with Dusk's transfer and stake contracts.

#![no_std]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

/// Block height type alias
pub type BlockHeight = u64;

pub mod bytecode;
pub mod reader;
pub mod stake;
pub mod transfer;

// elliptic curve types
pub use dusk_bls12_381::BlsScalar;
pub use dusk_jubjub::{
    JubJubAffine, JubJubExtended, JubJubScalar, GENERATOR_EXTENDED,
    GENERATOR_NUMS_EXTENDED,
};

// signature types
pub use bls12_381_bls::{
    Error as BlsSigError, PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    Signature as BlsSignature, APK as BlsAggPublicKey,
};

/// Secret key associated to a stake.
pub type StakeSecretKey = BlsSecretKey;
/// Public key associated to a stake.
pub type StakePublicKey = BlsPublicKey;
/// Signature associated with a stake.
pub type StakeSignature = BlsSignature;
/// Aggregated public key for multisignatures
pub type StakeAggPublicKey = BlsAggPublicKey;

pub use jubjub_schnorr::{
    PublicKey as SchnorrPublicKey, SecretKey as SchnorrSecretKey,
    Signature as SchnorrSignature, SignatureDouble as SchnorrSignatureDouble,
};
/// Secret key associated with a note.
pub type NoteSecretKey = SchnorrSecretKey;
/// Public key associated with a note.
pub type NotePublicKey = SchnorrPublicKey;
/// Signature to prove ownership of the note
pub type NoteSignature = SchnorrSignature;

// phoenix types
pub use phoenix_core::{
    value_commitment, Error as PhoenixError, Note, PublicKey, SecretKey,
    Sender, StealthAddress, TxSkeleton, ViewKey, NOTE_VAL_ENC_SIZE,
    OUTPUT_NOTES,
};
