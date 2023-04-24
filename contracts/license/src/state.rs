// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{LicenseNullifier, LicenseRequest, LicenseSession, SPPublicKey};
use alloc::{collections::BTreeMap, vec, vec::Vec};

/// License contract.
#[derive(Debug, Clone)]
pub struct License {
    pub requests: BTreeMap<SPPublicKey, LicenseRequest>,
    pub sessions: Vec<LicenseSession>, /* todo: possibly use map keyed with
                                        * nullifier */
}

impl License {
    pub const fn new() -> Self {
        Self {
            requests: BTreeMap::new(),
            sessions: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

impl License {
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

    pub fn issue_license(&mut self) {}

    pub fn get_license(&self) {}

    pub fn use_license(&mut self) {}

    pub fn get_session(
        &self,
        nullifier: LicenseNullifier,
    ) -> Option<LicenseSession> {
        rusk_abi::debug!("License contract: get_session");
        self.sessions
            .iter()
            .find(|&session| session.nullifier == nullifier)
            .cloned()
    }
}
