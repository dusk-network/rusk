//! RUES protocol message headers.
//!
//! This module provides types for handling RUES message headers according to
//! the specification. Headers contain metadata about RUES messages including:
//!
//! # Required Fields
//! - `Content-Location`: Path in format `/on/\[target\]/\[topic\]`
//! - `Content-Type`: MIME type (`application/json` or
//!   `application/octet-stream`)
//! - `Content-Length`: Payload size in bytes
//!
//! # Optional Fields
//! - `Rusk-Session-Id`: 16-byte session identifier (required for some
//!   operations)
//! - `Rusk-Version`: Protocol version in semver format
//! - `Accept`: Accepted response content types
//! - Custom headers with "X-" prefix
//!
//! # Examples
//!
//! Basic headers:
//! ```rust
//! use rusk::http::domain::RuesHeaders;
//!
//! let headers = RuesHeaders::builder()
//!     .content_location("/on/blocks/accepted")
//!     .content_type("application/json")
//!     .content_length(42)
//!     .build()
//!     .unwrap();
//! ```
//!
//! Headers with session ID and version:
//! ```rust
//! use rusk::http::domain::{RuesHeaders, Version};
//! # use rusk::http::domain::testing;
//!
//! let headers = RuesHeaders::builder()
//!     .content_location("/on/blocks/accepted")
//!     .content_type("application/json")
//!     .content_length(42)
//!     .session_id(testing::create_test_session_id())
//!     .version(testing::create_release_version())
//!     .build()
//!     .unwrap();
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::http::domain::{DomainError, SerDeError, SessionId, Version};

/// Headers for RUES protocol messages.
///
/// Contains all necessary metadata for RUES messages including:
/// - Content location (path)
/// - Content type
/// - Content length
/// - Session ID (optional)
/// - Protocol version (optional)
/// - Accept header (optional)
/// - Custom headers
///
/// # Required Fields
///
/// - `Content-Location`: Path in format `/on/[target]/[topic]`
/// - `Content-Type`: MIME type (`application/json` or
///   `application/octet-stream`)
/// - `Content-Length`: Payload size in bytes
///
/// # Optional Fields
///
/// - `Rusk-Session-Id`: 16-byte session identifier (required for some
///   operations)
/// - `Rusk-Version`: Protocol version in semver format
/// - `Accept`: Accepted response content types
/// - Custom headers (with "X-" prefix)
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::RuesHeaders;
/// # use rusk::http::domain::testing;
///
/// // Basic headers
/// let basic_headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .build()
///     .unwrap();
///
/// // Headers with optional fields
/// let full_headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .session_id(testing::create_test_session_id())
///     .version(testing::create_release_version())
///     .accept("application/json")
///     .custom_header("X-Request-ID", "123")
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuesHeaders {
    /// Message path (/on/[target]/[topic])
    content_location: String,
    /// Content type (application/json | application/octet-stream)
    content_type: String,
    /// Content length in bytes
    content_length: usize,
    // Optional "Rusk-Session-Id" header's value
    session_id: Option<SessionId>,
    // Optional "Rusk-Version" header's value
    version: Option<Version>,
    // Optional "Accept" header's value
    accept: Option<String>,
    /// Optional custom headers
    custom: HashMap<String, String>,
}

impl RuesHeaders {
    /// Creates a new headers builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn builder() -> RuesHeadersBuilder {
        RuesHeadersBuilder::default()
    }

    /// Serializes headers to bytes in JSON format.
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - JSON-encoded headers
    /// * `Err(DomainError)` - If serialization fails
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .build()
    ///     .unwrap();
    ///
    /// let bytes = headers.to_bytes().unwrap();
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, DomainError> {
        serde_json::to_vec(self).map_err(|e| SerDeError::Json(e).into())
    }

    /// Deserializes headers from JSON bytes.
    ///
    /// # Arguments
    /// * `bytes` - JSON-encoded headers
    ///
    /// # Returns
    /// * `Ok(RuesHeaders)` - Parsed headers
    /// * `Err(DomainError)` - If data is invalid
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    ///
    /// # let header_bytes = vec![/* ... */];
    /// if let Ok(headers) = RuesHeaders::from_bytes(&header_bytes) {
    ///     println!("Location: {}", headers.content_location());
    /// }
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DomainError> {
        serde_json::from_slice(bytes).map_err(|e| SerDeError::Json(e).into())
    }

    // Getters

    /// Returns the content location (path) of this message.
    ///
    /// The content location follows the format `/on/\[target\]/\[topic\]`
    /// where target and topic identify the event destination.
    ///
    /// # Returns
    /// Reference to the content location string
    pub fn content_location(&self) -> &str {
        &self.content_location
    }

    /// Returns the content type of this message.
    ///
    /// Common content types:
    /// - "application/json" for JSON data
    /// - "application/octet-stream" for binary data
    /// - "application/graphql" for GraphQL queries
    /// - "text/plain" for plain text
    ///
    /// # Returns
    /// Reference to the content type string
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Returns the content length in bytes.
    ///
    /// This value represents the length of the message payload,
    /// not including headers.
    ///
    /// # Returns
    /// Content length in bytes
    pub fn content_length(&self) -> usize {
        self.content_length
    }

    /// Returns a reference to the custom headers map.
    ///
    /// Custom headers can be used to include additional metadata
    /// with the message.
    ///
    /// # Returns
    /// Reference to the custom headers map
    pub fn custom_headers(&self) -> &HashMap<String, String> {
        &self.custom
    }

    /// Returns the value of a specific custom header.
    ///
    /// # Arguments
    /// * `key` - The header name to lookup
    ///
    /// # Returns
    /// Some reference to the header value if it exists, None otherwise
    pub fn get_custom_header(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }

    /// Returns the session ID if present.
    ///
    /// The session ID is a 16-byte identifier required for certain operations
    /// like Subscribe, Unsubscribe, and Message operations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .session_id(testing::create_test_session_id())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(headers.session_id().is_some());
    /// ```
    pub fn session_id(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }

    /// Returns the protocol version if present.
    ///
    /// The version follows semantic versioning format
    /// (major.minor.patch[-pre]). Used for protocol compatibility checking.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    ///
    /// assert!(headers.version().is_some());
    /// ```
    pub fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    /// Returns the accept header if present.
    ///
    /// The accept header specifies which content types are acceptable
    /// for the response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .accept("application/json")
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(headers.accept(), Some("application/json"));
    /// ```
    pub fn accept(&self) -> Option<&str> {
        self.accept.as_ref().map(|s| s.as_str())
    }

    // Setters

    /// Sets the content length.
    ///
    /// This method is used internally by the EventBuilder to set the content
    /// length after the payload serialization in the `build` method.
    pub(crate) fn set_content_length(&mut self, length: usize) {
        self.content_length = length;
    }
}

