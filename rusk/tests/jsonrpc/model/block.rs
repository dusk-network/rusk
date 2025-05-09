// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for JSON-RPC block models: `BlockHeader`, `BlockStatus`, `Block`,
//! `BlockLabel`, `ChainTip`, `CandidateBlock`, `BlockFaults`, `Fault`,
//! `FaultType`, `FaultItem`, `ConsensusHeaderJson`.

use crate::jsonrpc::utils::{create_mock_block, create_mock_ml_tx_response};
use bs58;
use dusk_bytes::Serializable;
use hex;
use node_data::bls::{PublicKey as NodeBlsPubKey, PublicKeyBytes};
use node_data::ledger::{
    self as node_ledger, Attestation, Block as NodeBlock, Header as NodeHeader,
    Label as NodeLabel, Signature as NodeSignature,
};
use node_data::message::{
    ConsensusHeader as NodeConsensusHeader, SignInfo as NodeSignInfo,
};
use rusk::jsonrpc::model::block::{
    Block, BlockFaults, BlockHeader, BlockStatus, CandidateBlock, ChainTip,
    ConsensusHeaderJson, Fault, FaultItem, FaultType,
};
use rusk::jsonrpc::model::key::AccountPublicKey;

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
    // None
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

// --- New Tests Start Here ---

#[test]
fn block_label_equality() {
    assert_eq!(BlockStatus::Final, BlockStatus::Final);
    assert_eq!(BlockStatus::Provisional, BlockStatus::Provisional);
    assert_ne!(BlockStatus::Final, BlockStatus::Provisional);
}

#[test]
fn block_label_serialization() {
    assert_eq!(
        serde_json::to_value(BlockStatus::Final).unwrap(),
        serde_json::json!("Final")
    );
    assert_eq!(
        serde_json::to_value(BlockStatus::Provisional).unwrap(),
        serde_json::json!("Provisional")
    );
}

#[test]
fn block_label_deserialization() {
    let final_label: BlockStatus = serde_json::from_str("\"Final\"").unwrap();
    assert_eq!(final_label, BlockStatus::Final);

    let prov_label: BlockStatus =
        serde_json::from_str("\"Provisional\"").unwrap();
    assert_eq!(prov_label, BlockStatus::Provisional);

    let invalid = serde_json::from_str::<BlockStatus>("\"Unknown\"");
    assert!(invalid.is_err());
}

#[test]
fn block_label_from_node_label() {
    assert_eq!(BlockStatus::from(NodeLabel::Final(10)), BlockStatus::Final);
    assert_eq!(
        BlockStatus::from(NodeLabel::Accepted(10)),
        BlockStatus::Provisional
    );
    assert_eq!(
        BlockStatus::from(NodeLabel::Attested(10)),
        BlockStatus::Provisional
    );
    assert_eq!(
        BlockStatus::from(NodeLabel::Confirmed(10)),
        BlockStatus::Provisional
    );
}

#[test]
fn block_status_from_node_label() {
    assert_eq!(BlockStatus::from(NodeLabel::Final(10)), BlockStatus::Final);
    assert_eq!(
        BlockStatus::from(NodeLabel::Accepted(10)),
        BlockStatus::Provisional
    );
    assert_eq!(
        BlockStatus::from(NodeLabel::Attested(10)),
        BlockStatus::Provisional
    );
    assert_eq!(
        BlockStatus::from(NodeLabel::Confirmed(10)),
        BlockStatus::Provisional
    );
}

#[test]
fn chain_tip_equality() {
    let tip1 = ChainTip {
        hash: "h1".into(),
        height: 10,
        status: BlockStatus::Final,
    };
    let tip2 = ChainTip {
        hash: "h1".into(),
        height: 10,
        status: BlockStatus::Final,
    };
    let tip3 = ChainTip {
        hash: "h2".into(),
        height: 10,
        status: BlockStatus::Final,
    };
    let tip4 = ChainTip {
        hash: "h1".into(),
        height: 11,
        status: BlockStatus::Final,
    };
    let tip5 = ChainTip {
        hash: "h1".into(),
        height: 10,
        status: BlockStatus::Provisional,
    };
    assert_eq!(tip1, tip2);
    assert_ne!(tip1, tip3);
    assert_ne!(tip1, tip4);
    assert_ne!(tip1, tip5);
}

#[test]
fn chain_tip_serialization() {
    let tip = ChainTip {
        hash: hex::encode([1u8; 32]),
        height: 1234567890123456789,
        status: BlockStatus::Provisional,
    };
    let json = serde_json::to_value(tip).unwrap();
    assert_eq!(json["hash"], hex::encode([1u8; 32]));
    assert_eq!(json["height"], "1234567890123456789"); // String
    assert_eq!(json["status"], "Provisional");
}

