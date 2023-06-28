// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_bytes::Error::InvalidData;
use dusk_bytes::{DeserializableSlice, Serializable, Write};
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_merkle::poseidon::Opening as PoseidonOpening;
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    self as wallet, StakeInfo, Store, Transaction, UnprovenTransaction,
};
use futures::StreamExt;
use once_cell::sync::Lazy;
use phoenix_core::{Crossover, Fee, Note};
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::error::Error;
use rusk::services::network::{KadcastDispatcher, NetworkServer};
use rusk::services::prover::{
    ExecuteProverRequest, StctProverRequest, WfctProverRequest,
};
use rusk::services::prover::{ProverServer, RuskProver};
use rusk::services::state::StateServer;
use rusk::services::state::{
    ExecuteStateTransitionRequest, FindExistingNullifiersRequest,
    GetAnchorRequest, GetNotesRequest, GetOpeningRequest, GetStakeRequest,
    PreverifyRequest, StateTransitionRequest, VerifyStateTransitionRequest,
};
use rusk::{Result, Rusk};
use rusk_abi::POSEIDON_TREE_DEPTH;
use rusk_recovery_tools::state::MINIMUM_STAKE;
use rusk_schema::network_client::NetworkClient;
use rusk_schema::prover_client::ProverClient;
use rusk_schema::state_client::StateClient;
use rusk_schema::{PropagateMessage, Transaction as TransactionProto};
use tempfile::tempdir;
use tonic::transport::Server;
use tracing::info;

use crate::common::state::new_state;
use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const GAS_LIMIT: u64 = 10_000_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/stake.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

static SSK: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

static SK: Lazy<SecretKey> = Lazy::new(|| {
    info!("Generating BLS SecretKey");
    TestStore.retrieve_sk(0).expect("Should not fail in test")
});

/// Stakes an amount Dusk and produces a block with this single transaction,
/// checking the stake is set successfully. It then proceeds to withdraw the
/// stake and checking it is correctly withdrawn.
fn wallet_stake(
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    channel: tonic::transport::Channel,
    value: u64,
) {
    // Sender psk
    let psk = SSK.public_spend_key();

    let mut rng = StdRng::seed_from_u64(0xdead);

    wallet.get_stake(0).expect("stake to not be found");

    let tx = wallet
        .stake(&mut rng, 0, 2, &psk, value, GAS_LIMIT, 1)
        .expect("Failed to stake");
    generator_procedure(channel.clone(), &tx)
        .expect("generator procedure to succeed");

    let stake = wallet.get_stake(2).expect("stake to be found");
    let stake_value = stake.amount.expect("stake should have an amount").0;

    assert_eq!(stake_value, value);

    let _ = wallet.get_stake(0).expect("stake to be found");

    let tx = wallet
        .unstake(&mut rng, 0, 0, &psk, GAS_LIMIT, 1)
        .expect("Failed to unstake");
    generator_procedure(channel.clone(), &tx)
        .expect("generator procedure to succeed");

    let stake = wallet.get_stake(0).expect("stake should still be state");
    assert_eq!(stake.amount, None);

    let tx = wallet
        .withdraw(&mut rng, 0, 1, &psk, GAS_LIMIT, 1)
        .expect("failed to withdraw reward");
    generator_procedure(channel, &tx).expect("generator procedure to succeed");

    let stake = wallet.get_stake(1).expect("stake should still be state");
    assert_eq!(stake.reward, 0);
}

