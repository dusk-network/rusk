//! Event types and operations for the RUES protocol.
//!
//! This module provides core event types and operations that represent the RUES
//! protocol's event system. Key components include:
//!
//! - [`Event`] - Core event type representing RUES protocol events
//! - [`Version`] - Protocol version following RUES binary format specification
//! - [`EventOperation`] - Supported event operations
//!   (Subscribe/Unsubscribe/Dispatch)
//! - [`EventBuilder`] - Builder for constructing valid events
//!
//! # Event Structure
//!
//! RUES events consist of:
//! - Path (optional for Connect): `/on/\[target\]/\[topic\]`
//! - Headers (optional for Connect)
//! - Payload (optional)
//! - Operation type
//! - Session ID (required for Subscribe/Unsubscribe/Message)
//! - Protocol version
//!
//! # Examples
//!
//! Creating an event with all components:
//! ```rust
//! use rusk::http::domain::{Event, EventOperation, RuesPath, Target, RuesValue};
//! # use rusk::http::domain::testing;
//! use serde_json::json;
//!
//! // Create path and payload
//! let contract_id = testing::create_test_contract_id();
//! let topic = testing::create_test_topic("deploy");
//! let path = RuesPath::new_modern(
//!     Target::Contracts,
//!     Some(contract_id),
//!     topic
//! );
//! let payload = RuesValue::Json(json!({"status": "success"}));
//!
//! // Build event
//! let event = Event::builder()
//!     .path(path)
//!     .payload(payload)
//!     .operation(EventOperation::Dispatch)
//!     .session_id(testing::create_test_session_id())
//!     .version(testing::create_release_version())
//!     .build()
//!     .unwrap();
//!
//! assert!(!event.is_legacy());
//! assert!(!event.requires_session() || event.has_session());
//! ```
//!
//! Working with versions:
//! ```rust
//! # use rusk::http::domain::testing;
//!
//! // Release version
//! let release = testing::create_release_version();
//! assert!(!release.is_pre_release());
//!
//! // Pre-release version
//! let pre_release = testing::create_pre_release_version();
//! assert!(pre_release.is_pre_release());
//!
//! // Version compatibility
//! assert!(!pre_release.is_compatible_with(&release));
//! ```
//!
//! # Thread Safety
//!
//! All types in this module are thread-safe and can be shared between threads:
//! - `Event` implements `Send` + `Sync`
//! - `Version` is `Copy` and thread-safe
//! - All builders use interior mutability
//! - Event metadata is immutable after construction
//!
//! # Binary Format
//!
//! Events can be serialized to/from the RUES binary format. The format
//! includes:
//! - Protocol version (8 bytes)
//! - Event metadata
//! - Headers (if present)
//! - Payload (if present)
//!
//! For details on the binary format, see the RUES binary format specification.

use std::{cmp::Ordering, fmt};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::http::domain::{
    BlockHash, ContractId, DomainError, RuesHeaders, RuesPath, RuesValue,
    SerDeError, SessionId, Target, TargetIdentifier, TransactionHash,
    ValidationError,
};

/// Core event type for the RUES protocol.
///
/// An event represents a complete RUES message that can be:
/// - Subscribed to (GET)
/// - Unsubscribed from (DELETE)
/// - Dispatched (POST)
/// - Sent over WebSocket (Connect/Message)
///
/// # Structure
///
/// Each event consists of:
/// - Path (optional for Connect): `/on/[target]/[topic]`
/// - Headers (optional for Connect): Content-Type, Content-Length, etc.
/// - Payload (optional): Binary data, JSON, text, or GraphQL
/// - Operation type: Connect/Subscribe/Unsubscribe/Dispatch/Message
/// - Session ID (required for Subscribe/Unsubscribe/Message)
/// - Protocol version
/// - Creation timestamp
///
/// # Construction
///
/// Events should be created using the builder pattern:
///
/// ```rust
/// use rusk::http::domain::{Event, EventOperation, RuesPath, Target, RuesValue};
/// use serde_json::json;
/// # use rusk::http::domain::testing;
///
/// let topic = testing::create_test_topic("info");
/// let path = RuesPath::new_modern(Target::Node, None, topic);
///
/// let event = Event::builder()
///     .path(path)
///     .payload(RuesValue::Json(json!({"status": "ready"})))
///     .operation(EventOperation::Dispatch)
///     .version(testing::create_release_version())
///     .build()
///     .unwrap();
///
/// assert!(!event.is_legacy());
/// assert_eq!(event.operation(), EventOperation::Dispatch);
/// ```
///
/// # Legacy Support
///
/// The event system supports both modern and legacy targets:
///
/// ```rust
/// use rusk::http::domain::{Event, EventOperation, RuesPath, Target, LegacyTarget};
/// # use rusk::http::domain::testing;
///
/// // Modern target
/// let topic = testing::create_test_topic("info");
/// let modern_path = RuesPath::new_modern(Target::Node, None, topic);
///
/// let modern_event = Event::builder()
///     .path(modern_path)
///     .operation(EventOperation::Subscribe)
///     .version(testing::create_release_version())
///     .build()
///     .unwrap();
/// assert!(!modern_event.is_legacy());
///
/// // Legacy target
/// let topic = testing::create_test_topic("status");
/// let legacy_path = RuesPath::new_legacy(
///     LegacyTarget::Host("system".into()),
///     topic
/// );
///
/// let legacy_event = Event::builder()
///     .path(legacy_path)
///     .operation(EventOperation::Subscribe)
///     .version(testing::create_release_version())
///     .build()
///     .unwrap();
///
/// assert!(legacy_event.is_legacy());
/// ```
///
/// # Thread Safety
///
/// `Event` is thread-safe and can be shared between threads:
/// - All fields are immutable after construction
/// - All types implement `Send` + `Sync`
/// - Event metadata is thread-safe
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Event path (optional for Connect)
    path: Option<RuesPath>,
    /// Event payload (optional for some operations)
    payload: Option<RuesValue>,
    /// Event headers (optional for Connect)
    headers: Option<RuesHeaders>,
    /// Event operation type
    operation: EventOperation,
    /// Session ID (required for Subscribe/Unsubscribe/Message)
    session_id: Option<SessionId>,
    /// Protocol version
    version: Version,
    /// Event timestamp
    timestamp: DateTime<Utc>,
}

