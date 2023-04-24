// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::{vec, vec::Vec};
use crate::{LicenseNullifier, LicenseRequest, LicenseSession};

/// License contract.
#[derive(Debug, Clone)]
pub struct License {
    pub requests: Vec<LicenseRequest>,
    pub sessions: Vec<LicenseSession>,
}

impl License {
    pub const fn new() -> Self {
        Self {
            requests: vec![],
            sessions: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn identifier() -> &'static [u8; 7] {
        b"license"
    }
}

impl License {
    pub fn request_license(&mut self) {
        self.requests.push(LicenseRequest {})
    }

    pub fn get_license_request(&self) {}

    pub fn issue_license(&mut self) {}

    pub fn get_license(&self) {}

    pub fn use_license(&mut self) {}

    pub fn get_session(&self, nullifier: LicenseNullifier) -> Option<LicenseSession> {
        rusk_abi::debug!("License contract: get_session");
        self.sessions
            .iter()
            .find(|&session| session.nullifier == nullifier)
            .cloned()
    }
}
