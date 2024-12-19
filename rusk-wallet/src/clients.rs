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
use dusk_core::stake::{StakeFundOwner, StakeKeys};
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::{Note, NoteLeaf, Prove};
use dusk_core::transfer::Transaction;
use dusk_core::Error as ExecutionCoreError;
use flume::Receiver;
use rues::RuesHttpClient;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};
use wallet_core::keys::{
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use wallet_core::pick_notes;
use zeroize::Zeroize;

use self::sync::sync_db;
use super::cache::Cache;
use super::*;
use crate::store::LocalStore;
use crate::{Error, MAX_PROFILES};

const TRANSFER_CONTRACT: &str =
    "0100000000000000000000000000000000000000000000000000000000000000";

const STAKE_CONTRACT: &str =
    "0200000000000000000000000000000000000000000000000000000000000000";

// Sync every 3 seconds for now
const SYNC_INTERVAL_SECONDS: u64 = 3;

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
    status: fn(&str),
    client: RuesHttpClient,
    prover: RuesHttpClient,
    store: LocalStore,
    pub sync_rx: Option<Receiver<String>>,
    sync_join_handle: Option<JoinHandle<()>>,
}

impl State {
    /// Creates a new state instance. Should only be called once.
    pub(crate) fn new(
        data_dir: &Path,
        status: fn(&str),
        client: RuesHttpClient,
        prover: RuesHttpClient,
        store: LocalStore,
    ) -> Result<Self, Error> {
        let cfs = (0..MAX_PROFILES)
            .flat_map(|i| {
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
            sync_join_handle: None,
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

    pub async fn register_sync(&mut self) -> Result<(), Error> {
        let (sync_tx, sync_rx) = flume::unbounded::<String>();

        self.sync_rx = Some(sync_rx);

        let cache = self.cache();
        let status = self.status;
        let client = self.client.clone();
        let store = self.store.clone();

        status("Starting Sync..");

        let handle = tokio::spawn(async move {
            loop {
                let _ = sync_tx.send("Syncing..".to_string());

                let _ = match sync_db(&client, &cache, &store, status).await {
                    Ok(_) => sync_tx.send("Syncing Complete".to_string()),
                    Err(e) => sync_tx.send(format!("Error during sync:.. {e}")),
                };

                sleep(Duration::from_secs(SYNC_INTERVAL_SECONDS)).await;
            }
        });

        self.sync_join_handle = Some(handle);

        Ok(())
    }

    pub async fn sync(&self) -> Result<(), Error> {
        sync_db(&self.client, &self.cache(), &self.store, self.status).await
    }

    /// Requests that a node prove the given shielded transaction.
    /// Returns the transaction unchanged for unshielded transaction.
    pub async fn prove(&self, tx: Transaction) -> Result<Transaction, Error> {
        let prover = &self.prover;
        let mut tx = tx;

        if let Transaction::Phoenix(utx) = &mut tx {
            let status = self.status;
            let proof = utx.proof();

            status("Attempt to prove tx...");

            let proof =
                prover.call("prover", None, "prove", proof).await.map_err(
                    |e| ExecutionCoreError::PhoenixCircuit(e.to_string()),
                )?;

            utx.set_proof(proof);

            status("Proving sucesss!");
        }

        Ok(tx)
    }

    /// Propagate a transaction to a node.
    pub async fn propagate(
        &self,
        tx: Transaction,
    ) -> Result<Transaction, Error> {
        let status = self.status;
        let tx_bytes = tx.to_var_bytes();

        status("Attempt to preverify tx...");
        let _ = self
            .client
            .call("transactions", None, "preverify", &tx_bytes)
            .await?;
        status("Preverify success!");

        status("Propagating tx...");
        let _ = self
            .client
            .call("transactions", None, "propagate", &tx_bytes)
            .await?;
        status("Transaction propagated!");

        Ok(tx)
    }

    /// Selects up to MAX_INPUT_NOTES unspent input notes from the cache. The
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
            .notes(&pk)?
            .into_iter()
            .map(|note_leaf| {
                let nullifier = note_leaf.note.gen_nullifier(&sk);
                (nullifier, note_leaf)
            })
            .collect();

        // pick up to MAX_INPUT_NOTES input-notes that cover the tx-cost
        let tx_input_notes = pick_notes(&vk, cached_notes.into(), tx_cost);
        if tx_input_notes.is_empty() {
            return Err(Error::NotEnoughBalance);
        }

        // construct the transaction input
        let mut tx_input = Vec::<(Note, NoteOpening, BlsScalar)>::new();
        for (nullifier, note_leaf) in tx_input_notes.iter() {
            // fetch the openings for the input-notes
            let opening = self.fetch_opening(note_leaf.as_ref()).await?;

            tx_input.push((note_leaf.note.clone(), opening, *nullifier));
        }

        sk.zeroize();

        Ok(tx_input)
    }

    pub(crate) async fn fetch_account(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<AccountData, Error> {
        let status = self.status;
        status("Fetching account-data...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let account: AccountData = rkyv::from_bytes(
            &self
                .client
                .contract_query::<_, _, 1024>(TRANSFER_CONTRACT, "account", pk)
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        status("account-data received!");

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
        status("Fetching root...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let root: BlsScalar = rkyv::from_bytes(
            &self
                .client
                .contract_query::<(), _, 0>(TRANSFER_CONTRACT, "root", &())
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        status("root received!");

        Ok(root)
    }

    /// Queries the node for the amount staked by a key.
    pub(crate) async fn fetch_stake(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<StakeData>, Error> {
        let status = self.status;
        status("Fetching stake...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let stake_data: Option<StakeData> = rkyv::from_bytes(
            &self
                .client
                .contract_query::<_, _, 1024>(STAKE_CONTRACT, "get_stake", pk)
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        status("Stake received!");

        println!("Staking address: {}", Address::Public(*pk));

        Ok(stake_data)
    }

    /// Get the stake owner of a given stake account.
    pub(crate) async fn fetch_stake_owner(
        &self,
        pk: &BlsPublicKey,
    ) -> Result<Option<StakeFundOwner>, Error> {
        let status = self.status;
        status("Fetching stake owner...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let stake_keys: Option<StakeKeys> = rkyv::from_bytes(
            &self
                .client
                .contract_query::<_, _, 1024>(
                    STAKE_CONTRACT,
                    "get_stake_keys",
                    pk,
                )
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        let stake_owner = stake_keys.map(|keys| keys.owner);

        Ok(stake_owner)
    }

    pub(crate) fn store(&self) -> &LocalStore {
        &self.store
    }

    pub(crate) async fn fetch_chain_id(&self) -> Result<u8, Error> {
        let status = self.status;
        status("Fetching chain_id...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let chain_id: u8 = rkyv::from_bytes(
            &self
                .client
                .contract_query::<_, _, { u8::SIZE }>(
                    TRANSFER_CONTRACT,
                    "chain_id",
                    &(),
                )
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        status("Chain id received!");

        Ok(chain_id)
    }

    /// Queries the node to find the merkle-tree opening for a specific note.
    async fn fetch_opening(&self, note: &Note) -> Result<NoteOpening, Error> {
        let status = self.status;
        status("Fetching note opening...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let opening: Option<NoteOpening> = rkyv::from_bytes(
            &self
                .client
                .contract_query::<_, _, 1024>(
                    TRANSFER_CONTRACT,
                    "opening",
                    note.pos(),
                )
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        // return an error here if the note opening couldn't be fetched
        let opening = opening.ok_or(Error::NoteNotFound)?;

        status("Note opening received!");

        Ok(opening)
    }

    /// Queries the transfer contract for the number of notes.
    pub async fn fetch_num_notes(&self) -> Result<u64, Error> {
        let status = self.status;
        status("Fetching note count...");

        // the target type of the deserialization has to match the return type
        // of the contract-query
        let note_count: u64 = rkyv::from_bytes(
            &self
                .client
                .contract_query::<_, _, { u64::SIZE }>(
                    TRANSFER_CONTRACT,
                    "num_notes",
                    &(),
                )
                .await?,
        )
        .map_err(|_| Error::Rkyv)?;

        status("Latest note count received!");

        Ok(note_count)
    }

    pub fn close(&mut self) {
        self.cache().close();
        let store = &mut self.store;

        // if there's sync handle we abort it
        if let Some(x) = self.sync_join_handle.as_ref() {
            x.abort();
        }

        store.inner_mut().zeroize();
    }
}
