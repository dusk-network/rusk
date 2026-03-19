// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use node_data::events::{Event, TransactionEvent};
use node_data::ledger::{LedgerTransaction, SpendingId};
use node_data::message::{Message, Payload};
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex as AsyncMutex, RwLock};
use tokio::time::Instant;
use tracing::{info, warn};

use super::{MempoolSrv, TxAcceptanceError};
use crate::{Network, database, vm};

pub(super) const RETRY_DELAY: Duration = Duration::from_millis(500);
pub(super) const RETRY_POLL_INTERVAL: Duration = Duration::from_millis(100);
pub(super) const MAX_RETRIES: usize = 10;

pub(super) type PrequeueKey = (Vec<u8>, u64);

#[derive(Clone, Copy, Debug)]
pub(super) enum EnqueueOutcome {
    Deferred,
    Replaced([u8; 32]),
    Ignored,
}

#[derive(Clone, Debug)]
pub(super) struct PendingPrequeueTx {
    pub(super) msg: Message,
    pub(super) key: PrequeueKey,
    pub(super) queued_at: Instant,
    next_retry_at: Instant,
    pub(super) retries_remaining: usize,
}

#[derive(Debug)]
struct Prequeue {
    max_size: usize,
    max_per_account: usize,
    pending: HashMap<PrequeueKey, PendingPrequeueTx>,
    per_account_counts: HashMap<Vec<u8>, usize>,
}

#[derive(Clone, Debug)]
pub struct FutureNonceRetryHandle {
    inner: Arc<AsyncMutex<Prequeue>>,
}

impl Prequeue {
    fn new(max_size: usize, max_per_account: usize) -> Self {
        Self {
            max_size,
            max_per_account,
            pending: HashMap::new(),
            per_account_counts: HashMap::new(),
        }
    }

    fn insert_pending(
        &mut self,
        key: PrequeueKey,
        msg: Message,
        queued_at: Instant,
        next_retry_at: Instant,
        retries_remaining: usize,
    ) {
        *self.per_account_counts.entry(key.0.clone()).or_default() += 1;
        self.pending.insert(
            key.clone(),
            PendingPrequeueTx {
                msg,
                key,
                queued_at,
                next_retry_at,
                retries_remaining,
            },
        );
    }

    fn enqueue(
        &mut self,
        msg: &Message,
    ) -> Result<EnqueueOutcome, TxAcceptanceError> {
        let tx = pending_tx(msg).ok_or_else(|| {
            TxAcceptanceError::Generic(anyhow!(
                "prequeue only supports Moonlight txs"
            ))
        })?;
        let key = prequeue_key(tx).ok_or_else(|| {
            TxAcceptanceError::Generic(anyhow!(
                "prequeue only supports Moonlight txs"
            ))
        })?;
        let tx_id = tx.id();

        if let Some(existing) = self.pending.get_mut(&key) {
            let existing_tx =
                pending_tx(&existing.msg).expect("message is a tx");
            if should_replace_queued_tx(existing_tx, tx) {
                let replaced = existing_tx.id();
                existing.msg = msg.clone();
                existing.queued_at = Instant::now();
                existing.next_retry_at = Instant::now() + RETRY_DELAY;
                existing.retries_remaining = MAX_RETRIES;
                info!(
                    event = "future_nonce_retry_replaced",
                    hash = hex::encode(tx_id),
                    nonce = key.1
                );
                return Ok(EnqueueOutcome::Replaced(replaced));
            }
            return Ok(EnqueueOutcome::Ignored);
        }

        let current_per_account = self
            .per_account_counts
            .get(&key.0)
            .copied()
            .unwrap_or_default();
        if current_per_account >= self.max_per_account {
            return Err(
                TxAcceptanceError::MaxMoonlightFutureNoncePerAccountExceeded(
                    self.max_per_account,
                ),
            );
        }

        if self.pending.len() >= self.max_size {
            warn!(
                hash = hex::encode(tx_id),
                "future nonce retry queue full, dropping tx"
            );
            return Err(TxAcceptanceError::MaxFutureNonceQueueExceeded(
                self.max_size,
            ));
        }

        self.insert_pending(
            key.clone(),
            msg.clone(),
            Instant::now(),
            Instant::now() + RETRY_DELAY,
            MAX_RETRIES,
        );
        info!(
            event = "future_nonce_retry_queued",
            hash = hex::encode(tx_id),
            nonce = key.1,
            delay_ms = RETRY_DELAY.as_millis() as u64,
            retries_remaining = MAX_RETRIES
        );

        Ok(EnqueueOutcome::Deferred)
    }

