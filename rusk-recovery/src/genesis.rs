// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::provisioners::PROVISIONERS;
use crate::theme::Theme;

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use lazy_static::lazy_static;
use microkelvin::{Backend, BackendCtor, Persistence};
use phoenix_core::Note;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rusk_abi::dusk::*;
use rusk_vm::{Contract, NetworkState, NetworkStateId};
use stake_contract::{Stake, StakeContract, MINIMUM_STAKE};
use std::error::Error;
use tracing::info;
use transfer_contract::TransferContract;

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
        let note = Note::transparent(&mut rng, &*FAUCET_KEY, FAUCET_DUSK);
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
