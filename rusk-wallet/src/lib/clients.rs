// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Source};
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::ViewKey;
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    ProverClient, StakeInfo, StateClient, Transaction, UnprovenTransaction,
    POSEIDON_TREE_DEPTH,
};
use phoenix_core::{Crossover, Fee, Note};
use serde::Deserialize;
use std::path::Path;
use std::sync::Mutex;
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use tonic::transport::Channel;

use crate::prompt;
use crate::rusk_proto::network_client::NetworkClient;
use crate::rusk_proto::prover_client::ProverClient as GrpcProverClient;
use crate::rusk_proto::state_client::StateClient as GrpcStateClient;
use crate::rusk_proto::{
    ExecuteProverRequest, FindExistingNullifiersRequest, GetAnchorRequest,
    GetNotesOwnedByRequest, GetOpeningRequest, GetStakeRequest,
    PreverifyRequest, PropagateMessage, StctProverRequest,
    Transaction as TransactionProto, WfctProverRequest,
};

use crate::{ProverError, StateError};

use super::cache::Cache;

const STCT_INPUT_SIZE: usize = Fee::SIZE
    + Crossover::SIZE
    + u64::SIZE
    + JubJubScalar::SIZE
    + BlsScalar::SIZE
    + Signature::SIZE;

const WFCT_INPUT_SIZE: usize =
    JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

/// Implementation of the ProverClient trait from wallet-core
#[derive(Debug)]
pub struct Prover {
    client: Mutex<GrpcProverClient<Channel>>,
    state: Mutex<GrpcStateClient<Channel>>,
    network: Mutex<NetworkClient<Channel>>,
}

impl Prover {
    pub fn new(
        client: GrpcProverClient<Channel>,
        state: GrpcStateClient<Channel>,
        network: NetworkClient<Channel>,
    ) -> Self {
        Prover {
            client: Mutex::new(client),
            state: Mutex::new(state),
            network: Mutex::new(network),
        }
    }
}

impl ProverClient for Prover {
    /// Error returned by the prover client.
    type Error = ProverError;

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<Transaction, Self::Error> {
        let utx_bytes = utx.to_var_bytes();
        let msg = ExecuteProverRequest { utx: utx_bytes };
        let req = tonic::Request::new(msg);

        prompt::status("Proving tx, please wait...");
        let mut prover = self.client.lock().unwrap();
        let proof_bytes = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_execute(req).await })
        })?
        .into_inner()
        .proof;
        prompt::status("Proof success!");

        prompt::status("Attempt to preverify tx...");
        let proof =
            Proof::from_slice(&proof_bytes).map_err(ProverError::Bytes)?;
        let tx = utx.clone().prove(proof);
        let tx_bytes = tx.to_var_bytes();
        let tx_proto = TransactionProto {
            version: 1,
            r#type: 1,
            payload: tx_bytes.clone(),
        };
        let msg = PreverifyRequest { tx: Some(tx_proto) };
        let req = tonic::Request::new(msg);
        let mut state = self.state.lock().unwrap();
        block_in_place(move || {
            Handle::current()
                .block_on(async move { state.preverify(req).await })
        })?;
        prompt::status("Preverify success!");

        prompt::status("Propagating tx...");
        let msg = PropagateMessage { message: tx_bytes };
        let req = tonic::Request::new(msg);

        let mut net = self.network.lock().unwrap();
        let _ = block_in_place(move || {
            Handle::current().block_on(async move { net.propagate(req).await })
        })?;
        prompt::status("Transaction propagated!");

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

        let msg = StctProverRequest {
            circuit_inputs: buf.to_vec(),
        };
        let req = tonic::Request::new(msg);

        prompt::status("Requesting stct proof...");
        let mut prover = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_stct(req).await })
        })?
        .into_inner()
        .proof;
        prompt::status("Stct proof success!");

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

        let msg = WfctProverRequest {
            circuit_inputs: buf.to_vec(),
        };
        let req = tonic::Request::new(msg);

        prompt::status("Requesting wfct proof...");
        let mut prover = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_wfct(req).await })
        })?
        .into_inner()
        .proof;
        prompt::status("Wfct proof success!");

        let mut proof_bytes = [0u8; Proof::SIZE];
        proof_bytes.copy_from_slice(&res);

        let proof = Proof::from_bytes(&proof_bytes)?;
        Ok(proof)
    }
}

/// Implementation of the StateClient trait from wallet-core
pub struct State {
    client: Mutex<GrpcStateClient<Channel>>,
    gql_url: String,
    cache: Cache,
}

