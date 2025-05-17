// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Debug;

use rusk::jsonrpc::infrastructure::subscription::filters::{
    BlockEventData, BlockFilter, ContractEventData, ContractFilter, Filter,
    MempoolEventData, MempoolFilter, TransferEventData, TransferFilter,
};

// --- BlockFilter Tests ---

// Helper struct for testing non-matching types
#[derive(Debug)]
struct NonBlockEvent;

#[test]
fn block_filter_builder_defaults() {
    let filter = BlockFilter::builder().build();
    assert!(!filter.include_txs(), "Default include_txs should be false");
}

#[test]
fn block_filter_builder_set_include_txs() {
    let filter_true = BlockFilter::builder().include_txs(true).build();
    assert!(filter_true.include_txs(), "include_txs should be true");

    let filter_false = BlockFilter::builder().include_txs(false).build();
    assert!(!filter_false.include_txs(), "include_txs should be false");
}

#[test]
fn block_filter_matches_correct_type() {
    let filter = BlockFilter::builder().build();
    let block_event = BlockEventData {
        height: 10,
        has_transactions: true,
    };
    assert!(
        filter.matches(&block_event),
        "Filter should match BlockEventData type"
    );
}

#[test]
fn block_filter_does_not_match_incorrect_type() {
    let filter = BlockFilter::builder().build();
    let non_block_event = NonBlockEvent;
    assert!(
        !filter.matches(&non_block_event),
        "Filter should not match NonBlockEvent type"
    );
}

#[test]
fn block_filter_matches_independent_of_include_txs_flag() {
    // Filter requesting txs
    let filter_with_txs = BlockFilter::builder().include_txs(true).build();
    // Filter not requesting txs
    let filter_without_txs = BlockFilter::builder().include_txs(false).build();

    // Event representing a block (irrespective of actual tx content)
    let block_event = BlockEventData {
        height: 20,
        has_transactions: false, // Example value
    };

    assert!(
        filter_with_txs.matches(&block_event),
        "Filter with include_txs=true should still match the event type"
    );
    assert!(
        filter_without_txs.matches(&block_event),
        "Filter with include_txs=false should still match the event type"
    );
}

#[test]
fn block_filter_implements_required_traits() {
    fn assert_traits<T: Debug + Send + Sync + Clone + 'static>() {}
    assert_traits::<BlockFilter>();
}

#[test]
fn block_filter_debug_output() {
    let filter_true = BlockFilter::builder().include_txs(true).build();
    assert_eq!(
        format!("{:?}", filter_true),
        "BlockFilter { include_txs: true }"
    );

    let filter_false = BlockFilter::builder().include_txs(false).build();
    assert_eq!(
        format!("{:?}", filter_false),
        "BlockFilter { include_txs: false }"
    );
}

// --- ContractFilter Tests ---

// Helper struct for testing non-matching types
#[derive(Debug)]
struct NonContractEvent;

#[test]
fn contract_filter_builder_defaults_built() {
    // Build the filter with the required contract ID and default optional
    // fields
    let filter = ContractFilter::builder()
        .contract_id("default_contract".to_string())
        // Do not set event_names or include_metadata to check defaults
        .build();

    assert_eq!(filter.contract_id(), "default_contract");
    assert!(
        filter.event_names().is_none(),
        "Default event_names should be None"
    );
    assert!(!filter.include_metadata());
}

#[test]
fn contract_filter_builder_set_fields() {
    let filter = ContractFilter::builder()
        .contract_id("contract_abc".to_string())
        .event_names(Some(vec!["EventA".to_string(), "EventB".to_string()]))
        .include_metadata(true)
        .build();

    assert_eq!(filter.contract_id(), "contract_abc");
    assert_eq!(
        filter.event_names(),
        Some(&["EventA".to_string(), "EventB".to_string()][..])
    );
    assert!(filter.include_metadata());
}

