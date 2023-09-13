// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_STCT: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/1e826837c2de377128fc73ebfac77d84f3a334fe8310ff5b316d8f55e2ff3661.vd"
));
const VD_STCO: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/8878bbe32b52953022d1b4895d77d325429715c9a69f90f46d80b543c4348728.vd"
));
const VD_WFCT: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/a56c87dcd43402dcae9aa719d378dc91b8e93c5fbe2cfbda099d0eeb75b5c628.vd"
));
const VD_WFCO: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/01f4bc9da62145d1e28ac7947cd6428fc4127a046e449a6583b56443d180a689.vd"
));

const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/90ed94f311a94d6401df61f1a4e98328ed029f42340537bc1661d551eab3319e.vd"
));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/2dcb577684657c0b0e10d32938cca7396cfcb579ed044aa6c9d3bad31fe5c005.vd"
));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/cba6ad03bbe9f53bdeddef0256ffad331652b3380b7996cd670fd7d92670fd53.vd"
));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/728a4c412d7a5651f1c530a855f40f2ebed120446fb7a87fa17f82164b789a17.vd"
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

/// Verifier data for the `STCO` circuit.
pub const fn verifier_data_stco() -> &'static [u8] {
    VD_STCO
}

/// Verifier data for the `STCT` circuit.
pub const fn verifier_data_stct() -> &'static [u8] {
    VD_STCT
}

/// Verifier data for the `WFCT` circuit.
pub const fn verifier_data_wfct() -> &'static [u8] {
    VD_WFCT
}

/// Verifier data for the `WFCO` circuit.
pub const fn verifier_data_wfco() -> &'static [u8] {
    VD_WFCO
}