impl State {
    pub fn new(
        client: GrpcStateClient<Channel>,
        gql_url: String,
        data_dir: &Path,
    ) -> Result<Self, StateError> {
        let cache = Cache::new(data_dir)?;
        Ok(State {
            client: Mutex::new(client),
            gql_url,
            cache,
        })
    }
}
/// Types that are clients of the state API.
impl StateClient for State {
    /// Error returned by the node client.
    type Error = StateError;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(
        &self,
        _height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        prompt::status("Fetching block height...");
        let current_block = self.fetch_block_height()?;
        let psk = &vk.public_spend_key().to_bytes()[..];
        prompt::status("Fetching cached notes...");
        let cached_block_height = self.cache.last_block_height(psk);
        let cached_notes = self.cache.cached_notes(psk)?;

        let msg = GetNotesOwnedByRequest {
            height: cached_block_height,
            vk: vk.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        prompt::status("Fetching fresh notes...");
        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_notes_owned_by(req).await })
        })?
        .into_inner()
        .notes;
        prompt::status("Notes received!");
        prompt::status("Handling notes...");

        // collect notes
        let mut fresh_notes: Vec<Note> = res
            .into_iter()
            .flat_map(|n| {
                let mut bytes = [0u8; Note::SIZE];
                bytes.copy_from_slice(&n);

                let note = Note::from_bytes(&bytes).unwrap();
                let key = note.hash().to_bytes().to_vec();
                match cached_notes.contains_key(&key) {
                    true => None,
                    false => Some(note),
                }
            })
            .collect();

        prompt::status("Caching notes...");
        self.cache.persist_notes(psk, &fresh_notes[..])?;
        self.cache.persist_block_height(psk, current_block)?;
        prompt::status("Cache updated!");

        let mut ret: Vec<Note> = cached_notes.into_values().collect();
        ret.append(&mut fresh_notes);
        Ok(ret)
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let msg = GetAnchorRequest {};
        let req = tonic::Request::new(msg);

        prompt::status("Fetching anchor...");
        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_anchor(req).await })
        })?
        .into_inner()
        .anchor;
        prompt::status("Anchor received!");

        let mut bytes = [0u8; BlsScalar::SIZE];
        bytes.copy_from_slice(&res);
        let anchor = BlsScalar::from_bytes(&bytes)?;
        Ok(anchor)
    }

    /// Asks the node to return the nullifiers that already exist from the given
    /// nullifiers.
    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        let null_bytes: Vec<_> =
            nullifiers.iter().map(|s| s.to_bytes().to_vec()).collect();

        let msg = FindExistingNullifiersRequest {
            nullifiers: null_bytes,
        };
        let req = tonic::Request::new(msg);

        prompt::status("Fetching nullifiers...");
        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current().block_on(async move {
                state.find_existing_nullifiers(req).await
            })
        })?
        .into_inner()
        .nullifiers;
        prompt::status("Nullifiers received!");

        let nullifiers = res
            .iter()
            .map(|n| BlsScalar::from_slice(n))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(nullifiers)
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error> {
        let msg = GetOpeningRequest {
            note: note.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        prompt::status("Fetching opening notes...");
        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_opening(req).await })
        })?
        .into_inner()
        .branch;
        prompt::status("Opening notes received!");

        let mut src = Source::new(&res);
        let branch = Canon::decode(&mut src)?;
        Ok(branch)
    }

    /// Queries the node for the amount staked by a key.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        let msg = GetStakeRequest {
            pk: pk.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        prompt::status("Fetching stake...");
        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_stake(req).await })
        })?
        .into_inner();
        prompt::status("Stake received!");

        Ok(StakeInfo {
            value: res.value,
            eligibility: res.eligibility,
            created_at: res.created_at,
        })
    }

    fn fetch_block_height(&self) -> Result<u64, Self::Error> {
        // graphql connection
        use gql_client::Client;
        let client = Client::new(&self.gql_url);

        // define helper structs to deserialize response
        #[derive(Deserialize)]
        struct Height {
            pub height: u64,
        }
        #[derive(Deserialize)]
        struct Header {
            pub header: Height,
        }
        #[derive(Deserialize)]
        struct Blocks {
            pub blocks: Vec<Header>,
        }

        // query the db
        let query = "{blocks(last:1){header{height}}}";
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { client.query::<Blocks>(query).await })
        })?;

        // collect response
        if let Some(r) = res {
            if !r.blocks.is_empty() {
                let h = r.blocks[0].header.height;
                return Ok(h);
            }
        }
        Err(StateError::BlockHeight)
    }
}
