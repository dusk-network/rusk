// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Flow control mechanisms for processing operations.
//!
//! This module provides implementations for controlling processing flow and
//! backpressure in the RUES system. It includes:
//!
//! - Trait definition for flow control mechanisms
//! - No-op implementation for scenarios not requiring flow control
//! - Token bucket implementation for rate limiting
//! - Thread-safe operation guarantees
//!
//! # Flow Control Strategies
//!
//! ## No-op Flow
//! When backpressure isn't needed:
//! ```rust
//! use rusk::http::domain::processing::flow::{FlowControl, NoopFlow};
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//!
//! let mut flow = NoopFlow::default();
//! let mut metrics = ProcessingMetrics::new();
//!
//! // Always allows processing
//! assert!(flow.can_proceed());
//!
//! // Updates metrics but doesn't affect flow
//! flow.update(&mut metrics);
//!
//! // Always reports zero pressure
//! assert_eq!(flow.pressure(), 0.0);
//! ```
//!
//! ## Token Bucket Flow
//! For rate-limited processing:
//! ```rust
//! use rusk::http::domain::processing::flow::{FlowControl, TokenBucketFlow};
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut flow = TokenBucketFlow::builder()
//!     .capacity(100)
//!     .refill_rate(10.0) // 10 tokens per second
//!     .build();
//!
//! let mut metrics = ProcessingMetrics::new();
//!
//! // Check if processing can proceed
//! if flow.can_proceed() {
//!     // Process item
//!     flow.update(&mut metrics);
//! }
//!
//! // Or with explicit token acquisition
//! flow.acquire_one().await?;
//! flow.update(&mut metrics);
//! # Ok(())
//! # }
//! ```
//!
//! # Thread Safety
//!
//! All flow control implementations are thread-safe and can be shared between
//! processors:
//!
//! ```rust
//! use rusk::http::domain::processing::flow::{FlowControl, TokenBucketFlow};
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let flow = Arc::new(Mutex::new(
//!     TokenBucketFlow::builder()
//!         .capacity(100)
//!         .refill_rate(10.0)
//!         .build()
//! ));
//!
//! let flow_clone = Arc::clone(&flow);
//! let metrics = Arc::new(Mutex::new(ProcessingMetrics::new()));
//! let metrics_clone = Arc::clone(&metrics);
//!
//! // Use in separate task
//! tokio::spawn(async move {
//!     let mut flow = flow_clone.lock().await;
//!     let mut metrics = metrics_clone.lock().await;
//!
//!     if flow.can_proceed() {
//!         flow.update(&mut metrics);
//!     }
//! });
//! # Ok(())
//! # }
//! ```
//!
//! # Custom Flow Control
//!
//! Implementing custom flow control:
//! ```rust
//! use rusk::http::domain::processing::flow::FlowControl;
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//! use std::sync::atomic::{AtomicUsize, Ordering};
//!
//! #[derive(Debug)]
//! struct ConnectionFlow {
//!     current: AtomicUsize,
//!     limit: usize,
//! }
//!
//! impl ConnectionFlow {
//!     fn new(limit: usize) -> Self {
//!         Self {
//!             current: AtomicUsize::new(0),
//!             limit,
//!         }
//!     }
//! }
//!
//! impl FlowControl for ConnectionFlow {
//!     fn can_proceed(&mut self) -> bool {
//!         self.current.load(Ordering::Relaxed) < self.limit
//!     }
//!
//!     fn update(&mut self, metrics: &mut ProcessingMetrics) {
//!         let current = self.current.fetch_add(1, Ordering::Relaxed);
//!         metrics.update_pressure(current as f64 / self.limit as f64);
//!     }
//!
//!     fn pressure(&mut self) -> f64 {
//!         let current = self.current.load(Ordering::Relaxed);
//!         current as f64 / self.limit as f64
//!     }
//! }
//!
//! // Usage
//! let mut flow = ConnectionFlow::new(100);
//! let mut metrics = ProcessingMetrics::new();
//!
//! if flow.can_proceed() {
//!     flow.update(&mut metrics);
//!     // Handle new connection
//! }
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{debug, warn};

use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ProcessingError, WithContext,
};
use crate::http::domain::processing::metrics::ProcessingMetrics;

/// Core trait for flow control mechanisms.
///
/// Implementors of this trait provide flow control logic for processors,
/// enabling backpressure and rate limiting. Flow control is used to prevent
/// system overload by controlling the rate of processing operations.
///
/// # Requirements
///
/// Implementations must be thread-safe (`Send + Sync`) and should:
/// - Maintain consistent state across method calls
/// - Update metrics appropriately
/// - Handle concurrent access safely
/// - Report pressure accurately
///
/// # Examples
///
/// Basic implementation:
/// ```rust
/// use rusk::http::domain::processing::flow::FlowControl;
/// use rusk::http::domain::processing::metrics::ProcessingMetrics;
/// use std::sync::atomic::{AtomicUsize, Ordering};
///
/// #[derive(Debug)]
/// struct ConnectionFlow {
///     current: AtomicUsize,
///     limit: usize,
/// }
///
/// impl ConnectionFlow {
///     fn new(limit: usize) -> Self {
///         Self {
///             current: AtomicUsize::new(0),
///             limit,
///         }
///     }
/// }
///
/// impl FlowControl for ConnectionFlow {
///     fn can_proceed(&mut self) -> bool {
///         self.current.load(Ordering::Relaxed) < self.limit
///     }
///
///     fn update(&mut self, metrics: &mut ProcessingMetrics) {
///         let current = self.current.fetch_add(1, Ordering::Relaxed);
///         metrics.update_pressure(current as f64 / self.limit as f64);
///     }
///
///     fn pressure(&mut self) -> f64 {
///         let current = self.current.load(Ordering::Relaxed);
///         current as f64 / self.limit as f64
///     }
/// }
///
/// // Usage
/// let mut flow = ConnectionFlow::new(100);
/// let mut metrics = ProcessingMetrics::new();
///
/// if flow.can_proceed() {
///     flow.update(&mut metrics);
///     // Process request...
/// }
/// ```
///
/// Using with a processor:
/// ```rust
/// use rusk::http::domain::processing::flow::FlowControl;
/// use rusk::http::domain::processing::Processor;
/// use rusk::http::domain::processing::metrics::ProcessingMetrics;
/// use rusk::http::domain::error::DomainError;
/// use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
/// use async_trait::async_trait;
///
/// #[derive(Debug, Default)]
/// struct CustomFlow(std::sync::atomic::AtomicBool);
///
/// impl FlowControl for CustomFlow {
///     fn can_proceed(&mut self) -> bool {
///         !self.0.load(std::sync::atomic::Ordering::Relaxed)
///     }
///
///     fn update(&mut self, metrics: &mut ProcessingMetrics) {
///         self.0.store(true, std::sync::atomic::Ordering::Relaxed);
///         metrics.update_pressure(1.0);
///     }
///
///     fn pressure(&mut self) -> f64 {
///         if self.0.load(std::sync::atomic::Ordering::Relaxed) {
///             1.0
///         } else {
///             0.0
///         }
///     }
/// }
///
/// #[derive(Debug)]
/// struct RateLimitedProcessor;
///
/// #[async_trait]
/// impl Processor for RateLimitedProcessor {
///     type Input = Vec<u8>;
///     type Output = Vec<u8>;
///     type Error = DomainError;
///     type FlowControl = CustomFlow;
///     type Context = DefaultContext;
///
///     async fn process_with_backpressure(
///         &self,
///         input: &Self::Input,
///         flow: &mut Self::FlowControl,
///         ctx: &mut Self::Context,
///     ) -> Result<Self::Output, Self::Error> {
///         if flow.can_proceed() {
///             flow.update(ctx.metrics());
///             Ok(input.clone())
///         } else {
///             // Handle backpressure...
///             Ok(vec![])
///         }
///     }
/// }
/// ```
pub trait FlowControl: Send + Sync {
    /// Checks if processing can proceed.
    ///
    /// This method should:
    /// - Be fast and non-blocking
    /// - Be thread-safe
    /// - Return quickly
    /// - Not update any state
    ///
    /// # Returns
    ///
    /// * `true` - If processing should proceed
    /// * `false` - If processing should wait/backoff
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::FlowControl;
    /// use rusk::http::domain::processing::metrics::ProcessingMetrics;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// #[derive(Debug)]
    /// struct QueueFlow {
    ///     queued: AtomicUsize,
    ///     limit: usize,
    /// }
    ///
    /// impl FlowControl for QueueFlow {
    ///     fn can_proceed(&mut self) -> bool {
    ///         self.queued.load(Ordering::Relaxed) < self.limit
    ///     }
    ///
    ///     fn update(&mut self, metrics: &mut ProcessingMetrics) {
    ///         let current = self.queued.fetch_add(1, Ordering::Relaxed);
    ///         metrics.update_pressure(current as f64 / self.limit as f64);
    ///     }
    ///
    ///     fn pressure(&mut self) -> f64 {
    ///         let current = self.queued.load(Ordering::Relaxed);
    ///         current as f64 / self.limit as f64
    ///     }
    /// }
    /// ```
    fn can_proceed(&mut self) -> bool;

    /// Updates flow control state with metrics.
    ///
    /// This method should:
    /// - Update internal state
    /// - Update provided metrics
    /// - Be thread-safe
    /// - Handle concurrent updates
    ///
    /// # Arguments
    ///
    /// * `metrics` - Metrics to update with current state
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::FlowControl;
    /// use rusk::http::domain::processing::metrics::ProcessingMetrics;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// #[derive(Debug)]
    /// struct RateLimiter {
    ///     requests: AtomicUsize,
    ///     limit: usize,
    /// }
    ///
    /// impl FlowControl for RateLimiter {
    ///     fn can_proceed(&mut self) -> bool {
    ///         self.requests.load(Ordering::Relaxed) < self.limit
    ///     }
    ///
    ///     fn update(&mut self, metrics: &mut ProcessingMetrics) {
    ///         // Update request count
    ///         let current = self.requests.fetch_add(1, Ordering::Relaxed);
    ///
    ///         // Update pressure metrics
    ///         let pressure = current as f64 / self.limit as f64;
    ///         metrics.update_pressure(pressure);
    ///
    ///         // Record operation
    ///         metrics.increment_counter("requests_processed");
    ///     }
    ///
    ///     fn pressure(&mut self) -> f64 {
    ///         let current = self.requests.load(Ordering::Relaxed);
    ///         current as f64 / self.limit as f64
    ///     }
    /// }
    /// ```
    fn update(&mut self, metrics: &mut ProcessingMetrics);

    /// Gets current pressure level.
    ///
    /// Returns a value between 0.0 and 1.0 indicating the current pressure
    /// level:
    /// - 0.0 means no pressure (system can readily accept more work)
    /// - 1.0 means maximum pressure (system is at capacity)
    ///
    /// This method should:
    /// - Be fast and non-blocking
    /// - Be thread-safe
    /// - Return current pressure accurately
    /// - Not update any state
    ///
    /// # Returns
    ///
    /// A float between 0.0 and 1.0 representing the current pressure level
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::FlowControl;
    /// use rusk::http::domain::processing::metrics::ProcessingMetrics;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// #[derive(Debug)]
    /// struct LoadBalancer {
    ///     active: AtomicUsize,
    ///     capacity: usize,
    /// }
    ///
    /// impl FlowControl for LoadBalancer {
    ///     fn can_proceed(&mut self) -> bool {
    ///         self.active.load(Ordering::Relaxed) < self.capacity
    ///     }
    ///
    ///     fn update(&mut self, metrics: &mut ProcessingMetrics) {
    ///         let current = self.active.fetch_add(1, Ordering::Relaxed);
    ///         metrics.update_pressure(self.pressure());
    ///     }
    ///
    ///     fn pressure(&mut self) -> f64 {
    ///         let current = self.active.load(Ordering::Relaxed);
    ///         (current as f64 / self.capacity as f64).min(1.0)
    ///     }
    /// }
    ///
    /// let mut balancer = LoadBalancer {
    ///     active: AtomicUsize::new(75),
    ///     capacity: 100,
    /// };
    ///
    /// assert!(balancer.pressure() >= 0.74 && balancer.pressure() <= 0.76);
    /// ```
    fn pressure(&mut self) -> f64;
}

/// No-op flow control implementation.
///
/// This implementation always allows processing to proceed and maintains no
/// state. Useful for scenarios where flow control isn't needed.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::processing::flow::{FlowControl, NoopFlow};
/// use rusk::http::domain::processing::metrics::ProcessingMetrics;
///
/// let mut flow = NoopFlow::default();
/// let mut metrics = ProcessingMetrics::new();
///
/// // Always allows processing
/// assert!(flow.can_proceed());
///
/// // Doesn't affect metrics
/// flow.update(&mut metrics);
/// assert_eq!(flow.pressure(), 0.0);
/// ```
#[derive(Debug, Default)]
pub struct NoopFlow;

impl FlowControl for NoopFlow {
    fn can_proceed(&mut self) -> bool {
        true
    }

    fn update(&mut self, metrics: &mut ProcessingMetrics) {
        // NoopFlow doesn't affect pressure
        metrics.update_pressure(0.0);
    }

    fn pressure(&mut self) -> f64 {
        0.0
    }
}

/// Token bucket flow control implementation.
///
/// Uses a token bucket algorithm to control processing rate:
/// - Tokens are added at a fixed rate
/// - Each processing operation consumes one token
/// - When no tokens are available, processing must wait
///
/// # Examples
///
/// Basic usage:
/// ```rust
/// use rusk::http::domain::processing::flow::{FlowControl, TokenBucketFlow};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut flow = TokenBucketFlow::builder()
///     .capacity(100)
///     .refill_rate(10.0) // 10 tokens per second
///     .build();
///
/// // Acquire token with timeout
/// match tokio::time::timeout(
///     Duration::from_secs(1),
///     flow.acquire_one()
/// ).await {
///     Ok(Ok(())) => println!("Token acquired"),
///     Ok(Err(e)) => println!("Acquisition failed: {}", e),
///     Err(_) => println!("Timeout waiting for token"),
/// }
/// # Ok(())
/// # }
/// ```
///
/// With processor:
/// ```rust
/// use rusk::http::domain::processing::flow::{TokenBucketFlow, FlowControl};
/// use rusk::http::domain::processing::Processor;
/// use rusk::http::domain::processing::metrics::ProcessingMetrics;
/// use rusk::http::domain::error::DomainError;
/// use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
/// use async_trait::async_trait;
///
/// #[derive(Debug)]
/// struct RateLimitedProcessor;
///
/// #[async_trait]
/// impl Processor for RateLimitedProcessor {
///     type Input = Vec<u8>;
///     type Output = Vec<u8>;
///     type Error = DomainError;
///     type FlowControl = TokenBucketFlow;
///     type Context = DefaultContext;
///
///     async fn process_with_backpressure(
///         &self,
///         input: &Self::Input,
///         flow: &mut Self::FlowControl,
///         ctx: &mut Self::Context,
///     ) -> Result<Self::Output, Self::Error> {
///         // Wait for token
///         flow.acquire_one().await?;
///
///         // Process with rate limiting
///         flow.update(ctx.metrics());
///         Ok(input.clone())
///     }
/// }
/// ```
#[derive(Debug)]
pub struct TokenBucketFlow {
    /// Maximum number of tokens
    capacity: usize,
    /// Current token count
    tokens: AtomicUsize,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucketFlow {
    /// Creates a new builder for configuring TokenBucketFlow.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::TokenBucketFlow;
    ///
    /// let flow = TokenBucketFlow::builder()
    ///     .capacity(100)
    ///     .refill_rate(10.0)
    ///     .build();
    /// ```
    pub fn builder() -> TokenBucketFlowBuilder {
        TokenBucketFlowBuilder::default()
    }

    /// Attempts to acquire one token, waiting if necessary.
    ///
    /// This method will:
    /// 1. Try to acquire a token immediately
    /// 2. If no token is available, wait for refill
    /// 3. Retry until either a token is acquired or timeout occurs
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If a token was successfully acquired
    /// * `Err(DomainError)` - If acquisition timed out or was cancelled
    ///
    /// # Errors
    ///
    /// Returns `ProcessingError::Timeout` if no token could be acquired
    /// within 30 seconds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::TokenBucketFlow;
    /// use tokio::time::timeout;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut flow = TokenBucketFlow::builder()
    ///     .capacity(5)
    ///     .refill_rate(10.0)
    ///     .build();
    ///
    /// // Try to acquire with timeout
    /// match timeout(Duration::from_secs(1), flow.acquire_one()).await {
    ///     Ok(Ok(())) => println!("Token acquired"),
    ///     Ok(Err(e)) => println!("Acquisition failed: {}", e),
    ///     Err(_) => println!("Timeout"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn acquire_one(&mut self) -> Result<(), DomainError> {
        let start = Instant::now();
        let timeout = Duration::from_secs(30); // Fixed timeout

        loop {
            if self.try_acquire_one() {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(ProcessingError::Timeout {
                    operation: "token_bucket_flow".into(),
                    duration: timeout,
                }
                .with_context("token_bucket_flow")
                .with_stage("acquire")
                .with_resource_usage(
                    self.capacity - self.tokens.load(Ordering::SeqCst),
                    self.capacity,
                    "tokens",
                ));
            }

            // Calculate optimal wait time based on refill rate
            let tokens_needed = 1;
            let expected_wait = Duration::from_secs_f64(
                tokens_needed as f64 / self.refill_rate,
            );

            // Use smaller of calculated wait and fixed interval
            let wait = expected_wait.min(Duration::from_millis(100));

            tokio::time::sleep(wait).await;
        }
    }

    /// Tries to acquire one token without waiting.
    ///
    /// This method attempts to atomically acquire a token if one is available.
    /// It first refills any tokens based on elapsed time, then attempts to
    /// acquire one token.
    ///
    /// # Returns
    ///
    /// * `true` - If a token was successfully acquired
    /// * `false` - If no tokens were available
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::TokenBucketFlow;
    ///
    /// let mut flow = TokenBucketFlow::builder()
    ///     .capacity(5)
    ///     .refill_rate(10.0)
    ///     .build();
    ///
    /// // Try to acquire tokens
    /// if flow.try_acquire_one() {
    ///     println!("Token acquired");
    /// } else {
    ///     println!("No tokens available");
    /// }
    /// ```
    ///
    /// # Thread Safety
    ///
    /// This method uses atomic operations to ensure thread-safe token
    /// acquisition.
    pub fn try_acquire_one(&mut self) -> bool {
        self.refill();
        match self.tokens.fetch_sub(1, Ordering::SeqCst) {
            n if n > 0 => true,
            _ => {
                // Restore the token if we couldn't acquire it
                self.tokens.fetch_add(1, Ordering::SeqCst);
                false
            }
        }
    }

    /// Refills tokens based on elapsed time since the last refill.
    ///
    /// This method:
    /// 1. Calculates elapsed time since last refill
    /// 2. Determines number of tokens to add based on refill rate
    /// 3. Adds tokens up to capacity
    /// 4. Updates last refill time
    ///
    /// # Implementation Notes
    ///
    /// - Uses atomic operations for thread safety
    /// - Caps total tokens at capacity
    /// - Only updates if at least one token should be added
    ///
    /// This method is automatically called by `try_acquire_one` and doesn't
    /// need to be called directly.
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed();
        let new_tokens = (elapsed.as_secs_f64() * self.refill_rate) as usize;

        if new_tokens > 0 {
            let current = self.tokens.load(Ordering::SeqCst);
            let new_total = (current + new_tokens).min(self.capacity);
            self.tokens.store(new_total, Ordering::SeqCst);
            self.last_refill = Instant::now();
        }
    }

    /// Gets the current pressure level of the token bucket.
    ///
    /// Pressure is calculated as `1.0 - (available_tokens / capacity)`,
    /// where:
    /// - 0.0 means the bucket is full (no pressure)
    /// - 1.0 means the bucket is empty (maximum pressure)
    ///
    /// # Returns
    ///
    /// A float between 0.0 and 1.0 representing the current pressure level.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::{FlowControl, TokenBucketFlow};
    ///
    /// let mut flow = TokenBucketFlow::builder()
    ///     .capacity(10)
    ///     .refill_rate(10.0)
    ///     .build();
    ///
    /// // Initially no pressure
    /// assert_eq!(flow.pressure(), 0.0);
    ///
    /// // Use some tokens
    /// for _ in 0..5 {
    ///     flow.try_acquire_one();
    /// }
    ///
    /// // Should be at about 50% pressure
    /// assert!(flow.pressure() >= 0.45 && flow.pressure() <= 0.55);
    /// ```
    fn get_pressure(&self) -> f64 {
        let current = self.tokens.load(Ordering::SeqCst);
        1.0 - (current as f64 / self.capacity as f64).clamp(0.0, 1.0)
    }
}

