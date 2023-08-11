// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use once_cell::sync::Lazy;
use phoenix_core::transaction::*;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::{dusk, Dusk};
use rusk_abi::{ContractData, ContractId, Session, VM};
use rusk_abi::{LICENSE_CONTRACT, STAKE_CONTRACT, TRANSFER_CONTRACT};
use std::error::Error;
use std::fs;
use std::path::Path;
use tracing::info;
use url::Url;

pub use snapshot::{Balance, GenesisStake, Governance, Snapshot};

mod http;
mod snapshot;
pub mod tar;
mod zip;

pub const MINIMUM_STAKE: Dusk = dusk(1000.0);

const GENESIS_BLOCK_HEIGHT: u64 = 0;

pub static DUSK_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../assets/dusk.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

pub static FAUCET_KEY: Lazy<PublicSpendKey> = Lazy::new(|| {
    let bytes = include_bytes!("../assets/faucet.psk");
    PublicSpendKey::from_bytes(bytes).expect("faucet should have a valid key")
});

fn deploy_governance_contract(
    session: &mut Session,
    governance: &Governance,
) -> Result<(), Box<dyn Error>> {
    let contract_id = governance.contract();
    let bytecode = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/governance_contract.wasm"
    );

    let theme = Theme::default();
    info!(
        "{} {} governance to {}",
        theme.action("Deploying"),
        governance.name,
        hex::encode(contract_id)
    );
    session.deploy(
        bytecode,
        ContractData::builder(governance.owner()).contract_id(contract_id),
        u64::MAX,
    )?;

    // Set the broker and the authority of the governance contract
    session.call::<_, ()>(
        contract_id,
        "set_broker",
        governance.broker(),
        u64::MAX,
    )?;
    session.call::<_, ()>(
        contract_id,
        "set_authority",
        governance.authority(),
        u64::MAX,
    )?;

    Ok(())
}

fn generate_transfer_state(
    session: &mut Session,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    let mut update_root = false;
    snapshot.transfers().enumerate().for_each(|(idx, balance)| {
        update_root = true;
        info!("{} balance #{}", theme.action("Generating"), idx,);

        let mut rng = match balance.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };

        balance.notes.iter().for_each(|&amount| {
            let note = Note::transparent(&mut rng, balance.address(), amount);
            session
                .call::<_, Note>(
                    TRANSFER_CONTRACT,
                    "push_note",
                    &(GENESIS_BLOCK_HEIGHT, note),
                    u64::MAX,
                )
                .expect("Genesis note to be pushed to the state");
        });
    });
    if update_root {
        session
            .call::<_, ()>(TRANSFER_CONTRACT, "update_root", &(), u64::MAX)
            .expect("Root to be updated after pushing genesis note");
    }
    Ok(())
}

fn generate_stake_state(
    session: &mut Session,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();
    snapshot.stakes().enumerate().for_each(|(idx, staker)| {
        info!("{} provisioner #{}", theme.action("Generating"), idx);
        let stake = StakeData {
            amount: Some((
                staker.amount,
                staker.eligibility.unwrap_or_default(),
            )),
            reward: staker.reward.unwrap_or_default(),
            counter: 0,
        };
        session
            .call::<_, ()>(
                STAKE_CONTRACT,
                "insert_stake",
                &(*staker.address(), stake),
                u64::MAX,
            )
            .expect("stake to be inserted into the state");
        session
            .call::<_, ()>(
                STAKE_CONTRACT,
                "insert_allowlist",
                staker.address(),
                u64::MAX,
            )
            .expect("staker to be inserted into the allowlist");
    });
    snapshot.owners().for_each(|provisioner| {
        session
            .call::<_, ()>(STAKE_CONTRACT, "add_owner", provisioner, u64::MAX)
            .expect("owner to be added into the state");
    });

    snapshot.allowlist().for_each(|provisioner| {
        session
            .call::<_, ()>(
                STAKE_CONTRACT,
                "insert_allowlist",
                provisioner,
                u64::MAX,
            )
            .expect("provisioner to be inserted into the allowlist");
    });

    let stake_balance: u64 = snapshot.stakes().map(|s| s.amount).sum();
    if stake_balance > 0 {
        let m: ContractId = STAKE_CONTRACT;
        session
            .call::<_, ()>(
                TRANSFER_CONTRACT,
                "add_module_balance",
                &(m, stake_balance),
                u64::MAX,
            )
            .expect("Stake contract balance to be set with provisioner stakes");
    }
    Ok(())
}

