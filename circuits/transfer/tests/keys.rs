// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::Error;

use dusk_plonk::prelude::*;

pub fn load_keys(name: impl AsRef<str>) -> Result<(Prover, Verifier), Error> {
    let circuit_profile = rusk_profile::Circuit::from_name(name.as_ref())
        .expect(&format!(
            "the circuit data for {} should be stores",
            name.as_ref()
        ));

    let (pk, vd) = circuit_profile
        .get_keys()
        .expect("The keys for the LicenseCircuit should be stored");

    let prover = Prover::try_from_bytes(&pk)?;
    let verifier = Verifier::try_from_bytes(&vd)?;

    Ok((prover, verifier))
}
