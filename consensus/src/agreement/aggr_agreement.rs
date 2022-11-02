// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregator::AggrSignature;
use crate::commons::RoundUpdate;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use crate::util::cluster::Cluster;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use super::{accumulator, verifiers};

pub async fn verify(
    ru: &RoundUpdate,
    committees_set: Arc<Mutex<CommitteeSet>>,
    msg: &Message,
) -> Result<(), super::verifiers::Error> {
    if let Payload::AggrAgreement(p) = &msg.payload {
        let hdr = &msg.header;

        info!("collected aggr agreement");

        verifiers::verify_votes(
            hdr.block_hash,
            p.bitset,
            p.aggr_signature,
            committees_set.clone(),
            sortition::Config::new(ru.seed, ru.round, hdr.step, 64),
        )
        .await?;

        // Verify agreement TODO:: new_agreement
        let m = Message {
            header: msg.header,
            payload: Payload::Agreement(p.agreement.clone()),
            metadata: Default::default(),
        };

        verifiers::verify_agreement(m, committees_set.clone(), ru.seed).await?;

        info!("verified aggr agreement");

        return Ok(());
    }

    Err(verifiers::Error::VerificationFailed)
}

/// Aggregates a list of agreement messages and creates a Message with AggrAgreement payload.
pub async fn aggregate(
    ru: &RoundUpdate,
    committees_set: Arc<Mutex<CommitteeSet>>,
    agreements: &accumulator::Output,
) -> Option<Message> {
    if agreements.is_empty() {
        return None;
    }

    let hdr = &agreements[0].header;

    let (aggr_signature, bitset) = {
        let voters = &mut Cluster::new();
        let mut aggr_sign = AggrSignature::default();

        agreements.iter().for_each(|msg| {
            if let Payload::Agreement(agr) = &msg.payload {
                voters.add(&msg.header.pubkey_bls);

                // Aggregate signatures
                _ = aggr_sign.add(agr.signature);
            }
        });

        (
            aggr_sign.aggregated_bytes()?,
            committees_set.lock().await.bits(
                voters,
                sortition::Config::new(ru.seed, ru.round, hdr.step, 64),
            ),
        )
    };

    if let Payload::Agreement(agr) = &agreements[0].payload {
        let payload = payload::AggrAgreement {
            agreement: agr.clone(),
            aggr_signature,
            bitset,
        };

        let mut hdr = agreements[0].header;
        hdr.topic = crate::commons::Topics::AggrAgreement as u8;

        return Some(Message::new_aggr_agreement(hdr, payload));
    }

    let x = true;
    debug_assert!(x, "accumulator returns non-agreement messages");

    None
}
