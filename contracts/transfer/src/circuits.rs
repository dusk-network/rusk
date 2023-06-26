// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_STCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9d267dfe1d1ede4f2ffa35c3609f8662cd84e4df1066b2185a0f3b5b17721c79.vd"));
const VD_STCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/c8c7d7fa2fe8eeabd5505056ae3c00b44c1aa13d9578eeff3a4fc7ddb3035da4.vd"));
const VD_WFCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/dcc4561c1bbd8a10cd14c9e826d51373567dd41bb2cfd498f92230abc602ed47.vd"));
const VD_WFCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/8f7301b53f3af3eb14563c7e474a539a6e12c1248e1e9bdb4b07eeb2ef1a8f2e.vd"));

const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/4d5e60c2cdb7b3f273649487ad277eb0e380e44dd2f2effb0d2dcb3c1ff615d4.vd"));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/77d27ac80d397cfec7d621e61af4fa4b7fb4b9e503fa347082c5e1e187e08d48.vd"));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/4fb4e239548c5bdf9f5c6125cd07da64ce70edb99e79478f13140b53f136c441.vd"));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/05fb339e4fb471c745c8f90181a349ccf9226d8ee719073d45986fadcde466b4.vd"));

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
