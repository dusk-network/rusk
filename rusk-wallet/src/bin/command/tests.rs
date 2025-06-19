// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::dusk;
use rusk_wallet::GraphQL;

use super::*;
use crate::command::history::TransactionDirection;
use crate::status;

mod utils;
use utils::*;

#[tokio::test]
async fn test_empty_history() {
    configure_logger();
    wait_for_nodes_to_start().await.unwrap();
    let (mut wallet, settings) = create_wallet().await.unwrap();
    let cmd = Command::History { profile_idx: None };
    let history = cmd.run(&mut wallet, &settings).await.unwrap();
    assert!(matches!(history, RunResult::History(_)));
    let RunResult::History(tx_history) = history else {
        unreachable!();
    };
    assert_eq!(tx_history, vec![]);
}

#[tokio::test]
async fn test_history_transfer_convert() {
    configure_logger();
    wait_for_nodes_to_start().await.unwrap();

    let (mut wallet, settings) = create_wallet().await.unwrap();
    let (other_wallet, _) = create_wallet().await.unwrap();
    let gql = GraphQL::new(
        settings.state.clone(),
        settings.archiver.clone(),
        status::headless,
    )
    .unwrap();
    let moonlight_addr = wallet.default_address();
    let phoenix_addr = wallet.shielded_account(0).unwrap();
    let gas_price = 1;

    let rcv_moonlight = rcv_moonlight_from_faucet(
        moonlight_addr.clone(),
        dusk(2750.0),
        gas_price,
    )
    .await
    .unwrap();

    let rcv_phoenix =
        rcv_phoenix_from_faucet(phoenix_addr.clone(), dusk(2500.0), gas_price)
            .await
            .unwrap();

    let mut txs_info = wait_for_tx_blocks_to_finalize(
        &gql,
        vec![&rcv_moonlight, &rcv_phoenix],
    )
    .await
    .unwrap();

    let moonlight_trans_to_other_wallet = transfer_moonlight(
        &mut wallet,
        other_wallet.default_address(),
        &settings,
        4_000,
        gas_price,
    )
    .await
    .unwrap();
    // Need to wait for each transaction to be included in a block before moving
    // to the next to ensure that the history ends up in the same order as the
    // transactions made.
    gql.wait_for(&moonlight_trans_to_other_wallet)
        .await
        .unwrap();

    let phoenix_trans_to_other_wallet = transfer_phoenix(
        &mut wallet,
        other_wallet.default_shielded_account(),
        &settings,
        3_000,
        gas_price,
    )
    .await
    .unwrap();
    gql.wait_for(&phoenix_trans_to_other_wallet).await.unwrap();

    let moonlight_to_phoenix = convert_moonlight_to_phoenix(
        &mut wallet,
        &settings,
        Dusk::new(2_500),
        gas_price,
    )
    .await
    .unwrap();
    gql.wait_for(&moonlight_to_phoenix).await.unwrap();

    let phoenix_to_moonlight = convert_phoenix_to_moonlight(
        &mut wallet,
        &settings,
        Dusk::new(5_000),
        gas_price,
    )
    .await
    .unwrap();
    gql.wait_for(&phoenix_to_moonlight).await.unwrap();

    txs_info.extend(
        wait_for_tx_blocks_to_finalize(
            &gql,
            vec![
                &moonlight_trans_to_other_wallet,
                &phoenix_trans_to_other_wallet,
                &phoenix_to_moonlight,
                &moonlight_to_phoenix,
            ],
        )
        .await
        .unwrap(),
    );

    let cmd = Command::History { profile_idx: None };
    let history = cmd.run(&mut wallet, &settings).await.unwrap();
    assert!(matches!(history, RunResult::History(_)));
    let RunResult::History(tx_history) = history else {
        unreachable!();
    };
    let tx_history: Vec<StrippedTxHistoryItem> =
        tx_history.into_iter().map(|item| item.into()).collect();

    assert_eq!(
        tx_history,
        vec![
            // Receive money from faucet to moonlight address
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: dusk(2750.0) as f64,
                fee: txs_info[&rcv_moonlight].gas_spent * gas_price,
                action: "transfer".to_string(),
            },
            // Receive money from faucet to phoenix address
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: dusk(2500.0) as f64,
                fee: txs_info[&rcv_phoenix].gas_spent * gas_price,
                action: "transfer".to_string(),
            },
            // Send 4000 to other wallet
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -4_000.0,
                fee: txs_info[&moonlight_trans_to_other_wallet].gas_spent
                    * gas_price,
                action: "transfer".to_string(),
            },
            // Send 3000 to other wallet
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -3_000.0,
                fee: txs_info[&phoenix_trans_to_other_wallet].gas_spent
                    * gas_price,
                action: "transfer".to_string(),
            },
            // Receive converted 2500 from moonlight to phoenix
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: 2_500.0,
                fee: 0,
                action: "convert".to_string(),
            },
            // Convert 2500 from moonlight to phoenix
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -2_500.0,
                fee: txs_info[&moonlight_to_phoenix].gas_spent * gas_price,
                action: "convert".to_string(),
            },
            // Convert 5000 from phoenix to moonlight
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -5_000.0,
                fee: txs_info[&phoenix_to_moonlight].gas_spent * gas_price,
                action: "convert".to_string(),
            },
            // Receive converted 5000 from phoenix to moonlight
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: 5_000.0,
                fee: 0,
                action: "convert".to_string(),
            },
        ]
    );
}

