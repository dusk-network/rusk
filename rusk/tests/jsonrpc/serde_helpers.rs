// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the serde helpers.

use base64::Engine as _;
use rusk::jsonrpc::model::serde_helper;
use serde::{Deserialize, Serialize};

// Test struct for u64_to_string
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct TestU64 {
    #[serde(with = "serde_helper::u64_to_string")]
    value: u64,
}

#[test]
fn test_u64_to_string_serde() {
    let original = TestU64 {
        value: 12345678901234567890,
    };
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, r#"{"value":"12345678901234567890"}"#);

    let deserialized: TestU64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_u64_to_string_serde_zero() {
    let original = TestU64 { value: 0 };
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, r#"{"value":"0"}"#);

    let deserialized: TestU64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_u64_to_string_serde_max() {
    let original = TestU64 { value: u64::MAX };
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, format!(r#"{{"value":"{}"}}"#, u64::MAX));

    let deserialized: TestU64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

// Test struct for opt_u64_to_string
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct TestOptU64 {
    #[serde(
        with = "serde_helper::opt_u64_to_string",
        skip_serializing_if = "Option::is_none"
    )]
    value: Option<u64>,
}

#[test]
fn test_opt_u64_to_string_serde_some() {
    let original = TestOptU64 {
        value: Some(9876543210987654321),
    };
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, r#"{"value":"9876543210987654321"}"#);

    let deserialized: TestOptU64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_opt_u64_to_string_serde_none() {
    let original = TestOptU64 { value: None };
    let json = serde_json::to_string(&original).unwrap();
    // Expecting empty object due to skip_serializing_if = "Option::is_none"
    assert_eq!(json, r#"{}"#);

    // Deserialization should work if the field is explicitly null
    let deserialized_null: TestOptU64 =
        serde_json::from_str(r#"{"value":null}"#).unwrap();
    assert_eq!(original, deserialized_null);

    // NOTE: Deserializing from an empty object {} is expected to fail here
    // because the `value` field is missing, and we haven't added
    // #[serde(default)] let deserialized_missing: TestOptU64 =
    // serde_json::from_str(r#"{}"#).unwrap(); // This would panic
    // assert_eq!(original, deserialized_missing);
}

// Test struct for base64_vec_u8
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct TestBase64 {
    #[serde(with = "serde_helper::base64_vec_u8")]
    data: Vec<u8>,
}

#[test]
fn test_base64_vec_u8_serde() {
    let original = TestBase64 {
        data: vec![0, 1, 2, 10, 100, 255],
    };
    let json = serde_json::to_string(&original).unwrap();
    // Calculate the expected base64 string dynamically
    let expected_b64 =
        base64::engine::general_purpose::STANDARD.encode(&original.data);
    assert_eq!(json, format!(r#"{{"data":"{}"}}"#, expected_b64));

    let deserialized: TestBase64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_base64_vec_u8_serde_empty() {
    let original = TestBase64 { data: vec![] };
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(json, r#"{"data":""}"#);

    let deserialized: TestBase64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn test_base64_vec_u8_serde_longer() {
    let data = b"This is a longer test string for base64 encoding!".to_vec();
    let original = TestBase64 { data: data.clone() };
    let json = serde_json::to_string(&original).unwrap();
    // Pre-calculate expected base64 string
    let expected_b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    assert_eq!(json, format!(r#"{{"data":"{}"}}"#, expected_b64));

    let deserialized: TestBase64 = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}
