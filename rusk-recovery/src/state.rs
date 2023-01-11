// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use http_req::request;
use once_cell::sync::Lazy;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
// use rusk_abi::ModuleId;
use stake_contract_types::StakeData;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tracing::info;
// use transfer_contract::TransferContract;
use url::Url;
use piecrust::{CommitId, ModuleId, Session};
use dusk_bls12_381::BlsScalar;

pub use snapshot::{Balance, GenesisStake, Snapshot};

mod snapshot;
pub mod zip;

const GENESIS_BLOCK_HEIGHT: u64 = 0;

pub static DUSK_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../assets/dusk.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

pub static FAUCET_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../assets/faucet.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

fn generate_transfer_state(
    session: &mut Session,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    println!("start generate_transfer_state");
    let theme = Theme::default();

    snapshot.transfers().enumerate().for_each(|(idx, balance)| {
        info!("{} balance #{} = {:?}", theme.action("Generating"), idx, balance.notes);

        let mut rng = match balance.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };

        info!("pushing notes {}", balance.notes.len());
        balance.notes.iter().for_each(|&amount| {
            let note = Note::transparent(&mut rng, balance.address(), amount);
            println!("about to push note with amount {}", amount);
            let _: Note = session
                .transact(
                    rusk_abi::transfer_module(),
                    "push_note",
                    (GENESIS_BLOCK_HEIGHT, note),
                )
                .expect("Genesis note to be pushed to the state");
            println!("pushed note with amount {}", amount);
        });
        info!("after pushing notes {}", balance.notes.len());
    });

    println!("updating root");
    let _: BlsScalar = session
        .transact(rusk_abi::transfer_module(), "update_root", ())
        .expect("Root to be updated after pushing genesis note");
    println!("after updating root");

    let stake_balance: u64 = snapshot.stakes().map(|s| s.amount).sum();

    println!("querying module balance");
    let stake_module_balance: u64 = session
        .query(rusk_abi::transfer_module(), "module_balance", rusk_abi::stake_module())
        .expect("Stake contract balance query should succeed");
    println!("after querying module balance, balance={}", stake_module_balance);

    let m: ModuleId = rusk_abi::stake_module();
    println!("adding balance");
    let _: BlsScalar = session
        .transact(rusk_abi::transfer_module(), "add_module_balance", (m, stake_balance))
        .expect("Stake contract balance to be set with provisioner stakes");
    println!("after adding balance");

    println!("end generate_transfer_state");

    Ok(())
}

fn generate_stake_state(
    session: &mut Session,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();
    snapshot.stakes().enumerate().for_each(|(idx, staker)| {
        info!("{} provisioner #{}", theme.action("Generating"), idx);
        let stake = StakeData::with_eligibility(
            staker.amount,
            staker.reward.unwrap_or_default(),
            staker.eligibility.unwrap_or_default(),
        );
        let _: Option<StakeData> = session
            .transact(rusk_abi::stake_module(), "insert_stake", (*staker.address(), stake))
            .expect("stake to be inserted into the state");
        let _: () = session
            .transact(rusk_abi::stake_module(), "insert_allowlist", *staker.address())
            .expect("staker to be inserted into the allowlist");
    });
    snapshot.owners().for_each(|provisioner| {
        let _: () = session
            .transact(rusk_abi::stake_module(), "add_owner", *provisioner)
            .expect("owner to be added into the state");
    });

    snapshot.allowlist().for_each(|provisioner| {
        let _: () = session
            .transact(rusk_abi::stake_module(), "insert_allowlist", *provisioner)
            .expect("provisioner to be inserted into the allowlist");
    });

    Ok(())
}

fn generate_empty_state(session: &mut Session) -> Result<(), Box<dyn Error>> {
    println!("start generate_empty_state");
    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let transfer_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    );

    let stake_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
    );

    // let mut transfer = TransferContract::default();
    //
    // transfer
    //     .add_balance(rusk_abi::stake_contract(), 0)
    //     .expect("stake contract balance to be set with provisioner stakes");
    // transfer
    //     .update_root()
    //     .expect("root to be updated after pushing genesis note");
    //
    // let transfer = Contract::new(transfer, transfer_code);
    // let stake = Contract::new(StakeContract::default(), stake_code);

    info!("{} Genesis Transfer Contract", theme.action("Deploying"));
    session.deploy_with_id(rusk_abi::transfer_module(), transfer_code).map_err(|_e|std::fmt::Error)?;// todo error conversion

    info!("{} Genesis Stake Contract", theme.action("Deploying"));
    session.deploy_with_id(rusk_abi::stake_module(), stake_code).map_err(|_e|std::fmt::Error)?;// todo error conversion

    // println!("about to do add_module_balance for stake module, balance 0");
    // let _: BlsScalar = session
    //     .transact(rusk_abi::transfer_module(), "add_module_balance", (rusk_abi::stake_module(), 0u64)).map_err(|e|{println!("{:?}", e); std::fmt::Error})?;
    //     //.expect("stake contract balance to be set with provisioner stakes");
    // println!("done add_module_balance for stake module");

    println!("about to update_root for transfer module");
    let _: BlsScalar = session
        .transact(rusk_abi::transfer_module(), "update_root", ())
        .expect("root to be updated after pushing genesis note"); // todo! not sure update root it is needed here
    println!("done update_root for transfer module");

    info!(
        "{} todo!",
        theme.action("Empty Root")//,
        //hex::encode(network.root())
    );
    println!("end generate_empty_state");

    Ok(())
}

