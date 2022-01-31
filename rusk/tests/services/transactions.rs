// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::*;
use canonical::{Canon, Source};
use dusk_bls12_381_sign::PublicKey;
use dusk_pki::{Ownable, SecretSpendKey, ViewKey};
use dusk_schnorr::{PublicKeyPair, Signature};
use parking_lot::Mutex;
use rusk::services::prover::ExecuteProverRequest;
use rusk::services::rusk_proto::prover_client::ProverClient;
use rusk::services::rusk_proto::state_client::StateClient;
use rusk::services::rusk_proto::Transaction as TransactionProto;
use rusk::services::state::{
    GetAnchorRequest, GetNotesOwnedByRequest, GetOpeningRequest,
    PreverifyRequest,
};

use dusk_bytes::{DeserializableSlice, Serializable};

use once_cell::sync::Lazy;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::error::Error;
use rusk::{Result, Rusk};

use microkelvin::{BackendCtor, DiskBackend};

use tracing::info;

use tonic::transport::Server;

use rusk::services::pki::{KeysServer, RuskKeys};
use rusk::services::prover::{ProverServer, RuskProver};
use rusk::services::state::StateServer;

use dusk_wallet_core::{
    self as wallet, Store, Transaction, UnprovenTransaction,
};

use phoenix_core::{Crossover, Fee, Note};

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_plonk::proof_system::Proof;

use dusk_poseidon::tree::PoseidonBranch;
use rusk_abi::POSEIDON_TREE_DEPTH;

pub fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

static STATE_LOCK: Lazy<Mutex<Rusk>> = Lazy::new(|| {
    let state_id = rusk_recovery_tools::state::deploy(&testbackend())
        .expect("Failed to deploy state");

    let mut rusk = Rusk::builder(testbackend)
        .id(state_id)
        .build()
        .expect("Error creating Rusk Instance");

    generate_note(&mut rusk).expect("Failed to generate note");

    Mutex::new(rusk)
});

const BLOCK_HEIGHT: u64 = 1;

pub static SSK: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

fn generate_note(rusk: &mut Rusk) -> Result<()> {
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let psk = SSK.public_spend_key();

    let initial_balance = 1_000_000_000_000;

    let note = Note::transparent(&mut rng, &psk, initial_balance);

    let mut rusk_state = rusk.state()?;
    let mut transfer = rusk_state.transfer_contract()?;

    transfer.push_note(BLOCK_HEIGHT, note)?;
    transfer.update_root()?;

    info!("Updating the new transfer contract state");
    unsafe {
        rusk_state
            .set_contract_state(&rusk_abi::transfer_contract(), &transfer)?;
    }
    rusk.persist(&mut rusk_state)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct TestStore;

impl wallet::Store for TestStore {
    type Error = ();

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok([0; 64])
    }
}

#[derive(Debug, Clone)]
struct TestStateClient {
    channel: tonic::transport::Channel,
}

impl wallet::StateClient for TestStateClient {
    type Error = Error;

    /// Find notes for a view key, starting from the given block height.
    fn fetch_notes(
        &self,
        height: u64,
        vk: &ViewKey,
    ) -> Result<Vec<Note>, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetNotesOwnedByRequest {
            height,
            vk: vk.to_bytes().to_vec(),
        });

        let response = client.get_notes_owned_by(request).wait()?;

        response
            .into_inner()
            .notes
            .iter()
            .map(|n| Note::from_slice(&n).map_err(Error::Serialization))
            .collect()
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetAnchorRequest {});

        let response = client.get_anchor(request).wait()?;

        BlsScalar::from_slice(&response.into_inner().anchor)
            .map_err(Error::Serialization)
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonBranch<POSEIDON_TREE_DEPTH>, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetOpeningRequest {
            note: note.to_bytes().to_vec(),
        });

        let response = client.get_opening(request).wait()?;
        let response = response.into_inner();

        let mut source = Source::new(&response.branch);
        Ok(PoseidonBranch::decode(&mut source)?)
    }

    /// Queries the node the amount staked by a key and its expiration.
    fn fetch_stake(&self, _pk: &PublicKey) -> Result<(u64, u64), Self::Error> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
struct TestProverClient {
    channel: tonic::transport::Channel,
}

impl wallet::ProverClient for TestProverClient {
    type Error = Error;
    /// Requests that a node prove the given transaction and later propagates it
    fn compute_proof_and_propagate(
        &self,
        utx: &UnprovenTransaction,
    ) -> Result<(), Self::Error> {
        let mut client = ProverClient::new(self.channel.clone());

        let request = tonic::Request::new(ExecuteProverRequest {
            utx: utx.to_var_bytes(),
        });

        let note = utx.inputs()[0].note();
        let sk_r = SSK.sk_r(note.stealth_address());
        let pkp: PublicKeyPair = sk_r.into();

        if !utx.inputs()[0].signature().verify(&pkp, utx.hash()) {
            panic!("Schnorr failed");
        }

        let response = client.prove_execute(request).wait()?;

        let response = response.into_inner();

        let tx_bytes = response.tx;
        let tx =
            Transaction::from_slice(&tx_bytes).map_err(Error::Serialization)?;
        let tx_hash = tx.hash();

        let tx = Some(TransactionProto {
            version: 1,
            r#type: 1,
            payload: tx_bytes,
        });

        let request = tonic::Request::new(PreverifyRequest { tx });

        let mut client = StateClient::new(self.channel.clone());

        let response = client.preverify(request).wait()?;

        let response = response.into_inner();

        assert_eq!(
            response.tx_hash,
            tx_hash.to_bytes().to_vec(),
            "Hash mismatch"
        );

        Ok(())
    }

    /// Requests an STCT proof.
    fn request_stct_proof(
        &self,
        _fee: &Fee,
        _crossover: &Crossover,
        _value: u64,
        _blinder: JubJubScalar,
        _address: BlsScalar,
        _signature: Signature,
    ) -> Result<Proof, Self::Error> {
        unimplemented!();
    }

    /// Request a WFCT proof.
    fn request_wfct_proof(
        &self,
        _commitment: JubJubAffine,
        _value: u64,
        _blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        unimplemented!();
    }
}

#[tokio::test(flavor = "multi_thread")]
pub async fn wallet_grpc() -> Result<()> {
    let rusk = STATE_LOCK.lock();

    let (channel, incoming) = setup().await;

    let keys = KeysServer::new(RuskKeys::default());
    let state = StateServer::new(rusk.clone());
    let prover = ProverServer::new(RuskProver::default());

    drop(rusk);

    tokio::spawn(async move {
        Server::builder()
            .add_service(keys)
            .add_service(state)
            .add_service(prover)
            .serve_with_incoming(incoming)
            .await
    });

    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            channel: channel.clone(),
        },
        TestProverClient { channel },
    );

    let psk = SSK.public_spend_key();
    let receiver = wallet
        .public_spend_key(1)
        .expect("Failed to get public spend key");

    let mut rng = StdRng::seed_from_u64(0xdead);
    let nonce = BlsScalar::random(&mut rng);

    println!("Balance before key 0: {:?}", wallet.get_balance(0));
    println!("Balance before key 1: {:?}", wallet.get_balance(1));

    wallet
        .transfer(&mut rng, 0, &psk, &receiver, 1_000, 1_000_000_000, 1, nonce)
        .expect("Failed to transfer");

    println!("Balance after key 0: {:?}", wallet.get_balance(0));
    println!("Balance after key 1: {:?}", wallet.get_balance(1));

    Ok(())
}
