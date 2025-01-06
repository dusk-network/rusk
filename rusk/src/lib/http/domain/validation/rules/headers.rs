// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Validation rules for RUES protocol headers.
//!
//! RUES headers must conform to specific format requirements:
//! - Required fields: Content-Location, Content-Type, Content-Length
//! - Optional fields: Accept, Rusk-Version, Rusk-Session-Id
//! - Specific format and content requirements for each field

use crate::http::domain::error::{DomainError, ValidationError};
use crate::http::domain::types::event::Version;
use crate::http::domain::types::headers::RuesHeaders;
use crate::http::domain::types::identifier::SessionId;
use crate::http::domain::types::path::RuesPath;
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::ValidationRule;

/// Validates basic headers format and required fields.
///
/// Note: This rule is currently a placeholder for future validations. Most of
/// the basic format validation is already handled during headers construction:
/// - Required fields presence (`Content-Location`, `Content-Type`,
///   `Content-Length`)
/// - Field types validation
/// - Basic format validation
///
/// According to RUES specification:
/// - GET requests (Subscribe) can have zero content length (no payload)
/// - DELETE requests (Unsubscribe) can have zero content length (no payload)
/// - POST requests (Dispatch) must have content length matching payload size
///
/// Future validations might include:
/// - Field value length limits
/// - Character set restrictions
/// - Custom header name validation (e.g., must start with "X-")
/// - Header value format validation
/// - Protocol-specific constraints
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::headers::HeadersFormatRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// use rusk::http::domain::types::headers::RuesHeaders;
///
/// let rule = HeadersFormatRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Headers with zero content length (valid for GET/DELETE)
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(0)
///     .build()
///     .unwrap();
/// assert!(rule.check(&headers, &mut ctx).is_ok());
///
/// // Headers with payload (valid for POST)
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .build()
///     .unwrap();
/// assert!(rule.check(&headers, &mut ctx).is_ok());
/// ```
#[derive(Debug, Default)]
pub struct HeadersFormatRule;

impl HeadersFormatRule {
    /// Creates a new headers format validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::headers::HeadersFormatRule;
    /// use rusk::http::domain::validation::rules::ValidationRule;
    ///
    /// let validator = HeadersFormatRule::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<RuesHeaders> for HeadersFormatRule {
    fn check(
        &self,
        _headers: &RuesHeaders,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("headers_format");
        let result = Ok(());
        ctx.complete_validation("headers_format", &result);
        result
    }
}

/// Validates Content-Type header value.
///
/// Ensures Content-Type is one of:
/// - application/json
/// - application/octet-stream
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::headers::ContentTypeRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// use rusk::http::domain::types::headers::RuesHeaders;
///
/// let rule = ContentTypeRule::new();
/// let mut ctx = ValidationContext::new();
///
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .build()
///     .unwrap();
///
/// assert!(rule.check(&headers, &mut ctx).is_ok());
/// ```
#[derive(Debug, Default)]
pub struct ContentTypeRule;

impl ContentTypeRule {
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<RuesHeaders> for ContentTypeRule {
    fn check(
        &self,
        headers: &RuesHeaders,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("content_type");
        let result = match headers.content_type() {
            "application/json" | "application/octet-stream" => Ok(()),
            _ => Err(ValidationError::InvalidFieldValue {
                field: "Content-Type".into(),
                reason: "Content-Type must be application/json or application/octet-stream".into(),
            }.into())
        };
        ctx.complete_validation("content_type", &result);
        result
    }
}

/// Validates Rusk-Session-Id header when required.
///
/// Session ID is required for:
/// - Subscribe operations
/// - Unsubscribe operations
/// - Message operations
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::headers::SessionHeaderRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// use rusk::http::domain::types::headers::RuesHeaders;
/// # use rusk::http::domain::testing;
///
/// let rule = SessionHeaderRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid session ID
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .session_id(testing::create_test_session_id())
///     .build()
///     .unwrap();
///
/// assert!(rule.check(&headers, &mut ctx).is_ok());
/// ```
#[derive(Debug, Default)]
pub struct SessionHeaderRule {
    required: bool,
}

impl SessionHeaderRule {
    /// Creates a new session header validator that treats session ID as
    /// optional.
    pub fn new() -> Self {
        Self { required: false }
    }

