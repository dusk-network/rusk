// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use cargo_toml::{Dependency, Manifest};
use dusk_plonk::prelude::{Circuit, Compiler, PublicParameters};
use once_cell::sync::Lazy;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_profile::{Circuit as CircuitProfile, Theme};
use std::io::{self, ErrorKind};
use tracing::{info, warn};
use tracing_subscriber::prelude::*;

pub static PUB_PARAMS: Lazy<PublicParameters> = Lazy::new(|| {
    match rusk_profile::get_common_reference_string() {
        Ok(buff) if rusk_profile::verify_common_reference_string(&buff) => unsafe {
            PublicParameters::from_slice_unchecked(&buff[..])
        },

        _ => {
            warn!(
                "{}   CRS due to cache miss",
                Theme::default().warn("Building"),
            );
            let mut rng = StdRng::seed_from_u64(0xbeef);

            let pp = PublicParameters::setup(1 << 17, &mut rng)
                .expect("Cannot initialize Public Parameters");

            rusk_profile::set_common_reference_string(pp.to_raw_var_bytes())
                .expect("Unable to write the CRS");

            pp
        }
    }
});

pub fn store_circuit<C>(name: Option<String>) -> io::Result<()>
where
    C: Circuit,
{
    // This force init is needed to check CRS and create it (if not available)
    // See also: https://github.com/dusk-network/rusk/issues/767
    Lazy::force(&PUB_PARAMS);

    // enable tracing logs
    let fmt_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_target(false)
        .with_level(false)
        .compact();
    let _ = tracing_subscriber::registry().with(fmt_layer).try_init();

    // check if a circuit with the same name is stored already
    let stored_circuit = match name {
        Some(ref circuit_name) => {
            info!(
                "{} {} circuit description",
                Theme::default().action("Fetching"),
                circuit_name
            );
            let stored_circuit = CircuitProfile::from_name(circuit_name);
            stored_circuit.ok()
        }
        None => None,
    };

    // compress circuit and prepare for storage
    let compressed = Compiler::compress::<C>(&PUB_PARAMS).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("Plonk circuit couldn't be compressed: {e}"),
        )
    })?;
    let version = parse_plonk_version()?;
    let circuit = CircuitProfile::new(compressed, version, name)?;

    // compare stored circuit (if any) against to-store circuit
    if let Some(stored) = stored_circuit {
        if circuit.id() == stored.id() && circuit.circuit() == stored.circuit()
        {
            info!(
                "{}   {}.cd",
                Theme::default().info("Found"),
                circuit.id_str()
            );
            return Ok(());
        } else {
            info!(
                "{} outdated circuit description",
                Theme::default().warn("Detected"),
            );
            stored.clean()?;
        }
    }
    circuit.store()?;
    Ok(())
}

fn parse_plonk_version() -> io::Result<String> {
    let cargo_toml = include_bytes!("../Cargo.toml");
    let cargo_toml = Manifest::from_slice(cargo_toml).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("Couldn't read manifest: {e}"),
        )
    })?;

    let plonk_dep = &cargo_toml.dependencies["dusk-plonk"];
    let version = match plonk_dep {
        Dependency::Simple(v) => v.clone(),
        Dependency::Detailed(d) => {
            let v = &d.version;
            if v.is_none() {
                return Err(io::Error::new(
                    ErrorKind::NotFound,
                    "Plonk version not found",
                ));
            }
            // due to the above check we can safely unwrap
            v.clone().unwrap()
        }
        _ => {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "Couldn't find plonk version",
            ))
        }
    };
    Ok(version)
}
