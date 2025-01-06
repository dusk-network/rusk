use crate::http::domain::constants::payload::{
    MAX_BINARY_SIZE, MAX_GRAPHQL_SIZE, MAX_JSON_SIZE, MAX_TEXT_SIZE,
};
use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ValidationError, WithContext,
};
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::ValidationRule;
use async_graphql::parser::parse_query;
use async_graphql::parser::types::{
    DocumentOperations, ExecutableDocument, OperationType,
};
use serde_json::Value as JsonValue;

/// Configuration for payload size limits
#[derive(Debug)]
pub struct PayloadLimits {
    /// Maximum size for JSON payloads (default: 10MB)
    pub max_json_size: usize,
    /// Maximum size for binary payloads (default: 50MB)
    pub max_binary_size: usize,
    /// Maximum size for GraphQL queries (default: 1MB)
    pub max_graphql_size: usize,
    /// Maximum size for text payloads (default: 1MB)
    pub max_text_size: usize,
}

impl Default for PayloadLimits {
    fn default() -> Self {
        Self {
            max_json_size: MAX_JSON_SIZE,
            max_binary_size: MAX_BINARY_SIZE,
            max_graphql_size: MAX_GRAPHQL_SIZE,
            max_text_size: MAX_TEXT_SIZE,
        }
    }
}

/// Validates raw payloads based on their Content-Type.
///
/// Ensures that:
/// - Payload size is within limits for the given content type
/// - JSON payloads are syntactically valid
/// - GraphQL queries are syntactically valid
/// - Binary payloads have valid length
/// - Text payloads are valid UTF-8
#[derive(Debug)]
pub struct RawPayloadRule {
    limits: PayloadLimits,
}

impl RawPayloadRule {
    /// Creates new payload validator with default limits
    pub fn new() -> Self {
        Self {
            limits: PayloadLimits::default(),
        }
    }

    /// Creates new payload validator with custom limits
    pub fn with_limits(limits: PayloadLimits) -> Self {
        Self { limits }
    }

    /// Validates JSON payload
    fn validate_json(&self, data: &[u8]) -> Result<(), DomainError> {
        // Check size limit
        if data.len() > self.limits.max_json_size {
            return Err(ValidationError::DataLength {
                field: "json_payload".into(),
                expected: self.limits.max_json_size,
                actual: data.len(),
            }
            .with_context("raw_payload_validation")
            .with_stage("json_size")
            .with_input_size(data.len()));
        }

        // Validate JSON syntax
        match serde_json::from_slice::<JsonValue>(data) {
            Ok(_) => Ok(()),
            Err(e) => Err(ValidationError::InvalidFormat(format!(
                "Invalid JSON: {}",
                e
            ))
            .with_context("raw_payload_validation")
            .with_stage("json_syntax")
            .with_input_size(data.len())),
        }
    }

    /// Validates binary payload
    fn validate_binary(&self, data: &[u8]) -> Result<(), DomainError> {
        if data.len() > self.limits.max_binary_size {
            return Err(ValidationError::DataLength {
                field: "binary_payload".into(),
                expected: self.limits.max_binary_size,
                actual: data.len(),
            }
            .with_context("raw_payload_validation")
            .with_stage("binary_size")
            .with_input_size(data.len()));
        }
        Ok(())
    }

    /// Validates GraphQL query
    fn validate_graphql(&self, data: &[u8]) -> Result<(), DomainError> {
        // Check size limit
        if data.len() > self.limits.max_graphql_size {
            return Err(ValidationError::DataLength {
                field: "graphql_payload".into(),
                expected: self.limits.max_graphql_size,
                actual: data.len(),
            }
            .with_context("raw_payload_validation")
            .with_stage("graphql_size")
            .with_input_size(data.len()));
        }

        // Convert to string and validate UTF-8
        let query = String::from_utf8(data.to_vec()).map_err(|_| {
            ValidationError::InvalidFormat(
                "Invalid UTF-8 in GraphQL query".into(),
            )
            .with_context("raw_payload_validation")
            .with_stage("graphql_encoding")
            .with_input_size(data.len())
        })?;

        // Parse query
        let doc = parse_query(&query).map_err(|e| {
            ValidationError::InvalidFormat(format!(
                "Invalid GraphQL syntax: {}",
                e
            ))
            .with_context("raw_payload_validation")
            .with_stage("graphql_syntax")
            .with_input_size(data.len())
        })?;

        // Validate query structure
        self.validate_graphql_document(&doc).map_err(|e| {
            e.with_context("raw_payload_validation")
                .with_stage("graphql_structure")
                .with_input_size(data.len())
        })
    }

