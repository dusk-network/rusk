// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk_vm::Schedule;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    ScheduleLoaderError(io::Error),
    ScheduleDeserializationError(toml::de::Error),
}

pub struct Loader {}

impl Loader {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Schedule, Error> {
        let schedule_string =
            fs::read_to_string(path).map_err(Error::ScheduleLoaderError)?;
        let schedule: Schedule = toml::from_str(&schedule_string)
            .map_err(Error::ScheduleDeserializationError)?;
        Ok(schedule)
    }
}
