// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::accumulator::Accumulator;
use crate::commons::{ConsensusError, Database, RoundUpdate};

use crate::queue::Queue;
use crate::user::committee::CommitteeSet;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use node_data::bls::PublicKey;
use node_data::ledger::{to_str, Block, Certificate};
use node_data::message::{AsyncQueue, Header, Message, Payload, Status};

use crate::agreement::aggr_agreement;
use crate::config;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, Instrument};

use super::accumulator;

const COMMITTEE_SIZE: usize = 64;

pub struct Agreement {
    pub inbound_queue: AsyncQueue<Message>,
    outbound_queue: AsyncQueue<Message>,

    future_msgs: Arc<Mutex<Queue<Message>>>,
}

impl Agreement {
    pub fn new(
        inbound_queue: AsyncQueue<Message>,
        outbound_queue: AsyncQueue<Message>,
    ) -> Self {
        Self {
            inbound_queue,
            outbound_queue,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
        }
    }

    /// Spawn a task to process agreement messages for a specified round
    /// There could be only one instance of this task per a time.
    pub(crate) fn spawn<D: Database + 'static>(
        &mut self,
        ru: RoundUpdate,
        provisioners: Provisioners,
        db: Arc<Mutex<D>>,
    ) -> JoinHandle<Result<Block, ConsensusError>> {
        let future_msgs = self.future_msgs.clone();
        let outbound = self.outbound_queue.clone();
        let inbound = self.inbound_queue.clone();

        tokio::spawn(async move {
            let round = ru.round;
            let pubkey = ru.pubkey_bls.to_bs58();
            // Run agreement life-cycle loop
            Executor::new(ru, provisioners, inbound, outbound, db)
                .run(future_msgs)
                .instrument(tracing::info_span!("agr_task", round, pubkey))
                .await
        })
    }
}

/// Executor implements life-cycle loop of a single agreement instance. This
/// should be started with each new round and dropped on round termination.
struct Executor<D: Database> {
    ru: RoundUpdate,

    inbound_queue: AsyncQueue<Message>,
    outbound_queue: AsyncQueue<Message>,

    committees_set: Arc<Mutex<CommitteeSet>>,
    db: Arc<Mutex<D>>,
}

impl<D: Database> Executor<D> {
    fn new(
        ru: RoundUpdate,
        provisioners: Provisioners,
        inbound_queue: AsyncQueue<Message>,
        outbound_queue: AsyncQueue<Message>,
        db: Arc<Mutex<D>>,
    ) -> Self {
        Self {
            inbound_queue,
            outbound_queue,
            ru,
            committees_set: Arc::new(Mutex::new(CommitteeSet::new(
                PublicKey::default(),
                provisioners,
            ))),
            db,
        }
    }

    async fn run(
        &mut self,
        future_msgs: Arc<Mutex<Queue<Message>>>,
    ) -> Result<Block, ConsensusError> {
        let (collected_votes_tx, mut collected_votes_rx) =
            mpsc::channel::<accumulator::Output>(10);

        // Accumulator
        let mut acc = Accumulator::new(config::ACCUMULATOR_QUEUE_CAP);

        acc.spawn_workers_pool(
            config::ACCUMULATOR_WORKERS_AMOUNT,
            collected_votes_tx,
            self.committees_set.clone(),
            self.ru.seed,
        );

        // drain future messages for current round and step.
        if self.ru.round > 0 {
            future_msgs.lock().await.clear_round(self.ru.round - 1);
        }

        if let Some(messages) =
            future_msgs.lock().await.drain_events(self.ru.round, 0)
        {
            for msg in messages {
                self.collect_inbound_msg(&mut acc, msg).await;
            }
        }

        // event_loop for agreements messages
        loop {
            select! {
                biased;
                 // Process the output message from the Accumulator
                 result = collected_votes_rx.recv() => {
                    if let Some(aggrements) = result {
                        if let Some(block) = self.collect_votes(aggrements).await {
                            // Winning block of this round found.
                            future_msgs.lock().await.clear_round(self.ru.round);
                            break Ok(block)
                        }
                    }
                 },
                // Process messages from outside world
                 msg = self.inbound_queue.recv() => {
                    if let Ok(msg) = msg {
                         match msg.header.compare_round(self.ru.round) {
                            Status::Future => {
                                // Future agreement message.
                                // Keep it for processing when we reach this round.
                                future_msgs
                                    .lock()
                                    .await
                                    .put_event(msg.header.round, 0, msg.clone());
                            }
                            Status::Present => { if let Some(block) = self.collect_inbound_msg(&mut acc, msg).await {break Ok(block)}}
                            _ => {}
                        };
                    }
                 }
            };
        }
    }

