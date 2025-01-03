// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error types and handling for the RUES domain layer.
//!
//! This module provides a comprehensive error handling system with:
//! - Rich error context and attributes
//! - Type-safe error categories
//! - Common error attributes
//! - Thread-safe context sharing
//! - Error conversion and wrapping
//!
//! # Core Components
//!
//! - [`DomainError`]: Root error type for all domain operations
//! - [`ErrorContext`]: Thread-safe error context with attributes
//! - [`WithContext`]: Trait for adding context to errors
//! - [`CommonErrorAttributes`]: Convenience methods for common attributes
//!
//! # Error Categories
//!
//! 1. **Validation Errors**: Input validation failures
//! 2. **Processing Errors**: Runtime processing failures
//! 3. **Conversion Errors**: Data format conversion issues
//! 4. **Protocol Errors**: RUES protocol violations
//! 5. **Resource Errors**: System resource issues
//! 6. **Serialization Errors**: Data encoding/decoding failures
//!
//! # Examples
//!
//! Basic error handling with context:
//! ```rust
//! use rusk::http::domain::error::{
//!     ValidationError, WithContext, CommonErrorAttributes,
//!     ProcessingError, DomainError,
//! };
//! use std::time::Duration;
//!
//! // Create error with context and attributes
//! let err = ValidationError::EmptyInput("no data".into())
//!     .with_context("validate_request")
//!     .with_input_size(0)
//!     .with_content_type("application/json")
//!     .with_request_id("req-123");
//!
//! assert_eq!(err.operation().unwrap(), "validate_request");
//! assert_eq!(err.get_attribute("input_size").unwrap(), "0");
//!
//! // Error wrapping with context preservation
//! let processing_err = ProcessingError::StageFailed {
//!     stage: "validation".into(),
//!     reason: "invalid input".into(),
//! }
//! .with_context("process_request")
//! .with_stage("validation")
//! .with_duration(Duration::from_secs(1));
//!
//! assert_eq!(processing_err.get_attribute("stage").unwrap(), "validation");
//! ```
//!
//! Error handling in processors:
//! ```rust
//! use rusk::http::domain::error::{
//!     DomainError, ProcessingError, ValidationError, ResourceError,
//!     WithContext, CommonErrorAttributes,
//! };
//! use std::time::Duration;
//!
//! const MAX_SIZE: usize = 1024 * 1024; // 1MB
//!
//! fn validate_input(data: &[u8]) -> Result<(), DomainError> {
//!     if data.is_empty() {
//!         return Err(ValidationError::EmptyInput("No data provided".into())
//!             .with_context("validate_input")
//!             .with_input_size(0));
//!     }
//!
//!     if data.len() > MAX_SIZE {
//!         return Err(ValidationError::InputTooLarge {
//!             size: data.len(),
//!             max: MAX_SIZE,
//!         }
//!         .with_context("validate_input")
//!         .with_input_size(data.len())
//!         .with_resource_usage(data.len(), MAX_SIZE, "bytes"));
//!     }
//!
//!     Ok(())
//! }
//!
//! async fn process_message(data: Vec<u8>) -> Result<Vec<u8>, DomainError> {
//!     // Validation with context
//!     validate_input(&data)?;
//!
//!     // Processing with timeout
//!     match tokio::time::timeout(Duration::from_secs(5), async {
//!         // Simulate processing
//!         Ok(data.clone())
//!     })
//!     .await
//!     {
//!         Ok(result) => result.map_err(|e: DomainError| {
//!             e.with_context("process_message")
//!                 .with_input_size(data.len())
//!         }),
//!         Err(_) => Err(ProcessingError::Timeout {
//!             operation: "process_message".into(),
//!             duration: Duration::from_secs(5),
//!         }
//!         .with_context("process_message")
//!         .with_duration(Duration::from_secs(5))),
//!     }
//! }
//! ```
//!
//! Error handling with resource monitoring:
//! ```rust
//! use rusk::http::domain::error::{ResourceError, WithContext, CommonErrorAttributes, DomainError};
//! use std::time::Duration;
//!
//! fn check_memory(required: usize, available: usize) -> Result<(), DomainError> {
//!     if required > available {
//!         return Err(ResourceError::MemoryLimitExceeded {
//!             requested: required,
//!             limit: available,
//!         }
//!         .with_context("memory_check")
//!         .with_resource_usage(required, available, "mb")
//!         .with_duration(Duration::from_millis(100)));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! # Thread Safety
//!
//! All error types and context are thread-safe:
//! - `ErrorContext` uses `Arc` and `RwLock` for safe sharing
//! - Attributes can be modified concurrently
//! - All error types implement `Send` + `Sync`
//! - Context cloning is efficient using `Arc`
//!
//! # Performance Considerations
//!
//! - Context creation is cheap (microseconds)
//! - Attribute access uses read-optimized locks
//! - Context cloning is O(1) using `Arc`
//! - Error wrapping preserves existing context

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc, time::Duration};
use thiserror::Error;

/// Context for domain errors providing additional information about when and
/// where the error occurred.
///
/// This type is thread-safe and can be shared between threads. It provides:
/// - Original error source
/// - Operation name where error occurred
/// - Timestamp when error occurred
/// - Optional context attributes
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::error::{DomainError, ErrorContext, ValidationError};
///
/// let error = ValidationError::EmptyInput("No data".into());
/// let context = ErrorContext::new(
///     error.into(),
///     "process_message",
/// );
///
/// assert_eq!(context.context_operation(), "process_message");
/// assert!(context.context_timestamp() <= chrono::Utc::now());
/// ```
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// The original error
    source: Arc<DomainError>,
    /// Operation where error occurred
    operation: String,
    /// When the error occurred
    timestamp: DateTime<Utc>,
    /// Current context attributes
    attributes: Arc<RwLock<HashMap<String, String>>>,
    /// Attributes waiting to be applied
    pending_attributes: Arc<RwLock<HashMap<String, String>>>,
}

