// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    License, LicenseNullifier, LicenseRequest, LicenseSession, SPPublicKey,
    UserPublicKey,
};
use alloc::collections::BTreeMap;

/// License contract.
#[derive(Debug, Clone)]
pub struct LicensesData {
    pub requests: BTreeMap<SPPublicKey, LicenseRequest>,
    pub sessions: BTreeMap<LicenseNullifier, LicenseSession>,
    pub licenses: BTreeMap<UserPublicKey, License>,
}

impl LicensesData {
    pub const fn new() -> Self {
        Self {
            requests: BTreeMap::new(),
            sessions: BTreeMap::new(),
            licenses: BTreeMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

impl LicensesData {
    pub fn request_license(&mut self, request: LicenseRequest) {
        rusk_abi::debug!("License contract: request_license");
        self.requests.insert(request.sp_public_key, request.clone());
    }

    pub fn get_license_request(
        &self,
        sp_public_key: SPPublicKey,
    ) -> Option<LicenseRequest> {
        rusk_abi::debug!("License contract: get_license_request");
        self.requests.get(&sp_public_key).cloned()
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
        rusk_abi::debug!("License contract: get_session");
        self.sessions.get(&nullifier).cloned()
    }
}
