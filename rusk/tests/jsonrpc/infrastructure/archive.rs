// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::jsonrpc::utils::{create_mock_moonlight_group, MockArchiveAdapter};
use rusk::jsonrpc::{
    infrastructure::archive::ArchiveAdapter,
    model::archive::{ArchivedEvent, Order},
};
use std::sync::Arc;

// Helper to create a default mock adapter
fn default_mock() -> MockArchiveAdapter {
    MockArchiveAdapter::default()
}

// --- Test get_moonlight_txs_by_memo ---

#[tokio::test]
async fn test_get_moonlight_txs_by_memo_success() {
    let mut mock = default_mock();
    let memo = vec![1, 2, 3];
    let expected_group = create_mock_moonlight_group("tx_memo", 100);
    mock.txs_by_memo.insert(memo.clone(), vec![expected_group]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_moonlight_txs_by_memo(memo).await;

    assert!(result.is_ok());
    let opt_groups = result.unwrap();
    assert!(opt_groups.is_some());
    let groups = opt_groups.unwrap();
    assert_eq!(groups.len(), 1);
    let expected_group_for_cmp = create_mock_moonlight_group("tx_memo", 100);
    assert_eq!(groups[0], expected_group_for_cmp);
}

#[tokio::test]
async fn test_get_moonlight_txs_by_memo_not_found() {
    let mock = default_mock(); // Empty mock
    let memo = vec![1, 2, 3];
    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_moonlight_txs_by_memo(memo).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_get_moonlight_txs_by_memo_error() {
    let mut mock = default_mock();
    let memo = vec![1, 2, 3];
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Forced error".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_moonlight_txs_by_memo(memo).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_last_archived_block ---

#[tokio::test]
async fn test_get_last_archived_block_success() {
    let mut mock = default_mock();
    let expected_height = 1234u64;
    let expected_hash = "hash1234".to_string();
    mock.last_archived_block = Some((expected_height, expected_hash.clone()));

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_last_archived_block().await;

    assert!(result.is_ok());
    let (height, hash) = result.unwrap();
    assert_eq!(height, expected_height);
    assert_eq!(hash, expected_hash);
}

#[tokio::test]
async fn test_get_last_archived_block_not_found() {
    let mock = default_mock(); // last_archived_block is None
    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_last_archived_block().await;

    // Mock implementation returns ArchiveError::NotFound if Option is None
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        rusk::jsonrpc::infrastructure::error::ArchiveError::NotFound(_)
    ));
}

#[tokio::test]
async fn test_get_last_archived_block_error() {
    let mut mock = default_mock();
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Forced DB error".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_last_archived_block().await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_block_events_by_hash ---

// Helper to create a mock archived event
fn create_mock_archived_event(origin: &str, topic: &str) -> ArchivedEvent {
    ArchivedEvent {
        origin: origin.to_string(),
        topic: topic.to_string(),
        source: "contract_source".to_string(),
        data: vec![0, 1, 2],
    }
}

#[tokio::test]
async fn test_get_block_events_by_hash_success() {
    let mut mock = default_mock();
    let hash = "hash1".to_string();
    let event1 = create_mock_archived_event("origin1", "topicA");
    let event2 = create_mock_archived_event("origin2", "topicB");
    mock.events_by_hash
        .insert(hash.clone(), vec![event1.clone(), event2.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_hash(&hash).await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], event1);
    assert_eq!(events[1], event2);
}

#[tokio::test]
async fn test_get_block_events_by_hash_empty() {
    let mut mock = default_mock();
    let hash = "hash1".to_string();
    // Insert empty vec for the hash
    mock.events_by_hash.insert(hash.clone(), vec![]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_hash(&hash).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_block_events_by_hash_not_found() {
    let mock = default_mock(); // Hash not present
    let hash = "hash_nonexistent".to_string();
    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_hash(&hash).await;

    // Mock returns Ok(vec![]) when not found
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_block_events_by_hash_error() {
    let mut mock = default_mock();
    let hash = "hash1".to_string();
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Error getting events".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_hash(&hash).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_block_events_by_height ---

#[tokio::test]
async fn test_get_block_events_by_height_success() {
    let mut mock = default_mock();
    let height = 500u64;
    let event1 = create_mock_archived_event("originH1", "topicH1");
    let event2 = create_mock_archived_event("originH2", "topicH2");
    mock.events_by_height
        .insert(height, vec![event1.clone(), event2.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_height(height).await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], event1);
    assert_eq!(events[1], event2);
}

#[tokio::test]
async fn test_get_block_events_by_height_empty() {
    let mut mock = default_mock();
    let height = 501u64;
    mock.events_by_height.insert(height, vec![]); // Insert empty vec

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_height(height).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_block_events_by_height_not_found() {
    let mock = default_mock(); // Height not present
    let height = 502u64;
    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_height(height).await;

    // Mock returns Ok(vec![]) when not found
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_block_events_by_height_error() {
    let mut mock = default_mock();
    let height = 503u64;
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Error getting events by height".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_block_events_by_height(height).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_latest_block_events ---

#[tokio::test]
async fn test_get_latest_block_events_success() {
    let mut mock = default_mock();
    let latest_height = 1234u64;
    let latest_hash = "hash1234".to_string();
    mock.last_archived_block = Some((latest_height, latest_hash));

    let event1 = create_mock_archived_event("latest1", "topicL1");
    let event2 = create_mock_archived_event("latest2", "topicL2");
    mock.events_by_height
        .insert(latest_height, vec![event1.clone(), event2.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_latest_block_events().await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], event1);
    assert_eq!(events[1], event2);
}

#[tokio::test]
async fn test_get_latest_block_events_error_first_call() {
    let mut mock = default_mock();
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::NotFound(
            "Mock last archived block not set".to_string(),
        );
    // Configure mock to fail on get_last_archived_block (by setting it to None)
    mock.last_archived_block = None;

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_latest_block_events().await;

    assert!(result.is_err());
    // Check that the error comes from the first internal call
    assert_eq!(result.unwrap_err(), expected_error);
}

#[tokio::test]
async fn test_get_latest_block_events_error_second_call() {
    let mut mock = default_mock();
    let latest_height = 1234u64;
    let latest_hash = "hash1234".to_string();
    mock.last_archived_block = Some((latest_height, latest_hash));

    // Don't populate events_by_height for latest_height, let it fail
    // Set a force_error to simulate failure in the second call
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Error in second call".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_latest_block_events().await;

    assert!(result.is_err());
    // Check that the error comes from the second internal call (forced)
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_contract_finalized_events ---

#[tokio::test]
async fn test_get_contract_finalized_events_success() {
    let mut mock = default_mock();
    let contract_id = "contract1".to_string();
    let event1 = create_mock_archived_event("contract_origin1", "topicC1");
    let event2 = create_mock_archived_event("contract_origin2", "topicC2");
    mock.finalized_events_by_contract
        .insert(contract_id.clone(), vec![event1.clone(), event2.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_contract_finalized_events(&contract_id).await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], event1);
    assert_eq!(events[1], event2);
}

#[tokio::test]
async fn test_get_contract_finalized_events_empty() {
    let mut mock = default_mock();
    let contract_id = "contract2".to_string();
    mock.finalized_events_by_contract
        .insert(contract_id.clone(), vec![]); // Empty vec

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_contract_finalized_events(&contract_id).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_contract_finalized_events_not_found() {
    let mock = default_mock(); // Contract not present
    let contract_id = "contract_nonexistent".to_string();
    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_contract_finalized_events(&contract_id).await;

    // Mock returns Ok(vec![]) when not found
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_get_contract_finalized_events_error() {
    let mut mock = default_mock();
    let contract_id = "contract3".to_string();
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Error getting finalized events".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_contract_finalized_events(&contract_id).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_next_block_with_phoenix_transaction ---

#[tokio::test]
async fn test_get_next_block_with_phoenix_success_found() {
    let mut mock = default_mock();
    let input_height = 100u64;
    let expected_next_height = 105u64;
    mock.next_phoenix_height
        .insert(input_height, Some(expected_next_height));

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_next_block_with_phoenix_transaction(input_height)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(expected_next_height));
}

#[tokio::test]
async fn test_get_next_block_with_phoenix_success_not_found() {
    let mut mock = default_mock();
    let input_height = 100u64;
    // Configure mock to return None for this input height
    mock.next_phoenix_height.insert(input_height, None);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_next_block_with_phoenix_transaction(input_height)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn test_get_next_block_with_phoenix_key_not_present() {
    let mock = default_mock(); // Key 100 not present in next_phoenix_height
    let input_height = 100u64;

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_next_block_with_phoenix_transaction(input_height)
        .await;

    // Mock implementation returns Ok(None) if key is not found
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[tokio::test]
async fn test_get_next_block_with_phoenix_error() {
    let mut mock = default_mock();
    let input_height = 100u64;
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Error finding next phoenix".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_next_block_with_phoenix_transaction(input_height)
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

// --- Test get_moonlight_transaction_history ---

#[tokio::test]
async fn test_get_moonlight_transaction_history_success() {
    let mut mock = default_mock();
    // Use a valid bs58 key for testing
    let pk_bs58 = "9w1yJeBMRpaEAzXj3 அப்பாற்பட்டது".to_string(); // Example bs58 key
    let group1 = create_mock_moonlight_group("hist_tx1", 200);
    let group2 = create_mock_moonlight_group("hist_tx2", 201);
    mock.moonlight_history
        .insert(pk_bs58.clone(), vec![group1.clone(), group2.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_moonlight_transaction_history(
            pk_bs58,
            Some(Order::Descending),
            Some(100),
            Some(300),
        )
        .await;

    assert!(result.is_ok());
    let opt_groups = result.unwrap();
    assert!(opt_groups.is_some());
    let groups = opt_groups.unwrap();
    assert_eq!(groups.len(), 2);
    // Note: Mock adapter doesn't actually implement filtering/ordering, just
    // returns stored value
    assert_eq!(groups[0], group1);
    assert_eq!(groups[1], group2);
}

#[tokio::test]
async fn test_get_moonlight_transaction_history_not_found() {
    let mock = default_mock();
    let pk_bs58 = "9w1yJeBMRpaEAzXj3 அப்பாற்பட்டது".to_string(); // Key not in mock

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_moonlight_transaction_history(pk_bs58, None, None, None)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // Mock returns None if key not found
}

#[tokio::test]
async fn test_get_moonlight_transaction_history_error_underlying() {
    let mut mock = default_mock();
    let pk_bs58 = "9w1yJeBMRpaEAzXj3 அப்பாற்பட்டது".to_string();
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::QueryFailed(
            "Underlying history error".to_string(),
        );
    mock.force_error = Some(expected_error.clone());

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_moonlight_transaction_history(pk_bs58, None, None, None)
        .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), expected_error);
}

#[tokio::test]
async fn test_get_moonlight_transaction_history_error_invalid_bs58() {
    let mock = default_mock();
    let pk_bs58_invalid = "Invalid Base58 String".to_string();

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_moonlight_transaction_history(pk_bs58_invalid, None, None, None)
        .await;

    // The MockArchiveAdapter doesn't perform input validation.
    // It treats the invalid key simply as not found in its map.
    // Therefore, we expect Ok(None), not an error.
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// Note: Testing the pk_bytes error case requires a valid bs58 string that
// doesn't decode to a valid public key. The RuskArchiveAdapter correctly
// handles the Result from NodePublicKey::from_slice, which performs these
// checks.

// --- Test Default Methods ---

#[tokio::test]
async fn test_default_get_last_archived_block_height() {
    let mut mock = default_mock();
    let expected_height = 1234u64;
    let expected_hash = "hash1234".to_string();
    // Mock the underlying primitive method
    mock.last_archived_block = Some((expected_height, expected_hash));

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    // Call the default method
    let result = adapter.get_last_archived_block_height().await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_height);
}

#[tokio::test]
async fn test_default_get_last_archived_block_height_error() {
    let mut mock = default_mock();
    let expected_error =
        rusk::jsonrpc::infrastructure::error::ArchiveError::NotFound(
            "Mock last archived block not set".to_string(),
        );
    // Make the underlying primitive fail
    mock.last_archived_block = None; // This makes the mock return NotFound

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter.get_last_archived_block_height().await;

    assert!(result.is_err());
    // Use assert_eq! to compare with the specific expected error instance
    assert_eq!(result.unwrap_err(), expected_error);
}

#[tokio::test]
async fn test_default_get_contract_events_by_topic() {
    let mut mock = default_mock();
    let contract_id = "contract_for_topic_test".to_string();
    let target_topic = "target_topic".to_string();
    let other_topic = "other_topic".to_string();

    let event1 = create_mock_archived_event("originT1", &target_topic);
    let event2 = create_mock_archived_event("originT2", &other_topic);
    let event3 = create_mock_archived_event("originT3", &target_topic);

    // Mock the underlying primitive method
    mock.finalized_events_by_contract.insert(
        contract_id.clone(),
        vec![event1.clone(), event2.clone(), event3.clone()],
    );

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    // Call the default method
    let result = adapter
        .get_contract_events_by_topic(&contract_id, &target_topic)
        .await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], event1);
    assert_eq!(events[1], event3); // Check that only events with target_topic
                                   // remain
}

#[tokio::test]
async fn test_default_get_contract_events_by_topic_none_match() {
    let mut mock = default_mock();
    let contract_id = "contract_for_topic_test".to_string();
    let target_topic = "target_topic".to_string();
    let other_topic = "other_topic".to_string();

    let event1 = create_mock_archived_event("originT1", &other_topic);
    let event2 = create_mock_archived_event("originT2", &other_topic);

    mock.finalized_events_by_contract
        .insert(contract_id.clone(), vec![event1.clone(), event2.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    let result = adapter
        .get_contract_events_by_topic(&contract_id, &target_topic)
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_empty()); // No events should match the topic
}

#[tokio::test]
async fn test_default_get_contract_events_by_block_height() {
    let mut mock = default_mock();
    let height = 999u64;
    let target_source = "target_contract".to_string();
    let other_source = "other_contract".to_string();

    let event1 = ArchivedEvent {
        source: target_source.clone(),
        ..create_mock_archived_event("originBH1", "topicBH1")
    };
    let event2 = ArchivedEvent {
        source: other_source.clone(),
        ..create_mock_archived_event("originBH2", "topicBH2")
    };
    let event3 = ArchivedEvent {
        source: target_source.clone(),
        ..create_mock_archived_event("originBH3", "topicBH3")
    };

    // Mock the underlying primitive method
    mock.events_by_height
        .insert(height, vec![event1.clone(), event2.clone(), event3.clone()]);

    let adapter = Arc::new(mock) as Arc<dyn ArchiveAdapter>;
    // Call the default method
    let result = adapter
        .get_contract_events_by_block_height(height, &target_source)
        .await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], event1);
    assert_eq!(events[1], event3); // Check that only events with target_source
                                   // remain
}
