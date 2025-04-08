// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the WebSocket subscription infrastructure.

use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
use rusk::jsonrpc::infrastructure::subscription::types::Topic;
use std::str::FromStr;

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

use rusk::jsonrpc::infrastructure::subscription::types::SubscriptionId;
use uuid::Uuid;

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
