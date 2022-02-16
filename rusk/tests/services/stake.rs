// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::*;
use canonical::{Canon, Source};
use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_schnorr::Signature;
use parking_lot::Mutex;
use rusk::services::network::{KadcastDispatcher, NetworkServer};
use rusk::services::prover::{
    ExecuteProverRequest, StctProverRequest, WfctProverRequest,
};
use rusk::services::rusk_proto::network_client::NetworkClient;
use rusk::services::rusk_proto::prover_client::ProverClient;
use rusk::services::rusk_proto::state_client::StateClient;
use rusk::services::rusk_proto::{
    PropagateMessage, Transaction as TransactionProto,
};
use rusk::services::state::{
    ExecuteStateTransitionRequest, FindExistingNullifiersRequest,
    GetAnchorRequest, GetNotesOwnedByRequest, GetOpeningRequest,
    GetStakeRequest, PreverifyRequest, StateTransitionRequest,
    VerifyStateTransitionRequest,
};
use stake_contract::Stake;

use dusk_bytes::{DeserializableSlice, Serializable, Write};

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
const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;
const MAX_NOTES: u64 = 10;
const GAS_LIMIT: u64 = 5_000_000_000;

// Function used to creates a temporary diskbackend for Rusk
fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

// Creates the Rusk initial state for the tests below
fn initial_state() -> Result<Rusk> {
    let state_id = rusk_recovery_tools::state::deploy(&testbackend())?;

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
    generate_stake(&mut rusk)?;

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

static SK: Lazy<SecretKey> = Lazy::new(|| {
    info!("Generating BLS SecretKey");
    TestStore.retrieve_sk(0).expect("Should not fail in test")
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

fn generate_stake(rusk: &mut Rusk) -> Result<()> {
    info!("Generating a stake");

    let pk = PublicKey::from(&*SK);

    let mut rusk_state = rusk.state()?;
    let mut stake = rusk_state.stake_contract()?;

    stake.push_stake(pk, Stake::with_eligibility(1_000_000_000, 0, 0), 0)?;

    info!("Updating the new stake contract state");
    unsafe {
        rusk_state.set_contract_state(&rusk_abi::stake_contract(), &stake)?;
    }

    rusk_state.finalize();

    Ok(())
}

/// Stakes an amount Dusk and produces a block with this single transaction,
/// checking the stake is set successfully. It then proceeds to withdraw the
/// stake and checking it is correctly withdrawn.
fn wallet_stake(
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    channel: tonic::transport::Channel,
    amount: u64,
) {
    // Sender psk
    let psk = SSK.public_spend_key();

    let mut rng = StdRng::seed_from_u64(0xdead);

    wallet.get_stake(0).expect("stake to not be found");

    let tx = wallet
        .stake(&mut rng, 0, 1, &psk, amount, GAS_LIMIT, 1)
        .expect("Failed to stake");
    generator_procedure(channel.clone(), &tx)
        .expect("generator procedure to succeed");

    let stake = wallet.get_stake(1).expect("stake to be found");

    assert_eq!(stake.value, amount);

    let _ = wallet.get_stake(0).expect("stake to be found");

    let tx = wallet
        .withdraw_stake(&mut rng, 0, 0, &psk, GAS_LIMIT, 1)
        .expect("Failed to withdraw stake");
    generator_procedure(channel, &tx).expect("generator procedure to succeed");

    wallet.get_stake(0).expect_err("stake is still in state");
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

    info!("First call to execute_state_transition");

    let response = client
        .execute_state_transition(ExecuteStateTransitionRequest {
            txs: vec![tx],
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
        })
        .wait()?
        .into_inner();

    assert_eq!(response.txs.len(), 2, "Should have two tx");

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

    assert_eq!(transfer_txs.len(), 1, "Only one transfer tx");
    assert_eq!(coinbase_txs.len(), 1, "One coinbase tx");

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

    /// Queries the node the amount staked by a key and its expiration.
    fn fetch_stake(&self, pk: &PublicKey) -> Result<StakeInfo, Self::Error> {
        let mut client = StateClient::new(self.channel.clone());

        let request = GetStakeRequest {
            pk: pk.to_bytes().to_vec(),
        };

        let response = client.get_stake(request).wait()?.into_inner();

        Ok(StakeInfo {
            value: response.value,
            eligibility: response.eligibility,
            created_at: response.created_at,
        })
    }

    fn fetch_block_height(&self) -> Result<u64, Self::Error> {
        Ok(BLOCK_HEIGHT)
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

    // Perform some staking actions.
    wallet_stake(&wallet, channel, 1_000_000_000);

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
