// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::aggregator::AggrSignature;
use crate::commons::RoundUpdate;
use crate::user::cluster::Cluster;
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use node_data::ledger::to_str;
use node_data::message::payload::AggrAgreement;
use node_data::message::{payload, Header, Message, Topics};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

use super::{accumulator, verifiers};

pub(super) async fn verify(
    aggr: &AggrAgreement,
    ru: &RoundUpdate,
    committees_set: Arc<Mutex<CommitteeSet>>,
    hdr: &Header,
) -> Result<(), super::verifiers::Error> {
    debug!("collected aggr agreement");

    verifiers::verify_votes(
        &hdr.block_hash,
        aggr.bitset,
        &aggr.aggregate_signature,
        &committees_set,
        &sortition::Config::new(ru.seed, ru.round, hdr.step, 64),
    )
    .await?;

    // Verify agreement
    verifiers::verify_agreement(
        Message::new_agreement(hdr.clone(), aggr.agreement.clone()),
        committees_set.clone(),
        ru.seed,
    )
    .await?;

    debug!(
        event = "aggr_agreement recv",
        signature = to_str(&aggr.aggregate_signature)
    );
    Ok(())
}

/// Aggregates a list of agreement messages and creates a Message with
/// AggrAgreement payload.
pub(super) async fn aggregate(
    ru: &RoundUpdate,
    committees_set: Arc<Mutex<CommitteeSet>>,
    agreements: &accumulator::Output,
) -> Message {
    let first_agreement = agreements
        .iter()
        .next()
        .expect("agreements to not be empty");

    let (aggregate_signature, bitset) = {
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
                &sortition::Config::new(
                    ru.seed,
                    ru.round,
                    first_agreement.header.step,
                    64,
                ),
            ),
        )
    };

    let mut header = first_agreement.header.clone();
    header.topic = Topics::AggrAgreement as u8;

    Message::new_aggr_agreement(
        header,
        payload::AggrAgreement {
            agreement: first_agreement.payload.clone(),
            aggregate_signature,
            bitset,
        },
    )
}
