// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::collection::Map;
use crate::error::Error;
use crate::{ContractLicense, LicenseNullifier, LicenseRequest, LicenseSession, SPPublicKey, UseLicenseArg, UserPublicKey};
use alloc::vec::Vec;
use rusk_abi::PublicInput;
use dusk_bytes::Serializable;
// use dusk_plonk::prelude::*;

use crate::license_circuits::verifier_data_license_circuit;

/// License contract.
#[derive(Debug, Clone)]
pub struct LicensesData {
    pub requests: Vec<LicenseRequest>,
    pub sessions: Map<LicenseNullifier, LicenseSession>,
    pub licenses: Vec<ContractLicense>,
}

#[allow(dead_code)]
impl LicensesData {
    pub const fn new() -> Self {
        Self {
            requests: Vec::new(),
            sessions: Map::new(),
            licenses: Vec::new(),
        }
    }

    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

#[allow(dead_code)]
impl LicensesData {
    /// Inserts a given license request in a collection of requests.
    pub fn request_license(&mut self, request: LicenseRequest) {
        rusk_abi::debug!("License contract: request_license");
        self.requests.push(request);
    }

    /// Returns and removes first found license request for a given SP.
    /// If not such license request is found, returns None.
    pub fn get_license_request(
        &mut self,
        sp_public_key: SPPublicKey,
    ) -> Option<LicenseRequest> {
        rusk_abi::debug!(
            "License contract: get_license_request {:?}",
            sp_public_key
        );
        self.requests
            .iter()
            .position(|e| e.sp_public_key == sp_public_key)
            .map(|index| self.requests.swap_remove(index))
    }

    /// Inserts a given license in a collection of licenses
    pub fn issue_license(&mut self, license: ContractLicense) {
        rusk_abi::debug!("License contract: issue_license");
        self.licenses.push(license);
    }

    /// Returns and removes first found license for a given user.
    /// If not such license is found, returns None.
    pub fn get_license(
        &mut self,
        user_pk: UserPublicKey,
    ) -> Option<ContractLicense> {
        rusk_abi::debug!("License contract: get_license");
        self.licenses
            .iter()
            .position(|e| e.user_pk == user_pk)
            .map(|index| self.licenses.swap_remove(index))
    }

    /// Verifies the proof of a given license, if successful,
    /// creates a session with the corresponding nullifier.
    pub fn use_license(&mut self, use_license_arg: UseLicenseArg) {
        let mut pi = Vec::new();
        for scalar in use_license_arg.public_inputs {
            pi.push(PublicInput::BlsScalar(scalar))
        }
        let vd = verifier_data_license_circuit();
        Self::assert_proof(vd, use_license_arg.proof.to_bytes().to_vec(), pi)
            .expect("Provided proof should succeed!");
    }

    /// Returns session containing a given license nullifier.
    pub fn get_session(
        &self,
        nullifier: LicenseNullifier,
    ) -> Option<LicenseSession> {
        rusk_abi::debug!("License contract: get_session {:?}", nullifier);
        self.sessions.get(&nullifier).cloned()
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
