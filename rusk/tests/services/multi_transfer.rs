// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::*;
use canonical::{Canon, Source};
use dusk_bls12_381_sign::PublicKey;
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_schnorr::Signature;
use parking_lot::Mutex;
use rusk::services::network::{KadcastDispatcher, NetworkServer};
use rusk::services::prover::ExecuteProverRequest;
use rusk::services::rusk_proto::network_client::NetworkClient;
use rusk::services::rusk_proto::prover_client::ProverClient;
use rusk::services::rusk_proto::state_client::StateClient;
use rusk::services::rusk_proto::{
    PropagateMessage, Transaction as TransactionProto,
};
use rusk::services::state::{
    ExecuteStateTransitionRequest, FindExistingNullifiersRequest,
    GetAnchorRequest, GetNotesOwnedByRequest, GetOpeningRequest,
    PreverifyRequest, StateTransitionRequest, VerifyStateTransitionRequest,
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
// This is purposefully chose to be low to
const BLOCK_GAS_LIMIT: u64 = 600_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;

// Function used to creates a temporary diskbackend for Rusk
fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

// Creates the Rusk initial state for the tests below
fn initial_state() -> Result<Rusk> {
    let state_id = rusk_recovery_tools::state::deploy(false, &testbackend())?;

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
        transfer.get_note(4)?.is_none(),
        "Expect to have only 3 notes",
    );

    rusk.state()?.finalize();

    Ok(rusk)
}

static STATE_LOCK: Lazy<Mutex<Rusk>> = Lazy::new(|| {
    let rusk = initial_state().expect("Failed to create initial state");
    Mutex::new(rusk)
});

static SSK_0: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

static SSK_1: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(1).expect("Should not fail in test")
});

static SSK_2: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey");
    TestStore.retrieve_ssk(2).expect("Should not fail in test")
});

fn generate_notes(rusk: &mut Rusk) -> Result<()> {
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let psk_0 = SSK_0.public_spend_key();
    let psk_1 = SSK_1.public_spend_key();
    let psk_2 = SSK_2.public_spend_key();

    let note_0 = Note::transparent(&mut rng, &psk_0, INITIAL_BALANCE);
    let note_1 = Note::transparent(&mut rng, &psk_1, INITIAL_BALANCE);
    let note_2 = Note::transparent(&mut rng, &psk_2, INITIAL_BALANCE);

    let mut rusk_state = rusk.state()?;
    let mut transfer = rusk_state.transfer_contract()?;

    transfer.push_note(BLOCK_HEIGHT, note_0)?;
    transfer.push_note(BLOCK_HEIGHT, note_1)?;
    transfer.push_note(BLOCK_HEIGHT, note_2)?;

    transfer.update_root()?;

    info!("Updating the new transfer contract state");
    unsafe {
        rusk_state
            .set_contract_state(&rusk_abi::transfer_contract(), &transfer)?;
    }

    rusk_state.finalize();

    Ok(())
}

