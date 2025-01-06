// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::http::domain::constants::binary;
use crate::http::domain::constants::binary::offset;
use crate::http::domain::constants::binary::size;
use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ValidationError, WithContext,
};
use crate::http::domain::types::event::Version;
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::ValidationRule;

/// Validates RUES binary file magic number
#[derive(Debug, Clone, Default)]
pub struct MagicNumberRule;

impl MagicNumberRule {
    /// Creates new magic number validator
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<&[u8]> for MagicNumberRule {
    fn check(
        &self,
        data: &&[u8],
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("magic_number");

        let result = if data.len() < size::MAGIC {
            let error = ValidationError::DataLength {
                field: "magic_number".into(),
                expected: size::MAGIC,
                actual: data.len(),
            }
            .with_context("binary_validation")
            .with_stage("magic_number")
            .with_input_size(data.len());
            // Create result
            let result = Err(error);
            // Pass reference to result for validation
            ctx.complete_validation("magic_number", &result);
            // Return the result
            result
        } else {
            let magic = match data[..size::MAGIC].try_into() {
                Ok(bytes) => u32::from_be_bytes(bytes),
                Err(_) => {
                    let error = ValidationError::DataLength {
                        field: "magic_number".into(),
                        expected: size::MAGIC,
                        actual: data.len(),
                    }
                    .with_context("binary_validation")
                    .with_stage("magic_number")
                    .with_input_size(data.len());
                    let result = Err(error);
                    ctx.complete_validation("magic_number", &result);
                    return result;
                }
            };

            if magic != binary::MAGIC_NUMBER {
                let error = ValidationError::BinaryFormat {
                    field: "magic_number".into(),
                    reason: format!(
                        "expected 0x{:08x}, got 0x{:08x}",
                        binary::MAGIC_NUMBER,
                        magic
                    ),
                }
                .with_context("binary_validation")
                .with_stage("magic_number")
                .with_input_size(data.len());
                let result = Err(error);
                ctx.complete_validation("magic_number", &result);
                result
            } else {
                let result = Ok(());
                ctx.complete_validation("magic_number", &result);
                result
            }
        };

        result
    }
}

/// Validates RUES binary file type field
#[derive(Debug, Clone)]
pub struct FileTypeRule {
    expected_type: u16,
}

impl FileTypeRule {
    /// Creates new file type validator
    pub fn new(expected_type: u16) -> Self {
        Self { expected_type }
    }
}

impl ValidationRule<&[u8]> for FileTypeRule {
    fn check(
        &self,
        data: &&[u8],
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("file_type");

        let result = if data.len() < offset::FILE_TYPE + size::FILE_TYPE {
            let error = ValidationError::DataLength {
                field: "file_type".into(),
                expected: offset::FILE_TYPE + size::FILE_TYPE,
                actual: data.len(),
            }
            .with_context("binary_validation")
            .with_stage("file_type")
            .with_input_size(data.len());
            let result = Err(error);
            ctx.complete_validation("file_type", &result);
            result
        } else {
            let file_type = match data
                [offset::FILE_TYPE..offset::FILE_TYPE + size::FILE_TYPE]
                .try_into()
            {
                Ok(bytes) => u16::from_le_bytes(bytes),
                Err(_) => {
                    let error = ValidationError::DataLength {
                        field: "file_type".into(),
                        expected: size::FILE_TYPE,
                        actual: data.len(),
                    }
                    .with_context("binary_validation")
                    .with_stage("file_type")
                    .with_input_size(data.len());
                    let result = Err(error);
                    ctx.complete_validation("file_type", &result);
                    return result;
                }
            };

            if file_type != self.expected_type {
                let error = ValidationError::BinaryFormat {
                    field: "file_type".into(),
                    reason: format!(
                        "expected 0x{:04x}, got 0x{:04x}",
                        self.expected_type, file_type
                    ),
                }
                .with_context("binary_validation")
                .with_stage("file_type")
                .with_input_size(data.len());
                let result = Err(error);
                ctx.complete_validation("file_type", &result);
                result
            } else {
                let result = Ok(());
                ctx.complete_validation("file_type", &result);
                result
            }
        };

        result
    }
}

/// Validates RUES binary version field structure
#[derive(Debug, Clone, Default)]
pub struct BinaryVersionRule;

impl BinaryVersionRule {
    /// Creates new binary version validator
    pub fn new() -> Self {
        Self
    }
}

impl ValidationRule<&[u8]> for BinaryVersionRule {
    fn check(
        &self,
        data: &&[u8],
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("binary_version");

        let result = if data.len() < offset::VERSION + size::VERSION {
            let error = ValidationError::DataLength {
                field: "version".into(),
                expected: offset::VERSION + size::VERSION,
                actual: data.len(),
            }
            .with_context("binary_validation")
            .with_stage("binary_version")
            .with_input_size(data.len());
            let result = Err(error);
            ctx.complete_validation("binary_version", &result);
            result
        } else {
            let version_bytes =
                &data[offset::VERSION..offset::VERSION + size::VERSION];
            let pre_release = version_bytes[3];

            if pre_release != 0 && (pre_release & 0x80) == 0 {
                let error = ValidationError::BinaryFormat {
                    field: "version".into(),
                    reason: "Invalid pre-release format".into(),
                }
                .with_context("binary_validation")
                .with_stage("binary_version")
                .with_input_size(data.len());
                let result = Err(error);
                ctx.complete_validation("binary_version", &result);
                result
            } else {
                let result = Ok(());
                ctx.complete_validation("binary_version", &result);
                result
            }
        };

        result
    }
}

/// Validates consistency between binary version and Version type
#[derive(Debug, Clone)]
pub struct VersionConsistencyRule {
    expected: Version,
}

impl VersionConsistencyRule {
    /// Creates new version consistency validator
    pub fn new(expected: Version) -> Self {
        Self { expected }
    }
}

impl ValidationRule<&[u8]> for VersionConsistencyRule {
    fn check(
        &self,
        data: &&[u8],
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        ctx.start_validation("version_consistency");

        let result = if data.len() < offset::VERSION + size::VERSION {
            let error = ValidationError::DataLength {
                field: "version".into(),
                expected: offset::VERSION + size::VERSION,
                actual: data.len(),
            }
            .with_context("binary_validation")
            .with_stage("version_consistency")
            .with_input_size(data.len());
            let result = Err(error);
            ctx.complete_validation("version_consistency", &result);
            result
        } else {
            let version_bytes =
                &data[offset::VERSION..offset::VERSION + size::VERSION];
            let major = version_bytes[0];
            let minor = version_bytes[1];
            let patch = version_bytes[2];
            let pre_release = if version_bytes[3] & 0x80 != 0 {
                Some(version_bytes[3] & 0x7f)
            } else {
                None
            };

            if major != self.expected.major()
                || minor != self.expected.minor()
                || patch != self.expected.patch()
                || pre_release != self.expected.pre_release()
            {
                let error = ValidationError::BinaryFormat {
                    field: "version".into(),
                    reason: format!(
                        "version mismatch: expected {}, got {}.{}.{}{}",
                        self.expected,
                        major,
                        minor,
                        patch,
                        pre_release
                            .map_or_else(String::new, |v| format!("-{}", v))
                    ),
                }
                .with_context("binary_validation")
                .with_stage("version_consistency")
                .with_input_size(data.len());
                let result = Err(error);
                ctx.complete_validation("version_consistency", &result);
                result
            } else {
                let result = Ok(());
                ctx.complete_validation("version_consistency", &result);
                result
            }
        };

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::domain::{constants::binary::file_type::*, testing};
    use proptest::prelude::*;

    fn setup_context() -> ValidationContext {
        ValidationContext::new()
    }

    // Generate valid header data directly
    fn make_valid_header() -> Vec<u8> {
        let mut header = Vec::with_capacity(size::HEADER);
        header.extend_from_slice(&binary::MAGIC_NUMBER.to_be_bytes());
        header.extend_from_slice(&SMART_CONTRACT.to_le_bytes());
        header.extend_from_slice(&[0u8; size::RESERVED]);
        header.extend_from_slice(&[1, 0, 0, 0]); // version 1.0.0
        header
    }

    proptest! {
        #[test]
        fn test_magic_number_validation(valid_header in Just(make_valid_header())) {
            let mut ctx = setup_context();
            let rule = MagicNumberRule::new();
            prop_assert!(rule.check(&&valid_header[..], &mut ctx).is_ok());
        }

        #[test]
        fn test_invalid_magic_number(wrong_magic in prop::array::uniform4(0u8..)) {
            prop_assume!(wrong_magic != binary::MAGIC_NUMBER.to_be_bytes());
            let mut header = make_valid_header();
            header[..4].copy_from_slice(&wrong_magic);

            let mut ctx = setup_context();
            let rule = MagicNumberRule::new();
            prop_assert!(rule.check(&&header[..], &mut ctx).is_err());
        }

        #[test]
        fn test_file_type_validation(file_type in prop::num::u16::ANY) {
            let mut header = make_valid_header();
            header[size::MAGIC..size::MAGIC + 2].copy_from_slice(&file_type.to_le_bytes());

            let mut ctx = setup_context();
            let rule = FileTypeRule::new(file_type);
            prop_assert!(rule.check(&&header[..], &mut ctx).is_ok());
        }

        #[test]
        fn test_version_format(
            major in 0u8..255,
            minor in 0u8..255,
            patch in 0u8..255,
            has_pre_release in prop::bool::ANY,
            pre_release_value in 0u8..127,
        ) {
            let mut header = make_valid_header();
            header[offset::VERSION] = major;
            header[offset::VERSION + 1] = minor;
            header[offset::VERSION + 2] = patch;
            header[offset::VERSION + 3] = if has_pre_release {
                pre_release_value | 0x80
            } else {
                0
            };

            let mut ctx = setup_context();
            let rule = BinaryVersionRule::new();
            prop_assert!(rule.check(&&header[..], &mut ctx).is_ok());
        }

        #[test]
        fn test_version_consistency(
            major in 0u8..255,
            minor in 0u8..255,
            patch in 0u8..255,
            has_pre_release in prop::bool::ANY,
            pre_release_value in 0u8..127,
        ) {
            let mut header = make_valid_header();
            header[offset::VERSION] = major;
            header[offset::VERSION + 1] = minor;
            header[offset::VERSION + 2] = patch;
            let pre_release_byte = if has_pre_release {
                pre_release_value | 0x80
            } else {
                0
            };
            header[offset::VERSION + 3] = pre_release_byte;

            let version = testing::create_test_version(
                major,
                minor,
                patch,
                if has_pre_release { Some(pre_release_value) } else { None }
            );

            let mut ctx = setup_context();
            let rule = VersionConsistencyRule::new(version);
            prop_assert!(rule.check(&&header[..], &mut ctx).is_ok());
        }
    }

    // Regular unit tests for edge cases...
}