/// Builder for RUES headers.
///
/// Provides a fluent interface for constructing `RuesHeaders` with proper
/// validation. All required fields must be set before calling `build()`.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::RuesHeaders;
///
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .custom_header("X-Custom", "value")
///     .build()
///     .unwrap();
/// ```
#[derive(Default)]
pub struct RuesHeadersBuilder {
    content_location: Option<String>,
    content_type: Option<String>,
    content_length: Option<usize>,
    session_id: Option<SessionId>,
    version: Option<Version>,
    accept: Option<String>,
    custom: HashMap<String, String>,
}

impl RuesHeadersBuilder {
    /// Sets the content location (path).
    ///
    /// # Arguments
    /// * `path` - RUES path in format `/on/[target]/[topic]`
    ///
    /// # Returns
    /// `Self` for method chaining
    pub fn content_location(mut self, path: impl Into<String>) -> Self {
        self.content_location = Some(path.into());
        self
    }

    /// Sets the content type.
    ///
    /// # Arguments
    /// * `content_type` - MIME type (e.g., "application/json")
    ///
    /// # Returns
    /// `Self` for method chaining
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Sets the content length.
    ///
    /// # Arguments
    /// * `length` - Content length in bytes
    ///
    /// # Returns
    /// `Self` for method chaining
    pub fn content_length(mut self, length: usize) -> Self {
        self.content_length = Some(length);
        self
    }

    /// Sets the session ID.
    ///
    /// The session ID is required for Subscribe, Unsubscribe, and Message
    /// operations. It must be a 16-byte identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The session identifier
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .session_id(testing::create_test_session_id())
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Sets the protocol version.
    ///
    /// The version is used for protocol compatibility checking.
    ///
    /// # Arguments
    ///
    /// * `version` - Protocol version in semver format
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    /// # use rusk::http::domain::testing;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .version(testing::create_release_version())
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn version(mut self, version: Version) -> Self {
        self.version = Some(version);
        self
    }