impl ErrorContext {
    /// Creates a new error context from a source error and operation name.
    ///
    /// # Arguments
    /// * `source` - The original error that occurred
    /// * `operation` - Name of the operation where error occurred
    ///
    /// # Examples
    /// ```rust
    /// use rusk::http::domain::error::{ErrorContext, ValidationError};
    ///
    /// let source = ValidationError::EmptyInput("No data".into()).into();
    /// let context = ErrorContext::new(source, "validate_message");
    ///
    /// assert_eq!(context.context_operation(), "validate_message");
    /// ```
    pub fn new(source: DomainError, operation: impl Into<String>) -> Self {
        Self {
            source: Arc::new(source),
            operation: operation.into(),
            timestamp: Utc::now(),
            attributes: Arc::new(RwLock::new(HashMap::new())),
            pending_attributes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns reference to source error
    pub fn source(&self) -> &DomainError {
        &self.source
    }

    /// Returns operation name
    pub fn context_operation(&self) -> &str {
        &self.operation
    }

    /// Returns error timestamp
    pub fn context_timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Adds context attribute and returns self for chaining
    ///
    /// If context is not yet set, attribute will be stored and applied when
    /// context is set. If context is never set, attribute will be
    /// discarded.
    ///
    /// # Arguments
    /// * `key` - Attribute key
    /// * `value` - Attribute value
    ///
    /// # Examples
    /// ```rust
    /// # use rusk::http::domain::error::{ErrorContext, ValidationError};
    /// let source = ValidationError::EmptyInput("No data".into()).into();
    /// let context = ErrorContext::new(source, "validate_message");
    ///
    /// // Add attributes individually
    /// context.add_attribute("input_size", "0");
    /// context.add_attribute("content_type", "application/json");
    ///
    /// assert_eq!(context.get_context_attribute("input_size").unwrap(), "0");
    /// ```
    pub fn add_attribute(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> &Self {
        // If we have an operation, add to current attributes
        if !self.operation.is_empty() {
            self.attributes.write().insert(key.into(), value.into());
        } else {
            // Otherwise, store as pending
            self.pending_attributes
                .write()
                .insert(key.into(), value.into());
        }
        self
    }

    pub fn apply_pending_attributes(&self) {
        let mut pending = self.pending_attributes.write();
        let mut current = self.attributes.write();
        current.extend(pending.drain());
    }

    /// Gets context attribute
    ///
    /// # Arguments
    /// * `key` - Attribute key
    ///
    /// # Returns
    /// * `Some(String)` - Attribute value if found
    /// * `None` - If attribute doesn't exist
    pub fn get_context_attribute(&self, key: &str) -> Option<String> {
        self.attributes.read().get(key).cloned()
    }

    /// Returns all attributes as a new HashMap
    pub fn context_attributes(&self) -> HashMap<String, String> {
        self.attributes.read().clone()
    }

    /// Creates a new error context while preserving existing attributes
    ///
    /// This is useful when wrapping an error while keeping its context
    ///
    /// # Arguments
    /// * `new_source` - New error to wrap the current one
    /// * `new_operation` - New operation name
    ///
    /// # Examples
    /// ```rust
    /// # use rusk::http::domain::error::{ErrorContext, ValidationError, ProcessingError};
    /// let validation_err = ValidationError::EmptyInput("No data".into()).into();
    /// let validation_ctx = ErrorContext::new(validation_err, "validate");
    ///
    /// validation_ctx.add_attribute("stage", "validation");
    ///
    /// let processing_err = ProcessingError::StageFailed {
    ///     stage: "processing".into(),
    ///     reason: "validation failed".into()
    /// }.into();
    ///
    /// let processing_ctx = validation_ctx.wrap(processing_err, "process");
    /// assert_eq!(processing_ctx.get_context_attribute("stage").unwrap(), "validation");
    /// ```
    pub fn wrap(
        &self,
        new_source: DomainError,
        new_operation: impl Into<String>,
    ) -> Self {
        let new_ctx = Self::new(new_source, new_operation);
        let existing_attrs = self.context_attributes();
        for (k, v) in existing_attrs {
            new_ctx.add_attribute(k, v);
        }
        new_ctx
    }
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error in operation '{}' at {}: {}",
            self.operation, self.timestamp, self.source
        )
    }
}

impl std::error::Error for ErrorContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Root error type for all domain operations.
///
/// This enum represents all possible errors that can occur in the RUES domain
/// layer. It supports rich error context through the `WithContext` variant and
/// provides conversion from specific error types.
///
/// # Error Categories
///
/// * `WithContext` - Error with additional context and attributes
/// * `Validation` - Input validation failures
/// * `Processing` - Runtime processing failures
/// * `Conversion` - Data format conversion issues
/// * `Protocol` - RUES protocol violations
/// * `Resource` - System resource issues
/// * `SerDe` - Serialization/deserialization failures
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::error::{DomainError, ValidationError, WithContext};
///
/// // Create error with context
/// let err = ValidationError::EmptyInput("no data".into())
///     .with_context_and_attributes(
///         "validate",
///         [
///             ("input_type", "json"),
///             ("request_id", "123"),
///         ],
///     );
///
/// // Access context information
/// assert_eq!(err.operation().unwrap(), "validate");
/// assert_eq!(err.get_attribute("input_type").unwrap(), "json");
///
/// // Error matching
/// match err {
///     DomainError::WithContext(ctx) => {
///         println!("Error in {}: {}", ctx.context_operation(), ctx.source());
///     }
///     _ => println!("Error without context: {}", err),
/// }
/// ```
///
/// # Thread Safety
///
/// The error type is thread-safe and can be shared between threads:
/// - `Clone` creates a new reference to the same context
/// - Context attributes can be modified concurrently
/// - All error variants implement `Send` + `Sync`
#[derive(Error, Debug)]
pub enum DomainError {
    /// Error with context
    #[error("{0}")]
    WithContext(Box<ErrorContext>),

    /// Validation errors (invalid input, state, etc.)
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Processing errors (timeouts, rate limits, etc.)
    #[error("Processing error: {0}")]
    Processing(#[from] ProcessingError),

    /// Conversion errors (format mismatches, etc.)
    #[error("Conversion error: {0}")]
    Conversion(#[from] ConversionError),

    /// Protocol errors (malformed messages, etc.)
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Resource errors (memory, connections, etc.)
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    /// Serialization/deserialization errors
    #[error("SerDe error: {0}")]
    SerDe(#[from] SerDeError),

    /// Other errors that don't fit above categories
    ///
    /// Note: the DomainError's context will be lost when converting to
    /// `anyhow::Error`
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl DomainError {
    /// Returns error context if available
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::WithContext(ctx) => Some(ctx),
            _ => None,
        }
    }

    /// Returns operation name if context is available
    pub fn operation(&self) -> Option<&str> {
        self.context().map(|ctx| ctx.context_operation())
    }

    /// Returns error timestamp if context is available
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.context().map(|ctx| ctx.context_timestamp())
    }

    /// Gets error context attribute if available
    pub fn get_attribute(&self, key: &str) -> Option<String> {
        self.context()
            .and_then(|ctx| ctx.get_context_attribute(key))
    }
}

/// Validation-specific errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Empty input provided
    #[error("Empty input: {0}")]
    EmptyInput(String),

