// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{Block, Header, RoundUpdate};
use crate::frame::Frame;
use crate::phase::Phase;

use crate::selection;
use crate::{firststep, secondstep};

use crate::messages::{MsgNewBlock, MsgReduction};
use crate::user::provisioners::Provisioners;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

pub const CONSENSUS_MAX_STEP: u8 = 213;

#[derive(Default)]
pub struct Context {}
impl Context {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Consensus {
    phases: [Phase; 3],
}

impl Consensus {
    pub fn new(
        new_block_rx: Receiver<MsgNewBlock>,
        first_red_rx: Receiver<MsgReduction>,
        sec_red_rx: Receiver<MsgReduction>,
    ) -> Self {
        let selection = Phase::Selection(selection::step::Selection::new(new_block_rx));
        trace!("phase memory size {}", std::mem::size_of_val(&selection));

        Self {
            phases: [
                selection,
                Phase::Reduction1(firststep::step::Reduction::new(first_red_rx)),
                Phase::Reduction2(secondstep::step::Reduction::new(sec_red_rx)),
            ],
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
        let mut frame = Frame::Empty;

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

                // Initialize new phase with frame created by previous phase.
                phase.initialize(&frame, ru.round, step);

                // Execute a phase.
                // An error returned here terminates consensus round.
                // This normally happens if consensus channel is cancelled
                // by agreement loop on finding the winning block for this round.
                match phase
                    .run(&mut provisioners, &mut round_ctx_rx, ru, step)
                    .await
                {
                    Ok(next_frame) => frame = next_frame,
                    Err(_) => {
                        break 'exit;
                    }
                }
            }
        }

        let winning_block = aggr_handle.await.unwrap();
        info!("Winning block: {}", winning_block);

        self.teardown();
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
            }
        })
    }

    pub fn teardown(&mut self) {}
}
