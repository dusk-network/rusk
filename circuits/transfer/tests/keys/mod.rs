// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::Error;

use dusk_plonk::prelude::*;

pub fn circuit_keys<C: Circuit>(
    id: &[u8; 32],
) -> Result<(Prover<C>, Verifier<C>), Error>
where
    C: Circuit,
{
    let keys = rusk_profile::keys_for(id)?;
    let pk = keys.get_prover()?;
    let vd = keys.get_verifier()?;

    let prover = Prover::try_from_bytes(&pk)?;
    let verifier = Verifier::try_from_bytes(&vd)?;

    Ok((prover, verifier))
}
