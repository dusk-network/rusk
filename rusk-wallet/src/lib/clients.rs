// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytes::{BytesMut, BufMut};
use dusk_wallet_core::{UnprovenTransaction, POSEIDON_TREE_DEPTH};
use dusk_jubjub::{BlsScalar, JubJubAffine, JubJubScalar};
use dusk_pki::{ViewKey, PublicKey};
use dusk_schnorr::Signature;
use dusk_plonk::prelude::Proof;
use dusk_poseidon::tree::PoseidonBranch;
use phoenix_core::{Note, Crossover, Fee};
use dusk_bytes::Serializable;
use canonical::{Canon, Source};

use std::sync::Mutex;
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use tonic::transport::Channel;

use crate::errors::CliError;

use crate::rusk_proto::state_client::{StateClient};
use crate::rusk_proto::stake_client::{StakeClient};
use crate::rusk_proto::prover_client::{ProverClient};
use crate::rusk_proto::network_client::{NetworkClient};

use crate::rusk_proto::{ExecuteProverRequest, StctProverRequest, WfctProverRequest};
use crate::rusk_proto::{GetNotesOwnedByRequest, GetAnchorRequest, GetOpeningRequest};
use crate::rusk_proto::{FindStakeRequest};
use crate::rusk_proto::{PropagateMessage};

/// Implementation of the ProverClient trait from wallet-core
#[derive(Debug)]
pub(crate) struct Prover {
    client: Mutex<ProverClient<Channel>>,
    network: Mutex<NetworkClient<Channel>>,
}

impl Prover {

    pub fn new(client: ProverClient<Channel>, network: NetworkClient<Channel>) -> Self {
        Prover{
            client: Mutex::new(client),
            network: Mutex::new(network),
        }
    }

}

impl dusk_wallet_core::ProverClient for Prover {

    /// Error returned by the prover client.
    type Error = CliError;

    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(&self, utx: &UnprovenTransaction) -> Result<(), Self::Error> {

        let utx_bytes = utx.to_var_bytes();
        let msg = ExecuteProverRequest{
            utx: utx_bytes,
        };
        let req = tonic::Request::new(msg);

        let mut prover = self.client.lock().unwrap();
        let tx_bytes = block_in_place(move || {
            Handle::current().block_on(async move {
                prover.prove_execute(req).await
            })
        })?.into_inner().tx;

        // todo: encode message
        let msg = PropagateMessage{
            message: tx_bytes,
        };
        let req = tonic::Request::new(msg);

        let mut net = self.network.lock().unwrap();
        let _ = block_in_place(move || {
            Handle::current().block_on(async move {
                net.propagate(req).await
            })
        })?;

        Ok(())
    }

    /// Requests an STCT proof.
    fn request_stct_proof(&self, fee: &Fee, crossover: &Crossover, value: u64, blinder: JubJubScalar, 
        address: BlsScalar, signature: Signature) -> Result<Proof, Self::Error> {

            let mut buf = BytesMut::new();
            buf.put_slice(&fee.to_bytes());
            buf.put_slice(&crossover.to_bytes());
            buf.put_slice(&value.to_bytes());
            buf.put_slice(&blinder.to_bytes());
            buf.put_slice(&address.to_bytes());
            buf.put_slice(&signature.to_bytes());

            let msg = StctProverRequest {
                circuit_inputs: buf.to_vec(),
            };
            let req = tonic::Request::new(msg);

            let mut prover = self.client.lock().unwrap();
            let res = block_in_place(move || {
                Handle::current().block_on(async move {
                    prover.prove_stct(req).await
                })
            })?.into_inner().proof;

            let mut proof_bytes = [0u8; Proof::SIZE];
            proof_bytes.copy_from_slice(&res);

            let proof = Proof::from_bytes(&proof_bytes)?;
            Ok(proof)

    }

    /// Request a WFCT proof.
    fn request_wfct_proof( &self, commitment: JubJubAffine, value: u64, blinder: JubJubScalar) -> Result<Proof, Self::Error> {

        let mut buf = BytesMut::new();
        buf.put_slice(&commitment.to_bytes());
        buf.put_slice(&value.to_bytes());
        buf.put_slice(&blinder.to_bytes());

        let msg = WfctProverRequest {
            circuit_inputs: buf.to_vec(),
        };
        let req = tonic::Request::new(msg);

        let mut prover = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current().block_on(async move {
                prover.prove_wfct(req).await
            })
        })?.into_inner().proof;

        let mut proof_bytes = [0u8; Proof::SIZE];
        proof_bytes.copy_from_slice(&res);

        let proof = Proof::from_bytes(&proof_bytes)?;
        Ok(proof)

    }

}


/// Implementation of the StateClient trait from wallet-core
#[derive(Debug)]
pub(crate) struct State {
    client: Mutex<StateClient<Channel>>,
    stake: Mutex<StakeClient<Channel>>
}

impl State {

    pub fn new(client: StateClient<Channel>, stake: StakeClient<Channel>) -> Self {
        State{
            client: Mutex::new(client),
            stake: Mutex::new(stake),
        }
    }

}

/// Types that are clients of the state API.
impl dusk_wallet_core::StateClient for State {

    /// Error returned by the node client.
    type Error = CliError;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(&self, height: u64, vk: &ViewKey) -> Result<Vec<Note>, Self::Error> {

        let msg = GetNotesOwnedByRequest{
            height: height,
            vk: vk.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current().block_on(async move {
                state.get_notes_owned_by(req).await
            })
        })?.into_inner().notes;

        // collect notes
        let notes: Vec<Note> = res.into_iter().map(|n| {
            let mut bytes = [0u8; Note::SIZE];
            bytes.copy_from_slice(&n);
            let note = Note::from_bytes(&bytes).unwrap();
            note
        }).collect();
        Ok(notes)

    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {

        let msg = GetAnchorRequest{};
        let req = tonic::Request::new(msg);

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current().block_on(async move {
                state.get_anchor(req).await
            })
        })?.into_inner().anchor;

        let mut bytes = [0u8; BlsScalar::SIZE];
        bytes.copy_from_slice(&res);
        let anchor = BlsScalar::from_bytes(&bytes)?;
        Ok(anchor)

    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(&self, note: &Note) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error> {

        let msg = GetOpeningRequest{
            note: note.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        let mut state = self.client.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current().block_on(async move {
                state.get_opening(req).await
            })
        })?.into_inner().branch;

        let mut src = Source::new(&res);
        let branch = Canon::decode(&mut src)?;
        Ok(branch)

    }

    /// Queries the node the amount staked by a key and its expiration.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<(u64, u32), Self::Error> {

        let msg = FindStakeRequest{
            pk: pk.to_bytes().to_vec(),
        };
        let req = tonic::Request::new(msg);

        let mut stake = self.stake.lock().unwrap();
        let res = block_in_place(move || {
            Handle::current().block_on(async move {
                stake.find_stake(req).await
            })
        })?.into_inner().stakes;
        todo!()

    }
}