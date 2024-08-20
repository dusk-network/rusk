// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod sync;

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use dusk_pki::ViewKey;
use dusk_plonk::prelude::*;
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    EnrichedNote, ProverClient, StakeInfo, StateClient, Transaction,
    UnprovenTransaction, POSEIDON_TREE_DEPTH,
};
use flume::Sender;
use phoenix_core::transaction::StakeData;
use phoenix_core::{Crossover, Fee, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use tokio::time::{sleep, Duration};

use std::path::Path;
use std::sync::{Arc, Mutex};

use self::sync::sync_db;

use super::block::Block;
use super::cache::Cache;

use crate::rusk::{RuskHttpClient, RuskRequest};
use crate::store::LocalStore;
use crate::Error;

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

const TRANSFER_CONTRACT: &str =
    "0100000000000000000000000000000000000000000000000000000000000000";

const STAKE_CONTRACT: &str =
    "0200000000000000000000000000000000000000000000000000000000000000";

// Sync every 3 seconds for now
const SYNC_INTERVAL_SECONDS: u64 = 3;

/// Implementation of the ProverClient trait from wallet-core
pub struct Prover {
    state: RuskHttpClient,
    prover: RuskHttpClient,
    status: fn(status: &str),
}

impl Prover {
    pub fn new(state: RuskHttpClient, prover: RuskHttpClient) -> Self {
        Prover {
            state,
            prover,
            status: |_| {},
        }
    }

    /// Sets the callback method to send status updates
    pub fn set_status_callback(&mut self, status: fn(&str)) {
        self.status = status;
    }

    pub async fn check_connection(&self) -> Result<(), reqwest::Error> {
        self.state.check_connection().await?;
        self.prover.check_connection().await
    }
}

impl ProverClient for Prover {
    /// Error returned by the prover client.
    type Error = Error;

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Transaction, Self::Error> {
        self.status("Proving tx, please wait...");
        let utx_bytes = utx.to_var_bytes();
        let prove_req = RuskRequest::new("prove_execute", utx_bytes);
        let proof_bytes = self.prover.call(2, "rusk", &prove_req).wait()?;
        self.status("Proof success!");
        let proof = Proof::from_slice(&proof_bytes).map_err(Error::Bytes)?;
        let tx = utx.clone().prove(proof);
        let tx_bytes = tx.to_var_bytes();

        self.status("Attempt to preverify tx...");
        let preverify_req = RuskRequest::new("preverify", tx_bytes.clone());
        let _ = self.state.call(2, "rusk", &preverify_req).wait()?;
        self.status("Preverify success!");

        self.status("Propagating tx...");
        let propagate_req = RuskRequest::new("propagate_tx", tx_bytes);
        let _ = self.state.call(2, "Chain", &propagate_req).wait()?;
        self.status("Transaction propagated!");

        Ok(tx)
    }

    /// Requests an STCT proof.
    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0; STCT_INPUT_SIZE];
        let mut writer = &mut buf[..];
        writer.write(&fee.to_bytes())?;
        writer.write(&crossover.to_bytes())?;
        writer.write(&value.to_bytes())?;
        writer.write(&blinder.to_bytes())?;
        writer.write(&address.to_bytes())?;
        writer.write(&signature.to_bytes())?;

        self.status("Requesting stct proof...");

        let prove_req = RuskRequest::new("prove_stct", buf.to_vec());
        let res = self.prover.call(2, "rusk", &prove_req).wait()?;

        self.status("Stct proof success!");

        let mut proof_bytes = [0u8; Proof::SIZE];
        proof_bytes.copy_from_slice(&res);

        let proof = Proof::from_bytes(&proof_bytes)?;
        Ok(proof)
    }

    /// Request a WFCT proof.
    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        let mut buf = [0; WFCT_INPUT_SIZE];
        let mut writer = &mut buf[..];
        writer.write(&commitment.to_bytes())?;
        writer.write(&value.to_bytes())?;
        writer.write(&blinder.to_bytes())?;

        self.status("Requesting wfct proof...");
        let prove_req = RuskRequest::new("prove_wfct", buf.to_vec());
        let res = self.prover.call(2, "rusk", &prove_req).wait()?;
        self.status("Wfct proof success!");

        let mut proof_bytes = [0u8; Proof::SIZE];
        proof_bytes.copy_from_slice(&res);

        let proof = Proof::from_bytes(&proof_bytes)?;
        Ok(proof)
    }
}

impl Prover {
    fn status(&self, text: &str) {
        (self.status)(text)
    }
}

/// Implementation of the StateClient trait from wallet-core
/// inner is an option because we don't want to open the db twice and lock it
/// We construct StateStore twice
pub struct StateStore {
    inner: Mutex<InnerState>,
    status: fn(&str),
    pub(crate) store: LocalStore,
}

