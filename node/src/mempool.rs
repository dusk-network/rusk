// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod admission;
pub mod conf;
mod prequeue;

use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use conf::{
    DEFAULT_DOWNLOAD_REDUNDANCY, DEFAULT_EXPIRY_TIME, DEFAULT_IDLE_INTERVAL,
};
use dusk_consensus::errors::BlobError;
use dusk_core::TxPreconditionError;
use dusk_core::transfer::TransactionFormat;
use node_data::events::{Event, TransactionEvent};
use node_data::get_current_timestamp;
use node_data::ledger::{CanonicalTransaction, LedgerTransaction};
use node_data::message::{AsyncQueue, Payload, Topics, payload};
pub use prequeue::FutureNonceRetryHandle;
use prequeue::{
    RETRY_POLL_INTERVAL, drain_unblocked_chain, handle_enqueue_outcome,
    process_due_retries,
};
use rkyv::Infallible;
use rkyv::ser::Serializer;
use rkyv::ser::serializers::{
    BufferScratch, BufferSerializer, BufferSerializerError,
    CompositeSerializer, CompositeSerializerError,
};
use thiserror::Error;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;
use tracing::{error, info, warn};

use self::admission::{TxAdmission, apply_mempool_admission};
use crate::database::{Ledger, Mempool};
use crate::mempool::conf::Params;
use crate::{LongLivedService, Message, Network, database, vm};

const TOPICS: &[u8] = &[Topics::Tx as u8];

pub(super) fn should_replace_conflicting_tx(
    existing: &LedgerTransaction,
    incoming: &LedgerTransaction,
) -> bool {
    incoming.gas_price() > existing.gas_price()
}

#[derive(Debug, Error)]
pub enum TxAcceptanceError {
    #[error("this transaction exists in the mempool")]
    AlreadyExistsInMempool,
    #[error("this transaction exists in the ledger")]
    AlreadyExistsInLedger,
    #[error("Transaction blob id {} is missing sidecar", hex::encode(.0))]
    BlobMissingSidecar([u8; 32]),
    #[error("No blobs provided")]
    BlobEmpty,
    #[error("Transaction has too many blobs: {0}")]
    BlobTooMany(usize),
    #[error("Invalid blob: {0}")]
    BlobInvalid(String),
    #[error("this transaction's spendId exists in the mempool")]
    SpendIdExistsInMempool,
    #[error("this transaction is invalid {0}")]
    VerificationFailed(String),
    #[error("gas price lower than minimum {0}")]
    GasPriceTooLow(u64),
    #[error("gas limit lower than minimum {0}")]
    GasLimitTooLow(u64),
    #[error(
        "transaction format {actual:?} is not supported for live ingress; minimum supported format is {minimum:?}"
    )]
    UnsupportedIngressFormat {
        actual: TransactionFormat,
        minimum: TransactionFormat,
    },
    #[error("Maximum count of transactions exceeded {0}")]
    MaxTxnCountExceeded(usize),
    #[error("Missing intermediate nonce {0}")]
    MissingIntermediateNonce(u64),
    #[error("Maximum future nonce retry queue size exceeded {0}")]
    MaxFutureNonceQueueExceeded(usize),
    #[error(
        "Maximum queued future Moonlight transactions per account exceeded {0}"
    )]
    MaxMoonlightFutureNoncePerAccountExceeded(usize),
    #[error("this transaction is too large to be serialized")]
    TooLarge,
    #[error("Maximum transaction size exceeded {0}")]
    MaxSizeExceeded(usize),
    #[error("A generic error occurred {0}")]
    Generic(anyhow::Error),
}

impl From<anyhow::Error> for TxAcceptanceError {
    fn from(err: anyhow::Error) -> Self {
        Self::Generic(err)
    }
}

impl From<BlobError> for TxAcceptanceError {
    fn from(err: BlobError) -> Self {
        match err {
            BlobError::MissingSidecar(id) => {
                TxAcceptanceError::BlobMissingSidecar(id)
            }
            BlobError::BlobEmpty => TxAcceptanceError::BlobEmpty,
            BlobError::BlobTooMany(n) => TxAcceptanceError::BlobTooMany(n),
            BlobError::BlobInvalid(msg) => TxAcceptanceError::BlobInvalid(msg),
        }
    }
}

