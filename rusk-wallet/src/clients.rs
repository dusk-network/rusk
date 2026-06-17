// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod sync;

use std::path::Path;
use std::sync::{Arc, Mutex};

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::{STAKE_CONTRACT, StakeData, StakeFundOwner, StakeKeys};
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::{
    ArchivedNoteLeaf, Note, NoteLeaf, NoteOpening, Prove,
    PublicKey as PhoenixPublicKey,
};
use dusk_core::transfer::{TRANSFER_CONTRACT, Transaction};
use dusk_core::{BlsScalar, Error as ExecutionCoreError};
use flume::Receiver;
use futures::executor::block_on;
use rkyv::Deserialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use wallet_core::keys::{
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use wallet_core::pick_notes;
use zeroize::Zeroize;

use self::sync::sync_db;
use super::cache::Cache;
use crate::rues::HttpClient as RuesHttpClient;
use crate::store::LocalStore;
use crate::{Address, Error, MAX_PROFILES, WalletStatus, WalletSyncStatus};

const SYNC_INTERVAL_SECONDS: u64 = 3;

fn report_sync_status(
    sync_tx: &flume::Sender<WalletStatus>,
    status: WalletStatus,
) {
    let message = status.to_string();
    if let Err(err) = sync_tx.send(status) {
        tracing::debug!("Dropping sync status update `{message}`: {err}");
    }
}

/// SIZE of the tree leaf
pub const TREE_LEAF: usize = std::mem::size_of::<ArchivedNoteLeaf>();

/// A prover struct that has the `Prove` trait from executio-core implemented.
/// It currently uses a hardcoded prover which delegates the proving to the
/// `prove_execute`
pub struct Prover;

impl Prove for Prover {
    fn prove(
        &self,
        tx_circuit_vec_bytes: &[u8],
    ) -> Result<Vec<u8>, ExecutionCoreError> {
        Ok(tx_circuit_vec_bytes.to_vec())
    }
}

/// The state struct is responsible for managing the state of the wallet
pub struct State {
    cache: Mutex<Arc<Cache>>,
    status: fn(WalletStatus),
    client: RuesHttpClient,
    prover: RuesHttpClient,
    store: LocalStore,
    pub sync_rx: Option<Receiver<WalletStatus>>,
    sync_shutdown: Option<(Arc<Notify>, JoinHandle<()>)>,
    /// Auto-reset stale cache on mismatch (only for local dev nodes).
    allow_cache_reset: bool,
}

impl State {
    /// Creates a new state instance. Should only be called once.
    pub(crate) fn new(
        data_dir: &Path,
        status: fn(WalletStatus),
        client: RuesHttpClient,
        prover: RuesHttpClient,
        store: LocalStore,
        allow_cache_reset: bool,
    ) -> Result<Self, Error> {
        let cfs = (0..MAX_PROFILES)
            .flat_map(|i| {
                // we know that `i < MAX_PROFILES <= u8::MAX`, so casting to u8
                // is safe here
                #[allow(clippy::cast_possible_truncation)]
                let pk: PhoenixPublicKey =
                    derive_phoenix_pk(store.get_seed(), i as u8);

                let pk = bs58::encode(pk.to_bytes()).into_string();

                [pk.clone(), format!("spent_{pk}")]
            })
            .collect();

        let cache = Mutex::new(Arc::new(Cache::new(data_dir, cfs, status)?));

        Ok(Self {
            cache,
            sync_rx: None,
            store,
            prover,
            status,
            client,
            sync_shutdown: None,
            allow_cache_reset,
        })
    }

    /// Returns the reference to the client
    pub fn client(&self) -> &RuesHttpClient {
        &self.client
    }

    pub async fn check_connection(&self) -> bool {
        self.client.check_connection().await.is_ok()
    }

    pub(crate) fn cache(&self) -> Arc<Cache> {
        let state = self.cache.lock();

        // We can get an error if the thread holding the lock panicked while
        // holding the lock. In this case, we can recover the guard from the
        // poison error and return the guard to the caller.
        match state {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    pub fn register_sync(&mut self) {
        let (sync_tx, sync_rx) = flume::unbounded::<WalletStatus>();

        self.sync_rx = Some(sync_rx);

        let cache = self.cache();
        let client = self.client.clone();
        let mut store = self.store.clone();
        let allow_cache_reset = self.allow_cache_reset;
        let shutdown = Arc::new(Notify::new());
        let shutdown_signal = shutdown.clone();

        let handle = tokio::spawn(async move {
            tracing::debug!("Starting background sync loop");
            loop {
                tokio::select! {
                    biased;
                    () = shutdown_signal.notified() => break,
                    () = sleep(Duration::from_secs(SYNC_INTERVAL_SECONDS)) => {
                        let sync_status = {
                            let sync_tx = sync_tx.clone();
                            move |status: WalletStatus| {
                                report_sync_status(&sync_tx, status);
                            }
                        };

                        match sync_db(
                            &client,
                            &cache,
                            &store,
                            sync_status,
                            allow_cache_reset,
                        ).await {
                            Ok(()) => {
                            }
                            Err(e) => {
                                report_sync_status(
                                    &sync_tx,
                                    WalletSyncStatus::Error(e.to_string())
                                        .into(),
                                );
                            }
                        }
                    }
                }
            }
            store.inner_mut().zeroize();
            tracing::debug!("Background sync loop stopped");
        });

        self.sync_shutdown = Some((shutdown, handle));
    }

    pub async fn sync(&self) -> Result<(), Error> {
        sync_db(
            &self.client,
            &self.cache(),
            &self.store,
            self.status,
            self.allow_cache_reset,
        )
        .await
    }

    /// Requests that a node prove the given shielded transaction.
    /// Returns the transaction unchanged for unshielded transaction.
    pub async fn prove(&self, tx: Transaction) -> Result<Transaction, Error> {
        let prover = &self.prover;
        let mut tx = tx;

        if let Transaction::Phoenix(utx) = &mut tx {
            let status = self.status;
            let proof = utx.proof();

            status(WalletStatus::Info("Attempt to prove tx...".into()));

            let proof =
                prover.call("prover", None, "prove", proof).await.map_err(
                    |e| ExecutionCoreError::PhoenixCircuit(e.to_string()),
                )?;

            utx.set_proof(proof);

            status(WalletStatus::Info("Proving success!".into()));
        }

        Ok(tx)
    }

    /// Propagate a transaction to a node.
    pub async fn propagate(
        &self,
        tx: Transaction,
    ) -> Result<Transaction, Error> {
        let status = self.status;
        let tx_bytes = tx.to_network_bytes();

        status(WalletStatus::Info("Attempt to preverify tx...".into()));
        let _ = self
            .client
            .call("transactions", None, "preverify", &tx_bytes)
            .await?;
        status(WalletStatus::Info("Preverify success!".into()));

        status(WalletStatus::Info("Propagating tx...".into()));
        let _ = self
            .client
            .call("transactions", None, "propagate", &tx_bytes)
            .await?;
        status(WalletStatus::Info("Transaction propagated!".into()));

        Ok(tx)
    }

    /// Selects up to `MAX_INPUT_NOTES` unspent input notes from the cache. The
    /// value of the input notes need to cover the cost of the transaction.
    pub(crate) async fn tx_input_notes(
        &self,
        index: u8,
        tx_cost: u64,
    ) -> Result<Vec<(Note, NoteOpening, BlsScalar)>, Error> {
        let vk = derive_phoenix_vk(self.store().get_seed(), index);
        let mut sk = derive_phoenix_sk(self.store().get_seed(), index);
        let pk = derive_phoenix_pk(self.store().get_seed(), index);

        // fetch the cached unspent notes
        let cached_notes: Vec<_> = self
            .cache()
            .notes(&pk)
            .inspect_err(|_| sk.zeroize())?
            .into_iter()
            .map(|note_leaf| {
                let nullifier = note_leaf.note.gen_nullifier(&sk);
                (nullifier, note_leaf)
            })
            .collect();

        sk.zeroize();

        // pick up to MAX_INPUT_NOTES input-notes that cover the tx-cost
        let tx_input_notes = pick_notes(&vk, cached_notes.into(), tx_cost);
        if tx_input_notes.is_empty() {
            return Err(Error::NotEnoughBalance);
        }

        // construct the transaction input
        let mut tx_input = Vec::<(Note, NoteOpening, BlsScalar)>::new();
        for (nullifier, note_leaf) in &tx_input_notes {
            // fetch the openings for the input-notes
            let opening = self.fetch_opening(note_leaf.as_ref()).await?;

            tx_input.push((note_leaf.note.clone(), opening, *nullifier));
        }

        Ok(tx_input)
    }

    pub(crate) async fn fetch_account(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching account-data...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<_, 1024>(TRANSFER_CONTRACT, "account", pk)
            .await?;
        let account: AccountData =
            rkyv::check_archived_root::<AccountData>(&bytes)
                .map_err(|_| Error::Rkyv)?
                .deserialize(&mut rkyv::Infallible)
                .unwrap();

        status(WalletStatus::Info("account-data received!".into()));

        Ok(account)
    }

    pub(crate) fn fetch_notes(
        &self,
        pk: &PhoenixPublicKey,
    ) -> Result<Vec<NoteLeaf>, Error> {
        self.cache().notes(pk).map(|set| set.into_iter().collect())
    }

    /// Fetch the current root of the state.
    pub(crate) async fn fetch_root(&self) -> Result<BlsScalar, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching root...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<(), 0>(TRANSFER_CONTRACT, "root", &())
            .await?;
        let root: BlsScalar = rkyv::check_archived_root::<BlsScalar>(&bytes)
            .map_err(|_| Error::Rkyv)?
            .deserialize(&mut rkyv::Infallible)
            .unwrap();

        status(WalletStatus::Info("root received!".into()));

        Ok(root)
    }

    /// Queries the node for the amount staked by a key.
    pub(crate) async fn fetch_stake(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<StakeData>, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching stake...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<_, 1024>(STAKE_CONTRACT, "get_stake", pk)
            .await?;
        let stake_data: Option<StakeData> =
            rkyv::check_archived_root::<Option<StakeData>>(&bytes)
                .map_err(|_| Error::Rkyv)?
                .deserialize(&mut rkyv::Infallible)
                .unwrap();

        status(WalletStatus::Info("Stake received!".into()));

        status(WalletStatus::Info(format!(
            "Stake account: {}",
            Address::Public(*pk)
        )));

        Ok(stake_data)
    }

    /// Get the stake owner of a given stake account.
    pub(crate) async fn fetch_stake_owner(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<StakeFundOwner>, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching stake owner...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<_, 1024>(STAKE_CONTRACT, "get_stake_keys", pk)
            .await?;
        let stake_keys: Option<StakeKeys> =
            rkyv::check_archived_root::<Option<StakeKeys>>(&bytes)
                .map_err(|_| Error::Rkyv)?
                .deserialize(&mut rkyv::Infallible)
                .unwrap();

        let stake_owner = stake_keys.map(|keys| keys.owner);

        Ok(stake_owner)
    }

    pub(crate) fn store(&self) -> &LocalStore {
        &self.store
    }

    pub(crate) async fn fetch_chain_id(&self) -> Result<u8, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching chain_id...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<_, { u8::SIZE }>(
                TRANSFER_CONTRACT,
                "chain_id",
                &(),
            )
            .await?;
        let chain_id: u8 = rkyv::check_archived_root::<u8>(&bytes)
            .map_err(|_| Error::Rkyv)?
            .deserialize(&mut rkyv::Infallible)
            .unwrap();

        status(WalletStatus::Info("Chain id received!".into()));

        Ok(chain_id)
    }

    /// Queries the node to find the merkle-tree opening for a specific note.
    async fn fetch_opening(&self, note: &Note) -> Result<NoteOpening, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching note opening...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<_, 1024>(TRANSFER_CONTRACT, "opening", note.pos())
            .await?;
        let opening: Option<NoteOpening> =
            rkyv::check_archived_root::<Option<NoteOpening>>(&bytes)
                .map_err(|_| Error::Rkyv)?
                .deserialize(&mut rkyv::Infallible)
                .unwrap();

        let opening = opening.ok_or(Error::NoteNotFound)?;

        status(WalletStatus::Info("Note opening received!".into()));

        Ok(opening)
    }

    /// Queries the transfer contract for the number of notes.
    pub async fn fetch_num_notes(&self) -> Result<u64, Error> {
        let status = self.status;
        status(WalletStatus::Info("Fetching note count...".into()));

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let bytes = self
            .client
            .contract_query::<_, { u64::SIZE }>(
                TRANSFER_CONTRACT,
                "num_notes",
                &(),
            )
            .await?;
        let note_count: u64 = rkyv::check_archived_root::<u64>(&bytes)
            .map_err(|_| Error::Rkyv)?
            .deserialize(&mut rkyv::Infallible)
            .unwrap();

        status(WalletStatus::Info("Latest note count received!".into()));

        Ok(note_count)
    }

    pub fn close(&mut self) {
        self.cache().close();
        let store = &mut self.store;

        if let Some((shutdown, handle)) = self.sync_shutdown.take() {
            shutdown.notify_one();
            if let Err(e) = block_on(handle) {
                eprintln!("Error while closing sync handle: {e}");
            }
        }
        store.inner_mut().zeroize();
    }
}