struct InnerState {
    client: RuskHttpClient,
    cache: Arc<Cache>,
}

impl StateStore {
    /// Creates a new state instance. Should only be called once.
    pub(crate) fn new(
        client: RuskHttpClient,
        data_dir: &Path,
        store: LocalStore,
        status: fn(&str),
    ) -> Result<Self, Error> {
        let cache = Arc::new(Cache::new(data_dir, &store, status)?);
        let inner = Mutex::new(InnerState { client, cache });

        Ok(Self {
            inner,
            status,
            store,
        })
    }

    pub async fn check_connection(&self) -> Result<(), reqwest::Error> {
        let client = { self.inner.lock().unwrap().client.clone() };

        client.check_connection().await
    }

    pub async fn register_sync(
        &self,
        sync_tx: Sender<String>,
    ) -> Result<(), Error> {
        let state = self.inner.lock().unwrap();
        let status = self.status;
        let store = self.store.clone();
        let client = state.client.clone();
        let cache = Arc::clone(&state.cache);
        let sender = Arc::new(sync_tx);

        status("Starting Sync..");

        tokio::spawn(async move {
            loop {
                let status = |_: &_| {};
                let sender = Arc::clone(&sender);
                let _ = sender.send("Syncing..".to_string());

                let sync_status =
                    sync_db(&client, &store, &cache, status).await;
                let _ = match sync_status {
                    Ok(_) => sender.send("Syncing Complete".to_string()),
                    Err(e) => sender.send(format!("Error during sync:.. {e}")),
                };

                sleep(Duration::from_secs(SYNC_INTERVAL_SECONDS)).await;
            }
        });

        Ok(())
    }

    pub async fn sync(&self) -> Result<(), Error> {
        let store = self.store.clone();
        let status = self.status;
        let (cache, client) = {
            let state = self.inner.lock().unwrap();

            let cache = state.cache.clone();
            let client = state.client.clone();
            (cache, client)
        };

        sync_db(&client, &store, cache.as_ref(), status).await
    }

    pub(crate) fn cache(&self) -> Arc<Cache> {
        let state = self.inner.lock().unwrap();
        Arc::clone(&state.cache)
    }
}

/// Types that are clients of the state API.
impl StateClient for StateStore {
    /// Error returned by the node client.
    type Error = Error;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(
        &self,
        vk: &ViewKey,
    ) -> Result<Vec<EnrichedNote>, Self::Error> {
        let psk = vk.public_spend_key();
        let state = self.inner.lock().unwrap();

        Ok(state
            .cache
            .notes(&psk)?
            .into_iter()
            .map(|data| (data.note, data.height))
            .collect())
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let state = self.inner.lock().unwrap();

        self.status("Fetching anchor...");

        let anchor = state
            .client
            .contract_query::<(), 0>(TRANSFER_CONTRACT, "root", &())
            .wait()?;
        self.status("Anchor received!");
        let anchor = rkyv::from_bytes(&anchor).map_err(|_| Error::Rkyv)?;
        Ok(anchor)
    }

    /// Asks the node to return the nullifiers that already exist from the given
    /// nullifiers.
    fn fetch_existing_nullifiers(
        &self,
        _nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        Ok(vec![])
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>, Self::Error> {
        let state = self.inner.lock().unwrap();

        self.status("Fetching opening notes...");

        let data = state
            .client
            .contract_query::<_, 1024>(TRANSFER_CONTRACT, "opening", note.pos())
            .wait()?;

        self.status("Opening notes received!");

        let branch = rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;
        Ok(branch)
    }

    /// Queries the node for the amount staked by a key.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        let state = self.inner.lock().unwrap();

        self.status("Fetching stake...");

        let data = state
            .client
            .contract_query::<_, 1024>(STAKE_CONTRACT, "get_stake", pk)
            .wait()?;

        let res: Option<StakeData> =
            rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;
        self.status("Stake received!");

        let staking_address = pk.to_bytes().to_vec();
        let staking_address = bs58::encode(staking_address).into_string();
        println!("Staking address: {}", staking_address);

        // FIX_ME: proper solution should to return an Option<StakeInfo>
        // changing the trait implementation. That would reflect the state of
        // the stake contract. It would be up to the consumer to decide what to
        // do with a None
        let stake = res
            .map(
                |StakeData {
                     amount,
                     reward,
                     counter,
                 }| StakeInfo {
                    amount,
                    reward,
                    counter,
                },
            )
            .unwrap_or_default();

        Ok(stake)
    }
}

impl StateStore {
    fn status(&self, text: &str) {
        (self.status)(text)
    }
}