    /// Creates a new session header validator that requires session ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::headers::SessionHeaderRule;
    /// use rusk::http::domain::validation::rules::ValidationRule;
    ///
    /// let validator = SessionHeaderRule::required();
    /// ```
    pub fn required() -> Self {
        Self { required: true }
    }
}

impl ValidationRule<RuesHeaders> for SessionHeaderRule {
    fn check(
        &self,
        headers: &RuesHeaders,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("session_header");
        let result = match headers.session_id() {
            Some(_) => Ok(()),
            None if self.required => {
                Err(ValidationError::MissingField("Rusk-Session-Id".into())
                    .into())
            }
            None => Ok(()),
        };
        ctx.complete_validation("session_header", &result);
        result
    }
}

/// Validates Rusk-Version header value.
///
/// Ensures that:
/// - If present, version is in valid semver format (handled by Version type)
/// - Version is compatible with RUES protocol version
///
/// The RUES protocol version follows semantic versioning rules:
/// - Major version changes indicate incompatible protocol changes
/// - Minor version changes indicate backward-compatible protocol additions
/// - Patch version changes indicate backward-compatible bug fixes
/// - Pre-release versions are not compatible with release versions
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::headers::VersionHeaderRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// use rusk::http::domain::types::headers::RuesHeaders;
/// # use rusk::http::domain::testing;
///
/// // Protocol version 1.0.0
/// let rule = VersionHeaderRule::new(testing::create_release_version());
/// let mut ctx = ValidationContext::new();
///
/// // Headers with compatible version (1.1.0 is compatible with 1.0.0)
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .version(testing::create_test_version(1, 1, 0, None))
///     .build()
///     .unwrap();
/// assert!(rule.check(&headers, &mut ctx).is_ok());
/// ```
#[derive(Debug)]
pub struct VersionHeaderRule {
    protocol_version: Version,
}

impl VersionHeaderRule {
    /// Creates a new version header validator with specified protocol version.
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - The RUES protocol version to check compatibility
    ///   against
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::validation::rules::headers::VersionHeaderRule;
    /// use rusk::http::domain::validation::rules::ValidationRule;
    /// # use rusk::http::domain::testing;
    ///
    /// let validator = VersionHeaderRule::new(testing::create_release_version());
    /// ```
    pub fn new(protocol_version: Version) -> Self {
        Self { protocol_version }
    }
}

impl ValidationRule<RuesHeaders> for VersionHeaderRule {
    fn check(
        &self,
        headers: &RuesHeaders,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("version_header");

        let result = if let Some(client_version) = headers.version() {
            if !client_version.is_compatible_with(&self.protocol_version) {
                Err(ValidationError::InvalidFieldValue {
                    field: "Rusk-Version".into(),
                    reason: format!(
                        "Client version {} is not compatible with RUES protocol version {}",
                        client_version,
                        self.protocol_version
                    ),
                }.into())
            } else {
                Ok(())
            }
        } else {
            Ok(()) // Version header is optional
        };

        ctx.complete_validation("version_header", &result);
        result
    }
}

/// Validates Content-Location header value.
///
/// Ensures that the path follows RUES format:
/// - Must start with "/on/"
/// - Modern path: `/on/[target]/[topic]` or `/on/[target]:[id]/[topic]`
/// - Legacy path: `/on/host:[data]/[topic]` or `/on/debugger:[data]/[topic]`
/// - No empty segments
/// - No extra slashes
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::validation::rules::headers::ContentLocationRule;
/// use rusk::http::domain::validation::rules::ValidationRule;
/// use rusk::http::domain::validation::context::ValidationContext;
/// use rusk::http::domain::types::headers::RuesHeaders;
///
/// let rule = ContentLocationRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid modern path
/// let headers = RuesHeaders::builder()
///     .content_location("/on/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .build()
///     .unwrap();
/// assert!(rule.check(&headers, &mut ctx).is_ok());
///
/// // Invalid path (missing /on/ prefix)
/// let headers = RuesHeaders::builder()
///     .content_location("/blocks/accepted")
///     .content_type("application/json")
///     .content_length(42)
///     .build()
///     .unwrap();
/// assert!(rule.check(&headers, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct ContentLocationRule;

impl ContentLocationRule {
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<RuesHeaders> for ContentLocationRule {
    fn check(
        &self,
        headers: &RuesHeaders,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("content_location");
        let path = headers.content_location();
        let result = if !path.starts_with("/on/") {
            Err(ValidationError::InvalidFormat(
                "Path must start with /on/".into(),
            )
            .into())
        } else {
            let segments: Vec<&str> = path[1..].split('/').collect();
            match segments.as_slice() {
                ["on", target, topic]
                    if !target.is_empty() && !topic.is_empty() =>
                {
                    Ok(())
                }
                _ => Err(ValidationError::InvalidFormat(
                    "Path must have format /on/[target]/[topic]".into(),
                )
                .into()),
            }
        };
        ctx.complete_validation("content_location", &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::domain::testing;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    #[test]
    fn test_session_header_optional() {
        let rule = SessionHeaderRule::new();
        let mut ctx = setup_context();

        // Valid with session ID
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .session_id(testing::create_test_session_id())
            .build()
            .unwrap();
        assert!(rule.check(&headers, &mut ctx).is_ok());

        // Valid without session ID
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .build()
            .unwrap();
        assert!(rule.check(&headers, &mut ctx).is_ok());
    }

    #[test]
    fn test_session_header_required() {
        let rule = SessionHeaderRule::required();
        let mut ctx = setup_context();

        // Valid with session ID
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .session_id(testing::create_test_session_id())
            .build()
            .unwrap();
        assert!(rule.check(&headers, &mut ctx).is_ok());

        // Invalid without session ID
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .build()
            .unwrap();
        assert!(matches!(
            rule.check(&headers, &mut ctx),
            Err(DomainError::Validation(ValidationError::MissingField(field)))
            if field == "Rusk-Session-Id"
        ));
    }

    #[test]
    fn test_version_header_validation() {
        let server_version = testing::create_release_version(); // 1.0.0
        let rule = VersionHeaderRule::new(server_version);
        let mut ctx = setup_context();

        // Test without version (valid)
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .build()
            .unwrap();
        assert!(rule.check(&headers, &mut ctx).is_ok());

        // Test with compatible version (valid)
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .version(testing::create_test_version(1, 1, 0, None)) // 1.1.0 compatible with 1.0.0
            .build()
            .unwrap();
        assert!(rule.check(&headers, &mut ctx).is_ok());

        // Test with incompatible version
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .version(testing::create_test_version(2, 0, 0, None)) // 2.0.0 not compatible with 1.0.0
            .build()
            .unwrap();
        let err = rule.check(&headers, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, .. })
            if field == "Rusk-Version"
        ));

        // Test with pre-release version
        let headers = RuesHeaders::builder()
            .content_location("/on/blocks/accepted")
            .content_type("application/json")
            .content_length(42)
            .version(testing::create_test_version(1, 0, 0, Some(1))) // 1.0.0-1 not compatible with 1.0.0
            .build()
            .unwrap();
        let err = rule.check(&headers, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFieldValue { field, .. })
            if field == "Rusk-Version"
        ));
    }

    #[test]
    fn test_content_location_format() {
        let rule = ContentLocationRule::new();
        let mut ctx = setup_context();

        // Valid paths
        let valid_paths = vec![
            "/on/blocks/accepted",
            "/on/blocks:1234abcd/accepted",
            "/on/transactions/executed",
            "/on/contracts/deploy",
            "/on/node/info",
            "/on/network/peers",
            "/on/graphql/query",
            "/on/host:system/event",
            "/on/debugger:trace/log",
        ];

        for path in valid_paths {
            let headers = RuesHeaders::builder()
                .content_location(path)
                .content_type("application/json")
                .content_length(42)
                .build()
                .unwrap();
            assert!(
                rule.check(&headers, &mut ctx).is_ok(),
                "Path should be valid: {}",
                path
            );
        }

        // Invalid paths
        let invalid_paths = vec![
            "/blocks/accepted",       // Missing /on/ prefix
            "on/blocks/accepted",     // Missing leading slash
            "/on//accepted",          // Empty target
            "/on/blocks/",            // Empty topic
            "/on/",                   // Missing target and topic
            "/on",                    // Missing slash and target and topic
            "/on/blocks/topic/extra", // Extra segments
            "invalid",                // Completely wrong format
        ];

        for path in invalid_paths {
            let headers = RuesHeaders::builder()
                .content_location(path)
                .content_type("application/json")
                .content_length(42)
                .build()
                .unwrap();
            assert!(
                rule.check(&headers, &mut ctx).is_err(),
                "Path should be invalid: {}",
                path
            );
        }
    }
}