impl Event {
    /// Creates a new event with the given components.
    ///
    /// This constructor is for internal use only. For public API, use
    /// [`Event::builder()`].
    ///
    /// # Arguments
    ///
    /// * `path` - Event path (optional for Connect)
    /// * `payload` - Event payload (optional)
    /// * `headers` - Event headers (optional for Connect)
    /// * `operation` - Event operation type
    /// * `session_id` - Session ID (required for Subscribe/Unsubscribe/Message)
    /// * `version` - Protocol version
    pub(crate) fn new(
        path: Option<RuesPath>,
        payload: Option<RuesValue>,
        headers: Option<RuesHeaders>,
        operation: EventOperation,
        session_id: Option<SessionId>,
        version: Version,
    ) -> Self {
        Self {
            path,
            payload,
            headers,
            operation,
            session_id,
            version,
            timestamp: Utc::now(),
        }
    }

    /// Creates a new event builder.
    ///
    /// This is the recommended way to construct `Event` instances as it ensures
    /// all required fields are set and validates the event structure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target, RuesValue};
    /// # use rusk::http::domain::testing;
    /// use serde_json::json;
    ///
    /// let contract_id = testing::create_test_contract_id();
    /// let topic = testing::create_test_topic("deploy");
    /// let path = RuesPath::new_modern(
    ///     Target::Contracts,
    ///     Some(contract_id),
    ///     topic
    /// );
    ///
    /// let event = Event::builder()
    ///     .path(path)
    ///     .payload(RuesValue::Json(json!({"status": "ready"})))
    ///     .operation(EventOperation::Dispatch)
    ///     .session_id(testing::create_test_session_id())
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(event.has_session());
    /// assert_eq!(event.operation(), EventOperation::Dispatch);
    /// ```
    pub fn builder() -> EventBuilder {
        EventBuilder::default()
    }

    // Core accessors

