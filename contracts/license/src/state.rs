// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use alloc::vec::Vec;
use contract_helpers::Map;
use core::ops::Range;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use poseidon_merkle::{Opening, Tree};
use rusk_abi::PublicInput;

use crate::license_circuits::verifier_data_license_circuit;
use crate::license_types::{
    LicenseSession, LicenseSessionId, PoseidonItem, UseLicenseArg,
};

const DEPTH: usize = 17; // depth of the Merkle tree
const ARITY: usize = 4; // arity of the Merkle tree

#[derive(Debug, Clone)]
pub struct RequestEntry {
    pub block_height: u64,
    pub request: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LicenseEntry {
    pub block_height: u64,
    pub license: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LicenseContractState {
    pub sessions: Map<LicenseSessionId, LicenseSession>,
    pub licenses: Map<u64, LicenseEntry>,
    pub tree: Tree<(), DEPTH, ARITY>,
}

#[allow(dead_code)]
impl LicenseContractState {
    pub const fn new() -> Self {
        Self {
            sessions: Map::new(),
            licenses: Map::new(),
            tree: Tree::<(), DEPTH, ARITY>::new(),
        }
    }

    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

#[allow(dead_code)]
impl LicenseContractState {
    /// Inserts a license into the collection of licenses.
    /// Method intended to be called by the License Provider.
    pub fn issue_license(&mut self, license: Vec<u8>, hash: BlsScalar) {
        let item = PoseidonItem { hash, data: () };
        let mut pos = self.tree.len();
        while self.tree.contains(pos) {
            pos += 1;
        }
        self.tree.insert(pos, item);
        let block_height = rusk_abi::block_height();
        self.licenses.insert(
            pos,
            LicenseEntry {
                block_height,
                license,
            },
        );
    }

    /// Returns licenses for a given range of block-heights.
    /// Method intended to be called by the user.
    pub fn get_licenses(
        &mut self,
        block_heights: Range<u64>,
    ) -> Vec<(u64, Vec<u8>)> {
        self.licenses
            .entries_filter(|(_, le)| block_heights.contains(&le.block_height))
            .map(|(pos, le)| (*pos, le.license.clone()))
            .collect()
    }

    /// Returns merkle opening for a given position in the merkle tree of
    /// license hashes. Returns none if the given position slot in the tree is
    /// empty. Method intended to be called by the user.
    pub fn get_merkle_opening(
        &mut self,
        position: u64,
    ) -> Option<Opening<(), DEPTH, ARITY>> {
        self.tree.opening(position)
    }

    /// Verifies the proof of a given license, if successful,
    /// creates a session with the corresponding session id.
    /// Method intended to be called by the user.
    pub fn use_license(&mut self, use_license_arg: UseLicenseArg) {
        let mut pi = Vec::new();
        for scalar in use_license_arg.public_inputs.iter() {
            pi.push(PublicInput::BlsScalar(*scalar));
        }
        let vd = verifier_data_license_circuit();
        Self::assert_proof(vd, use_license_arg.proof.to_bytes().to_vec(), pi)
            .expect("Provided proof verification should succeed!");

        // after a successful proof verification we can add a session to a
        // shared list of sessions
        let license_session = LicenseSession {
            public_inputs: use_license_arg.public_inputs,
        };
        let session_id = license_session.session_id();
        if self.sessions.get(&session_id).is_some() {
            panic!("License already nullified");
        }
        self.sessions.insert(session_id, license_session);
    }

    /// Returns session with a given session id.
    /// Method intended to be called by the Service Provider.
    pub fn get_session(
        &self,
        session_id: LicenseSessionId,
    ) -> Option<LicenseSession> {
        self.sessions.get(&session_id).cloned()
    }

    /// Noop needed for inserting payloads into blockchain
    pub fn noop(&self) {}

    fn assert_proof(
        verifier_data: &[u8],
        proof: Vec<u8>,
        public_inputs: Vec<PublicInput>,
    ) -> Result<(), Error> {
        rusk_abi::verify_proof(verifier_data.to_vec(), proof, public_inputs)
            .then_some(())
            .ok_or(Error::ProofVerification)
    }

    /// Info about contract state
    pub fn get_info(&self) -> (u32, u32, u32) {
        (
            self.licenses.len() as u32,
            self.tree.len() as u32,
            self.sessions.len() as u32,
        )
    }
}