    /// Validates GraphQL document structure
    fn validate_graphql_document(
        &self,
        doc: &ExecutableDocument,
    ) -> Result<(), ValidationError> {
        let mut has_query = false;

        // Iterate over operations using the proper API
        for (_, op) in doc.operations.iter() {
            match op.node.ty {
                OperationType::Query => {
                    if has_query {
                        return Err(ValidationError::InvalidFormat(
                            "Multiple query operations are not allowed".into(),
                        ));
                    }
                    has_query = true;
                }
                OperationType::Mutation => {
                    return Err(ValidationError::InvalidFormat(
                        "Mutation operations are not allowed".into(),
                    ));
                }
                OperationType::Subscription => {
                    return Err(ValidationError::InvalidFormat(
                        "Subscription operations are not allowed".into(),
                    ));
                }
            }
        }

        // For DocumentOperations::Single, we'll get exactly one iteration
        // For DocumentOperations::Multiple, we'll iterate over all named
        // operations
        if !has_query {
            return Err(ValidationError::InvalidFormat(
                "Document must contain exactly one query operation".into(),
            ));
        }

        Ok(())
    }

    /// Validates text payload
    fn validate_text(&self, data: &[u8]) -> Result<(), DomainError> {
        // Check size limit
        if data.len() > self.limits.max_text_size {
            return Err(ValidationError::DataLength {
                field: "text_payload".into(),
                expected: self.limits.max_text_size,
                actual: data.len(),
            }
            .with_context("raw_payload_validation")
            .with_stage("text_size")
            .with_input_size(data.len()));
        }

        // Validate UTF-8
        String::from_utf8(data.to_vec()).map_err(|_| {
            ValidationError::InvalidFormat(
                "Invalid UTF-8 in text payload".into(),
            )
            .with_context("raw_payload_validation")
            .with_stage("text_encoding")
            .with_input_size(data.len())
        })?;

        Ok(())
    }
}

/// Input for payload validation containing content type and data
#[derive(Debug)]
pub struct RawPayload<'a> {
    /// Content type of the payload
    pub content_type: &'a str,
    /// Raw payload data
    pub data: &'a [u8],
}