/// Executes the procedure a block generator will go through to generate a block
/// including a single transfer transaction, checking the outputs are as
/// expected.
fn generator_procedure(
    channel: tonic::transport::Channel,
    tx: &Transaction,
) -> Result<()> {
    let tx_hash_input_bytes = tx.to_hash_input_bytes();
    let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

    let tx_bytes = tx.to_var_bytes();

    let tx = TransactionProto {
        version: 1,
        r#type: 1,
        payload: tx_bytes,
    };

    let mut client = StateClient::new(channel);

    let response = client
        .preverify(PreverifyRequest {
            tx: Some(tx.clone()),
        })
        .wait()?
        .into_inner();

    assert_eq!(
        response.tx_hash,
        tx_hash.to_bytes().to_vec(),
        "Hash mismatch"
    );

    info!("First call to execute_state_transition");

    let generator = PublicKey::from(&*SK);

    let response = client
        .execute_state_transition(ExecuteStateTransitionRequest {
            txs: vec![tx],
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
            generator: generator.to_bytes().to_vec(),
        })
        .wait()?
        .into_inner();

    assert_eq!(response.txs.len(), 1, "Should have one tx");

    let transfer_txs: Vec<_> = response
        .txs
        .iter()
        .filter(|etx| etx.tx.as_ref().unwrap().r#type == 1)
        .collect();

    assert_eq!(transfer_txs.len(), 1, "Only one transfer tx");

    assert_eq!(
        transfer_txs[0].tx_hash,
        tx_hash.to_bytes().to_vec(),
        "Hash mismatch"
    );

    let execute_state_root = response.state_root.clone();

    info!(
        "execute_state_transition new root: {:?}",
        hex::encode(&execute_state_root)
    );

    let mut txs = vec![];
    txs.extend(transfer_txs);

    let txs: Vec<_> = txs
        .iter()
        .map(|tx| tx.tx.as_ref().unwrap())
        .cloned()
        .collect();

    client
        .verify_state_transition(VerifyStateTransitionRequest {
            txs: txs.clone(),
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
            generator: generator.to_bytes().to_vec(),
        })
        .wait()?;

    let response = client
        .accept(StateTransitionRequest {
            txs,
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
            state_root: execute_state_root.clone(),
            generator: generator.to_bytes().to_vec(),
        })
        .wait()?
        .into_inner();

    assert_eq!(response.txs.len(), 1, "Should have one tx");

    let accept_state_root = response.state_root;
    info!("accept new root: {:?}", hex::encode(&accept_state_root));

    assert_eq!(
        accept_state_root, execute_state_root,
        "Root should be equal"
    );

    Ok(())
}

#[derive(Debug, Clone)]
struct TestStore;

impl Store for TestStore {
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
        vk: &ViewKey,
    ) -> Result<Vec<(Note, u64)>, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetNotesRequest {
            height: 0,
            vk: vk.to_bytes().to_vec(),
        });

        let stream = client.get_notes(request).wait()?.into_inner();

        Ok(stream
            .map(|response| {
                let response = response.expect("Stream item should be Ok()");
                let note = Note::from_slice(&response.note)
                    .expect("Note should be valid");
                (note, response.height)
            })
            .collect()
            .wait())
    }

    /// Fetch the current anchor of the state.
    fn fetch_anchor(&self) -> Result<BlsScalar, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetAnchorRequest {});

        let response = client.get_anchor(request).wait()?;

        BlsScalar::from_slice(&response.into_inner().anchor)
            .map_err(Error::Serialization)
    }

    fn fetch_existing_nullifiers(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<Vec<BlsScalar>, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = FindExistingNullifiersRequest {
            nullifiers: nullifiers
                .iter()
                .map(|n| n.to_bytes().to_vec())
                .collect(),
        };

        let response = client
            .find_existing_nullifiers(request)
            .wait()?
            .into_inner();

        let nullifiers = response
            .nullifiers
            .into_iter()
            .map(|n| BlsScalar::from_slice(&n).map_err(Error::Serialization))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(nullifiers)
    }

    /// Queries the node to find the opening for a specific note.
    fn fetch_opening(
        &self,
        note: &Note,
    ) -> Result<PoseidonOpening<(), POSEIDON_TREE_DEPTH, 4>, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetOpeningRequest {
            note: note.to_bytes().to_vec(),
        });

        let response = client.get_opening(request).wait()?;
        let response = response.into_inner();

        Ok(rkyv::from_bytes(&response.branch)
            .map_err(|_| Error::Serialization(InvalidData))?)
    }

    /// Queries the node the amount staked by a key and its expiration.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = GetStakeRequest {
            pk: pk.to_bytes().to_vec(),
        };

        let response = client.get_stake(request).wait()?.into_inner();

        let amount = response
            .amount
            .map(|amount| (amount.value, amount.eligibility));

        Ok(StakeInfo {
            amount,
            reward: response.reward,
            counter: response.counter,
        })
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
    ) -> Result<Transaction, Self::Error> {
        let mut client = ProverClient::new(self.channel.clone());

        let response = client
            .prove_execute(ExecuteProverRequest {
                utx: utx.to_var_bytes(),
            })
            .wait()?
            .into_inner();

        let proof =
            Proof::from_slice(&response.proof).map_err(Error::Serialization)?;
        let tx = utx.clone().prove(proof);

        let propagate_request = tonic::Request::new(PropagateMessage {
            message: tx.to_var_bytes(),
        });

        let mut network_client = NetworkClient::new(self.channel.clone());
        let _ = network_client.propagate(propagate_request).wait()?;

        Ok(tx)
    }

    /// Requests an STCT proof.
    #[allow(unused_must_use)]
    fn request_stct_proof(
        &self,
        fee: &Fee,
        crossover: &Crossover,
        value: u64,
        blinder: JubJubScalar,
        address: BlsScalar,
        signature: Signature,
    ) -> Result<Proof, Self::Error> {
        let size = Fee::SIZE
            + Crossover::SIZE
            + u64::SIZE
            + JubJubScalar::SIZE
            + BlsScalar::SIZE
            + Signature::SIZE;

        let mut circuit_inputs = vec![0; size];
        let mut writer = &mut circuit_inputs[..];

        writer.write(&fee.to_bytes());
        writer.write(&crossover.to_bytes());
        writer.write(&value.to_bytes());
        writer.write(&blinder.to_bytes());
        writer.write(&address.to_bytes());
        writer.write(&signature.to_bytes());

        let mut client = ProverClient::new(self.channel.clone());

        let response = client
            .prove_stct(StctProverRequest { circuit_inputs })
            .wait()?
            .into_inner();

        let proof =
            Proof::from_slice(&response.proof).map_err(Error::Serialization)?;
        Ok(proof)
    }

    /// Request a WFCT proof.
    #[allow(unused_must_use)]
    fn request_wfct_proof(
        &self,
        commitment: JubJubAffine,
        value: u64,
        blinder: JubJubScalar,
    ) -> Result<Proof, Self::Error> {
        let size = JubJubAffine::SIZE + u64::SIZE + JubJubScalar::SIZE;

        let mut circuit_inputs = vec![0; size];
        let mut writer = &mut circuit_inputs[..];

        writer.write(&commitment.to_bytes());
        writer.write(&value.to_bytes());
        writer.write(&blinder.to_bytes());

        let mut client = ProverClient::new(self.channel.clone());

        let response = client
            .prove_wfct(WfctProverRequest { circuit_inputs })
            .wait()?
            .into_inner();

        let proof =
            Proof::from_slice(&response.proof).map_err(Error::Serialization)?;
        Ok(proof)
    }
}