    async fn collect_inbound_msg(
        &mut self,
        acc: &mut Accumulator,
        msg: Message,
    ) -> Option<Block> {
        if !self.is_member(&msg.header).await {
            return None;
        }

        match msg.payload {
            Payload::AggrAgreement(_) => {
                // process aggregated agreement
                return self.collect_aggr_agreement(msg).await;
            }
            Payload::Agreement(_) => {
                // Accumulate the agreement
                self.collect_agreement(acc, msg).await;
            }
            _ => {}
        };

        None
    }

    async fn collect_agreement(&mut self, acc: &mut Accumulator, msg: Message) {
        // Publish the agreement
        self.outbound_queue
            .send(msg.clone())
            .await
            .unwrap_or_else(|err| {
                error!("unable to publish a collected agreement msg {:?}", err)
            });

        // Accumulate the agreement
        acc.process(msg.clone()).await;
    }

    /// Collects accumulator output (a list of agreements) and publishes
    /// AggrAgreement.
    ///
    /// Returns the winning block.
    async fn collect_votes(
        &mut self,
        agreements: accumulator::Output,
    ) -> Option<Block> {
        if config::ENABLE_AGGR_AGREEMENT {
            let msg = aggr_agreement::aggregate(
                &self.ru,
                self.committees_set.clone(),
                &agreements,
            )
            .await;

            tracing::debug!("broadcast aggr_agreement {:#?}", msg);
            // Broadcast AggrAgreement message
            self.publish(msg).await;
        }

        let (cert, hash) = agreements
            .into_iter()
            .next()
            .map(|a| (a.payload.generate_certificate(), a.header.block_hash))?;

        // Create winning block
        self.create_winning_block(&hash, &cert).await
    }

    async fn collect_aggr_agreement(&mut self, msg: Message) -> Option<Block> {
        if let Payload::AggrAgreement(aggr) = &msg.payload {
            // Perform verification of aggregated agreement message
            if let Err(e) = aggr_agreement::verify(
                aggr,
                &self.ru,
                self.committees_set.clone(),
                &msg.header,
            )
            .await
            {
                error!("failed to verify aggr agreement err: {}", e);
                return None;
            }

            // Re-publish the agreement message
            self.publish(msg.clone()).await;

            // Generate certificate from an agreement
            let cert = aggr.agreement.generate_certificate();

            return self
                .create_winning_block(&msg.header.block_hash, &cert)
                .await;
        }

        None
    }

    async fn is_member(&self, hdr: &Header) -> bool {
        self.committees_set.lock().await.is_member(
            &hdr.pubkey_bls,
            &sortition::Config::new(
                self.ru.seed,
                hdr.round,
                hdr.step,
                COMMITTEE_SIZE,
            ),
        )
    }

    // Publishes a message
    async fn publish(&mut self, msg: Message) {
        let topic = msg.header.topic;
        self.outbound_queue.send(msg).await.unwrap_or_else(|err| {
            error!("unable to publish msg(id:{}) {:?}", topic, err)
        });
    }

    async fn create_winning_block(
        &self,
        hash: &[u8; 32],
        cert: &Certificate,
    ) -> Option<Block> {
        debug!(event = "create winning block", hash = to_str(hash));

        // Retrieve winning block from local storage
        match self.db.lock().await.get_candidate_block_by_hash(hash).await {
            Ok(mut block) => {
                debug!("winning block retrieved");
                block.header.cert = cert.clone();
                Some(block)
            }

            Err(e) => {
                error!("failed to retrieve winning block err: {}", e);
                None
            }
        }
    }
}