impl From<TxPreconditionError> for TxAcceptanceError {
    fn from(err: TxPreconditionError) -> Self {
        match err {
            TxPreconditionError::BlobLowLimit(min) => {
                TxAcceptanceError::GasLimitTooLow(min)
            }
            TxPreconditionError::DeployChargeOverflow => {
                TxAcceptanceError::VerificationFailed(
                    "deploy charge overflow".into(),
                )
            }
            TxPreconditionError::BlobChargeOverflow => {
                TxAcceptanceError::VerificationFailed(
                    "blob charge overflow".into(),
                )
            }
            TxPreconditionError::DeployLowLimit(min) => {
                TxAcceptanceError::GasLimitTooLow(min)
            }
            TxPreconditionError::DeployLowPrice(min) => {
                TxAcceptanceError::GasPriceTooLow(min)
            }
            TxPreconditionError::BlobEmpty => TxAcceptanceError::BlobEmpty,
            TxPreconditionError::BlobTooMany(n) => {
                TxAcceptanceError::BlobTooMany(n)
            }
            TxPreconditionError::PhoenixFeeOverflow => {
                TxAcceptanceError::VerificationFailed(
                    "phoenix fee overflow".into(),
                )
            }
            TxPreconditionError::PhoenixFeeTampered => {
                TxAcceptanceError::VerificationFailed(
                    "phoenix fee tampered".into(),
                )
            }
            TxPreconditionError::PhoenixFeeRefundMismatch => {
                TxAcceptanceError::VerificationFailed(
                    "phoenix fee refund stealth address mismatch".into(),
                )
            }
        }
    }
}

fn check_supported_ingress_tx_format(
    tx: &CanonicalTransaction,
) -> Result<(), TxAcceptanceError> {
    // Live network admission accepts both Aegis and Boreas envelopes. Only the
    // historical PreAegis encoding remains replay-only.
    if tx.format() == TransactionFormat::PreAegis {
        return Err(TxAcceptanceError::UnsupportedIngressFormat {
            actual: tx.format(),
            minimum: TransactionFormat::Aegis,
        });
    }

    Ok(())
}

fn normalize_ingress_tx(
    tx: &LedgerTransaction,
    block_height: u64,
) -> Result<LedgerTransaction, TxAcceptanceError> {
    check_supported_ingress_tx_format(tx.canonical())?;
    Ok(tx.reformat_for_ingress(block_height))
}

pub struct MempoolSrv {
    inbound: AsyncQueue<Message>,
    conf: Params,
    /// Sender channel for sending out RUES events
    event_sender: Sender<Event>,
    future_nonce_retry_queue: FutureNonceRetryHandle,
}

impl MempoolSrv {
    pub fn new(conf: Params, event_sender: Sender<Event>) -> Self {
        let queue = FutureNonceRetryHandle::new(
            conf.max_queue_size,
            conf.max_moonlight_future_nonce_per_account,
        );
        Self::with_future_nonce_retry_queue(conf, event_sender, queue)
    }