#[tokio::test(flavor = "multi_thread")]
pub async fn stake() -> Result<()> {
    // Setup the logger and gRPC channels
    let (channel, incoming) = setup().await;

    let tmp = tempdir().expect("Creating temporary directory should succeed");
    let rusk =
        initial_state(&tmp).expect("Creating initial state should succeed");

    let state = StateServer::new(rusk.clone());
    let prover = ProverServer::new(RuskProver::default());
    let dispatcher = KadcastDispatcher::default();
    let mut kadcast_recv = dispatcher.subscribe();
    let network = NetworkServer::new(dispatcher);

    // Build and Spawn the server
    tokio::spawn(async move {
        Server::builder()
            .add_service(state)
            .add_service(prover)
            .add_service(network)
            .serve_with_incoming(incoming)
            .await
    });

    // Create a wallet
    let wallet = wallet::Wallet::new(
        TestStore,
        TestStateClient {
            channel: channel.clone(),
        },
        TestProverClient {
            channel: channel.clone(),
        },
    );

    let original_root = rusk.state_root();

    info!("Original Root: {:?}", hex::encode(original_root));

    // Perform some staking actions.
    wallet_stake(&wallet, channel, MINIMUM_STAKE);

    // Check the state's root is changed from the original one
    let new_root = rusk.state_root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    let recv = kadcast_recv.try_recv();
    let (_, _, h) = recv.expect("Transaction has not been locally propagated");
    assert_eq!(h, 0, "Transaction locally propagated with wrong height");

    Ok(())
}
