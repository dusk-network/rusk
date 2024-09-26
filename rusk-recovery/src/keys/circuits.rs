// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, ErrorKind};

use cargo_toml::{Dependency, Manifest};
use tracing::info;

use dusk_plonk::prelude::Circuit;
use execution_core::transfer::phoenix::{TxCircuit, NOTES_TREE_DEPTH};

use license_circuits::LicenseCircuit;

type TxCircuitOneTwo = TxCircuit<NOTES_TREE_DEPTH, 1>;
type TxCircuitTwoTwo = TxCircuit<NOTES_TREE_DEPTH, 2>;
type TxCircuitThreeTwo = TxCircuit<NOTES_TREE_DEPTH, 3>;
type TxCircuitFourTwo = TxCircuit<NOTES_TREE_DEPTH, 4>;

use rusk_profile::{Circuit as CircuitProfile, Theme};

pub fn cache_all() -> io::Result<()> {
    // cache the circuit description, this only updates the circuit description
    // if the new circuit is different from a previously cached version
    cache::<TxCircuitOneTwo>(Some(String::from("TxCircuitOneTwo")))?;
    cache::<TxCircuitTwoTwo>(Some(String::from("TxCircuitTwoTwo")))?;
    cache::<TxCircuitThreeTwo>(Some(String::from("TxCircuitThreeTwo")))?;
    cache::<TxCircuitFourTwo>(Some(String::from("TxCircuitFourTwo")))?;
    cache::<LicenseCircuit>(Some(String::from("LicenseCircuit")))?;

    Ok(())
}

// Caches the compressed circuit description of the generic `Circuit`.
// If there is a circuit stored under the same name already, it is only
// overridden if the description changed or plonk had a major verision bump.
pub fn cache<C>(name: Option<String>) -> io::Result<()>
where
    C: Circuit,
{
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

// Returns that string that defines the plonk-version
fn parse_plonk_version() -> io::Result<String> {
    let cargo_toml = include_bytes!("../../../Cargo.toml");
    let cargo_toml = Manifest::from_slice(cargo_toml).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("Couldn't parse workspace manifest: {e}"),
        )
    })?;

    let plonk_dep = &cargo_toml
        .workspace
        .ok_or(io::Error::new(
            ErrorKind::InvalidInput,
            "Cargo.toml at crate root should define a workspace",
        ))?
        .dependencies["dusk-plonk"];
    let mut version = match plonk_dep {
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
    // sanitize plonk version
    if version.starts_with('=') {
        version.remove(0);
    }
    Ok(version)
}