    /// Returns the event operation type.
    ///
    /// This method indicates how this event should be processed
    /// (Subscribe/Unsubscribe/Dispatch/etc.).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target};
    /// # use rusk::http::domain::testing;
    ///
    /// let event = Event::builder()
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.operation(), EventOperation::Subscribe);
    /// ```
    pub fn operation(&self) -> EventOperation {
        self.operation
    }

    /// Returns the protocol version of this event.
    ///
    /// The version follows RUES binary format specification and is used for
    /// compatibility checking.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation};
    /// # use rusk::http::domain::testing;
    ///
    /// let version = testing::create_release_version();
    /// let event = Event::builder()
    ///     .version(version)
    ///     .operation(EventOperation::Message)
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.version(), version);
    /// ```
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns the timestamp when this event was created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation};
    /// use chrono::{DateTime, Utc};
    /// # use rusk::http::domain::testing;
    ///
    /// let event = Event::builder()
    ///     .operation(EventOperation::Message)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// let now: DateTime<Utc> = Utc::now();
    /// assert!(event.timestamp() <= now);
    /// ```
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    // Path-related methods

    /// Returns a reference to the event path if present.
    ///
    /// The path follows the format `/on/[target]/[topic]` and is required for
    /// all operations except `Connect`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target};
    /// # use rusk::http::domain::testing;
    ///
    /// let topic = testing::create_test_topic("info");
    /// let path = RuesPath::new_modern(Target::Node, None, topic);
    /// let event = Event::builder()
    ///     .path(path.clone())
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.path(), Some(&path));
    /// ```
    pub fn path(&self) -> Option<&RuesPath> {
        self.path.as_ref()
    }

    /// Returns the modern target type if available.
    ///
    /// Returns `None` if:
    /// - Path is not set (Connect operation)
    /// - Path uses legacy target
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target};
    /// # use rusk::http::domain::testing;
    ///
    /// let topic = testing::create_test_topic("info");
    /// let path = RuesPath::new_modern(Target::Node, None, topic);
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.target(), Some(Target::Node));
    /// ```
    pub fn target(&self) -> Option<Target> {
        self.path.as_ref().and_then(|p| p.target().modern_target())
    }

    /// Returns a reference to the target identifier if present.
    ///
    /// Returns `None` if:
    /// - Path is not set
    /// - Target doesn't have an identifier
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target};
    /// # use rusk::http::domain::testing;
    ///
    /// // Path with target ID
    /// let block_id = testing::create_test_block_hash();
    /// let topic = testing::create_test_topic("accepted");
    /// let path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     Some(block_id.clone()),
    ///     topic
    /// );
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.target_id(), Some(&block_id));
    ///
    /// // Path without target ID
    /// let topic = testing::create_test_topic("info");
    /// let path = RuesPath::new_modern(Target::Node, None, topic);
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.target_id(), None);
    /// ```
    pub fn target_id(&self) -> Option<&TargetIdentifier> {
        self.path.as_ref().and_then(|p| p.target().id())
    }

    /// Returns the event topic if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target};
    /// # use rusk::http::domain::testing;
    ///
    /// let topic = testing::create_test_topic("info");
    /// let path = RuesPath::new_modern(Target::Node, None, topic);
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.topic(), Some("info"));
    /// ```
    pub fn topic(&self) -> Option<&str> {
        self.path.as_ref().map(|p| p.topic())
    }

    // Payload-related methods

    /// Returns a reference to the event payload if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesValue};
    /// # use rusk::http::domain::testing;
    /// use serde_json::json;
    ///
    /// let event = Event::builder()
    ///     .payload(RuesValue::Json(json!({"status": "ok"})))
    ///     .operation(EventOperation::Message)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(event.payload().is_some());
    /// ```
    pub fn payload(&self) -> Option<&RuesValue> {
        self.payload.as_ref()
    }

    // Headers-related methods

    /// Returns a reference to the event headers if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesHeaders};
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/node/info")
    ///     .content_type("application/json")
    ///     .content_length(0)
    ///     .build()
    ///     .unwrap();
    ///
    /// let event = Event::builder()
    ///     .headers(headers.clone())
    ///     .operation(EventOperation::Message)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.headers(), Some(&headers));
    /// ```
    pub fn headers(&self) -> Option<&RuesHeaders> {
        self.headers.as_ref()
    }

    /// Returns the content type of the event payload if headers are present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesHeaders};
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/node/info")
    ///     .content_type("application/json")
    ///     .content_length(0)
    ///     .build()
    ///     .unwrap();
    ///
    /// let event = Event::builder()
    ///     .headers(headers)
    ///     .operation(EventOperation::Message)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.content_type(), Some("application/json"));
    /// ```
    pub fn content_type(&self) -> Option<&str> {
        self.headers.as_ref().map(|h| h.content_type())
    }

    /// Returns the content length if headers are present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesHeaders};
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/node/info")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .build()
    ///     .unwrap();
    ///
    /// let event = Event::builder()
    ///     .headers(headers)
    ///     .operation(EventOperation::Message)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.content_length(), Some(42));
    /// ```
    pub fn content_length(&self) -> Option<usize> {
        self.headers.as_ref().map(|h| h.content_length())
    }

    // Session-related methods

    /// Returns a reference to the session ID if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation};
    /// # use rusk::http::domain::testing;
    ///
    /// let session_id = testing::create_test_session_id();
    ///
    /// let event = Event::builder()
    ///     .session_id(session_id.clone())
    ///     .operation(EventOperation::Message)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(event.session_id(), Some(&session_id));
    /// ```
    pub fn session_id(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }

    // Operation-specific checks

    /// Returns true if this event requires WebSocket connection.
    ///
    /// Only Connect and Message operations require WebSocket.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation};
    /// # use rusk::http::domain::testing;
    ///
    /// let connect = Event::builder()
    ///     .operation(EventOperation::Connect)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    /// assert!(connect.is_websocket());
    ///
    /// let dispatch = Event::builder()
    ///     .operation(EventOperation::Dispatch)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(!dispatch.is_websocket());
    /// ```
    pub fn is_websocket(&self) -> bool {
        self.operation.is_websocket()
    }

    /// Returns true if this event requires a session ID.
    ///
    /// Subscribe, Unsubscribe and Message operations require session ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation};
    /// # use rusk::http::domain::testing;
    ///
    /// let subscribe = Event::builder()
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    /// assert!(subscribe.requires_session());
    ///
    /// let dispatch = Event::builder()
    ///     .operation(EventOperation::Dispatch)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(!dispatch.requires_session());
    /// ```
    pub fn requires_session(&self) -> bool {
        self.operation.requires_session()
    }

    /// Returns true if this event has a session ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation};
    /// # use rusk::http::domain::testing;
    ///
    /// let event = Event::builder()
    ///     .session_id(testing::create_test_session_id())
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(event.has_session());
    /// ```
    pub fn has_session(&self) -> bool {
        self.session_id.is_some()
    }

    // Target-specific methods

    /// Returns the block hash if this event targets a block.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{
    ///     Event, EventOperation, RuesPath, Target, TargetIdentifier,
    /// };
    /// # use rusk::http::domain::testing;
    ///
    /// let block_id = testing::create_test_block_hash();
    /// let topic = testing::create_test_topic("accepted");
    /// let path = RuesPath::new_modern(
    ///     Target::Blocks,
    ///     Some(block_id.clone()),
    ///     topic
    /// );
    ///
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// match block_id {
    ///     TargetIdentifier::Block(expected_hash) => {
    ///         assert_eq!(event.block_hash(), Some(&expected_hash));
    ///     }
    ///     _ => panic!("Expected Block variant"),
    /// }
    /// ```
    pub fn block_hash(&self) -> Option<&BlockHash> {
        self.target_id().and_then(|id| match id {
            TargetIdentifier::Block(hash) => Some(hash),
            _ => None,
        })
    }

    /// Returns the contract ID if this event targets a contract.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target, TargetIdentifier};
    /// # use rusk::http::domain::testing;
    ///
    /// let contract_id = testing::create_test_contract_id();
    /// let topic = testing::create_test_topic("deploy");
    /// let path = RuesPath::new_modern(
    ///     Target::Contracts,
    ///     Some(contract_id.clone()),
    ///     topic
    /// );
    ///
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// match contract_id {
    ///     TargetIdentifier::Contract(expected_id) => {
    ///         assert_eq!(event.contract_id(), Some(&expected_id));
    ///     }
    ///     _ => panic!("Expected Contract variant"),
    /// }
    /// ```
    pub fn transaction_hash(&self) -> Option<&TransactionHash> {
        self.target_id().and_then(|id| match id {
            TargetIdentifier::Transaction(hash) => Some(hash),
            _ => None,
        })
    }

    /// Returns the contract ID if this event targets a contract.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target, TargetIdentifier};
    /// # use rusk::http::domain::testing;
    ///
    /// let contract_id = testing::create_test_contract_id();
    /// let topic = testing::create_test_topic("deploy");
    /// let path = RuesPath::new_modern(
    ///     Target::Contracts,
    ///     Some(contract_id.clone()),
    ///     topic
    /// );
    ///
    /// let event = Event::builder()
    ///     .path(path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// match contract_id {
    ///     TargetIdentifier::Contract(expected_id) => {
    ///         assert_eq!(event.contract_id(), Some(&expected_id));
    ///     }
    ///     _ => panic!("Expected Contract variant"),
    /// }
    /// ```
    pub fn contract_id(&self) -> Option<&ContractId> {
        self.target_id().and_then(|id| match id {
            TargetIdentifier::Contract(id) => Some(id),
            _ => None,
        })
    }

    // Legacy support

    /// Returns true if this event uses a legacy target.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{Event, EventOperation, RuesPath, Target, LegacyTarget};
    /// # use rusk::http::domain::testing;
    ///
    /// // Modern path
    /// let topic = testing::create_test_topic("info");
    /// let modern_path = RuesPath::new_modern(Target::Node, None, topic);
    /// let event = Event::builder()
    ///     .path(modern_path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    /// assert!(!event.is_legacy());
    ///
    /// // Legacy path
    /// let topic = testing::create_test_topic("status");
    /// let legacy_path = RuesPath::new_legacy(
    ///     LegacyTarget::Host("system".into()),
    ///     topic
    /// );
    /// let event = Event::builder()
    ///     .path(legacy_path)
    ///     .operation(EventOperation::Subscribe)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    /// assert!(event.is_legacy());
    /// ```
    pub fn is_legacy(&self) -> bool {
        self.path.as_ref().map_or(false, |p| p.is_legacy())
    }
}

