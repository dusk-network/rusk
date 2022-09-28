// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use http_req::request;
use microkelvin::{Backend, BackendCtor, DiskBackend, Persistence};
use once_cell::sync::Lazy;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_vm::{Contract, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract};
use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing::info;
use tracing::log::error;
use transfer_contract::TransferContract;
use zip::ZipArchive;

pub use snapshot::{Balance, GenesisStake, Snapshot};

mod snapshot;

const GENESIS_BLOCK_HEIGHT: u64 = 0;

pub static DUSK_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../assets/dusk.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

pub static FAUCET_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../assets/faucet.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

fn existing_diskbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(|| DiskBackend::new(rusk_profile::get_rusk_state_dir()?))
}

fn empty_diskbackend() -> BackendCtor<DiskBackend> {
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
fn transfer_state(snapshot: &Snapshot) -> TransferContract {
    let mut transfer = TransferContract::default();

    snapshot.transfers().enumerate().for_each(|(idx, value)| {
        let theme = Theme::default();
        info!("{} Added {} transfer", theme.action("Generating"), idx);

        let mut rng = match value.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };

        value.notes.iter().for_each(|&amount| {
            let note = Note::transparent(&mut rng, value.address(), amount);
            transfer
                .push_note(GENESIS_BLOCK_HEIGHT, note)
                .expect("Genesis note to be pushed to the state");
        });
    });

    transfer
        .update_root()
        .expect("Root to be updated after pushing genesis note");

    let stake_balance = snapshot.stakes().map(|s| s.amount).sum();

    transfer
        .add_balance(rusk_abi::stake_contract(), stake_balance)
        .expect("Stake contract balance to be set with provisioner stakes");

    transfer
}

/// Creates a new stake contract state with preset stakes added for the
/// staking/consensus keys in the `keys/` folder. The stakes will all be the
/// same and the minimum amount.
fn stake_state(snapshot: &Snapshot) -> StakeContract {
    let theme = Theme::default();
    let mut stake_contract = StakeContract::default();
    snapshot.stakes().for_each(|staker| {
        info!("{} Added provisioner", theme.action("Generating"),);
        let stake = Stake::with_eligibility(
            staker.amount,
            staker.reward.unwrap_or_default(),
            staker.eligibility.unwrap_or_default(),
        );
        stake_contract
            .insert_stake(*staker.address(), stake)
            .expect("Genesis stake to be pushed to the stake");
        stake_contract
            .insert_allowlist(*staker.address())
            .expect("Failed to allow genesis provisioner");
    });
    snapshot.owners().for_each(|provisioner| {
        stake_contract
            .add_owner(*provisioner)
            .expect("Failed to set stake contract owner");
    });

    snapshot.allowlist().for_each(|provisioner| {
        stake_contract
            .insert_allowlist(*provisioner)
            .expect("Failed to allow");
    });

    stake_contract
}

pub fn deploy_from_contracts<B>(
    snapshot: &Snapshot,
    ctor: &BackendCtor<B>,
    contracts_folder: Option<&PathBuf>,
) -> Result<NetworkStateId, Box<dyn Error>>
where
    B: 'static + Backend,
{
    Persistence::with_backend(ctor, |_| Ok(()))?;

    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let transfer_code = match contracts_folder {
        Some(folder) => {
            let mut buffer = Vec::new();
            let mut file = File::open(folder.join("transfer_contract.wasm"))?;
            file.read_to_end(&mut buffer)?;
            buffer
        }
        None => include_bytes!(
            "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
        )
        .to_vec(),
    };

    let stake_code = match contracts_folder {
        Some(folder) => {
            let mut buffer = Vec::new();
            let mut file = File::open(folder.join("stake_contract.wasm"))?;
            file.read_to_end(&mut buffer)?;
            buffer
        }
        None => include_bytes!(
            "../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
        )
        .to_vec(),
    };

    let transfer = Contract::new(transfer_state(snapshot), transfer_code);
    let stake = Contract::new(stake_state(snapshot), stake_code);

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

pub fn deploy<B>(
    snapshot: &Snapshot,
    ctor: &BackendCtor<B>,
) -> Result<NetworkStateId, Box<dyn Error>>
where
    B: 'static + Backend,
{
    deploy_from_contracts(snapshot, ctor, None)
}

pub struct ExecConfig {
    pub build: bool,
    pub force: bool,
    pub use_prebuilt_contracts: bool,
}

pub fn exec(
    config: ExecConfig,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    info!("{} Network state", theme.action("Checking"));
    let state_path = rusk_profile::get_rusk_state_dir()?;
    let id_path = rusk_profile::get_rusk_state_id_path()?;

    // if we're not forcing a rebuild/download and the state already exists in
    // the expected path, stop early.
    if !config.force && state_path.exists() && id_path.exists() {
        info!("{} existing state", theme.info("Found"));

        try_network_restore()?;

        info!(
            "{} state id at {}",
            theme.success("Checked"),
            id_path.display()
        );
        return Ok(());
    }

    if config.build {
        info!("{} new state", theme.info("Building"));

        let contracts_folder = match config.use_prebuilt_contracts {
            true => Some(get_contracts()?),
            false => None,
        };

        let state_id = deploy_from_contracts(
            snapshot,
            &empty_diskbackend(),
            contracts_folder.as_ref(),
        )
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
    try_network_restore()?;

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

fn try_network_restore() -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();
    Persistence::with_backend(&existing_diskbackend(), |_| Ok(()))?;
    let network = NetworkState::new();
    let id = NetworkStateId::read(rusk_profile::get_rusk_state_id_path()?)?;
    let network = network.restore(id)?;
    info!(
        "{} restored {}",
        theme.action("Root"),
        hex::encode(network.root())
    );
    Ok(())
}

const STATE_URL: &str =
    "https://dusk-infra.ams3.digitaloceanspaces.com/keys/rusk-state.zip";
const CONTRACTS_URL: &str =
    "https://dusk-infra.ams3.digitaloceanspaces.com/keys/contracts.zip";

/// Downloads the state into the rusk profile directory.
fn download_state() -> Result<(), Box<dyn Error>> {
    let mut profile_path = rusk_profile::get_rusk_profile_dir()?;
    profile_path.pop();
    download_and_unzip("state", STATE_URL, &profile_path)?;
    Ok(())
}

fn get_contracts() -> Result<PathBuf, Box<dyn Error>> {
    let folder = rusk_profile::get_rusk_profile_dir()?.join("contracts");
    fs::create_dir_all(folder.as_path())
        .expect("Unable to create contracts folder");

    let transfer_missing = !folder.join("transfer_contract.wasm").is_file();
    let stake_missing = !folder.join("stake_contract.wasm").is_file();

    if transfer_missing || stake_missing {
        download_and_unzip("contracts", CONTRACTS_URL, &folder)?;
    }
    Ok(folder)
}

/// Downloads a zip file and unzip it into the output directory.
fn download_and_unzip(
    description: &str,
    uri: &str,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    let mut buffer = vec![];
    let response = request::get(uri, &mut buffer)?;

    // only accept success codes.
    if !response.status_code().is_success() {
        return Err(format!(
            "{} download error: HTTP {}",
            description,
            response.status_code()
        )
        .into());
    }

    info!("{} {} archive into", theme.info("Unzipping"), description);

    let reader = Cursor::new(buffer);
    let mut zip = ZipArchive::new(reader)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let entry_path = output.join(entry.name());

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