impl Default for TokenBucketFlow {
    /// Creates a default TokenBucketFlow with:
    /// - capacity: 100 tokens
    /// - refill_rate: 10 tokens per second
    fn default() -> Self {
        Self::builder()
            .capacity(100) // Default from TokenBucketFlowBuilder
            .refill_rate(10.0) // Default from TokenBucketFlowBuilder
            .build()
    }
}

impl FlowControl for TokenBucketFlow {
    fn can_proceed(&mut self) -> bool {
        self.try_acquire_one()
    }

    fn update(&mut self, metrics: &mut ProcessingMetrics) {
        // Update pressure based on theoretical maximum tokens
        let p = self.pressure();
        metrics.update_pressure(p);
    }

    fn pressure(&mut self) -> f64 {
        self.get_pressure()
    }
}

/// Builder for TokenBucketFlow configuration.
///
/// Provides a fluent interface for configuring token bucket parameters.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::processing::flow::TokenBucketFlow;
///
/// let flow = TokenBucketFlow::builder()
///     .capacity(1000)
///     .refill_rate(100.0)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct TokenBucketFlowBuilder {
    capacity: Option<usize>,
    refill_rate: Option<f64>,
}

impl TokenBucketFlowBuilder {
    /// Sets the maximum number of tokens the bucket can hold.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of tokens
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::TokenBucketFlow;
    ///
    /// let flow = TokenBucketFlow::builder()
    ///     .capacity(1000) // Can hold up to 1000 tokens
    ///     .refill_rate(100.0)
    ///     .build();
    /// ```
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    /// Sets the rate at which tokens are refilled.
    ///
    /// # Arguments
    ///
    /// * `rate` - Number of tokens to add per second
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::TokenBucketFlow;
    ///
    /// let flow = TokenBucketFlow::builder()
    ///     .capacity(1000)
    ///     .refill_rate(100.0) // 100 tokens per second
    ///     .build();
    /// ```
    pub fn refill_rate(mut self, rate: f64) -> Self {
        self.refill_rate = Some(rate);
        self
    }

