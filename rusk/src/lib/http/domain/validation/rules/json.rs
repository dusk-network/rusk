//! Validation rules for JSON data in RUES protocol.
//!
//! This module provides validation rules for JSON data according to the RUES
//! specification:
//! - Basic JSON format and structure validation
//! - RUES headers format validation
//! - Depth and complexity limits
//!
//! Note: Content-specific validation is handled at the application layer.
//!
//! # JSON Depth Calculation
//!
//! The depth of a JSON structure is calculated as follows:
//! - Base values (null, boolean, number, string) have depth 1
//! - Objects and arrays add 1 to their deepest member's depth
//!
//! Examples:
//! ```text
//! // Simple values - depth 1
//! 42                          // depth 1
//! "string"                    // depth 1
//! true                       // depth 1
//!
//! // Arrays add 1 level
//! [1, 2, 3]                  // depth 2 (array + values)
//! ["a", ["b"]]               // depth 3 (outer array + inner array + value)
//!
//! // Objects add 1 level
//! {"key": "value"}           // depth 2 (object + value)
//! {"a": {"b": "c"}}         // depth 3 (outer object + inner object + value)
//!
//! // Complex structure takes maximum depth
//! {
//!     "shallow": [1, 2],     // depth 2 (object + array + values)
//!     "deep": {              // starts at depth 2 (object)
//!         "x": [             // depth 3 (object)
//!             {              // depth 4 (array)
//!                 "y": "z"   // depth 5, 6 (object + value)
//!             }
//!         ]
//!     }
//! }                         // maximum depth is 6
//! ```
//!
//! # Examples
//!
//! Validating JSON format:
//! ```rust
//! use rusk::http::domain::{JsonFormatRule, ValidationRule, ValidationContext};
//! use serde_json::json;
//!
//! let rule = JsonFormatRule::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Valid JSON with acceptable depth
//! let valid_json = json!({
//!     "string": "value",
//!     "array": [1, 2, 3],
//!     "nested": {
//!         "object": {"key": "value"}
//!     }
//! });
//! assert!(rule.check(&valid_json, &mut ctx).is_ok());
//!
//! // Invalid: empty object
//! let empty_json = json!({});
//! assert!(rule.check(&empty_json, &mut ctx).is_err());
//!
//! // Invalid: too deep
//! let deep_json = json!({
//!     "level1": {
//!         "level2": {
//!             "level3": {
//!                 "level4": {
//!                     "level5": "too deep"
//!                 }
//!             }
//!         }
//!     }
//! });
//! let rule = JsonFormatRule::with_max_depth(4);
//! assert!(rule.check(&deep_json, &mut ctx).is_err());
//! ```
//!
//! Validating RUES headers:
//! ```rust
//! use rusk::http::domain::{JsonHeaderRule, ValidationRule, ValidationContext};
//! use serde_json::json;
//!
//! let rule = JsonHeaderRule::new();
//! let mut ctx = ValidationContext::new();
//!
//! // Valid RUES headers
//! let valid_headers = json!({
//!     "Content-Location": "/on/blocks/accepted",
//!     "Content-Type": "application/json",
//!     "Content-Length": 42
//! });
//! assert!(rule.check(&valid_headers, &mut ctx).is_ok());
//!
//! // Invalid: missing required field
//! let invalid_headers = json!({
//!     "Content-Type": "application/json",
//!     "Content-Length": 42
//!     // Missing Content-Location
//! });
//! assert!(rule.check(&invalid_headers, &mut ctx).is_err());
//! ```
//!
//! # Error Handling
//!
//! The validation rules return specific errors:
//! - `ValidationError::InvalidFormat` for structural issues
//! - `ValidationError::MissingField` for missing required headers
//! - `ValidationError::InvalidFieldValue` for wrong field types

use crate::http::domain::{
    DomainError, RuesHeaders, ValidationContext, ValidationError,
    ValidationRule,
};
use serde_json::Value as JsonValue;

/// Validates basic JSON format and structure.
///
/// Ensures that:
/// - JSON is syntactically valid
/// - Not empty (not null, empty object, or empty array)
/// - Within reasonable size and depth limits
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::{JsonFormatRule, ValidationRule, ValidationContext};
/// use serde_json::json;
///
/// let rule = JsonFormatRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid JSON
/// let valid_json = json!({
///     "key": "value",
///     "number": 42,
///     "array": [1, 2, 3]
/// });
/// assert!(rule.check(&valid_json, &mut ctx).is_ok());
///
/// // Invalid: empty object
/// let empty_json = json!({});
/// assert!(rule.check(&empty_json, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct JsonFormatRule {
    max_depth: usize,
}

impl JsonFormatRule {
    /// Creates a new JSON format validator with default settings.
    ///
    /// Default maximum depth is 32 levels.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{JsonFormatRule, ValidationRule};
    ///
    /// let validator = JsonFormatRule::new();
    /// ```
    pub fn new() -> Self {
        Self { max_depth: 32 }
    }

