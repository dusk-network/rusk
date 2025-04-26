// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for JSON-RPC block models: `BlockHeader`, `BlockStatus`, `Block`.

use crate::jsonrpc::utils::{create_mock_block, create_mock_ml_tx_response};
use hex;
use rusk::jsonrpc::model::block::{Block, BlockHeader, BlockStatus};

#[test]
fn block_status_equality() {
    assert_eq!(BlockStatus::Final, BlockStatus::Final);
    assert_eq!(BlockStatus::Provisional, BlockStatus::Provisional);
    assert_ne!(BlockStatus::Final, BlockStatus::Provisional);
}

#[test]
fn block_header_equality() {
    let header1 = create_mock_block(10, "h1").header;
    let header2 = create_mock_block(10, "h1").header; // Identical
    let header3 = create_mock_block(11, "h1").header; // Different height
                                                      // header4 is now identical to header1 because hash_prefix is ignored
                                                      // let header4 = create_mock_block(10, "h2").header;

    assert_eq!(header1, header2);
    assert_ne!(header1, header3);
    // assert_ne!(header1, header4); // This assertion is no longer valid
}

#[test]
fn block_equality() {
    let block1 = create_mock_block(20, "b1");
    let block2 = create_mock_block(20, "b1"); // Identical
    let block3 = create_mock_block(21, "b1"); // Different header (height)
    let mut block4 = create_mock_block(20, "b1");
    block4.status = Some(BlockStatus::Provisional); // Different status

    assert_eq!(block1, block2);
    assert_ne!(block1, block3);
    assert_ne!(block1, block4);
}

#[test]
fn block_header_serialization() {
    let header = create_mock_block(10, "h1").header;
    let json = serde_json::to_value(header).unwrap();

    // Use deterministic hashes based on height 10
    let hash_10_hex = hex::encode([10u8; 32]);
    let prev_hash_9_hex = hex::encode([9u8; 32]);
    let state_hash_str = format!("state_{}", hash_10_hex);
    let txroot_str = format!("txroot_{}", hash_10_hex);
    let seed_str = format!("seed_{}", hash_10_hex);

    assert_eq!(json["version"], 1);
    assert_eq!(json["height"], "10"); // Serialized as string
    assert_eq!(json["previous_hash"], prev_hash_9_hex);
    assert_eq!(json["timestamp"], "1600010000"); // Serialized as string
    assert_eq!(json["hash"], hash_10_hex);
    assert_eq!(json["state_hash"], state_hash_str);
    assert_eq!(json["validator"], "validator_base58_key");
    assert_eq!(json["transactions_root"], txroot_str);
    assert_eq!(json["gas_limit"], "100000"); // Serialized as string
    assert_eq!(json["seed"], seed_str);
    assert_eq!(json["sequence"], 1);
}

#[test]
fn block_serialization_no_txs() {
    let block = create_mock_block(20, "b1");
    let json = serde_json::to_value(block).unwrap();

    assert!(json["header"].is_object());
    assert_eq!(json["header"]["hash"], hex::encode([20u8; 32])); // Use deterministic hash
    assert_eq!(json["header"]["height"], "20"); // Check height serialization
    assert_eq!(json["status"], "Final");
    assert!(json["transactions"].is_null()); // None -> null, not skipped yet
    assert_eq!(json["transactions_count"], 0);
    assert_eq!(json["block_reward"], "5000"); // Serialized as string
    assert_eq!(json["total_gas_limit"], "50000"); // Serialized as string
}

#[test]
fn block_serialization_with_txs() {
    let mut block = create_mock_block(30, "b_tx");
    let tx1 = create_mock_ml_tx_response("tx1_in_block");
    let tx2 = create_mock_ml_tx_response("tx2_in_block");
    block.transactions = Some(vec![tx1.clone(), tx2.clone()]);
    block.transactions_count = 2;

    let json = serde_json::to_value(&block).unwrap();

    // Use deterministic hash for height 30
    let hash_30_hex = hex::encode([30u8; 32]);

    assert!(json["header"].is_object());
    assert_eq!(json["header"]["hash"], hash_30_hex);
    assert_eq!(json["status"], "Final");
    assert_eq!(json["transactions_count"], 2);

    // Check transactions array
    assert!(json["transactions"].is_array());
    let txs_json = json["transactions"].as_array().unwrap();
    assert_eq!(txs_json.len(), 2);

    // Check some fields of the first transaction
    assert_eq!(txs_json[0]["tx_hash"], "tx1_in_block");
    assert_eq!(txs_json[0]["tx_type"], "Moonlight");
    assert!(txs_json[0]["status"].is_string()); // Check status is present
    assert_eq!(txs_json[0]["transaction_data"]["value"], "1000");

    // Check hash of the second transaction
    assert_eq!(txs_json[1]["tx_hash"], "tx2_in_block");
}

