// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::verifiers::verify_agreement;
use crate::commons::{Hash, RoundUpdate};
use crate::messages;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info, warn, Instrument};

/// AgreementsPerStep is a mapping of StepNum to Set of Agreements,
/// where duplicated agreements per step are not allowed.
type AgreementsPerStep = HashMap<u8, (HashSet<payload::Agreement>, usize)>;

/// StorePerHash implements a mapping of a block hash to AgreementsPerStep,
/// where AgreementsPerStep is a mapping of StepNum to Set of Agreements.
type StorePerHash = HashMap<Hash, AgreementsPerStep>;

pub(crate) struct Accumulator {
    workers: Vec<JoinHandle<()>>,
    inbound: Sender<Message>,
}

impl Accumulator {
    pub fn new(
        _workers_amount: usize,
        collected_votes_tx: Sender<Message>,
        committees_set: Arc<Mutex<CommitteeSet>>,
        ru: RoundUpdate,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<Message>(100);

        let mut a = Self {
            workers: vec![],
            inbound: tx,
        };

        // Spawn a single worker to process all agreement message verifications
        // It does also accumulate results and exists by providing a final CollectVotes message back to Agreement loop.
        let handle = tokio::spawn(
            async move {
                let mut stores = StorePerHash::default();
                // Process each request for verification
                while let Some(msg) = rx.recv().await {
                    if let Err(e) =
                        verify_agreement(msg.clone(), committees_set.clone(), ru.seed).await
                    {
                        error!("{:#?}", e);
                        continue;
                    }

                    if let Some(msg) =
                        Self::accumulate(&mut stores, committees_set.clone(), msg, ru.seed).await
                    {
                        collected_votes_tx.send(msg).await.unwrap_or_else(|err| {
                            error!("unable to send_msg collected_votes {:?}", err)
                        });
                        break;
                    }
                }
            }
            .instrument(tracing::info_span!("acc_task",)),
        );

        a.workers.push(handle);
        a
    }

    pub async fn process(&mut self, msg: Message) {
        self.inbound
            .send(msg)
            .await
            .unwrap_or_else(|err| error!("unable to queue agreement_msg {:?}", err));
    }

    async fn accumulate(
        stores: &mut StorePerHash,
        committees_set: Arc<Mutex<CommitteeSet>>,
        msg: messages::Message,
        seed: [u8; 32],
    ) -> Option<messages::Message> {
        let hdr = msg.header;

        let cfg = sortition::Config::new(seed, hdr.round, hdr.step, 64);

        // Mutex guard used here to fetch all data needed from CommitteeSet
        let (weight, target_quorum) = {
            let mut guard = committees_set.lock().await;

            let weight = guard.votes_for(hdr.pubkey_bls, cfg)?;
            if *weight == 0 {
                warn!("Agreement was not accumulated since it is not from a committee member");
                return None;
            }

            Some((*weight, guard.quorum(cfg)))
        }?;

        if let Payload::Agreement(payload) = msg.payload {
            let entry = stores
                .entry(hdr.block_hash)
                .or_insert_with(AgreementsPerStep::default)
                .entry(hdr.step)
                .or_insert((HashSet::new(), 0));

            if entry.0.contains(&payload) {
                warn!("Agreement was not accumulated since it is a duplicate");
                return None;
            }

            // Save agreement to avoid duplicates
            entry.0.insert(payload);

            // Increase the cumulative weight
            entry.1 += weight;

            if entry.1 >= target_quorum {
                info!(
                    "event=quorum reached, msg_round={}, msg_step={}, target={}, aggr_count={} ",
                    hdr.round, hdr.step, target_quorum, entry.1
                );

                // TODO: CollectedVotes Message
                return Some(Message::empty());
            }
        }

        None
    }
}

impl Drop for Accumulator {
    fn drop(&mut self) {
        // Abort all workers
        for handle in self.workers.iter() {
            handle.abort();
        }

        self.workers.clear();
    }
}
