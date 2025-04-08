// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the WebSocket subscription infrastructure.

use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
use rusk::jsonrpc::infrastructure::subscription::types::*;
use serde_json;
use std::convert::TryFrom;
use std::str::FromStr;
use uuid::Uuid;

// --- Topic Tests ---

#[test]
fn topic_serialization_deserialization() {
    let topics = vec![
        Topic::BlockAcceptance,
        Topic::BlockFinalization,
        Topic::ChainReorganization,
        Topic::ContractEvents,
        Topic::ContractTransferEvents,
        Topic::MempoolAcceptance,
        Topic::MempoolEvents,
    ];

    for topic in topics {
        let serialized = serde_json::to_string(&topic).unwrap();
        let expected_str = format!("\"{}\"", topic.as_str());
        assert_eq!(serialized, expected_str);

        let deserialized: Topic = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, topic);
    }
}

#[test]
fn topic_display() {
    assert_eq!(Topic::BlockAcceptance.to_string(), "BlockAcceptance");
    assert_eq!(Topic::BlockFinalization.to_string(), "BlockFinalization");
    assert_eq!(
        Topic::ChainReorganization.to_string(),
        "ChainReorganization"
    );
    assert_eq!(Topic::ContractEvents.to_string(), "ContractEvents");
    assert_eq!(
        Topic::ContractTransferEvents.to_string(),
        "ContractTransferEvents"
    );
    assert_eq!(Topic::MempoolAcceptance.to_string(), "MempoolAcceptance");
    assert_eq!(Topic::MempoolEvents.to_string(), "MempoolEvents");
}

#[test]
fn topic_debug() {
    assert_eq!(format!("{:?}", Topic::BlockAcceptance), "BlockAcceptance");
    // Add other variants if needed, Debug usually matches Display for simple
    // enums
}

#[test]
fn topic_as_str() {
    assert_eq!(Topic::BlockAcceptance.as_str(), "BlockAcceptance");
    assert_eq!(Topic::BlockFinalization.as_str(), "BlockFinalization");
    assert_eq!(Topic::ChainReorganization.as_str(), "ChainReorganization");
    assert_eq!(Topic::ContractEvents.as_str(), "ContractEvents");
    assert_eq!(
        Topic::ContractTransferEvents.as_str(),
        "ContractTransferEvents"
    );
    assert_eq!(Topic::MempoolAcceptance.as_str(), "MempoolAcceptance");
    assert_eq!(Topic::MempoolEvents.as_str(), "MempoolEvents");
}

#[test]
fn topic_from_str_valid() {
    assert_eq!(
        Topic::from_str("BlockAcceptance").unwrap(),
        Topic::BlockAcceptance
    );
    assert_eq!(
        Topic::from_str("BlockFinalization").unwrap(),
        Topic::BlockFinalization
    );
    assert_eq!(
        Topic::from_str("ChainReorganization").unwrap(),
        Topic::ChainReorganization
    );
    assert_eq!(
        Topic::from_str("ContractEvents").unwrap(),
        Topic::ContractEvents
    );
    assert_eq!(
        Topic::from_str("ContractTransferEvents").unwrap(),
        Topic::ContractTransferEvents
    );
    assert_eq!(
        Topic::from_str("MempoolAcceptance").unwrap(),
        Topic::MempoolAcceptance
    );
    assert_eq!(
        Topic::from_str("MempoolEvents").unwrap(),
        Topic::MempoolEvents
    );
}

#[test]
fn topic_from_str_invalid() {
    let invalid_topic = "InvalidTopicName";
    let result = Topic::from_str(invalid_topic);
    assert!(result.is_err());
    match result.err().unwrap() {
        SubscriptionError::InvalidTopic(topic) => {
            assert_eq!(topic, invalid_topic)
        }
        _ => panic!("Expected InvalidTopic error"),
    }
}

// --- SubscriptionId Tests ---

#[test]
fn subscription_id_new_unique() {
    let id1 = SubscriptionId::new();
    let id2 = SubscriptionId::new();
    assert_ne!(id1, id2);
}

