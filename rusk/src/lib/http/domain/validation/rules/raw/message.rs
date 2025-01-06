// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::http::domain::constants::message::MAX_HEADER_SIZE;
use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ValidationError, WithContext,
};
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::ValidationRule;

/// Validates raw WebSocket message structure according to RUES specification.
///
/// The message format is:
/// ```text
/// [4 bytes] Header length (u32 LE)
/// [N bytes] JSON headers
/// [M bytes] Event data
/// ```
///
/// Ensures that:
/// - Message has minimum required length (4 bytes for header length)
/// - Header length field is valid
/// - Total message size matches header length + payload
#[derive(Debug, Default)]
pub struct RawMessageRule;

impl RawMessageRule {
    /// Creates new raw message validator
    pub fn new() -> Self {
        Self
    }

    /// Validates header length field
    fn validate_header_length(&self, data: &[u8]) -> Result<u32, DomainError> {
        if data.len() < 4 {
            return Err(ValidationError::DataLength {
                field: "message_header".into(),
                expected: 4,
                actual: data.len(),
            }
            .with_context("raw_message_validation")
            .with_stage("header_length")
            .with_input_size(data.len()));
        }

        let header_len =
            u32::from_le_bytes(data[..4].try_into().map_err(|_| {
                ValidationError::BinaryFormat {
                    field: "message_header".into(),
                    reason: "Invalid header length bytes".into(),
                }
                .with_context("raw_message_validation")
                .with_stage("header_length")
                .with_input_size(data.len())
            })?);

        if header_len == 0 {
            return Err(ValidationError::DataLength {
                field: "message_header".into(),
                expected: 1,
                actual: 0,
            }
            .with_context("raw_message_validation")
            .with_stage("header_length")
            .with_input_size(data.len()));
        }

        if header_len > MAX_HEADER_SIZE {
            return Err(ValidationError::DataLength {
                field: "message_header".into(),
                expected: MAX_HEADER_SIZE as usize,
                actual: header_len as usize,
            }
            .with_context("raw_message_validation")
            .with_stage("header_length")
            .with_input_size(data.len()));
        }

        Ok(header_len)
    }

    /// Validates total message size
    fn validate_message_size(
        &self,
        data: &[u8],
        header_len: u32,
    ) -> Result<(), DomainError> {
        let required_len = 4 + header_len as usize;
        if data.len() < required_len {
            return Err(ValidationError::DataLength {
                field: "message_header".into(),
                expected: required_len,
                actual: data.len(),
            }
            .with_context("raw_message_validation")
            .with_stage("message_size")
            .with_input_size(data.len()));
        }
        Ok(())
    }
}

impl ValidationRule<&[u8]> for RawMessageRule {
    fn check(
        &self,
        data: &&[u8],
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("raw_message");

        let result = (|| {
            let header_len = self.validate_header_length(data)?;
            self.validate_message_size(data, header_len)?;
            Ok(())
        })();

        ctx.complete_validation("raw_message", &result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    fn create_test_message(header_len: u32, total_len: usize) -> Vec<u8> {
        let mut msg = Vec::with_capacity(total_len);
        msg.extend_from_slice(&header_len.to_le_bytes());
        msg.extend_from_slice(&vec![0u8; total_len - 4]);
        msg
    }

    #[test]
    fn test_valid_message() {
        let rule = RawMessageRule::new();
        let mut ctx = setup_context();

        // Valid message with minimal headers
        let msg = create_test_message(10, 14);
        assert!(rule.check(&&msg[..], &mut ctx).is_ok());

        // Valid message with larger headers
        let msg = create_test_message(100, 104);
        assert!(rule.check(&&msg[..], &mut ctx).is_ok());
    }

    #[test]
    fn test_invalid_message_length() {
        let rule = RawMessageRule::new();
        let mut ctx = setup_context();

        // Too short for header length
        let msg = vec![0u8; 3];
        let err = rule.check(&&msg[..], &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::DataLength { ref field, .. })
                if field == "message_header"
            )
        ));

        // Too short for headers
        let msg = create_test_message(100, 50);
        let err = rule.check(&&msg[..], &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::DataLength { ref field, .. })
                if field == "message_header"
            )
        ));
    }

    #[test]
    fn test_invalid_header_length() {
        let rule = RawMessageRule::new();
        let mut ctx = setup_context();

        // Zero header length
        let msg = create_test_message(0, 4);
        let err = rule.check(&&msg[..], &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::DataLength { ref field, .. })
                if field == "message_header"
            )
        ));

        // Header length too large
        let msg = create_test_message(MAX_HEADER_SIZE + 1, 4); // Using constant here
        let err = rule.check(&&msg[..], &mut ctx).unwrap_err();
        assert!(matches!(
            err,
            DomainError::WithContext(ctx) if matches!(
                *ctx.source(),
                DomainError::Validation(ValidationError::DataLength { ref field, .. })
                if field == "message_header"
            )
        ));
    }

    #[test]
    fn test_metrics_recording() {
        let rule = RawMessageRule::new();
        let mut ctx = setup_context();

        let msg = create_test_message(10, 14);
        assert!(rule.check(&&msg[..], &mut ctx).is_ok());

        let msg = vec![0u8; 3];
        assert!(rule.check(&&msg[..], &mut ctx).is_err());
    }

    #[test]
    fn test_error_context() {
        let rule = RawMessageRule::new();
        let mut ctx = setup_context();

        // Test header length error context
        let msg = vec![0u8; 3];
        let err = rule.check(&&msg[..], &mut ctx).unwrap_err();
        assert_eq!(err.operation().unwrap(), "raw_message_validation");
        assert_eq!(err.get_attribute("stage").unwrap(), "header_length");
        assert_eq!(err.get_attribute("input_size").unwrap(), "3");

        // Test message size error context
        let msg = create_test_message(100, 50);
        let err = rule.check(&&msg[..], &mut ctx).unwrap_err();
        assert_eq!(err.operation().unwrap(), "raw_message_validation");
        assert_eq!(err.get_attribute("stage").unwrap(), "message_size");
        assert_eq!(err.get_attribute("input_size").unwrap(), "50");
    }
}