    /// Input too large
    #[error("Input too large: {size} bytes (max: {max} bytes)")]
    InputTooLarge { size: usize, max: usize },

    /// Invalid content type
    #[error("Invalid content type: {found} (expected: {expected})")]
    InvalidContentType { found: String, expected: String },

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value
    #[error("Invalid field value: {field} ({reason})")]
    InvalidFieldValue { field: String, reason: String },

    /// Invalid operation requested
    #[error("Invalid operation: {operation} ({reason})")]
    InvalidOperation { operation: String, reason: String },
}

/// Processing-specific errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProcessingError {
    /// Operation was cancelled
    #[error("Operation cancelled: {operation} (reason: {reason})")]
    Cancelled {
        /// Name of the cancelled operation
        operation: String,
        /// Reason for cancellation (e.g., "timeout", "user requested",
        /// "shutdown")
        reason: String,
    },

    /// Operation timed out
    #[error("Operation timed out after {duration:?}: {operation}")]
    Timeout {
        operation: String,
        duration: Duration,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {current} (limit: {limit})")]
    RateLimitExceeded { limit: usize, current: usize },

    /// Processing stage failed
    #[error("Processing stage failed: {stage} ({reason})")]
    StageFailed { stage: String, reason: String },

    /// Processor configuration error
    #[error("Processor configuration error: {0}")]
    Configuration(String),

    /// Invalid processing state
    #[error("Invalid processing state: {state} ({reason})")]
    InvalidState { state: String, reason: String },

    /// Pipeline execution error
    #[error("Pipeline execution error: {stage} ({reason})")]
    PipelineExecution { stage: String, reason: String },
}

/// Protocol-specific errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Invalid message header
    #[error("Invalid message header: {0}")]
    InvalidHeader(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),

    /// Protocol version mismatch
    #[error("Protocol version mismatch: client={client}, server={server}")]
    VersionMismatch { client: String, server: String },

    /// Missing required header
    #[error("Missing required header: {0}")]
    MissingHeader(String),

    /// Invalid content type
    #[error("Invalid content type: {found} (expected: {expected})")]
    InvalidContentType { found: String, expected: String },
}

/// Resource-specific errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    /// Memory limit exceeded
    #[error("Memory limit exceeded: {requested} bytes (limit: {limit} bytes)")]
    MemoryLimitExceeded { limit: usize, requested: usize },

    /// Connection limit exceeded
    #[error("Connection limit exceeded: {current} (limit: {limit})")]
    ConnectionLimitExceeded { limit: u32, current: u32 },

    /// Resource temporarily unavailable
    #[error("Resource unavailable: {resource} ({reason})")]
    Unavailable { resource: String, reason: String },

    /// Resource exhausted
    #[error("Resource exhausted: {resource} ({reason})")]
    Exhausted { resource: String, reason: String },
}

/// Conversion-specific errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// Unsupported conversion
    #[error("Unsupported conversion: {from} -> {to}")]
    UnsupportedConversion { from: String, to: String },

    /// Data loss during conversion
    #[error("Data loss during conversion: {reason}")]
    DataLoss { reason: String },

    /// Invalid source format
    #[error("Invalid source format: {format} ({reason})")]
    InvalidSourceFormat { format: String, reason: String },

    /// Invalid target format
    #[error("Invalid target format: {format} ({reason})")]
    InvalidTargetFormat { format: String, reason: String },
}

/// Serialization and deserialization errors
#[derive(Error, Debug)]
pub enum SerDeError {
    /// JSON errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Binary format errors
    #[error("Binary format error: {kind} ({reason})")]
    Binary { kind: String, reason: String },

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field type
    #[error("Invalid field type: {field} (expected: {expected})")]
    InvalidFieldType { field: String, expected: String },
}

impl From<serde_json::Error> for DomainError {
    fn from(err: serde_json::Error) -> Self {
        DomainError::SerDe(SerDeError::Json(err))
    }
}

/// Helper trait for adding context to errors.
///
/// This trait provides methods to add operation context and attributes to
/// errors. It's implemented for all specific error types and `DomainError`
/// itself.
///
/// # Methods
///
/// * `with_context` - Adds basic operation context
/// * `with_attributes` - Adds multiple attributes
/// * `with_context_and_attributes` - Combines both operations
///
/// # Examples
///
/// Basic context:
/// ```rust
/// use rusk::http::domain::error::{ProcessingError, WithContext};
/// use std::time::Duration;
///
/// let err = ProcessingError::Timeout {
///     operation: "request".into(),
///     duration: Duration::from_secs(5),
/// }
/// .with_context("http_handler");
///
/// assert_eq!(err.operation().unwrap(), "http_handler");
/// ```
///
/// Context with attributes:
/// ```rust
/// # use rusk::http::domain::error::{ProcessingError, WithContext};
/// # use std::time::Duration;
/// let err = ProcessingError::Timeout {
///     operation: "request".into(),
///     duration: Duration::from_secs(5),
/// }
/// .with_context_and_attributes(
///     "http_handler",
///     [
///         ("request_id", "123"),
///         ("client_ip", "127.0.0.1"),
///     ],
/// );
///
/// assert_eq!(err.get_attribute("request_id").unwrap(), "123");
/// ```
pub trait WithContext: Into<DomainError> + Sized {
    /// Adds operation context to the error.
    ///
    /// If there were any attributes added before context, they will be applied
    /// now.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ValidationError, WithContext, DomainError};
    ///
    /// let err = ValidationError::EmptyInput("no data".into())
    ///     .with_attributes([("key", "value")])  // Attributes stored as pending
    ///     .with_context("validate");  // Context set and pending attributes applied
    ///
    /// // Pattern match to ensure we're dealing with a WithContext variant
    /// if let DomainError::WithContext(ctx) = err {
    ///     assert_eq!(ctx.get_context_attribute("key").unwrap(), "value");
    ///     assert_eq!(ctx.context_operation(), "validate");
    /// } else {
    ///     panic!("Expected WithContext variant");
    /// }
    /// ```
    fn with_context(self, operation: impl Into<String>) -> DomainError {
        let domain_error = self.into();

        // Get attributes from previous context if any
        let previous_attrs =
            if let DomainError::WithContext(ctx) = &domain_error {
                ctx.context_attributes()
            } else {
                HashMap::new()
            };

        // Create new context with the operation
        let ctx = ErrorContext::new(domain_error, operation);

        // Transfer previous attributes
        for (k, v) in previous_attrs {
            ctx.add_attribute(k, v);
        }

        DomainError::WithContext(Box::new(ctx))
    }

