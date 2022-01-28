// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::{Canon, Source};
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::{Serializable, Write};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::ViewKey;
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    ProverClient, StateClient, UnprovenTransaction, POSEIDON_TREE_DEPTH,
};
use phoenix_core::{Crossover, Fee, Note};

use std::sync::Mutex;
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use tonic::transport::Channel;

use crate::rusk_proto::network_client::NetworkClient;
use crate::rusk_proto::prover_client::ProverClient as GrpcProverClient;
use crate::rusk_proto::state_client::StateClient as GrpcStateClient;
use crate::rusk_proto::PropagateMessage;
use crate::rusk_proto::{
    ExecuteProverRequest, StctProverRequest, WfctProverRequest,
};
use crate::rusk_proto::{
    GetAnchorRequest, GetNotesOwnedByRequest, GetOpeningRequest,
    GetStakeRequest,
};

use crate::Error;

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
    network: Mutex<NetworkClient<Channel>>,
}

impl Prover {
    pub fn new(
        client: GrpcProverClient<Channel>,
        network: NetworkClient<Channel>,
    ) -> Self {
        Prover {
            client: Mutex::new(client),
            network: Mutex::new(network),
        }
    }
}

impl ProverClient for Prover {
    /// Error returned by the prover client.
    type Error = Error;

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<(), Self::Error> {
        let utx_bytes = utx.to_var_bytes();
        let msg = ExecuteProverRequest { utx: utx_bytes };
        let req = tonic::Request::new(msg);

        let mut prover = self.client.lock().unwrap();
        let tx_bytes = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_execute(req).await })
        })?
        .into_inner()
        .tx;

        let msg = PropagateMessage { message: tx_bytes };
        let req = tonic::Request::new(msg);

        let mut net = self.network.lock().unwrap();
        let _ = block_in_place(move || {
            Handle::current().block_on(async move { net.propagate(req).await })
        })?;

        Ok(())
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

        let mut prover = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_stct(req).await })
        })?
        .into_inner()
        .proof;

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

        let mut prover = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { prover.prove_wfct(req).await })
        })?
        .into_inner()
        .proof;

        let mut proof_bytes = [0u8; Proof::SIZE];
        proof_bytes.copy_from_slice(&res);

        let proof = Proof::from_bytes(&proof_bytes)?;
        Ok(proof)
    }
}

/// Implementation of the StateClient trait from wallet-core
#[derive(Debug)]
pub struct State {
    client: Mutex<GrpcStateClient<Channel>>,
}

impl State {
    pub fn new(client: GrpcStateClient<Channel>) -> Self {
        State {
            client: Mutex::new(client),
        }
    }
}

/// Types that are clients of the state API.
impl StateClient for State {
    /// Error returned by the node client.
    type Error = Error;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        let msg = GetNotesOwnedByRequest {
            height,
            vk: vk.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_notes_owned_by(req).await })
        })?
        .into_inner()
        .notes;

        // collect notes
        let notes: Vec<Note> = res
            .into_iter()
            .map(|n| {
                let mut bytes = [0u8; Note::SIZE];
                bytes.copy_from_slice(&n);
                Note::from_bytes(&bytes).unwrap()
            })
            .collect();
        Ok(notes)
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let msg = GetAnchorRequest {};
        let req = tonic::Request::new(msg);

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_anchor(req).await })
        })?
        .into_inner()
        .anchor;

        let mut bytes = [0u8; BlsScalar::SIZE];
        bytes.copy_from_slice(&res);
        let anchor = BlsScalar::from_bytes(&bytes)?;
        Ok(anchor)
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

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_opening(req).await })
        })?
        .into_inner()
        .branch;

        let mut src = Source::new(&res);
        let branch = Canon::decode(&mut src)?;
        Ok(branch)
    }

    /// Queries the node the amount staked by a key and its expiration.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<(u64, u64), Self::Error> {
        let msg = GetStakeRequest {
            pk: pk.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { state.get_stake(req).await })
        })?
        .into_inner();

        Ok((res.stake, res.expiration))
    }
}