    pub fn with_future_nonce_retry_queue(
        conf: Params,
        event_sender: Sender<Event>,
        future_nonce_retry_queue: FutureNonceRetryHandle,
    ) -> Self {
        info!("MempoolSrv::new with conf {}", conf);
        Self {
            inbound: AsyncQueue::bounded(
                conf.max_queue_size,
                "mempool_inbound",
            ),
            conf,
            event_sender,
            future_nonce_retry_queue,
        }
    }
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for MempoolSrv
{
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        // Request mempool update from N alive peers
        self.request_mempool(&network).await;

        let idle_interval =
            self.conf.idle_interval.unwrap_or(DEFAULT_IDLE_INTERVAL);

        let mempool_expiry = self
            .conf
            .mempool_expiry
            .unwrap_or(DEFAULT_EXPIRY_TIME)
            .as_secs();

        let retry_queue = self.future_nonce_retry_queue.clone();
        let retry_event_sender = self.event_sender.clone();
        let retry_max_mempool_txn_count = self.conf.max_mempool_txn_count;
        let retry_network = network.clone();
        let retry_db = db.clone();
        let retry_vm = vm.clone();
        tokio::spawn(async move {
            MempoolSrv::run_retry_worker(
                retry_queue,
                retry_event_sender,
                retry_max_mempool_txn_count,
                retry_network,
                retry_db,
                retry_vm,
            )
            .await;
        });

        // Mempool service loop
        let mut on_idle_event = tokio::time::interval(idle_interval);
        loop {
            tokio::select! {
                biased;
                _ = on_idle_event.tick() => {
                    info!(event = "mempool_idle", interval = ?idle_interval);

                    let expiration_time = get_current_timestamp()
                        .checked_sub(mempool_expiry)
                        .expect("valid duration");

                    // Remove expired transactions from the mempool
                    db.read().await.update(|db| {
                        let expired_txs = db.mempool_expired_txs(expiration_time).unwrap_or_else(|e| {
                            error!("cannot get expired txs: {e}");
                            vec![]
                        });
                        for tx_id in expired_txs {
                            info!(event = "expired_tx", hash = hex::encode(tx_id));
                            let deleted_txs = db.delete_mempool_tx(tx_id, true).unwrap_or_else(|e| {
                                error!("cannot delete expired tx: {e}");
                                vec![]
                            });
                            for deleted_tx_id in deleted_txs{
                                let event = TransactionEvent::Removed(deleted_tx_id);
                                info!(event = "mempool_deleted", hash = hex::encode(deleted_tx_id));
                                if let Err(e) = self.event_sender.try_send(event.into()) {
                                    warn!("cannot notify mempool removed transaction {e}")
                                };
                            }
                        }
                        Ok(())
                    })?;

                },
                msg = self.inbound.recv() => {
                    if let Ok(msg) = msg {
                        match &msg.payload {
                            Payload::Transaction(tx) => {
                                if let Err(e) = self
                                    .handle_tx_message(
                                        &network,
                                        &db,
                                        &vm,
                                        &msg,
                                    )
                                    .await
                                {
                                    error!("Tx {} not accepted: {e}", hex::encode(tx.id()));
                                };
                            }
                            _ => error!("invalid inbound message payload"),
                        }
                    }
                }
            }
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "mempool"
    }
}

impl MempoolSrv {
    async fn run_retry_worker<
        N: Network,
        DB: database::DB,
        VM: vm::VMExecution,
    >(
        future_nonce_retry_queue: FutureNonceRetryHandle,
        event_sender: Sender<Event>,
        max_mempool_txn_count: usize,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        vm: Arc<RwLock<VM>>,
    ) {
        let mut on_retry_event = tokio::time::interval(RETRY_POLL_INTERVAL);

        loop {
            on_retry_event.tick().await;
            process_due_retries(
                &future_nonce_retry_queue,
                &event_sender,
                max_mempool_txn_count,
                &network,
                &db,
                &vm,
                Instant::now(),
            )
            .await;
        }
    }

    async fn broadcast_tx<N: Network>(network: &Arc<RwLock<N>>, msg: &Message) {
        let network = network.read().await;
        if let Err(e) = network.broadcast(msg).await {
            warn!("Unable to broadcast accepted tx: {e}");
        };
    }

    async fn broadcast_accepted_tx<N: Network>(
        network: &Arc<RwLock<N>>,
        msg: &Message,
        tx: &LedgerTransaction,
        source: Option<&str>,
        queue_age_ms: Option<u64>,
    ) {
        if let Some(source) = source {
            info!(
                event = "future_nonce_retry_accepted",
                hash = hex::encode(tx.id()),
                source,
                queue_age_ms
            );
        }
        Self::broadcast_tx(network, msg).await;
    }

    async fn handle_tx_message<
        N: Network,
        DB: database::DB,
        VM: vm::VMExecution,
    >(
        &mut self,
        network: &Arc<RwLock<N>>,
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        msg: &Message,
    ) -> Result<(), TxAcceptanceError> {
        let Payload::Transaction(tx) = &msg.payload else {
            return Err(TxAcceptanceError::Generic(anyhow!(
                "invalid inbound message payload"
            )));
        };

        let next_block_height = db
            .read()
            .await
            .view(|db| db.latest_block())
            .map_err(|e| {
                TxAcceptanceError::Generic(anyhow!(
                    "Cannot get tip block height from the database: {e}"
                ))
            })?
            .header
            .height
            .saturating_add(1);
        let tx = normalize_ingress_tx(tx, next_block_height)?;
        let msg = {
            let mut normalized = msg.clone();
            normalized.payload = tx.clone().into();
            normalized
        };

        match Self::accept_tx(
            &self.event_sender,
            self.conf.max_mempool_txn_count,
            db,
            vm,
            &tx,
        )
        .await
        {
            Ok(()) => {
                Self::broadcast_accepted_tx(network, &msg, &tx, None, None)
                    .await;
                drain_unblocked_chain(
                    &self.future_nonce_retry_queue,
                    &self.event_sender,
                    self.conf.max_mempool_txn_count,
                    network,
                    db,
                    vm,
                    &tx,
                )
                .await;
                Ok(())
            }
            Err(TxAcceptanceError::MissingIntermediateNonce(_)) => {
                handle_enqueue_outcome(
                    &self.event_sender,
                    &tx,
                    self.future_nonce_retry_queue
                        .enqueue_message_with_outcome(&msg)
                        .await,
                )
            }
            Err(err) => Err(err),
        }
    }

