// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for archive-related JSON-RPC models.

use base64::Engine;
use rusk::jsonrpc::model::archive::ArchivedEvent;

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
