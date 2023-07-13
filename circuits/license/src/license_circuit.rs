// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;

use zk_citadel::gadgets;
use zk_citadel::license::{CitadelProverParameters, SessionCookie};

pub const DEPTH: usize = 17; // depth of the n-ary Merkle tree
pub const ARITY: usize = 4; // arity of the Merkle tree

#[derive(Default, Debug)]
pub struct LicenseCircuit {
    lpp: CitadelProverParameters<DEPTH, ARITY>,
    sc: SessionCookie,
}

impl LicenseCircuit {
    pub fn new(
        lpp: &CitadelProverParameters<DEPTH, ARITY>,
        sc: &SessionCookie,
    ) -> Self {
        Self { lpp: *lpp, sc: *sc }
    }

    pub const fn circuit_id() -> &'static [u8; 32] {
        &Self::CIRCUIT_ID
    }
}

#[code_hasher::hash(name = "CIRCUIT_ID", version = "0.2.0")]
impl Circuit for LicenseCircuit {
    fn circuit<C>(&self, composer: &mut C) -> Result<(), Error>
    where
        C: Composer,
    {
        gadgets::use_license_citadel(composer, &self.lpp, &self.sc)?;
        Ok(())
    }
}
