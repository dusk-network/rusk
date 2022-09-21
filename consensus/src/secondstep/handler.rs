use tracing::info;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{sign, verify_signature, ConsensusError, Hash, RoundUpdate};
use crate::event_loop::MsgHandler;

use crate::aggregator::Aggregator;
use crate::messages;
use crate::messages::payload::StepVotes;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::Committee;

pub struct Reduction {
    pub(crate) aggr: Aggregator,
    pub(crate) firstStepVotes: StepVotes,
}

impl MsgHandler<Message> for Reduction {
    // Collect the reduction message.
    fn handle_internal(
        &mut self,
        msg: Message,
        committee: &Committee,
        ru: RoundUpdate,
        step: u8,
    ) -> Result<Message, ConsensusError> {
        let msg_payload = match msg.payload {
            Payload::Reduction(p) => Ok(p),
            Payload::Empty => Ok(payload::Reduction::default()),
            _ => Err(ConsensusError::InvalidMsgType),
        }?;

        if let Err(_) = verify_signature(&msg.header, msg_payload.signed_hash) {
            return Err(ConsensusError::InvalidSignature);
        }

        //TODO:  Republish

        // Collect vote, if msg payload is of reduction type
        if let Some(sv) = self.aggr.collect_vote(committee, msg.header, msg_payload) {
            // At that point, we have reached a quorum for 2th_reduction on an empty on non-empty block.
            // Return an empty message as this iteration terminates here.
            info!("reached quorum at 2nd reduction step: {}", step);
            return Ok(self.build_agreement_msg(ru, step, sv));
        }

        Err(ConsensusError::NotReady)
    }
}

impl Reduction {
    fn build_agreement_msg(
        &self,
        ru: RoundUpdate,
        step: u8,
        sv: (Hash, payload::StepVotes),
    ) -> Message {
        let hdr = messages::Header {
            pubkey_bls: ru.pubkey_bls,
            round: ru.round,
            step: step,
            block_hash: sv.0.into(),
        };

        let payload = payload::Agreement {
            signature: sign(ru.secret_key, ru.pubkey_bls.to_bls_pk(), hdr),
            votes_per_step: (self.firstStepVotes, sv.1),
        };

        Message::new_agreement(hdr, payload)
    }
}
