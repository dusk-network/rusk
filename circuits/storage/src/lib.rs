// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use cargo_toml::{Dependency, Manifest};
use dusk_plonk::prelude::Circuit;
use rusk_profile::{Circuit as CircuitProfile, Theme};
use std::io::{self, ErrorKind};
use tracing::info;
use tracing_subscriber::prelude::*;

pub fn store_circuit<C>(name: Option<String>) -> io::Result<()>
where
    C: Circuit,
{
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
    let compressed = C::compress().map_err(|e| {
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
