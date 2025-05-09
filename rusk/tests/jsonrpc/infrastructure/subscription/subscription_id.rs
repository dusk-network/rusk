// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
use rusk::jsonrpc::infrastructure::subscription::types::*;
use serde_json;
use std::str::FromStr;
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
    let expected_json = format!("\"{}\"", sub_id);
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
    assert!(debug_str.ends_with(')'));
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
