// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use piecrust::VM;

use rusk::{Result, Rusk};
use rusk_recovery_tools::state::{self, Snapshot};

// Creates a Rusk initial state in the given directory
pub fn new_state<P: AsRef<Path>>(dir: P, snapshot: &Snapshot) -> Result<Rusk> {
    let dir = dir.as_ref();

    let mut vm = VM::new(dir).expect("Instantiating a VM should succeed");
    rusk_abi::register_host_queries(&mut vm);

    let session = vm.session();

    let commit_id_path = rusk_profile::to_rusk_state_id_path(dir);
    let commit_id = state::deploy(commit_id_path, snapshot, session)
        .expect("Deploying initial state should succeed");

    vm.persist()?;

    let rusk = Rusk::new(dir).expect("Instantiating rusk should succeed");

    assert_eq!(
        commit_id,
        rusk.state_root(),
        "The current commit should be the commit of the initial state"
    );
    assert_eq!(
        commit_id,
        rusk.base_root(),
        "The base commit should be the commit of the initial state"
    );

    Ok(rusk)
}