#[test]
fn contract_filter_builder_required_contract_id() {
    let filter = ContractFilter::builder()
        .contract_id("required_id".to_string())
        .build();
    assert_eq!(filter.contract_id(), "required_id");
    assert!(filter.event_names().is_none());
    assert!(!filter.include_metadata());
}

#[test]
fn contract_filter_matches_correct_type_and_contract() {
    let filter = ContractFilter::builder()
        .contract_id("match_contract".to_string())
        .build();
    let event = ContractEventData {
        contract_id: "match_contract".to_string(),
        event_name: "SomeEvent".to_string(),
        has_metadata: false,
    };
    assert!(filter.matches(&event));
}

#[test]
fn contract_filter_does_not_match_incorrect_type() {
    let filter = ContractFilter::builder()
        .contract_id("any_contract".to_string())
        .build();
    let non_event = NonContractEvent;
    assert!(!filter.matches(&non_event));
}

#[test]
fn contract_filter_does_not_match_different_contract() {
    let filter = ContractFilter::builder()
        .contract_id("filter_contract".to_string())
        .build();
    let event = ContractEventData {
        contract_id: "different_contract".to_string(),
        event_name: "SomeEvent".to_string(),
        has_metadata: false,
    };
    assert!(!filter.matches(&event));
}

#[test]
fn contract_filter_matches_specific_event_name() {
    let filter = ContractFilter::builder()
        .contract_id("contract1".to_string())
        .event_names(Some(vec!["MatchEvent".to_string()]))
        .build();
    let event_match = ContractEventData {
        contract_id: "contract1".to_string(),
        event_name: "MatchEvent".to_string(),
        has_metadata: false,
    };
    let event_no_match_name = ContractEventData {
        contract_id: "contract1".to_string(),
        event_name: "OtherEvent".to_string(),
        has_metadata: false,
    };
    let event_no_match_contract = ContractEventData {
        contract_id: "contract2".to_string(),
        event_name: "MatchEvent".to_string(),
        has_metadata: false,
    };

    assert!(filter.matches(&event_match));
    assert!(!filter.matches(&event_no_match_name));
    assert!(!filter.matches(&event_no_match_contract));
}

#[test]
fn contract_filter_matches_any_event_name_when_none_specified() {
    let filter = ContractFilter::builder()
        .contract_id("contract_all".to_string())
        .event_names(None) // Explicitly None
        .build();
    let event1 = ContractEventData {
        contract_id: "contract_all".to_string(),
        event_name: "EventA".to_string(),
        has_metadata: false,
    };
    let event2 = ContractEventData {
        contract_id: "contract_all".to_string(),
        event_name: "EventB".to_string(),
        has_metadata: false,
    };
    let event_other_contract = ContractEventData {
        contract_id: "other".to_string(),
        event_name: "EventA".to_string(),
        has_metadata: false,
    };

    assert!(filter.matches(&event1));
    assert!(filter.matches(&event2));
    assert!(!filter.matches(&event_other_contract));
}

#[test]
fn contract_filter_matches_independent_of_include_metadata_flag() {
    let filter_with_meta = ContractFilter::builder()
        .contract_id("meta_contract".to_string())
        .include_metadata(true)
        .build();
    let filter_without_meta = ContractFilter::builder()
        .contract_id("meta_contract".to_string())
        .include_metadata(false)
        .build();

    let event = ContractEventData {
        contract_id: "meta_contract".to_string(),
        event_name: "AnyEvent".to_string(),
        has_metadata: true, // Event has metadata, doesn't affect filter match
    };

    assert!(
        filter_with_meta.matches(&event),
        "Filter with metadata should match"
    );
    assert!(
        filter_without_meta.matches(&event),
        "Filter without metadata should match"
    );
}

#[test]
fn contract_filter_implements_required_traits() {
    fn assert_traits<T: Debug + Send + Sync + Clone + 'static>() {}
    assert_traits::<ContractFilter>();
}

