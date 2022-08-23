// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::Block;

#[derive(Default, Debug)]
pub struct MsgHeader {
    pub pubkey_bls: [u8; 32],
    pub round: u64,
    pub step: u8,
    pub block_hash: [u8; 32],
}

impl MsgHeader {
    pub fn compare(&self, _round: u64, _step: u8) -> bool {
        // TODO: implement header compare
        true
    }
}

pub trait Message {
    fn compare(&self, round: u64, step: u8) -> bool;
}

#[derive(Default, Debug)]
pub struct MsgReduction {
    pub header: MsgHeader,
    pub signed_hash: [u8; 32],
}

impl Message for MsgReduction {
    fn compare(&self, round: u64, step: u8) -> bool {
        self.header.compare(round, step)
    }
}

#[derive(Default, Debug)]
pub struct MsgNewBlock {
    pub header: MsgHeader,
    pub prev_hash: [u8; 32],
    pub candidate: Block,
    pub signed_hash: [u8; 32],
}

impl Message for MsgNewBlock {
    fn compare(&self, round: u64, step: u8) -> bool {
        self.header.compare(round, step)
    }
}
