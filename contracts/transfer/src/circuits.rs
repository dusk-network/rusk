// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

const VD_STCT: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/cfebfdcd309a070b44e1b407b7228ca9b900720e7cff283d653400357161899a.vd"
));
const VD_STCO: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/d7fbe016d385b7d3b44c510225388a0f2a9889d07294ba3e3f9c037801d3148e.vd"
));
const VD_WFCT: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/d0b52061b33cb2f2ef79448b53cd3d2dbca30819ca4a55e151c8af01e6c7efcd.vd"
));
const VD_WFCO: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/7824ae42a6208eb0eca9f7c5e7ca964efa04a500fc3275e1c89541a26876808a.vd"
));

const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/97f7335c7fc873e7d31238fb0d476d32175e16377d1b5c175c34154ffc14156f.vd"
));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/77b6fcc405a60d463456e4893eb8966635773c550a19166111d0975fee0dd571.vd"
));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/d9d4357e5fbe36a323fbd4758f49b6ff6e66c9f27536a485b1d471053f910fbe.vd"
));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(
    env!("RUSK_PROFILE_PATH"),
    "/.rusk/keys/0a08b8746ac2de5deafd476b9ad690586ac7586dc6ec62d474d17da674bb074a.vd"
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