/// Protocol version following RUES binary format specification
/// (SemVer 2.0.0).
///
/// 8 bytes version field with major, minor, patch, and pre-release.
///
/// The version consists of:
/// - Major version (required)
/// - Minor version (required)
/// - Patch version (required)
/// - Pre-release version (optional)
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::Version;
/// # use rusk::http::domain::testing;
///
/// let version = testing::create_release_version();
/// assert_eq!(version.major(), 1);
/// assert_eq!(version.minor(), 0);
/// assert_eq!(version.patch(), 0);
/// assert!(!version.is_pre_release());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    // According to RUES-binary-format.md:
    // 8 bytes version field with major, minor, patch, and pre-release
    major: u8,
    minor: u8,
    patch: u8,
    pre_release: Option<u8>,
}

impl Version {
    pub(crate) fn new(
        major: u8,
        minor: u8,
        patch: u8,
        pre_release: Option<u8>,
    ) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release,
        }
    }

    /// Returns the major version number.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::Version;
    /// # use rusk::http::domain::testing;
    ///
    /// let version = testing::create_release_version();
    /// assert_eq!(version.major(), 1);
    /// ```
    pub fn major(&self) -> u8 {
        self.major
    }

    /// Returns the minor version number.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::Version;
    /// # use rusk::http::domain::testing;
    ///
    /// let version = testing::create_release_version();
    /// assert_eq!(version.minor(), 0);
    /// ```
    pub fn minor(&self) -> u8 {
        self.minor
    }

    /// Returns the patch version number.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::Version;
    /// # use rusk::http::domain::testing;
    ///
    /// let version = testing::create_release_version();
    /// assert_eq!(version.patch(), 0);
    /// ```
    pub fn patch(&self) -> u8 {
        self.patch
    }

    /// Returns the pre-release version number if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::Version;
    /// # use rusk::http::domain::testing;
    ///
    /// let version = testing::create_pre_release_version();
    /// assert_eq!(version.pre_release(), Some(1));
    /// ```
    pub fn pre_release(&self) -> Option<u8> {
        self.pre_release
    }

    /// Returns true if this is a pre-release version
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::Version;
    /// # use rusk::http::domain::testing;
    ///
    /// let release = testing::create_release_version();
    /// assert!(!release.is_pre_release());
    ///
    /// let pre_release = testing::create_pre_release_version();
    /// assert!(pre_release.is_pre_release());
    /// ```
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    /// Checks if this version is compatible with another version according to
    /// SemVer 2.0.0.
    ///
    /// According to SemVer:
    /// - Major version differences are incompatible
    /// - Minor and patch differences are compatible if major version matches
    /// - Pre-release versions are only compatible with the same pre-release
    ///   version
    ///
    /// # Examples
    /// ```
    /// # use rusk::http::domain::testing;
    ///
    /// // Compatible: same major version
    /// assert!(testing::create_test_version(1, 1, 0, None)
    ///     .is_compatible_with(&testing::create_test_version(1, 0, 0, None)));
    ///
    /// // Incompatible: different major versions
    /// assert!(!testing::create_test_version(2, 0, 0, None)
    ///     .is_compatible_with(&testing::create_test_version(1, 0, 0, None)));
    ///
    /// // Pre-release vs release
    /// let release = testing::create_release_version();
    /// let pre_release = testing::create_pre_release_version();
    /// assert!(!pre_release.is_compatible_with(&release));
    /// ```
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        if self.major != other.major {
            // Different major versions are never compatible
            return false;
        }

        match (self.pre_release, other.pre_release) {
            // Both are normal releases - compatible if major versions match
            (None, None) => true,
            // Pre-release is only compatible with the exact same pre-release
            (Some(a), Some(b)) => {
                self.minor == other.minor && self.patch == other.patch && a == b
            }
            // Pre-release is not compatible with normal release
            _ => false,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(pre) = self.pre_release() {
            write!(
                f,
                "{}.{}.{}-{}",
                self.major(),
                self.minor(),
                self.patch(),
                pre
            )
        } else {
            write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // First compare major version
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return Some(ord),
        }

        // Then compare minor version
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return Some(ord),
        }

        // Then compare patch version
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return Some(ord),
        }

        // Only if all version numbers are equal, consider pre-release
        match (self.pre_release, other.pre_release) {
            (None, None) => Some(Ordering::Equal),
            (Some(_), None) => Some(Ordering::Less), // pre-release < release
            (None, Some(_)) => Some(Ordering::Greater), // release > pre-release
            (Some(a), Some(b)) => Some(a.cmp(&b)),   /* compare pre-release
                                                       * numbers */
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer, // Added trait bound
    {
        let version_str = if let Some(pre) = self.pre_release() {
            format!(
                "{}.{}.{}-{}",
                self.major(),
                self.minor(),
                self.patch(),
                pre
            )
        } else {
            format!("{}.{}.{}", self.major(), self.minor(), self.patch())
        };
        serializer.serialize_str(&version_str)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version_str = String::deserialize(deserializer)?;

        // Parse version string (e.g., "1.2.3" or "1.2.3-1")
        let parts: Vec<&str> = version_str.split('-').collect();
        let version_nums: Vec<&str> = parts[0].split('.').collect();

        if version_nums.len() != 3 {
            return Err(serde::de::Error::custom(
                "Version must be in format 'major.minor.patch[-pre]'",
            ));
        }

        let major = version_nums[0]
            .parse::<u8>()
            .map_err(|_| serde::de::Error::custom("Invalid major version"))?;
        let minor = version_nums[1]
            .parse::<u8>()
            .map_err(|_| serde::de::Error::custom("Invalid minor version"))?;
        let patch = version_nums[2]
            .parse::<u8>()
            .map_err(|_| serde::de::Error::custom("Invalid patch version"))?;

        let pre_release = if parts.len() > 1 {
            Some(parts[1].parse::<u8>().map_err(|_| {
                serde::de::Error::custom("Invalid pre-release version")
            })?)
        } else {
            None
        };

        Ok(Version::new(major, minor, patch, pre_release))
    }
}

