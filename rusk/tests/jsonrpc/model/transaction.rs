// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for JSON-RPC transaction models.

use crate::jsonrpc::utils::create_mock_ml_tx_response;
use rusk::jsonrpc::model::transaction::{
    BaseTransaction, MoonlightTransactionData, PhoenixTransactionData,
    TransactionDataType, TransactionResponse, TransactionStatus,
    TransactionStatusType, TransactionType,
};
use serde_json;

#[test]
fn transaction_type_equality() {
    assert_eq!(TransactionType::Phoenix, TransactionType::Phoenix);
    assert_eq!(TransactionType::Moonlight, TransactionType::Moonlight);
    assert_ne!(TransactionType::Phoenix, TransactionType::Moonlight);
}

#[test]
fn transaction_status_type_equality() {
    assert_eq!(
        TransactionStatusType::Pending,
        TransactionStatusType::Pending
    );
    assert_eq!(
        TransactionStatusType::Executed,
        TransactionStatusType::Executed
    );
    assert_eq!(TransactionStatusType::Failed, TransactionStatusType::Failed);
    assert_ne!(
        TransactionStatusType::Pending,
        TransactionStatusType::Executed
    );
}

#[test]
fn base_transaction_equality() {
    let base1 = BaseTransaction {
        tx_hash: "h1".into(),
        version: 1,
        tx_type: TransactionType::Phoenix,
        gas_price: 100,
        gas_limit: 2000,
        raw: "raw1".into(),
    };
    let base2 = BaseTransaction {
        tx_hash: "h1".into(),
        version: 1,
        tx_type: TransactionType::Phoenix,
        gas_price: 100,
        gas_limit: 2000,
        raw: "raw1".into(),
    };
    let base3 = BaseTransaction {
        tx_hash: "h2".into(),
        version: 1,
        tx_type: TransactionType::Phoenix,
        gas_price: 100,
        gas_limit: 2000,
        raw: "raw1".into(),
    };
    assert_eq!(base1, base2);
    assert_ne!(base1, base3);
}

#[test]
fn transaction_status_equality() {
    let status1 = TransactionStatus {
        status: TransactionStatusType::Executed,
        block_height: Some(100),
        block_hash: Some("bh1".into()),
        gas_spent: Some(1500),
        timestamp: Some(12345),
        error: None,
    };
    let status2 = TransactionStatus {
        status: TransactionStatusType::Executed,
        block_height: Some(100),
        block_hash: Some("bh1".into()),
        gas_spent: Some(1500),
        timestamp: Some(12345),
        error: None,
    };
    let status3 = TransactionStatus {
        status: TransactionStatusType::Failed,
        block_height: None,
        block_hash: None,
        gas_spent: None,
        timestamp: None,
        error: Some("bad stuff".into()),
    };
    assert_eq!(status1, status2);
    assert_ne!(status1, status3);
}

#[test]
fn transaction_type_serialization() {
    assert_eq!(
        serde_json::to_string(&TransactionType::Phoenix).unwrap(),
        "\"Phoenix\""
    );
    assert_eq!(
        serde_json::to_string(&TransactionType::Moonlight).unwrap(),
        "\"Moonlight\""
    );
}

#[test]
fn transaction_status_type_serialization() {
    assert_eq!(
        serde_json::to_string(&TransactionStatusType::Pending).unwrap(),
        "\"Pending\""
    );
    assert_eq!(
        serde_json::to_string(&TransactionStatusType::Executed).unwrap(),
        "\"Executed\""
    );
    assert_eq!(
        serde_json::to_string(&TransactionStatusType::Failed).unwrap(),
        "\"Failed\""
    );
}

#[test]
fn transaction_response_equality() {
    let tx1 = create_mock_ml_tx_response("tx1");
    let tx2 = create_mock_ml_tx_response("tx1"); // Identical
    let tx3 = create_mock_ml_tx_response("tx2"); // Different base.tx_hash

    let mut tx4 = create_mock_ml_tx_response("tx1");
    tx4.status = None; // Different status

    let mut tx5 = create_mock_ml_tx_response("tx1");
    if let TransactionDataType::Moonlight(ref mut data) = tx5.transaction_data {
        data.value = 999; // Different transaction_data.value
    }

    assert_eq!(tx1, tx2); // Should be equal
    assert_ne!(tx1, tx3); // Different hash
    assert_ne!(tx1, tx4); // Different status option
    assert_ne!(tx1, tx5); // Different transaction data
}

