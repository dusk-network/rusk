// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::accumulator;
use crate::agreement::accumulator::Accumulator;
use crate::commons::{Block, RoundUpdate};
use crate::messages::Message;
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use std::fmt::Error;
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{error, info};

pub struct Agreement {
    inbound_msgs: Option<Sender<Message>>,
    future_msgs: Arc<Queue<Message>>,
}

impl Agreement {
    pub fn new() -> Self {
        Self {
            inbound_msgs: None,
            future_msgs: Arc::new(Queue::default()),
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

    // Add provisioners clone
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
            Agreement::run(agreement_rx, future_msgs, ru, provisioners).await
        })
    }
}

// Agreement non-pub methods
impl Agreement {
    async fn run(
        mut inbound_msg: Receiver<Message>,
        future_msgs: Arc<Queue<Message>>,
        ru: RoundUpdate,
        provisioners: Provisioners,
    ) -> Result<Block, Error> {
        // Accumulator
        let (collected_votes_tx, mut collected_votes_rx) = mpsc::channel::<Message>(10);
        // TODO verifyChan
        // TODO: Pass committeee
        let acc = Accumulator::new(4, collected_votes_tx);

        // drain future messages for current round and step.
        if let Ok(messages) = future_msgs.get_events(ru.round, 255).await {
            for msg in messages {
                Self::collect_agreement(Some(msg), &provisioners);
            }
        }

        loop {
            select! {
                biased;
                // Process messages from outside world
                 msg = inbound_msg.recv() => {
                    // TODO: collect AggrAgreement message
                    Self::collect_agreement(msg, &provisioners);
                 },
                 // Process an output message from the Accumulator
                 msg = collected_votes_rx.recv() => {
                    if let Some(block) = Self::collect_votes(msg, &provisioners) {
                        // Winning block of this round found.
                        break Ok(block)
                    }
                 }
            };
        }
    }

    // TODO: committee
    fn collect_agreement(_msg: Option<Message>, _provisioners: &Provisioners) {
        //TODO: is_member

        // TODO: Impl Accumulator
        //TODO: self.accumulator.Process(aggr)
        // verifyChan.send(aggr)
    }

    fn collect_votes(_msg: Option<Message>, _provisioners: &Provisioners) -> Option<Block> {
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
