// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Debug;

use rusk::jsonrpc::infrastructure::subscription::filter::{
    BlockEventData, BlockFilter, Filter,
};

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
