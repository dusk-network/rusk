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
use dusk_pki::{Ownable, StealthAddress};
use dusk_plonk::prelude::*;
use dusk_poseidon::cipher::PoseidonCipher;
use dusk_poseidon::tree::PoseidonLeaf;
use nstack::annotation::Keyed;

#[derive(Debug, Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct DataLeaf {
    license_hash: BlsScalar,

    pos: u64,
}

// Keyed needs to be implemented for a leaf type and the tree key.
impl Keyed<()> for DataLeaf {
    fn key(&self) -> &() {
        &()
    }
}

#[allow(dead_code)]
impl DataLeaf {
    // pub fn random<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
    //     let license_hash = BlsScalar::random(rng);
    //     let pos = 0;
    //
    //     Self { license_hash, pos }
    // }
    pub fn new(hash: BlsScalar, n: u64) -> DataLeaf {
        DataLeaf {
            license_hash: hash,
            pos: n,
        }
    }
}

impl From<u64> for DataLeaf {
    fn from(n: u64) -> DataLeaf {
        DataLeaf {
            license_hash: BlsScalar::from(n),
            pos: n,
        }
    }
}

impl PoseidonLeaf for DataLeaf {
    fn poseidon_hash(&self) -> BlsScalar {
        // the license hash (the leaf) is computed into the circuit
        self.license_hash
    }

    fn pos(&self) -> &u64 {
        &self.pos
    }

    fn set_pos(&mut self, pos: u64) {
        self.pos = pos;
    }
}

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

/// SessionId.
#[derive(Debug, Clone, Copy, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct SessionId {
    id: BlsScalar,
}

impl SessionId {
    pub fn new(id: BlsScalar) -> SessionId {
        SessionId { id }
    }

    pub fn inner(&self) -> BlsScalar {
        self.id
    }
}

/// Session.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Session {
    pub session_hash: BlsScalar,
    pub session_id: BlsScalar,

    pub com_0: BlsScalar,      // Hash commitment 0
    pub com_1: JubJubExtended, // Pedersen Commitment 1
    pub com_2: JubJubExtended, // Pedersen Commitment 2
}

impl Session {
    pub fn from(public_inputs: &[BlsScalar]) -> Self {
        // public inputs are in negated form, we negate them again to assert
        // correctly
        let session_id = -public_inputs[0];
        let session_hash = -public_inputs[1];

        let com_0 = -public_inputs[2];
        let com_1 = JubJubExtended::from(JubJubAffine::from_raw_unchecked(
            -public_inputs[3],
            -public_inputs[4],
        ));
        let com_2 = JubJubExtended::from(JubJubAffine::from_raw_unchecked(
            -public_inputs[5],
            -public_inputs[6],
        ));

        Self {
            session_hash,
            session_id,

            com_0,
            com_1,
            com_2,
        }
    }

    pub fn session_id(&self) -> SessionId {
        SessionId {
            id: self.session_id,
        }
    }
}

/// License.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct License {
    pub lsa: StealthAddress, // license stealth address
    pub enc_1: PoseidonCipher, /* encryption of the license signature and
                              * attributes */
    pub nonce_1: BlsScalar, // IV for the encryption
    pub enc_2: PoseidonCipher, /* encryption of the license signature and
                             * attributes */
    pub nonce_2: BlsScalar, // IV for the encryption
    pub pos: BlsScalar,     /* position of the license in the Merkle tree of
                             * licenses */
}

/// Use License Request.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct UseLicenseArg {
    pub proof: Proof,
    pub public_inputs: Vec<BlsScalar>,
    pub license: License,
}

#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Request {
    pub rsa: StealthAddress, // request stealth address
    pub enc_1: PoseidonCipher, /* encryption of the license stealth address
                              * and k_lic */
    pub nonce_1: BlsScalar, // IV for the encryption
    pub enc_2: PoseidonCipher, /* encryption of the license stealth address
                             * and k_lic */
    pub nonce_2: BlsScalar, // IV for the encryption
    pub enc_3: PoseidonCipher, /* encryption of the license stealth address
                             * and k_lic */
    pub nonce_3: BlsScalar, // IV for the encryption
}

impl Ownable for Request {
    fn stealth_address(&self) -> &StealthAddress {
        &self.rsa
    }
}