#[test]
fn base_transaction_serialization() {
    let base = BaseTransaction {
        tx_hash: "h_ser".into(),
        version: 2,
        tx_type: TransactionType::Moonlight,
        gas_price: 123456789012345,
        gas_limit: 98765432109876,
        raw: "raw_ser".into(),
    };
    let json = serde_json::to_value(base).unwrap();
    assert_eq!(json["tx_hash"], "h_ser");
    assert_eq!(json["version"], 2);
    assert_eq!(json["tx_type"], "Moonlight");
    assert_eq!(json["gas_price"], "123456789012345"); // String
    assert_eq!(json["gas_limit"], "98765432109876"); // String
    assert_eq!(json["raw"], "raw_ser");
}

#[test]
fn transaction_status_serialization() {
    let status_exec = TransactionStatus {
        status: TransactionStatusType::Executed,
        block_height: Some(1001),
        block_hash: Some("bh_ser".into()),
        gas_spent: Some(12345),
        timestamp: Some(98765),
        error: None,
    };
    let json_exec = serde_json::to_value(status_exec).unwrap();
    assert_eq!(json_exec["status"], "Executed");
    assert_eq!(json_exec["block_height"], "1001"); // String
    assert_eq!(json_exec["block_hash"], "bh_ser");
    assert_eq!(json_exec["gas_spent"], "12345"); // String
    assert_eq!(json_exec["timestamp"], "98765"); // String
    assert!(json_exec.get("error").is_none()); // Skipped if None

    let status_fail = TransactionStatus {
        status: TransactionStatusType::Failed,
        block_height: None,
        block_hash: None,
        gas_spent: None,
        timestamp: None,
        error: Some("Error message".into()),
    };
    let json_fail = serde_json::to_value(status_fail).unwrap();
    assert_eq!(json_fail["status"], "Failed");
    assert!(json_fail.get("block_height").is_none()); // Skipped if None
    assert!(json_fail.get("block_hash").is_none());
    assert!(json_fail.get("gas_spent").is_none());
    assert!(json_fail.get("timestamp").is_none());
    assert_eq!(json_fail["error"], "Error message");
}

#[test]
fn transaction_data_serialization() {
    let ml_data = TransactionDataType::Moonlight(MoonlightTransactionData {
        sender: "sender_ser".into(),
        receiver: Some("receiver_ser".into()),
        value: 11111,
        nonce: 22222,
        memo: Some("memo_ser".into()),
    });
    let json_ml = serde_json::to_value(ml_data).unwrap();
    assert_eq!(json_ml["sender"], "sender_ser");
    assert_eq!(json_ml["receiver"], "receiver_ser");
    assert_eq!(json_ml["value"], "11111"); // String
    assert_eq!(json_ml["nonce"], "22222"); // String
    assert_eq!(json_ml["memo"], "memo_ser");

    let ph_data = TransactionDataType::Phoenix(PhoenixTransactionData {
        nullifiers: vec!["n1".into(), "n2".into()],
        outputs: vec!["o1".into()],
        proof: "proof_ser".into(),
    });
    let json_ph = serde_json::to_value(ph_data).unwrap();
    assert!(json_ph["nullifiers"].is_array());
    assert_eq!(json_ph["nullifiers"][0], "n1");
    assert!(json_ph["outputs"].is_array());
    assert_eq!(json_ph["outputs"][0], "o1");
    assert_eq!(json_ph["proof"], "proof_ser");
}

#[test]
fn transaction_response_serialization() {
    let tx = create_mock_ml_tx_response("tx_full_ser");
    let json = serde_json::to_value(tx).unwrap();

    // Check flattened base fields
    assert_eq!(json["tx_hash"], "tx_full_ser");
    assert_eq!(json["tx_type"], "Moonlight");
    assert_eq!(json["gas_price"], "10"); // String

    // Check flattened status fields (if Some)
    assert_eq!(json["status"], "Executed");
    assert_eq!(json["block_height"], "101"); // String
    assert!(json.get("error").is_none());

    // Check transaction_data
    assert!(json["transaction_data"].is_object());
    assert_eq!(json["transaction_data"]["sender"], "sender");
    assert_eq!(json["transaction_data"]["value"], "1000"); // String
}

#[test]
fn transaction_response_serialization_no_status() {
    let mut tx = create_mock_ml_tx_response("tx_no_status");
    tx.status = None;
    let json = serde_json::to_value(&tx).unwrap();

    // Base fields still present
    assert_eq!(json["tx_hash"], "tx_no_status");

    // Status fields should be absent (skipped)
    assert!(json.get("status").is_none());
    assert!(json.get("block_height").is_none());
    assert!(json.get("gas_spent").is_none());
}

#[test]
fn transaction_response_deserialization() {
    // Create an expected response
    let expected_tx = create_mock_ml_tx_response("tx_deser");

    // Serialize it to JSON
    let json_str = serde_json::to_string(&expected_tx).unwrap();

    // Deserialize it back
    let deserialized_tx: TransactionResponse =
        serde_json::from_str(&json_str).unwrap();

    // Compare
    assert_eq!(deserialized_tx, expected_tx);
}