    /// Sets the accept header.
    ///
    /// Specifies which content types are acceptable for the response.
    ///
    /// # Arguments
    ///
    /// * `accept` - Acceptable content types
    ///
    /// # Returns
    ///
    /// `Self` for method chaining
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    ///
    /// let headers = RuesHeaders::builder()
    ///     .content_location("/on/blocks/accepted")
    ///     .content_type("application/json")
    ///     .content_length(42)
    ///     .accept("application/json")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn accept(mut self, accept: impl Into<String>) -> Self {
        self.accept = Some(accept.into());
        self
    }

    /// Adds a custom header.
    ///
    /// # Arguments
    /// * `key` - Header name
    /// * `value` - Header value
    ///
    /// # Returns
    /// `Self` for method chaining
    pub fn custom_header(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Builds the headers after validating required fields.
    ///
    /// # Returns
    /// * `Ok(RuesHeaders)` - Valid headers
    /// * `Err(DomainError)` - If required fields are missing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::RuesHeaders;
    ///
    /// let result = RuesHeaders::builder()
    ///     .content_type("application/json")
    ///     // Missing required fields
    ///     .build();
    ///
    /// assert!(result.is_err());
    /// ```
    pub fn build(self) -> Result<RuesHeaders, DomainError> {
        Ok(RuesHeaders {
            content_location: self.content_location.ok_or_else(|| {
                SerDeError::MissingField("content_location".to_string())
            })?,
            content_type: self.content_type.ok_or_else(|| {
                SerDeError::MissingField("content_type".to_string())
            })?,
            // If the content length is not set, default to 0
            content_length: self.content_length.unwrap_or_default(),
            session_id: self.session_id,
            version: self.version,
            accept: self.accept,
            custom: self.custom,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::http::domain::{testing, RuesValue};

    use super::*;
    use bytes::{BufMut, Bytes, BytesMut};
    use serde_json::json;

    #[test]
    fn test_headers_serialization() {
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .custom_header("X-Test", "value")
            .build()
            .unwrap();

        let bytes = headers.to_bytes().unwrap();
        let parsed = RuesHeaders::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.content_location, "/on/blocks/accepted");
        assert_eq!(parsed.content_type, "application/json");
        assert_eq!(parsed.content_length, 42);
        assert_eq!(parsed.custom.get("X-Test").unwrap(), "value");
    }

    #[test]
    fn test_graphql_handling() -> Result<(), DomainError> {
        let query = "{ query { test } }";
        let value = RuesValue::GraphQL(query.to_string());
        let serialized = value.to_bytes()?;

        let headers = RuesHeaders::builder()
            .content_location("/on/graphql/query")
            .content_type("application/graphql")
            .content_length(serialized.len())
            .build()?;

        // Create complete message
        let mut message = Vec::new();
        let headers_bytes = headers.to_bytes()?;
        message.extend_from_slice(&(headers_bytes.len() as u32).to_le_bytes());
        message.extend_from_slice(&headers_bytes);
        message.extend_from_slice(&serialized);

        // Deserialize
        let (parsed_headers, parsed_value) =
            RuesValue::from_message_bytes(&message)?;

        assert_eq!(parsed_headers.content_type, "application/graphql");
        assert_eq!(parsed_value, value);
        Ok(())
    }

    #[test]
    fn test_message_roundtrip() -> Result<(), DomainError> {
        let value = RuesValue::Json(json!({ "t": 1 }));
        let value_bytes = value.to_bytes()?;

        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(value_bytes.len())
            .build()?;

        // Create complete message
        let mut message = BytesMut::new();
        let headers_bytes = headers.to_bytes()?;
        message.put_u32_le(headers_bytes.len() as u32);
        message.extend_from_slice(&headers_bytes);
        message.extend_from_slice(&value_bytes);

        // Deserialize
        let (parsed_headers, parsed_value) =
            RuesValue::from_message_bytes(&message.freeze())?;

        // Verify
        assert_eq!(parsed_headers.content_location, headers.content_location);
        assert_eq!(parsed_headers.content_type, headers.content_type);
        assert_eq!(parsed_value, value);
        Ok(())
    }

    #[test]
    fn test_session_id_handling() {
        // Without session ID
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .build()
            .unwrap();
        assert!(headers.session_id().is_none());

        // With session ID
        let session_id = testing::create_test_session_id();
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .session_id(session_id.clone())
            .build()
            .unwrap();
        assert_eq!(headers.session_id(), Some(&session_id));

        // Serialize and deserialize
        let bytes = headers.to_bytes().unwrap();
        let parsed = RuesHeaders::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.session_id(), Some(&session_id));
    }

    #[test]
    fn test_version_handling() {
        // Without version
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .build()
            .unwrap();
        assert!(headers.version().is_none());

        // With version
        let version = testing::create_release_version();
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .version(version.clone())
            .build()
            .unwrap();
        assert_eq!(headers.version(), Some(&version));

        // Serialize and deserialize
        let bytes = headers.to_bytes().unwrap();
        let parsed = RuesHeaders::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.version(), Some(&version));
    }

    #[test]
    fn test_accept_handling() {
        // Without accept
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .build()
            .unwrap();
        assert!(headers.accept().is_none());

        // With accept
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .accept("application/json")
            .build()
            .unwrap();
        assert_eq!(headers.accept(), Some("application/json"));

        // Serialize and deserialize
        let bytes = headers.to_bytes().unwrap();
        let parsed = RuesHeaders::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.accept(), Some("application/json"));
    }

    #[test]
    fn test_headers_json_format() {
        let session_id = testing::create_test_session_id();
        let version = testing::create_release_version();

        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .session_id(session_id)
            .version(version)
            .accept("application/json")
            .custom_header("X-Request-ID", "123")
            .build()
            .unwrap();

        let json = serde_json::to_value(&headers).unwrap();

        // Verify JSON structure
        assert!(json.is_object());
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("content_location"));
        assert!(obj.contains_key("content_type"));
        assert!(obj.contains_key("content_length"));
        assert!(obj.contains_key("session_id"));
        assert!(obj.contains_key("version"));
        assert!(obj.contains_key("accept"));
        assert!(obj.contains_key("custom"));

        // Verify custom headers
        let custom = obj.get("custom").unwrap().as_object().unwrap();
        assert_eq!(custom.get("X-Request-ID").unwrap(), "123");
    }
}