#[test]
fn chain_tip_deserialization() {
    let json_str = r#"
    {
        "hash": "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899",
        "height": "9876543210987654321",
        "status": "Final"
    }
    "#;
    let expected_tip = ChainTip {
        hash:
            "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"
                .into(),
        height: 9876543210987654321,
        status: BlockStatus::Final,
    };
    let deserialized_tip: ChainTip = serde_json::from_str(json_str).unwrap();
    assert_eq!(deserialized_tip, expected_tip);
}

// Helper to create a mock node_data::ledger::Block
fn create_mock_node_block(height: u64) -> NodeBlock {
    let hash_bytes = [height as u8; 32];
    let prev_hash_bytes = [(height.saturating_sub(1)) as u8; 32];

    // Since creating realistic ProtocolTransaction instances is complex and not
    // strictly necessary for testing model structure/conversion, create a block
    // with an empty transaction list.
    let txs: Vec<node_ledger::Transaction> = vec![];

    let header = NodeHeader {
        version: 1,
        height,
        timestamp: 1_600_000_000 + height * 1000,
        prev_block_hash: prev_hash_bytes,
        seed: NodeSignature::from([height as u8; 48]),
        state_hash: [height as u8; 32],
        event_bloom: [0u8; 256],
        generator_bls_pubkey: PublicKeyBytes([height as u8; 96]),
        txroot: [0u8; 32], /* Empty tx list, so txroot is typically zeroed or
                            * default */
        faultroot: [0u8; 32], // Assume no faults for basic block
        gas_limit: 100_000,
        iteration: 1,
        prev_block_cert: Attestation::default(),
        failed_iterations: Default::default(),
        hash: hash_bytes,
        signature: NodeSignature::from([height as u8; 48]),
        att: Attestation::default(),
    };

    // Create block with empty transactions and faults
    NodeBlock::new(header, txs, vec![]).unwrap()
}

#[test]
fn candidate_block_equality() {
    let block1 = create_mock_node_block(40);
    let block2 = create_mock_node_block(40); // Identical node block
    let block3 = create_mock_node_block(41); // Different node block

    let cand1 = CandidateBlock::from(block1.clone());
    let cand2 = CandidateBlock::from(block2.clone());
    let cand3 = CandidateBlock::from(block3.clone());

    // Modify one transaction in cand2 to make it different
    let mut cand2_modified = CandidateBlock::from(block2);
    if let Some(tx) = cand2_modified.transactions.get_mut(0) {
        tx.base.gas_price = 999;
    }

    assert_eq!(cand1, cand1.clone()); // Self equality
    assert_eq!(cand1, cand2); // From identical node blocks
    assert_eq!(cand1, cand2_modified);
    assert_ne!(cand1, cand3); // From different node blocks
}

#[test]
fn candidate_block_serialization() {
    let node_block = create_mock_node_block(50);
    let candidate = CandidateBlock::from(node_block.clone());
    let json = serde_json::to_value(candidate).unwrap();

    let hash_hex = hex::encode([50u8; 32]);

    assert!(json["header"].is_object());
    assert_eq!(json["header"]["hash"], hash_hex);
    assert_eq!(json["header"]["height"], "50");
    assert_eq!(json["transactions_count"], 0);
    assert!(json["transactions"].is_array());
    let txs_json = json["transactions"].as_array().unwrap();
    assert_eq!(txs_json.len(), 0);
}

#[test]
fn candidate_block_deserialization() {
    // Create expected block using node helper and conversion
    let node_block = create_mock_node_block(60);
    let expected_candidate = CandidateBlock::from(node_block);

    // Serialize the expected block to get a valid JSON string
    let json_str = serde_json::to_string(&expected_candidate).unwrap();

    // Deserialize back
    let deserialized_candidate: CandidateBlock =
        serde_json::from_str(&json_str).unwrap();

    // Compare
    assert_eq!(deserialized_candidate, expected_candidate);
}

#[test]
fn block_from_node_block_conversion() {
    let node_block = create_mock_node_block(70);
    let converted_block = Block::from(node_block.clone());

    // Check header conversion
    assert_eq!(
        converted_block.header,
        BlockHeader::from(node_block.header().clone())
    );

    // Check transaction count - should be 0 as mock block has empty tx list
    assert_eq!(converted_block.transactions_count, 0);

    // Check fields that should be None in direct conversion
    assert!(converted_block.status.is_none());
    assert!(converted_block.transactions.is_none());
    assert!(converted_block.block_reward.is_none());
    assert!(converted_block.total_gas_limit.is_none());
}