    /// Adds attributes to the error.
    ///
    /// If context is not yet set, attributes will be stored and applied when
    /// context is set. If context is never set, attributes will be
    /// discarded.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rusk::http::domain::error::{ValidationError, WithContext};
    /// // Attributes before context
    /// let err = ValidationError::EmptyInput("no data".into())
    ///     .with_attributes([
    ///         ("input_type", "json"),
    ///         ("request_id", "123"),
    ///     ])
    ///     .with_context("validate");
    ///
    /// assert_eq!(err.get_attribute("input_type").unwrap(), "json");
    ///
    /// // Attributes after context
    /// let err = ValidationError::EmptyInput("no data".into())
    ///     .with_context("validate")
    ///     .with_attributes([
    ///         ("input_type", "json"),
    ///         ("request_id", "123"),
    ///     ]);
    ///
    /// assert_eq!(err.get_attribute("input_type").unwrap(), "json");
    /// ```
    fn with_attributes(
        self,
        attributes: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> DomainError {
        let domain_error = self.into();
        match domain_error {
            DomainError::WithContext(ctx) => {
                // Context already exists, add attributes directly
                for (k, v) in attributes {
                    ctx.add_attribute(k, v);
                }
                DomainError::WithContext(ctx)
            }
            other => {
                // Create a special pending context
                let ctx = ErrorContext::new(other, "_pending_");
                for (k, v) in attributes {
                    ctx.add_attribute(k, v);
                }
                DomainError::WithContext(Box::new(ctx))
            }
        }
    }

    /// Convenience method to set both context and attributes at once
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rusk::http::domain::error::{ValidationError, WithContext};
    /// let err = ValidationError::EmptyInput("no data".into())
    ///     .with_context_and_attributes(
    ///         "validate",
    ///         [
    ///             ("input_type", "json"),
    ///             ("request_id", "123"),
    ///         ],
    ///     );
    ///
    /// assert_eq!(err.operation().unwrap(), "validate");
    /// assert_eq!(err.get_attribute("input_type").unwrap(), "json");
    /// ```
    fn with_context_and_attributes(
        self,
        operation: impl Into<String>,
        attributes: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> DomainError {
        let err = self.with_context(operation);
        if let DomainError::WithContext(ctx) = err {
            for (k, v) in attributes {
                ctx.add_attribute(k, v);
            }
            DomainError::WithContext(ctx)
        } else {
            err
        }
    }
}

impl<T: Into<DomainError> + Sized> WithContext for T {}

/// Extension trait providing convenience methods for adding common error
/// attributes.
///
/// This trait offers a fluent interface for adding commonly used attributes
/// like:
/// - Input/output sizes
/// - Content types
/// - Request IDs
/// - Processing stages
/// - Resource usage
/// - Version information
///
/// # Examples
///
/// Size and type attributes:
/// ```rust
/// use rusk::http::domain::error::{ValidationError, WithContext, CommonErrorAttributes};
///
/// let err = ValidationError::EmptyInput("no data".into())
///     .with_context("validate")
///     .with_input_size(0)
///     .with_content_type("application/json");
///
/// assert_eq!(err.get_attribute("input_size").unwrap(), "0");
/// ```
///
/// Resource usage:
/// ```rust
/// # use rusk::http::domain::error::{ResourceError, WithContext, CommonErrorAttributes};
/// let err = ResourceError::MemoryLimitExceeded {
///     requested: 150,
///     limit: 100,
/// }
/// .with_context("allocate")
/// .with_resource_usage(150, 100, "mb");
///
/// assert_eq!(err.get_attribute("usage_percent_mb").unwrap(), "150.00");
/// ```
pub trait CommonErrorAttributes {
    /// Adds input size attribute to the error.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the input in bytes
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use rusk::http::domain::error::{ValidationError, WithContext, CommonErrorAttributes};
    /// let err = ValidationError::EmptyInput("no data".into())
    ///     .with_context("validate")
    ///     .with_input_size(1024);
    ///
    /// assert_eq!(err.get_attribute("input_size").unwrap(), "1024");
    /// ```
    fn with_input_size(self, size: usize) -> DomainError;

    /// Adds output size attribute to the error.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the output in bytes
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ConversionError, WithContext, CommonErrorAttributes};
    ///
    /// let err = ConversionError::DataLoss {
    ///     reason: "truncated".into(),
    /// }
    /// .with_context("convert_json")
    /// .with_output_size(512);
    ///
    /// assert_eq!(err.get_attribute("output_size").unwrap(), "512");
    /// ```
    fn with_output_size(self, size: usize) -> DomainError;

    /// Adds content type attribute to the error.
    ///
    /// # Arguments
    ///
    /// * `content_type` - MIME type of the content (e.g., "application/json")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ProtocolError, WithContext, CommonErrorAttributes};
    ///
    /// let err = ProtocolError::InvalidContentType {
    ///     found: "text/plain".into(),
    ///     expected: "application/json".into(),
    /// }
    /// .with_context("validate_payload")
    /// .with_content_type("text/plain");
    ///
    /// assert_eq!(err.get_attribute("content_type").unwrap(), "text/plain");
    /// ```
    fn with_content_type(self, content_type: impl Into<String>) -> DomainError;

    /// Adds request ID attribute to the error.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the request
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ProcessingError, WithContext, CommonErrorAttributes};
    /// use std::time::Duration;
    ///
    /// let err = ProcessingError::Timeout {
    ///     operation: "process".into(),
    ///     duration: Duration::from_secs(5),
    /// }
    /// .with_context("handle_request")
    /// .with_request_id("req-123-abc");
    ///
    /// assert_eq!(err.get_attribute("request_id").unwrap(), "req-123-abc");
    /// ```
    fn with_request_id(self, id: impl Into<String>) -> DomainError;