#[tokio::test]
async fn test_history_stake_unstake() {
    configure_logger();
    wait_for_nodes_to_start().await.unwrap();

    let (mut wallet, settings) = create_wallet().await.unwrap();
    let gql = GraphQL::new(
        settings.state.clone(),
        settings.archiver.clone(),
        status::headless,
    )
    .unwrap();
    let moonlight_addr = wallet.default_address();
    let phoenix_addr = wallet.shielded_account(0).unwrap();
    let gas_price = 1;

    let rcv_moonlight = rcv_moonlight_from_faucet(
        moonlight_addr.clone(),
        dusk(1500.0),
        gas_price,
    )
    .await
    .unwrap();

    let rcv_phoenix =
        rcv_phoenix_from_faucet(phoenix_addr.clone(), dusk(1500.0), gas_price)
            .await
            .unwrap();

    let stake_moonlight =
        stake_moonlight(&mut wallet, &settings, dusk(1000.0).into(), gas_price)
            .await
            .unwrap();
    gql.wait_for(&stake_moonlight).await.unwrap();

    let unstake_moonlight =
        unstake_moonlight(&mut wallet, &settings, gas_price)
            .await
            .unwrap();
    gql.wait_for(&unstake_moonlight).await.unwrap();

    let stake_phoenix =
        stake_phoenix(&mut wallet, &settings, dusk(1000.0).into(), gas_price)
            .await
            .unwrap();
    gql.wait_for(&stake_phoenix).await.unwrap();

    let unstake_phoenix = unstake_phoenix(&mut wallet, &settings, gas_price)
        .await
        .unwrap();

    let txs_info = wait_for_tx_blocks_to_finalize(
        &gql,
        vec![
            &rcv_moonlight,
            &rcv_phoenix,
            &stake_moonlight,
            &unstake_moonlight,
            &stake_phoenix,
            &unstake_phoenix,
        ],
    )
    .await
    .unwrap();

    let cmd = Command::History { profile_idx: None };
    let history = cmd.run(&mut wallet, &settings).await.unwrap();
    assert!(matches!(history, RunResult::History(_)));
    let RunResult::History(tx_history) = history else {
        unreachable!();
    };
    let tx_history: Vec<StrippedTxHistoryItem> =
        tx_history.into_iter().map(|item| item.into()).collect();

    assert_eq!(
        tx_history,
        vec![
            // Receive money from faucet to moonlight address
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: dusk(1500.0) as f64,
                fee: txs_info[&rcv_moonlight].gas_spent * gas_price,
                action: "transfer".to_string(),
            },
            // Receive money from faucet to phoenix address
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: dusk(1500.0) as f64,
                fee: txs_info[&rcv_phoenix].gas_spent * gas_price,
                action: "transfer".to_string(),
            },
            // Stake 1000 dusk with moonlight
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -(dusk(1000.0) as f64),
                fee: txs_info[&stake_moonlight].gas_spent * gas_price,
                action: "stake".to_string(),
            },
            // Unstake with moonlight
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: dusk(1000.0) as f64,
                fee: txs_info[&unstake_moonlight].gas_spent * gas_price,
                action: "unstake".to_string(),
            },
            // Stake 1000 dusk with phoenix
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -(dusk(1000.0) as f64),
                fee: txs_info[&stake_phoenix].gas_spent * gas_price,
                action: "stake".to_string(),
            },
            // Unstake with phoenix
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: dusk(1000.0) as f64,
                fee: txs_info[&unstake_phoenix].gas_spent * gas_price,
                action: "unstake".to_string(),
            },
        ]
    )
}
