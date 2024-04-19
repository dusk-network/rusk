// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// Note: all ID environment variables are set in the contracts build script
const VD_STCT: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_SENDTOCONTRACTTRANSPARENTCIRCUIT"),
    ".vd"
));
const VD_WFCT: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_WITHDRAWFROMTRANSPARENTCIRCUIT"),
    ".vd"
));

const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_EXECUTECIRCUITONETWO"),
    ".vd"
));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_EXECUTECIRCUITTWOTWO"),
    ".vd"
));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_EXECUTECIRCUITTHREETWO"),
    ".vd"
));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_BUILT_KEYS_PATH"),
    "/",
    env!("ID_EXECUTECIRCUITFOURTWO"),
    ".vd"
));

/// Verifier data for the execute circuits.
pub const fn verifier_data_execute(inputs: usize) -> Option<&'static [u8]> {
    let vd = match inputs {
        1 => VD_EXEC_1_2,
        2 => VD_EXEC_2_2,
        3 => VD_EXEC_3_2,
        4 => VD_EXEC_4_2,
        _ => return None,
    };

    Some(vd)
}

/// Verifier data for the `STCT` circuit.
pub const fn verifier_data_stct() -> &'static [u8] {
    VD_STCT
}

/// Verifier data for the `WFCT` circuit.
pub const fn verifier_data_wfct() -> &'static [u8] {
    VD_WFCT
}
