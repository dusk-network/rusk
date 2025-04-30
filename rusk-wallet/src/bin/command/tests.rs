// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

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
async fn test_non_empty_history() {
    configure_logger();
    wait_for_nodes_to_start().await.unwrap();

    let (mut wallet, settings) = create_wallet().await.unwrap();
    let (other_wallet, _) = create_wallet().await.unwrap();
    let gql = GraphQL::new(settings.state.clone(), status::headless).unwrap();
    let moonlight_addr = wallet.default_address();
    let phoenix_addr = wallet.shielded_account(0).unwrap();
    let gas_price = 1;

    let rcv_moonlight = rcv_moonlight_from_faucet(
        moonlight_addr.clone(),
        6_000_000_000,
        gas_price,
    )
    .await
    .unwrap();
    // Need to wait for each transaction to be included in a block before moving
    // to the next to ensure that the history ends up in the same order as the
    // transactions made.
    gql.wait_for(&rcv_moonlight).await.unwrap();

    let rcv_phoenix =
        rcv_phoenix_from_faucet(phoenix_addr.clone(), 4_000_000_000, gas_price)
            .await
            .unwrap();
    gql.wait_for(&rcv_phoenix).await.unwrap();

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
                amount: 6_000_000_000.0,
                fee: txs_info[&rcv_moonlight].gas_spent * gas_price,
            },
            // Receive money from faucet to phoenix address
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: 4_000_000_000.0,
                fee: txs_info[&rcv_phoenix].gas_spent * gas_price,
            },
            // Send 4000 to other wallet
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -4_000.0,
                fee: txs_info[&moonlight_trans_to_other_wallet].gas_spent
                    * gas_price,
            },
            // Send 3000 to other wallet
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -3_000.0,
                fee: txs_info[&phoenix_trans_to_other_wallet].gas_spent
                    * gas_price,
            },
            // Receive converted 2500 from moonlight to phoenix
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: 2_500.0,
                fee: 0,
            },
            // Convert 2500 from moonlight to phoenix
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -2_500.0,
                fee: txs_info[&moonlight_to_phoenix].gas_spent * gas_price,
            },
            // Convert 5000 from phoenix to moonlight
            StrippedTxHistoryItem {
                direction: TransactionDirection::Out,
                amount: -5_000.0,
                fee: txs_info[&phoenix_to_moonlight].gas_spent * gas_price,
            },
            // Receive converted 5000 from phoenix to moonlight
            StrippedTxHistoryItem {
                direction: TransactionDirection::In,
                amount: 5_000.0,
                fee: 0,
            },
        ]
    );
}