    /// Builds a new TokenBucketFlow with the configured parameters.
    ///
    /// Uses default values if not specified:
    /// - capacity: 100 tokens
    /// - refill_rate: 10 tokens per second
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rusk::http::domain::processing::flow::TokenBucketFlow;
    ///
    /// // With custom values
    /// let custom_flow = TokenBucketFlow::builder()
    ///     .capacity(1000)
    ///     .refill_rate(100.0)
    ///     .build();
    ///
    /// // With defaults
    /// let default_flow = TokenBucketFlow::builder().build();
    /// ```
    pub fn build(self) -> TokenBucketFlow {
        let capacity = self.capacity.unwrap_or(100);
        let refill_rate = self.refill_rate.unwrap_or(10.0);

        TokenBucketFlow {
            capacity,
            tokens: AtomicUsize::new(capacity),
            refill_rate,
            last_refill: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Barrier;
    use tokio::time::timeout;

    use crate::http::domain::error::{
        CommonErrorAttributes, DomainError, ProcessingError, WithContext,
    };

    #[test]
    fn test_noop_flow() {
        let mut flow = NoopFlow::default();
        let mut metrics = ProcessingMetrics::new();

        // Basic functionality
        assert!(flow.can_proceed());
        flow.update(&mut metrics);
        assert_eq!(flow.pressure(), 0.0);

        // Multiple updates shouldn't affect state
        for _ in 0..100 {
            flow.update(&mut metrics);
            assert!(flow.can_proceed());
            assert_eq!(flow.pressure(), 0.0);
        }
    }

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(10.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Initial state
        assert_eq!(flow.pressure(), 0.0);

        // Use tokens one by one
        for i in 0..5 {
            // Each can_proceed() consumes a token
            assert!(flow.can_proceed(), "Failed at iteration {}", i);
            // Update metrics but don't consume another token
            flow.update(&mut metrics);
        }

        // Check pressure is about 50%
        assert!(flow.pressure() >= 0.4 && flow.pressure() <= 0.6);

        // Wait for some refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should have tokens again
        assert!(flow.can_proceed());
        assert!(flow.pressure() < 0.5);
    }

    #[tokio::test]
    async fn test_token_bucket_acquisition() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(10.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Explicit token acquisition
        for i in 0..5 {
            flow.acquire_one()
                .await
                .unwrap_or_else(|_| panic!("Failed to acquire token {}", i));
            flow.update(&mut metrics);
        }

        // Verify pressure
        assert!(flow.pressure() >= 0.4 && flow.pressure() <= 0.6);

        // Try to acquire more tokens
        for i in 0..5 {
            match flow.acquire_one().await {
                Ok(_) => flow.update(&mut metrics),
                Err(e) => {
                    assert!(matches!(
                        e,
                        DomainError::Processing(
                            ProcessingError::Timeout { .. }
                        )
                    ));
                    break;
                }
            }
        }

        // Verify high pressure
        assert!(flow.pressure() > 0.8);
    }

    #[tokio::test]
    async fn test_token_bucket_usage_patterns() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0)
            .build();

        // Pattern 1: Direct acquisition
        for i in 0..5 {
            flow.acquire_one()
                .await
                .unwrap_or_else(|_| panic!("Failed to acquire token {}", i));
        }

        let p1 = flow.pressure();
        assert!(
            p1 >= 0.45 && p1 <= 0.55,
            "Expected pressure around 0.5 after using 5 tokens, got {}",
            p1
        );

        // Pattern 2: can_proceed() checks
        let can_proceed_count = (0..5).filter(|_| flow.can_proceed()).count();

        assert!(
            can_proceed_count > 0,
            "Should be able to proceed at least once"
        );

        // Final pressure should be higher
        let p2 = flow.pressure();
        assert!(
            p2 > p1,
            "Pressure should increase after more usage: {} -> {}",
            p1,
            p2
        );
    }

