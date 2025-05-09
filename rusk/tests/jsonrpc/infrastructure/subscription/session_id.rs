// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
use rusk::jsonrpc::infrastructure::subscription::types::*;
use serde_json;
use std::convert::TryFrom;
use std::str::FromStr;

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
    assert!(debug_str.ends_with(')'));
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
