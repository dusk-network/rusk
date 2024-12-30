mod binary;
mod core;
mod headers;
mod identifier;
mod json;
mod rues_path;

pub use core::{RuleBuilder, RuleSet};
pub use headers::{
    ContentLocationRule, ContentTypeRule, HeadersFormatRule, SessionHeaderRule,
    VersionHeaderRule,
};
pub use identifier::{
    BlockHashValidator, ContractIdValidator, SessionIdValidator,
    TransactionHashValidator,
};
pub use json::{JsonFormatRule, JsonHeaderRule};
pub use rues_path::{
    TargetIdRule, TargetTypeRule, TopicFormatRule, TopicValidityRule,
};

use crate::http::domain::{DomainError, ValidationContext};

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
