# Alternative Plans for Subscription Manager Implementation (Section 3.6.3)

This document outlines two alternative approaches for implementing Section 4 ("Subscription Manager Implementation") of the JSON-RPC server feature plan. The choice between these depends on the strategy for transitioning from the legacy HTTP server to the new JSON-RPC server.

---

## Variant A: Dual Server / Parse Legacy `RuesEvent`

**Assumption:** The new JSON-RPC server will run simultaneously with the legacy HTTP server for a transition period. Event producers (`Rusk` VM, `ChainEventStreamer`) will continue to emit events in the existing `RuesEvent` format, and the legacy server requires this format. The new JSON-RPC server must adapt to this existing format.

### 3.6.3. Subscription Manager Implementation (Variant A)

#### 3.6.3.0 Relocate `RuesEvent` Definition (Prerequisite)

* **Task:** Identify and copy the definitions of `RuesEvent`, `RuesEventUri`, `DataType`, and any other necessary dependent types from the legacy `rusk::lib::http::event` module.
* **Task:** Place these copied definitions into a stable, shared location accessible by both event producers and the new `jsonrpc` module (e.g., `node-data` crate or a new `rusk-events-common` crate). Let's refer to the copied type as `RelocatedRuesEvent`.
* **Task:** **Crucially, update the `use` statements in all event producer modules (`Rusk`, `ChainEventStreamer`, etc.) to import `RelocatedRuesEvent` from its new location.** This ensures producers compile and function correctly *before* the `http` module is removed.
* **Task:** Ensure the broadcast channel created in `RuskNodeBuilder` uses the relocated type: `broadcast::channel::<RelocatedRuesEvent>()`.

#### 3.6.3.1 Create `SubscriptionManager` skeleton

* **Task:** Define the basic `SubscriptionManager` struct in `rusk::lib::jsonrpc::infrastructure::subscription::manager.rs`.
* **Task:** Include the dual registry fields using appropriate collections (e.g., `DashMap` or `RwLock<HashMap>` depending on concurrency needs):
  * `topic_subscriptions`: Map from internal `Topic` to a collection of `SubscriptionId` -> `SubscriptionSink`.
  * `session_subscriptions`: Map from `SessionId` to a collection of associated `(Topic, SubscriptionId)` tuples.
* **Task:** Add filter storage: Map from `SubscriptionId` to `Box<dyn Filter>`.
* **Task:** Add subscription statistics tracking: Map from `SubscriptionId` to `SubscriptionStats`.
* **Task:** Implement the `SubscriptionManager::new` constructor:
  * Accepts `broadcast::Receiver<RelocatedRuesEvent>` (the receiver for the channel carrying the copied/relocated event type).
  * Spawns the background event processing task (detailed in 4.4).

#### 3.6.3.2 Implement thread safety

* **Task:** Wrap all shared internal collections (`topic_subscriptions`, `session_subscriptions`, `filters`, `subscription_stats`) in appropriate concurrent primitives (e.g., `Arc<DashMap<...>>` or `Arc<RwLock<HashMap<...>>>`) to allow safe access from multiple threads (RPC handler threads adding/removing subscriptions, background task reading subscriptions).
* **Task:** Analyze and document the locking strategy to prevent deadlocks, especially between adding/removing subscriptions and the background task iterating over them.
* **Task:** Ensure `SubscriptionManager` itself derives `Clone` and is `Send + Sync`.
* **Task:** Document the thread-safety guarantees and expected concurrent usage patterns.

#### 3.6.3.3 Define Internal `SystemEvent` Structures

* **Task:** Define a structured `SystemEvent` enum within the `rusk::lib::jsonrpc::infrastructure::subscription` module (or a submodule).
* **Task:** Define variants corresponding to the logical event types relevant to subscriptions (e.g., `BlockFinalized`, `ContractEventEmitted`, `MempoolTxAccepted`). Each variant should hold the corresponding *structured* Rust data (e.g., `BlockHeader`, `ContractTxEvent`, `Transaction`). Import these underlying data structures from appropriate crates (e.g., `node-data`).
* **Task:** Define a `try_parse_relocated_rues_event(event: RelocatedRuesEvent) -> Result<(Topic, SystemEvent), ParseError>` function (or similar). This function is the core adaptation layer:
  * It inspects `event.uri` (based on RUES specification: `/on/[target]/[topic]`) to determine the event type and map it to the internal `Topic` enum.
  * It deserializes the `event.data` (likely `DataType::Binary`) into the appropriate structured Rust type (e.g., `BlockHeader`, `ContractTxEvent`) based on the URI.
  * It constructs and returns the corresponding `SystemEvent` variant containing the deserialized structured data.