    /// Creates a new JSON format validator with custom maximum depth.
    ///
    /// # Arguments
    ///
    /// * `max_depth` - Maximum allowed depth of JSON structure
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{JsonFormatRule, ValidationRule};
    ///
    /// let validator = JsonFormatRule::with_max_depth(16);
    /// ```
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl ValidationRule<JsonValue> for JsonFormatRule {
    fn check(
        &self,
        json: &JsonValue,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("json_format");

        let result = match json {
            JsonValue::Null => {
                Err(ValidationError::InvalidFormat("JSON is null".into()))
            }
            JsonValue::Object(obj) if obj.is_empty() => {
                Err(ValidationError::InvalidFormat("Empty JSON object".into()))
            }
            JsonValue::Array(arr) if arr.is_empty() => {
                Err(ValidationError::InvalidFormat("Empty JSON array".into()))
            }
            _ => {
                let depth = calculate_depth(json);
                if depth > self.max_depth {
                    Err(ValidationError::InvalidFormat(format!(
                        "JSON depth {} exceeds maximum allowed depth {}",
                        depth, self.max_depth
                    )))
                } else {
                    Ok(())
                }
            }
        }
        .map_err(Into::into);

        ctx.complete_validation("json_format", &result);
        result
    }
}

/// Validates RUES message headers format.
///
/// Ensures headers contain all required fields with correct types:
/// - Content-Location: string
/// - Content-Type: string (valid MIME type)
/// - Content-Length: number
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::{JsonHeaderRule, ValidationRule, ValidationContext};
/// use serde_json::json;
///
/// let rule = JsonHeaderRule::new();
/// let mut ctx = ValidationContext::new();
///
/// // Valid headers
/// let valid_headers = json!({
///     "Content-Location": "/on/blocks/accepted",
///     "Content-Type": "application/json",
///     "Content-Length": 42
/// });
/// assert!(rule.check(&valid_headers, &mut ctx).is_ok());
///
/// // Invalid: missing required field
/// let invalid_headers = json!({
///     "Content-Type": "application/json",
///     "Content-Length": 42
/// });
/// assert!(rule.check(&invalid_headers, &mut ctx).is_err());
/// ```
#[derive(Debug, Default)]
pub struct JsonHeaderRule;

impl JsonHeaderRule {
    /// Creates a new header format validator.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::{JsonHeaderRule, ValidationRule};
    ///
    /// let validator = JsonHeaderRule::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<JsonValue> for JsonHeaderRule {
    fn check(
        &self,
        json: &JsonValue,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("json_headers");

        let result = if let JsonValue::Object(obj) = json {
            // Check required fields presence
            let has_location = obj.contains_key("Content-Location");
            let has_type = obj.contains_key("Content-Type");
            let has_length = obj.contains_key("Content-Length");

            if !has_location {
                Err(ValidationError::MissingField("Content-Location".into()))
            } else if !has_type {
                Err(ValidationError::MissingField("Content-Type".into()))
            } else if !has_length {
                Err(ValidationError::MissingField("Content-Length".into()))
            } else {
                // Validate field types
                let location = obj.get("Content-Location").unwrap();
                let content_type = obj.get("Content-Type").unwrap();
                let length = obj.get("Content-Length").unwrap();

                if !location.is_string() {
                    Err(ValidationError::InvalidFieldValue {
                        field: "Content-Location".into(),
                        reason: "Must be a string".into(),
                    })
                } else if !content_type.is_string() {
                    Err(ValidationError::InvalidFieldValue {
                        field: "Content-Type".into(),
                        reason: "Must be a string".into(),
                    })
                } else if !length.is_number() {
                    Err(ValidationError::InvalidFieldValue {
                        field: "Content-Length".into(),
                        reason: "Must be a number".into(),
                    })
                } else {
                    Ok(())
                }
            }
        } else {
            Err(ValidationError::InvalidFormat(
                "Headers must be a JSON object".into(),
            ))
        }
        .map_err(Into::into);

        ctx.complete_validation("json_headers", &result);
        result
    }
}

// Helper function to calculate JSON depth
fn calculate_depth(json: &JsonValue) -> usize {
    match json {
        JsonValue::Object(obj) => {
            1 + obj.values().map(calculate_depth).max().unwrap_or(0)
        }
        JsonValue::Array(arr) => {
            1 + arr.iter().map(calculate_depth).max().unwrap_or(0)
        }
        _ => 1, // Base values have depth 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    #[test]
    fn test_json_format_rule() {
        let rule = JsonFormatRule::new();
        let mut ctx = setup_context();

        // Positive cases
        let valid_cases = vec![
            json!({
                "string": "value",
                "number": 42,
                "boolean": true,
                "array": [1, 2, 3],
                "object": {"key": "value"}
            }),
            json!([1, 2, 3]),
            json!(42),
            json!("string"),
            json!(true),
        ];

        for (i, json) in valid_cases.iter().enumerate() {
            assert!(
                rule.check(json, &mut ctx).is_ok(),
                "Valid case {} should pass: {:?}",
                i,
                json
            );
        }

        // Negative cases
        let invalid_cases = vec![
            (json!(null), "JSON is null"),
            (json!({}), "Empty JSON object"),
            (json!([]), "Empty JSON array"),
        ];

        for (json, expected_error) in invalid_cases {
            let err = rule.check(&json, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::Validation(ValidationError::InvalidFormat(msg))
                if msg == expected_error
            ));
        }
    }

    #[test]
    fn test_json_format_depth() {
        let rule = JsonFormatRule::with_max_depth(3);
        let mut ctx = setup_context();

        // Valid - depth 3
        let valid_json = json!({            // depth 1
            "level1": {                     // depth 2
                "level2": "value"           // depth 3
            }
        });
        assert!(rule.check(&valid_json, &mut ctx).is_ok());

        // Invalid - depth 4
        let invalid_json = json!({          // depth 1
            "level1": {                     // depth 2
                "level2": {                 // depth 3
                    "level3": "value"       // depth 4
                }
            }
        });
        let err = rule.check(&invalid_json, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation(ValidationError::InvalidFormat(msg))
            if msg.contains("depth 4 exceeds maximum allowed depth 3")
        ));
    }

