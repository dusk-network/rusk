// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::accumulator::Accumulator;
use crate::commons::{Block, RoundUpdate};
use crate::messages::Message;
use crate::queue::Queue;
use crate::user::committee::CommitteeSet;
use crate::user::provisioners::Provisioners;
use crate::util::pubkey::PublicKey;
use std::fmt::Error;
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};

const WORKERS_AMOUNT: usize = 4;

pub struct Agreement {
    inbound_msgs: Option<Sender<Message>>,
    future_msgs: Arc<Mutex<Queue<Message>>>,
}

impl Agreement {
    pub fn new() -> Self {
        Self {
            inbound_msgs: None,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
        }
    }

    pub async fn send_msg(&mut self, msg: Message) {
        match self.inbound_msgs.as_ref() {
            Some(tx) => {
                tx.send(msg)
                    .await
                    .unwrap_or_else(|err| error!("unable to send_msg {:?}", err));
            }
            None => error!("no inbound message queue set"),
        };
    }

    /// Spawn a task to process agreement messages for a specified round
    /// There could be only one instance of this task per a time.
    pub(crate) fn spawn(
        &mut self,
        ru: RoundUpdate,
        provisioners: Provisioners,
    ) -> JoinHandle<Result<Block, Error>> {
        let (agreement_tx, agreement_rx) = mpsc::channel::<Message>(10);
        self.inbound_msgs = Some(agreement_tx);

        let future_msgs = self.future_msgs.clone();

        tokio::spawn(async move {
            // Run agreement life-cycle loop
            let res = Executor::new(ru, provisioners)
                .run(agreement_rx, future_msgs)
                .await;

            res
        })
    }
}

/// Executor implements life-cycle loop of a single agreement instance. This should be started with each new round and dropped on round termination.
struct Executor {
    ru: RoundUpdate,
    // TODO: Consider sharing CommitteesSet between main Consensus loop and agreement step.
    committees_set: Arc<Mutex<CommitteeSet>>,
}

// Agreement non-pub methods
impl Executor {
    fn new(ru: RoundUpdate, provisioners: Provisioners) -> Self {
        Self {
            ru,
            committees_set: Arc::new(Mutex::new(CommitteeSet::new(
                PublicKey::default(),
                provisioners,
            ))),
        }
    }

    async fn run(
        &mut self,
        mut inbound_msg: Receiver<Message>,
        future_msgs: Arc<Mutex<Queue<Message>>>,
    ) -> Result<Block, Error> {
        let (collected_votes_tx, mut collected_votes_rx) = mpsc::channel::<Message>(10);

        // Accumulator
        let mut acc = Accumulator::new(
            WORKERS_AMOUNT,
            collected_votes_tx,
            self.committees_set.clone(),
            self.ru,
        );

        // drain future messages for current round and step.
        if let Ok(messages) = future_msgs.lock().await.get_events(self.ru.round, 0) {
            for msg in messages {
                self.collect_agreement(&mut acc, Some(msg)).await;
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
                 msg = inbound_msg.recv() => {
                    // TODO: should process
                    future_msgs.lock().await.put_event(self.ru.round, 0, msg.as_ref().unwrap().clone());
                    self.collect_agreement(&mut acc, msg).await;
                 }
            };
        }
    }

    async fn collect_agreement(&mut self, acc: &mut Accumulator, msg: Option<Message>) {
        if msg.is_none() {
            error!("invalid message");
            return;
        }
        acc.process(msg.unwrap()).await;
    }

    fn collect_votes(&self, _msg: Option<Message>) -> Option<Block> {
        info!("consensus_achieved");

        // TODO: generate committee per round, step
        // comm := handler.Committee(r.Round, evs[0].State().Step)
        // 	pubs := new(sortedset.Set)

        // TODO: Republish

        // TODO: GenerateCertificate

        // TODO: createWinningBlock

        None
    }
}