#[test]
fn subscription_id_default() {
    let id1 = SubscriptionId::default();
    let id2 = SubscriptionId::default();
    assert_ne!(id1, id2); // Default should also generate a new UUID
}

#[test]
fn subscription_id_inner() {
    let id = SubscriptionId::new();
    let inner_uuid = id.inner();
    assert_eq!(id.to_string(), inner_uuid.to_string());
}

#[test]
fn subscription_id_display() {
    let uuid_val = Uuid::new_v4();
    let sub_id_str = uuid_val.to_string();
    let sub_id = SubscriptionId::from_str(&sub_id_str).unwrap();
    assert_eq!(sub_id.to_string(), sub_id_str);
}

#[test]
fn subscription_id_from_str_valid() {
    let uuid_str = "a1b2c3d4-e5f6-7890-1234-567890abcdef";
    let sub_id = SubscriptionId::from_str(uuid_str).unwrap();
    assert_eq!(sub_id.to_string(), uuid_str);
}

#[test]
fn subscription_id_from_str_invalid() {
    let invalid_uuid_str = "not-a-valid-uuid";
    let result = SubscriptionId::from_str(invalid_uuid_str);
    assert!(result.is_err());
    match result.err().unwrap() {
        SubscriptionError::InvalidSubscriptionIdFormat(_) => {
            // Correct error type, no need to check the exact message
        }
        _ => panic!("Expected InvalidSubscriptionIdFormat error"),
    }
}

#[test]
fn subscription_id_serde() {
    let sub_id = SubscriptionId::new();
    let serialized = serde_json::to_string(&sub_id).unwrap();
    // Expected format is just the UUID string because of #[serde(transparent)]
    let expected_json = format!("\"{}\"", sub_id.to_string());
    assert_eq!(serialized, expected_json);

    let deserialized: SubscriptionId =
        serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, sub_id);
}

#[test]
fn subscription_id_serde_invalid() {
    let invalid_json = "\"invalid-uuid-string\"";
    let result: Result<SubscriptionId, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}

#[test]
fn subscription_id_serde_invalid_format() {
    // Test deserialization failure from non-string JSON types
    let invalid_json_number = "123";
    let result_num: Result<SubscriptionId, _> =
        serde_json::from_str(invalid_json_number);
    assert!(result_num.is_err());

    let invalid_json_object = "{}";
    let result_obj: Result<SubscriptionId, _> =
        serde_json::from_str(invalid_json_object);
    assert!(result_obj.is_err());
}

#[test]
fn subscription_id_debug() {
    // Check that Debug format includes the struct name and the UUID string
    let sub_id = SubscriptionId::new();
    let debug_str = format!("{:?}", sub_id);
    assert!(debug_str.starts_with("SubscriptionId("));
    assert!(debug_str.ends_with(")"));
    assert!(debug_str.contains(&sub_id.to_string()));
}

#[test]
fn subscription_id_hash() {
    use std::collections::HashSet;

    let id1 = SubscriptionId::new();
    let id2 = SubscriptionId::new();
    let id1_clone = id1; // Test copy/clone works with hash

    let mut set = HashSet::new();
    assert!(set.insert(id1));
    assert!(set.insert(id2));
    assert!(!set.insert(id1_clone)); // Should not insert the same ID again

    assert_eq!(set.len(), 2);
    assert!(set.contains(&id1));
    assert!(set.contains(&id2));
}

#[test]
fn subscription_id_serde_invalid_empty() {
    // Explicitly test that deserializing an empty string fails
    let empty_json = "\"\"";
    let result: Result<SubscriptionId, _> = serde_json::from_str(empty_json);
    assert!(
        result.is_err(),
        "Deserializing an empty string into SubscriptionId should fail"
    );
}

// --- SessionId Tests ---

#[test]
fn session_id_creation_valid() {
    let id_str = "valid-session-id";
    let session_id = SessionId::from_str(id_str).unwrap();
    assert_eq!(session_id.inner(), id_str);
    assert_eq!(session_id.to_string(), id_str);

    // Test with TryFrom<String>
    let session_id_try = SessionId::try_from(id_str.to_string()).unwrap();
    assert_eq!(session_id_try, session_id);
}

