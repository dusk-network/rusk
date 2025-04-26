// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::infrastructure::subscription::types::*;
use serde_json;

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
