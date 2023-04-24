// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

extern crate alloc;

#[allow(unused)]
#[path = "../src/license_types.rs"]
mod license_types;
use license_types::*;

use piecrust::{ModuleId, VM};

const LICENSE_CONTRACT_ID: ModuleId = {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xf8;
    ModuleId::from_bytes(bytes)
};

const POINT_LIMIT: u64 = 0x10000000;

#[test]
fn get_session() {
    let vm = VM::ephemeral().expect("Creating a VM should succeed");

    let bytecode = include_bytes!(
        "../../../target/wasm32-unknown-unknown/release/license.wasm"
    );

    let mut session = vm.genesis_session();

    session.set_point_limit(POINT_LIMIT);

    session
        .deploy_with_id(LICENSE_CONTRACT_ID, bytecode)
        .expect("Deploying the license contract should succeed");

    let nullifier = LicenseNullifier {};

    let license_session = session.query::<LicenseNullifier, Option<LicenseSession>>(LICENSE_CONTRACT_ID, "get_session", &nullifier).expect("Querying the session should succeed");

    assert_eq!(None::<LicenseSession>, license_session);
}