#[test]
fn session_id_creation_empty() {
    // Test TryFrom<String> directly
    let result_try = SessionId::try_from("".to_string());
    assert!(result_try.is_err());
    match result_try.err().unwrap() {
        SubscriptionError::InvalidSessionIdFormat(msg) => {
            assert_eq!(msg, "Session ID cannot be empty");
        }
        _ => panic!("Expected InvalidSessionIdFormat error"),
    }

    // Test FromStr delegates correctly
    let result_from = SessionId::from_str("");
    assert!(result_from.is_err());
    match result_from.err().unwrap() {
        SubscriptionError::InvalidSessionIdFormat(msg) => {
            assert_eq!(msg, "Session ID cannot be empty");
        }
        _ => panic!("Expected InvalidSessionIdFormat error"),
    }
}

#[test]
fn session_id_display() {
    let id_str = "display-test";
    let session_id = SessionId::from_str(id_str).unwrap();
    assert_eq!(format!("{}", session_id), id_str);
}

#[test]
fn session_id_serde() {
    let id_str = "serde-session-id";
    let session_id = SessionId::from_str(id_str).unwrap();
    let serialized = serde_json::to_string(&session_id).unwrap();
    let expected_json = format!("\"{}\"", id_str);
    assert_eq!(serialized, expected_json);

    let deserialized: SessionId = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, session_id);
}

#[test]
fn session_id_serde_invalid_empty() {
    let empty_json = "\"\"";
    let result: Result<SessionId, _> = serde_json::from_str(empty_json);
    // Custom Deserialize impl using TryFrom catches this
    assert!(result.is_err());
}

#[test]
fn session_id_serde_invalid_format() {
    let invalid_json_number = "123";
    let result_num: Result<SessionId, _> =
        serde_json::from_str(invalid_json_number);
    assert!(result_num.is_err());

    let invalid_json_object = "{}";
    let result_obj: Result<SessionId, _> =
        serde_json::from_str(invalid_json_object);
    assert!(result_obj.is_err());
}

#[test]
fn session_id_debug() {
    let id_str = "debug-id";
    let session_id = SessionId::from_str(id_str).unwrap();
    let debug_str = format!("{:?}", session_id);
    assert!(debug_str.starts_with("SessionId("));
    assert!(debug_str.ends_with(")"));
    assert!(debug_str.contains(id_str));
}

#[test]
fn session_id_hash() {
    use std::collections::HashSet;

    let id1 = SessionId::from_str("hash-id-1").unwrap();
    let id2 = SessionId::from_str("hash-id-2").unwrap();
    let id1_clone = id1.clone();

    let mut set = HashSet::new();
    assert!(set.insert(id1.clone()));
    assert!(set.insert(id2));
    assert!(!set.insert(id1_clone)); // Should not insert the same ID again

    assert_eq!(set.len(), 2);
    assert!(set.contains(&id1));
}

// --- Parameter Types Tests ---

// Test BlockSubscriptionParams
#[test]
fn test_block_params_builder_defaults() {
    let params = BlockSubscriptionParams::builder().build();
    assert_eq!(params.include_txs(), None);
}

#[test]
fn test_block_params_builder_with_txs() {
    let params = BlockSubscriptionParams::builder().include_txs(true).build();
    assert_eq!(params.include_txs(), Some(true));
}

#[test]
fn test_block_params_builder_without_txs() {
    let params = BlockSubscriptionParams::builder()
        .include_txs(false)
        .build();
    assert_eq!(params.include_txs(), Some(false));
}

