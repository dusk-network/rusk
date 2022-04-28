// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::common::keys::*;
use crate::common::*;

use canonical::{Canon, Sink};
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use futures::StreamExt;
use parking_lot::Mutex;
use phoenix_core::Note;
use rusk::services::state::{
    GetAnchorRequest, GetNotesOwnedByRequest, GetNotesRequest,
    GetOpeningRequest, GetStakeRequest,
};
use rusk_schema::state_client::StateClient;

use dusk_bytes::{DeserializableSlice, Serializable};

use once_cell::sync::Lazy;
use rand::prelude::*;
use rand::rngs::StdRng;
use rusk::{Result, Rusk};

use microkelvin::{BackendCtor, DiskBackend};

use tempfile::TempDir;
use tempfile::tempdir;
use tracing::{info, trace};

use tonic::transport::Server;

use rusk::services::state::StateServer;
use stake_contract::Stake;


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

static STATE_LOCK: Lazy<Mutex<Rusk>> = Lazy::new(|| {
    let state_id = rusk_recovery_tools::state::deploy_state (TEMP_DIR.path())
        .expect("Failed to deploy state");

    let rusk = Rusk::builder(testbackend)
        .id(state_id)
        .build()
        .expect("Error creating Rusk Instance");

    Mutex::new(rusk)
});

const BLOCK_HEIGHT: u64 = 1;

#[allow(deprecated)]
fn fetch_note(rusk: &Rusk) -> Result<Option<Note>> {
    info!("Fetching the first note from the state");
    let vk = SSK.view_key();
    let (notes, _) = rusk.state()?.fetch_notes(BLOCK_HEIGHT, &vk)?;

    if notes.len() == 1 {
        trace!("Note found");
        Ok(Some(notes[0]))
    } else {
        trace!("Note not found");
        Ok(None)
    }
}

fn generate_note(rusk: &mut Rusk) -> Result<Option<Note>> {
    info!("Generating a note");
    let mut rng = StdRng::seed_from_u64(0xdead);

    let psk = SSK.public_spend_key();

    let initial_balance = 1_000_000_000;

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
    rusk_state.finalize();

    fetch_note(rusk)
}

fn generate_stake(rusk: &mut Rusk) -> Result<(BlsPublicKey, Stake)> {
    info!("Generating a stake");

    let pk = BlsPublicKey::from(&*BLS_SK);

    let mut rusk_state = rusk.state()?;
    let mut stake_contract = rusk_state.stake_contract()?;

    let stake = Stake::with_eligibility(0xdead, 0, 0);
    stake_contract.insert_stake(pk, stake.clone())?;

    info!("Updating the new stake contract state");
    unsafe {
        rusk_state
            .set_contract_state(&rusk_abi::stake_contract(), &stake_contract)?;
    }
    rusk_state.finalize();

    Ok((pk, stake))
}

fn get_note(rusk: &mut Rusk) -> Result<Option<Note>> {
    info!("Try to obtain the first note from the state");
    fetch_note(rusk).or_else(|_| generate_note(rusk))
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_get_notes() -> Result<()> {
    let (channel, incoming) = setup().await;

    let rusk_server = STATE_LOCK.lock().clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let note = get_note(&mut STATE_LOCK.lock())?;

    let vk = SSK.view_key();

    assert!(note.is_some(), "One note expected to be in the state");
    let note = note.unwrap();

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

    assert_eq!(notes.len(), 1, "There should be one note in the state");
    assert_eq!(notes[0], note, "Received note should be the generated note");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[allow(deprecated)]
pub async fn test_fetch_notes() -> Result<()> {
    let (channel, incoming) = setup().await;

    let rusk_server = STATE_LOCK.lock().clone();

    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let note = get_note(&mut STATE_LOCK.lock())?;

    let vk = SSK.view_key();

    assert!(note.is_some(), "One note expected to be in the state");

    let mut client = StateClient::new(channel.clone());

    let request = tonic::Request::new(GetNotesOwnedByRequest {
        height: BLOCK_HEIGHT,
        vk: vk.to_bytes().to_vec(),
    });

    let response = client.get_notes_owned_by(request).await?.into_inner();

    assert_eq!(response.notes.len(), 1, "Expected 1 note");
    assert_eq!(
        response.height, BLOCK_HEIGHT,
        "Expected latest block height to be {}",
        BLOCK_HEIGHT
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
pub async fn test_fetch_anchor() -> Result<()> {
    let (channel, incoming) = setup().await;

    let rusk_server = STATE_LOCK.lock().clone();
    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let anchor = {
        let mut rusk = STATE_LOCK.lock();
        let note = get_note(&mut rusk)?;

        assert!(note.is_some(), "One note expected to be in the state");

        let rusk_state = rusk.state()?;
        rusk_state.fetch_anchor()?
    };

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

    let rusk_server = STATE_LOCK.lock().clone();
    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let (note, opening) = {
        let mut rusk = STATE_LOCK.lock();
        let note = get_note(&mut rusk)?;

        assert!(note.is_some(), "One note expected to be in the state");
        let note = note.unwrap();

        let rusk_state = rusk.state()?;
        let opening = rusk_state.fetch_opening(&note)?;

        (note, opening)
    };

    let mut client = StateClient::new(channel.clone());

    let request = tonic::Request::new(GetOpeningRequest {
        note: note.to_bytes().to_vec(),
    });

    let response = client.get_opening(request).await?;
    let branch = response.into_inner().branch;

    const PAGE_SIZE: usize = 1024 * 64;
    let mut bytes = [0u8; PAGE_SIZE];
    let mut sink = Sink::new(&mut bytes[..]);
    opening.encode(&mut sink);
    let len = opening.encoded_len();
    let opening = (&bytes[..len]).to_vec();

    assert_eq!(branch, opening, "Expected same branch");

    Ok(())
}
#[tokio::test(flavor = "multi_thread")]
pub async fn test_fetch_stake() -> Result<()> {
    let (channel, incoming) = setup().await;

    let rusk_server = STATE_LOCK.lock().clone();
    tokio::spawn(async move {
        Server::builder()
            .add_service(StateServer::new(rusk_server))
            .serve_with_incoming(incoming)
            .await
    });

    let (pk, stake) = generate_stake(&mut STATE_LOCK.lock())?;

    let mut client = StateClient::new(channel);

    let request = tonic::Request::new(GetStakeRequest {
        pk: pk.to_bytes().to_vec(),
    });

    let response = client.get_stake(request).await?.into_inner();

    let response_amount = response
        .amount
        .map(|amount| (amount.value, amount.eligibility));

    assert_eq!(stake.amount(), response_amount.as_ref());
    assert_eq!(stake.reward(), response.reward);
    assert_eq!(stake.counter(), response.counter);

    Ok(())
}
