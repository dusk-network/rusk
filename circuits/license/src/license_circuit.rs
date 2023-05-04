// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;

use zk_citadel::{gadget, license::License};

#[derive(Default, Debug)]
pub struct LicenseCircuit {
    license: License,
}

impl LicenseCircuit {
    pub fn new(license: License) -> Self {
        Self { license }
    }

    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }
}

#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.1.0")]
impl Circuit for LicenseCircuit {
    fn circuit<C>(&self, composer: &mut C) -> Result<(), Error>
    where
        C: Composer,
    {
        gadget::nullify_license(composer, &self.license)?;
        Ok(())
    }
}