#[test]
fn test_block_params_serde() {
    // None
    let params_none = BlockSubscriptionParams::builder().build();
    let json_none = serde_json::to_string(&params_none).unwrap();
    assert_eq!(json_none, r#"{}"#);
    let de_none: BlockSubscriptionParams =
        serde_json::from_str(&json_none).unwrap();
    assert_eq!(params_none, de_none);
    let de_none_null: BlockSubscriptionParams =
        serde_json::from_str(r#"{"includeTxs":null}"#).unwrap();
    assert_eq!(params_none, de_none_null);

    // Some(true)
    let params_true =
        BlockSubscriptionParams::builder().include_txs(true).build();
    let json_true = serde_json::to_string(&params_true).unwrap();
    assert_eq!(json_true, r#"{"includeTxs":true}"#);
    let de_true: BlockSubscriptionParams =
        serde_json::from_str(&json_true).unwrap();
    assert_eq!(params_true, de_true);

    // Some(false)
    let params_false = BlockSubscriptionParams::builder()
        .include_txs(false)
        .build();
    let json_false = serde_json::to_string(&params_false).unwrap();
    assert_eq!(json_false, r#"{"includeTxs":false}"#);
    let de_false: BlockSubscriptionParams =
        serde_json::from_str(&json_false).unwrap();
    assert_eq!(params_false, de_false);
}

#[test]
fn test_block_params_debug() {
    let params = BlockSubscriptionParams::builder().include_txs(true).build();
    assert_eq!(
        format!("{:?}", params),
        "BlockSubscriptionParams { include_txs: Some(true) }"
    );
    let params_none = BlockSubscriptionParams::builder().build();
    assert_eq!(
        format!("{:?}", params_none),
        "BlockSubscriptionParams { include_txs: None }"
    );
}

// Test ContractSubscriptionParams (Type-State Builder)
#[test]
fn test_contract_params_builder_minimal() {
    let contract_id = "contract_123".to_string();
    let params = ContractSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .build();
    assert_eq!(params.contract_id(), contract_id);
    assert_eq!(params.event_names(), None);
    assert_eq!(params.include_metadata(), None);
    assert_eq!(params.min_amount(), None);
}

#[test]
fn test_contract_params_builder_full() {
    let contract_id = "contract_456".to_string();
    let event_names = vec!["EventA".to_string(), "EventB".to_string()];
    let min_amount = "100".to_string();

    let params = ContractSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .event_names(event_names.clone())
        .include_metadata(true)
        .min_amount(min_amount.clone())
        .build();

    assert_eq!(params.contract_id(), contract_id);
    assert_eq!(params.event_names(), Some(&event_names));
    assert_eq!(params.include_metadata(), Some(true));
    assert_eq!(params.min_amount(), Some(&min_amount));
}

#[test]
fn test_contract_params_serde() {
    // Minimal (only contract_id)
    let contract_id = "contract_min".to_string();
    let params_min = ContractSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .build();
    let json_min = serde_json::to_string(&params_min).unwrap();
    // Escape literal JSON braces {{ and }}
    let expected_json_min = format!(r#"{{"contractId":"{}"}}"#, contract_id);
    assert_eq!(json_min, expected_json_min);
    let de_min: ContractSubscriptionParams =
        serde_json::from_str(&json_min).unwrap();
    assert_eq!(params_min, de_min);

    // Full
    let event_names = vec!["Ev1".to_string()];
    let min_amount = "50".to_string();
    let params_full = ContractSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .event_names(event_names.clone())
        .include_metadata(false) // Explicit false
        .min_amount(min_amount.clone())
        .build();
    let json_full = serde_json::to_string(&params_full).unwrap();
    // Construct expected string separately, use camelCase, and escape literal
    // JSON braces {{ and }}
    let expected_json_full = format!(
        // Note the doubled {{ and }}
        r#"{{"contractId":"{}","eventNames":["Ev1"],"includeMetadata":false,"minAmount":"{}"}}"#,
        contract_id, min_amount
    );
    assert_eq!(json_full, expected_json_full);
    let de_full: ContractSubscriptionParams =
        serde_json::from_str(&json_full).unwrap();
    assert_eq!(params_full, de_full);
}

#[test]
fn test_contract_params_debug() {
    let params = ContractSubscriptionParams::builder()
        .contract_id("debug_contract".to_string())
        .event_names(vec!["DbgEvent".to_string()])
        .build();
    assert!(format!("{:?}", params).contains("contract_id: \"debug_contract\""));
    assert!(
        format!("{:?}", params).contains("event_names: Some([\"DbgEvent\"])")
    );
    assert!(format!("{:?}", params).contains("include_metadata: None"));
    assert!(format!("{:?}", params).contains("min_amount: None"));
}

// Test MempoolSubscriptionParams
#[test]
fn test_mempool_params_builder_defaults() {
    let params = MempoolSubscriptionParams::builder().build();
    assert_eq!(params.contract_id(), None);
    assert_eq!(params.include_details(), None);
}

#[test]
fn test_mempool_params_builder_contract() {
    let contract_id = "mempool_contract".to_string();
    let params = MempoolSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .build();
    assert_eq!(params.contract_id(), Some(&contract_id));
    assert_eq!(params.include_details(), None);
}

#[test]
fn test_mempool_params_builder_details() {
    let params = MempoolSubscriptionParams::builder()
        .include_details(true)
        .build();
    assert_eq!(params.contract_id(), None);
    assert_eq!(params.include_details(), Some(true));
}

#[test]
fn test_mempool_params_builder_both() {
    let contract_id = "mempool_contract_both".to_string();
    let params = MempoolSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .include_details(false)
        .build();
    assert_eq!(params.contract_id(), Some(&contract_id));
    assert_eq!(params.include_details(), Some(false));
}

#[test]
fn test_mempool_params_serde() {
    // Define contract_id at the start of the function for shared use
    let contract_id = "mempool_serde".to_string();

    // None
    let params_none = MempoolSubscriptionParams::builder().build();
    let json_none = serde_json::to_string(&params_none).unwrap();
    assert_eq!(json_none, r#"{}"#);
    let de_none: MempoolSubscriptionParams =
        serde_json::from_str(&json_none).unwrap();
    assert_eq!(params_none, de_none);

    // Contract ID only
    let params_contract = MempoolSubscriptionParams::builder()
        .contract_id(contract_id.clone())
        .build();
    let json_contract = serde_json::to_string(&params_contract).unwrap();
    // Construct expected string separately, use camelCase, and escape literal
    // JSON braces {{ and }}
    let expected_json_contract =
        format!(r#"{{"contractId":"{}"}}"#, contract_id);
    assert_eq!(json_contract, expected_json_contract);
    let de_contract: MempoolSubscriptionParams =
        serde_json::from_str(&json_contract).unwrap();
    assert_eq!(params_contract, de_contract);

    // Details only
    let params_details = MempoolSubscriptionParams::builder()
        .include_details(true)
        .build();
    let json_details = serde_json::to_string(&params_details).unwrap();
    // Use camelCase
    assert_eq!(json_details, r#"{"includeDetails":true}"#);
    let de_details: MempoolSubscriptionParams =
        serde_json::from_str(&json_details).unwrap();
    assert_eq!(params_details, de_details);

    // Both
    let params_both = MempoolSubscriptionParams::builder()
        .contract_id(contract_id.clone()) // Uses contract_id defined at function scope
        .include_details(false)
        .build();
    let json_both = serde_json::to_string(&params_both).unwrap();
    // Construct expected string separately, use camelCase, and escape literal
    // JSON braces {{ and }}
    let expected_json_both = format!(
        r#"{{"contractId":"{}","includeDetails":false}}"#,
        contract_id
    );
    assert_eq!(json_both, expected_json_both);
    let de_both: MempoolSubscriptionParams =
        serde_json::from_str(&json_both).unwrap();
    assert_eq!(params_both, de_both);
}

#[test]
fn test_mempool_params_debug() {
    let params = MempoolSubscriptionParams::builder()
        .contract_id("dbg_mempool".to_string())
        .build();
    assert!(
        format!("{:?}", params).contains("contract_id: Some(\"dbg_mempool\")")
    );
    assert!(format!("{:?}", params).contains("include_details: None"));

    let params_details = MempoolSubscriptionParams::builder()
        .include_details(false)
        .build();
    assert!(format!("{:?}", params_details).contains("contract_id: None"));
    assert!(format!("{:?}", params_details)
        .contains("include_details: Some(false)"));
}
