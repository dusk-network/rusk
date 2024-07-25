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

extern crate alloc;

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
