// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::verifiers;
use crate::commons::Hash;
use crate::messages;
use crate::messages::{payload, Message, Payload};
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use hex::ToHex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info, warn, Instrument};

#[derive(Debug, Clone, Eq)]
pub(super) struct AgreementMessage {
    pub(super) header: messages::Header,
    pub(super) payload: payload::Agreement,
}

impl std::hash::Hash for AgreementMessage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.payload.signature.hash(state)
    }
}

impl PartialEq for AgreementMessage {
    fn eq(&self, other: &Self) -> bool {
        self.payload.signature == other.payload.signature
    }
}

/// AgreementsPerStep is a mapping of StepNum to Set of Agreements,
/// where duplicated agreements per step are not allowed.
type AgreementsPerStep = HashMap<u8, (HashSet<AgreementMessage>, usize)>;

/// StorePerHash implements a mapping of a block hash to AgreementsPerStep,
/// where AgreementsPerStep is a mapping of StepNum to Set of Agreements.
type StorePerHash = HashMap<Hash, AgreementsPerStep>;

/// Output from accumulation
pub(super) type Output = HashSet<AgreementMessage>;

pub(super) struct Accumulator {
    workers: Vec<JoinHandle<()>>,
    tx: async_channel::Sender<Message>,
    rx: async_channel::Receiver<Message>,
}

impl Accumulator {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = async_channel::bounded(cap);

        Self {
            workers: vec![],
            tx,
            rx,
        }
    }

    /// Spawns a set of tokio tasks that process agreement verifications concurrently.
    ///
    /// # Arguments
    ///
    /// * `workers_amount` - Number of workers to spawn. Must be > 0
    ///
    /// * `output_chan` - If successful, the final result of workers pool is written into output_chan
    pub fn spawn_workers_pool(
        &mut self,
        workers_amount: usize,
        output_chan: Sender<Output>,
        committees_set: Arc<Mutex<CommitteeSet>>,
        seed: [u8; 32],
    ) {
        assert!(workers_amount > 0);

        let stores = Arc::new(Mutex::new(StorePerHash::default()));

        // Spawn a set of workers to process all agreement message
        // verifications and accumulate results.
        // Final result is written to output_chan.
        for _i in 0..workers_amount {
            let rx = self.rx.clone();
            let committees_set = committees_set.clone();
            let output_chan = output_chan.clone();
            let stores = stores.clone();

            self.workers.push(tokio::spawn(
                async move {
                    // Process each request for verification
                    while let Ok(msg) = rx.recv().await {
                        if rx.is_closed() {
                            break;
                        }

                        if msg.header.block_hash == [0; 32] {
                            // discard empty block hash
                            continue;
                        }

                        if let Err(e) = verifiers::verify_agreement(
                            msg.clone(),
                            committees_set.clone(),
                            seed,
                        )
                        .await
                        {
                            error!("{:#?}", e);
                            continue;
                        }

                        if let Some(msg) = Self::accumulate(
                            stores.clone(),
                            committees_set.clone(),
                            msg,
                            seed,
                        )
                        .await
                        {
                            rx.close();
                            output_chan.send(msg).await.unwrap_or_else(|err| {
                                warn!(
                                    "unable to send_msg collected_votes {:?}",
                                    err
                                )
                            });
                            break;
                        }
                    }
                }
                .instrument(tracing::info_span!("acc_task",)),
            ));
        }
    }

    /// Queues the message for processing by the workers.
    ///
    /// # Panics
    ///
    /// If workers pool is not spawned, this will panic.
    pub async fn process(&mut self, msg: Message) {
        assert!(!self.workers.is_empty());

        self.tx.send(msg).await.unwrap_or_else(|err| {
            error!("unable to queue agreement_msg {:?}", err)
        });
    }

    /// Accumulates a verified agreement messages in a shared set of stores.
    ///
    /// Returns CollectedVotes Message if quorum is reached.
    async fn accumulate(
        stores: Arc<Mutex<StorePerHash>>,
        committees_set: Arc<Mutex<CommitteeSet>>,
        msg: messages::Message,
        seed: [u8; 32],
    ) -> Option<Output> {
        let hdr = msg.header.clone();

        let cfg = sortition::Config::new(seed, hdr.round, hdr.step, 64);

        // Mutex guard used here to fetch all data needed from CommitteeSet
        let (weight, target_quorum) = {
            let mut guard = committees_set.lock().await;

            let weight = guard.votes_for(&hdr.pubkey_bls, &cfg)?;
            if weight == 0 {
                warn!("Agreement was not accumulated since it is not from a committee member");
                return None;
            }

            Some((weight, guard.quorum(&cfg)))
        }?;

        if let Payload::Agreement(payload) = msg.payload {
            let mut guard = stores.lock().await;

            let (agr_set, agr_weight) = guard
                .entry(hdr.block_hash)
                .or_insert_with(AgreementsPerStep::default)
                .entry(hdr.step)
                .or_insert((HashSet::new(), 0));

            let key = AgreementMessage {
                header: msg.header,
                payload,
            };

            if agr_set.contains(&key) {
                warn!("Agreement was not accumulated since it is a duplicate");
                return None;
            }

            // Save agreement to avoid duplicates
            agr_set.insert(key);

            // Increase the cumulative weight
            *agr_weight += weight;

            if *agr_weight >= target_quorum {
                info!(
                    event = "quorum reached",
                    hash = hdr.block_hash.encode_hex::<String>(),
                    msg_round = hdr.round,
                    msg_step = hdr.step,
                    target = target_quorum,
                    aggr_count = agr_weight
                );

                return Some(agr_set.clone());
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

        self.rx.close();
        self.tx.close();
    }
}