/// Supported operations in the RUES protocol.
///
/// This enum represents all possible operations that can be performed with RUES
/// events:
/// - `Connect`: WebSocket connection establishment
/// - `Subscribe`: Subscribe to events (GET)
/// - `Unsubscribe`: Unsubscribe from events (DELETE)
/// - `Dispatch`: Dispatch event (POST)
/// - `Message`: WebSocket message exchange
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::EventOperation;
///
/// // WebSocket operations
/// assert!(EventOperation::Connect.is_websocket());
/// assert!(EventOperation::Message.is_websocket());
/// assert!(!EventOperation::Dispatch.is_websocket());
///
/// // Session requirements
/// assert!(EventOperation::Subscribe.requires_session());
/// assert!(EventOperation::Unsubscribe.requires_session());
/// assert!(!EventOperation::Dispatch.requires_session());
///
/// // HTTP methods
/// assert_eq!(EventOperation::Subscribe.http_method(), Some("GET"));
/// assert_eq!(EventOperation::Unsubscribe.http_method(), Some("DELETE"));
/// assert_eq!(EventOperation::Dispatch.http_method(), Some("POST"));
/// assert_eq!(EventOperation::Connect.http_method(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventOperation {
    /// WebSocket connection establishment
    Connect,
    /// Subscribe to events (GET)
    Subscribe,
    /// Unsubscribe from events (DELETE)
    Unsubscribe,
    /// Dispatch event (POST)
    Dispatch,
    /// WebSocket message exchange
    Message,
}

