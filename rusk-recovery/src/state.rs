// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;
use microkelvin::{BackendCtor, DiskBackend};
use rusk_abi;
use rusk_vm::{Contract, NetworkState};
use stake_contract::StakeContract;
use std::{fs, io};
use tracing::info;
use transfer_contract::TransferContract;

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

    info!("{} new network state", theme.action("Generating"));

    let transfer = Contract::new(
        TransferContract::default(),
        &include_bytes!(
      "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    )[..],
    );

    let stake = Contract::new(
        StakeContract::default(),
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

    let state_id = network
        .persist(&diskbackend())
        .expect("Error in persistence");

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