#[test]
fn transaction_response_deserialization_no_status() {
    // Create a response with status explicitly set to None
    let mut expected_tx = create_mock_ml_tx_response("tx_deser_no_status");
    expected_tx.status = None;

    // Serialize it (status fields will be omitted by flatten +
    // skip_serializing_if)
    let json_str = serde_json::to_string(&expected_tx).unwrap();

    // Deserialize it back
    let deserialized_tx: TransactionResponse =
        serde_json::from_str(&json_str).unwrap();

    // Compare
    assert_eq!(deserialized_tx, expected_tx);
    assert!(deserialized_tx.status.is_none());
}

#[test]
fn transaction_data_deserialization_moonlight() {
    let json_str = r#"
    {
        "sender": "s_deser",
        "receiver": "r_deser",
        "value": "12345",
        "nonce": "67",
        "memo": "memo_deser"
    }
    "#;
    let expected_data = MoonlightTransactionData {
        sender: "s_deser".into(),
        receiver: Some("r_deser".into()),
        value: 12345,
        nonce: 67,
        memo: Some("memo_deser".into()),
    };
    let deserialized_data: TransactionDataType =
        serde_json::from_str(json_str).unwrap();

    match deserialized_data {
        TransactionDataType::Moonlight(data) => assert_eq!(data, expected_data),
        _ => panic!("Expected Moonlight data type"),
    }
}

#[test]
fn transaction_data_deserialization_phoenix() {
    let json_str = r#"
    {
        "nullifiers": ["n1_d", "n2_d"],
        "outputs": ["o1_d"],
        "proof": "p_deser"
    }
    "#;
    let expected_data = PhoenixTransactionData {
        nullifiers: vec!["n1_d".into(), "n2_d".into()],
        outputs: vec!["o1_d".into()],
        proof: "p_deser".into(),
    };
    let deserialized_data: TransactionDataType =
        serde_json::from_str(json_str).unwrap();

    match deserialized_data {
        TransactionDataType::Phoenix(data) => assert_eq!(data, expected_data),
        _ => panic!("Expected Phoenix data type"),
    }
}

#[test]
fn transaction_status_deserialization() {
    let json_str = r#"
    {
        "status": "Executed",
        "block_height": "1001",
        "block_hash": "bh_deser",
        "gas_spent": "12345",
        "timestamp": "98765"
    }
    "#;
    // Error field is missing -> deserializes to None
    let expected_status = TransactionStatus {
        status: TransactionStatusType::Executed,
        block_height: Some(1001),
        block_hash: Some("bh_deser".into()),
        gas_spent: Some(12345),
        timestamp: Some(98765),
        error: None,
    };
    let deserialized_status: TransactionStatus =
        serde_json::from_str(json_str).unwrap();
    assert_eq!(deserialized_status, expected_status);
}

#[test]
fn base_transaction_deserialization() {
    let json_str = r#"
    {
        "tx_hash": "h_deser",
        "version": 2,
        "tx_type": "Phoenix",
        "gas_price": "123456",
        "gas_limit": "987654",
        "raw": "raw_deser"
    }
    "#;
    let expected_base = BaseTransaction {
        tx_hash: "h_deser".into(),
        version: 2,
        tx_type: TransactionType::Phoenix,
        gas_price: 123456,
        gas_limit: 987654,
        raw: "raw_deser".into(),
    };
    let deserialized_base: BaseTransaction =
        serde_json::from_str(json_str).unwrap();
    assert_eq!(deserialized_base, expected_base);
}

#[test]
fn transaction_status_type_deserialization() {
    assert_eq!(
        serde_json::from_str::<TransactionStatusType>("\"Pending\"").unwrap(),
        TransactionStatusType::Pending
    );
    assert_eq!(
        serde_json::from_str::<TransactionStatusType>("\"Executed\"").unwrap(),
        TransactionStatusType::Executed
    );
    assert_eq!(
        serde_json::from_str::<TransactionStatusType>("\"Failed\"").unwrap(),
        TransactionStatusType::Failed
    );
    assert!(
        serde_json::from_str::<TransactionStatusType>("\"Unknown\"").is_err()
    );
}

#[test]
fn transaction_type_deserialization() {
    assert_eq!(
        serde_json::from_str::<TransactionType>("\"Phoenix\"").unwrap(),
        TransactionType::Phoenix
    );
    assert_eq!(
        serde_json::from_str::<TransactionType>("\"Moonlight\"").unwrap(),
        TransactionType::Moonlight
    );
    assert!(serde_json::from_str::<TransactionType>("\"Invalid\"").is_err());
}
