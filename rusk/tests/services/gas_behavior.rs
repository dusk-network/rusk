// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Error::InvalidData;
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_pki::ViewKey;
use dusk_plonk::proof_system::Proof;
use dusk_schnorr::Signature;
use dusk_wallet_core::{
    self as wallet, StakeInfo, Store, Transaction, UnprovenTransaction,
};
use futures::StreamExt;
use phoenix_core::{Crossover, Fee, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::error::Error;
use rusk::services::network::{KadcastDispatcher, NetworkServer};
use rusk::services::prover::ExecuteProverRequest;
use rusk::services::prover::{ProverServer, RuskProver};
use rusk::services::state::StateServer;
use rusk::services::state::{
    ExecuteStateTransitionRequest, FindExistingNullifiersRequest,
    GetAnchorRequest, GetNotesRequest, GetOpeningRequest, PreverifyRequest,
    StateTransitionRequest, VerifyStateTransitionRequest,
};
use rusk::{Result, Rusk, GAS_PER_INPUT};
use rusk_abi::{ContractId, POSEIDON_TREE_DEPTH, TRANSFER_CONTRACT};
use rusk_schema::network_client::NetworkClient;
use rusk_schema::prover_client::ProverClient;
use rusk_schema::state_client::StateClient;
use rusk_schema::{PropagateMessage, Transaction as TransactionProto};
use tempfile::tempdir;
use tonic::transport::Server;
use tracing::info;

use crate::common::keys::BLS_SK;
use crate::common::state::new_state;
use crate::common::*;

const BLOCK_HEIGHT: u64 = 1;
const BLOCK_GAS_LIMIT: u64 = 1_000_000_000;
const INITIAL_BALANCE: u64 = 10_000_000_000;

const GAS_LIMIT_0: u64 = 2_000_000;
const GAS_LIMIT_1: u64 = 100_000_000;

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/gas-behavior.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

const SENDER_INDEX_0: u64 = 0;
const SENDER_INDEX_1: u64 = 1;

fn make_transactions(
    wallet: &wallet::Wallet<TestStore, TestStateClient, TestProverClient>,
    channel: tonic::transport::Channel,
) {
    // We will refund the transaction to ourselves.
    let refund_0 = wallet
        .public_spend_key(SENDER_INDEX_0)
        .expect("Getting a public spend key should succeed");

    let initial_balance_0 = wallet
        .get_balance(SENDER_INDEX_0)
        .expect("Getting initial balance should succeed")
        .value;

    // We will refund the transaction to ourselves.
    let refund_1 = wallet
        .public_spend_key(SENDER_INDEX_1)
        .expect("Getting a public spend key should succeed");

    let initial_balance_1 = wallet
        .get_balance(SENDER_INDEX_1)
        .expect("Getting initial balance should succeed")
        .value;

    assert_eq!(
        initial_balance_0, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );
    assert_eq!(
        initial_balance_1, INITIAL_BALANCE,
        "The sender should have the given initial balance"
    );

    let mut rng = StdRng::seed_from_u64(0xdead);

    // The first transaction will be a `wallet.execute` to a contract that is
    // not deployed. This will produce an error in call execution and should
    // consume all the gas provided.
    let tx_0 = wallet
        .execute(
            &mut rng,
            ContractId::from([0x42; 32]),
            String::from("nonsense"),
            (),
            SENDER_INDEX_0,
            &refund_0,
            GAS_LIMIT_0,
            1,
        )
        .expect("Making the transaction should succeed");

    // The second transaction will also be a `wallet.execute`, but this time to
    // the transfer contract, querying for the root of the tree. This will be
    // tested for gas cost.
    let tx_1 = wallet
        .execute(
            &mut rng,
            TRANSFER_CONTRACT,
            String::from("root"),
            (),
            SENDER_INDEX_1,
            &refund_1,
            GAS_LIMIT_1,
            1,
        )
        .expect("Making the transaction should succeed");

    generator_procedure(channel, tx_0.clone(), tx_1.clone())
        .expect("generator procedure should succeed");

    let final_balance_0 = wallet
        .get_balance(SENDER_INDEX_0)
        .expect("Getting final balance should succeed")
        .value;

    let final_balance_1 = wallet
        .get_balance(SENDER_INDEX_1)
        .expect("Getting final balance should succeed")
        .value;

    // The first transaction should consume all gas given, while the second one
    // should consume a little more due to the root query.
    assert_eq!(
        final_balance_0,
        initial_balance_0 - GAS_LIMIT_0,
        "Transaction should consume all the gas"
    );

    assert!(
        final_balance_1 < initial_balance_1 - GAS_PER_INPUT,
        "Transaction should consume more gas than just for one input"
    );
    assert!(
        final_balance_1 > GAS_LIMIT_1,
        "Transaction should consume less gas than all given"
    );
}

/// Executes the procedure a block generator will go through to generate a block
/// including the contract call in the block.
fn generator_procedure(
    channel: tonic::transport::Channel,
    tx_0: Transaction,
    tx_1: Transaction,
) -> Result<()> {
    let mut client = StateClient::new(channel);

    let proto_0 = TransactionProto {
        version: 1,
        r#type: 1,
        payload: tx_0.to_var_bytes(),
    };
    let proto_1 = TransactionProto {
        version: 1,
        r#type: 1,
        payload: tx_1.to_var_bytes(),
    };

    // Run pre-verification
    for (tx, proto) in [&tx_0, &tx_1].into_iter().zip([&proto_0, &proto_1]) {
        let preverify_response = client
            .preverify(PreverifyRequest {
                tx: Some(proto.clone()),
            })
            .wait()?
            .into_inner();

        let tx_hash_input_bytes = tx.to_hash_input_bytes();
        let tx_hash = rusk_abi::hash(tx_hash_input_bytes);

        assert_eq!(
            preverify_response.tx_hash,
            tx_hash.to_bytes().to_vec(),
            "Transaction hashes should match in pre-verification response"
        );
    }

    // Execute state transition
    let generator = PublicKey::from(&*BLS_SK);

    let response = client
        .execute_state_transition(ExecuteStateTransitionRequest {
            txs: vec![proto_0, proto_1],
            block_height: BLOCK_HEIGHT,
            block_gas_limit: BLOCK_GAS_LIMIT,
            generator: generator.to_bytes().to_vec(),
        })
        .wait()?
        .into_inner();

    assert_eq!(
        response.txs.len(),
        2,
        "Both transactions should be included in the block"
    );

    let transfer_txs: Vec<_> = response
        .txs
        .iter()
        .filter(|etx| etx.tx.as_ref().unwrap().r#type == 1)
        .collect();

    let execute_state_root = response.state_root.clone();

    info!(
        "execute_state_transition new root: {}",
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

    assert_eq!(
        response.txs.len(),
        2,
        "There should be two transactions in the block"
    );

    let accept_state_root = response.state_root;
    info!("accept new root: {}", hex::encode(&accept_state_root));

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
                let height = response.height;
                (note, height)
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
pub async fn erroring_tx_charged_full() -> Result<()> {
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

    make_transactions(&wallet, channel);

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
