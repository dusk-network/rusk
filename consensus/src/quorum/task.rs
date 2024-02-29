// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};

use crate::queue::Queue;
use crate::user::committee::CommitteeSet;
use crate::user::provisioners::Provisioners;
use node_data::ledger::{to_str, Block, Certificate};
use node_data::message::payload::{RatificationResult, Vote};
use node_data::message::{AsyncQueue, Message, Payload, Status};

use crate::quorum::verifiers;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, Instrument};

pub struct Quorum {
    pub inbound_queue: AsyncQueue<Message>,
    outbound_queue: AsyncQueue<Message>,

    future_msgs: Arc<Mutex<Queue<Message>>>,
}

impl Quorum {
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

    /// Spawn a task to process quorum messages for a specified round
    /// There could be only one instance of this task per a time.
    pub(crate) fn spawn<D: Database + 'static>(
        &self,
        ru: RoundUpdate,
        provisioners: Arc<Provisioners>,
        db: Arc<Mutex<D>>,
    ) -> JoinHandle<Result<Block, ConsensusError>> {
        let future_msgs = self.future_msgs.clone();
        let outbound = self.outbound_queue.clone();
        let inbound = self.inbound_queue.clone();

        tokio::spawn(async move {
            let round = ru.round;
            let pubkey = ru.pubkey_bls.to_bs58();
            // Run quorum life-cycle loop
            Executor::new(ru, &provisioners, inbound, outbound, db)
                .run(future_msgs)
                .instrument(tracing::info_span!("agr_task", round, pubkey))
                .await
        })
    }
}

/// Executor implements life-cycle loop of a single quorum instance. This
/// should be started with each new round and dropped on round termination.
struct Executor<'p, D: Database> {
    ru: RoundUpdate,

    inbound_queue: AsyncQueue<Message>,
    outbound_queue: AsyncQueue<Message>,

    committees_set: RwLock<CommitteeSet<'p>>,
    db: Arc<Mutex<D>>,
}

impl<'p, D: Database> Executor<'p, D> {
    fn new(
        ru: RoundUpdate,
        provisioners: &'p Provisioners,
        inbound_queue: AsyncQueue<Message>,
        outbound_queue: AsyncQueue<Message>,
        db: Arc<Mutex<D>>,
    ) -> Self {
        Self {
            inbound_queue,
            outbound_queue,
            ru,
            committees_set: RwLock::new(CommitteeSet::new(provisioners)),
            db,
        }
    }

    async fn run(
        &self,
        future_msgs: Arc<Mutex<Queue<Message>>>,
    ) -> Result<Block, ConsensusError> {
        // drain future messages for current round and step.
        if self.ru.round > 0 {
            future_msgs.lock().await.clear_round(self.ru.round - 1);
        }

        if let Some(messages) =
            future_msgs.lock().await.drain_events(self.ru.round, 0)
        {
            for msg in messages {
                self.collect_inbound_msg(msg).await;
            }
        }

        // msg_loop for quorum messages
        loop {
            // Process messages from outside world
            if let Ok(msg) = self.inbound_queue.recv().await {
                match msg.header.compare_round(self.ru.round) {
                    Status::Future => {
                        // Future quorum message.
                        // Keep it for processing when we reach this round.
                        future_msgs.lock().await.put_event(
                            msg.header.round,
                            0,
                            msg.clone(),
                        );

                        self.publish(msg.clone()).await;
                    }
                    Status::Present => {
                        if let Some(block) = self.collect_inbound_msg(msg).await
                        {
                            break Ok(block);
                        }
                    }
                    _ => {}
                };
            }
        }
    }

    async fn collect_inbound_msg(&self, msg: Message) -> Option<Block> {
        let hdr = &msg.header;
        let step = msg.get_step();
        debug!(
            event = "msg received",
            topic = ?msg.topic(),
            iteration = hdr.iteration,
            step,
        );

        self.collect_quorum(msg).await
    }

    async fn collect_quorum(&self, msg: Message) -> Option<Block> {
        if let Payload::Quorum(quorum) = &msg.payload {
            // Verify quorum
            verifiers::verify_quorum(
                quorum,
                &self.committees_set,
                self.ru.seed(),
            )
            .await
            .ok()?;

            debug!(
                event = "quorum_collected",
                result = ?quorum.cert.result,
                iter = quorum.header.iteration,
                round = quorum.header.round,
            );

            // Publish the quorum
            self.publish(msg.clone()).await;

            if let RatificationResult::Success(Vote::Valid(hash)) =
                &quorum.cert.result
            {
                // Create winning block
                debug!("generate block from quorum msg");
                return self.create_winning_block(hash, &quorum.cert).await;
            }
        }

        None
    }

    // Publishes a message
    async fn publish(&self, msg: Message) {
        let topic = msg.topic();
        self.outbound_queue.send(msg).await.unwrap_or_else(|err| {
            error!("unable to publish msg(topic:{topic:?}) {err:?}")
        });
    }

    async fn create_winning_block(
        &self,
        hash: &[u8; 32],
        cert: &Certificate,
    ) -> Option<Block> {
        // Retrieve winning block from local storage
        match self.db.lock().await.get_candidate_block_by_hash(hash).await {
            Ok(mut block) => {
                debug!(event = "winner block retrieved", hash = to_str(hash));
                block.set_certificate(*cert);

                Some(block)
            }

            Err(e) => {
                error!(
                    event = "block retrieval failed",
                    err = format!("{}", e),
                    hash = to_str(hash)
                );
                None
            }
        }
    }
}
