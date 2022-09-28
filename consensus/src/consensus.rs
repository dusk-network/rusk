use std::time::Duration;
// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{Block, ConsensusError, RoundUpdate};
use crate::phase::Phase;

use crate::agreement::step;
use crate::execution_ctx::ExecutionCtx;
use crate::messages::Message;
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use crate::util::pending_queue::PendingQueue;
use crate::{config, selection};
use crate::{firststep, secondstep};
use tracing::Instrument;

use tokio::sync::oneshot;
use tracing::trace;

pub struct Consensus {
    phases: [Phase; 3],

    /// inbound is a queue of messages that comes from outside world
    inbound: PendingQueue,
    /// outbound_msgs is a queue of messages, this consensus instance shares
    /// with the outside world.
    outbound: PendingQueue,

    /// future_msgs is a queue of messages read from inbound_msgs queue. These
    /// msgs are pending to be handled in a future round/step.
    future_msgs: Queue<Message>,

    /// agreement_layer implements Agreement message handler within the context
    /// of a separate task execution.
    agreement_process: step::Agreement,
}

impl Consensus {
    pub fn new(
        inbound: PendingQueue,
        outbound: PendingQueue,
        aggr_inbound_queue: PendingQueue,
        aggr_outbound_queue: PendingQueue,
    ) -> Self {
        Self {
            phases: [
                Phase::Selection(selection::step::Selection::new()),
                Phase::Reduction1(firststep::step::Reduction::new()),
                Phase::Reduction2(secondstep::step::Reduction::new()),
            ],
            inbound,
            outbound,
            future_msgs: Queue::<Message>::default(),
            agreement_process: step::Agreement::new(aggr_inbound_queue, aggr_outbound_queue),
        }
    }

    /// Spin the consensus state machine. The consensus runs for the whole round
    /// until either a new round is produced or the node needs to re-sync. The
    /// Agreement loop (acting roundwise) runs concurrently with the
    /// generation-selection-reduction loop (acting step-wise).
    pub async fn spin(&mut self, ru: RoundUpdate, mut provisioners: Provisioners) {
        // Enable/Disable all members stakes depending on the current round. If
        // a stake is not eligible for this round, it's disabled.
        provisioners.update_eligibility_flag(ru.round);

        // Round context channel.
        let (cancel_chan_tx, mut cancel_chan_rx) = oneshot::channel::<bool>();

        // Agreement loop Executes agreement loop in a separate tokio::task to
        // collect (aggr)Agreement messages.
        let aggr_handle = self
            .agreement_process
            .spawn(cancel_chan_tx, ru, provisioners.clone());

        // Consensus loop Initialize and run consensus loop concurrently with
        // agreement loop.
        let mut step: u8 = 0;

        loop {
            // Perform a single iteration. An iteration runs all registered
            // phases in a row.
            if let Ok(msg) = self
                .run_iteration(&mut cancel_chan_rx, ru, &mut step, &mut provisioners)
                .await
            {
                // Delegate (agreement) message result to agreement loop for
                // further processing.
                self.agreement_process.send_msg(msg.clone()).await;

                // Delay next iteration execution so we avoid consensus-split situation.
                // NB: This should be moved to Block Generator when mempool is integrated.
                tokio::time::sleep(Duration::from_millis(config::CONSENSUS_DELAY_MS)).await;
            } else {
                aggr_handle.abort();
                break;
            }
        }

        let winner = aggr_handle.await.unwrap_or_else(|_| Ok(Block::default()));
        trace!("winning block: {:?}", winner);

        self.teardown(ru.round).await;
    }

    async fn run_iteration(
        &mut self,
        cancel_chan_rx: &mut oneshot::Receiver<bool>,
        ru: RoundUpdate,
        step: &mut u8,
        provisioners: &mut Provisioners,
    ) -> Result<Message, ConsensusError> {
        let mut msg = Message::empty();

        for phase in self.phases.iter_mut() {
            *step += 1;

            // info_span!(round: ru.round, step: *step, bls_key: ru.blskey);

            // Initialize new phase with message returned by previous phase.
            phase.initialize(&msg, ru.round, *step);

            // Construct phase execution context
            let ctx = ExecutionCtx::new(
                cancel_chan_rx,
                self.inbound.clone(),
                self.outbound.clone(),
                &mut self.future_msgs,
                provisioners,
                ru,
                *step,
            );

            // Execute a phase.
            // An error returned here terminates consensus
            // round. This normally happens if consensus channel is cancelled by
            // agreement loop on finding the winning block for this round.
            msg = phase
                .run(ctx)
                .instrument(tracing::info_span!(
                    "main_task",
                    round = ru.round,
                    step = *step,
                    pubkey = ru.pubkey_bls.encode_short_hex(),
                ))
                .await?;

            if *step >= config::CONSENSUS_MAX_STEP {
                return Err(ConsensusError::MaxStepReached);
            }
        }

        Ok(msg)
    }

    async fn teardown(&mut self, round: u64) {
        self.future_msgs.clear(round);
    }
}