    fn take_due(&mut self, now: Instant) -> Vec<PendingPrequeueTx> {
        let mut due = Vec::new();
        self.pending.retain(|_, pending| {
            if pending.next_retry_at <= now {
                due.push(pending.clone());
                false
            } else {
                true
            }
        });

        for pending in &due {
            decrement_account_count(
                &mut self.per_account_counts,
                &pending.key.0,
            );
        }

        due.sort_by(|lhs, rhs| lhs.key.cmp(&rhs.key));
        due
    }

    fn take_by_key(&mut self, key: &PrequeueKey) -> Option<PendingPrequeueTx> {
        let pending = self.pending.remove(key)?;
        decrement_account_count(&mut self.per_account_counts, &key.0);
        Some(pending)
    }

    fn reschedule(&mut self, pending: PendingPrequeueTx, now: Instant) {
        let tx = pending_tx(&pending.msg).expect("message is a tx");
        let retries_remaining = pending.retries_remaining - 1;
        let tx_id = tx.id();

        self.insert_pending(
            pending.key,
            pending.msg,
            pending.queued_at,
            now + RETRY_DELAY,
            retries_remaining,
        );
        info!(
            event = "future_nonce_retry_rescheduled",
            hash = hex::encode(tx_id),
            retries_remaining,
            queue_age_ms =
                now.duration_since(pending.queued_at).as_millis() as u64
        );
    }
}

impl FutureNonceRetryHandle {
    pub fn new(max_size: usize, max_per_account: usize) -> Self {
        Self {
            inner: Arc::new(AsyncMutex::new(Prequeue::new(
                max_size,
                max_per_account,
            ))),
        }
    }

    pub async fn enqueue_message_report(
        &self,
        msg: &Message,
    ) -> (Vec<Event>, Result<(), TxAcceptanceError>) {
        let outcome = self.inner.lock().await.enqueue(msg);
        let Some(tx) = pending_tx(msg) else {
            return (vec![], outcome.map(|_| ()));
        };

        enqueue_outcome_report(tx, outcome)
    }

    pub(super) async fn enqueue_message_with_outcome(
        &self,
        msg: &Message,
    ) -> Result<EnqueueOutcome, TxAcceptanceError> {
        self.inner.lock().await.enqueue(msg)
    }

    pub(super) async fn take_due(
        &self,
        now: Instant,
    ) -> Vec<PendingPrequeueTx> {
        self.inner.lock().await.take_due(now)
    }

    pub(super) async fn take_for_account_nonce(
        &self,
        account: &[u8],
        nonce: u64,
    ) -> Option<PendingPrequeueTx> {
        let key = (account.to_vec(), nonce);
        self.inner.lock().await.take_by_key(&key)
    }

    pub(super) async fn reschedule_message(
        &self,
        pending: PendingPrequeueTx,
        now: Instant,
    ) {
        self.inner.lock().await.reschedule(pending, now);
    }
}

fn emit_tx_event(event_sender: &Sender<Event>, event: TransactionEvent<'_>) {
    if let Err(e) = event_sender.try_send(event.into()) {
        warn!("cannot notify transaction event {e}");
    }
}

fn emit_prequeue_dropped(
    event_sender: &Sender<Event>,
    tx: &LedgerTransaction,
    reason: &'static str,
) {
    emit_tx_event(event_sender, TransactionEvent::Dropped(tx.id(), reason));
}

