// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod sync;

use dusk_bytes::Serializable;
use execution_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    transfer::{
        moonlight::AccountData,
        phoenix::{Note, NoteLeaf, Prove},
        Transaction,
    },
    Error as ExecutionCoreError,
};
use flume::Receiver;
use futures::{StreamExt, TryStreamExt};
use rues::RuesHttpClient;
use tokio::{
    task::JoinHandle,
    time::{sleep, Duration},
};
use wallet_core::{
    keys::{derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk},
    pick_notes,
};
use zeroize::Zeroize;

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use self::sync::sync_db;

use super::{cache::Cache, *};

use crate::{
    rusk::{RuskHttpClient, RuskRequest},
    store::LocalStore,
    Error, MAX_PROFILES,
};

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
    prover: RuskHttpClient,
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
        prover: RuskHttpClient,
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

    pub(crate) fn cache(&self) -> Arc<Cache> {
        let state = self.cache.lock().unwrap();

        Arc::clone(&state)
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

            let prove_req = RuskRequest::new("prove_execute", proof.to_vec());

            let proof =
                prover.call(2, "rusk", &prove_req).await.map_err(|e| {
                    ExecutionCoreError::PhoenixCircuit(e.to_string())
                })?;

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

    /// Find notes for a view key, starting from the given block height.
    pub(crate) async fn inputs(
        &self,
        index: u8,
        target: u64,
    ) -> Result<Vec<(Note, NoteOpening, BlsScalar)>, Error> {
        let vk = derive_phoenix_vk(self.store().get_seed(), index);
        let mut sk = derive_phoenix_sk(self.store().get_seed(), index);
        let pk = derive_phoenix_pk(self.store().get_seed(), index);

        let inputs: Result<Vec<_>, Error> = self
            .cache()
            .notes(&pk)?
            .into_iter()
            .map(|data| {
                let note = data.note;
                let block_height = data.block_height;
                let nullifier = note.gen_nullifier(&sk);
                let leaf = NoteLeaf { note, block_height };
                Ok((nullifier, leaf))
            })
            .collect();

        let input_notes = pick_notes(&vk, inputs?.into(), target);

        let inputs = input_notes.iter().map(|(scalar, note)| async {
            let opening = self.fetch_opening(note.as_ref()).await?;

            Ok((note.note.clone(), opening, *scalar))
        });

        // to not overwhelm the node, we buffer the requests
        // 10 in line
        let inputs = futures::stream::iter(inputs)
            .buffer_unordered(10)
            .try_collect()
            .await;

        sk.zeroize();

        inputs
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

        println!("Staking address: {}", Address::Public { addr: *pk });

        Ok(stake_data)
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
        // UNWRAP: its okay to panic here because we're closing the database
        // if there's an error we want an exception to happen
        self.cache().close().unwrap();
        let store = &mut self.store;

        // if there's sync handle we abort it
        if let Some(x) = self.sync_join_handle.as_ref() {
            x.abort();
        }

        store.inner_mut().zeroize();
    }
}
