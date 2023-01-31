// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use http_req::request;
use once_cell::sync::Lazy;
use phoenix_core::transaction::*;
use phoenix_core::Note;
use piecrust::{CommitId, ModuleId, Session};
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::{dusk, Dusk};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use url::Url;

use crate::provisioners::DUSK_KEY as DUSK_BLS_KEY;
pub use snapshot::{Balance, GenesisStake, Snapshot};

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
    governance: &Governance,
    state: &mut NetworkState,
) -> Result<(), Box<dyn Error>> {
    let gov_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/governance_contract.wasm"
    )
    .to_vec();
    let contract = Contract::new(GovernanceContract::default(), gov_code);
    let contract_id = governance.contract();

    let theme = Theme::default();
    info!(
        "{} {} governance to {}",
        theme.action("Deploying"),
        governance.name,
        contract_id
    );
    state.deploy_with_id(contract_id, contract)?;

    let mut gov_state: GovernanceContract =
        state.get_contract_cast_state(&contract_id)?;
    gov_state.authority = *governance.authority();
    gov_state.broker = Some(*governance.broker());

    unsafe {
        set_contract_state(&contract_id, &gov_state, state)?;
    }
    Ok(())
}

fn generate_transfer_state(
    session: &mut Session,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    snapshot.transfers().enumerate().for_each(|(idx, balance)| {
        info!(
            "{} balance #{} = {:?}",
            theme.action("Generating"),
            idx,
            balance.notes
        );

        let mut rng = match balance.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };

        info!("pushing notes {}", balance.notes.len());
        balance.notes.iter().for_each(|&amount| {
            let note = Note::transparent(&mut rng, balance.address(), amount);
            let _: Note = session
                .transact(
                    rusk_abi::transfer_module(),
                    "push_note",
                    &(GENESIS_BLOCK_HEIGHT, note),
                )
                .expect("Genesis note to be pushed to the state");
        });
        info!("after pushing notes {}", balance.notes.len());
    });

    let _: BlsScalar = session
        .transact(rusk_abi::transfer_module(), "update_root", &())
        .expect("Root to be updated after pushing genesis note");

    let stake_balance: u64 = snapshot.stakes().map(|s| s.amount).sum();

    let _: u64 = session
        .query(
            rusk_abi::transfer_module(),
            "module_balance",
            &rusk_abi::stake_module(),
        )
        .expect("Stake contract balance query should succeed");

    let m: ModuleId = rusk_abi::stake_module();
    let _: () = session
        .transact(
            rusk_abi::transfer_module(),
            "add_module_balance",
            &(m, stake_balance),
        )
        .expect("Stake contract balance to be set with provisioner stakes");

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
        let _: () = session
            .transact(
                rusk_abi::stake_module(),
                "insert_stake",
                &(*staker.address(), stake),
            )
            .expect("stake to be inserted into the state");
        let _: () = session
            .transact(
                rusk_abi::stake_module(),
                "insert_allowlist",
                staker.address(),
            )
            .expect("staker to be inserted into the allowlist");
    });
    snapshot.owners().for_each(|provisioner| {
        let _: () = session
            .transact(rusk_abi::stake_module(), "add_owner", provisioner)
            .expect("owner to be added into the state");
    });

    snapshot.allowlist().for_each(|provisioner| {
        let _: () = session
            .transact(rusk_abi::stake_module(), "insert_allowlist", provisioner)
            .expect("provisioner to be inserted into the allowlist");
    });

    Ok(())
}

fn generate_empty_state(session: &mut Session) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let transfer_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    );

    let stake_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
    );

    info!("{} Genesis Transfer Contract", theme.action("Deploying"));
    session.deploy_with_id(rusk_abi::transfer_module(), transfer_code)?;

    info!("{} Genesis Stake Contract", theme.action("Deploying"));
    session.deploy_with_id(rusk_abi::stake_module(), stake_code)?;

    let _: () = session
        .transact(
            rusk_abi::transfer_module(),
            "add_module_balance",
            &(rusk_abi::stake_module(), 0u64),
        )
        .expect("stake contract balance to be set with provisioner stakes");

    let _: BlsScalar = session
        .transact(rusk_abi::transfer_module(), "update_root", &())
        .expect("root to be updated after pushing genesis note");

    let _: Option<StakeData> = session
        .query(rusk_abi::stake_module(), "get_stake", &*DUSK_BLS_KEY)
        .expect("Querying a stake should succeed");

    Ok(())
}

// note: deploy consumes session as it produces commit id
pub fn deploy<P: AsRef<Path>>(
    commit_id_path: P,
    snapshot: &Snapshot,
    mut session: Session,
) -> Result<CommitId, Box<dyn Error>> {
    let theme = Theme::default();

    rusk_abi::set_block_height(&mut session, GENESIS_BLOCK_HEIGHT);
    session.set_point_limit(u64::MAX);

    match snapshot.base_state() {
        Some(state) => load_state(&mut session, state),
        None => generate_empty_state(&mut session),
    }?;
    generate_transfer_state(&mut session, snapshot)?;
    generate_stake_state(&mut session, snapshot)?;

    let commit_id = session.commit()?;
    commit_id.persist(commit_id_path)?;

    info!(
        "{} {}",
        theme.action("Init Root"),
        hex::encode(commit_id.as_bytes())
    );

    Ok(commit_id)
}

/// Restore a state from a specific id_path
pub fn restore_state(
    session: &mut Session,
    id_path: &PathBuf,
) -> Result<CommitId, Box<dyn Error>> {
    if !id_path.exists() {
        return Err(
            format!("Missing persisted id at {}", id_path.display()).into()
        );
    }
    let commit_id = CommitId::restore(id_path)?;
    session.restore(&commit_id)?;
    Ok(commit_id)
}

/// Load a state file and save it into the rusk state directory.
fn load_state(session: &mut Session, url: &str) -> Result<(), Box<dyn Error>> {
    let state_dir = rusk_profile::get_rusk_state_dir()?;
    let id_path = rusk_profile::to_rusk_state_id_path(&state_dir);

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

    tar::unarchive(&buffer, output)?;

    restore_state(session, &id_path)?;
    info!(
        "{} {}",
        Theme::default().action("Base Root"),
        hex::encode(session.root(false)?)
    );
    Ok(())
}