#[test]
fn contract_filter_debug_output() {
    let filter = ContractFilter::builder()
        .contract_id("debug_contract".to_string())
        .event_names(Some(vec!["DebugEvent".to_string()]))
        .include_metadata(true)
        .build();

    let expected_debug = "ContractFilter { contract_id: \"debug_contract\", event_names: Some([\"DebugEvent\"]), include_metadata: true }";
    assert_eq!(format!("{:?}", filter), expected_debug);

    let filter_minimal = ContractFilter::builder()
        .contract_id("minimal_contract".to_string())
        .build();

    let expected_debug_minimal = "ContractFilter { contract_id: \"minimal_contract\", event_names: None, include_metadata: false }";
    assert_eq!(format!("{:?}", filter_minimal), expected_debug_minimal);
}

// --- TransferFilter Tests ---

// Helper struct for testing non-matching types
#[derive(Debug)]
struct NonTransferEvent;

#[test]
fn transfer_filter_builder_defaults_built() {
    // Build the filter with the required contract ID and default optional
    // fields
    let filter = TransferFilter::builder()
        .contract_id("default_token".to_string())
        // Do not set min_amount or include_metadata to check defaults
        .build();

    assert_eq!(filter.contract_id(), "default_token");
    assert!(filter.min_amount().is_none());
    assert!(!filter.include_metadata());
}

#[test]
fn transfer_filter_builder_set_fields() {
    let filter = TransferFilter::builder()
        .contract_id("token_xyz".to_string())
        .min_amount(Some(500))
        .include_metadata(true)
        .build();

    assert_eq!(filter.contract_id(), "token_xyz");
    assert_eq!(filter.min_amount(), Some(500));
    assert!(filter.include_metadata());
}

#[test]
fn transfer_filter_builder_required_contract_id() {
    let filter = TransferFilter::builder()
        .contract_id("required_token".to_string())
        .build();
    assert_eq!(filter.contract_id(), "required_token");
    assert!(filter.min_amount().is_none());
    assert!(!filter.include_metadata());
}

#[test]
fn transfer_filter_matches_correct_type_and_contract() {
    let filter = TransferFilter::builder()
        .contract_id("match_token".to_string())
        .build(); // No min_amount
    let event = TransferEventData {
        contract_id: "match_token".to_string(),
        amount: 100, // Amount doesn't matter here
    };
    assert!(filter.matches(&event));
}

#[test]
fn transfer_filter_does_not_match_incorrect_type() {
    let filter = TransferFilter::builder()
        .contract_id("any_token".to_string())
        .build();
    let non_event = NonTransferEvent;
    assert!(!filter.matches(&non_event));
}

#[test]
fn transfer_filter_does_not_match_different_contract() {
    let filter = TransferFilter::builder()
        .contract_id("filter_token".to_string())
        .build();
    let event = TransferEventData {
        contract_id: "different_token".to_string(),
        amount: 100,
    };
    assert!(!filter.matches(&event));
}

#[test]
fn transfer_filter_matches_minimum_amount() {
    let filter = TransferFilter::builder()
        .contract_id("amount_token".to_string())
        .min_amount(Some(1000))
        .build();

    let event_equal = TransferEventData {
        contract_id: "amount_token".to_string(),
        amount: 1000,
    };
    let event_greater = TransferEventData {
        contract_id: "amount_token".to_string(),
        amount: 1500,
    };
    let event_less = TransferEventData {
        contract_id: "amount_token".to_string(),
        amount: 999,
    };
    let event_other_contract = TransferEventData {
        contract_id: "other".to_string(),
        amount: 2000,
    };

    assert!(filter.matches(&event_equal));
    assert!(filter.matches(&event_greater));
    assert!(!filter.matches(&event_less));
    assert!(!filter.matches(&event_other_contract));
}