    /// Adds processing stage attribute to the error.
    ///
    /// # Arguments
    ///
    /// * `stage` - Current processing stage when error occurred
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ProcessingError, WithContext, CommonErrorAttributes};
    ///
    /// let err = ProcessingError::StageFailed {
    ///     stage: "validation".into(),
    ///     reason: "invalid format".into(),
    /// }
    /// .with_context("process_event")
    /// .with_stage("input_validation");
    ///
    /// assert_eq!(err.get_attribute("stage").unwrap(), "input_validation");
    /// ```
    fn with_stage(self, stage: impl Into<String>) -> DomainError;

    /// Adds duration attribute to the error in milliseconds.
    ///
    /// # Arguments
    ///
    /// * `duration` - Time duration to record
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ProcessingError, WithContext, CommonErrorAttributes};
    /// use std::time::Duration;
    ///
    /// let err = ProcessingError::Timeout {
    ///     operation: "process".into(),
    ///     duration: Duration::from_secs(5),
    /// }
    /// .with_context("process_request")
    /// .with_duration(Duration::from_secs(5));
    ///
    /// assert_eq!(err.get_attribute("duration_ms").unwrap(), "5000");
    /// ```
    fn with_duration(self, duration: Duration) -> DomainError;

    /// Adds resource usage attributes including used amount, total capacity,
    /// and usage percentage.
    ///
    /// # Arguments
    ///
    /// * `used` - Amount of resource currently in use
    /// * `total` - Total resource capacity
    /// * `unit` - Unit of measurement (e.g., "mb", "connections")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ResourceError, WithContext, CommonErrorAttributes};
    ///
    /// // Memory usage
    /// let err = ResourceError::MemoryLimitExceeded {
    ///     requested: 150,
    ///     limit: 100,
    /// }
    /// .with_context("allocate_buffer")
    /// .with_resource_usage(150, 100, "mb");
    ///
    /// assert_eq!(err.get_attribute("used_mb").unwrap(), "150");
    /// assert_eq!(err.get_attribute("total_mb").unwrap(), "100");
    /// assert_eq!(err.get_attribute("usage_percent_mb").unwrap(), "150.00");
    ///
    /// // Connection usage
    /// let err = ResourceError::ConnectionLimitExceeded {
    ///     current: 1000,
    ///     limit: 800,
    /// }
    /// .with_context("accept_connection")
    /// .with_resource_usage(1000, 800, "connections");
    ///
    /// assert_eq!(err.get_attribute("used_connections").unwrap(), "1000");
    /// ```
    fn with_resource_usage(
        self,
        used: usize,
        total: usize,
        unit: impl Into<String>,
    ) -> DomainError;

    /// Adds version information for a component to the error.
    ///
    /// # Arguments
    ///
    /// * `component` - Name of the component (e.g., "client", "server")
    /// * `version` - Version string
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::error::{ProtocolError, WithContext, CommonErrorAttributes};
    ///
    /// let err = ProtocolError::VersionMismatch {
    ///     client: "1.0.0".into(),
    ///     server: "2.0.0".into(),
    /// }
    /// .with_context("version_check")
    /// .with_version_info("client", "1.0.0")
    /// .with_version_info("server", "2.0.0");
    ///
    /// assert_eq!(err.get_attribute("client_version").unwrap(), "1.0.0");
    /// assert_eq!(err.get_attribute("server_version").unwrap(), "2.0.0");
    /// ```
    fn with_version_info(
        self,
        component: impl Into<String>,
        version: impl Into<String>,
    ) -> DomainError;
}