* **Task:** Define a suitable `ParseError` enum to handle errors during URI parsing or data deserialization within `try_parse_relocated_rues_event`.

#### 3.6.3.4 Implement background task for event processing

* **Task:** Create a dedicated Tokio task spawned within the `SubscriptionManager::new` constructor.
* **Task:** **Core Event Loop:**
  * The task continuously loops, receiving `RelocatedRuesEvent` messages from the broadcast receiver passed during construction.
  * Handle potential `broadcast::error::RecvError::Lagged` errors gracefully (log a warning and continue).
* **Task:** **Event Processing Steps:** Upon receiving a `RelocatedRuesEvent`:
    1. **Parse & Deserialize:** Call `try_parse_relocated_rues_event` to convert the raw event into `Result<(Topic, SystemEvent), ParseError>`. Log `ParseError` and skip the event if conversion fails.
    2. **Identify Subscribers:** On `Ok((topic, system_event))`, acquire necessary locks and look up all `(SubscriptionId, SubscriptionSink)` pairs associated with the `topic` in the `topic_subscriptions` registry.
    3. **Apply Filters:** For each potential subscriber (`subscription_id`, `sink`):
        * Look up the associated `Filter` in the `filters` registry using `subscription_id`.
        * If a filter exists, call its `matches(&system_event_data)` method, passing a reference to the *structured data* contained within the `system_event` variant (e.g., `&block_header`). Use `&dyn Any` for the filter to downcast.
        * Proceed to the next step only if there is no filter OR the filter's `matches` method returns `true`.
    4. **Format JSON Payload:** Using the structured data within the `system_event`, construct the JSON-RPC notification payload expected by the client for this subscription type. This involves serializing the structured data into the appropriate JSON format according to the API specification.
    5. **Send to Sink:** Create the `jsonrpsee::SubscriptionMessage` containing the formatted JSON payload and attempt to send it asynchronously using `SubscriptionSink::send()`.
    6. **Handle Sink Errors:** Monitor the result of `sink.send()`. If an error occurs (e.g., `Disconnected`), log the error, potentially increment failure counters in `SubscriptionStats`, and implement logic for potential subscription cleanup (see Task 7.1/7.2).
    7. **Update Statistics:** Increment relevant counters in `SubscriptionStats` for the `subscription_id` (e.g., `events_processed`, `events_dropped_buffer` on sink errors).
* **Task:** Implement graceful shutdown handling for the background task (e.g., using a `tokio::sync::watch` channel or similar signal to break the loop).
* **Task:** Add detailed `tracing` logs for the task lifecycle, event reception, parsing success/failure, filtering decisions, sink errors, and shutdown.

#### 3.6.3.5 Document `manager` module

* **Task:** Write comprehensive documentation for the `SubscriptionManager` struct, explaining its role, the dual-registry mechanism, and its interaction with the background task.
* **Task:** Document the `SystemEvent` enum, the `try_parse_relocated_rues_event` function, and the `ParseError` type.
* **Task:** Clearly document the thread-safety model and locking strategy.
* **Task:** Provide usage examples relevant to the `jsonrpsee` context.

---

## Variant B: Hard Cutover / Use `NodeBroadcastEvent`

**Assumption:** The legacy HTTP server will be completely removed when the new JSON-RPC server is deployed. Event producers can be modified to emit a new, structured event type directly, eliminating the need for the legacy `RuesEvent` format entirely.

### 3.6.3. Variant B: Hard Cutover / Use `NodeBroadcastEvent`

#### 3.6.3.0 Define `NodeBroadcastEvent` & Refactor Producers (Prerequisite)

