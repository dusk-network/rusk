// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::theme::Theme;

use canonical::{Canon, Sink, Source};
use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use governance_contract::GovernanceContract;
use http_req::request;
use microkelvin::{Backend, BackendCtor, Persistence};
use once_cell::sync::Lazy;
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::ContractId;
use rusk_vm::dusk_abi::ContractState;
use rusk_vm::{Contract, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use tracing::info;
use transfer_contract::TransferContract;
use url::Url;

pub use snapshot::{Balance, GenesisStake, Snapshot};

use self::snapshot::Governance;

mod snapshot;
pub mod tar;
mod zip;

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
    let contract_id = ContractId::reserved(governance.contract_address);

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
    snapshot: &Snapshot,
    state: &mut NetworkState,
) -> Result<TransferContract, Box<dyn Error>> {
    let mut transfer: TransferContract =
        state.get_contract_cast_state(&rusk_abi::transfer_contract())?;
    let theme = Theme::default();

    snapshot.transfers().enumerate().for_each(|(idx, balance)| {
        info!("{} balance #{}", theme.action("Generating"), idx);

        let mut rng = match balance.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };

        balance.notes.iter().for_each(|&amount| {
            let note = Note::transparent(&mut rng, balance.address(), amount);
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

    Ok(transfer)
}

fn generate_stake_state(
    snapshot: &Snapshot,
    state: &mut NetworkState,
) -> Result<StakeContract, Box<dyn Error>> {
    let theme = Theme::default();
    let mut stake_contract: StakeContract =
        state.get_contract_cast_state(&rusk_abi::stake_contract())?;
    snapshot.stakes().enumerate().for_each(|(idx, staker)| {
        info!("{} provisioner #{}", theme.action("Generating"), idx);
        let stake = Stake::with_eligibility(
            staker.amount,
            staker.reward.unwrap_or_default(),
            staker.eligibility.unwrap_or_default(),
        );
        stake_contract
            .insert_stake(*staker.address(), stake)
            .expect("stake to be inserted into the state");
        stake_contract
            .insert_allowlist(*staker.address())
            .expect("staker to be inserted into the allowlist");
    });
    snapshot.owners().for_each(|provisioner| {
        stake_contract
            .add_owner(*provisioner)
            .expect("owner to be added into the state");
    });

    let to_allow = snapshot.allowlist().enumerate();
    to_allow.for_each(|(idx, provisioner)| {
        info!("{} additional provisioner #{idx}", theme.action("Allowing"));
        stake_contract
            .insert_allowlist(*provisioner)
            .expect("provisioner to be inserted into the allowlist");
    });

    Ok(stake_contract)
}

fn generate_empty_state() -> Result<NetworkState, Box<dyn Error>> {
    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let transfer_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
    )
    .to_vec();

    let stake_code = include_bytes!(
        "../../target/wasm32-unknown-unknown/release/stake_contract.wasm"
    )
    .to_vec();

    let mut transfer = TransferContract::default();

    transfer
        .add_balance(rusk_abi::stake_contract(), 0)
        .expect("stake contract balance to be set with provisioner stakes");
    transfer
        .update_root()
        .expect("root to be updated after pushing genesis note");

    let transfer = Contract::new(transfer, transfer_code);
    let stake = Contract::new(StakeContract::default(), stake_code);

    let mut network = NetworkState::default();

    info!("{} Genesis Transfer Contract", theme.action("Deploying"));
    network.deploy_with_id(rusk_abi::transfer_contract(), transfer)?;

    info!("{} Genesis Stake Contract", theme.action("Deploying"));
    network.deploy_with_id(rusk_abi::stake_contract(), stake)?;

    info!(
        "{} {}",
        theme.action("Empty Root"),
        hex::encode(network.root())
    );

    Ok(network)
}

/// Set the contract state for the given Contract Id.
///
/// # Safety
///
/// This function will corrupt the state if the contract state given is
/// not the same type as the one stored in the state at the address
/// provided; and the subsequent contract's call will fail.
pub unsafe fn set_contract_state<C>(
    contract_id: &ContractId,
    state: &C,
    network: &mut NetworkState,
) -> Result<(), Box<dyn Error>>
where
    C: Canon,
{
    const PAGE_SIZE: usize = 1024 * 64;
    let mut bytes = [0u8; PAGE_SIZE];
    let mut sink = Sink::new(&mut bytes[..]);
    ContractState::from_canon(state).encode(&mut sink);
    let mut source = Source::new(&bytes[..]);
    let contract_state = ContractState::decode(&mut source).unwrap();
    *network.get_contract_mut(contract_id)?.state_mut() = contract_state;

    Ok(())
}

pub fn deploy<B>(
    snapshot: &Snapshot,
    ctor: &BackendCtor<B>,
) -> Result<NetworkStateId, Box<dyn Error>>
where
    B: 'static + Backend,
{
    let theme = Theme::default();
    Persistence::with_backend(ctor, |_| Ok(()))?;

    let mut network = match snapshot.base_state() {
        Some(state) => load_state(state),
        None => generate_empty_state(),
    }?;
    let transfer = generate_transfer_state(snapshot, &mut network)?;
    let stake = generate_stake_state(snapshot, &mut network)?;

    // SAFETY: this is safe because we know the contracts exist
    unsafe {
        set_contract_state(&rusk_abi::stake_contract(), &stake, &mut network)?;
        set_contract_state(
            &rusk_abi::transfer_contract(),
            &transfer,
            &mut network,
        )?;
    };

    for governance in snapshot.governance_contracts() {
        deploy_governance_contract(governance, &mut network)?;
    }

    network.commit();
    network.push();

    info!(
        "{} {}",
        theme.action("Init Root"),
        hex::encode(network.root())
    );

    let state_id = network.persist(ctor)?;

    Ok(state_id)
}

/// Restore a state from a specific id_path
pub fn restore_state(
    id_path: &PathBuf,
) -> Result<NetworkState, Box<dyn Error>> {
    if !id_path.exists() {
        return Err(
            format!("Missing persisted id at {}", id_path.display()).into()
        );
    }

    let id = NetworkStateId::read(id_path)?;
    let network = NetworkState::new().restore(id)?;

    Ok(network)
}

/// Load a state file and save it into the rusk state directory.
fn load_state(url: &str) -> Result<NetworkState, Box<dyn Error>> {
    let id_path = rusk_profile::get_rusk_state_id_path()?;
    assert!(
        restore_state(&id_path).is_err(),
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

    let network = restore_state(&id_path)?;
    info!(
        "{} {}",
        Theme::default().action("Base Root"),
        hex::encode(network.root())
    );
    Ok(network)
}