impl EventOperation {
    /// Returns true if this operation requires a session ID.
    ///
    /// Subscribe, Unsubscribe and Message operations require a session ID.
    pub fn requires_session(&self) -> bool {
        matches!(self, Self::Subscribe | Self::Unsubscribe | Self::Message)
    }

    /// Returns the HTTP method for this operation, if applicable.
    ///
    /// Returns `None` for WebSocket-only operations (Connect and Message).
    pub fn http_method(&self) -> Option<&'static str> {
        match self {
            Self::Subscribe => Some("GET"),
            Self::Unsubscribe => Some("DELETE"),
            Self::Dispatch => Some("POST"),
            Self::Connect | Self::Message => None,
        }
    }

    /// Returns true if this operation requires a WebSocket connection.
    ///
    /// Only Connect and Message operations require WebSocket.
    pub fn is_websocket(&self) -> bool {
        matches!(self, Self::Connect | Self::Message)
    }

    /// Returns true if this operation requires a path.
    ///
    /// All operations except Connect require a path.
    pub fn requires_path(&self) -> bool {
        !matches!(self, Self::Connect)
    }

    /// Returns true if this operation requires headers.
    ///
    /// All operations except Connect require headers.
    pub fn requires_headers(&self) -> bool {
        !matches!(self, Self::Connect)
    }
}

/// Event builder to ensure proper construction
#[derive(Debug, Default)]
pub struct EventBuilder {
    path: Option<RuesPath>,
    payload: Option<RuesValue>,
    headers: Option<RuesHeaders>,
    operation: Option<EventOperation>,
    session_id: Option<SessionId>,
    version: Option<Version>,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the path for this event.
    pub fn path(mut self, path: RuesPath) -> Self {
        self.path = Some(path);
        self
    }

