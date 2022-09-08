// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{Block, Header, RoundUpdate};
use crate::phase::Phase;

use crate::selection;
use crate::{firststep, secondstep};

use crate::messages::Message;
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

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
        }
    }

    // reset_state_machine ...
    pub fn reset_state_machine(&mut self) {
        // TODO:
    }

    // Spin the consensus state machine. The consensus runs for the whole round
    // until either a new round is produced or the node needs to re-sync. The
    // Agreement loop (acting roundwise) runs concurrently with the generation-selection-reduction
    // loop (acting step-wise).
    pub async fn spin(&mut self, ru: RoundUpdate, mut provisioners: Provisioners) {
        // Enable/Disable all members stakes depending on the current round.
        // If a stake is not eligible for this round, it's disabled.
        provisioners.update_eligibility_flag(ru.round);

        // Round context channel.
        let (round_ctx_tx, mut round_ctx_rx) = oneshot::channel::<Context>();

        // Agreement loop
        // Executes agreement loop in a separate tokio::task to collect (aggr)Agreement messages.
        let aggr_handle = self.spawn_agreement_loop(round_ctx_tx, ru);

        // Consensus loop
        // Initialize and run consensus loop concurrently with agreement loop.
        let mut step: u8 = 0;
        let mut msg = Message::empty();

        'exit: loop {
            // Perform a single iteration.
            // An iteration runs all registered phases in a row.
            for phase in self.phases.iter_mut() {
                step += 1;
                if step >= CONSENSUS_MAX_STEP {
                    error!("max steps reached");
                    aggr_handle.abort();
                    break 'exit;
                }

                // Initialize new phase with message returned by previous phase.
                phase.initialize(&msg, ru.round, step);

                // Execute a phase.
                // An error returned here terminates consensus round.
                // This normally happens if consensus channel is cancelled
                // by agreement loop on finding the winning block for this round.
                if let Ok(next_msg) = phase
                    .run(
                        &mut provisioners,
                        &mut self.future_msgs,
                        &mut round_ctx_rx,
                        &mut self.inbound_msgs,
                        &mut self.outbound_msgs,
                        ru,
                        step,
                    )
                    .await
                {
                    msg = next_msg;
                } else {
                    break 'exit;
                }
            }
        }

        let winning_block = aggr_handle.await.unwrap();
        info!("Winning block: {}", winning_block);

        self.teardown(ru.round).await;
    }

    // TODO: Implement agreement loop.
    pub fn spawn_agreement_loop(
        &mut self,
        round_ctx_sender: oneshot::Sender<Context>,
        ru: RoundUpdate,
    ) -> JoinHandle<Block> {
        tokio::spawn(async move {
            let mut counter: u8 = 0;
            loop {
                counter += 1;
                trace!("run agreement loop at round:{} ", ru.round);

                //TODO: Remove the delay
                // Simulate time spent on agreements collecting
                sleep(Duration::from_millis(1000));

                if counter == 3 * 5 + 2 {
                    let _ = round_ctx_sender.send(Context::new());
                    break;
                }
            }

            // Return winning block to the parent loop
            Block {
                header: Header {
                    height: ru.round,
                    ..Default::default()
                },
                txs: vec![],
            }
        })
    }

    async fn teardown(&mut self, round: u64) {
        let _ = self.future_msgs.clear(round).await;
    }
}
