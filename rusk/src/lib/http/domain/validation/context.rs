// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::http::domain::error::DomainError;
use crate::http::domain::processing::metrics::ProcessingMetrics;

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
            Ok(()) => {
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
