// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::messages;
use crate::messages::payload::StepVotes;
use crate::messages::{Message, Payload};
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::util::pubkey::PublicKey;

pub enum Error {
    VoteSetTooSmall,
    InvalidType,
}

pub fn verify_agreement(
    msg: Message,
    provisioners: Provisioners,
    seed: [u8; 32],
) -> Result<(), Error> {
    match msg.payload {
        Payload::Agreement(payload) => {
            // TODO: verifyWhole();

            // Verify 1th_reduction step_votes
            verify_step_votes(
                payload.votes_per_step.0,
                provisioners.clone(),
                seed,
                &msg.header,
                0,
            )?;

            // Verify 2th_reduction step_votes
            verify_step_votes(payload.votes_per_step.1, provisioners, seed, &msg.header, 1)?;

            // Verification done
            Ok(())
        }
        _ => Err(Error::InvalidType),
    }
}

fn verify_step_votes(
    sv: StepVotes,
    mut provisioners: Provisioners,
    seed: [u8; 32],
    hdr: &messages::Header,
    step_offset: u8,
) -> Result<(), Error> {
    let step = hdr.step - 1 + step_offset;
    let c = Committee::new(
        PublicKey::default(),
        &mut provisioners,
        sortition::Config::new(seed, hdr.round, step, 64),
    );

    let sub_committee = c.intersect(sv.bitset);

    if c.total_occurrences(&sub_committee) < c.quorum() {
        return Err(Error::VoteSetTooSmall);
    }

    // TODO: aggregate public keys

    // TODO: verify Signature

    Ok(())
}
