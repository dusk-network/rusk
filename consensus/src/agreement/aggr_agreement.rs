// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregator::AggrSignature;
use crate::commons::{RoundUpdate, Topics};
use crate::messages::{payload, Message, Payload};
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use crate::util::cluster::Cluster;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

use super::{accumulator, verifiers};

pub(super) async fn verify(
    ru: &RoundUpdate,
    committees_set: Arc<Mutex<CommitteeSet>>,
    msg: &Message,
) -> Result<(), super::verifiers::Error> {
    if let Payload::AggrAgreement(p) = &msg.payload {
        // let hdr = &msg.header;

        debug!("collected aggr agreement");

        verifiers::verify_votes(
            &msg.header.block_hash,
            p.bitset,
            &p.aggr_signature,
            &committees_set,
            &sortition::Config::new(ru.seed, ru.round, msg.header.step, 64),
        )
        .await?;

        // Verify agreement TODO:: new_agreement
        let m = Message {
            header: msg.header.clone(),
            payload: Payload::Agreement(p.agreement.clone()),
            metadata: Default::default(),
        };

        verifiers::verify_agreement(m, committees_set.clone(), ru.seed).await?;

        debug!("valid aggr agreement");

        return Ok(());
    }

    Err(verifiers::Error::VerificationFailed(
        dusk_bls12_381_sign::Error::InvalidSignature,
    ))
}

/// Aggregates a list of agreement messages and creates a Message with AggrAgreement payload.
pub(super) async fn aggregate(
    ru: &RoundUpdate,
    committees_set: Arc<Mutex<CommitteeSet>>,
    agreements: &accumulator::Output,
) -> Message {
    let first_agreement = agreements
        .iter()
        .next()
        .expect("agreements to not be empty");

    let (aggr_signature, bitset) = {
        let voters = &mut Cluster::new();
        let mut aggr_sign = AggrSignature::default();

        agreements.iter().for_each(|m| {
            voters.add(&m.header.pubkey_bls);

            // Aggregate signatures
            aggr_sign
                .add(&m.payload.signature)
                .expect("invalid signature");
        });

        (
            aggr_sign
                .aggregated_bytes()
                .expect("empty aggregated bytes"),
            committees_set.lock().await.bits(
                voters,
                &sortition::Config::new(ru.seed, ru.round, first_agreement.header.step, 64),
            ),
        )
    };

    let mut header = first_agreement.header.clone();
    header.topic = Topics::AggrAgreement as u8;

    Message::new_aggr_agreement(
        header,
        payload::AggrAgreement {
            agreement: first_agreement.payload.clone(),
            aggr_signature,
            bitset,
        },
    )
}