/// Executes three different transactions in the same block, expecting only two
/// to be included due to exceeding th block gas limit
fn wallet_transfer(
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    channel: tonic::transport::Channel,
    amount: u64,
) {
    // Sender psk
    let psk_0 = SSK_0.public_spend_key();
    let psk_1 = SSK_1.public_spend_key();
    let psk_2 = SSK_2.public_spend_key();

    let refunds = vec![psk_0, psk_1, psk_2];

    // Generate a receiver psk
    let receiver = wallet
        .public_spend_key(3)
        .expect("Failed to get public spend key");

    let mut rng = StdRng::seed_from_u64(0xdead);
    let nonce = BlsScalar::random(&mut rng);

    let initial_balance_0 =
        wallet.get_balance(0).expect("Failed to get the balance");
    let initial_balance_1 =
        wallet.get_balance(1).expect("Failed to get the balance");
    let initial_balance_2 =
        wallet.get_balance(2).expect("Failed to get the balance");

    // Check the senders initial balance is correct
    assert_eq!(
        initial_balance_0, INITIAL_BALANCE,
        "Wrong initial balance for the sender"
    );
    assert_eq!(
        initial_balance_1, INITIAL_BALANCE,
        "Wrong initial balance for the sender"
    );
    assert_eq!(
        initial_balance_2, INITIAL_BALANCE,
        "Wrong initial balance for the sender"
    );

    // Check the receiver initial balance is zero
    assert_eq!(
        wallet.get_balance(3).expect("Failed to get the balance"),
        0,
        "Wrong initial balance for the receiver"
    );

    let mut txs = Vec::with_capacity(3);

    for i in 0..3 {
        let tx = wallet
            .transfer(
                &mut rng,
                i,
                &refunds[i as usize],
                &receiver,
                amount,
                1_000_000_000,
                1,
                nonce,
            )
            .expect("Failed to transfer");
        txs.push(tx);
    }

    generator_procedure(channel, txs.clone())
        .expect("generator procedure to succeed");

    // Check the receiver's balance is changed accordingly
    assert_eq!(
        wallet.get_balance(3).expect("Failed to get the balance"),
        2 * amount,
        "Wrong resulting balance for the receiver"
    );

    let final_balance_0 =
        wallet.get_balance(0).expect("Failed to get the balance");
    let fee_0 = txs[0].fee();
    let fee_0 = fee_0.gas_limit * fee_0.gas_price;

    let final_balance_1 =
        wallet.get_balance(1).expect("Failed to get the balance");
    let fee_1 = txs[1].fee();
    let fee_1 = fee_1.gas_limit * fee_1.gas_price;

    assert!(
        initial_balance_0 - amount - fee_0 <= final_balance_0,
        "Final sender balance {} should be greater or equal than {}",
        final_balance_0,
        initial_balance_0 - amount - fee_0
    );

    assert!(
        initial_balance_0 - amount >= final_balance_0,
        "Final sender balance {} should be lesser or equal than {}",
        final_balance_0,
        initial_balance_0 - amount
    );

    assert!(
        initial_balance_1 - amount - fee_1 <= final_balance_1,
        "Final sender balance {} should be greater or equal than {}",
        final_balance_1,
        initial_balance_1 - amount - fee_1
    );

    assert!(
        initial_balance_1 - amount >= final_balance_1,
        "Final sender balance {} should be lesser or equal than {}",
        final_balance_1,
        initial_balance_1 - amount
    );

    // Check the discarded transaction didn't change the balance
    assert_eq!(
        wallet.get_balance(2).expect("Failed to get the balance"),
        initial_balance_2,
        "Wrong resulting balance for discarded TX sender"
    );
}

/// Executes the procedure a block generator will go through to generate a block
/// including two transfer transactions and discarding the last (3rd) due to it
/// exceeding the block gas limit, and then checking the outputs are as
/// expected.
fn generator_procedure(
    channel: tonic::transport::Channel,
    txs: Vec<Transaction>,
) -> Result<()> {
    let mut client = StateClient::new(channel);

    let protos: Vec<_> = txs
        .iter()
        .map(|tx| TransactionProto {
            version: 1,
            r#type: 1,
            payload: tx.to_var_bytes(),
        })
        .collect();

    for (i, tx) in txs.iter().enumerate() {
        let tx_hash = tx.hash();

        let response = client
            .preverify(PreverifyRequest {
                tx: Some(protos[i].clone()),
            })
            .wait()?
            .into_inner();

        assert_eq!(
            response.tx_hash,
            tx_hash.to_bytes().to_vec(),
            "Hash mismatch"
        );
    }

    let response = client
        .execute_state_transition(ExecuteStateTransitionRequest {
            txs: protos,
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
        })
        .wait()?
        .into_inner();

    assert_eq!(response.txs.len(), 3, "Should have three txs");

    let transfer_txs: Vec<_> = response
        .txs
        .iter()
        .filter(|etx| etx.tx.as_ref().unwrap().r#type == 1)
        .collect();

    let coinbase_txs: Vec<_> = response
        .txs
        .iter()
        .filter(|etx| etx.tx.as_ref().unwrap().r#type == 0)
        .collect();

    assert_eq!(transfer_txs.len(), 2, "Two transfer txs");
    assert_eq!(coinbase_txs.len(), 1, "One coinbase tx");

    let execute_state_root = response.state_root.clone();

    info!(
        "execute_state_transition new root: {:?}",
        hex::encode(&execute_state_root)
    );

    let mut txs = vec![];
    txs.extend(transfer_txs);
    txs.extend(coinbase_txs);

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
        })
        .wait()?;

    let response = client
        .accept(StateTransitionRequest {
            txs,
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
            state_root: execute_state_root.clone(),
        })
        .wait()?
        .into_inner();

    assert_eq!(response.txs.len(), 2, "Should have two txs");

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
            .map(|n| Note::from_slice(n).map_err(Error::Serialization))
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

    fn fetch_block_height(&self) -> Result<u64, Self::Error> {
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
pub async fn multi_transfer() -> Result<()> {
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
    let state = rusk.state()?;
    let original_root = state.root();

    info!("Original Root: {:?}", hex::encode(original_root));

    wallet_transfer(&wallet, channel, 1_000);

    // Check the state's root is changed from the original one
    let new_root = state.root();
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