pub(super) fn handle_enqueue_outcome(
    event_sender: &Sender<Event>,
    tx: &LedgerTransaction,
    outcome: Result<EnqueueOutcome, TxAcceptanceError>,
) -> Result<(), TxAcceptanceError> {
    let (events, result) = enqueue_outcome_report(tx, outcome);
    for event in events {
        if let Err(e) = event_sender.try_send(event) {
            warn!("cannot notify transaction event {e}");
        }
    }
    result
}

fn enqueue_outcome_report(
    tx: &LedgerTransaction,
    outcome: Result<EnqueueOutcome, TxAcceptanceError>,
) -> (Vec<Event>, Result<(), TxAcceptanceError>) {
    let mut events = Vec::new();
    let result = match outcome {
        Ok(EnqueueOutcome::Deferred) => {
            events.push(
                TransactionEvent::Deferred(
                    tx.id(),
                    "missing_intermediate_nonce",
                )
                .into(),
            );
            Ok(())
        }
        Ok(EnqueueOutcome::Replaced(replaced)) => {
            events.push(
                TransactionEvent::Dropped(replaced, "replaced_in_prequeue")
                    .into(),
            );
            events.push(
                TransactionEvent::Deferred(
                    tx.id(),
                    "missing_intermediate_nonce",
                )
                .into(),
            );
            Ok(())
        }
        Ok(EnqueueOutcome::Ignored) => {
            events.push(
                TransactionEvent::Dropped(tx.id(), "superseded_by_staged_tx")
                    .into(),
            );
            Ok(())
        }
        Err(err) => {
            match &err {
                TxAcceptanceError::MaxFutureNonceQueueExceeded(_) => {
                    events.push(
                        TransactionEvent::Dropped(tx.id(), "prequeue_full")
                            .into(),
                    );
                }
                TxAcceptanceError::MaxMoonlightFutureNoncePerAccountExceeded(
                    _,
                ) => {
                    events.push(
                        TransactionEvent::Dropped(
                            tx.id(),
                            "prequeue_account_limit",
                        )
                        .into(),
                    );
                }
                _ => {}
            }
            Err(err)
        }
    };

    (events, result)
}

async fn process_pending_tx<
    N: Network,
    DB: database::DB,
    VM: vm::VMExecution,
>(
    ctx: &PrequeueProcessCtx<'_, N, DB, VM>,
    pending: PendingPrequeueTx,
    now: Instant,
    source: &'static str,
    failure_reason: &'static str,
) -> bool {
    let Payload::Transaction(tx) = &pending.msg.payload else {
        return false;
    };
    let queue_age_ms = now.duration_since(pending.queued_at).as_millis() as u64;

    match MempoolSrv::accept_tx(
        ctx.event_sender,
        ctx.max_mempool_txn_count,
        ctx.db,
        ctx.vm,
        tx,
    )
    .await
    {
        Ok(()) => {
            MempoolSrv::broadcast_accepted_tx(
                ctx.network,
                &pending.msg,
                tx,
                Some(source),
                Some(queue_age_ms),
            )
            .await;
            true
        }
        Err(TxAcceptanceError::MissingIntermediateNonce(_))
            if pending.retries_remaining > 1 =>
        {
            ctx.prequeue.reschedule_message(pending, now).await;
            false
        }
        Err(err) => {
            let reason = match err {
                TxAcceptanceError::MissingIntermediateNonce(_) => {
                    "retry_exhausted"
                }
                _ => failure_reason,
            };
            emit_prequeue_dropped(ctx.event_sender, tx, reason);
            warn!(
                hash = hex::encode(tx.id()),
                queue_age_ms, "future nonce {source} dropped: {err}"
            );
            false
        }
    }
}

struct PrequeueProcessCtx<'a, N, DB, VM> {
    prequeue: &'a FutureNonceRetryHandle,
    event_sender: &'a Sender<Event>,
    max_mempool_txn_count: usize,
    network: &'a Arc<RwLock<N>>,
    db: &'a Arc<RwLock<DB>>,
    vm: &'a Arc<RwLock<VM>>,
}

