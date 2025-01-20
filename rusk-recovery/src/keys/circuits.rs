// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, ErrorKind};

use dusk_core::transfer::phoenix::{TxCircuit, NOTES_TREE_DEPTH};
use dusk_plonk::prelude::Circuit;
use tracing::info;

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
    let version = env!("RUSK_KEY_PLONK_VERSION").into();
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