#[test]
fn transfer_filter_matches_any_amount_when_none_specified() {
    let filter = TransferFilter::builder()
        .contract_id("any_amount_token".to_string())
        .min_amount(None) // Explicitly None
        .build();

    let event1 = TransferEventData {
        contract_id: "any_amount_token".to_string(),
        amount: 0, // Should match 0
    };
    let event2 = TransferEventData {
        contract_id: "any_amount_token".to_string(),
        amount: 1_000_000,
    };
    let event_other_contract = TransferEventData {
        contract_id: "other".to_string(),
        amount: 500,
    };

    assert!(filter.matches(&event1));
    assert!(filter.matches(&event2));
    assert!(!filter.matches(&event_other_contract));
}

#[test]
fn transfer_filter_matches_independent_of_include_metadata_flag() {
    let filter_with_meta = TransferFilter::builder()
        .contract_id("meta_token".to_string())
        .min_amount(Some(100))
        .include_metadata(true)
        .build();
    let filter_without_meta = TransferFilter::builder()
        .contract_id("meta_token".to_string())
        .min_amount(Some(100))
        .include_metadata(false)
        .build();

    let event = TransferEventData {
        contract_id: "meta_token".to_string(),
        amount: 200, // Amount matches min_amount
    };

    assert!(filter_with_meta.matches(&event));
    assert!(filter_without_meta.matches(&event));
}

#[test]
fn transfer_filter_implements_required_traits() {
    fn assert_traits<T: Debug + Send + Sync + Clone + 'static>() {}
    assert_traits::<TransferFilter>();
}

#[test]
fn transfer_filter_debug_output() {
    let filter = TransferFilter::builder()
        .contract_id("debug_token".to_string())
        .min_amount(Some(10000))
        .include_metadata(true)
        .build();

    let expected_debug = "TransferFilter { contract_id: \"debug_token\", min_amount: Some(10000), include_metadata: true }";
    assert_eq!(format!("{:?}", filter), expected_debug);

    let filter_minimal = TransferFilter::builder()
        .contract_id("minimal_token".to_string())
        .build();

    let expected_debug_minimal = "TransferFilter { contract_id: \"minimal_token\", min_amount: None, include_metadata: false }";
    assert_eq!(format!("{:?}", filter_minimal), expected_debug_minimal);
}

// --- MempoolFilter Tests ---

// Helper struct for testing non-matching types
#[derive(Debug)]
struct NonMempoolEvent;

#[test]
fn mempool_filter_builder_defaults() {
    let filter = MempoolFilter::builder().build();
    assert!(filter.contract_id().is_none());
    assert!(!filter.include_details()); // Default is false
}

#[test]
fn mempool_filter_builder_set_fields() {
    let filter = MempoolFilter::builder()
        .contract_id(Some("contract_abc".to_string()))
        .include_details(true)
        .build();

    assert_eq!(filter.contract_id(), Some("contract_abc"));
    assert!(filter.include_details());

    let filter_no_contract = MempoolFilter::builder()
        .contract_id(None)
        .include_details(false)
        .build();

    assert!(filter_no_contract.contract_id().is_none());
    assert!(!filter_no_contract.include_details());
}

#[test]
fn mempool_filter_matches_correct_type() {
    let filter_any = MempoolFilter::builder().build(); // Matches any contract
    let event = MempoolEventData {
        contract_id: None,
        has_details: false,
    };
    assert!(filter_any.matches(&event));
}

#[test]
fn mempool_filter_does_not_match_incorrect_type() {
    let filter = MempoolFilter::builder().build();
    let non_event = NonMempoolEvent;
    assert!(!filter.matches(&non_event));
}

#[test]
fn mempool_filter_matches_specific_contract() {
    let filter = MempoolFilter::builder()
        .contract_id(Some("match_contract".to_string()))
        .build();

    let event_match = MempoolEventData {
        contract_id: Some("match_contract".to_string()),
        has_details: true,
    };
    let event_different = MempoolEventData {
        contract_id: Some("other_contract".to_string()),
        has_details: true,
    };
    let event_none = MempoolEventData {
        contract_id: None,
        has_details: false,
    };

    assert!(filter.matches(&event_match));
    assert!(!filter.matches(&event_different));
    assert!(!filter.matches(&event_none)); // Event has no contract, filter
                                           // expects one
}

