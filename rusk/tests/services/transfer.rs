// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::keys::BLS_SK;
use crate::common::*;

use canonical::{Canon, Source};
use dusk_bls12_381_sign::PublicKey;
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_schnorr::Signature;
use futures::StreamExt;
use parking_lot::Mutex;
use rusk::services::network::{KadcastDispatcher, NetworkServer};
use rusk::services::prover::ExecuteProverRequest;
use rusk::services::state::{
    ExecuteStateTransitionRequest, ExecuteStateTransitionResponse,
    FindExistingNullifiersRequest, GetAnchorRequest, GetNotesRequest,
    GetOpeningRequest, PreverifyRequest, StateTransitionRequest,
    VerifyStateTransitionRequest,
};
use rusk_schema::network_client::NetworkClient;
use rusk_schema::prover_client::ProverClient;
use rusk_schema::state_client::StateClient;
use rusk_schema::{PropagateMessage, Transaction as TransactionProto};

use dusk_bytes::{DeserializableSlice, Serializable};

use once_cell::sync::Lazy;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::error::Error;
use rusk::{Result, Rusk};

use microkelvin::{BackendCtor, DiskBackend};

use tempfile::{tempdir, TempDir};
use tracing::info;

use tonic::transport::Server;

use rusk::services::prover::{ProverServer, RuskProver};
use rusk::services::state::StateServer;

use dusk_wallet_core::{
    self as wallet, StakeInfo, Store, Transaction, UnprovenTransaction,
};

use phoenix_core::{Crossover, Fee, Note};

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_plonk::proof_system::Proof;

use dusk_poseidon::tree::PoseidonBranch;
use rusk_abi::POSEIDON_TREE_DEPTH;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;
const MAX_NOTES: u64 = 10;

static TEMP_DIR: Lazy<TempDir> = Lazy::new(|| tempdir().unwrap());

fn ephemeral() -> Result<DiskBackend, microkelvin::PersistError> {
    let dir = TEMP_DIR.path();
    let mut dir = dir.to_path_buf();
    dir.push("state");
    DiskBackend::new(dir)
}

// Function used to creates a temporary diskbackend for Rusk
fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(ephemeral)
}

// Creates the Rusk initial state for the tests below
fn initial_state() -> Result<Rusk> {
    let state_id = rusk_recovery_tools::state::deploy_state(TEMP_DIR.path())?;

    let mut rusk = Rusk::builder(testbackend).id(state_id).build()?;

    let state = rusk.state()?;
    let transfer = state.transfer_contract()?;

    assert!(
        transfer.get_note(0)?.is_some(),
        "Expect to have one note at the genesis state",
    );

    assert!(
        transfer.get_note(1)?.is_none(),
        "Expect to have ONLY one note at the genesis state",
    );

    generate_notes(&mut rusk)?;

    let transfer = state.transfer_contract()?;

    assert!(transfer.get_note(1)?.is_some(), "Expect to have more notes",);
    assert!(
        transfer.get_note(MAX_NOTES + 1)?.is_none(),
        "Expect to have only {} notes",
        MAX_NOTES
    );

    rusk.state()?.finalize();

    Ok(rusk)
}

static STATE_LOCK: Lazy<Mutex<Rusk>> = Lazy::new(|| {
    let rusk = initial_state().expect("Failed to create initial state");
    Mutex::new(rusk)
});

static SSK: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

static EXECUTE_STATE_TRANSITION_RESPONSE: Lazy<
    Mutex<Option<ExecuteStateTransitionResponse>>,
> = Lazy::new(|| {
    info!("Setup the coinbase only for the first execute_state_transition");
    Mutex::new(None)
});

fn generate_notes(rusk: &mut Rusk) -> Result<()> {
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let psk = SSK.public_spend_key();

    let note = Note::transparent(&mut rng, &psk, INITIAL_BALANCE);

    let mut rusk_state = rusk.state()?;
    let mut transfer = rusk_state.transfer_contract()?;

    for _ in 0..MAX_NOTES {
        transfer.push_note(BLOCK_HEIGHT, note)?;
    }

    transfer.update_root()?;

    info!("Updating the new transfer contract state");
    unsafe {
        rusk_state
            .set_contract_state(&rusk_abi::transfer_contract(), &transfer)?;
    }

    rusk_state.finalize();

    Ok(())
}

