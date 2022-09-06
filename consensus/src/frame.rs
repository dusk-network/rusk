// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// RoundUpdate carries the data about the new Round, such as the active
// Provisioners, the BidList, the Seed and the Hash.
#[derive(Default, Debug)]
pub struct StepVotes {}

#[derive(Default, Clone, Debug)]
pub struct NewBlock {}

#[allow(unused)]
#[derive(Debug)]
pub enum Frame2 {
    StepVotes(StepVotes),
    NewBlock(NewBlock),
    Empty,
}