#[test]
fn block_status_deserialization() {
    let final_status: BlockStatus = serde_json::from_str("\"Final\"").unwrap();
    assert_eq!(final_status, BlockStatus::Final);

    let prov_status: BlockStatus =
        serde_json::from_str("\"Provisional\"").unwrap();
    assert_eq!(prov_status, BlockStatus::Provisional);

    let invalid = serde_json::from_str::<BlockStatus>("\"Unknown\"");
    assert!(invalid.is_err());
}

#[test]
fn block_header_deserialization() {
    // Use deterministic hashes based on height 10
    let hash_10_hex = hex::encode([10u8; 32]);
    let prev_hash_9_hex = hex::encode([9u8; 32]);
    let state_hash_str = format!("state_{}", hash_10_hex);
    let txroot_str = format!("txroot_{}", hash_10_hex);
    let seed_str = format!("seed_{}", hash_10_hex);

    let json_str = format!(
        r#"
    {{
        "version": 1,
        "height": "10",
        "previous_hash": "{}",
        "timestamp": "1600010000",
        "hash": "{}",
        "state_hash": "{}",
        "validator": "validator_base58_key",
        "transactions_root": "{}",
        "gas_limit": "100000",
        "seed": "{}",
        "sequence": 1
    }}
    "#,
        prev_hash_9_hex, hash_10_hex, state_hash_str, txroot_str, seed_str
    );

    let deserialized_header: BlockHeader =
        serde_json::from_str(&json_str).unwrap();
    let expected_header = create_mock_block(10, "h1").header;

    assert_eq!(deserialized_header, expected_header);
}

#[test]
fn block_deserialization_no_txs() {
    // Use deterministic hash
    let hash_hex = hex::encode([20u8; 32]);
    let prev_hash_hex = hex::encode([19u8; 32]);
    let state_hash_str = format!("state_{}", hash_hex);
    let txroot_str = format!("txroot_{}", hash_hex);
    let seed_str = format!("seed_{}", hash_hex);

    let json_str = format!(
        r#"
    {{
        "header": {{
            "version": 1,
            "height": "20",
            "previous_hash": "{}",
            "timestamp": "1600020000",
            "hash": "{}",
            "state_hash": "{}",
            "validator": "validator_base58_key",
            "transactions_root": "{}",
            "gas_limit": "100000",
            "seed": "{}",
            "sequence": 1
        }},
        "status": "Final",
        "transactions_count": 0,
        "block_reward": "5000",
        "total_gas_limit": "50000"
    }}
    "#,
        prev_hash_hex, hash_hex, state_hash_str, txroot_str, seed_str
    );

    // Note: `transactions` field is missing, which deserialize correctly to
    // None Obsolete fields total_fees and total_gas_spent are removed from
    // JSON
    let deserialized_block: Block = serde_json::from_str(&json_str).unwrap();
    let expected_block = create_mock_block(20, "b1"); // Helper creates with transactions=None

    assert_eq!(deserialized_block.header, expected_block.header);
    assert_eq!(deserialized_block.status, expected_block.status);
    assert_eq!(
        deserialized_block.transactions_count,
        expected_block.transactions_count
    );
    assert_eq!(deserialized_block.block_reward, expected_block.block_reward);
    assert_eq!(
        deserialized_block.total_gas_limit,
        expected_block.total_gas_limit
    );
    assert!(deserialized_block.transactions.is_none()); // Explicit check
}

#[test]
fn block_deserialization_with_txs() {
    // Create expected block using helpers
    let mut expected_block = create_mock_block(30, "b_tx");
    let tx1 = create_mock_ml_tx_response("tx1_in_block");
    let tx2 = create_mock_ml_tx_response("tx2_in_block");
    expected_block.transactions = Some(vec![tx1.clone(), tx2.clone()]);
    expected_block.transactions_count = 2;

    // Serialize the expected block to get a valid JSON string
    let json_str = serde_json::to_string(&expected_block).unwrap();

    // Deserialize back
    let deserialized_block: Block = serde_json::from_str(&json_str).unwrap();

    // Compare the deserialized block with the original
    assert_eq!(deserialized_block, expected_block);
    assert!(deserialized_block.transactions.is_some());
    assert_eq!(deserialized_block.transactions.unwrap().len(), 2);
}
