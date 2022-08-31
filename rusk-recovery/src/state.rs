// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::provisioners::PROVISIONERS;
use crate::theme::Theme;

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use http_req::request;
use lazy_static::lazy_static;
use microkelvin::{Backend, BackendCtor, DiskBackend, Persistence};
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::*;
use rusk_vm::{Contract, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract, MINIMUM_STAKE};
use std::error::Error;
use std::io::{Cursor, Read};
use std::{fs, io};
use tracing::info;
use tracing::log::error;
use transfer_contract::TransferContract;
use zip::ZipArchive;

/// Amount of the note inserted in the genesis state.
const GENESIS_DUSK: Dusk = dusk(1_000.0);

/// Faucet note value.
const FAUCET_DUSK: Dusk = dusk(1_000_000_000.0);

lazy_static! {
    pub static ref DUSK_KEY: PublicSpendKey = {
        let bytes = include_bytes!("../dusk.psk");
        PublicSpendKey::from_bytes(bytes)
            .expect("faucet should have a valid key")
    };
    pub static ref FAUCET_KEY: PublicSpendKey = {
        let bytes = include_bytes!("../faucet.psk");
        PublicSpendKey::from_bytes(bytes)
            .expect("faucet should have a valid key")
    };
}

fn diskbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(|| {
        let dir = rusk_profile::get_rusk_state_dir()
            .expect("Failed to get Rusk profile directory");

        fs::remove_dir_all(&dir)
            .or_else(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .expect("Failed to clean up Network State directory");

        fs::create_dir_all(&dir)
            .expect("Failed to create Network State directory");

        DiskBackend::new(dir)
    })
}

/// Creates a new transfer contract state with a single note in it - ownership
/// of Dusk Network. If `testnet` is true an additional note - ownership of the
/// faucet address - is added to the state.
fn genesis_transfer(testnet: bool) -> TransferContract {
    let mut transfer = TransferContract::default();
    let mut rng = StdRng::seed_from_u64(0xdead_beef);

    let note = Note::transparent(&mut rng, &DUSK_KEY, GENESIS_DUSK);

    transfer
        .push_note(0, note)
        .expect("Genesis note to be pushed to the state");

    if testnet {
        let note = Note::transparent(&mut rng, &FAUCET_KEY, FAUCET_DUSK);
        transfer
            .push_note(0, note)
            .expect("Faucet note to be pushed in the state");
    }

    transfer
        .update_root()
        .expect("Root to be updated after pushing genesis note");

    let stake_amount = stake_amount(testnet);
    let stake_balance = stake_amount * PROVISIONERS.len() as u64;

    transfer
        .add_balance(rusk_abi::stake_contract(), stake_balance)
        .expect("Stake contract balance to be set with provisioner stakes");

    transfer
}

const fn stake_amount(testnet: bool) -> Dusk {
    match testnet {
        true => dusk(2_000_000.0),
        false => MINIMUM_STAKE,
    }
}

/// Creates a new stake contract state with preset stakes added for the
/// staking/consensus keys in the `keys/` folder. The stakes will all be the
/// same and the minimum amount.
fn genesis_stake(testnet: bool) -> StakeContract {
    let theme = Theme::default();
    let mut stake_contract = StakeContract::default();

    let stake_amount = stake_amount(testnet);

    for provisioner in PROVISIONERS.iter() {
        let stake = Stake::with_eligibility(stake_amount, 0, 0);
        stake_contract
            .insert_stake(*provisioner, stake)
            .expect("Genesis stake to be pushed to the stake");
    }
    info!(
        "{} Added {} provisioners",
        theme.action("Generating"),
        PROVISIONERS.len()
    );

    stake_contract
}

pub fn deploy<B>(
    testnet: bool,
    ctor: &BackendCtor<B>,
) -> Result<NetworkStateId, Box<dyn Error>>
where
    B: 'static + Backend,
{
    Persistence::with_backend(ctor, |_| Ok(()))?;

    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let transfer = Contract::new(
        genesis_transfer(testnet),
        &include_bytes!(
      "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    )[..],
    );

    let stake = Contract::new(
        genesis_stake(testnet),
        &include_bytes!(
            "../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
        )[..],
    );

    let mut network = NetworkState::default();

    info!(
        "{} Genesis Transfer Contract state",
        theme.action("Deploying")
    );

    network
        .deploy_with_id(rusk_abi::transfer_contract(), transfer)
        .expect("Genesis Transfer Contract should be deployed");

    info!("{} Genesis Stake Contract state", theme.action("Deploying"));

    network
        .deploy_with_id(rusk_abi::stake_contract(), stake)
        .expect("Genesis Transfer Contract should be deployed");

    info!("{} network state", theme.action("Storing"));

    network.commit();
    network.push();

    info!("{} {}", theme.action("Root"), hex::encode(network.root()));

    let state_id = network.persist(ctor).expect("Error in persistence");

    Ok(state_id)
}

pub struct ExecConfig {
    pub build: bool,
    pub force: bool,
    pub testnet: bool,
}

pub fn exec(config: ExecConfig) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    info!("{} Network state", theme.action("Checking"));
    let state_path = rusk_profile::get_rusk_state_dir()?;
    let id_path = rusk_profile::get_rusk_state_id_path()?;

    // if we're not forcing a rebuild/download and the state already exists in
    // the expected path, stop early.
    if !config.force && state_path.exists() && id_path.exists() {
        info!("{} existing state", theme.info("Found"));

        let _ = NetworkStateId::read(&id_path)?;

        info!(
            "{} state id at {}",
            theme.success("Checked"),
            id_path.display()
        );
        return Ok(());
    }

    if config.build {
        info!("{} new state", theme.info("Building"));
        let state_id = deploy(config.testnet, &diskbackend())
            .expect("Failed to deploy network state");

        info!("{} persisted id", theme.success("Storing"));
        state_id.write(&id_path)?;
    } else {
        info!("{} state from previous build", theme.info("Downloading"));

        if let Err(err) = download_state() {
            error!("{} downloading state", theme.error("Failed"));
            return Err(err);
        }
    }

    if !state_path.exists() {
        error!(
            "{} network state at {}",
            theme.error("Missing"),
            state_path.display()
        );
        return Err("Missing state at expected path".into());
    }

    if !id_path.exists() {
        error!(
            "{} persisted id at {}",
            theme.error("Missing"),
            id_path.display()
        );
        return Err("Missing persisted id at expected path".into());
    }

    info!(
        "{} network state at {}",
        theme.success("Stored"),
        state_path.display()
    );
    info!(
        "{} persisted id at {}",
        theme.success("Stored"),
        id_path.display()
    );

    Ok(())
}

const STATE_URL: &str =
    "https://dusk-infra.ams3.digitaloceanspaces.com/keys/rusk-state.zip";

/// Downloads the state into the rusk profile directory.
fn download_state() -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    let mut buffer = vec![];
    let response = request::get(STATE_URL, &mut buffer)?;

    // only accept success codes.
    if !response.status_code().is_success() {
        return Err(format!(
            "State download error: HTTP {}",
            response.status_code()
        )
        .into());
    }

    info!("{} state archive into", theme.info("Unzipping"));

    let reader = Cursor::new(buffer);
    let mut zip = ZipArchive::new(reader)?;

    let mut profile_path = rusk_profile::get_rusk_profile_dir()?;
    profile_path.pop();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let entry_path = profile_path.join(entry.name());

        if entry.is_dir() {
            let _ = fs::create_dir_all(entry_path);
        } else {
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;
            fs::write(entry_path, buffer)?;
        }
    }

    Ok(())
}
