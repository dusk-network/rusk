// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod sync;

use dusk_bytes::Serializable;
use execution_core::{
    signatures::bls::PublicKey as AccountPublicKey,
    transfer::{moonlight::AccountData, phoenix::Prove, Transaction},
    Error as ExecutionCoreError,
};

use execution_core::transfer::phoenix::{Note, NoteLeaf};

use flume::Receiver;
use rues::RuesHttpClient;
use tokio::time::{sleep, Duration};
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

use super::{block::Block, cache::Cache, *};

use crate::{
    cache::NoteData,
    rusk::{RuskHttpClient, RuskRequest},
    store::LocalStore,
    Error, MAX_ADDRESSES,
};

const TRANSFER_CONTRACT: &str =
    "0100000000000000000000000000000000000000000000000000000000000000";

const STAKE_CONTRACT: &str =
    "0200000000000000000000000000000000000000000000000000000000000000";

// Sync every 3 seconds for now
const SYNC_INTERVAL_SECONDS: u64 = 3;

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
        let cfs = (0..MAX_ADDRESSES)
            .flat_map(|i| {
                let pk: PhoenixPublicKey =
                    derive_phoenix_pk(store.get_seed(), i as u8);

                [format!("{:?}", pk), format!("spent_{:?}", pk)]
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

        tokio::spawn(async move {
            loop {
                let _ = sync_tx.send("Syncing..".to_string());

                let _ = match sync_db(&client, &cache, &store, status).await {
                    Ok(_) => sync_tx.send("Syncing Complete".to_string()),
                    Err(e) => sync_tx.send(format!("Error during sync:.. {e}")),
                };

                sleep(Duration::from_secs(SYNC_INTERVAL_SECONDS)).await;
            }
        });

        Ok(())
    }

    pub async fn sync(&self) -> Result<(), Error> {
        sync_db(&self.client, &self.cache(), &self.store, self.status).await
    }

    /// Requests that a node prove the given transaction and later propagates it
    /// Skips writing the proof for non phoenix transactions
    pub fn prove_and_propagate(
        &self,
        tx: Transaction,
    ) -> Result<Transaction, Error> {
        let status = self.status;
        let prover = &self.prover;
        let mut tx = tx;

        if let Transaction::Phoenix(utx) = &mut tx {
            let status = self.status;
            let proof = utx.proof();

            status("Attempt to prove tx...");

            let prove_req = RuskRequest::new("prove_execute", proof.to_vec());

            let proof =
                prover.call(2, "rusk", &prove_req).wait().map_err(|e| {
                    ExecutionCoreError::PhoenixCircuit(e.to_string())
                })?;

            utx.set_proof(proof);

            status("Proving sucesss!");
        }

        let tx_bytes = tx.to_var_bytes();

        status("Attempt to preverify tx...");
        let _ = self
            .client
            .call("transactions", None, "preverify", &tx_bytes)
            .wait()?;
        status("Preverify success!");

        status("Propagating tx...");
        let _ = self
            .client
            .call("transactions", None, "propagate", &tx_bytes)
            .wait()?;
        status("Transaction propagated!");

        Ok(tx)
    }

    /// Find notes for a view key, starting from the given block height.
    pub(crate) fn inputs(
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
                let block_height = data.height;
                let nullifier = note.gen_nullifier(&sk);
                let leaf = NoteLeaf { note, block_height };
                Ok((nullifier, leaf))
            })
            .collect();

        let inputs = pick_notes(&vk, inputs?.into(), target)
            .into_iter()
            .map(|(scalar, note)| {
                let opening = self.fetch_opening(note.as_ref())?;

                Ok((note.note.clone(), opening, *scalar))
            })
            .collect();

        sk.zeroize();

        inputs
    }

    pub(crate) fn fetch_account(
        &self,
        pk: &AccountPublicKey,
    ) -> Result<AccountData, Error> {
        let status = self.status;
        status("Fetching account-data...");

        let account = self
            .client
            .contract_query::<_, _, 1024>(TRANSFER_CONTRACT, "account", pk)
            .wait()?;
        let account = rkyv::from_bytes(&account).map_err(|_| Error::Rkyv)?;
        status("account-data received!");

        Ok(account)
    }

    pub(crate) fn fetch_notes(
        &self,
        pk: &PhoenixPublicKey,
    ) -> Result<Vec<NoteData>, Error> {
        self.cache()
            .notes(pk)?
            .into_iter()
            .map(|data| {
                Ok(NoteData {
                    note: data.note,
                    height: data.height,
                })
            })
            .collect()
    }

    /// Fetch the current root of the state.
    pub(crate) fn fetch_root(&self) -> Result<BlsScalar, Error> {
        let status = self.status;
        status("Fetching root...");

        let root = self
            .client
            .contract_query::<(), _, 0>(TRANSFER_CONTRACT, "root", &())
            .wait()?;
        status("root received!");
        let root = rkyv::from_bytes(&root).map_err(|_| Error::Rkyv)?;
        Ok(root)
    }

    /// Queries the node for the amount staked by a key.
    pub(crate) fn fetch_stake(
        &self,
        pk: &AccountPublicKey,
    ) -> Result<Option<StakeData>, Error> {
        let status = self.status;
        status("Fetching stake...");

        let data = self
            .client
            .contract_query::<_, _, 1024>(STAKE_CONTRACT, "get_stake", pk)
            .wait()?;

        let res: Option<StakeData> =
            rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;
        status("Stake received!");

        let staking_address = pk.to_bytes().to_vec();
        let staking_address = bs58::encode(staking_address).into_string();
        println!("Staking address: {}", staking_address);

        Ok(res)
    }

    pub(crate) fn store(&self) -> &LocalStore {
        &self.store
    }

    pub(crate) fn fetch_chain_id(&self) -> Result<u8, Error> {
        let status = self.status;
        status("Fetching chain_id...");

        let data = self
            .client
            .contract_query::<_, _, { u8::SIZE }>(
                TRANSFER_CONTRACT,
                "chain_id",
                &(),
            )
            .wait()?;

        let res: u8 = rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;
        status("Chain id received!");

        Ok(res)
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(&self, note: &Note) -> Result<NoteOpening, Error> {
        let status = self.status;
        status("Fetching opening notes...");

        let data = self
            .client
            .contract_query::<_, _, 1024>(
                TRANSFER_CONTRACT,
                "opening",
                note.pos(),
            )
            .wait()?;

        status("Opening notes received!");

        let branch = rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;
        Ok(branch)
    }
}
