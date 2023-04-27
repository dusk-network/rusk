// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::collection::Map;
use crate::{
    License, LicenseNullifier, LicenseRequest, LicenseSession, SPPublicKey,
    UserPublicKey,
};
use alloc::vec::Vec;

/// License contract.
#[derive(Debug, Clone)]
pub struct LicensesData {
    pub requests: Vec<LicenseRequest>,
    pub sessions: Map<LicenseNullifier, LicenseSession>,
    pub licenses: Map<UserPublicKey, License>, /* todo: key is has to allow multiple licenses per user */
}

#[allow(dead_code)]
impl LicensesData {
    pub const fn new() -> Self {
        Self {
            requests: Vec::new(),
            sessions: Map::new(),
            licenses: Map::new(),
        }
    }

    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

#[allow(dead_code)]
impl LicensesData {
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

    pub fn issue_license(&mut self, license: License) {
        rusk_abi::debug!("License contract: issue_license");
        self.licenses.insert(license.user_pk, license);
    }

    pub fn get_license(&self, user_pk: UserPublicKey) -> Option<License> {
        rusk_abi::debug!("License contract: get_license");
        self.licenses.get(&user_pk).cloned()
    }

    pub fn use_license(&mut self) {}

    pub fn get_session(
        &self,
        nullifier: LicenseNullifier,
    ) -> Option<LicenseSession> {
        rusk_abi::debug!("License contract: get_session {:?}", nullifier);
        self.sessions.get(&nullifier).cloned()
    }
}
