// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use transfer_circuits::Error;

use dusk_plonk::prelude::*;

pub fn circuit_keys<C>(
) -> Result<(PublicParameters, ProverKey, VerifierData), Error>
where
    C: Circuit,
{
    let pp = rusk_profile::get_common_reference_string().map(|pp| unsafe {
        PublicParameters::from_slice_unchecked(pp.as_slice())
    })?;

    let keys = rusk_profile::keys_for(&C::CIRCUIT_ID)?;
    let pk = keys.get_prover()?;
    let vd = keys.get_verifier()?;

    let pk = ProverKey::from_slice(pk.as_slice())?;
    let vd = VerifierData::from_slice(vd.as_slice())?;

    Ok((pp, pk, vd))
}
