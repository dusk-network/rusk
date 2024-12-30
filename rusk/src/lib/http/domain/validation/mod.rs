mod context;
mod rules;

use crate::http::domain::{DomainError, ProcessingMetrics};
pub use context::ValidationContext;
pub use rules::{
    BlockHashValidator, ContentLocationRule, ContentTypeRule,
    ContractIdValidator, HeadersFormatRule, JsonFormatRule, JsonHeaderRule,
    RuleBuilder, RuleSet, SessionHeaderRule, SessionIdValidator, TargetIdRule,
    TargetTypeRule, TopicFormatRule, TopicValidityRule,
    TransactionHashValidator, ValidationRule, ValidationRuleExt,
    VersionHeaderRule,
};

/// Core validation trait
#[async_trait::async_trait]
pub trait Validator<T>: Send + Sync {
    async fn validate(
        &self,
        value: &T,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError>;
}
