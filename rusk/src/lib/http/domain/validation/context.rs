use crate::http::domain::{DomainError, ProcessingMetrics};
use std::time::Duration;

/// Context for validation operations
#[derive(Debug)]
pub struct ValidationContext {
    metrics: ProcessingMetrics,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self {
            metrics: ProcessingMetrics::new(),
        }
    }

    pub fn metrics(&mut self) -> &mut ProcessingMetrics {
        &mut self.metrics
    }

    pub fn start_validation(&mut self, rule_type: &str) {
        self.metrics
            .start_operation(format!("validate_{}", rule_type));
    }

    pub fn complete_validation(
        &mut self,
        rule_type: &str,
        result: &Result<(), DomainError>,
    ) {
        match result {
            Ok(_) => {
                self.metrics.increment_counter("validations_success");
            }
            Err(e) => {
                self.metrics.record_error_with_context("validation", e);
            }
        }
        self.metrics
            .complete_operation(format!("validate_{}", rule_type));
    }

    pub fn record_validation_pressure(&mut self, pressure: f64) {
        self.metrics.update_pressure(pressure);
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::new()
    }
}
