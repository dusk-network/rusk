// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod transfer;
use transfer::*;

use crate::theme::Theme;
use dusk_plonk::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use once_cell::sync::Lazy;

use tracing::{info, warn};

pub static PUB_PARAMS: Lazy<PublicParameters> = Lazy::new(|| {
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

pub trait CircuitLoader {
    fn circuit_id(&self) -> &[u8; 32];

    fn circuit_name(&self) -> &'static str;

    fn compile_circuit(
        &self,
    ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>>;
}

fn clear_outdated_keys(
    loader_list: &[&dyn CircuitLoader],
) -> Result<(), Box<dyn std::error::Error>> {
    let id_list: Vec<_> = loader_list
        .iter()
        .map(|loader| loader.circuit_id())
        .cloned()
        .collect();

    Ok(rusk_profile::clean_outdated_keys(&id_list)?)
}

fn check_keys_cache(
    loader_list: &[&dyn CircuitLoader],
) -> Result<Vec<()>, Box<dyn std::error::Error>> {
    let theme = Theme::default();
    loader_list
        .iter()
        .map(|loader| {
            info!(
                "{} {} key from cache",
                theme.action("Fetching"),
                loader.circuit_name()
            );

            let keys = rusk_profile::keys_for(loader.circuit_id())?;
            match keys.get_verifier() {
                Ok(_) => {
                    info!(
                        "{}   {}",
                        theme.info("Loaded"),
                        hex::encode(loader.circuit_id())
                    );
                    Ok(())
                }
                _ => {
                    warn!("{} due to cache miss", theme.warn("Compiling"),);

                    let (pk, vd) = loader.compile_circuit()?;
                    rusk_profile::add_keys_for(loader.circuit_id(), pk, vd)?;
                    info!(
                        "{}   {}",
                        theme.info("Cached"),
                        hex::encode(loader.circuit_id())
                    );
                    Ok(())
                }
            }
        })
        .collect::<Result<Vec<()>, Box<dyn std::error::Error>>>()
}

pub fn run_circuit_keys_checks(
    keep_keys: bool,
    loader_list: Vec<&dyn CircuitLoader>,
) -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::default();

    if !keep_keys {
        info!("{} untracked keys", theme.action("Cleaning"),);
        clear_outdated_keys(&loader_list)?;
    } else {
        info!("{} untracked keys", theme.action("Keeping"),);
    }
    check_keys_cache(&loader_list).map(|_| ())
}

pub fn exec(keep_keys: bool) -> Result<(), Box<dyn std::error::Error>> {
    Lazy::force(&PUB_PARAMS);

    run_circuit_keys_checks(
        keep_keys,
        vec![
            &StctCircuitLoader {},
            &StcoCircuitLoader {},
            &WftCircuitLoader {},
            &WfoCircuitLoader {},
            &ExecuteOneTwoCircuitLoader {},
            &ExecuteTwoTwoCircuitLoader {},
            &ExecuteThreeTwoCircuitLoader {},
            &ExecuteFourTwoCircuitLoader {},
        ],
    )?;

    Ok(())
}