    #[test]
    fn test_json_header_rule() {
        let rule = JsonHeaderRule::new();
        let mut ctx = setup_context();

        // Valid headers
        let valid_headers = json!({
            "Content-Location": "/on/blocks/accepted",
            "Content-Type": "application/json",
            "Content-Length": 42
        });
        assert!(rule.check(&valid_headers, &mut ctx).is_ok());

        // Missing fields
        let missing_fields = vec![
            (
                json!({"Content-Type": "application/json", "Content-Length": 42}),
                "Content-Location",
            ),
            (
                json!({"Content-Location": "/path", "Content-Length": 42}),
                "Content-Type",
            ),
            (
                json!({"Content-Location": "/path", "Content-Type": "application/json"}),
                "Content-Length",
            ),
        ];

        for (headers, missing_field) in missing_fields {
            let err = rule.check(&headers, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::Validation(ValidationError::MissingField(field))
                if field == missing_field
            ));
        }

        // Wrong field types
        let wrong_types = vec![
            json!({
                "Content-Location": 42,  // should be string
                "Content-Type": "application/json",
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/path",
                "Content-Type": 42,  // should be string
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/path",
                "Content-Type": "application/json",
                "Content-Length": "42"  // should be number
            }),
        ];

        for headers in wrong_types {
            let err = rule.check(&headers, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::Validation(
                    ValidationError::InvalidFieldValue { .. }
                )
            ));
        }

        // Not an object
        let invalid_types =
            vec![json!([]), json!(42), json!("string"), json!(null)];

        for headers in invalid_types {
            let err = rule.check(&headers, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::Validation(ValidationError::InvalidFormat(msg))
                if msg == "Headers must be a JSON object"
            ));
        }
    }

    #[test]
    fn test_calculate_depth() {
        let test_cases = vec![
            // Base values have depth 1
            (json!(42), 1),
            (json!("string"), 1),
            (json!(true), 1),
            (json!(null), 1),
            // Arrays add 1 level to their deepest element
            (json!([1, 2, 3]), 2),    // depth 1 + 1 = 2
            (json!(["a", ["b"]]), 3), // depth 1 + 1 + 1 = 3
            // Objects add 1 level to their deepest value
            (json!({"key": "value"}), 2), // depth 1 + 1 = 2
            (json!({"a": {"b": "c"}}), 3), // depth 1 + 1 + 1 = 3
            // Multiple arrays - takes maximum depth
            (
                json!({                                // depth 1 +
                    "arr1": [1, 2, 3],                  // depth 2 = 3
                    "arr2": [                           // depth 2 +
                        [                               // depth 1 +
                            ["deepest"]                 // depth 1 = 5
                        ]
                    ],
                    "arr3": ["shallow"]                 // depth 2 = 3
                }),
                5,
            ), // max depth is 5
            // Multiple nested objects - takes maximum depth
            (
                json!({                                // depth 1 +
                    "obj1": {"a": "b"},                 // depth 2 = 3
                    "obj2": {                           // depth 2 +
                        "c": {                          // depth 1 +
                            "d": {"e": "f"}             // depth 1 = 5
                        }
                    },
                    "obj3": {"x": "y"}                  // depth 2 = 3
                }),
                5,
            ), // max depth is 5
            // Mixed nesting with multiple paths
            (
                json!({                                // depth 1
                    "path1": {                          // depth 2
                        "a": {"b": "c"}                 // depth 3, 4
                    },
                    "path2": [                          // depth 2
                        {"x": [                         // depth 3, 4
                            {"y": "z"}                  // depth 5, 6
                        ]}
                    ],
                    "path3": "shallow"                  // depth 2
                }),
                6,
            ), // max depth is 6
        ];

        for (json, expected_depth) in test_cases {
            assert_eq!(
                calculate_depth(&json),
                expected_depth,
                "Wrong depth for JSON: {:?}",
                json
            );
        }
    }
}
