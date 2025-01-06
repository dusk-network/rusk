use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ValidationError, WithContext,
};
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::ValidationRule;
use serde_json::Value as JsonValue;

/// Validates raw JSON headers before conversion to RuesHeaders.
///
/// Ensures that:
/// - JSON is a valid object (not null, array, or primitive)
/// - Required fields are present (Content-Location, Content-Type,
///   Content-Length)
/// - Field types are correct (strings for Location/Type, number for Length)
/// - Content-Location starts with "/on/"
/// - Content-Type is a valid MIME type
/// - Content-Length is non-negative
#[derive(Debug, Default)]
pub struct RawHeadersRule;

impl RawHeadersRule {
    /// Creates new raw headers validator
    pub fn new() -> Self {
        Self
    }

    /// Validates JSON structure is an object with required fields
    fn validate_structure(&self, json: &JsonValue) -> Result<(), DomainError> {
        let obj = match json {
            JsonValue::Object(obj) => obj,
            _ => {
                return Err(ValidationError::InvalidFormat(
                    "Headers must be a JSON object".into(),
                )
                .with_context("raw_headers_validation")
                .with_stage("structure")
                .with_input_size(json.to_string().len()));
            }
        };

        // Check required fields presence
        let required_fields =
            ["Content-Location", "Content-Type", "Content-Length"];
        for field in required_fields {
            if !obj.contains_key(field) {
                return Err(ValidationError::MissingField(field.into())
                    .with_context("raw_headers_validation")
                    .with_stage("structure"));
            }
        }

        Ok(())
    }

    /// Validates field types and formats
    fn validate_fields(&self, json: &JsonValue) -> Result<(), DomainError> {
        let obj = json.as_object().unwrap(); // Safe after structure validation

        // Validate Content-Location
        let location = obj.get("Content-Location").unwrap();
        if !location.is_string() {
            return Err(ValidationError::InvalidFieldValue {
                field: "Content-Location".into(),
                reason: "Must be a string".into(),
            }
            .with_context("raw_headers_validation")
            .with_stage("field_types"));
        }
        let location_str = location.as_str().unwrap();
        if !location_str.starts_with("/on/") {
            return Err(ValidationError::InvalidFieldValue {
                field: "Content-Location".into(),
                reason: "Must start with /on/".into(),
            }
            .with_context("raw_headers_validation")
            .with_stage("field_format"));
        }

        // Validate Content-Type
        let content_type = obj.get("Content-Type").unwrap();
        if !content_type.is_string() {
            return Err(ValidationError::InvalidFieldValue {
                field: "Content-Type".into(),
                reason: "Must be a string".into(),
            }
            .with_context("raw_headers_validation")
            .with_stage("field_types"));
        }
        let type_str = content_type.as_str().unwrap();
        if !matches!(
            type_str,
            "application/json"
                | "application/octet-stream"
                | "application/graphql"
                | "text/plain"
        ) {
            return Err(ValidationError::InvalidFieldValue {
                field: "Content-Type".into(),
                reason: "Invalid MIME type".into(),
            }
            .with_context("raw_headers_validation")
            .with_stage("field_format"));
        }

        // Validate Content-Length
        let length = obj.get("Content-Length").unwrap();
        if !length.is_number() {
            return Err(ValidationError::InvalidFieldValue {
                field: "Content-Length".into(),
                reason: "Must be a number".into(),
            }
            .with_context("raw_headers_validation")
            .with_stage("field_types"));
        }
        let length_num = length.as_u64().unwrap_or(u64::MAX);
        if length_num > i64::MAX as u64 {
            return Err(ValidationError::InvalidFieldValue {
                field: "Content-Length".into(),
                reason: "Value too large".into(),
            }
            .with_context("raw_headers_validation")
            .with_stage("field_format"));
        }

        Ok(())
    }
}