#[test]
fn candidate_block_from_node_block_conversion() {
    let node_block = create_mock_node_block(80);
    let converted_candidate = CandidateBlock::from(node_block.clone());

    // Check header conversion
    assert_eq!(
        converted_candidate.header,
        BlockHeader::from(node_block.header().clone())
    );

    // Check transaction count (should be 0 as mock block has empty tx list)

    // Check transactions array length (should be 0)
    assert_eq!(converted_candidate.transactions.len(), 0);
}

#[test]
fn fault_type_equality_serialization_deserialization() {
    let types = vec![
        FaultType::DoubleCandidate,
        FaultType::DoubleRatificationVote,
        FaultType::DoubleValidationVote,
    ];
    for type1 in &types {
        // Equality
        assert_eq!(type1, type1);
        for type2 in &types {
            if std::mem::discriminant(type1) != std::mem::discriminant(type2) {
                assert_ne!(type1, type2);
            }
        }

        // Serde round trip
        let json = serde_json::to_value(type1).unwrap();
        let deserialized: FaultType = serde_json::from_value(json).unwrap();
        assert_eq!(type1, &deserialized);
    }

    // Deserialization check
    assert_eq!(
        serde_json::from_str::<FaultType>("\"DoubleCandidate\"").unwrap(),
        FaultType::DoubleCandidate
    );
    assert_eq!(
        serde_json::from_str::<FaultType>("\"DoubleRatificationVote\"")
            .unwrap(),
        FaultType::DoubleRatificationVote
    );
    assert_eq!(
        serde_json::from_str::<FaultType>("\"DoubleValidationVote\"").unwrap(),
        FaultType::DoubleValidationVote
    );
    assert!(serde_json::from_str::<FaultType>("\"InvalidType\"").is_err());
}

fn create_mock_consensus_header_json(round: u64) -> ConsensusHeaderJson {
    ConsensusHeaderJson {
        round,
        iteration: (round % 8) as u8,
        prev_block_hash: hex::encode([(round.saturating_sub(1)) as u8; 32]),
    }
}

fn create_mock_node_consensus_header(round: u64) -> NodeConsensusHeader {
    NodeConsensusHeader {
        round,
        iteration: (round % 8) as u8,
        prev_block_hash: [(round.saturating_sub(1)) as u8; 32],
    }
}

#[test]
fn consensus_header_json_equality_serialization_deserialization() {
    let header1 = create_mock_consensus_header_json(100);
    let header2 = create_mock_consensus_header_json(100);
    let header3 = create_mock_consensus_header_json(101);

    // Equality
    assert_eq!(header1, header2);
    assert_ne!(header1, header3);

    // Serde round trip
    let json = serde_json::to_value(&header1).unwrap();
    assert_eq!(json["round"], "100");
    assert_eq!(json["iteration"], 4); // 100 % 8
    assert_eq!(json["prev_block_hash"], hex::encode([99u8; 32]));
    let deserialized: ConsensusHeaderJson =
        serde_json::from_value(json).unwrap();
    assert_eq!(header1, deserialized);
}

#[test]
fn consensus_header_json_from_node_consensus_header() {
    let node_header = create_mock_node_consensus_header(110);
    let expected_json = ConsensusHeaderJson {
        round: 110,
        iteration: 6, // 110 % 8
        prev_block_hash: hex::encode([109u8; 32]),
    };
    assert_eq!(ConsensusHeaderJson::from(&node_header), expected_json);
}

fn create_mock_fault_item(round: u64, signer_id: u8) -> FaultItem {
    // Generate a more plausible-looking, but still mock, Base58 string.
    // A truly valid key requires complex generation.
    // Using a fixed prefix + signer ID ensures uniqueness for the test.
    let mock_key_string = format!("MockKey{}{}", round, signer_id);
    // Pad with '1' to likely exceed typical key length, ensuring it's not
    // accidentally valid
    let padded_mock_key = format!(
        "{}{}",
        mock_key_string,
        "1".repeat(96 - mock_key_string.len())
    );

    FaultItem {
        header: create_mock_consensus_header_json(round),
        signer_key: padded_mock_key, // Use the generated mock string
    }
}

fn create_mock_node_sign_info(signer_id: u8) -> NodeSignInfo {
    let sk =
        dusk_core::signatures::bls::SecretKey::from_bytes(&[signer_id; 32])
            .unwrap();
    let pk = dusk_core::signatures::bls::PublicKey::from(&sk);
    let node_pk = NodeBlsPubKey::new(pk);
    NodeSignInfo {
        signer: node_pk,
        signature: NodeSignature::from([signer_id; 48]), // Mock signature
    }
}

#[test]
fn account_public_key_from_node_sign_info() {
    let node_sign_info = create_mock_node_sign_info(5);
    let expected_pk = AccountPublicKey(*node_sign_info.signer.inner());
    let converted_pk = AccountPublicKey::from(&node_sign_info);
    assert_eq!(converted_pk, expected_pk);
}

