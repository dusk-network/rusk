# Current Feature Implementation Plan

## Table of Contents

- [Current Feature Implementation Plan](#current-feature-implementation-plan)
  - [Table of Contents](#table-of-contents)
  - [Feature Description](#feature-description)
  - [Subscription Management Implementation Tasks](#subscription-management-implementation-tasks)
    - [1. Module Structure and Setup](#1-module-structure-and-setup)
      - [1.1 Create module structure](#11-create-module-structure)
      - [1.2 Define subscription error types](#12-define-subscription-error-types)
    - [2. Core Subscription Types](#2-core-subscription-types)
      - [2.1 Create `Topic` enum](#21-create-topic-enum)
      - [2.2 Create `SubscriptionId` type](#22-create-subscriptionid-type)
      - [2.3 Create `SessionId` type](#23-create-sessionid-type)
      - [2.4 Create subscription parameters types](#24-create-subscription-parameters-types)
      - [2.5 Document `types` module](#25-document-types-module)
    - [3. Filter Implementation (`jsonrpc::infrastructure::subscription::filters`)](#3-filter-implementation-jsonrpcinfrastructuresubscriptionfilters)
      - [3.1 Design `Filter` trait](#31-design-filter-trait)
      - [3.2 Implement `BlockFilter`](#32-implement-blockfilter)
      - [3.3 Implement `ContractFilter`](#33-implement-contractfilter)
      - [3.4 Implement `TransferFilter`](#34-implement-transferfilter)
      - [3.5 Implement `MempoolFilter`](#35-implement-mempoolfilter)
      - [3.6 Document `filters` module and all its submodules](#36-document-filters-module-and-all-its-submodules)
    - [4. Subscription Manager Implementation](#4-subscription-manager-implementation)
      - [4.1 Create `SubscriptionManager` skeleton](#41-create-subscriptionmanager-skeleton)
      - [4.2 Implement thread safety](#42-implement-thread-safety)
      - [4.3 Add event channel and handling](#43-add-event-channel-and-handling)
      - [4.4 Implement background task for event processing](#44-implement-background-task-for-event-processing)
      - [4.5 Document `manager` module](#45-document-manager-module)
    - [5. Subscription Lifecycle Methods](#5-subscription-lifecycle-methods)
      - [5.1 Implement `add_subscription` method](#51-implement-add_subscription-method)
      - [5.2 Implement `remove_subscription` method](#52-implement-remove_subscription-method)
      - [5.3 Implement `remove_session_subscriptions` method](#53-implement-remove_session_subscriptions-method)
      - [5.4 Implement synchronous `publish` method](#54-implement-synchronous-publish-method)
      - [5.5 Implement asynchronous `publish_async` method](#55-implement-asynchronous-publish_async-method)
    - [6. Subscription Status and Management](#6-subscription-status-and-management)
      - [6.1 Implement `SubscriptionStats` struct](#61-implement-subscriptionstats-struct)
      - [6.2 Implement `get_subscription_status` method](#62-implement-get_subscription_status-method)
      - [6.3 Integrate `ManualRateLimiters` for Event Delivery](#63-integrate-manualratelimiters-for-event-delivery)
      - [6.4 Integrate `ManualRateLimiters` for Subscription Creation](#64-integrate-manualratelimiters-for-subscription-creation)
    - [7. Error Handling and Cleanup](#7-error-handling-and-cleanup)
      - [7.1 Enhance background task with error handling](#71-enhance-background-task-with-error-handling)
      - [7.2 Add subscription cleanup logic](#72-add-subscription-cleanup-logic)
    - [8. State Integration](#8-state-integration)
      - [8.1 Remove placeholder from state.rs](#81-remove-placeholder-from-staters)
      - [8.2 Update `AppState` to use new implementation](#82-update-appstate-to-use-new-implementation)
    - [9. Documentation](#9-documentation)
      - [9.1 Add module-level documentation](#91-add-module-level-documentation)
      - [9.2 Document synchronous vs asynchronous publishing](#92-document-synchronous-vs-asynchronous-publishing)
      - [9.3 Add thread-safety documentation](#93-add-thread-safety-documentation)
    - [Testing](#testing)
      - [Create unit tests for SubscriptionManager](#create-unit-tests-for-subscriptionmanager)
      - [Create integration tests for each subscription type](#create-integration-tests-for-each-subscription-type)
      - [Test background task behavior](#test-background-task-behavior)
  - [Files Involved](#files-involved)
  - [Notes](#notes)
    - [Reference for documentation](#reference-for-documentation)
      - [Synchronous vs Asynchronous Publishing](#synchronous-vs-asynchronous-publishing)
        - [Synchronous Publishing](#synchronous-publishing)
        - [Asynchronous Publishing](#asynchronous-publishing)

## Feature Description

WebSocket subscriptions handling for the JSON-RPC server created with the `jsonrpsee` crate.

```rust
pub struct SubscriptionManager {
    // Topic-based registry for efficient publishing
    topic_subscriptions: HashMap<Topic, HashMap<SubscriptionId, SubscriptionSink>>,
    
    // Session-based registry for efficient unsubscribe
    session_subscriptions: HashMap<SessionId, HashSet<(Topic, SubscriptionId)>>,
    
    // Optional filters for subscriptions
    filters: HashMap<SubscriptionId, Filter>,
    
    // Channel for background event processing
    event_sender: mpsc::Sender<(Topic, Box<dyn Any + Send>)>,
}
```

- `SubscriptionId` is a unique identifier (typically UUID or string) assigned to each subscription. In the `jsonrpsee` crate, this is used to identify and manage individual subscriptions.
- `SubscriptionSink` represents the communication channel to a specific client. In `jsonrpsee` crate, it's the object that allows sending subscription notifications back to clients. It's effectively a handle to the WebSocket connection for a particular subscription.
- `Filter` would allow fine-grained control over what events a client receives:
  - For example, when subscribing to new blocks, a client might only want blocks containing transactions to specific addresses
  - Without filters, we'd send all events of a topic to every subscriber
  - With filters, we can reduce network traffic by only sending relevant events

## Subscription Management Implementation Tasks

### 1. Module Structure and Setup

#### 1.1 Create module structure

- [x] Create `src/lib/jsonrpc/infrastructure/subscription/mod.rs`
- [x] Set up submodules: `types.rs`, `filter.rs`, `manager.rs`, `error.rs`

#### 1.2 Define subscription error types

- [x] Create `error.rs` with `SubscriptionError` enum with variants:
  - [x] `InvalidTopic` - When topic is not supported/valid
  - [x] `InvalidSubscription` - When subscription ID doesn't exist/is invalid
  - [x] `InvalidSubscriptionIdFormat` - When subscription ID is not a valid UUID
  - [x] `InvalidSessionIdFormat` - When session ID is not a valid UUID
  - [x] `InvalidFilter` - When filter configuration is invalid
  - [x] `SessionNotFound` - When session ID doesn't exist
  - [x] `PublishFailed` - When event delivery fails
  - [x] `ChannelClosed` - When event channel is closed
  - [x] `TopicClosed` - When a topic has been closed
  - [x] `TooManySubscriptions` - When subscription limits are exceeded
- [x] Document each error variant and its purpose with examples and doc tests
- [x] Add comprehensive documentation to the module with examples and doc tests

### 2. Core Subscription Types

#### 2.1 Create `Topic` enum

- [x] Define in `types.rs` with variants matching JSON-RPC methods from `rusk/docs/JSON-RPC-websocket-methods.md`:
  - [x] `BlockAcceptance` - For subscribeBlockAcceptance
  - [x] `BlockFinalization` - For subscribeBlockFinalization
  - [x] `ChainReorganization` - For subscribeChainReorganization
  - [x] `ContractEvents` - For subscribeContractEvents
  - [x] `ContractTransferEvents` - For subscribeContractTransferEvents
  - [x] `MempoolAcceptance` - For subscribeMempoolAcceptance
  - [x] `MempoolEvents` - For subscribeMempoolEvents
- [x] Implement Serialize/Deserialize, Debug, Clone, PartialEq, Eq, Hash
- [x] Document each topic type and its purpose (the source of truth is `rusk/docs/JSON-RPC-websocket-methods.md`) with examples and doc tests
- [x] Add comprehensive documentation to the module with examples and doc tests
- [x] Add tests for the `Topic` enum, serialize/deserialize, debug, Display, FromStr, and as_str method

#### 2.2 Create `SubscriptionId` type

- [x] Create UUID-based newtype in `types.rs`. Look at the `Cargo.toml` and `rusk/Cargo.toml` files to see the dependencies we can reuse in the implementation.
- [x] Implement generation, validation, and formatting methods
- [x] Add conversion from/to strings for JSON-RPC compatibility
- [x] Add tests for SubscriptionId generation, Display, FromStr, serialize/deserialize

#### 2.3 Create `SessionId` type

- [x] Define newtype wrapper for client session IDs in `types.rs`
- [x] Add validation methods
- [x] Document relationship with WebSocket connections
- [x] Implement necessary traits (Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)
- [x] Add tests for creation, validation, Display, FromStr, Serde, Debug, Hash

#### 2.4 Create subscription parameters types

- [x] `BlockSubscriptionParams` with `include_txs: Option<bool>`
- [x] `ContractSubscriptionParams` with fields:
  - [x] `contract_id: String`
  - [x] `event_names: Option<Vec<String>>`
  - [x] `include_metadata: Option<bool>`
  - [x] `min_amount: Option<String>` (for transfer events)
- [x] `MempoolSubscriptionParams` with fields:
  - [x] `contract_id: Option<String>`
  - [x] `include_details: Option<bool>`
- [x] Add comprehensive documentation to all types and their fields with examples and doc tests
- [x] Add state-builder pattern to `ContractSubscriptionParams` and the regular builder pattern to the rest of the types
- [x] Add tests for the builder pattern, serialize/deserialize, Debug, and all methods

#### 2.5 Document `types` module

- [x] Document all types, traits, and their fields and methods with examples and doc tests
- [x] Add comprehensive documentation to the module with examples and doc tests

### 3. Filter Implementation (`jsonrpc::infrastructure::subscription::filters`)

#### 3.1 Design `Filter` trait

- [x] Create base `Filter` trait in `filter.rs`
- [x] Define `matches(&self, event: &dyn Any) -> bool` method
- [x] Add trait bounds: `Send + Sync + 'static`
- [x] Document extension patterns with examples and doc tests

#### 3.2 Implement `BlockFilter`

- [x] Create `BlockFilter` struct in `block_filter.rs`
- [x] Add field for `include_txs: bool`
- [x] Implement `Filter` trait for block events
- [x] Add builder pattern for construction
- [x] Document the filter implementation and all its methods and fields with examples and doc tests
- [x] Document the builder pattern and all its methods and fields with examples and doc tests
- [x] Add tests for the filter implementation and all its methods and fields

#### 3.3 Implement `ContractFilter`

- [x] Create `ContractFilter` struct in `contract_filter.rs`
- [x] Add fields:
  - [x] `contract_id: String`
  - [x] `event_names: Option<Vec<String>>`
  - [x] `include_metadata: bool`
- [x] Implement `Filter` trait for contract events
- [x] Add builder pattern for construction
- [x] Document the filter implementation and all its methods and fields with examples and doc tests
- [x] Document the builder pattern and all its methods and fields with examples and doc tests
- [x] Add tests for the filter implementation and all its methods and fields

#### 3.4 Implement `TransferFilter`

- [x] Extend `ContractFilter` for transfer events in `transfer_filter.rs`
- [x] Add field for `min_amount: Option<u64>`
- [x] Implement specialized filtering for transfer events
- [x] Add builder pattern
- [x] Document the filter implementation and all its methods and fields with examples and doc tests
- [x] Document the builder pattern and all its methods and fields with examples and doc tests
- [x] Add tests for the filter implementation and all its methods and fields

#### 3.5 Implement `MempoolFilter`

- [x] Create `MempoolFilter` struct in `mempool_filter.rs`
- [x] Add fields:
  - [x] `contract_id: Option<String>`
  - [x] `include_details: bool`
- [x] Implement `Filter` trait for mempool events
- [x] Add builder pattern
- [x] Document the filter implementation and all its methods and fields with examples and doc tests
- [x] Document the builder pattern and all its methods and fields with examples and doc tests
- [x] Add tests for the filter implementation and all its methods and fields

#### 3.6 Document `filters` module and all its submodules

- [x] Document all types, traits, and their fields and methods with examples and doc tests
- [x] Add comprehensive module-level documentation to the `filters` module and all its submodules with examples and doc tests

### 4. Subscription Manager Implementation

#### 4.1 Create `SubscriptionManager` skeleton

- [ ] Define basic struct in `manager.rs`
- [ ] Include the dual registry fields:
  - [ ] `topic_subscriptions: HashMap<Topic, HashMap<SubscriptionId, SubscriptionSink>>`
  - [ ] `session_subscriptions: HashMap<SessionId, HashSet<(Topic, SubscriptionId)>>`
- [ ] Add filter storage: `filters: HashMap<SubscriptionId, Box<dyn Filter>>`
- [ ] Add subscription statistics tracking: `subscription_stats: HashMap<SubscriptionId, SubscriptionStats>`
- [ ] Add constructor:
  - [ ] Accepts `broadcast::Receiver<RuesEvent>` (from the node builder)
  - [ ] Spawns the background task (defined in 4.4)

#### 4.2 Implement thread safety

- [ ] Wrap collections in appropriate synchronization primitives
- [ ] Ensure proper locking strategy to prevent deadlocks
- [ ] Document thread-safety guarantees
- [ ] Ensure `Send + Sync` trait implementation

#### 4.3 Add event channel and handling

- [ ] Create event channel in constructor: `event_sender: mpsc::Sender<(Topic, Box<dyn Any + Send>)>`
- [ ] Define event type and format
- [ ] Implement channel creation in constructor
- [ ] Add backpressure handling

#### 4.4 Implement background task for event processing

- [ ] Create a Tokio task within the `SubscriptionManager` constructor.
- [ ] **Core Logic:**
  - [ ] The task holds and continuously listens to the `broadcast::Receiver<RuesEvent>` provided during construction.
  - [ ] When a `RuesEvent` is received (handle potential `Lagged` errors from the broadcast receiver):
    - [ ] **Map `RuesEvent` to `Topic`:** Determine the corresponding internal `Topic` based on the `RuesEvent`'s type or data (e.g., `RuesEvent` containing a block might map to `Topic::BlockFinalization`). Define this mapping logic clearly.
    - [ ] **Identify Subscribers:** Look up the relevant `SubscriptionId`s and `SubscriptionSink`s for the determined `Topic` in the `topic_subscriptions` registry.
    - [ ] **Apply Filters:** For each potential subscriber, retrieve their associated `Filter` (if any) from the `filters` map.
    - [ ] Use the `filter.matches(&event)` method (passing the relevant data from the `RuesEvent`) to check if the event should be sent to this specific subscriber.
    - [ ] **Send to Sink:** If the event matches the filter (or if there's no filter), attempt to send the appropriately formatted event payload (derived from `RuesEvent`) to the client using their `SubscriptionSink`.
    - [ ] **Handle Sink Errors:** Manage errors from `SubscriptionSink::send` (e.g., client disconnected, buffer full). Implement logic to track failures and potentially clean up subscriptions after repeated errors (as planned in Task 7.1/7.2).
    - [ ] **Update Statistics:** Update `SubscriptionStats` (events processed, dropped due to buffer full, etc.).
- [ ] Add graceful shutdown handling (e.g., listening for a shutdown signal).
- [ ] Add logging for task lifecycle, event processing, and errors.

#### 4.5 Document `manager` module

- [ ] Document all types, traits, and their fields and methods with examples and doc tests
- [ ] Add comprehensive documentation to the module with examples and doc tests

### 5. Subscription Lifecycle Methods

#### 5.1 Implement `add_subscription` method

- [ ] Add parameters: `session_id: SessionId, topic: Topic, sink: SubscriptionSink, filter: Option<Box<dyn Filter>>`
- [ ] Generate subscription ID
- [ ] Register in topic_subscriptions
- [ ] Register in session_subscriptions
- [ ] Store filter if provided
- [ ] Initialize subscription statistics
- [ ] Return subscription ID
- [ ] Document with examples and doc tests

#### 5.2 Implement `remove_subscription` method

- [ ] Accept subscription ID
- [ ] Remove from topic registry
- [ ] Remove from session registry
- [ ] Remove associated filter
- [ ] Remove statistics
- [ ] Return appropriate error if not found
- [ ] Document with examples and doc tests

#### 5.3 Implement `remove_session_subscriptions` method

- [ ] Accept session ID
- [ ] Find all subscriptions for session
- [ ] Remove each from both registries
- [ ] Clean up filters and statistics
- [ ] Handle errors properly
- [ ] Document with examples and doc tests

#### 5.4 Implement synchronous `publish` method

- [ ] Accept topic and event
- [ ] Find all subscriptions for topic
- [ ] Apply filters to determine relevant subscribers
- [ ] Send to each subscription sink
- [ ] Update statistics (events processed/dropped)
- [ ] Handle failures and track failed deliveries
- [ ] Document blocking nature and use cases with examples and doc tests

#### 5.5 Implement asynchronous `publish_async` method

- [ ] Accept topic and event
- [ ] Send to channel
- [ ] Handle backpressure
- [ ] Return quickly
- [ ] Document non-blocking behavior and use cases with examples and doc tests

### 6. Subscription Status and Management

This approach leverages the existing `ManualRateLimiters` stored in `AppState`:

- Event Delivery: By calling `check_method_limit` before sending an event to a specific client's `SubscriptionSink`, we can enforce per-client, per-topic rate limits defined in the configuration. This prevents a single client subscription from being overwhelmed or abusing resources for a high-frequency topic. The `ClientInfo` needed for the check must be associated with the subscription upon creation.
- Subscription Creation: We can also limit how fast a client can create new subscriptions using a dedicated pattern. This prevents clients from rapidly creating and destroying subscriptions, potentially causing unnecessary overhead.

#### 6.1 Implement `SubscriptionStats` struct

- [ ] Add fields to track:
  - [ ] `events_processed: u64`
  - [ ] `events_dropped_buffer: u64` (Events dropped due to client buffer full/slow)
  - [ ] `events_dropped_rate_limit: u64` (Events dropped due to manual rate limiter)
  - [ ] `last_event_time: Option<u64>`
  - [ ] `creation_time: u64`
- [ ] Implement update methods for these statistics.

#### 6.2 Implement `get_subscription_status` method

- [ ] Accept subscription ID
- [ ] Retrieve and return the `SubscriptionStats` for that ID.
- [ ] Format the output according to the `getSubscriptionStatus` WebSocket method specification (including `active`, `events_processed`, `events_dropped` (sum of dropped counts), `last_event_time`, and potentially calculating `throttled` based on recent `events_dropped_rate_limit`).

#### 6.3 Integrate `ManualRateLimiters` for Event Delivery

- [ ] Determine Rate Limit Patterns: Define specific method patterns in the `RateLimitConfig` for different subscription topics (e.g., "subscription:BlockAcceptance", "subscription:ContractEvents:<contract_id_pattern>"). These patterns will be used by `ManualRateLimiters`.
- [ ] Check Before Sending: In both `publish` (within the loop sending to sinks) and the background task (before sending to a sink), call `AppState::manual_rate_limiters().check_method_limit(&client_info, "<topic_pattern>")` before attempting to send an event via the `SubscriptionSink`.
  - The relevant `ClientInfo` would likely need to be stored alongside the `SubscriptionSink` when the subscription is created.
- [ ] Handle Rate Limit Errors: If `check_method_limit` returns `Err(RateLimitError::ManualMethodLimitExceeded)`, do not send the event to that specific sink.
- [ ] Update Statistics: Increment the `events_dropped_rate_limit` counter in `SubscriptionStats` for the specific subscription when an event is dropped due to the rate limit check.

#### 6.4 Integrate `ManualRateLimiters` for Subscription Creation

- [ ] Define Creation Pattern: Define a pattern like "subscription:create" in the `RateLimitConfig`.
- [ ] Check in `add_subscription`: At the beginning of the `add_subscription` method, call `AppState::manual_rate_limiters().check_method_limit(&client_info, "subscription:create")`.
- [ ] Handle Creation Limit Errors: If the check fails, return an appropriate `SubscriptionError` (e.g., `SubscriptionError::RateLimitExceeded`) to the caller, preventing the subscription from being created.

### 7. Error Handling and Cleanup

#### 7.1 Enhance background task with error handling

- [ ] Add retry logic for temporary failures
- [ ] Track failed sends
- [ ] Remove subscriptions after repeated failures
- [ ] Add metrics for failed sends (metrics are implemented in `rusk::jsonrpc::infrastructure::metrics`)

#### 7.2 Add subscription cleanup logic

- [ ] Detect and remove stale subscriptions
- [ ] Implement periodic cleanup task
- [ ] Add timeout configuration
- [ ] Document cleanup strategy

### 8. State Integration

#### 8.1 Remove placeholder from state.rs

- [ ] Delete the `SubscriptionManager` placeholder struct
- [ ] Update imports

#### 8.2 Update `AppState` to use new implementation

- [ ] Modify the type in `AppState`
- [ ] Update the constructor
- [ ] Ensure all methods still work
- [ ] Fix the tests and doc tests
- [ ] Add comprehensive documentation to the module with examples and doc tests

### 9. Documentation

#### 9.1 Add module-level documentation

- [ ] Explain overall subscription architecture
- [ ] Document dual-registry design rationale
- [ ] Explain thread-safety model
- [ ] Provide usage examples matching the WebSocket methods document (see `rusk/docs/JSON-RPC-websocket-methods.md`)

#### 9.2 Document synchronous vs asynchronous publishing

- [ ] Explain differences in detail
- [ ] Provide examples for when to use each:
  - [ ] Synchronous: critical notifications, sequential dependent events, low-volume events
  - [ ] Asynchronous: high-frequency events, non-critical background notifications, batched updates
- [ ] Discuss performance implications
- [ ] Cover error handling differences

#### 9.3 Add thread-safety documentation

- [ ] Explain concurrent access patterns
- [ ] Document lock ordering if applicable
- [ ] Explain safe shutdown procedures
- [ ] Address potential deadlocks

### Testing

#### Create unit tests for SubscriptionManager

- [ ] Test subscription registration/removal
- [ ] Test session cleanup
- [ ] Test filter application
- [ ] Test statistics tracking
- [ ] Test error handling

#### Create integration tests for each subscription type

- [ ] Test block event subscriptions
- [ ] Test contract event subscriptions
- [ ] Test mempool event subscriptions
- [ ] Test status checking
- [ ] Test unsubscribe functionality

#### Test background task behavior

- [ ] Test event processing
- [ ] Test cleanup of failed subscriptions
- [ ] Test shutdown behavior
- [ ] Test with simulated errors
- [ ] Test throttling behavior

## Files Involved

- **`rusk/docs/current-feature-plan.md`**: (This plan)
- **`rusk/docs/instructions.md`**: (Instructions to follow)
- **`rusk/docs/implementation-plan.md`**: (To update task status later)
- **`rusk/docs/JSON-RPC-websocket-methods.md`**: (Reference) Specifics of the JSON-RPC websocket methods we need to implement
- **`rusk/src/lib/jsonrpc/infrastructure/metrics.rs`**: Metrics implementation
- **`rusk/src/lib/jsonrpc/infrastructure/manual_limiter.rs`**: Manual rate limiter implementation
- **`rusk/src/lib/jsonrpc/infrastructure/state.rs`**: `AppState` implementation
- **`rusk/src/lib/jsonrpc/infrastructure/error.rs`**: Infrastructure error implementation
- **`rusk/src/lib/jsonrpc/error.rs`**: Central place for error handling

## Notes

### Reference for documentation

#### Synchronous vs Asynchronous Publishing

The distinction serves different use cases:

##### Synchronous Publishing

```rust
fn publish(topic, event) -> Result<(), Error>
```

**Use cases:**

1. **Critical notifications** where you need confirmation of delivery:
   - Account balance changes where confirmation is essential
   - Security-critical events where you need to know clients received the notification

2. **Sequential dependent events** that must arrive in perfect order:
   - A transaction status changing through multiple states
   - Events where the sequence is critical to client understanding

3. **Low-volume, high-importance events** where blocking is acceptable:
   - Administrative notifications
   - System state changes that affect client behavior

##### Asynchronous Publishing

```rust
fn publish_async(topic, event) -> Result<(), Error>
```

**Use cases:**

1. **High-frequency events** where throughput is critical:
   - Block header notifications during chain synchronization
   - Market data updates
   - Metrics reporting

2. **Non-critical background notifications**:
   - Network statistics
   - Peer discovery events
   - Debug information

3. **Batched updates** that can be delivered together:
   - Multiple related transactions
   - Groups of events that occur in bursts

The async option prevents notification handling from blocking your main processing flow, which is crucial for maintaining server responsiveness under high load.