    // Add a separate test for mixed usage patterns
    #[tokio::test]
    async fn test_token_bucket_mixed_patterns() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(20.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Initial state
        assert_eq!(flow.pressure(), 0.0);

        // Alternating patterns
        for i in 0..3 {
            // Pattern 1: can_proceed + update
            assert!(
                flow.can_proceed(),
                "Should proceed on iteration {} (pressure: {})",
                i,
                flow.pressure()
            );
            flow.update(&mut metrics);

            // Pattern 2: explicit acquire
            flow.acquire_one()
                .await
                .unwrap_or_else(|_| panic!("Failed to acquire token {}", i));
            flow.update(&mut metrics);

            // Log pressure after each pair
            let p = flow.pressure();
            assert!(p <= 0.7, "Pressure too high after iteration {}: {}", i, p);

            // Small wait to allow some refill
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Final pressure should be moderate
        let final_pressure = flow.pressure();
        assert!(
            final_pressure <= 0.5,
            "Final pressure too high: {}",
            final_pressure
        );
    }

    // Add a test for gradual pressure increase
    #[tokio::test]
    async fn test_token_bucket_gradual_pressure() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(10.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        let mut last_pressure = 0.0;

        // Use tokens one by one with small delays
        for i in 0..5 {
            assert!(
                flow.can_proceed(),
                "Failed to proceed at step {} (pressure: {})",
                i,
                flow.pressure()
            );
            flow.update(&mut metrics);

            let current_pressure = flow.pressure();
            assert!(
                current_pressure >= last_pressure,
                "Pressure decreased unexpectedly: {} -> {}",
                last_pressure,
                current_pressure
            );
            last_pressure = current_pressure;

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Final pressure should be moderate due to gradual usage
        assert!(
            flow.pressure() <= 0.6,
            "Final pressure too high: {}",
            flow.pressure()
        );
    }

    // Add a more focused test for pressure calculations
    #[tokio::test]
    async fn test_token_bucket_basic_pressure() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0) // Much faster refill for testing
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Initial state
        assert_eq!(flow.pressure(), 0.0, "Initial pressure should be 0");

        // Use 5 tokens
        for i in 0..5 {
            assert!(
                flow.can_proceed(),
                "Should be able to proceed at step {} (pressure: {})",
                i,
                flow.pressure()
            );
            flow.update(&mut metrics);
        }

        // Check pressure is about 50%
        let p = flow.pressure();
        assert!(
            p >= 0.45 && p <= 0.55,
            "Expected pressure around 0.5, got {}",
            p
        );
    }

