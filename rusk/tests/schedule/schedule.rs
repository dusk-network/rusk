// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::schedule::{self, Error::*};

#[test]
fn valid_schedule_file() {
    let schedule =
        schedule::Loader::load("tests/resources/schedule.toml").unwrap();
    assert_eq!(schedule.max_table_size, 16384)
}

#[test]
fn missing_schedule_file() {
    assert!(matches!(
        schedule::Loader::load("missing_schedule.toml"),
        Err(ScheduleLoaderError(_))
    ));
}

#[test]
fn invalid_schedule_file() {
    assert!(matches!(
        schedule::Loader::load("tests/resources/invalid_schedule.toml"),
        Err(ScheduleDeserializationError(_))
    ));
}
