// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod common;
pub mod services;

use futures::executor::block_on;
#[tokio::test(threaded_scheduler)]
async fn rusk_integration_tests() {
    let channel = block_on(common::setup()).expect("Error on the test setup");
    // Blindbid walkthrough tests
    //blindbid_service::walkthrough_works(channel)?;
    // Pki walkthrough tests
    assert!(services::pki_service::pki_walkthrough_uds(channel.clone())
        .await
        .is_ok());
    // Echo ping test
    assert!(services::echo_service::echo_works_uds(channel)
        .await
        .is_ok());
}