    #[tokio::test]
    async fn test_token_bucket_incremental_usage() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Use tokens gradually
        for i in 0..3 {
            assert!(flow.can_proceed(), "Step {}", i);
            flow.update(&mut metrics);

            let p = flow.pressure();
            assert!(
                    p <= 0.4,
                    "Pressure should stay low during gradual usage, got {} at step {}",
                    p,
                    i
                );

            // Wait a bit between operations
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[tokio::test]
    async fn test_token_bucket_full_cycle() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0)
            .build();

        // Initial state
        assert_eq!(flow.pressure(), 0.0);

        // Use all tokens
        let mut acquired = 0;
        while flow.try_acquire_one() {
            acquired += 1;
            if acquired >= 10 {
                break;
            }
        }
        assert_eq!(acquired, 10, "Should acquire exactly 10 tokens");

        // Verify we're at max pressure
        let max_pressure = flow.pressure();
        assert!(
            max_pressure > 0.95,
            "Should be at maximum pressure, got {}",
            max_pressure
        );

        // Immediate acquisition should fail
        assert!(
            !flow.try_acquire_one(),
            "Should not acquire token when at maximum pressure"
        );

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should be able to acquire tokens again
        assert!(flow.try_acquire_one(), "Should acquire token after refill");

        let final_pressure = flow.pressure();
        assert!(
            final_pressure < max_pressure,
            "Pressure should decrease after refill: {} -> {}",
            max_pressure,
            final_pressure
        );
    }

    // Add more focused tests for pressure behavior
    #[tokio::test]
    async fn test_token_bucket_pressure_states() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0)
            .build();

        // Empty bucket (pressure = 0.0)
        assert_eq!(flow.pressure(), 0.0, "Empty bucket should have 0 pressure");

        // Fill halfway
        for _ in 0..5 {
            flow.acquire_one().await.expect("Should acquire token");
        }

        let half_pressure = flow.pressure();
        assert!(
            half_pressure >= 0.45 && half_pressure <= 0.55,
            "Half-full bucket should have ~0.5 pressure, got {}",
            half_pressure
        );

        // Fill completely
        for _ in 0..5 {
            flow.acquire_one().await.expect("Should acquire token");
        }

        let full_pressure = flow.pressure();
        assert!(
            full_pressure > 0.9,
            "Full bucket should have high pressure, got {}",
            full_pressure
        );
    }

    #[tokio::test]
    async fn test_token_bucket_refill_behavior() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0) // 50 tokens/second = 1 token/20ms
            .build();

        // Use all tokens
        for _ in 0..10 {
            flow.acquire_one().await.expect("Should acquire token");
        }

        let start_pressure = flow.pressure();
        assert!(
            start_pressure > 0.9,
            "Should start at high pressure, got {}",
            start_pressure
        );

        // Wait for approximately 5 tokens to refill (100ms)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Try to acquire exactly what should have refilled
        for _ in 0..5 {
            flow.acquire_one()
                .await
                .expect("Should acquire refilled token");
        }

        // Should be back at high pressure
        let end_pressure = flow.pressure();
        assert!(
            end_pressure > 0.9,
            "Should be at high pressure after using refilled tokens, got {}",
            end_pressure
        );
    }

    #[tokio::test]
    async fn test_token_bucket_continuous_usage() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        let start_time = std::time::Instant::now();
        let mut tokens_acquired = 0;

        // Try to acquire tokens for a fixed time period
        while start_time.elapsed() < Duration::from_millis(200) {
            if flow.can_proceed() {
                flow.update(&mut metrics);
                tokens_acquired += 1;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Should have acquired more tokens than the initial capacity
        // due to refills during the test period
        assert!(
            tokens_acquired > 10,
            "Should acquire more tokens than capacity due to refill, got {}",
            tokens_acquired
        );
    }

    // Add a test specifically for refill rates
    #[tokio::test]
    async fn test_token_bucket_refill_rates() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0)
            .build();

        // Initial state
        assert_eq!(flow.pressure(), 0.0);

        // Use 5 tokens
        for _ in 0..5 {
            assert!(flow.try_acquire_one(), "Should acquire initial tokens");
        }

        let initial_pressure = flow.pressure();
        assert!(
            initial_pressure >= 0.45 && initial_pressure <= 0.55,
            "Expected initial pressure around 0.5, got {}",
            initial_pressure
        );

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Use more tokens
        let acquired = (0..3).filter(|_| flow.try_acquire_one()).count();

        assert!(acquired > 0, "Should acquire at least one refilled token");

        let final_pressure = flow.pressure();
        assert!(
            final_pressure > initial_pressure,
            "Pressure should increase after using more tokens: {} -> {}",
            initial_pressure,
            final_pressure
        );
    }

    // Add a test for very small time intervals
    #[tokio::test]
    async fn test_token_bucket_small_intervals() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(100.0) // Fast refill rate
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Use 3 tokens quickly
        for _ in 0..3 {
            assert!(flow.can_proceed());
            flow.update(&mut metrics);
        }

        let p1 = flow.pressure();

        // Very short wait
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Should see small but measurable refill
        let p2 = flow.pressure();
        assert!(p2 <= p1, "Pressure should not increase: {} -> {}", p1, p2);
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(50.0) // 50 tokens per second = 1 token per 20ms
            .build();

        // Initial state
        assert_eq!(flow.pressure(), 0.0, "Initial pressure should be 0.0");

        // Use 8 tokens
        for i in 0..8 {
            flow.acquire_one()
                .await
                .unwrap_or_else(|_| panic!("Failed to acquire token {}", i));
        }

        // Verify high pressure but not maxed
        let initial_pressure = flow.pressure();
        assert!(
            initial_pressure >= 0.7 && initial_pressure <= 0.8,
            "Should be at high pressure after using 8/10 tokens, got {}",
            initial_pressure
        );

        // Wait for tokens to refill (200ms should give us 10 tokens)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Try to acquire refilled tokens
        for i in 0..5 {
            flow.acquire_one().await.unwrap_or_else(|_| {
                panic!("Failed to acquire refilled token {}", i)
            });
        }

        // Should still have tokens available
        assert!(flow.can_proceed(), "Should still have tokens after refill");
    }

    #[tokio::test]
    async fn test_token_bucket_acquire_timeout() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(1)
            .refill_rate(0.1) // Very slow refill
            .build();

        // Use the only token
        flow.acquire_one().await.unwrap();

        // Next acquisition should timeout
        let result =
            timeout(Duration::from_millis(100), flow.acquire_one()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_bucket_error_handling() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(10)
            .refill_rate(10.0) // Slower refill rate for error testing
            .build();

        // Use all tokens
        for i in 0..10 {
            flow.acquire_one()
                .await
                .unwrap_or_else(|_| panic!("Failed to acquire token {}", i));
        }

        // Try to acquire when empty with a timeout
        match timeout(Duration::from_millis(50), flow.acquire_one()).await {
            Ok(Ok(_)) => panic!("Should not acquire token when empty"),
            Ok(Err(e)) => {
                assert!(matches!(
                    e,
                    DomainError::Processing(ProcessingError::Timeout { .. })
                ));

                // Verify error context
                assert_eq!(e.operation().unwrap(), "token_bucket_flow");
                assert_eq!(e.get_attribute("stage").unwrap(), "acquire");

                // Verify resource usage is recorded
                assert!(e.get_attribute("used_tokens").is_some());
                assert!(e.get_attribute("total_tokens").is_some());
            }
            Err(_elapsed) => {
                // Timeout is also an acceptable outcome
                // as acquire_one might wait for tokens
            }
        }

        // Verify we're at high pressure
        assert!(
            flow.pressure() > 0.9,
            "Pressure should be high when tokens exhausted"
        );

        // Wait for partial refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Try to acquire with timeout after refill
        match timeout(Duration::from_millis(50), flow.acquire_one()).await {
            Ok(Ok(_)) => {
                // Successfully acquired a token after refill
            }
            other => panic!(
                "Should be able to acquire token after refill, got {:?}",
                other
            ),
        }
    }

    // Add a test specifically for timeout behavior
    #[tokio::test]
    async fn test_token_bucket_timeout_behavior() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(5)
            .refill_rate(1.0) // Very slow refill
            .build();

        // Use all tokens
        for _ in 0..5 {
            flow.acquire_one()
                .await
                .expect("Should acquire initial tokens");
        }

        // Try multiple acquisitions with different timeouts
        let start = std::time::Instant::now();
        let mut timeouts = 0;
        let mut errors = 0;

        for i in 0..3 {
            match timeout(Duration::from_millis(50), flow.acquire_one()).await {
                Ok(Ok(_)) => {
                    panic!("Should not acquire token {} when empty", i)
                }
                Ok(Err(_)) => errors += 1,
                Err(_) => timeouts += 1,
            }
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(200),
            "Error handling took too long: {:?}",
            elapsed
        );
        assert!(
            timeouts + errors > 0,
            "Should get either timeouts or errors when tokens exhausted"
        );
    }

    // Add a test for rapid token requests
    #[tokio::test]
    async fn test_token_bucket_rapid_requests() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(5)
            .refill_rate(10.0)
            .build();

        let mut successful = 0;
        let mut failed = 0;

        // Try rapid requests with short timeouts
        for i in 0..10 {
            match timeout(Duration::from_millis(10), flow.acquire_one()).await {
                Ok(Ok(_)) => successful += 1,
                Ok(Err(_)) => failed += 1,
                Err(_) => failed += 1,
            }

            if i % 2 == 0 {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        assert!(successful > 0, "Should succeed at least a few times");
        assert!(failed > 0, "Should fail at least a few times");
        assert_eq!(successful + failed, 10, "Should account for all attempts");
    }

    #[tokio::test]
    async fn test_concurrent_token_bucket() {
        let flow = Arc::new(tokio::sync::Mutex::new(
            TokenBucketFlow::builder()
                .capacity(5)
                .refill_rate(1.0) // Slower refill rate
                .build(),
        ));
        let barrier = Arc::new(Barrier::new(5));
        let start_time = std::time::Instant::now();
        let mut handles = Vec::new();

        // Launch 5 concurrent tasks
        for i in 0..5 {
            let flow = Arc::clone(&flow);
            let barrier = Arc::clone(&barrier);

            let handle = tokio::spawn(async move {
                barrier.wait().await;

                let mut flow = flow.lock().await;
                let result = flow.acquire_one().await;
                (i, result)
            });
            handles.push(handle);
        }

        // Collect results
        let mut successful_acquisitions = 0;
        for handle in handles {
            let (i, result) = handle.await.unwrap();
            if result.is_ok() {
                successful_acquisitions += 1;
            }
        }

        // Verify we got exactly 5 successful acquisitions
        assert_eq!(
            successful_acquisitions, 5,
            "Expected exactly 5 successful token acquisitions"
        );

        // Now immediately try to acquire another token
        // This should fail because we've used all tokens and haven't waited
        // long enough for refill
        let mut flow = Arc::try_unwrap(flow).unwrap().into_inner();

        // Try to acquire with a short timeout
        let result = timeout(
            Duration::from_millis(50), // Short timeout
            flow.acquire_one(),
        )
        .await;

        assert!(result.is_err(), "Should not be able to acquire token immediately after using all tokens");

        // Verify elapsed time is reasonable
        assert!(
            start_time.elapsed() < Duration::from_secs(1),
            "Test took too long, possible timing issue"
        );
    }

    #[test]
    fn test_token_bucket_builder() {
        // Test default values
        let flow = TokenBucketFlow::builder().build();

        // Test custom values
        let flow = TokenBucketFlow::builder()
            .capacity(1000)
            .refill_rate(50.0)
            .build();

        // Test zero capacity
        let flow = TokenBucketFlow::builder()
            .capacity(0)
            .refill_rate(1.0)
            .build();

        // Test zero refill rate
        let flow = TokenBucketFlow::builder()
            .capacity(100)
            .refill_rate(0.0)
            .build();
    }

    #[tokio::test]
    async fn test_token_bucket_pressure_accuracy() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(100)
            .refill_rate(10.0)
            .build();
        let mut metrics = ProcessingMetrics::new();

        // Initial pressure should be 0
        assert_eq!(flow.pressure(), 0.0);

        // Use 50% of tokens
        for _ in 0..50 {
            flow.acquire_one().await.unwrap();
            flow.update(&mut metrics);
        }

        // Pressure should be around 0.5
        assert!(flow.pressure() >= 0.45 && flow.pressure() <= 0.55);

        // Use remaining tokens
        for _ in 0..50 {
            flow.acquire_one().await.unwrap();
            flow.update(&mut metrics);
        }

        // Pressure should be maximum
        assert!(flow.pressure() > 0.95);
    }

    #[tokio::test]
    async fn test_token_bucket_cancel_safety() {
        let mut flow = TokenBucketFlow::builder()
            .capacity(1)
            .refill_rate(0.1)
            .build();

        // Use the only token
        flow.acquire_one().await.unwrap();

        // Start acquisition that will be cancelled
        let acquire_task =
            tokio::spawn(async move { flow.acquire_one().await });

        // Cancel after small delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        acquire_task.abort();

        // Wait for task to complete
        match acquire_task.await {
            Ok(_) => panic!("Task should have been cancelled"),
            Err(e) => assert!(e.is_cancelled()),
        }
    }

    #[test]
    fn test_custom_flow_implementation() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Debug)]
        struct TestFlow {
            count: AtomicUsize,
            limit: usize,
        }

        impl TestFlow {
            fn new(limit: usize) -> Self {
                Self {
                    count: AtomicUsize::new(0),
                    limit,
                }
            }
        }

        impl FlowControl for TestFlow {
            fn can_proceed(&mut self) -> bool {
                self.count.load(Ordering::Relaxed) < self.limit
            }

            fn update(&mut self, metrics: &mut ProcessingMetrics) {
                let count = self.count.fetch_add(1, Ordering::Relaxed);
                metrics.update_pressure(count as f64 / self.limit as f64);
            }

            fn pressure(&mut self) -> f64 {
                self.count.load(Ordering::Relaxed) as f64 / self.limit as f64
            }
        }

        let mut flow = TestFlow::new(10);
        let mut metrics = ProcessingMetrics::new();

        // Test basic functionality
        assert!(flow.can_proceed());
        flow.update(&mut metrics);
        assert!(flow.pressure() > 0.0);

        // Test limit
        for _ in 0..9 {
            flow.update(&mut metrics);
        }
        assert!(!flow.can_proceed());
        assert_eq!(flow.pressure(), 1.0);
    }
}