    async fn accept_tx<DB: database::DB, VM: vm::VMExecution>(
        event_sender: &Sender<Event>,
        max_mempool_txn_count: usize,
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        tx: &LedgerTransaction,
    ) -> Result<(), TxAcceptanceError> {
        let events =
            MempoolSrv::check_tx(db, vm, tx, false, max_mempool_txn_count)
                .await?;

        tracing::info!(
            event = "transaction accepted",
            hash = hex::encode(tx.id())
        );

        for tx_event in events {
            let node_event = tx_event.into();
            if let Err(e) = event_sender.try_send(node_event) {
                warn!("cannot notify mempool accepted transaction {e}")
            };
        }

        Ok(())
    }

    pub async fn check_tx<'t, DB: database::DB, VM: vm::VMExecution>(
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        tx: &'t LedgerTransaction,
        dry_run: bool,
        max_mempool_txn_count: usize,
    ) -> Result<Vec<TransactionEvent<'t>>, TxAcceptanceError> {
        let admission = TxAdmission::new(db, vm, max_mempool_txn_count)
            .check(tx.canonical())
            .await?;

        let mut events = vec![];
        db.read().await.update_dry_run(dry_run, |db| {
            events = apply_mempool_admission(
                db,
                tx,
                &admission.facts,
                admission.tx_to_delete,
                get_current_timestamp(),
            )?;
            Ok(())
        })?;

        Ok(events)
    }

    pub async fn check_canonical_tx_at_tip<
        DB: database::DB,
        VM: vm::VMExecution,
    >(
        db: &Arc<RwLock<DB>>,
        vm: &Arc<RwLock<VM>>,
        tx: &CanonicalTransaction,
        tip_height: u64,
        max_mempool_txn_count: usize,
    ) -> Result<LedgerTransaction, TxAcceptanceError> {
        let _ = TxAdmission::new(db, vm, max_mempool_txn_count)
            .check_with_tip(tx, tip_height)
            .await?;
        Ok(tx.clone().into())
    }

    /// Requests full mempool data from N alive peers
    ///
    /// Message flow:
    /// GetMempool -> Inv -> GetResource -> Tx
    async fn request_mempool<N: Network>(&self, network: &Arc<RwLock<N>>) {
        const WAIT_TIMEOUT: Duration = Duration::from_secs(5);
        let max_peers = self
            .conf
            .mempool_download_redundancy
            .unwrap_or(DEFAULT_DOWNLOAD_REDUNDANCY);

        let net = network.read().await;
        net.wait_for_alive_nodes(max_peers, WAIT_TIMEOUT).await;

        let msg = payload::GetMempool::default().into();
        if let Err(err) = net.send_to_alive_peers(msg, max_peers).await {
            error!("could not request mempool from network: {err}");
        }
    }
}