// Set the contract state for the given Contract Id.
//
// # Safety
//
// This function will corrupt the state if the contract state given is
// not the same type as the one stored in the state at the address
// provided; and the subsequent contract's call will fail.
// pub unsafe fn set_contract_state<C>(
//     contract_id: &ContractId,
//     state: &C,
//     session: &mut Session,
// ) -> Result<(), Box<dyn Error>>
// where
//     C: Canon,
// {
//     const PAGE_SIZE: usize = 1024 * 64;
//     let mut bytes = [0u8; PAGE_SIZE];
//     let mut sink = Sink::new(&mut bytes[..]);
//     ContractState::from_canon(state).encode(&mut sink);
//     let mut source = Source::new(&bytes[..]);
//     let contract_state = ContractState::decode(&mut source).unwrap();
//     *network.get_contract_mut(contract_id)?.state_mut() = contract_state;
//
//     Ok(())
// }

pub fn deploy(
    snapshot: &Snapshot,
    session: &mut Session,
) -> Result<CommitId, Box<dyn Error>>
{
    println!("start deploy");
    let theme = Theme::default();

    match snapshot.base_state() {
        Some(state) => load_state(session, state),
        None => generate_empty_state(session),
    }?;
    generate_transfer_state(session, snapshot)?;
    generate_stake_state(session, snapshot)?;

    // SAFETY: this is safe because we know the contracts exist
    // unsafe {
    //     set_contract_state(&rusk_abi::stake_contract(), &stake, &mut vm_session)?;
    //     set_contract_state(
    //         &rusk_abi::transfer_contract(),
    //         &transfer,
    //         &mut vm_session,
    //     )?;
    // };
    // vm_session.commit();
    // vm_session.push();

    info!(
        "{} todo!",
        theme.action("Init Root")//,
        //hex::encode(vm_session.root())
    );

    let commit_id = session.commit().map_err(|_e|std::fmt::Error)?;// todo error conversion
    println!("end deploy");

    Ok(commit_id)
}

/// Restore a state from a specific id_path
pub fn restore_state(
    session: &mut Session,
    id_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    if !id_path.exists() {
        return Err(
            format!("Missing persisted id at {}", id_path.display()).into()
        );
    }
    let commit_id = CommitId::restore(id_path).map_err(|_e|std::fmt::Error)?;// todo error conversion
    session.restore(&commit_id).map_err(|_e|std::fmt::Error)?;// todo error conversion
    Ok(())
}

/// Load a state file and save it into the rusk state directory.
fn load_state(session: &mut Session, url: &str) -> Result<(), Box<dyn Error>> {
    println!("start load_state");
    let id_path = rusk_profile::get_rusk_state_id_path()?;
    assert!(
        restore_state(session, &id_path).is_err(),
        "No valid state should be found"
    );

    info!(
        "{} base state from {url}",
        Theme::default().action("Retrieving"),
    );
    let url = Url::parse(url)?;
    let buffer = match url.scheme() {
        "http" | "https" => {
            let mut buffer = vec![];

            let response = request::get(url, &mut buffer)?;

            // only accept success codes.
            if !response.status_code().is_success() {
                return Err(format!(
                    "State download error: HTTP {}",
                    response.status_code()
                )
                .into());
            }
            buffer
        }
        "file" => fs::read(url.path())?,
        _ => Err("Unsupported scheme for base state")?,
    };

    let state_dir = rusk_profile::get_rusk_state_dir()?;
    let output = state_dir.parent().expect("state dir not equal to root");

    zip::unzip(&buffer, output)?;

    restore_state(session, &id_path)?;
    info!(
        "{} todo!",
        Theme::default().action("Base Root")//,
        //hex::encode(network.root())
    );
    println!("end load_state");
    Ok(())
}
