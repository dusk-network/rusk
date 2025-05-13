// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::model;

/// Creates a mock `Block` for testing with basic fields populated.
pub fn create_basic_mock_block(
    height: u64,
    _hash_prefix: &str,
) -> model::block::Block {
    // Use a simple, deterministic hex hash based on height
    let hash_bytes = [height as u8; 32];
    let hash = hex::encode(hash_bytes);
    let prev_hash = hex::encode([(height.saturating_sub(1)) as u8; 32]);

    model::block::Block {
        header: model::block::BlockHeader {
            version: 1,
            height,
            previous_hash: prev_hash, // Deterministic prev hash
            timestamp: 1_600_000_000 + height * 1000,
            hash: hash.clone(), // Use deterministic hash
            state_hash: format!("state_{}", hash),
            validator: "validator_base58_key".to_string(),
            transactions_root: format!("txroot_{}", hash),
            gas_limit: 100_000,
            seed: format!("seed_{}", hash),
            sequence: 1,
        },
        status: Some(model::block::BlockStatus::Final),
        transactions: None,
        faults: None,
        transactions_count: 0,
        block_reward: Some(5000),
        total_gas_limit: Some(50_000),
    }
}

/// Creates a mock `model::block::Block` with optional transactions for testing
/// the RPC service layer.
pub fn create_mock_model_block_with_optional_transactions(
    hash_str: &str,
    height: u64,
    include_transactions: bool,
) -> model::block::Block {
    // Create mock header according to model::block::BlockHeader fields
    let mock_model_header = model::block::BlockHeader {
        version: 1,
        height,
        previous_hash: hex::encode([(height.saturating_sub(1)) as u8; 32]),
        timestamp: 1_600_000_000 + height * 1000,
        hash: hash_str.to_string(),
        state_hash: hex::encode([height as u8; 32]),
        validator: bs58::encode([height as u8; 96]).into_string(),
        transactions_root: hex::encode([height as u8; 32]),
        gas_limit: 1_000_000,
        seed: hex::encode([height as u8; 33]),
        sequence: 1,
    };

    // Create mock transactions responses ONLY if requested
    let transactions: Option<Vec<model::transaction::TransactionResponse>> =
        if include_transactions {
            // Create one mock transaction response
            let mock_base_tx = model::transaction::BaseTransaction {
                tx_hash: format!("tx_hash_{}_{}", hash_str, height),
                version: 1,
                tx_type: model::transaction::TransactionType::Moonlight,
                gas_price: 10,
                gas_limit: 21000,
                raw: hex::encode(vec![0; 100]),
            };
            let mock_tx_status = model::transaction::TransactionStatus {
                status: model::transaction::TransactionStatusType::Executed,
                block_height: Some(height),
                block_hash: Some(hash_str.to_string()),
                gas_spent: Some(21000),
                timestamp: Some(1_600_000_000 + height * 1000 + 10),
                error: None,
            };
            let mock_tx_response = model::transaction::TransactionResponse {
                base: mock_base_tx,
                status: Some(mock_tx_status),
                transaction_data:
                    model::transaction::TransactionDataType::Moonlight(
                        model::transaction::MoonlightTransactionData {
                            sender: "mock_sender_address".to_string(),
                            receiver: Some("mock_receiver_address".to_string()),
                            value: 1000,
                            nonce: 1,
                            memo: Some("Mock memo".to_string()),
                        },
                    ),
            };
            Some(vec![mock_tx_response])
        } else {
            None // Explicitly None if not including transactions
        };

    // Calculate transaction count based on the Option<Vec<...>>
    let transactions_count = transactions
        .as_ref()
        .map(|txs| txs.len() as u64)
        .unwrap_or_default();

    // Construct the final model::block::Block according to its fields
    model::block::Block {
        header: mock_model_header,
        status: Some(model::block::BlockStatus::Final),
        transactions,
        faults: None,
        transactions_count,
        block_reward: Some(1000),
        total_gas_limit: Some(1_000_000),
    }
}

/// Creates a mock `MoonlightEventGroup` for testing.
pub fn create_mock_moonlight_group(
    tx_hash_prefix: &str,
    block_height: u64,
) -> model::archive::MoonlightEventGroup {
    model::archive::MoonlightEventGroup {
        origin: format!("{}_{}", tx_hash_prefix, block_height),
        block_height,
        events: vec![], // Keep it simple for mock tests
    }
}

/// Helper to create a simple Moonlight Tx Response for testing.
pub fn create_mock_ml_tx_response(
    hash: &str,
) -> model::transaction::TransactionResponse {
    model::transaction::TransactionResponse {
        base: model::transaction::BaseTransaction {
            tx_hash: hash.into(),
            version: 1,
            tx_type: model::transaction::TransactionType::Moonlight,
            gas_price: 10,
            gas_limit: 1000,
            raw: format!("raw_{}", hash),
        },
        status: Some(model::transaction::TransactionStatus {
            status: model::transaction::TransactionStatusType::Executed,
            block_height: Some(101),
            block_hash: Some(format!("bh_{}", hash)),
            gas_spent: Some(800),
            timestamp: Some(54321),
            error: None,
        }),
        transaction_data: model::transaction::TransactionDataType::Moonlight(
            model::transaction::MoonlightTransactionData {
                sender: "sender".to_string(),
                receiver: Some("receiver".to_string()),
                value: 1000,
                nonce: 5,
                memo: Some("memo".to_string()),
            },
        ),
    }
}
