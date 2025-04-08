// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Tests for the WebSocket subscription infrastructure.

use rusk::jsonrpc::infrastructure::subscription::error::SubscriptionError;
use rusk::jsonrpc::infrastructure::subscription::types::Topic;
use std::str::FromStr;

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
