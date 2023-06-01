// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;

use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::{SecretSpendKey, ViewKey};
use dusk_wallet_core::Store;
use futures::StreamExt;
use once_cell::sync::Lazy;
use phoenix_core::Note;
use rusk::services::state::StateServer;
use rusk::services::state::{
    GetAnchorRequest, GetNotesRequest, GetOpeningRequest, GetStakeRequest,
};
use rusk::{Result, Rusk};
use rusk_schema::state_client::StateClient;
use rusk_schema::GetProvisionersRequest;
use tempfile::tempdir;
use tonic::transport::Server;
use tracing::info;

use crate::common::state::new_state;
use crate::common::*;

const BLOCK_HEIGHT: u64 = 0;

static SSK: Lazy<SecretSpendKey> = Lazy::new(|| {
    info!("Generating SecretSpendKey #0");
    TestStore.retrieve_ssk(0).expect("Should not fail in test")
});

static SK: Lazy<SecretKey> = Lazy::new(|| {
    info!("Generating BLS SecretKey");
    TestStore.retrieve_sk(0).expect("Should not fail in test")
});

#[derive(Debug, Clone)]
struct TestStore;

impl Store for TestStore {
    type Error = ();

    fn get_seed(&self) -> Result<[u8; 64], Self::Error> {
        Ok([0; 64])
    }
}

// Creates the Rusk initial state for the tests below
fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!("../config/state_service.toml"))
        .expect("Cannot deserialize config");

    new_state(dir, &snapshot)
}

fn get_notes_owned_by(rusk: &Rusk, vk: &ViewKey) -> Vec<Note> {
    rusk.leaves_in_range(0..1)
        .expect("Getting leaves should work")
        .into_iter()
        .map(|leaf| leaf.note)
        .filter_map(|note| vk.owns(&note).then_some(note))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_get_notes() -> Result<()> {
    let (channel, incoming) = setup().await;

    let tmp = tempdir().expect("Creating temporary directory should succeed");

    let rusk =
        initial_state(&tmp).expect("Creating initial state should succeed");
    let rusk_server = rusk.clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let vk = SSK.view_key();

    let notes = get_notes_owned_by(&rusk, &vk);

    assert_eq!(notes.len(), 1, "Only one note expected to be in the state");
    let note = notes[0];

    let mut client = StateClient::new(channel.clone());

    // request with a view key
    let request = tonic::Request::new(GetNotesRequest {
        height: BLOCK_HEIGHT,
        vk: vk.to_bytes().to_vec(),
    });

    let mut stream = client.get_notes(request).await?.into_inner();
    let mut notes = vec![];

    while let Some(response) = stream.next().await {
        let response = response.expect("The response should be successful");
        notes.push(Note::from_slice(&response.note)?);
    }

    assert_eq!(notes.len(), 1, "There should be one note in the state");
    assert_eq!(notes[0], note, "Received note should be the generated note");

    // request without a view key
    let request = tonic::Request::new(GetNotesRequest {
        height: BLOCK_HEIGHT,
        vk: vec![],
    });

    let mut stream = client.get_notes(request).await?.into_inner();
    let mut notes = vec![];

    while let Some(response) = stream.next().await {
        let response = response.expect("The response should be successful");
        notes.push(Note::from_slice(&response.note)?);
    }

    assert_eq!(notes.len(), 3, "There should be three notes in the state");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_fetch_anchor() -> Result<()> {
    let (channel, incoming) = setup().await;

    let tmp = tempdir().expect("Creating temporary directory should succeed");
    let rusk =
        initial_state(&tmp).expect("Creating initial state should succeed");

    let rusk_server = rusk.clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let anchor = rusk
        .tree_root()
        .expect("Querying the tree root should succeed");

    let mut client = StateClient::new(channel.clone());

    let request = tonic::Request::new(GetAnchorRequest {});

    let response = client.get_anchor(request).await?;
    let fetched_anchor = response.into_inner().anchor;

    assert_eq!(
        &anchor.to_bytes()[..],
        &fetched_anchor[..],
        "Expected same anchor"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_fetch_opening() -> Result<()> {
    let (channel, incoming) = setup().await;

    let tmp = tempdir().expect("Creating temporary directory should succeed");
    let rusk =
        initial_state(&tmp).expect("Creating initial state should succeed");

    let rusk_server = rusk.clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let vk = SSK.view_key();

    let notes = get_notes_owned_by(&rusk, &vk);
    assert_eq!(notes.len(), 1, "One note of ours should be in the state");

    let (note, opening) = {
        let note = notes[0];

        let opening = rusk
            .tree_opening(*note.pos())?
            .expect("The opening should exist");

        (note, opening)
    };

    let mut client = StateClient::new(channel.clone());

    let request = tonic::Request::new(GetOpeningRequest {
        note: note.to_bytes().to_vec(),
    });

    let response = client.get_opening(request).await?;

    let branch = response.into_inner().branch;
    let opening = opening.to_bytes().to_vec();

    assert_eq!(branch, opening, "Expected same branch");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_fetch_stake() -> Result<()> {
    let (channel, incoming) = setup().await;

    let tmp = tempdir().expect("Creating temporary directory should succeed");
    let rusk =
        initial_state(&tmp).expect("Creating initial state should succeed");

    let rusk_server = rusk.clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let pk = PublicKey::from(&*SK);
    let stake = rusk.stake(pk)?.expect("A stake should exist for this key");

    let mut client = StateClient::new(channel);

    let request = tonic::Request::new(GetStakeRequest {
        pk: pk.to_bytes().to_vec(),
    });

    let response = client.get_stake(request).await?.into_inner();

    let response_amount = response
        .amount
        .map(|amount| (amount.value, amount.eligibility));

    assert_eq!(stake.amount, response_amount);
    assert_eq!(stake.reward, response.reward);
    assert_eq!(stake.counter, response.counter);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_get_provisioners() -> Result<()> {
    let (channel, incoming) = setup().await;

    let tmp = tempdir().expect("Creating temporary directory should succeed");
    let rusk =
        initial_state(&tmp).expect("Creating initial state should succeed");

    let rusk_server = rusk.clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let mut client = StateClient::new(channel);

    let request = tonic::Request::new(GetProvisionersRequest {});

    let response = client.get_provisioners(request).await?.into_inner();

    let response_amount = response.provisioners.len();

    // `state_service.toml` is configured to have more than 8 provisioners
    // (actually 9) to properly test the query_seq with MAX=8
    assert_eq!(9, response_amount);

    Ok(())
}
