use parking_lot::RwLock;
use std::marker::PhantomData;

use crate::http::domain::{DomainError, ValidationContext, ValidationRule};

pub struct RuleSet<T> {
    rules: Vec<Box<dyn ValidationRule<T>>>,
}

impl<T> RuleSet<T> {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule<R>(mut self, rule: R) -> Self
    where
        R: ValidationRule<T> + 'static,
    {
        self.rules.push(Box::new(rule));
        self
    }

    pub fn check(
        &self,
        value: &T,
        ctx: &mut ValidationContext,
    ) -> Result<(), DomainError> {
        for rule in &self.rules {
            rule.check(value, ctx)?;
        }
        Ok(())
    }
}

pub struct RuleBuilder<T> {
    rules: RuleSet<T>,
}

impl<T> RuleBuilder<T> {
    pub fn new() -> Self {
        Self {
            rules: RuleSet::new(),
        }
    }

    pub fn add_rule<R>(mut self, rule: R) -> Self
    where
        R: ValidationRule<T> + 'static,
    {
        self.rules = self.rules.add_rule(rule);
        self
    }

    pub fn build(self) -> RuleSet<T> {
        self.rules
    }
}
