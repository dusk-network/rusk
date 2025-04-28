// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for archive-related JSON-RPC models.

use base64::Engine;
use rusk::jsonrpc::model::archive::MoonlightEventGroup;
use rusk::jsonrpc::model::archive::{ArchivedEvent, ContractEvent};

use crate::jsonrpc::utils::create_mock_moonlight_group;

#[test]
fn test_archived_event_serde() {
    let event_data = vec![10, 20, 30, 40, 50];
    let expected_b64_data =
        base64::engine::general_purpose::STANDARD.encode(&event_data);

    let original = ArchivedEvent {
        origin: "0xabcdef123456".to_string(),
        topic: "test_topic".to_string(),
        source: "contract_123".to_string(),
        data: event_data,
    };

    let expected_json = format!(
        r#"{{"origin":"{}","topic":"{}","source":"{}","data":"{}"}}"#,
        original.origin, original.topic, original.source, expected_b64_data
    );

    // Test serialization
    let json = serde_json::to_string(&original).expect("Serialization failed");
    assert_eq!(json, expected_json);

    // Test deserialization
    let deserialized: ArchivedEvent =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(original, deserialized);
}

#[test]
fn test_archived_event_serde_empty_data() {
    let event_data: Vec<u8> = vec![];
    let expected_b64_data =
        base64::engine::general_purpose::STANDARD.encode(&event_data);
    assert_eq!(expected_b64_data, ""); // Ensure empty data encodes to empty string

    let original = ArchivedEvent {
        origin: "0xabcdef123456".to_string(),
        topic: "test_topic".to_string(),
        source: "contract_123".to_string(),
        data: event_data,
    };

    let expected_json = format!(
        r#"{{"origin":"{}","topic":"{}","source":"{}","data":"{}"}}"#,
        original.origin, original.topic, original.source, expected_b64_data
    );

    // Test serialization
    let json = serde_json::to_string(&original).expect("Serialization failed");
    assert_eq!(json, expected_json);

    // Test deserialization
    let deserialized: ArchivedEvent =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(original, deserialized);
}

#[test]
fn test_contract_event_serde() {
    let original = ContractEvent {
        target: "contract_abc".to_string(),
        topic: "event_xyz".to_string(),
        data: hex::encode(b"some_data"), // Assuming data is hex encoded
    };

    let expected_json = format!(
        r#"{{"target":"{}","topic":"{}","data":"{}"}}"#,
        original.target, original.topic, original.data
    );

    // Test serialization
    let json = serde_json::to_string(&original).expect("Serialization failed");
    assert_eq!(json, expected_json);

    // Test deserialization
    let deserialized: ContractEvent =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(original, deserialized);
}

#[test]
fn test_moonlight_event_group_serde() {
    let event1 = ContractEvent {
        target: "contract_1".to_string(),
        topic: "topic_a".to_string(),
        data: hex::encode(b"data1"),
    };
    let event2 = ContractEvent {
        target: "contract_2".to_string(),
        topic: "topic_b".to_string(),
        data: hex::encode(b"data2"),
    };

    let original = MoonlightEventGroup {
        events: vec![event1.clone(), event2.clone()],
        origin:
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .to_string(),
        block_height: 12345,
    };

    // Construct expected JSON manually (including stringified block_height)
    let expected_json = format!(
        r#"{{"events":[{{"target":"{}","topic":"{}","data":"{}"}},{{"target":"{}","topic":"{}","data":"{}"}}],"origin":"{}","block_height":"{}"}}"#,
        event1.target,
        event1.topic,
        event1.data,
        event2.target,
        event2.topic,
        event2.data,
        original.origin,
        original.block_height
    );

    // Test serialization
    let json = serde_json::to_string(&original).expect("Serialization failed");
    assert_eq!(json, expected_json);

    // Test deserialization
    let deserialized: MoonlightEventGroup =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(original, deserialized);
}

#[test]
fn test_moonlight_event_group_serde_empty_events() {
    let original = MoonlightEventGroup {
        events: vec![],
        origin:
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .to_string(),
        block_height: 98765,
    };

    let expected_json = format!(
        r#"{{"events":[],"origin":"{}","block_height":"{}"}}"#,
        original.origin, original.block_height
    );

    // Test serialization
    let json = serde_json::to_string(&original).expect("Serialization failed");
    assert_eq!(json, expected_json);

    // Test deserialization
    let deserialized: MoonlightEventGroup =
        serde_json::from_str(&json).expect("Deserialization failed");
    assert_eq!(original, deserialized);
}

#[test]
fn contract_event_equality() {
    let event1 = ContractEvent {
        target: "target1".to_string(),
        topic: "topic1".to_string(),
        data: "data1".to_string(),
    };
    let event2 = ContractEvent {
        target: "target1".to_string(),
        topic: "topic1".to_string(),
        data: "data1".to_string(),
    };
    let event3 = ContractEvent {
        target: "target2".to_string(),
        topic: "topic1".to_string(),
        data: "data1".to_string(),
    };
    assert_eq!(event1, event2);
    assert_ne!(event1, event3);
}

#[test]
fn moonlight_event_group_equality() {
    let group1 = create_mock_moonlight_group("tx1", 100);
    let group2 = create_mock_moonlight_group("tx1", 100);
    let group3 = create_mock_moonlight_group("tx2", 100);
    let mut group4 = create_mock_moonlight_group("tx1", 100);
    group4.events.push(ContractEvent {
        target: "t".into(),
        topic: "t".into(),
        data: "d".into(),
    }); // Add an event

    assert_eq!(group1, group2);
    assert_ne!(group1, group3);
    assert_ne!(group1, group4);
}