impl ValidationRule<JsonValue> for RawHeadersRule {
    fn check(
        &self,
        json: &JsonValue,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("raw_headers");

        let result = (|| {
            self.validate_structure(json)?;
            self.validate_fields(json)?;
            Ok(())
        })();

        ctx.complete_validation("raw_headers", &result);
        result
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
    fn test_valid_headers() {
        let rule = RawHeadersRule::new();
        let mut ctx = setup_context();

        let valid_cases = vec![
            json!({
                "Content-Location": "/on/blocks/accepted",
                "Content-Type": "application/json",
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/on/contracts/deploy",
                "Content-Type": "application/octet-stream",
                "Content-Length": 0
            }),
            json!({
                "Content-Location": "/on/graphql/query",
                "Content-Type": "application/graphql",
                "Content-Length": 100
            }),
            json!({
                "Content-Location": "/on/node/info",
                "Content-Type": "text/plain",
                "Content-Length": 1
            }),
        ];

        for (i, headers) in valid_cases.iter().enumerate() {
            assert!(
                rule.check(headers, &mut ctx).is_ok(),
                "Valid case {} should pass: {:?}",
                i,
                headers
            );
        }
    }

    #[test]
    fn test_invalid_structure() {
        let rule = RawHeadersRule::new();
        let mut ctx = setup_context();

        let invalid_cases = vec![
            (json!(null), "JSON null"),
            (json!([]), "JSON array"),
            (json!(42), "JSON number"),
            (json!("string"), "JSON string"),
            (json!(true), "JSON boolean"),
            (json!({}), "Empty object"),
            (
                json!({
                    "Content-Type": "application/json",
                    "Content-Length": 42
                }),
                "Missing Content-Location",
            ),
            (
                json!({
                    "Content-Location": "/on/blocks/accepted",
                    "Content-Length": 42
                }),
                "Missing Content-Type",
            ),
            (
                json!({
                    "Content-Location": "/on/blocks/accepted",
                    "Content-Type": "application/json"
                }),
                "Missing Content-Length",
            ),
        ];

        for (headers, case) in invalid_cases {
            let err = rule.check(&headers, &mut ctx).unwrap_err();
            assert!(
                matches!(err, DomainError::WithContext(_)),
                "Case '{}' should fail with context: {:?}",
                case,
                err
            );
        }
    }

    #[test]
    fn test_invalid_field_types() {
        let rule = RawHeadersRule::new();
        let mut ctx = setup_context();

        let invalid_cases = vec![
            json!({
                "Content-Location": 42,  // should be string
                "Content-Type": "application/json",
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/on/blocks/accepted",
                "Content-Type": 42,  // should be string
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/on/blocks/accepted",
                "Content-Type": "application/json",
                "Content-Length": "42"  // should be number
            }),
        ];

        for headers in invalid_cases {
            let err = rule.check(&headers, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::WithContext(ctx) if matches!(
                    *ctx.source(),
                    DomainError::Validation(ValidationError::InvalidFieldValue { .. })
                )
            ));
        }
    }

    #[test]
    fn test_invalid_field_formats() {
        let rule = RawHeadersRule::new();
        let mut ctx = setup_context();

        let invalid_cases = vec![
            json!({
                "Content-Location": "/invalid/path",  // missing /on/ prefix
                "Content-Type": "application/json",
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/on/blocks/accepted",
                "Content-Type": "invalid/type",  // invalid MIME type
                "Content-Length": 42
            }),
            json!({
                "Content-Location": "/on/blocks/accepted",
                "Content-Type": "application/json",
                "Content-Length": -1  // negative length
            }),
        ];

        for headers in invalid_cases {
            let err = rule.check(&headers, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::WithContext(ctx) if matches!(
                    *ctx.source(),
                    DomainError::Validation(ValidationError::InvalidFieldValue { .. })
                )
            ));
        }
    }

    #[test]
    fn test_metrics_recording() {
        let rule = RawHeadersRule::new();
        let mut ctx = setup_context();

        // Valid case
        let valid_headers = json!({
            "Content-Location": "/on/blocks/accepted",
            "Content-Type": "application/json",
            "Content-Length": 42
        });
        assert!(rule.check(&valid_headers, &mut ctx).is_ok());

        // Invalid case
        let invalid_headers = json!({});
        assert!(rule.check(&invalid_headers, &mut ctx).is_err());
    }
}
