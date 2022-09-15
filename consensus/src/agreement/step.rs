// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::accumulator::Accumulator;
use crate::commons::{Block, RoundUpdate};
use crate::messages::Message;
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use std::fmt::Error;

use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

const WORKERS_AMOUNT: usize = 4;

pub struct Agreement {
    inbound_msgs: Option<Sender<Message>>,
    future_msgs: Arc<Mutex<Queue<Message>>>,

    is_running: Arc<Mutex<bool>>,
}

impl Agreement {
    pub fn new() -> Self {
        Self {
            inbound_msgs: None,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
            is_running: Arc::new(Mutex::new(false)),
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
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            if *is_running.lock().await {
                warn!("another agreement task is still running");
            }

            (*is_running.lock().await) = true;

            // Run agreement life-cycle loop
            let res = Executor {
                ru,
                _provisioners: provisioners,
            }
            .run(agreement_rx, future_msgs)
            .await;

            (*is_running.lock().await) = false;
            res
        })
    }
}

/// Executor implements life-cycle loop of a single agreement instance. This should be started with each new round and dropped on round termination.
struct Executor {
    ru: RoundUpdate,
    _provisioners: Provisioners,
}

// Agreement non-pub methods
impl Executor {
    async fn run(
        &self,
        mut inbound_msg: Receiver<Message>,
        future_msgs: Arc<Mutex<Queue<Message>>>,
    ) -> Result<Block, Error> {
        // Accumulator
        let (collected_votes_tx, mut collected_votes_rx) = mpsc::channel::<Message>(10);
        // TODO verifyChan
        // TODO: Pass committeee
        let _acc = Accumulator::new(WORKERS_AMOUNT, collected_votes_tx);

        // drain future messages for current round and step.
        if let Ok(messages) = future_msgs.lock().await.get_events(self.ru.round, 0) {
            for msg in messages {
                self.collect_agreement(Some(msg));
            }
        }

        loop {
            select! {
                biased;
                // Process messages from outside world
                 msg = inbound_msg.recv() => {
                    // TODO: should process
                    future_msgs.lock().await.put_event(self.ru.round, 0, msg.as_ref().unwrap().clone());

                    // TODO: Add collect AggrAgreement message
                    self.collect_agreement(msg);
                 },
                 // Process an output message from the Accumulator
                 msg = collected_votes_rx.recv() => {
                    if let Some(block) = self.collect_votes(msg) {
                        // Winning block of this round found.
                        future_msgs.lock().await.clear(self.ru.round);
                        break Ok(block)
                    }
                 }
            };
        }
    }

    // TODO: committee
    fn collect_agreement(&self, _msg: Option<Message>) {
        //TODO: is_member

        // TODO: Impl Accumulator
        //TODO: self.accumulator.Process(aggr)
        // verifyChan.send(aggr)
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
