// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error::Error;
use std::fs;
use std::path::Path;

use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::stake::{StakeAmount, StakeData, StakeKeys, STAKE_CONTRACT};
use dusk_core::transfer::phoenix::{Note, Sender};
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_core::JubJubScalar;
use dusk_vm::{ContractData, Session, VM};
use ff::Field;
use rand::rngs::StdRng;
use rand::SeedableRng;

use tracing::info;
use url::Url;

use crate::Theme;

mod http;
mod zip;

mod snapshot;
pub use snapshot::{GenesisStake, PhoenixBalance, Snapshot};

pub mod tar;

pub const DEFAULT_SNAPSHOT: &str =
    include_str!("../config/testnet_remote.toml");

const GENESIS_BLOCK_HEIGHT: u64 = 0;
const GENESIS_CHAIN_ID: u8 = 0xFA;

fn generate_transfer_state(
    session: &mut Session,
    snapshot: &Snapshot,
) -> Result<(), Box<dyn Error>> {
    let theme = Theme::default();

    let mut update_root = false;

    snapshot
        .phoenix_balances()
        .enumerate()
        .for_each(|(idx, balance)| {
            update_root = true;
            info!("{} phoenix balance #{idx}", theme.action("Generating"));

            let mut rng = match balance.seed {
                Some(seed) => StdRng::seed_from_u64(seed),
                None => StdRng::from_entropy(),
            };

            balance.notes.iter().for_each(|&amount| {
                let r = JubJubScalar::random(&mut rng);
                let address = balance.address().gen_stealth_address(&r);
                // the sender is "genesis"
                let sender = Sender::ContractInfo([0u8; 128]);
                let note = Note::transparent_stealth(address, amount, sender);
                session
                    .call::<(u64, Note), ()>(
                        TRANSFER_CONTRACT,
                        "push_note",
                        &(GENESIS_BLOCK_HEIGHT, note),
                        u64::MAX,
                    )
                    .expect("Minting should succeed");
            });
        });

    snapshot
        .moonlight_accounts()
        .enumerate()
        .for_each(|(idx, account)| {
            info!("{} moonlight account #{idx}", theme.action("Generating"));

            session
                .call::<(AccountPublicKey, u64), ()>(
                    TRANSFER_CONTRACT,
                    "add_account_balance",
                    &(*account.address(), account.balance),
                    u64::MAX,
                )
                .expect("Making account should succeed");
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

        let amount = (staker.amount > 0).then(|| StakeAmount {
            value: staker.amount,
            eligibility: staker.eligibility.unwrap_or_default(),
            locked: 0,
        });

        let stake = StakeData {
            amount,
            reward: staker.reward.unwrap_or_default(),
            faults: 0,
            hard_faults: 0,
        };

        session
            .call::<_, ()>(
                STAKE_CONTRACT,
                "insert_stake",
                &(staker.to_stake_keys(), stake),
                u64::MAX,
            )
            .expect("stake to be inserted into the state");
    });

    let stake_balance: u64 = snapshot.stakes().map(|s| s.amount).sum();
    if stake_balance > 0 {
        let m: ContractId = STAKE_CONTRACT;
        session
            .call::<_, ()>(
                TRANSFER_CONTRACT,
                "add_contract_balance",
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
    dusk_key: AccountPublicKey,
) -> Result<(VM, [u8; 32]), Box<dyn Error>> {
    let theme = Theme::default();
    info!("{} new network state", theme.action("Generating"));

    let state_dir = state_dir.as_ref();

    let vm = VM::new(state_dir)?;
    let mut session = vm.genesis_session(GENESIS_CHAIN_ID);

    let transfer_code = include_bytes!("../assets/transfer_contract.wasm");
    let stake_code = include_bytes!("../assets/stake_contract.wasm");

    let owner = snapshot.owner_or(dusk_key);

    info!("{} Genesis Transfer Contract", theme.action("Deploying"));
    session.deploy(
        transfer_code,
        ContractData::builder()
            .owner(owner)
            .contract_id(TRANSFER_CONTRACT),
        u64::MAX,
    )?;

    info!("{} Genesis Stake Contract", theme.action("Deploying"));
    session.deploy(
        stake_code,
        ContractData::builder()
            .owner(owner)
            .contract_id(STAKE_CONTRACT),
        u64::MAX,
    )?;

    session
        .call::<_, ()>(
            STAKE_CONTRACT,
            "insert_stake",
            &(StakeKeys::single_key(dusk_key), StakeData::default()),
            u64::MAX,
        )
        .expect("stake to be inserted into the state");

    session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "add_contract_balance",
            &(STAKE_CONTRACT, 0u64),
            u64::MAX,
        )
        .expect("stake contract balance to be set with provisioner stakes");

    session
        .call::<_, ()>(TRANSFER_CONTRACT, "update_root", &(), u64::MAX)
        .expect("root to be updated after pushing genesis note");

    let commit_id = session.commit()?;

    info!("{} {}", theme.action("Empty Root"), hex::encode(commit_id));

    Ok((vm, commit_id))
}

/// Deploys a snapshot.
/// note: deploy consumes session as it produces commit id so it gives
/// the caller a possibility of providing a closure to perform additional
/// operations on the session (an empty closure is required when this is not
/// needed).
pub fn deploy<P: AsRef<Path>, F>(
    state_dir: P,
    snapshot: &Snapshot,
    dusk_key: AccountPublicKey,
    closure: F,
) -> Result<(VM, [u8; 32]), Box<dyn Error>>
where
    F: FnOnce(&mut Session),
{
    let theme = Theme::default();

    let state_dir = state_dir.as_ref();
    let state_id_path = rusk_profile::to_rusk_state_id_path(state_dir);

    let (vm, old_commit_id) = match snapshot.base_state() {
        Some(state) => load_state(state_dir, state),
        None => generate_empty_state(state_dir, snapshot, dusk_key),
    }?;

    let mut session =
        vm.session(old_commit_id, GENESIS_CHAIN_ID, GENESIS_BLOCK_HEIGHT)?;

    generate_transfer_state(&mut session, snapshot)?;
    generate_stake_state(&mut session, snapshot)?;

    closure(&mut session);

    info!("{} persisted id", theme.success("Storing"));
    let commit_id = session.commit()?;
    fs::write(state_id_path, commit_id)?;

    if old_commit_id != commit_id {
        info!(
            "{} {}",
            theme.action("Finalizing"),
            hex::encode(old_commit_id)
        );
        vm.finalize_commit(old_commit_id)?;
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

    let vm = VM::new(state_dir)?;
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

#[cfg(test)]
mod tests {

    use std::error::Error;

    use dusk_bytes::DeserializableSlice;

    use super::*;

    pub(crate) fn mainnet_from_file() -> Result<Snapshot, Box<dyn Error>> {
        let toml = include_str!("../config/mainnet.toml");
        let snapshot = toml::from_str(toml)?;
        Ok(snapshot)
    }

    fn dusk_mainnet_key() -> AccountPublicKey {
        let bytes = include_bytes!("../../rusk/src/assets/dusk.cpk");
        AccountPublicKey::from_slice(&bytes[..])
            .expect("faucet should have a valid key")
    }

    #[test]
    fn mainnet_genesis() -> Result<(), Box<dyn Error>> {
        let mainnet = mainnet_from_file()?;
        let tmp = tempfile::TempDir::with_prefix("genesis")
            .expect("Should be able to create temporary directory");
        let (_, root) =
            deploy(tmp.path(), &mainnet, dusk_mainnet_key(), |_| {})?;
        let root = hex::encode(root);
        let mainnet_root =
            "d90d03cf808252037ac2fdd8677868e1ac419caab09ec4cf0e87eafa86b8a612";
        assert_eq!(root, mainnet_root);

        Ok(())
    }
}