* **Task:** Define a new `NodeBroadcastEvent` enum in a stable, shared location (e.g., `node-data` or `rusk-events-common`). This enum should have variants corresponding to logical event types, holding the relevant *structured* Rust data directly (e.g., `BlockHeader`, `ContractTxEvent`). Ensure it derives `Clone` and `Debug`.
* **Task:** **Refactor Event Producers:** Modify all event producer modules (`Rusk`, `ChainEventStreamer`, etc.):
  * Remove all dependencies on the legacy `RuesEvent` (from `http::event` or the relocated copy).
  * Implement simple `impl From<RawNodeStructure> for NodeBroadcastEvent` for relevant raw types (e.g., `BlockHeader`, `ContractTxEvent`). These implementations should directly wrap the data into the enum variant without any serialization. Remember to handle cloning if the raw structure is needed elsewhere.
  * Update the event sending logic to create and send `NodeBroadcastEvent` instead of `RuesEvent`.
* **Task:** Update the broadcast channel creation in `RuskNodeBuilder` to use the new type: `broadcast::channel::<NodeBroadcastEvent>()`. Ensure the sender/receiver are passed correctly.

#### 3.6.3.1 Create `SubscriptionManager` skeleton (Variant B)

* **Task:** Define the basic `SubscriptionManager` struct in `rusk::lib::jsonrpc::infrastructure::subscription::manager.rs`.
* **Task:** Include the dual registry fields using appropriate collections (e.g., `DashMap` or `RwLock<HashMap>`):
  * `topic_subscriptions`: Map from internal `Topic` to a collection of `SubscriptionId` -> `SubscriptionSink`.
  * `session_subscriptions`: Map from `SessionId` to a collection of associated `(Topic, SubscriptionId)` tuples.
* **Task:** Add filter storage: Map from `SubscriptionId` to `Box<dyn Filter>`.
* **Task:** Add subscription statistics tracking: Map from `SubscriptionId` to `SubscriptionStats`.
* **Task:** Implement the `SubscriptionManager::new` constructor:
  * Accepts `broadcast::Receiver<NodeBroadcastEvent>` (the receiver for the channel carrying the new structured event type).
  * Spawns the background event processing task (detailed in 4.4).

#### 3.6.3.2 Implement thread safety (Variant B)

* **Task:** Wrap all shared internal collections (`topic_subscriptions`, `session_subscriptions`, `filters`, `subscription_stats`) in appropriate concurrent primitives (e.g., `Arc<DashMap<...>>` or `Arc<RwLock<HashMap<...>>>`) for safe multi-threaded access.
* **Task:** Analyze and document the locking strategy to prevent deadlocks.
* **Task:** Ensure `SubscriptionManager` itself derives `Clone` and is `Send + Sync`.
* **Task:** Document the thread-safety guarantees and expected concurrent usage patterns.

#### 3.6.3.3 Map `NodeBroadcastEvent` to Internal `Topic`

* **Task:** Implement a mechanism (e.g., a function `get_topic_for_event(event: &NodeBroadcastEvent) -> Option<Topic>`) within the subscription module to map the received `NodeBroadcastEvent` variant to the corresponding internal `Topic` enum used for routing subscriptions. This mapping should be straightforward based on the enum variant.

#### 3.6.3.4 Implement background task for event processing (Variant B)

* **Task:** Create a dedicated Tokio task spawned within the `SubscriptionManager::new` constructor.
* **Task:** **Core Event Loop:**
  * The task continuously loops, receiving `NodeBroadcastEvent` messages from the broadcast receiver passed during construction.
  * Handle potential `broadcast::error::RecvError::Lagged` errors gracefully.
