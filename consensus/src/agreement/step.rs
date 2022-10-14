// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::accumulator::Accumulator;
use crate::commons::{Block, ConsensusError, RoundUpdate};
use crate::messages::{Header, Message, Status};
use crate::queue::Queue;
use crate::user::committee::CommitteeSet;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::util::pending_queue::PendingQueue;
use crate::util::pubkey::PublicKey;

use crate::config;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info, trace, Instrument};

const COMMITTEE_SIZE: usize = 64;

pub struct Agreement {
    pub inbound_queue: PendingQueue,
    outbound_queue: PendingQueue,

    future_msgs: Arc<Mutex<Queue<Message>>>,
}

impl Agreement {
    pub fn new(inbound_queue: PendingQueue, outbound_queue: PendingQueue) -> Self {
        Self {
            inbound_queue,
            outbound_queue,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
        }
    }

    /// Spawn a task to process agreement messages for a specified round
    /// There could be only one instance of this task per a time.
    pub(crate) fn spawn(
        &mut self,
        ru: RoundUpdate,
        provisioners: Provisioners,
    ) -> JoinHandle<Result<Block, ConsensusError>> {
        let future_msgs = self.future_msgs.clone();
        let outbound = self.outbound_queue.clone();
        let inbound = self.inbound_queue.clone();

        tokio::spawn(async move {
            // Run agreement life-cycle loop
            Executor::new(ru, provisioners, inbound, outbound)
                .run(future_msgs)
                .instrument(tracing::info_span!(
                    "agr_task",
                    round = ru.round,
                    pubkey = ru.pubkey_bls.encode_short_hex(),
                ))
                .await
        })
    }
}

/// Executor implements life-cycle loop of a single agreement instance. This should be started with each new round and dropped on round termination.
struct Executor {
    ru: RoundUpdate,

    inbound_queue: PendingQueue,
    outbound_queue: PendingQueue,

    committees_set: Arc<Mutex<CommitteeSet>>,
}

impl Executor {
    fn new(
        ru: RoundUpdate,
        provisioners: Provisioners,
        inbound_queue: PendingQueue,
        outbound_queue: PendingQueue,
    ) -> Self {
        Self {
            inbound_queue,
            outbound_queue,
            ru,
            committees_set: Arc::new(Mutex::new(CommitteeSet::new(
                PublicKey::default(),
                provisioners,
            ))),
        }
    }

    async fn run(
        &mut self,
        future_msgs: Arc<Mutex<Queue<Message>>>,
    ) -> Result<Block, ConsensusError> {
        let (collected_votes_tx, mut collected_votes_rx) = mpsc::channel::<Message>(10);

        // Accumulator
        let mut acc = Accumulator::new(
            config::ACCUMULATOR_WORKERS_AMOUNT,
            collected_votes_tx,
            self.committees_set.clone(),
            self.ru,
        );

        // drain future messages for current round and step.
        if self.ru.round > 0 {
            future_msgs.lock().await.clear(self.ru.round - 1);
        }

        if let Ok(messages) = future_msgs.lock().await.get_events(self.ru.round, 0) {
            for msg in messages {
                self.collect_agreement(&mut acc, msg).await;
            }
        }

        // event_loop for agreements messages
        loop {
            select! {
                biased;
                 // Process the output message from the Accumulator
                 msg = collected_votes_rx.recv() => {
                    if let Some(block) = self.collect_votes(msg) {
                        // Winning block of this round found.
                        future_msgs.lock().await.clear(self.ru.round);
                        break Ok(block)
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
                            Status::Present => { self.collect_agreement(&mut acc, msg).await;}
                            _ => {}
                        };
                    }
                 }
            };
        }
    }

    async fn collect_agreement(&mut self, acc: &mut Accumulator, msg: Message) {
        let hdr = &msg.header;

        if !self.is_member(hdr).await {
            trace!(
                "message is from non-committee member {:?} {:?}",
                self.ru,
                *hdr
            );
            return;
        }

        // Publish the agreement
        self.outbound_queue
            .send(msg.clone())
            .await
            .unwrap_or_else(|err| error!("unable to publish a collected agreement msg {:?}", err));

        // Accumulate the agreement
        acc.process(msg.clone()).await;
    }

    fn collect_votes(&self, _msg: Option<Message>) -> Option<Block> {
        info!("consensus_achieved");

        // TODO: Generate winning block. This should be feasible once append-only db is enabled.
        // generate committee per round, step
        //  republish, generate_certificate, createWinningBlock

        Some(Block::default())
    }

    async fn is_member(&self, hdr: &Header) -> bool {
        self.committees_set.lock().await.is_member(
            hdr.pubkey_bls,
            sortition::Config::new(self.ru.seed, hdr.round, hdr.step, COMMITTEE_SIZE),
        )
    }
}