pub(super) async fn process_due_retries<
    N: Network,
    DB: database::DB,
    VM: vm::VMExecution,
>(
    prequeue: &FutureNonceRetryHandle,
    event_sender: &Sender<Event>,
    max_mempool_txn_count: usize,
    network: &Arc<RwLock<N>>,
    db: &Arc<RwLock<DB>>,
    vm: &Arc<RwLock<VM>>,
    now: Instant,
) {
    let ctx = PrequeueProcessCtx {
        prequeue,
        event_sender,
        max_mempool_txn_count,
        network,
        db,
        vm,
    };
    let due = ctx.prequeue.take_due(now).await;
    if !due.is_empty() {
        let oldest_queue_age_ms = due
            .iter()
            .map(|pending| now.duration_since(pending.queued_at).as_millis())
            .max()
            .unwrap_or_default() as u64;
        info!(
            event = "future_nonce_retry_due_batch",
            count = due.len(),
            oldest_queue_age_ms
        );
    }

    for pending in due {
        let Payload::Transaction(tx) = &pending.msg.payload else {
            continue;
        };
        let accepted_tx = tx.clone();
        if process_pending_tx(&ctx, pending, now, "retry", "retry_failed").await
        {
            drain_unblocked_chain(
                ctx.prequeue,
                ctx.event_sender,
                ctx.max_mempool_txn_count,
                ctx.network,
                ctx.db,
                ctx.vm,
                &accepted_tx,
            )
            .await;
        }
    }
}

pub(super) async fn drain_unblocked_chain<
    N: Network,
    DB: database::DB,
    VM: vm::VMExecution,
>(
    prequeue: &FutureNonceRetryHandle,
    event_sender: &Sender<Event>,
    max_mempool_txn_count: usize,
    network: &Arc<RwLock<N>>,
    db: &Arc<RwLock<DB>>,
    vm: &Arc<RwLock<VM>>,
    accepted_tx: &LedgerTransaction,
) {
    let ctx = PrequeueProcessCtx {
        prequeue,
        event_sender,
        max_mempool_txn_count,
        network,
        db,
        vm,
    };
    let Some(key) = account_nonce_key(accepted_tx) else {
        return;
    };

    let mut next_nonce = key.1 + 1;
    loop {
        let Some(pending) = ctx
            .prequeue
            .take_for_account_nonce(&key.0, next_nonce)
            .await
        else {
            break;
        };
        if !process_pending_tx(
            &ctx,
            pending,
            Instant::now(),
            "contiguous_drain",
            "contiguous_drain_failed",
        )
        .await
        {
            break;
        }
        next_nonce += 1;
    }
}

fn pending_tx(msg: &Message) -> Option<&LedgerTransaction> {
    let Payload::Transaction(tx) = &msg.payload else {
        return None;
    };
    Some(tx)
}

fn prequeue_key(tx: &LedgerTransaction) -> Option<PrequeueKey> {
    let spend_ids = tx.to_spend_ids();
    let [SpendingId::AccountNonce(account, nonce)] = spend_ids.as_slice()
    else {
        return None;
    };
    Some((account.to_raw_bytes().to_vec(), *nonce))
}

pub(super) fn account_nonce_key(tx: &LedgerTransaction) -> Option<PrequeueKey> {
    prequeue_key(tx)
}

fn should_replace_queued_tx(
    existing: &LedgerTransaction,
    incoming: &LedgerTransaction,
) -> bool {
    incoming.gas_price() > existing.gas_price()
        || (incoming.gas_price() == existing.gas_price()
            && incoming.protocol().gas_limit()
                > existing.protocol().gas_limit())
}

fn decrement_account_count(
    per_account_counts: &mut HashMap<Vec<u8>, usize>,
    account: &[u8],
) {
    if let Some(count) = per_account_counts.get_mut(account) {
        if *count <= 1 {
            per_account_counts.remove(account);
        } else {
            *count -= 1;
        }
    }
}