    /// Set the payload for this event.
    pub fn payload(mut self, payload: RuesValue) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Set the headers for this event.
    pub fn headers(mut self, headers: RuesHeaders) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Set the operation for this event.
    pub fn operation(mut self, operation: EventOperation) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Set the session ID for this event.
    pub fn session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set the protocol version for this event.
    pub fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the content length in headers if both headers and payload are
    /// present and returns the built event.
    pub fn build(mut self) -> Result<Event, DomainError> {
        let headers = self.headers.as_mut();

        // If we have both headers and payload, update content length
        if let (Some(h), Some(payload)) = (headers, &self.payload) {
            h.set_content_length(payload.byte_len()?);
        }

        Ok(Event::new(
            self.path,
            self.payload,
            self.headers,
            self.operation.unwrap_or(EventOperation::Message),
            self.session_id,
            self.version.unwrap_or_else(|| Version::new(0, 1, 0, None)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use serde_json::{from_value, json, to_value};

    use super::*;
    use crate::http::domain::testing;

    #[test]
    fn test_version_compatibility() {
        // Same major version - compatible
        assert!(testing::create_test_version(1, 0, 0, None)
            .is_compatible_with(&testing::create_test_version(1, 0, 0, None)));

        // Different major version - incompatible
        assert!(!testing::create_test_version(2, 0, 0, None)
            .is_compatible_with(&testing::create_test_version(1, 0, 0, None)));

        // Pre-release versions
        let v1_pre1 = testing::create_test_version(1, 0, 0, Some(1));
        let v1_pre2 = testing::create_test_version(1, 0, 0, Some(2));
        let v1_release = testing::create_release_version();

        assert!(v1_pre1.is_compatible_with(&v1_pre1));
        assert!(!v1_pre1.is_compatible_with(&v1_pre2));
        assert!(!v1_pre1.is_compatible_with(&v1_release));
    }

    #[test]
    fn test_version_serialization() {
        let test_cases = vec![
            (testing::create_release_version(), "1.0.0"),
            (testing::create_test_version(2, 3, 4, None), "2.3.4"),
            (testing::create_test_version(1, 2, 3, Some(4)), "1.2.3-4"),
        ];

        for (version, expected_str) in test_cases {
            let value = to_value(&version).unwrap();
            assert_eq!(value, json!(expected_str));

            let deserialized: Version = from_value(value).unwrap();
            assert_eq!(version, deserialized);
        }
    }

    #[test]
    fn test_invalid_version() {
        let invalid_cases = vec![
            "1.0",     // Missing patch
            "1.0.0.0", // Too many components
            "1.0.0-",  // Empty pre-release
            "x.0.0",   // Invalid major
            "1.y.0",   // Invalid minor
            "1.0.z",   // Invalid patch
            "1.0.0-x", // Invalid pre-release
        ];

        for invalid in invalid_cases {
            let result =
                serde_json::from_str::<Version>(&format!("\"{}\"", invalid));
            assert!(result.is_err(), "Should fail for input: {}", invalid);
        }
    }

    #[test]
    fn test_event_builder() {
        // Create objects that can be created directly
        let path = RuesPath::new_modern(
            Target::Blocks,
            Some(testing::create_test_block_hash()),
            testing::create_test_topic("accepted"),
        );
        let payload = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/octet-stream")
            .content_length(3)
            .build()
            .unwrap();

        // Create event with all fields
        let event = EventBuilder::new()
            .path(path)
            .payload(payload)
            .headers(headers)
            .operation(EventOperation::Subscribe)
            .session_id(testing::create_test_session_id())
            .version(testing::create_release_version())
            .build()
            .unwrap();

        // Verify all fields
        assert!(event.path().is_some());
        assert!(event.payload().is_some());
        assert!(event.headers().is_some());
        assert_eq!(event.operation(), EventOperation::Subscribe);
        assert!(event.session_id().is_some());
        assert_eq!(event.version(), testing::create_release_version());

        // Test defaults
        let default_event = EventBuilder::new().build().unwrap();
        assert_eq!(default_event.operation(), EventOperation::Message);
        assert_eq!(default_event.version(), Version::new(0, 1, 0, None));
    }

    #[test]
    fn test_event_builder_content_length() {
        // Test with payload - should set content length
        let binary_payload = RuesValue::Binary(Bytes::from(vec![1, 2, 3]));
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/octet-stream")
            .build()
            .unwrap();

        let event = EventBuilder::new()
            .path(RuesPath::new_modern(
                Target::Blocks,
                None,
                testing::create_test_topic("accepted"),
            ))
            .payload(binary_payload)
            .headers(headers)
            .build()
            .unwrap();

        assert_eq!(event.content_length(), Some(3));

        // Test without payload - should default to 0
        let event = EventBuilder::new()
            .path(RuesPath::new_modern(
                Target::Blocks,
                None,
                testing::create_test_topic("accepted"),
            ))
            .headers(
                RuesHeaders::builder()
                    .content_location("/on/blocks/accepted")
                    .content_type("application/json")
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();

        assert_eq!(event.content_length(), Some(0));

        // Test without headers
        let event = EventBuilder::new()
            .path(RuesPath::new_modern(
                Target::Blocks,
                None,
                testing::create_test_topic("accepted"),
            ))
            .build()
            .unwrap();

        assert_eq!(event.content_length(), None);
    }

    #[test]
    fn test_event_builder_content_length_matches_binary() {
        // Test that content length matches actual binary payload size
        let payloads = vec![
            RuesValue::Binary(Bytes::from(vec![1, 2, 3])),
            RuesValue::Json(json!({"test": "Hello 🦀"})),
            RuesValue::Text("Hello 🦀".into()),
            RuesValue::GraphQL("query { test }".into()),
            RuesValue::Proof(Bytes::from(vec![4, 5, 6])),
        ];

        for payload in payloads {
            let headers = RuesHeaders::builder()
                .content_location("/on/blocks/accepted")
                .content_type(payload.content_type())
                .build()
                .unwrap();

            let event = EventBuilder::new()
                .path(RuesPath::new_modern(
                    Target::Blocks,
                    None,
                    testing::create_test_topic("accepted"),
                ))
                .payload(payload.clone())
                .headers(headers)
                .build()
                .unwrap();

            // The content length should match the actual payload size
            let expected_len = payload.byte_len().unwrap();
            assert_eq!(event.content_length(), Some(expected_len));

            // Verify that content length matches the actual binary
            // representation
            let payload_bytes = payload.to_bytes().unwrap();
            assert_eq!(payload_bytes[5..].len(), expected_len); // Skip tag and
                                                                // length prefix
        }
    }

    #[test]
    fn test_event_operations() {
        // Test session requirements
        assert!(EventOperation::Subscribe.requires_session());
        assert!(EventOperation::Unsubscribe.requires_session());
        assert!(EventOperation::Message.requires_session());
        assert!(!EventOperation::Connect.requires_session());
        assert!(!EventOperation::Dispatch.requires_session());

        // Test HTTP methods
        assert_eq!(EventOperation::Subscribe.http_method(), Some("GET"));
        assert_eq!(EventOperation::Unsubscribe.http_method(), Some("DELETE"));
        assert_eq!(EventOperation::Dispatch.http_method(), Some("POST"));
        assert_eq!(EventOperation::Connect.http_method(), None);
        assert_eq!(EventOperation::Message.http_method(), None);

        // Test WebSocket operations
        assert!(EventOperation::Connect.is_websocket());
        assert!(EventOperation::Message.is_websocket());
        assert!(!EventOperation::Subscribe.is_websocket());
        assert!(!EventOperation::Unsubscribe.is_websocket());
        assert!(!EventOperation::Dispatch.is_websocket());

        // Test path requirements
        assert!(!EventOperation::Connect.requires_path());
        assert!(EventOperation::Message.requires_path());
        assert!(EventOperation::Subscribe.requires_path());
        assert!(EventOperation::Unsubscribe.requires_path());
        assert!(EventOperation::Dispatch.requires_path());

        // Test headers requirements
        assert!(!EventOperation::Connect.requires_headers());
        assert!(EventOperation::Message.requires_headers());
        assert!(EventOperation::Subscribe.requires_headers());
        assert!(EventOperation::Unsubscribe.requires_headers());
        assert!(EventOperation::Dispatch.requires_headers());
    }

    #[test]
    fn test_event_accessors() {
        // Create event with directly constructible objects
        let path = RuesPath::new_modern(
            Target::Blocks,
            Some(testing::create_test_block_hash()),
            testing::create_test_topic("accepted"),
        );
        let payload = RuesValue::Json(serde_json::json!({ "test": true }));
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .build()
            .unwrap();

        let event = EventBuilder::new()
            .path(path)
            .payload(payload)
            .headers(headers)
            .operation(EventOperation::Subscribe)
            .session_id(testing::create_test_session_id())
            .version(testing::create_release_version())
            .build()
            .unwrap();

        // Test path-related accessors
        assert!(event.path().is_some());
        assert_eq!(event.target(), Some(Target::Blocks));
        assert_eq!(event.topic(), Some("accepted"));
        assert!(!event.is_legacy());

        // Test target-specific accessors
        assert!(event.block_hash().is_some());
        assert!(event.transaction_hash().is_none());
        assert!(event.contract_id().is_none());

        // Test headers-related accessors
        assert!(event.headers().is_some());
        assert_eq!(event.content_type(), Some("application/json"));
        assert!(event.content_length().is_some());

        // Test session-related accessors
        assert!(event.session_id().is_some());
        assert!(event.has_session());

        // Test timestamp
        assert!(event.timestamp() <= Utc::now());
    }

    #[test]
    fn test_version_display() {
        // Release versions
        let test_cases = vec![
            (testing::create_test_version(1, 0, 0, None), "1.0.0"),
            (testing::create_test_version(0, 1, 0, None), "0.1.0"),
            (testing::create_test_version(0, 0, 1, None), "0.0.1"),
            (testing::create_test_version(2, 3, 4, None), "2.3.4"),
        ];

        for (version, expected) in test_cases {
            assert_eq!(version.to_string(), expected);
        }

        // Pre-release versions
        let test_cases = vec![
            (testing::create_test_version(1, 0, 0, Some(1)), "1.0.0-1"),
            (testing::create_test_version(2, 3, 4, Some(5)), "2.3.4-5"),
            (testing::create_test_version(0, 0, 1, Some(2)), "0.0.1-2"),
        ];

        for (version, expected) in test_cases {
            assert_eq!(version.to_string(), expected);
        }

        // Format in error messages
        let version = testing::create_test_version(1, 2, 3, Some(4));
        let err = ValidationError::InvalidFormat(format!(
            "Invalid version: {}",
            version
        ));
        assert_eq!(
            err.to_string(),
            "Invalid message format: Invalid version: 1.2.3-4"
        );
    }
}