* **Task:** **Event Processing Steps:** Upon receiving a `NodeBroadcastEvent`:
    1. **Determine Topic:** Call `get_topic_for_event` (from Task 4.3) to get the internal `Topic`. If no topic maps (shouldn't happen with exhaustive matching), log an error and skip.
    2. **Identify Subscribers:** Acquire necessary locks and look up all `(SubscriptionId, SubscriptionSink)` pairs associated with the `Topic` in the `topic_subscriptions` registry.
    3. **Apply Filters:** For each potential subscriber (`subscription_id`, `sink`):
        * Look up the associated `Filter` in the `filters` registry using `subscription_id`.
        * If a filter exists, call its `matches(&node_broadcast_event_data)` method, passing a reference to the *structured data* held directly within the received `NodeBroadcastEvent` variant (e.g., `&block_header` if the event is `NodeBroadcastEvent::BlockFinalized(block_header)`). Use `&dyn Any` for the filter to downcast.
        * Proceed to the next step only if there is no filter OR the filter's `matches` method returns `true`.
    4. **Format JSON Payload:** Using the structured data within the `NodeBroadcastEvent`, construct the JSON-RPC notification payload expected by the client for this subscription type. This involves serializing the structured data into the appropriate JSON format according to the API specification.
    5. **Send to Sink:** Create the `jsonrpsee::SubscriptionMessage` containing the formatted JSON payload and attempt to send it asynchronously using `SubscriptionSink::send()`.
    6. **Handle Sink Errors:** Monitor the result of `sink.send()`. If an error occurs (e.g., `Disconnected`), log the error, potentially increment failure counters in `SubscriptionStats`, and implement logic for potential subscription cleanup (see Task 7.1/7.2).
    7. **Update Statistics:** Increment relevant counters in `SubscriptionStats` for the `subscription_id`.
* **Task:** Implement graceful shutdown handling for the background task.
* **Task:** Add detailed `tracing` logs for the task lifecycle, event reception, topic mapping, filtering decisions, sink errors, and shutdown.

#### 3.6.3.5 Document `manager` module (Variant B)

* **Task:** Write comprehensive documentation for the `SubscriptionManager` struct, explaining its role and the dual-registry mechanism.
* **Task:** Document the expected `NodeBroadcastEvent` type (likely linking to its definition in the shared crate).
* **Task:** Clearly document the thread-safety model and locking strategy.
* **Task:** Provide usage examples relevant to the `jsonrpsee` context.

---

## Conclusion: Comparison of Variants

| Feature                    | Variant A (Dual Server / Parse `RuesEvent`)                                                                                       | Variant B (Hard Cutover / `NodeBroadcastEvent`)                                                                          |
| :------------------------- | :-------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------- |
| **Producer Changes**       | **Minimal:** Only requires updating `use` statements for relocated `RuesEvent`.                                                   | **Significant:** Requires removing `RuesEvent` logic, adding `NodeBroadcastEvent` logic.                                 |
| **`jsonrpc` Server Logic** | **More Complex:** Needs `RelocatedRuesEvent` definition, `SystemEvent` definition, and `try_parse...` function (deserialization). | **Simpler:** Receives structured `NodeBroadcastEvent` directly, no parsing/deserialization needed for the event itself.  |
| **Efficiency**             | Performs one **deserialization** per event in `jsonrpc`. Producers still perform **serialization** for legacy `RuesEvent`.        | **Optimal:** No event serialization/deserialization between producer and `jsonrpc`. Data flows as structs.               |
| **Transition Complexity**  | Lower risk *during* transition, producers largely untouched. Allows simultaneous server operation.                                | Higher risk *during* transition due to core producer refactoring. Does not easily support simultaneous server operation. |
| **Final Architecture**     | Leaves adaptation layer (`try_parse...`) in `jsonrpc`. Requires later refactoring to remove `RelocatedRuesEvent` fully.           | **Cleaner:** Results directly in the desired state with structured events and no legacy format.                          |
| **Code Duplication/Debt**  | Introduces `RelocatedRuesEvent` copy. Parsing logic depends on legacy format knowledge.                                           | Eliminates legacy format dependency entirely.                                                                            |
| **Scope Alignment**        | Better fits a scope focused *only* on replacing the server interface while preserving producer behavior *initially*.              | Better fits a scope where removing the legacy server *includes* refactoring event production for efficiency.             |

**Recommendation:**

* **If simultaneous operation is required or minimizing disruption to producers *during the initial server replacement* is paramount:** Choose **Variant A**. It isolates changes mostly to the new `jsonrpc` module and defers the larger producer refactoring. Accept the technical debt of copying/parsing `RuesEvent` temporarily.
* **If a hard cutover is acceptable and refactoring producers as part of the server replacement is feasible:** Choose **Variant B**. It's more work upfront involving core components but leads directly to the cleaner, more efficient final architecture without intermediate steps or temporary code duplication/parsing logic.

Variant B is architecturally superior in the long run, but Variant A provides a more phased approach if required by project constraints or risk tolerance during the transition.
