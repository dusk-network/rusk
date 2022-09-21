// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{Block, ConsensusError, RoundUpdate};
use crate::phase::Phase;

use crate::agreement::step;
use crate::messages::Message;
use crate::queue::Queue;
use crate::selection;
use crate::user::provisioners::Provisioners;
use crate::{firststep, secondstep};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tracing::{error, info};

pub const CONSENSUS_MAX_STEP: u8 = 213;
pub const CONSENSUS_QUORUM_THRESHOLD: f64 = 0.67;

#[derive(Default)]
pub struct Context {}
impl Context {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Consensus {
    phases: [Phase; 3],

    /// inbound_msgs is a queue of messages that comes from outside world
    inbound_msgs: Receiver<Message>,

    /// future_msgs is a queue of messages read from inbound_msgs queue.
    /// These msgs are pending to be handled in a future round/step.
    future_msgs: Queue<Message>,

    /// outbound_msgs is a queue of messages, this consensus instance shares with the outside world.
    outbound_msgs: Sender<Message>,

    /// agreement_layer implements Agreement message handler within the context of a separate task execution.
    agreement_layer: step::Agreement,
}

impl Consensus {
    pub fn new(inbound_msgs: Receiver<Message>, outbound_msg: Sender<Message>) -> Self {
        Self {
            phases: [
                Phase::Selection(selection::step::Selection::new()),
                Phase::Reduction1(firststep::step::Reduction::new()),
                Phase::Reduction2(secondstep::step::Reduction::new()),
            ],
            future_msgs: Queue::<Message>::default(),
            inbound_msgs,
            outbound_msgs: outbound_msg,
            agreement_layer: step::Agreement::new(),
        }
    }

    // reset_state_machine ...
    pub fn reset_state_machine(&mut self) {
        // TODO:
    }

    /// Spin the consensus state machine. The consensus runs for the whole round
    /// until either a new round is produced or the node needs to re-sync. The
    /// Agreement loop (acting roundwise) runs concurrently with the generation-selection-reduction
    /// loop (acting step-wise).
    pub async fn spin(&mut self, ru: RoundUpdate, mut provisioners: Provisioners) {
        // Enable/Disable all members stakes depending on the current round.
        // If a stake is not eligible for this round, it's disabled.
        provisioners.update_eligibility_flag(ru.round);

        // Round context channel.
        let (_ctx_tx, mut ctx_rx) = oneshot::channel::<Context>();

        // Agreement loop
        // Executes agreement loop in a separate tokio::task to collect (aggr)Agreement messages.
        let aggr_handle = self.agreement_layer.spawn(ru, provisioners.clone());

        // Consensus loop
        // Initialize and run consensus loop concurrently with agreement loop.
        let mut step: u8 = 0;

        loop {
            // Perform a single iteration.
            // An iteration runs all registered phases in a row.
            let iter_result = self
                .run_iteration(&mut ctx_rx, ru, &mut step, &mut provisioners)
                .await;

            match iter_result {
                Ok(msg) => {
                    // Delegate (agreement) message result to agreement loop for further processing.
                    self.agreement_layer.send_msg(msg.clone()).await;
                }
                Err(err) => {
                    error!("loop terminated with err: {:?}", err);
                    aggr_handle.abort();
                    break;
                }
            };
        }

        let winning_block = aggr_handle.await.unwrap_or_else(|_| Ok(Block::default()));
        info!("Winning block: {:?}", winning_block);

        self.teardown(ru.round).await;
    }

    async fn run_iteration(
        &mut self,
        ctx: &mut oneshot::Receiver<Context>,
        ru: RoundUpdate,
        step: &mut u8,
        provisioners: &mut Provisioners,
    ) -> Result<Message, ConsensusError> {
        let mut msg = Message::empty();

        for phase in self.phases.iter_mut() {
            *step += 1;

            // Initialize new phase with message returned by previous phase.
            phase.initialize(&msg, ru.round, *step);

            // Execute a phase.
            // An error returned here terminates consensus round.
            // This normally happens if consensus channel is cancelled
            // by agreement loop on finding the winning block for this round.
            match phase
                .run(
                    provisioners,
                    &mut self.future_msgs,
                    ctx,
                    &mut self.inbound_msgs,
                    &mut self.outbound_msgs,
                    ru,
                    *step,
                )
                .await
            {
                Ok(next_msg) => msg = next_msg,
                Err(e) => {
                    error!("err={:#?}", e);
                    return Err(ConsensusError::NotReady);
                }
            };

            if *step >= CONSENSUS_MAX_STEP {
                return Err(ConsensusError::MaxStepReached);
            }
        }

        Ok(msg)
    }

    async fn teardown(&mut self, round: u64) {
        self.future_msgs.clear(round);
    }
}