fn check_tx_serialization(
    tx: &dusk_core::transfer::Transaction,
) -> Result<(), TxAcceptanceError> {
    // The transaction is an argument to the transfer contract, so
    // its serialized size has to be within the same 64Kib limit.
    const SCRATCH_BUF_BYTES: usize = 1024;
    const ARGBUF_LEN: usize = 64 * 1024;
    let stripped_tx = tx.strip_off_bytecode().or(tx.blob_to_memo());
    let mut sbuf = [0u8; SCRATCH_BUF_BYTES];
    let mut buffer = [0u8; ARGBUF_LEN];
    let scratch = BufferScratch::new(&mut sbuf);
    let ser = BufferSerializer::new(&mut buffer);
    let mut ser = CompositeSerializer::new(ser, scratch, Infallible);
    if let Err(err) = ser.serialize_value(stripped_tx.as_ref().unwrap_or(tx)) {
        match err {
            CompositeSerializerError::SerializerError(err) => match err {
                BufferSerializerError::Overflow { .. } => {
                    return Err(TxAcceptanceError::TooLarge);
                }
            },
            err => return Err(TxAcceptanceError::Generic(anyhow!("{err}"))),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use dusk_core::signatures::bls::{PublicKey, SecretKey};
    use rand::rngs::StdRng;
    use rand::{CryptoRng, Rng, RngCore, SeedableRng};
    use wallet_core::transaction::moonlight_deployment;

    use super::*;

    fn new_moonlight_deploy_tx<R: RngCore + CryptoRng>(
        rng: &mut R,
        bytecode: Vec<u8>,
        init_args: Vec<u8>,
    ) -> dusk_core::transfer::Transaction {
        const CHAIN_ID: u8 = 0xfa;
        let sk = SecretKey::random(rng);
        let pk = PublicKey::from(&SecretKey::random(rng));

        let gas_limit: u64 = rng.r#gen();
        let gas_price: u64 = rng.r#gen();
        let nonce: u64 = rng.r#gen();
        let deploy_nonce: u64 = rng.r#gen();

        moonlight_deployment(
            &sk,
            bytecode,
            &pk,
            init_args,
            gas_limit,
            gas_price,
            nonce,
            deploy_nonce,
            CHAIN_ID,
        )
        .expect("should create a transaction")
    }

    const MAX_MOONLIGHT_ARG_SIZE: usize = 64 * 1024 - 2320;

    #[test]
    fn test_tx_serialization_check_normal() {
        let mut rng = StdRng::seed_from_u64(42);
        let tx = new_moonlight_deploy_tx(
            &mut rng,
            vec![0; 64 * 1024],
            vec![0; MAX_MOONLIGHT_ARG_SIZE],
        );
        let result = check_tx_serialization(&tx);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn test_tx_serialization_check_tx_too_large() {
        let mut rng = StdRng::seed_from_u64(42);
        let tx = new_moonlight_deploy_tx(
            &mut rng,
            vec![0; 64 * 1024],
            vec![0; MAX_MOONLIGHT_ARG_SIZE + 1],
        );
        let result = check_tx_serialization(&tx);
        assert!(matches!(result, Err(TxAcceptanceError::TooLarge)));
    }

    #[test]
    fn test_supported_ingress_format_check_rejects_pre_aegis() {
        let mut rng = StdRng::seed_from_u64(42);
        let tx = CanonicalTransaction::canonicalize(
            new_moonlight_deploy_tx(&mut rng, vec![0; 32], vec![0; 32]),
            TransactionFormat::PreAegis,
        );

        let result = check_supported_ingress_tx_format(&tx);

        assert!(matches!(
            result,
            Err(TxAcceptanceError::UnsupportedIngressFormat {
                actual: TransactionFormat::PreAegis,
                minimum: TransactionFormat::Aegis,
            })
        ));
    }

    #[test]
    fn test_supported_ingress_format_check_accepts_aegis() {
        let mut rng = StdRng::seed_from_u64(42);
        let tx = CanonicalTransaction::canonicalize(
            new_moonlight_deploy_tx(&mut rng, vec![0; 32], vec![0; 32]),
            node_data::hard_fork::ingress_tx_format_at(1),
        );

        let result = check_supported_ingress_tx_format(&tx);

        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn test_normalize_ingress_tx_reformats_aegis_to_boreas() {
        let mut rng = StdRng::seed_from_u64(42);
        let tx = LedgerTransaction::from_protocol_with_format(
            new_moonlight_deploy_tx(&mut rng, vec![0; 32], vec![0; 32]),
            TransactionFormat::Aegis,
        );

        let normalized = normalize_ingress_tx(&tx, u64::MAX)
            .expect("aegis ingress should normalize to boreas");

        assert_eq!(normalized.format(), TransactionFormat::Boreas);
        assert_eq!(normalized.id(), tx.id());
    }

    #[test]
    fn test_normalize_ingress_tx_reformats_boreas_to_aegis() {
        let mut rng = StdRng::seed_from_u64(42);
        let tx = LedgerTransaction::from_protocol_with_format(
            new_moonlight_deploy_tx(&mut rng, vec![0; 32], vec![0; 32]),
            TransactionFormat::Boreas,
        );

        let normalized = normalize_ingress_tx(&tx, 1)
            .expect("boreas ingress should normalize to aegis");

        assert_eq!(normalized.format(), TransactionFormat::Aegis);
        assert_eq!(normalized.id(), tx.id());
    }
}