#[test]
fn mempool_filter_matches_any_contract_when_none_specified() {
    let filter_any = MempoolFilter::builder()
        .contract_id(None) // Explicitly None
        .build();

    let event_contract_1 = MempoolEventData {
        contract_id: Some("contract_1".to_string()),
        has_details: true,
    };
    let event_contract_2 = MempoolEventData {
        contract_id: Some("contract_2".to_string()),
        has_details: false,
    };
    let event_no_contract = MempoolEventData {
        contract_id: None,
        has_details: true,
    };

    assert!(filter_any.matches(&event_contract_1));
    assert!(filter_any.matches(&event_contract_2));
    assert!(filter_any.matches(&event_no_contract));
}

#[test]
fn mempool_filter_matches_independent_of_include_details_flag() {
    // Filter requesting details, specific contract
    let filter_details_specific = MempoolFilter::builder()
        .contract_id(Some("details_contract".to_string()))
        .include_details(true)
        .build();
    // Filter not requesting details, specific contract
    let filter_no_details_specific = MempoolFilter::builder()
        .contract_id(Some("details_contract".to_string()))
        .include_details(false)
        .build();

    // Filter requesting details, any contract
    let filter_details_any = MempoolFilter::builder()
        .contract_id(None)
        .include_details(true)
        .build();
    // Filter not requesting details, any contract
    let filter_no_details_any = MempoolFilter::builder()
        .contract_id(None)
        .include_details(false)
        .build();

    let event_matching_contract = MempoolEventData {
        contract_id: Some("details_contract".to_string()),
        has_details: true, // Arbitrary event detail status
    };
    let event_non_matching_contract = MempoolEventData {
        contract_id: Some("other".to_string()),
        has_details: false,
    };
    let event_no_contract = MempoolEventData {
        contract_id: None,
        has_details: true,
    };

    // Check specific contract filters
    assert!(filter_details_specific.matches(&event_matching_contract));
    assert!(filter_no_details_specific.matches(&event_matching_contract));
    assert!(!filter_details_specific.matches(&event_non_matching_contract));
    assert!(!filter_no_details_specific.matches(&event_non_matching_contract));
    assert!(!filter_details_specific.matches(&event_no_contract));
    assert!(!filter_no_details_specific.matches(&event_no_contract));

    // Check any contract filters
    assert!(filter_details_any.matches(&event_matching_contract));
    assert!(filter_no_details_any.matches(&event_matching_contract));
    assert!(filter_details_any.matches(&event_non_matching_contract));
    assert!(filter_no_details_any.matches(&event_non_matching_contract));
    assert!(filter_details_any.matches(&event_no_contract));
    assert!(filter_no_details_any.matches(&event_no_contract));
}

#[test]
fn mempool_filter_implements_required_traits() {
    fn assert_traits<T: Debug + Send + Sync + Clone + Default + 'static>() {}
    assert_traits::<MempoolFilter>();
}

#[test]
fn mempool_filter_debug_output() {
    let filter_specific = MempoolFilter::builder()
        .contract_id(Some("debug_contract".to_string()))
        .include_details(true)
        .build();
    let expected_debug_specific = "MempoolFilter { contract_id: Some(\"debug_contract\"), include_details: true }";
    assert_eq!(format!("{:?}", filter_specific), expected_debug_specific);

    let filter_any = MempoolFilter::builder()
        .contract_id(None)
        .include_details(false)
        .build();
    let expected_debug_any =
        "MempoolFilter { contract_id: None, include_details: false }";
    assert_eq!(format!("{:?}", filter_any), expected_debug_any);
}
