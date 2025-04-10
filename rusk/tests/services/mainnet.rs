// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use dusk_core::dusk;
use dusk_core::stake::{StakeData, StakeKeys, STAKE_CONTRACT};
use dusk_core::transfer::moonlight::AccountData;
use dusk_core::transfer::phoenix::NoteLeaf;
use dusk_core::transfer::TRANSFER_CONTRACT;
use rusk::node::RuskVmConfig;
use rusk::{Result, Rusk};
use tempfile::tempdir;

use crate::common::logger;
use crate::common::state::new_state;

const GENESIS_BALANCE: u64 = dusk(500_000_000.0);

// Creates the Rusk initial state for the tests below
async fn initial_state<P: AsRef<Path>>(dir: P) -> Result<Rusk> {
    let snapshot = toml::from_str(include_str!(
        "../../../rusk-recovery/config/mainnet.toml"
    ))
    .expect("Cannot deserialize config");

    new_state(dir, &snapshot, RuskVmConfig::default()).await
}

#[tokio::test(flavor = "multi_thread")]
pub async fn mainnet_genesis() -> Result<()> {
    // Setup the logger
    logger();

    let tmp = tempdir().expect("Should be able to create temporary directory");
    let rusk = initial_state(&tmp).await?;
    let mut total_amount = 0u64;

    let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
        mpsc::channel();
    rusk.feeder_query(STAKE_CONTRACT, "stakes", &(), sender, None)?;
    for bytes in receiver.into_iter() {
        let (_, data) = rkyv::from_bytes::<(StakeKeys, StakeData)>(&bytes)
            .expect(
                "The contract should only return (StakeKeys, StakeData) tuples",
            );
        total_amount += data.amount.unwrap_or_default().total_funds();
    }

    let skate_balance: u64 = rusk
        .query(TRANSFER_CONTRACT, "contract_balance", &STAKE_CONTRACT)
        .expect("Query to succeed");
    assert_eq!(
        total_amount, skate_balance,
        "Total stake amount should match"
    );

    let sync_range = (0u64, u64::MAX);
    let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
        std::sync::mpsc::channel();
    rusk.feeder_query(
        TRANSFER_CONTRACT,
        "sync_accounts",
        &sync_range,
        sender,
        None,
    )?;
    for bytes in receiver.into_iter() {
        let (data, _) = rkyv::from_bytes::<(AccountData, [u8; 193])>(&bytes)
            .expect(
            "The contract should only return (AccountData, [u8; 193]) tuples",
        );
        total_amount += data.balance;
    }

    let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) =
        std::sync::mpsc::channel();
    rusk.feeder_query(TRANSFER_CONTRACT, "sync", &sync_range, sender, None)?;
    for bytes in receiver.into_iter() {
        let leaf: NoteLeaf = rkyv::from_bytes(&bytes)
            .expect("The contract should only return NoteLeaf");

        total_amount += leaf.note.value(None).expect("Transparent note");
    }

    assert_eq!(total_amount, GENESIS_BALANCE, "Total amount should match");

    Ok(())
}