impl ValidationRule<RawPayload<'_>> for RawPayloadRule {
    fn check(
        &self,
        payload: &RawPayload,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("raw_payload");

        let result = match payload.content_type {
            "application/json" => self.validate_json(payload.data),
            "application/octet-stream" => self.validate_binary(payload.data),
            "application/graphql" => self.validate_graphql(payload.data),
            "text/plain" => self.validate_text(payload.data),
            _ => Err(ValidationError::InvalidFieldValue {
                field: "Content-Type".into(),
                reason: format!(
                    "Unsupported content type: {}",
                    payload.content_type
                ),
            }
            .with_context("raw_payload_validation")
            .with_stage("content_type")
            .with_input_size(payload.data.len())),
        };

        ctx.complete_validation("raw_payload", &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    #[test]
    fn test_valid_json_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        let valid_json = r#"{"key": "value", "array": [1, 2, 3]}"#.as_bytes();
        let payload = RawPayload {
            content_type: "application/json",
            data: valid_json,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_invalid_json_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        let invalid_json = r#"{"key": "value", invalid}"#.as_bytes();
        let payload = RawPayload {
            content_type: "application/json",
            data: invalid_json,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(_))
            )
        ));
    }

    #[test]
    fn test_valid_binary_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        let binary = vec![1u8; 1024]; // 1KB binary data
        let payload = RawPayload {
            content_type: "application/octet-stream",
            data: &binary,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_invalid_graphql_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Syntactically invalid GraphQL
        let invalid_query = r#"
            query {
                incomplete query syntax {
                missing closing brace
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: invalid_query,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Invalid GraphQL syntax")
            )
        ));

        // Multiple operations
        let multiple_ops = r#"
            query First { field }
            query Second { field }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: multiple_ops,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Multiple query operations")
            )
        ));

        // Mutation operation
        let mutation = r#"
            mutation { updateField }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: mutation,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Mutation operations")
            )
        ));

        // Subscription operation
        let subscription = r#"
            subscription { onUpdate }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: subscription,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Subscription operations")
            )
        ));
    }

    #[test]
    fn test_valid_graphql_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Single anonymous query
        let anonymous_query = r#"
            {
                field
            }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: anonymous_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Single named query
        let named_query = r#"
            query GetField {
                field
            }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: named_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_graphql_operations() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Valid query
        let valid_query = r#"
                query GetBlockInfo {
                    block(height: 123) {
                        hash
                        height
                        timestamp
                    }
                }
            "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: valid_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Multiple queries
        let multiple_queries = r#"
                query First { block(height: 1) { hash } }
                query Second { block(height: 2) { hash } }
            "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: multiple_queries,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Multiple query operations")
            )
        ));

        // Mutation operation
        let mutation = r#"
                mutation UpdateBlock {
                    updateBlock(height: 123) {
                        hash
                    }
                }
            "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: mutation,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Mutation operations")
            )
        ));

        // Subscription operation
        let subscription = r#"
                subscription OnNewBlock {
                    newBlock {
                        hash
                    }
                }
            "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: subscription,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Subscription operations")
            )
        ));

        // Anonymous query
        let anonymous_query = r#"
                {
                    block(height: 123) {
                        hash
                        height
                    }
                }
            "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: anonymous_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Invalid syntax
        let invalid_syntax = r#"
                query GetBlockInfo {
                    block(height: 123) {
                        hash
                        height
                    # Missing closing brace
            "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: invalid_syntax,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Invalid GraphQL syntax")
            )
        ));
    }

    #[test]
    fn test_graphql_size_limits() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Create a valid query that repeats to reach size limit
        let base_query = "query { field }";
        let spaces = " ".repeat(50); // padding to help reach size limit
        let valid_query = format!("query {{\n{}\n}}", spaces);
        let query =
            valid_query.repeat(MAX_GRAPHQL_SIZE / valid_query.len() + 1);

        let payload = RawPayload {
            content_type: "application/graphql",
            data: query.as_bytes(),
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::DataLength { ref field, .. })
                if field == "graphql_payload"
            )
        ));

        // Test with a valid small query
        let small_query = "query { field }".as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: small_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_graphql_encoding() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Valid UTF-8 with unicode characters
        let unicode_query =
            r#"query { field(name: "Hello 世界 🦀") }"#.as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: unicode_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Invalid UTF-8
        let invalid_utf8 = &[0xFF, 0xFF];
        let payload = RawPayload {
            content_type: "application/graphql",
            data: invalid_utf8,
        };
        assert!(matches!(
            rule.check(&payload, &mut ctx).unwrap_err(),
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(ref msg))
                if msg.contains("Invalid UTF-8")
            )
        ));
    }

    #[test]
    fn test_graphql_query_structure() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Complex but valid query with fragments and variables
        let complex_query = r#"
            fragment BlockFields on Block {
                hash
                height
                timestamp
            }

            query GetBlockInfo($height: Int!) {
                block(height: $height) {
                    ...BlockFields
                    transactions {
                        hash
                    }
                }
            }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: complex_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Empty document
        let empty = "".as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: empty,
        };
        assert!(matches!(
            rule.check(&payload, &mut ctx).unwrap_err(),
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(_))
            )
        ));
    }

    #[test]
    fn test_graphql_edge_cases() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Query with maximum nesting
        let deeply_nested = r#"
            query {
                field1 {
                    field2 {
                        field3 {
                            field4 {
                                field5
                            }
                        }
                    }
                }
            }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: deeply_nested,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Query with comments
        let query_with_comments = r#"
            # This is a comment
            query {
                # Another comment
                field # Inline comment
            }
        "#
        .as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: query_with_comments,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());

        // Query with whitespace variations
        let whitespace_query =
            "\n\n\n  query   \n\n   {    \n  field   \n  }   \n\n".as_bytes();
        let payload = RawPayload {
            content_type: "application/graphql",
            data: whitespace_query,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_valid_text_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        let text = "Hello, world! 👋".as_bytes();
        let payload = RawPayload {
            content_type: "text/plain",
            data: text,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_invalid_text_payload() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        let invalid_utf8 = vec![0xFF, 0xFF]; // Invalid UTF-8
        let payload = RawPayload {
            content_type: "text/plain",
            data: &invalid_utf8,
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFormat(_))
            )
        ));
    }

    #[test]
    fn test_size_limits() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        // Test each content type with data exceeding its limit
        let test_cases = vec![
            (
                "application/json",
                vec![0; rule.limits.max_json_size + 1],
                "json_payload",
            ),
            (
                "application/octet-stream",
                vec![0; rule.limits.max_binary_size + 1],
                "binary_payload",
            ),
            (
                "application/graphql",
                vec![0; rule.limits.max_graphql_size + 1],
                "graphql_payload",
            ),
            (
                "text/plain",
                vec![0; rule.limits.max_text_size + 1],
                "text_payload",
            ),
        ];

        for (content_type, data, field_name) in test_cases {
            let payload = RawPayload {
                content_type,
                data: &data,
            };
            let err = rule.check(&payload, &mut ctx).unwrap_err();
            assert!(matches!(
                err,
                DomainError::WithContext(ctx) if matches!(
                    *ctx.source(),
                    DomainError::Validation(ValidationError::DataLength { ref field, .. })
                    if field == field_name
                )
            ));
        }
    }

    #[test]
    fn test_custom_limits() {
        let limits = PayloadLimits {
            max_json_size: 100,
            max_binary_size: 200,
            max_graphql_size: 300,
            max_text_size: 400,
        };
        let rule = RawPayloadRule::with_limits(limits);
        let mut ctx = setup_context();

        let data = vec![0; 150];

        // Should fail for JSON (limit 100)
        let payload = RawPayload {
            content_type: "application/json",
            data: &data,
        };
        assert!(rule.check(&payload, &mut ctx).is_err());

        // Should pass for binary (limit 200)
        let payload = RawPayload {
            content_type: "application/octet-stream",
            data: &data,
        };
        assert!(rule.check(&payload, &mut ctx).is_ok());
    }

    #[test]
    fn test_unsupported_content_type() {
        let rule = RawPayloadRule::new();
        let mut ctx = setup_context();

        let payload = RawPayload {
            content_type: "application/unknown",
            data: &[],
        };
        let err = rule.check(&payload, &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::InvalidFieldValue { ref field, .. })
                if field == "Content-Type"
            )
        ));
    }
}
