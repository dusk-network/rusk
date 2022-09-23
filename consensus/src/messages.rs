// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::util::pubkey::PublicKey;
pub enum Status {
    Past,
    Present,
    Future,
}

pub trait MessageTrait {
    fn compare(&self, round: u64, step: u8) -> Status;
    fn get_pubkey_bls(&self) -> PublicKey;
    fn get_block_hash(&self) -> [u8; 32];
}

/// Message is a data unit that consensus phase can process.
#[derive(Debug, Default, Clone)]
pub struct Message {
    pub header: Header,
    pub payload: Payload,
}

impl MessageTrait for Message {
    fn compare(&self, round: u64, step: u8) -> Status {
        self.header.compare(round, step)
    }
    fn get_pubkey_bls(&self) -> PublicKey {
        self.header.pubkey_bls
    }
    fn get_block_hash(&self) -> [u8; 32] {
        self.header.block_hash
    }
}

impl Message {
    pub fn from_newblock(header: Header, p: payload::NewBlock) -> Message {
        Self {
            header,
            payload: Payload::NewBlock(Box::new(p)),
        }
    }

    pub fn from_stepvotes(p: payload::StepVotesWithCandidate) -> Message {
        Self {
            header: Header::default(),
            payload: Payload::StepVotesWithCandidate(p),
        }
    }

    pub fn new_reduction(header: Header, payload: payload::Reduction) -> Message {
        Self {
            header,
            payload: Payload::Reduction(payload),
        }
    }

    pub fn new_agreement(header: Header, payload: payload::Agreement) -> Message {
        Self {
            header,
            payload: Payload::Agreement(payload),
        }
    }

    pub fn empty() -> Message {
        Self {
            header: Header::default(),
            payload: Payload::Empty,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Header {
    pub pubkey_bls: PublicKey,
    pub round: u64,
    pub step: u8,
    pub block_hash: [u8; 32],
}

impl Header {
    pub fn compare(&self, round: u64, step: u8) -> Status {
        if self.round == round {
            if self.step == step {
                return Status::Present;
            }

            if self.step > step {
                return Status::Past;
            }

            if self.step < step {
                return Status::Future;
            }
        }

        if self.round > round {
            return Status::Past;
        }

        if self.round < round {
            return Status::Future;
        }

        Status::Past
    }
}

#[derive(Debug, Clone)]
pub enum Payload {
    Reduction(payload::Reduction),
    NewBlock(Box<payload::NewBlock>),
    StepVotes(payload::StepVotes),
    StepVotesWithCandidate(payload::StepVotesWithCandidate),
    Agreement(payload::Agreement),
    Empty,
}

impl Default for Payload {
    fn default() -> Self {
        Payload::Empty
    }
}

pub mod payload {
    use crate::commons::Block;
    use crate::commons::Signature;

    #[derive(Debug, Copy, Clone)]
    pub struct Reduction {
        pub signed_hash: [u8; 48],
    }

    impl Default for Reduction {
        fn default() -> Self {
            Self {
                signed_hash: [0; 48],
            }
        }
    }

    #[derive(Default, Debug, Clone)]
    pub struct NewBlock {
        pub prev_hash: [u8; 32],
        pub candidate: Block,
        pub signed_hash: [u8; 32],
    }

    #[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
    pub struct StepVotes {
        pub bitset: u64,
        pub signature: [u8; 48],
    }

    #[derive(Debug, Clone)]
    pub struct StepVotesWithCandidate {
        pub sv: StepVotes,
        pub candidate: Block,
    }

    #[derive(Debug, Clone, Eq, Hash, PartialEq)]
    pub struct Agreement {
        pub signature: [u8; 48],

        /// StepVotes of both 1th and 2nd Reduction steps
        pub votes_per_step: (StepVotes, StepVotes),
    }
}
