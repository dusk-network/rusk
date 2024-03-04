// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Theme;
use dusk_plonk::prelude::{Compiler, PublicParameters};
use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::{fs, io};

use rusk_profile::Circuit as CircuitProfile;

use tracing::{info, warn};

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

static PUB_PARAMS: Lazy<PublicParameters> = Lazy::new(|| {
    let theme = Theme::default();
    info!("{} CRS from cache", theme.action("Fetching"));
    match rusk_profile::get_common_reference_string() {
        Ok(buff) if rusk_profile::verify_common_reference_string(&buff) => unsafe {
            let pp = PublicParameters::from_slice_unchecked(&buff[..]);
            info!("{} CRS", theme.info("Loaded"));
            pp
        },

        _ => {
            warn!("{} new CRS due to cache miss", theme.warn("Building"));

            fs::remove_dir_all(
                rusk_profile::get_rusk_keys_dir()
                    .expect("Cannot find or create keys dir"),
            )
            .expect("Cannot remove key files");

            let mut rng = StdRng::seed_from_u64(0xbeef);

            let pp = PublicParameters::setup(1 << 17, &mut rng)
                .expect("Cannot initialize Public Parameters");

            rusk_profile::set_common_reference_string(pp.to_raw_var_bytes())
                .expect("Unable to write the CRS");

            info!("{} CRS", theme.info("Cached"));

            pp
        }
    }
});

fn check_circuits_cache(
    circuit_list: Vec<CircuitProfile>,
) -> Result<(), io::Error> {
    let theme = Theme::default();
    for circuit in circuit_list {
        info!(
            "{} {} verifier data from cache",
            theme.action("Fetching"),
            circuit.name()
        );
        match circuit.get_verifier() {
            Ok(_) => {
                info!("{}   {}.vd", theme.info("Found"), circuit.id_str());
            }

            _ => {
                warn!("{} due to cache miss", theme.warn("Compiling"),);

                let compressed = circuit.get_compressed();
                let (pk, vd) = Compiler::decompress(
                    &PUB_PARAMS,
                    TRANSCRIPT_LABEL,
                    compressed,
                )
                .map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Couldn't compile keys for {}: {}",
                            circuit.name(),
                            e
                        ),
                    )
                })?;
                circuit.add_keys(pk.to_bytes(), vd.to_bytes())?;
                info!("{}   {}.vd", theme.info("Cached"), circuit.id_str());
                info!("{}   {}.pk", theme.info("Cached"), circuit.id_str());
            }
        }
    }
    Ok(())
}

fn circuits_from_names(
    names: &[&str],
) -> Result<Vec<CircuitProfile>, io::Error> {
    let mut circuits = Vec::new();
    for name in names {
        let circuit = CircuitProfile::from_name(name)?;
        circuits.push(circuit);
    }
    Ok(circuits)
}

fn run_stored_circuits_checks(
    keep_circuits: bool,
    circuit_list: Vec<CircuitProfile>,
) -> Result<(), io::Error> {
    let theme = Theme::default();

    if !keep_circuits {
        warn!("{} for untracked circuits", theme.warn("Checking"),);
        rusk_profile::clean_outdated(&circuit_list)?;
    } else {
        info!("{} untracked circuits", theme.action("Keeping"),);
    }
    check_circuits_cache(circuit_list).map(|_| ())
}

pub fn exec(keep_circuits: bool) -> Result<(), Box<dyn std::error::Error>> {
    // This force init is needed to check CRS and create it (if not available)
    // See also: https://github.com/dusk-network/rusk/issues/767
    Lazy::force(&PUB_PARAMS);

    let circuits = circuits_from_names(&[
        "SendToContractTransparentCircuit",
        "SendToContractObfuscatedCircuit",
        "WithdrawFromTransparentCircuit",
        "WithdrawFromObfuscatedCircuit",
        "ExecuteCircuitOneTwo",
        "ExecuteCircuitTwoTwo",
        "ExecuteCircuitThreeTwo",
        "ExecuteCircuitFourTwo",
        "LicenseCircuit",
    ])?;

    run_stored_circuits_checks(keep_circuits, circuits)?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_crs() {
        Lazy::force(&PUB_PARAMS);
    }
}