/// Transacts between two accounts on the in the same wallet and produces a
/// block with a single transaction, checking balances are transferred
/// successfully.
fn wallet_transfer(
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    channel: tonic::transport::Channel,
    amount: u64,
) {
    // Sender psk
    let psk = SSK.public_spend_key();

    // Generate a receiver psk
    let receiver = wallet
        .public_spend_key(1)
        .expect("Failed to get public spend key");

    let mut rng = StdRng::seed_from_u64(0xdead);
    let nonce = BlsScalar::random(&mut rng);

    // Store the sender initial balance
    let sender_initial_balance = wallet
        .get_balance(0)
        .expect("Failed to get the balance")
        .value;

    // Check the sender's initial balance is correct
    assert_eq!(
        sender_initial_balance,
        INITIAL_BALANCE * MAX_NOTES,
        "Wrong initial balance for the sender"
    );

    // Check the receiver initial balance is zero
    assert_eq!(
        wallet
            .get_balance(1)
            .expect("Failed to get the balance")
            .value,
        0,
        "Wrong initial balance for the receiver"
    );

    // Execute a transfer
    let tx = wallet
        .transfer(
            &mut rng,
            0,
            &psk,
            &receiver,
            amount,
            1_000_000_000,
            2,
            nonce,
        )
        .expect("Failed to transfer");
    info!("Tx: {}", hex::encode(tx.to_var_bytes()));
    info!("Tx ID: {}", hex::encode(tx.hash().to_bytes()));
    generator_procedure(channel, &tx).expect("generator procedure to succeed");

    // Check the receiver's balance is changed accordingly
    assert_eq!(
        wallet
            .get_balance(1)
            .expect("Failed to get the balance")
            .value,
        amount,
        "Wrong resulting balance for the receiver"
    );

    // Check the sender's balance is changed accordingly
    let sender_final_balance = wallet
        .get_balance(0)
        .expect("Failed to get the balance")
        .value;
    let fee = tx.fee();
    let fee = fee.gas_limit * fee.gas_price;

    assert!(
        sender_initial_balance - amount - fee <= sender_final_balance,
        "Final sender balance {} should be greater or equal than {}",
        sender_final_balance,
        sender_initial_balance - amount - fee
    );

    assert!(
        sender_initial_balance - amount >= sender_final_balance,
        "Final sender balance {} should be lesser or equal than {}",
        sender_final_balance,
        sender_initial_balance - amount
    );
}

/// Executes the procedure a block generator will go through to generate a block
/// including a single transfer transaction, checking the outputs are as
/// expected.
fn generator_procedure(
    channel: tonic::transport::Channel,
    tx: &Transaction,
) -> Result<()> {
    let tx_hash = tx.hash();
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

    let generator = PublicKey::from(&*BLS_SK);

    // Since the purpose of the test is simulate a real transaction between
    // different nodes, the 1st execute state transition response is cached
    // and reused in the subsequent calls. This is done since only one
    // node is actually involved in calling the `execute_state_transition`,
    // where the rest is using the data received from the network.
    let mut previous_response = EXECUTE_STATE_TRANSITION_RESPONSE.lock();
    let response = match &*previous_response {
        None => {
            info!("First call to execute_state_transition");

            let response = client
                .execute_state_transition(ExecuteStateTransitionRequest {
                    txs: vec![tx],
                    block_height: BLOCK_HEIGHT,
                    block_gas_limit: BLOCK_GAS_LIMIT,
                    generator: generator.to_bytes().to_vec(),
                })
                .wait()?
                .into_inner();
            *previous_response = Some(response.clone());
            response
        }
        Some(previous_response) => {
            info!("Use response from the first execute_state_transition");

            previous_response.clone()
        }
    };

    assert_eq!(response.txs.len(), 1, "Should have only one tx");

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
    fn fetch_notes(&self, vk: &ViewKey) -> Result<Vec<Note>, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = tonic::Request::new(GetNotesRequest {
            height: 0,
            vk: vk.to_bytes().to_vec(),
        });

        let stream = client.get_notes(request).wait()?.into_inner();

        Ok(stream
            .map(|response| {
                let response = response.expect("Stream item should be Ok()");
                Note::from_slice(&response.note).expect("Note should be valid")
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

    fn fetch_stake(&self, _pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
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
    // Setup the logger and gRPC channels
    let (channel, incoming) = setup().await;

    // Get the Rusk's instance to pass to the `StateServer`
    let rusk = STATE_LOCK.lock();

    let state = StateServer::new(rusk.clone());
    let prover = ProverServer::new(RuskProver::default());
    let dispatcher = KadcastDispatcher::default();
    let mut kadcast_recv = dispatcher.subscribe();
    let network = NetworkServer::new(dispatcher);

    // Drop the Rusk instance so it can be re-acquired later on
    drop(rusk);

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

    let rusk = STATE_LOCK.lock();
    let mut state = rusk.state()?;
    let original_root = state.root();

    info!("Original Root: {:?}", hex::encode(original_root));

    wallet_transfer(&wallet, channel.clone(), 1_000);

    // Check the state's root is changed from the original one
    let new_root = state.root();
    info!(
        "New root after the 1st transfer: {:?}",
        hex::encode(new_root)
    );
    assert_ne!(original_root, new_root, "Root should have changed");

    // Revert the state
    state.revert();

    // Check the state's root is back to the original one
    info!("Root after reset: {:?}", hex::encode(state.root()));
    assert_eq!(original_root, state.root(), "Root be the same again");

    wallet_transfer(&wallet, channel, 1_000);

    // Check the state's root is back to the original one
    info!(
        "New root after the 2nd transfer: {:?}",
        hex::encode(state.root())
    );
    assert_eq!(
        new_root,
        state.root(),
        "Root is the same compare to the first transfer"
    );

    let recv = kadcast_recv.try_recv();
    let (tx, _, h) = recv.expect("Transaction has not been locally propagated");
    info!("Tx Wire Message {}", hex::encode(tx));
    assert_eq!(h, 0, "Transaction locally propagated with wrong height");

    Ok(())
}