/// Implementation of common error attributes for any domain error.
///
/// This implementation allows adding common attributes to any error type
/// that can be converted to `DomainError`, providing a convenient and
/// consistent way to add context information.
///
/// # Adding Context First
///
/// If the error doesn't have context yet, it will be automatically added
/// with an "unknown" operation name before adding attributes.
///
/// # Examples
///
/// Direct usage with `DomainError`:
/// ```rust
/// use rusk::http::domain::error::{DomainError, ValidationError, CommonErrorAttributes};
///
/// let err: DomainError = ValidationError::EmptyInput("no data".into()).into();
/// let err = err
///     .with_input_size(0)
///     .with_content_type("application/json");
///
/// assert_eq!(err.get_attribute("input_size").unwrap(), "0");
/// ```
///
/// Usage with specific error types:
/// ```rust
/// use rusk::http::domain::error::{
///     ProcessingError, WithContext, CommonErrorAttributes,
///     ResourceError,
/// };
/// use std::time::Duration;
///
/// // Processing error with metrics
/// let err = ProcessingError::Timeout {
///     operation: "request".into(),
///     duration: Duration::from_secs(5),
/// }
/// .with_context("http_handler")
/// .with_request_id("req-123")
/// .with_duration(Duration::from_secs(5))
/// .with_stage("processing");
///
/// assert_eq!(err.get_attribute("request_id").unwrap(), "req-123");
/// assert_eq!(err.get_attribute("duration_ms").unwrap(), "5000");
///
/// // Resource error with usage metrics
/// let err = ResourceError::MemoryLimitExceeded {
///     requested: 150,
///     limit: 100,
/// }
/// .with_context("memory_check")
/// .with_resource_usage(150, 100, "mb")
/// .with_stage("allocation");
///
/// assert_eq!(err.get_attribute("used_mb").unwrap(), "150");
/// assert_eq!(err.get_attribute("stage").unwrap(), "allocation");
/// ```
///
/// Chaining multiple attributes:
/// ```rust
/// use rusk::http::domain::error::{
///     ValidationError, WithContext, CommonErrorAttributes
/// };
///
/// let err = ValidationError::InputTooLarge {
///     size: 2048,
///     max: 1024,
/// }
/// .with_context("validate_payload")
/// .with_input_size(2048)
/// .with_content_type("application/json")
/// .with_request_id("req-456")
/// .with_stage("size_validation")
/// .with_resource_usage(2048, 1024, "bytes");
///
/// assert_eq!(err.get_attribute("input_size").unwrap(), "2048");
/// assert_eq!(err.get_attribute("usage_percent_bytes").unwrap(), "200.00");
/// ```
impl CommonErrorAttributes for DomainError {
    fn with_input_size(self, size: usize) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                ctx.add_attribute("input_size", size.to_string());
                DomainError::WithContext(ctx)
            }
            _ => self.with_context("unknown").with_input_size(size),
        }
    }

    fn with_output_size(self, size: usize) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                ctx.add_attribute("output_size", size.to_string());
                DomainError::WithContext(ctx)
            }
            _ => self.with_context("unknown").with_output_size(size),
        }
    }

    fn with_content_type(self, content_type: impl Into<String>) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                ctx.add_attribute("content_type", content_type.into());
                DomainError::WithContext(ctx)
            }
            _ => self.with_context("unknown").with_content_type(content_type),
        }
    }

    fn with_request_id(self, id: impl Into<String>) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                ctx.add_attribute("request_id", id.into());
                DomainError::WithContext(ctx)
            }
            _ => self.with_context("unknown").with_request_id(id),
        }
    }

    fn with_stage(self, stage: impl Into<String>) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                ctx.add_attribute("stage", stage.into());
                DomainError::WithContext(ctx)
            }
            _ => self.with_context("unknown").with_stage(stage),
        }
    }

    fn with_duration(self, duration: Duration) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                ctx.add_attribute(
                    "duration_ms",
                    duration.as_millis().to_string(),
                );
                DomainError::WithContext(ctx)
            }
            _ => self.with_context("unknown").with_duration(duration),
        }
    }

    fn with_resource_usage(
        self,
        used: usize,
        total: usize,
        unit: impl Into<String>,
    ) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                let unit = unit.into();
                ctx.add_attribute(format!("used_{}", unit), used.to_string())
                    .add_attribute(format!("total_{}", unit), total.to_string())
                    .add_attribute(
                        format!("usage_percent_{}", unit),
                        format!("{:.2}", (used as f64 / total as f64) * 100.0),
                    );
                DomainError::WithContext(ctx)
            }
            _ => self
                .with_context("unknown")
                .with_resource_usage(used, total, unit),
        }
    }

    fn with_version_info(
        self,
        component: impl Into<String>,
        version: impl Into<String>,
    ) -> DomainError {
        match self {
            DomainError::WithContext(ctx) => {
                let component = component.into();
                ctx.add_attribute(
                    format!("{}_version", component),
                    version.into(),
                );
                DomainError::WithContext(ctx)
            }
            _ => self
                .with_context("unknown")
                .with_version_info(component, version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;

    #[test]
    fn test_validation_errors() {
        let err = ValidationError::EmptyInput("No data provided".into());
        let domain_err: DomainError = err.clone().into();

        assert!(matches!(domain_err, DomainError::Validation(_)));
        assert_eq!(err.to_string(), "Empty input: No data provided");

        let err = ValidationError::InputTooLarge {
            size: 1024,
            max: 512,
        };
        assert!(err.to_string().contains("1024"));
        assert!(err.to_string().contains("512"));
    }

    #[test]
    fn test_processing_errors() {
        let err = ProcessingError::Timeout {
            operation: "request".into(),
            duration: Duration::from_secs(5),
        };
        let domain_err: DomainError = err.clone().into();

        assert!(matches!(domain_err, DomainError::Processing(_)));
        assert!(err.to_string().contains("5s"));

        let err = ProcessingError::RateLimitExceeded {
            limit: 100,
            current: 150,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("150"));
    }

    #[test]
    fn test_protocol_errors() {
        let err = ProtocolError::VersionMismatch {
            client: "1.0".into(),
            server: "2.0".into(),
        };
        let domain_err: DomainError = err.clone().into();

        assert!(matches!(domain_err, DomainError::Protocol(_)));
        assert!(err.to_string().contains("1.0"));
        assert!(err.to_string().contains("2.0"));
    }

    #[test]
    fn test_resource_errors() {
        let err = ResourceError::MemoryLimitExceeded {
            limit: 1024,
            requested: 2048,
        };
        let domain_err: DomainError = err.clone().into();

        assert!(matches!(domain_err, DomainError::Resource(_)));
        assert!(err.to_string().contains("1024"));
        assert!(err.to_string().contains("2048"));
    }

    #[test]
    fn test_error_conversion() {
        // Test JSON error conversion
        let json_err =
            serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let serde_err = SerDeError::Json(json_err);
        let domain_err: DomainError = serde_err.into();
        assert!(matches!(domain_err, DomainError::SerDe(_)));

        // Test anyhow error conversion
        fn test_error_conversion_context_preservation() {
            let err = ValidationError::EmptyInput("test".into())
                .with_context("validate")
                .with_input_size(0);

            // Get context information before conversion
            let input_size = err.get_attribute("input_size").unwrap();
            let operation = err.operation().unwrap().to_string();

            // Convert to anyhow error and back
            let domain_err: DomainError = anyhow::Error::from(err).into();

            // Context should be preserved
            assert_eq!(
                domain_err.get_attribute("input_size").unwrap(),
                input_size
            );
            assert_eq!(domain_err.operation().unwrap(), operation);
        }
        let other_err = anyhow::anyhow!("test error");
        let domain_err: DomainError = other_err.into();
        assert!(matches!(domain_err, DomainError::Other(_)));
    }

    #[test]
    fn test_complex_error_scenarios() {
        // Test nested error conversion
        let stage_err = ProcessingError::StageFailed {
            stage: "validation".into(),
            reason: ValidationError::EmptyInput("empty".into()).to_string(),
        };
        let domain_err: DomainError = stage_err.into();
        assert!(matches!(domain_err, DomainError::Processing(_)));

        // Test error with context
        let ctx_err = ProcessingError::PipelineExecution {
            stage: "transform".into(),
            reason: format!(
                "{}",
                ConversionError::DataLoss {
                    reason: "precision loss".into()
                }
            ),
        };
        assert!(ctx_err.to_string().contains("precision loss"));
    }

    #[test]
    fn test_cancellation_error() {
        let err = ProcessingError::Cancelled {
            operation: "process_message".into(),
            reason: "user requested".into(),
        };

        // Test direct error
        assert!(err.to_string().contains("Operation cancelled"));
        assert!(err.to_string().contains("process_message"));
        assert!(err.to_string().contains("user requested"));

        // Test with context
        let err_with_ctx = err
            .with_context("websocket_handler")
            .with_stage("processing")
            .with_request_id("req-123")
            .with_duration(Duration::from_secs(1));

        assert!(matches!(err_with_ctx, DomainError::WithContext(_)));
        assert_eq!(err_with_ctx.operation().unwrap(), "websocket_handler");
        assert_eq!(err_with_ctx.get_attribute("stage").unwrap(), "processing");
        assert_eq!(
            err_with_ctx.get_attribute("request_id").unwrap(),
            "req-123"
        );
        assert_eq!(err_with_ctx.get_attribute("duration_ms").unwrap(), "1000");
    }

    #[test]
    fn test_cancellation_error_conversion() {
        let err = ProcessingError::Cancelled {
            operation: "validate".into(),
            reason: "timeout".into(),
        };

        // Test conversion to DomainError
        let domain_err: DomainError = err.into();
        assert!(matches!(domain_err, DomainError::Processing(_)));
    }

    #[test]
    fn test_common_attributes() {
        let err = ValidationError::EmptyInput("test".into())
            .with_context("validate")
            .with_input_size(0)
            .with_content_type("application/json")
            .with_request_id("req-123")
            .with_stage("parsing")
            .with_duration(Duration::from_millis(100));

        assert_eq!(err.get_attribute("input_size").unwrap(), "0");
        assert_eq!(
            err.get_attribute("content_type").unwrap(),
            "application/json"
        );
        assert_eq!(err.get_attribute("request_id").unwrap(), "req-123");
        assert_eq!(err.get_attribute("stage").unwrap(), "parsing");
        assert_eq!(err.get_attribute("duration_ms").unwrap(), "100");
    }

    #[test]
    fn test_version_attributes() {
        let err = ProtocolError::VersionMismatch {
            client: "1.0.0".into(),
            server: "2.0.0".into(),
        }
        .with_context("version_check")
        .with_version_info("client", "1.0.0")
        .with_version_info("server", "2.0.0");

        assert_eq!(err.get_attribute("client_version").unwrap(), "1.0.0");
        assert_eq!(err.get_attribute("server_version").unwrap(), "2.0.0");
    }

    #[test]
    fn test_error_context_operations() {
        let source = ValidationError::EmptyInput("test".into()).into();
        let ctx = ErrorContext::new(source, "test_op");

        // Add attributes first
        ctx.add_attribute("key1", "value1");
        assert_eq!(ctx.get_context_attribute("key1").unwrap(), "value1");

        // Then test context wrapping
        let new_source = ProcessingError::StageFailed {
            stage: "process".into(),
            reason: "failed".into(),
        }
        .into();
        let wrapped = ctx.wrap(new_source, "new_op");

        // Verify wrapped context preserves attributes
        assert_eq!(wrapped.get_context_attribute("key1").unwrap(), "value1");
        assert_eq!(wrapped.context_operation(), "new_op");
    }

    #[tokio::test]
    async fn test_error_context_thread_safety() {
        let ctx = Arc::new(ErrorContext::new(
            ValidationError::EmptyInput("test".into()).into(),
            "thread_test",
        ));

        let mut handles = vec![];

        // Spawn multiple tasks modifying attributes
        for i in 0..10 {
            let ctx_clone = Arc::clone(&ctx);
            let handle = tokio::spawn(async move {
                ctx_clone
                    .add_attribute(format!("key{}", i), format!("value{}", i));
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all attributes were set correctly
        for i in 0..10 {
            assert_eq!(
                ctx.get_context_attribute(&format!("key{}", i)).unwrap(),
                format!("value{}", i)
            );
        }
    }

    #[test]
    fn test_error_context_chaining() {
        let err = ValidationError::EmptyInput("test".into())
            .with_context("validate")
            .with_input_size(0)
            .with_content_type("application/json");

        // Chain a new error while preserving context
        let chained = ProcessingError::StageFailed {
            stage: "process".into(),
            reason: "validation failed".into(),
        }
        .with_context("process");

        // Verify both contexts are preserved
        assert!(chained.to_string().contains("process"));
        assert!(chained.to_string().contains("validation failed"));
    }

    // Test WithContext implementation consistency
    #[test]
    fn test_with_context_implementations() {
        // Test for all error types
        let validation_err =
            ValidationError::EmptyInput("test".into()).with_context("validate");
        assert_eq!(validation_err.operation().unwrap(), "validate");

        let processing_err = ProcessingError::Timeout {
            operation: "test".into(),
            duration: Duration::from_secs(1),
        }
        .with_context("process");
        assert_eq!(processing_err.operation().unwrap(), "process");

        let conversion_err = ConversionError::DataLoss {
            reason: "test".into(),
        }
        .with_context("convert");
        assert_eq!(conversion_err.operation().unwrap(), "convert");

        // ... test other error types ...
    }

    #[test]
    fn test_attribute_edge_cases() {
        let err = ValidationError::EmptyInput("test".into())
            .with_context("test")
            .with_attributes([
                ("empty", ""),
                ("spaces", "   "),
                ("unicode", "🦀"),
            ]);

        assert_eq!(err.get_attribute("empty").unwrap(), "");
        assert_eq!(err.get_attribute("spaces").unwrap(), "   ");
        assert_eq!(err.get_attribute("unicode").unwrap(), "🦀");
    }

    #[test]
    fn test_resource_usage_calculations() {
        let err = ResourceError::MemoryLimitExceeded {
            requested: 150,
            limit: 100,
        }
        .with_context("allocate")
        .with_resource_usage(150, 100, "mb");

        assert_eq!(err.get_attribute("used_mb").unwrap(), "150");
        assert_eq!(err.get_attribute("total_mb").unwrap(), "100");
        assert_eq!(err.get_attribute("usage_percent_mb").unwrap(), "150.00");

        // Test zero division protection
        let err = ResourceError::MemoryLimitExceeded {
            requested: 100,
            limit: 0,
        }
        .with_context("allocate")
        .with_resource_usage(100, 0, "mb");

        // Should handle division by zero gracefully
        assert!(err
            .get_attribute("usage_percent_mb")
            .unwrap()
            .contains("inf"));
    }

    #[test]
    fn test_attributes_before_context() {
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "value")])
            .with_context("operation");

        assert_eq!(err.operation().unwrap(), "operation");
        assert_eq!(err.get_attribute("key").unwrap(), "value");
    }

    #[test]
    fn test_attributes_after_context() {
        let err = ValidationError::EmptyInput("test".into())
            .with_context("operation")
            .with_attributes([("key", "value")]);

        assert_eq!(err.operation().unwrap(), "operation");
        assert_eq!(err.get_attribute("key").unwrap(), "value");
    }

    #[test]
    fn test_multiple_attributes() {
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key1", "value1"), ("key2", "value2")])
            .with_context("operation")
            .with_attributes([
                ("key3", "value3"),
                ("key1", "updated"), // Overwrite existing
            ]);

        assert_eq!(err.get_attribute("key1").unwrap(), "updated");
        assert_eq!(err.get_attribute("key2").unwrap(), "value2");
        assert_eq!(err.get_attribute("key3").unwrap(), "value3");
    }

    #[test]
    fn test_error_conversion_preserves_context() {
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "value")])
            .with_context("operation");

        // Convert to different error types
        let domain_err: DomainError = err.into();
        assert_eq!(domain_err.operation().unwrap(), "operation");
        assert_eq!(domain_err.get_attribute("key").unwrap(), "value");
    }

    #[tokio::test]
    async fn test_concurrent_attribute_access() {
        use std::sync::Arc;

        let err = Arc::new(
            ValidationError::EmptyInput("test".into())
                .with_context("operation"),
        );

        let mut handles = vec![];
        for i in 0..10 {
            let err = Arc::clone(&err);
            let handle = tokio::spawn(async move {
                if let DomainError::WithContext(ctx) = &*err {
                    ctx.add_attribute(
                        format!("key{}", i),
                        format!("value{}", i),
                    );
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all attributes were set correctly
        for i in 0..10 {
            assert_eq!(
                err.get_attribute(&format!("key{}", i)).unwrap(),
                format!("value{}", i)
            );
        }
    }

    #[test]
    fn test_empty_attributes() {
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes(Vec::<(String, String)>::new()) // Empty attributes
            .with_context("operation");

        assert_eq!(err.operation().unwrap(), "operation");
    }

    #[test]
    fn test_duplicate_attributes() {
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "value1")])
            .with_context("operation")
            .with_attributes([("key", "value2")]);

        assert_eq!(err.get_attribute("key").unwrap(), "value2");
    }

    #[test]
    fn test_special_characters() {
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([
                ("key with spaces", "value"),
                ("unicode-key-🦀", "value"),
                ("", "empty key"), // Empty key
            ])
            .with_context("operation");

        assert_eq!(err.get_attribute("key with spaces").unwrap(), "value");
        assert_eq!(err.get_attribute("unicode-key-🦀").unwrap(), "value");
    }

    #[test]
    fn test_context_chaining() {
        // Basic chaining
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "value")])
            .with_context("first")
            .with_context("second"); // Should preserve attributes

        assert_eq!(err.operation().unwrap(), "second");
        assert_eq!(err.get_attribute("key").unwrap(), "value");

        // Multiple attributes and multiple chains
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key1", "value1"), ("key2", "value2")])
            .with_context("first")
            .with_attributes([("key3", "value3")])
            .with_context("second")
            .with_attributes([("key4", "value4")])
            .with_context("third");

        assert_eq!(err.operation().unwrap(), "third");
        assert_eq!(err.get_attribute("key1").unwrap(), "value1");
        assert_eq!(err.get_attribute("key2").unwrap(), "value2");
        assert_eq!(err.get_attribute("key3").unwrap(), "value3");
        assert_eq!(err.get_attribute("key4").unwrap(), "value4");

        // Overwriting attributes in chain
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "value1")])
            .with_context("first")
            .with_attributes([("key", "value2")])
            .with_context("second");

        assert_eq!(err.operation().unwrap(), "second");
        assert_eq!(err.get_attribute("key").unwrap(), "value2");

        // Mixed attribute setting
        let err = ValidationError::EmptyInput("test".into())
            .with_context("first")
            .with_attributes([("key1", "value1")])
            .with_context("second")
            .with_attributes([("key2", "value2")])
            .with_context("third");

        assert_eq!(err.operation().unwrap(), "third");
        assert_eq!(err.get_attribute("key1").unwrap(), "value1");
        assert_eq!(err.get_attribute("key2").unwrap(), "value2");

        // Complex chaining with error conversion
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("source", "validation")])
            .with_context("validate")
            .with_attributes([("stage", "processing")])
            .with_context("process");

        let processing_err = ProcessingError::StageFailed {
            stage: "final".into(),
            reason: "test".into(),
        }
        .with_attributes([("result", "failed")])
        .with_context("finalize");

        // Convert first error to processing error while preserving attributes
        let final_err = processing_err
            .with_attributes([("original_error", err.to_string())])
            .with_context("complete");

        assert_eq!(final_err.operation().unwrap(), "complete");
        assert_eq!(final_err.get_attribute("result").unwrap(), "failed");
        assert_eq!(final_err.get_attribute("original_error").is_some(), true);
    }

    #[test]
    fn test_attribute_precedence() {
        // Test that later attributes take precedence
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "first"), ("unique1", "value1")])
            .with_context("first")
            .with_attributes([("key", "second"), ("unique2", "value2")])
            .with_context("second");

        assert_eq!(err.get_attribute("key").unwrap(), "second");
        assert_eq!(err.get_attribute("unique1").unwrap(), "value1");
        assert_eq!(err.get_attribute("unique2").unwrap(), "value2");
    }

    #[test]
    fn test_empty_context_chaining() {
        // Test chaining with empty attributes
        let err = ValidationError::EmptyInput("test".into())
            .with_context("first")
            .with_attributes(Vec::<(String, String)>::new())
            .with_context("second");

        assert_eq!(err.operation().unwrap(), "second");

        // Test chaining with empty contexts
        let err = ValidationError::EmptyInput("test".into())
            .with_attributes([("key", "value")])
            .with_context("operation");

        assert_eq!(err.operation().unwrap(), "operation");
        assert_eq!(err.get_attribute("key").unwrap(), "value");
    }

    #[test]
    fn test_context_attribute_isolation() {
        // Test that attributes are properly isolated between different error
        // chains
        let err1 = ValidationError::EmptyInput("test1".into())
            .with_attributes([("key", "value1")])
            .with_context("op1");

        let err2 = ValidationError::EmptyInput("test2".into())
            .with_attributes([("key", "value2")])
            .with_context("op2");

        assert_eq!(err1.get_attribute("key").unwrap(), "value1");
        assert_eq!(err2.get_attribute("key").unwrap(), "value2");
    }
}
