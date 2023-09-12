// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::mpsc;

use tempfile::tempdir;

use crate::common::logger;
use crate::common::state::new_state;

#[tokio::test(flavor = "multi_thread")]
async fn growth() {
    // Setup the logger
    logger();

    let tmp_dir = tempdir().expect("Creating temp dir should succeed");

    let snapshot = toml::from_str(include_str!("../config/growth.toml"))
        .expect("Cannot deserialize config");

    let rusk = new_state(&tmp_dir, &snapshot)
        .expect("Creating the state should succeed");

    let (sender, receiver) = mpsc::channel();

    rusk.leaves_from_height(0, sender)
        .expect("Querying leaves should succeed");

    let leaf_count = receiver.into_iter().count();

    assert_eq!(
        leaf_count, 1_000,
        "There should be a thousand notes in the state"
    );
}
