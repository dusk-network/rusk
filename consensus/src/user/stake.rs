// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[derive(Clone, Default, Debug)]
pub struct Stake {
    // Value should be initialized only at constructor.
    // It's later used to restore intermediate_value on each new sortition
    // execution. In that way, we don't need to perform a deep copy of all
    // provisioners members and their stakes as it used to be.
    value: u64,

    pub reward: u64,
    pub counter: u64,
    pub eligible_since: u64,
}

impl Stake {
    pub fn new(value: u64, reward: u64, eligible_since: u64) -> Self {
        Self {
            value,
            reward,
            eligible_since,
            counter: 0,
        }
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