fn generate_empty_state<P: AsRef<Path>>(
    state_dir: P,
    snapshot: &Snapshot,
) -> Result<(VM, [u8; 32]), Box<dyn Error>> {
    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let state_dir = state_dir.as_ref();

    let vm = rusk_abi::new_vm(state_dir)?;
    let mut session = rusk_abi::new_genesis_session(&vm);

    let transfer_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    );

    let stake_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
    );

    let license_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/license_contract.wasm"
    );

    info!("{} Genesis Transfer Contract", theme.action("Deploying"));
    session.deploy(
        transfer_code,
        ContractData::builder(snapshot.owner()).contract_id(TRANSFER_CONTRACT),
        u64::MAX,
    )?;

    info!("{} Genesis Stake Contract", theme.action("Deploying"));
    session.deploy(
        stake_code,
        ContractData::builder(snapshot.owner()).contract_id(STAKE_CONTRACT),
        u64::MAX,
    )?;

    info!("{} Genesis License Contract", theme.action("Deploying"));
    session.deploy(
        license_code,
        ContractData::builder(snapshot.owner()).contract_id(LICENSE_CONTRACT),
        u64::MAX,
    )?;

    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_module_balance",
            &(STAKE_CONTRACT, 0u64),
            u64::MAX,
        )
        .expect("stake contract balance to be set with provisioner stakes");

    session
        .call::<_, ()>(TRANSFER_CONTRACT, "update_root", &(), u64::MAX)
        .expect("root to be updated after pushing genesis note");

    let owners = session
        .call::<_, Vec<BlsPublicKey>>(STAKE_CONTRACT, "owners", &(), u64::MAX)
        .expect("Querying the stake owners should succeed");
    assert!(
        owners.data.is_empty(),
        "Genesis stake should have no owners"
    );

    session
        .call::<_, ()>(LICENSE_CONTRACT, "noop", &(), u64::MAX)
        .expect("license contract noop should succeed");

    let commit_id = session.commit()?;

    info!("{} {}", theme.action("Empty Root"), hex::encode(commit_id));

    Ok((vm, commit_id))
}

// note: deploy consumes session as it produces commit id
pub fn deploy<P: AsRef<Path>>(
    state_dir: P,
    snapshot: &Snapshot,
) -> Result<(VM, [u8; 32]), Box<dyn Error>> {
    let theme = Theme::default();

    let state_dir = state_dir.as_ref();
    let state_id_path = rusk_profile::to_rusk_state_id_path(state_dir);

    let (vm, old_commit_id) = match snapshot.base_state() {
        Some(state) => load_state(state_dir, state),
        None => generate_empty_state(state_dir, snapshot),
    }?;

    let mut session =
        rusk_abi::new_session(&vm, old_commit_id, GENESIS_BLOCK_HEIGHT)?;

    generate_transfer_state(&mut session, snapshot)?;
    generate_stake_state(&mut session, snapshot)?;

    for governance in snapshot.governance_contracts() {
        deploy_governance_contract(&mut session, governance)?;
    }

    info!("{} persisted id", theme.success("Storing"));
    let commit_id = session.commit()?;
    fs::write(state_id_path, commit_id)?;

    if old_commit_id != commit_id {
        vm.delete_commit(old_commit_id)?;
        vm.squash_commit(commit_id)?;
    }

    info!("{} {}", theme.action("Init Root"), hex::encode(commit_id));

    Ok((vm, commit_id))
}

/// Restore a state from the given directory.
pub fn restore_state<P: AsRef<Path>>(
    state_dir: P,
) -> Result<(VM, [u8; 32]), Box<dyn Error>> {
    let state_dir = state_dir.as_ref();
    let state_id_path = rusk_profile::to_rusk_state_id_path(state_dir);

    if !state_id_path.exists() {
        return Err(format!("Missing ID at {}", state_id_path.display()).into());
    }

    let commit_id_bytes = fs::read(state_id_path)?;
    if commit_id_bytes.len() != 32 {
        return Err(format!(
            "Wrong length for id {}, expected 32",
            commit_id_bytes.len()
        )
        .into());
    }
    let mut commit_id = [0u8; 32];
    commit_id.copy_from_slice(&commit_id_bytes);

    let vm = rusk_abi::new_vm(state_dir)?;
    Ok((vm, commit_id))
}

/// Load a state file and save it into the rusk state directory.
fn load_state<P: AsRef<Path>>(
    state_dir: P,
    url: &str,
) -> Result<(VM, [u8; 32]), Box<dyn Error>> {
    let state_dir = state_dir.as_ref();
    let state_id_path = rusk_profile::to_rusk_state_id_path(state_dir);

    if state_id_path.exists() {
        return Err("No valid state should be found".into());
    }

    info!(
        "{} base state from {url}",
        Theme::default().action("Retrieving"),
    );
    let url = Url::parse(url)?;
    let buffer = match url.scheme() {
        "http" | "https" => http::download(url)?,
        "file" => fs::read(url.path())?,
        _ => Err("Unsupported scheme for base state")?,
    };

    tar::unarchive(&buffer, state_dir)?;

    let (vm, commit) = restore_state(state_dir)?;
    info!(
        "{} {}",
        Theme::default().action("Base Root"),
        hex::encode(commit)
    );

    Ok((vm, commit))
}
