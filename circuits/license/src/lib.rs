// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::{Circuit, Composer, Error as PlonkError};

use zk_citadel::gadgets;
use zk_citadel::license::{CitadelProverParameters, SessionCookie};

mod error;
pub use error::Error;

pub const DEPTH: usize = 17; // depth of the n-ary Merkle tree
pub const ARITY: usize = 4; // arity of the Merkle tree

#[derive(Default, Debug)]
pub struct LicenseCircuit {
    lpp: CitadelProverParameters<DEPTH, ARITY>,
    sc: SessionCookie,
}

impl LicenseCircuit {
    pub fn new(
        lpp: CitadelProverParameters<DEPTH, ARITY>,
        sc: SessionCookie,
    ) -> Self {
        Self { lpp, sc }
    }
}

impl Circuit for LicenseCircuit {
    fn circuit(&self, composer: &mut Composer) -> Result<(), PlonkError> {
        gadgets::use_license_citadel(composer, &self.lpp, &self.sc)?;
        Ok(())
    }
}