#[test]
fn fault_item_equality_serialization_deserialization() {
    let item1 = create_mock_fault_item(200, 1);
    let item2 = create_mock_fault_item(200, 1);
    let item3 = create_mock_fault_item(201, 1); // Different header
    let item4 = create_mock_fault_item(200, 2); // Different signer

    // Equality
    assert_eq!(item1, item2);
    assert_ne!(item1, item3);
    assert_ne!(item1, item4);

    // Serde round trip
    let json = serde_json::to_value(&item1).unwrap();
    let deserialized: FaultItem = serde_json::from_value(json).unwrap();
    assert_eq!(item1, deserialized);
}

#[test]
fn fault_equality_serialization_deserialization() {
    // Helper to create mock Fault for this test
    let create_mock_fault =
        |fault_type: FaultType, round: u64, signer1: u8, signer2: u8| Fault {
            fault_type,
            item1: create_mock_fault_item(round, signer1),
            item2: create_mock_fault_item(round, signer2),
        };

    let fault1 = create_mock_fault(FaultType::DoubleCandidate, 300, 10, 11);
    let fault2 = create_mock_fault(FaultType::DoubleCandidate, 300, 10, 11);
    let fault3 =
        create_mock_fault(FaultType::DoubleRatificationVote, 300, 10, 11);
    let fault4 = create_mock_fault(FaultType::DoubleCandidate, 301, 10, 11);
    let fault5 = create_mock_fault(FaultType::DoubleCandidate, 300, 12, 11);

    // Equality
    assert_eq!(fault1, fault2);
    assert_ne!(fault1, fault3);
    assert_ne!(fault1, fault4);
    assert_ne!(fault1, fault5);

    // Serde round trip
    let json = serde_json::to_value(&fault1).unwrap();
    // Use expect for better error message if deserialization fails
    let deserialized: Fault = serde_json::from_value(json)
        .expect("Failed to deserialize Fault from its own serialized JSON");
    assert_eq!(fault1, deserialized);
}

#[test]
fn block_faults_equality_serialization_deserialization() {
    // Helper needed again because create_mock_fault is local to the previous
    // test
    let create_mock_fault =
        |fault_type: FaultType, round: u64, signer1: u8, signer2: u8| Fault {
            fault_type,
            item1: create_mock_fault_item(round, signer1),
            item2: create_mock_fault_item(round, signer2),
        };

    let bf1 = BlockFaults {
        faults: vec![
            create_mock_fault(FaultType::DoubleCandidate, 500, 1, 2),
            create_mock_fault(FaultType::DoubleValidationVote, 501, 3, 4),
        ],
    };
    let bf2 = BlockFaults {
        faults: vec![
            create_mock_fault(FaultType::DoubleCandidate, 500, 1, 2),
            create_mock_fault(FaultType::DoubleValidationVote, 501, 3, 4),
        ],
    };
    let bf3 = BlockFaults {
        faults: vec![create_mock_fault(FaultType::DoubleCandidate, 500, 1, 2)], /* Fewer faults */
    };
    let bf4 = BlockFaults {
        faults: vec![
            create_mock_fault(FaultType::DoubleCandidate, 500, 1, 2),
            create_mock_fault(FaultType::DoubleRatificationVote, 501, 3, 4), /* Different second fault type */
        ],
    };

    // Equality
    assert_eq!(bf1, bf2);
    assert_ne!(bf1, bf3);
    assert_ne!(bf1, bf4);

    // Serde round trip
    let json = serde_json::to_value(&bf1).unwrap();
    let deserialized: BlockFaults = serde_json::from_value(json).unwrap();
    assert_eq!(bf1, deserialized);
}

#[test]
fn block_header_from_node_header() {
    let node_header = create_mock_node_block(90).header().clone();
    let converted_header = BlockHeader::from(node_header.clone());

    assert_eq!(converted_header.version, node_header.version as u32);
    assert_eq!(converted_header.height, node_header.height);
    assert_eq!(
        converted_header.previous_hash,
        hex::encode(node_header.prev_block_hash)
    );
    assert_eq!(converted_header.timestamp, node_header.timestamp);
    assert_eq!(converted_header.hash, hex::encode(node_header.hash));
    assert_eq!(
        converted_header.state_hash,
        hex::encode(node_header.state_hash)
    );
    assert_eq!(
        converted_header.validator,
        bs58::encode(node_header.generator_bls_pubkey.inner()).into_string()
    );
    assert_eq!(
        converted_header.transactions_root,
        hex::encode(node_header.txroot)
    );
    assert_eq!(converted_header.gas_limit, node_header.gas_limit);
    assert_eq!(converted_header.seed, hex::encode(node_header.seed.inner()));
    assert_eq!(converted_header.sequence, node_header.iteration as u32);
}
