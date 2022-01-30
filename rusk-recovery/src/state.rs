// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::provisioners::PROVISIONERS;
use crate::theme::Theme;

use microkelvin::{Backend, BackendCtor, DiskBackend, PersistError};
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi;
use rusk_vm::{Contract, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract, MINIMUM_STAKE};
use std::{fs, io};
use tracing::info;
use transfer_contract::TransferContract;

/// Initial amount of the note inserted in the state.
const GENESIS_DUSK: u64 = 1_000_000_000; // 1000 Dusk.
/// The number of blocks after which the genesis stake expires.
const GENESIS_EXPIRATION: u64 = 1_000_000;

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
/// of Dusk Network.
fn genesis_transfer() -> TransferContract {
    let mut transfer = TransferContract::default();
    let mut rng = StdRng::seed_from_u64(0xdead_beef);

    let note =
        Note::transparent(&mut rng, TransferContract::dusk_key(), GENESIS_DUSK);
    transfer
        .push_note(0, note)
        .expect("Genesis note to be pushed to the state");
    transfer
        .update_root()
        .expect("Root to be updated after pushing genesis note");

    transfer
}

/// Creates a new stake contract state with preset stakes added for the
/// staking/consensus keys in the `keys/` folder. The stakes will all be the
/// same and the minimum amount.
fn genesis_stake() -> StakeContract {
    let mut stake_contract = StakeContract::default();

    let stake = Stake::new(MINIMUM_STAKE, 0, GENESIS_EXPIRATION);

    for provisioner in PROVISIONERS.iter() {
        stake_contract
            .push_stake(*provisioner, stake)
            .expect("Genesis stake to be pushed to the stake");
    }

    stake_contract
}

pub fn deploy<B>(ctor: &BackendCtor<B>) -> Result<NetworkStateId, PersistError>
where
    B: 'static + Backend,
{
    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let transfer = Contract::new(
        genesis_transfer(),
        &include_bytes!(
      "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    )[..],
    );

    let stake = Contract::new(
        genesis_stake(),
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

    let state_id = network.persist(ctor).expect("Error in persistence");

    Ok(state_id)
}

pub fn exec(overwrite: bool) -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::default();

    info!("{} Network state", theme.action("Checking"));
    let state_path = rusk_profile::get_rusk_state_dir()?;
    let id_path = rusk_profile::get_rusk_state_id_path()?;

    let has_state = state_path.exists() && id_path.exists();

    if has_state {
        if overwrite {
            info!("{} previous network state", theme.info("Found"));
        } else {
            info!("{} previous network state", theme.info("Keep"));
            return Ok(());
        }
    } else {
        info!("{} previous network state", theme.info("Missing"));
    }

    let state_id =
        deploy(&diskbackend()).expect("Failed to deploy network state");

    info!(
        "{} network state at {}",
        theme.info("Stored"),
        state_path.display()
    );
    info!("{} persisted id", theme.action("Storing"));
    state_id.write(&id_path)?;

    info!(
        "{} persisted id at {}",
        theme.info("Stored"),
        id_path.display()
    );

    Ok(())
}
