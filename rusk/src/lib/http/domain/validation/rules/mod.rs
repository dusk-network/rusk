// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod binary;
pub mod core;
pub mod headers;
pub mod identifier;
pub mod json;
pub mod path;

use crate::http::domain::error::DomainError;
use crate::http::domain::validation::context::ValidationContext;
use crate::http::domain::validation::rules::core::{RuleBuilder, RuleSet};

/// Core validation rule trait
pub trait ValidationRule<T>: Send + Sync {
    fn check(
        &self,
        value: &T,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError>;
}

/// Extension methods for validation rules
pub trait ValidationRuleExt<T>: ValidationRule<T> {
    fn and<R>(self, other: R) -> RuleSet<T>
    where
        Self: Sized,
        R: ValidationRule<T>;

    fn or<R>(self, other: R) -> RuleSet<T>
    where
        Self: Sized,
        R: ValidationRule<T>;

    fn optional(self) -> RuleSet<T>
    where
        Self: Sized;
}
