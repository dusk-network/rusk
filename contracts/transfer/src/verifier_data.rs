// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// Note: all ID environment variables are set in the contracts build script
const TX_CIRCUIT_1_2_VERIFIER: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_TXCIRCUITONETWO"),
    ".vd"
));
const TX_CIRCUIT_2_2_VERIFIER: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_TXCIRCUITTWOTWO"),
    ".vd"
));
const TX_CIRCUIT_3_2_VERIFIER: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_TXCIRCUITTHREETWO"),
    ".vd"
));
const TX_CIRCUIT_4_2_VERIFIER: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_TXCIRCUITFOURTWO"),
    ".vd"
));

/// Verifier data for the phoenix-circuits.
pub const fn tx_circuit_verifier(inputs: usize) -> Option<&'static [u8]> {
    let vd = match inputs {
        1 => TX_CIRCUIT_1_2_VERIFIER,
        2 => TX_CIRCUIT_2_2_VERIFIER,
        3 => TX_CIRCUIT_3_2_VERIFIER,
        4 => TX_CIRCUIT_4_2_VERIFIER,
        _ => return None,
    };

    Some(vd)
}
