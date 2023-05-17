// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::{DataLeaf, License, Request, Session, SessionId, UseLicenseArg};
use alloc::vec::Vec;
use contract_helpers::Map;
use dusk_bytes::Serializable;
use dusk_pki::ViewKey;
use dusk_poseidon::tree::PoseidonTree;
use rusk_abi::PublicInput;

use crate::license_circuits::verifier_data_license_circuit;
const DEPTH: usize = 17; // depth of the 4-ary Merkle tree

/// License contract.
#[derive(Debug, Clone)]
pub struct LicensesData {
    pub requests: Vec<Request>,
    pub sessions: Map<SessionId, Session>,
    pub licenses: Map<u64, License>,
    pub tree: PoseidonTree<DataLeaf, (), DEPTH>,
}

#[allow(dead_code)]
impl LicensesData {
    pub const fn new() -> Self {
        Self {
            requests: Vec::new(),
            sessions: Map::new(),
            licenses: Map::new(),
            tree: PoseidonTree::<DataLeaf, (), DEPTH>::new(),
        }
    }

    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

#[allow(dead_code)]
impl LicensesData {
    /// Inserts a given license request in a collection of requests.
    /// Method intended to be called by the user.
    pub fn request_license(&mut self, request: Request) {
        self.requests.push(request);
    }

    /**
     * FIXME: Note that we base the querying for a license on ViewKey,
     * which is not optimal from the privacy point of view.
     * We rely on this solution until a streaming infrastructure
     * for contracts is available, so that we can move the filtering
     * part to the license provider, for proper anonymization.
     * */
    /// Returns and removes first found license request for a given License
    /// Provider. Returns None if not such license request is found.
    /// Method intended to be called by the License Provider.
    pub fn get_license_request(
        &mut self,
        view_key: ViewKey,
    ) -> Option<Request> {
        self.requests
            .iter()
            .position(|r| view_key.owns(r))
            .map(|index| self.requests.swap_remove(index))
    }

    /// Inserts a given license in the collection of licenses
    /// Method intended to be called by the License Provider.
    pub fn issue_license(&mut self, license: License) {
        rusk_abi::debug!("issuing license (contract) at pos {}", license.pos);
        // todo: fixme: remove the code below and take 'pos' from 'license'
        let temp = self.licenses.len() + 1; // we need to make pos unique eventually
                                            // self.licenses.insert(license.pos, license);
        self.licenses.insert(temp as u64, license);
        // insert License into the tree at position `license.pos`
        // self.licenses.insert(license.pos, license);
    }

    /// Returns licenses for a given user.
    /// Returns an empty collection if no licenses are found.
    /// Method intended to be called by the user.
    pub fn get_licenses(&mut self, view_key: ViewKey) -> Vec<License> {
        self.licenses
            .filter(|l| view_key.owns(l))
            .into_iter()
            .cloned()
            .collect()
    }

    /// Returns merkle proof for a given position in the merkle tree of license
    /// hashes. If the position is empty, returns None.
    /// Method intended to be called by the user.
    pub fn get_merkle_proof(&mut self, _position: u64) -> Option<Vec<u64>> {
        Some(Vec::<u64>::new())
    }

    /// Verifies the proof of a given license, if successful,
    /// creates a session with the corresponding session id.
    /// Method intended to be called by the user.
    pub fn use_license(&mut self, use_license_arg: UseLicenseArg) -> SessionId {
        let mut pi = Vec::new();
        for scalar in use_license_arg.public_inputs.iter() {
            pi.push(PublicInput::BlsScalar(scalar.neg()))
        }
        let vd = verifier_data_license_circuit();
        Self::assert_proof(vd, use_license_arg.proof.to_bytes().to_vec(), pi)
            .expect("Provided proof should succeed!");

        // after a successful proof verification we can add a session to a
        // shared list of sessions
        let license_session =
            Session::from(use_license_arg.public_inputs.as_slice());
        self.sessions
            .insert(license_session.session_id(), license_session.clone());
        license_session.session_id()
    }

    /// Returns session with a given session id.
    /// Method intended to be called by the Service Provider.
    pub fn get_session(&self, session_id: SessionId) -> Option<Session> {
        self.sessions.get(&session_id).cloned()
    }

    fn assert_proof(
        verifier_data: &[u8],
        proof: Vec<u8>,
        public_inputs: Vec<PublicInput>,
    ) -> Result<(), Error> {
        rusk_abi::verify_proof(verifier_data.to_vec(), proof, public_inputs)
            .then(|| ())
            .ok_or(Error::ProofVerificationError)
    }
}
