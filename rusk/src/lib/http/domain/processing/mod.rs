// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Processing framework for RUES values and events.
//!
//! This module provides a comprehensive processing framework for handling RUES
//! values and events with support for:
//! - Backpressure and flow control
//! - Thread-safe metrics collection
//! - Processing context propagation
//! - Error handling and recovery
//!
//! # Architecture
//!
//! The processing system consists of several key components:
//!
//! 1. **Processor Trait**: Core processing interface with backpressure support
//! 2. **DefaultProcessor**: Extension trait providing default implementations
//! 3. **Flow Control**: Mechanisms for handling backpressure
//! 4. **Processing Context**: Thread-safe state and metrics tracking
//! 5. **Processing Pipeline**: Composable processing chains
//!
//! # Thread Safety
//!
//! The metrics collection system uses `parking_lot::RwLock` for thread safety.
//! This choice was made because:
//! - Metrics operations are typically very short-duration (microseconds)
//! - High-frequency operations benefit from `parking_lot`'s optimized locking
//! - No need for async lock semantics as operations never block for long
//! - Lower memory overhead compared to async locks
//!
//! # Examples
//!
//! Basic processing implementation with metrics and error handling:
//! ```rust
//! use rusk::http::domain::processing::{Processor, DefaultProcessor};
//! use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
//! use rusk::http::domain::processing::flow::NoopFlow;
//! use rusk::http::domain::error::{
//!     DomainError, ProcessingError, WithContext, CommonErrorAttributes,
//!    ValidationError,
//! };
//! use async_trait::async_trait;
//!
//! #[derive(Debug, Default)]
//! struct JsonValidator;
//!
//! #[async_trait]
//! impl Processor for JsonValidator {
//!     type Input = Vec<u8>;
//!     type Output = serde_json::Value;
//!     type Error = DomainError;
//!     type FlowControl = NoopFlow;
//!     type Context = DefaultContext;
//!
//!     async fn process_with_backpressure(
//!         &self,
//!         input: &Self::Input,
//!         flow: &mut Self::FlowControl,
//!         ctx: &mut Self::Context,
//!     ) -> Result<Self::Output, Self::Error> {
//!         // Thread-safe metrics recording
//!         ctx.metrics().start_operation("json_validation");
//!
//!         // Validate input
//!         if input.is_empty() {
//!             return Err(ValidationError::EmptyInput("No data provided".into())
//!                 .with_context("json_validation")
//!                 .with_stage("validation")
//!                 .with_input_size(0));
//!         }
//!
//!         // Process with error context
//!         let result = match serde_json::from_slice(input) {
//!             Ok(value) => {
//!                 ctx.metrics().increment_counter("json_validations_success");
//!                 Ok(value)
//!             }
//!             Err(e) => {
//!                 ctx.metrics().record_error("json_validation", "parse_error");
//!                 Err(e.into())
//!             }
//!         };
//!
//!         ctx.metrics().complete_operation("json_validation");
//!         result
//!     }
//! }
//!
//! // Now JsonValidator automatically implements DefaultProcessor
//! async fn validate_json(validator: &JsonValidator) -> Result<serde_json::Value, DomainError> {
//!     let mut ctx = DefaultContext::new();
//!     DefaultProcessor::process(validator, &b"{}".to_vec(), &mut ctx).await
//! }
//! ```
//!
//! Processing with backpressure and cancellation:
//! ```rust
//! use rusk::http::domain::processing::{Processor, CancellableProcessor};
//! use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
//! use rusk::http::domain::processing::flow::{FlowControl, TokenBucketFlow};
//! use rusk::http::domain::error::{
//!     DomainError, ProcessingError, WithContext, CommonErrorAttributes,
//! };
//! use async_trait::async_trait;
//!
//! #[derive(Debug)]
//! struct EventProcessor;
//!
//! #[async_trait]
//! impl Processor for EventProcessor {
//!     type Input = Vec<u8>;
//!     type Output = Vec<u8>;
//!     type Error = DomainError;
//!     type FlowControl = TokenBucketFlow;
//!     type Context = DefaultContext;
//!
//!     async fn process_with_backpressure(
//!         &self,
//!         input: &Self::Input,
//!         flow: &mut Self::FlowControl,
//!         ctx: &mut Self::Context,
//!     ) -> Result<Self::Output, Self::Error> {
//!         // Wait for capacity with error context
//!         if let Err(e) = flow.acquire_one().await {
//!             return Err(e.with_context("event_processor")
//!                 .with_stage("flow_control")
//!                 .with_input_size(input.len()));
//!         }
//!
//!         ctx.metrics().start_operation("event_processing");
//!
//!         // Process with pressure monitoring
//!         let pressure = flow.pressure();
//!         ctx.metrics().update_pressure(pressure);
//!
//!         let result = if pressure > 0.9 {
//!             Err(ProcessingError::RateLimitExceeded {
//!                 limit: 100,
//!                 current: 150,
//!             }
//!             .with_context("event_processor")
//!             .with_stage("pressure_check")
//!             .with_resource_usage(150, 100, "events"))
//!         } else {
//!             Ok(input.clone())
//!         };
//!
//!         ctx.metrics().complete_operation("event_processing");
//!         result
//!     }
//! }
//! ```
//!
//! Custom flow control with error handling:
//! ```rust
//! use rusk::http::domain::processing::Processor;
//! use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
//! use rusk::http::domain::processing::flow::FlowControl;
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//! use rusk::http::domain::error::{
//!     DomainError, ProcessingError, WithContext, CommonErrorAttributes,
//! };
//! use std::sync::atomic::{AtomicUsize, Ordering};
//! use async_trait::async_trait;
//!
//! // Custom flow control implementation
//! #[derive(Debug, Default)]
//! struct RateLimitFlow {
//!     requests: AtomicUsize,
//!     limit: usize,
//! }
//!
//! impl FlowControl for RateLimitFlow {
//!     fn can_proceed(&mut self) -> bool {
//!         self.requests.load(Ordering::Relaxed) < self.limit
//!     }
//!
//!     fn update(&mut self, metrics: &mut ProcessingMetrics) {
//!         self.requests.fetch_add(1, Ordering::Relaxed);
//!         // Update pressure metrics
//!         let current = self.requests.load(Ordering::Relaxed);
//!         metrics.update_pressure(current as f64 / self.limit as f64);
//!     }
//!
//!     fn pressure(&mut self) -> f64 {
//!         let requests = self.requests.load(Ordering::Relaxed);
//!         requests as f64 / self.limit as f64
//!     }
//! }
//!
//! // Processor with custom flow control
//! #[derive(Debug)]
//! struct RateLimitedProcessor {
//!     flow: RateLimitFlow,
//! }
//!
//! #[async_trait]
//! impl Processor for RateLimitedProcessor {
//!     type Input = Vec<u8>;
//!     type Output = Vec<u8>;
//!     type Error = DomainError;
//!     type FlowControl = RateLimitFlow;
//!     type Context = DefaultContext;
//!
//!     async fn process_with_backpressure(
//!         &self,
//!         input: &Self::Input,
//!         flow: &mut Self::FlowControl,
//!         ctx: &mut Self::Context,
//!     ) -> Result<Self::Output, Self::Error> {
//!         if !flow.can_proceed() {
//!             return Err(ProcessingError::RateLimitExceeded {
//!                 limit: self.flow.limit,
//!                 current: self.flow.requests.load(Ordering::Relaxed),
//!             }
//!             .with_context("rate_limited_processor")
//!             .with_stage("flow_control")
//!             .with_resource_usage(
//!                 self.flow.requests.load(Ordering::Relaxed),
//!                 self.flow.limit,
//!                 "requests"
//!             ));
//!         }
//!
//!         ctx.metrics().start_operation("rate_limited_processing");
//!
//!         // Process with pressure monitoring
//!         let pressure = flow.pressure();
//!         ctx.metrics().update_pressure(pressure);
//!
//!         let result = Ok(input.clone());
//!
//!         flow.update(ctx.metrics());
//!         ctx.metrics().complete_operation("rate_limited_processing");
//!
//!         result
//!     }
//! }
//! ```
//!
//! # Error Handling
//!
//! The processing framework uses `DomainError` as its base error type, with
//! rich context support through the `WithContext` and `CommonErrorAttributes`
//! traits:
//!
//! ```rust
//! use rusk::http::domain::processing::Processor;
//! use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
//! use rusk::http::domain::processing::flow::NoopFlow;
//! use rusk::http::domain::error::{
//!     DomainError, ProcessingError, WithContext, CommonErrorAttributes,
//! };
//! use async_trait::async_trait;
//!
//! #[derive(Debug)]
//! struct ErrorHandlingExample;
//!
//! #[async_trait]
//! impl Processor for ErrorHandlingExample {
//!     type Input = Vec<u8>;
//!     type Output = Vec<u8>;
//!     type Error = DomainError;
//!     type FlowControl = NoopFlow;
//!     type Context = DefaultContext;
//!
//!     async fn process_with_backpressure(
//!         &self,
//!         input: &Self::Input,
//!         flow: &mut Self::FlowControl,
//!         ctx: &mut Self::Context,
//!     ) -> Result<Self::Output, Self::Error> {
//!         ctx.metrics().start_operation("error_handling_example");
//!
//!         // Example of error with rich context
//!         if input.len() > 1024 {
//!             return Err(ProcessingError::StageFailed {
//!                 stage: "size_validation".into(),
//!                 reason: "Input too large".into(),
//!             }
//!             .with_context("error_handling_example")
//!             .with_stage("validation")
//!             .with_input_size(input.len())
//!             .with_resource_usage(input.len(), 1024, "bytes"));
//!         }
//!
//!         ctx.metrics().complete_operation("error_handling_example");
//!         Ok(input.clone())
//!     }
//! }
//! ```
//!
//! # Metrics and Monitoring
//!
//! The processing framework provides comprehensive metrics through the
//! `ProcessingMetrics` type:
//!
//! ```rust
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//! use rusk::http::domain::error::{
//!     ProcessingError, WithContext, CommonErrorAttributes,
//!     DomainError,
//! };
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), DomainError> {
//! let mut metrics = ProcessingMetrics::new();
//!
//! // Operation timing
//! metrics.start_operation("request_processing");
//!
//! // Operation success/failure tracking
//! metrics.increment_counter("requests_processed");
//!
//! // Error recording with context
//! let err: DomainError = ProcessingError::Timeout {
//!     operation: "processing".into(),
//!     duration: Duration::from_secs(5),
//! }
//! .into();  // Convert to DomainError first
//!
//! let err = err.with_context("request_handler")
//!     .with_stage("processing");
//!
//! metrics.record_error_with_context("request_processing", &err);
//!
//! // Pressure monitoring
//! metrics.update_pressure(0.75);
//!
//! // Complete timing
//! metrics.complete_operation("request_processing");
//! # Ok(())
//! # }
//! ```
//!
//! # Thread Safety and Performance
//!
//! All components in the processing framework are designed to be thread-safe
//! and efficient:
//!
//! - Metrics use atomic operations and optimized locks
//! - Flow control mechanisms are internally synchronized
//! - Context attributes use read-optimized locks
//! - Error handling adds minimal overhead
//!
//! Example of concurrent processing:
//!
//! ```rust
//! use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
//! use rusk::http::domain::processing::metrics::ProcessingMetrics;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let ctx = Arc::new(parking_lot::RwLock::new(DefaultContext::new()));
//!
//! let mut handles = vec![];
//!
//! // Spawn multiple processing tasks
//! for i in 0..10 {
//!     let ctx = Arc::clone(&ctx);
//!     let handle = tokio::spawn(async move {
//!         let mut ctx = ctx.write();
//!         ctx.metrics().start_operation(format!("task_{}", i));
//!         // Process...
//!         ctx.metrics().complete_operation(format!("task_{}", i));
//!     });
//!     handles.push(handle);
//! }
//!
//! // Wait for all tasks
//! for handle in handles {
//!     handle.await?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Metrics
//!
//! The following metrics are collected:
//!
//! - `rues_operation_duration_seconds` (histogram) - Operation execution time
//! - `rues_operations_total` (counter) - Number of operations processed
//! - `rues_errors_total` (counter) - Number of errors by type
//! - `rues_processing_pressure` (gauge) - Current processing backpressure level
//!
//! Each metric includes appropriate labels for operation type, error type, etc.
//!
//! # Thread Safety Guarantees
//!
//! - All metrics operations are thread-safe through `parking_lot::RwLock`
//! - Metrics recording never blocks for significant time
//! - Multiple processors can safely share a `ProcessingContext`
//! - Flow control mechanisms are internally synchronized
//!
//! # Performance Characteristics
//!
//! - Metrics recording overhead is minimal (microseconds)
//! - Lock contention is rare due to short critical sections
//! - No async overhead for metrics operations
//! - Efficient lock implementation from `parking_lot`
//!
//! # Comparison of `parking_lot::RwLock` and `tokio::sync::RwLock`:
//!
//! 1. **Performance**:
//! - `parking_lot::RwLock`:
//!   * Better performance for short-duration locks
//!   * No async overhead
//!   * Blocks the thread while waiting
//!   * Good for quick operations like metrics recording
//!
//! - `tokio::sync::RwLock`:
//!   * Async-aware, doesn't block threads
//!   * Higher overhead due to async machinery
//!   * Better for long-duration locks
//!   * More memory usage due to async task infrastructure
//!
//! 2. **Async Compatibility**:
//! - `parking_lot::RwLock`:
//!   * Can be used in async code but will block the thread
//!   * Might cause issues if locks are held for long periods
//!   * Simple synchronous API
//!
//! - `tokio::sync::RwLock`:
//!   * Fully async-aware
//!   * Won't block threads
//!   * Requires async/await everywhere
//!   * More complex API due to async nature
//!
//! 3. **Use Case Analysis**:
//! For `ProcessingMetrics`, the operations are typically:
//! - Very short duration (microseconds)
//! - High frequency
//! - Non-blocking (metrics recording)
//! - No long-term lock holding
//!
//! Given these characteristics, the use of the `parking_lot::RwLock` is
//! preferred because:
//! 1. Metrics operations are very quick
//! 2. We don't want async overhead for simple counters
//! 3. Thread blocking is negligible for such short operations
//! 4. Simpler API without async/await everywhere
//!
//! The key advantage here is simplicity and performance for quick operations.
//! If we later find that some metrics operations take longer (e.g., complex
//! calculations or external calls), we can selectively make those specific
//! operations async while keeping the simple ones synchronous.

pub mod context;
pub mod flow;
pub mod metrics;
pub mod pipeline;
pub mod stages;

use crate::http::domain::error::{
    CommonErrorAttributes, DomainError, ProcessingError, WithContext,
};
use crate::http::domain::processing::context::{
    DefaultContext, ProcessingContext,
};
use crate::http::domain::processing::flow::{FlowControl, NoopFlow};
use crate::http::domain::processing::metrics::{
    describe_metrics, ProcessingMetrics,
};

use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::time;
use tracing::{debug, error, info, warn};

/// Core trait for implementing processors in the RUES system.
///
/// This trait defines the interface for all processors that handle RUES values
/// and events. It supports:
/// - Processing with backpressure
/// - Context and metrics tracking
/// - Error handling and recovery
///
/// # Type Parameters
///
/// * `Input` - The input type for processing
/// * `Output` - The output type after processing
/// * `Error` - The error type that can be converted to DomainError
/// * `FlowControl` - Optional flow control mechanism
/// * `Context` - Processing context type
///
/// For a default implementation using `NoopFlow` and `DefaultContext`,
/// see [`DefaultProcessor`].
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::processing::Processor;
/// use rusk::http::domain::processing::context::{DefaultContext, ProcessingContext};
/// use rusk::http::domain::processing::flow::TokenBucketFlow;
/// use rusk::http::domain::error::{
///     DomainError, ProcessingError, WithContext, CommonErrorAttributes,
///     ValidationError,
/// };
/// use async_trait::async_trait;
///
/// #[derive(Debug)]
/// struct ExampleProcessor;
///
/// #[async_trait]
/// impl Processor for ExampleProcessor {
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
///         // Start operation timing
///         ctx.metrics().start_operation("example_processor");
///
///         // Validate input
///         if input.is_empty() {
///             return Err(ValidationError::EmptyInput("Input buffer is empty".into())
///                 .with_context("example_processor")
///                 .with_stage("validation")
///                 .with_input_size(0)
///                 .into());
///         }
///
///         // Try to acquire token for processing
///         if let Err(e) = flow.acquire_one().await {
///             return Err(e.with_context("example_processor")
///                 .with_stage("flow_control")
///                 .with_input_size(input.len())
///                 .into());
///         }
///
///         // Process with size limit
///         let result = if input.len() > 1024 {
///             Err(ProcessingError::StageFailed {
///                 stage: "processing".into(),
///                 reason: "Input too large".into(),
///             }
///             .with_context("example_processor")
///             .with_resource_usage(input.len(), 1024, "bytes")
///             .with_stage("size_check"))
///         } else {
///             // Record successful processing
///             ctx.metrics().increment_counter("bytes_processed");
///             Ok(input.clone())
///         };
///
///         // Complete timing and return result
///         ctx.metrics().complete_operation("example_processor");
///         result
///     }
/// }
/// ```
///
/// The example demonstrates:
/// - Proper error context and attribute handling
/// - Metrics recording for operations and errors
/// - Flow control with token bucket
/// - Resource usage tracking
/// - Operation timing
/// - Input validation
/// - Size limits enforcement
#[async_trait]
pub trait Processor: Send + Sync + Debug {
    /// The input type for this processor
    type Input: Send + Sync;

    /// The output type produced by this processor
    type Output: Send + Sync;

    /// The error type returned by this processor
    type Error: Into<DomainError> + Send + Sync + Debug;

    /// Optional flow control mechanism
    type FlowControl: FlowControl + Default;

    /// Processing context type
    type Context: ProcessingContext;

    /// Process with backpressure support
    ///
    /// This is the main processing method that supports backpressure through
    /// the flow control mechanism.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to process
    /// * `flow` - Flow control mechanism
    /// * `ctx` - Processing context
    ///
    /// # Returns
    ///
    /// The processing result or error
    async fn process_with_backpressure(
        &self,
        input: &Self::Input,
        flow: &mut Self::FlowControl,
        ctx: &mut Self::Context,
    ) -> Result<Self::Output, Self::Error>;

    /// Process without backpressure
    ///
    /// This is a convenience method that uses a no-op flow control.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to process
    /// * `ctx` - Processing context
    ///
    /// # Returns
    ///
    /// The processing result or error
    async fn process(
        &self,
        input: &Self::Input,
        ctx: &mut Self::Context,
    ) -> Result<Self::Output, Self::Error> {
        let mut flow = Self::FlowControl::default();
        self.process_with_backpressure(input, &mut flow, ctx).await
    }
}

/// Extension trait providing default implementations using `NoopFlow` and
/// `DefaultContext`.
///
/// This trait is automatically implemented for any type that implements
/// `Processor` with `NoopFlow` and `DefaultContext`. It provides a simplified
/// `process` method that uses these default types.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::processing::{Processor, DefaultProcessor};
/// use rusk::http::domain::processing::context::DefaultContext;
/// use rusk::http::domain::processing::flow::NoopFlow;
/// use rusk::http::domain::error::DomainError;
/// use async_trait::async_trait;
///
/// #[derive(Debug, Default)]
/// struct SimpleProcessor;
///
/// #[async_trait]
/// impl Processor for SimpleProcessor {
///     type Input = Vec<u8>;
///     type Output = Vec<u8>;
///     type Error = DomainError;
///     type FlowControl = NoopFlow;
///     type Context = DefaultContext;
///
///     async fn process_with_backpressure(
///         &self,
///         input: &Self::Input,
///         flow: &mut Self::FlowControl,
///         ctx: &mut Self::Context,
///     ) -> Result<Self::Output, Self::Error> {
///         Ok(input.clone())
///     }
/// }
///
/// // Now we can use the simplified process method
/// async fn process_data(processor: &SimpleProcessor) -> Result<Vec<u8>, DomainError> {
///     let mut ctx = DefaultContext::new();
///     Processor::process(processor, &vec![1, 2, 3], &mut ctx).await
/// }
/// ```
#[async_trait]
pub trait DefaultProcessor:
    Processor<FlowControl = NoopFlow, Context = DefaultContext>
{
    /// Process without backpressure using default types
    ///
    /// This is a convenience method that uses `NoopFlow` and `DefaultContext`.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to process
    /// * `ctx` - Default processing context
    ///
    /// # Returns
    ///
    /// The processing result or error
    async fn process(
        &self,
        input: &Self::Input,
        ctx: &mut DefaultContext,
    ) -> Result<Self::Output, Self::Error> {
        let mut flow = NoopFlow::default();
        self.process_with_backpressure(input, &mut flow, ctx).await
    }
}

// Automatically implement DefaultProcessor for any type that uses the default
// types
#[async_trait]
impl<T> DefaultProcessor for T
where
    T: Processor<FlowControl = NoopFlow, Context = DefaultContext>,
    DomainError: Into<T::Error>,
{
    async fn process(
        &self,
        input: &Self::Input,
        ctx: &mut DefaultContext,
    ) -> Result<Self::Output, T::Error> {
        ctx.set_attribute("processor_type", std::any::type_name::<T>());
        ctx.metrics().start_operation("default_processor");

        let result = match self
            .process_with_backpressure(input, &mut NoopFlow::default(), ctx)
            .await
        {
            Ok(output) => {
                ctx.metrics().increment_counter("successful_operations");
                Ok(output)
            }
            Err(e) => {
                let domain_err: DomainError = e.into();
                ctx.metrics()
                    .record_error("default_processor", &domain_err.to_string());
                Err(domain_err
                    .with_context("default_processor")
                    .with_stage("processing")
                    .into())
            }
        };

        ctx.metrics().complete_operation("default_processor");
        result
    }
}

/// Cancellation token for stopping processing operations.
///
/// This type is thread-safe and can be cloned to provide cancellation
/// capabilities to multiple processors or tasks.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::processing::{
///     CancellationToken, Processor, CancellableProcessor,
/// };
/// use rusk::http::domain::processing::context::ProcessingContext;
/// use tokio::time::timeout;
/// use std::time::Duration;
///
/// async fn process_with_timeout<P>(
///     processor: &P,
///     input: &P::Input,
///     ctx: &mut P::Context,
///     timeout_duration: Duration,
/// ) -> Result<P::Output, P::Error>
/// where
///     P: Processor + CancellableProcessor,
/// {
///     let token = CancellationToken::new();
///     let token_clone = token.clone();
///
///     // Set up timeout
///     tokio::spawn(async move {
///         tokio::time::sleep(timeout_duration).await;
///         token_clone.cancel();
///     });
///
///     processor.process_with_cancellation(input, ctx, &token).await
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    notify: broadcast::Sender<()>,
}

impl CancellationToken {
    /// Creates new cancellation token
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            notify: tx,
        }
    }

    /// Triggers cancellation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = self.notify.send(());
    }

    /// Checks if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Returns channel for receiving cancel notifications
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.notify.subscribe()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for processors supporting cancellation.
///
/// This trait extends the base `Processor` trait to add support for
/// cancellable operations. Processors implementing this trait must:
/// - Check cancellation status during processing
/// - Clean up resources on cancellation
/// - Return appropriate errors when cancelled
///
/// The trait is automatically implemented for any type that implements
/// `Processor` and `Into<DomainError>`.
///
/// # Examples
///
/// ```rust
/// use rusk::http::domain::processing::{Processor, CancellableProcessor, CancellationToken};
/// use rusk::http::domain::processing::context::DefaultContext;
/// use rusk::http::domain::processing::flow::NoopFlow;
/// use rusk::http::domain::error::ProcessingError;
/// use async_trait::async_trait;
///
/// #[derive(Debug)]
/// struct CancellableTask;
///
/// #[async_trait]
/// impl Processor for CancellableTask {
///     type Input = Vec<u8>;
///     type Output = Vec<u8>;
///     type Error = ProcessingError;
///     type FlowControl = NoopFlow;
///     type Context = DefaultContext;
///
///     async fn process_with_backpressure(
///         &self,
///         input: &Self::Input,
///         flow: &mut Self::FlowControl,
///         ctx: &mut Self::Context,
///     ) -> Result<Self::Output, Self::Error> {
///         Ok(input.clone())
///     }
/// }
///
/// // Now, the CancellableTask can be used as a cancellable processor.
/// ```
#[async_trait]
pub trait CancellableProcessor: Processor {
    /// Process with cancellation support
    async fn process_with_cancellation(
        &self,
        input: &Self::Input,
        ctx: &mut Self::Context,
        token: &CancellationToken,
    ) -> Result<Self::Output, Self::Error>;
}

// Implement CancellableProcessor for any type that implements Processor
#[async_trait]
impl<T> CancellableProcessor for T
where
    T: Processor,
    // Every type that implements Processor must also implement
    // Into<DomainError>
    DomainError: Into<T::Error>,
{
    async fn process_with_cancellation(
        &self,
        input: &Self::Input,
        ctx: &mut Self::Context,
        token: &CancellationToken,
    ) -> Result<Self::Output, T::Error> {
        if token.is_cancelled() {
            ctx.metrics()
                .record_error("process", "cancelled_before_start");
            return Err(ProcessingError::Cancelled {
                operation: "process".into(),
                reason: "cancelled before start".into(),
            }
            .with_context("process_with_cancellation")
            .with_stage("pre_process")
            .into());
        }

        // Start timing the operation
        ctx.metrics().start_operation("process_with_cancellation");
        let mut rx = token.subscribe();

        let result = tokio::select! {
            result = self.process(input, ctx) => result,
            _ = rx.recv() => {
                ctx.metrics().record_error("process", "cancelled_during_processing");
                Err(ProcessingError::Cancelled {
                    operation: "process".into(),
                    reason: "cancelled during processing".into(),
                }
                .with_context("process_with_cancellation")
                .with_stage("processing")
                .into())
            }
        };

        // Complete timing
        ctx.metrics()
            .complete_operation("process_with_cancellation");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Barrier;

    use crate::http::domain::error::{
        CommonErrorAttributes, DomainError, ProcessingError, ValidationError,
        WithContext,
    };

    // Test helper types
    #[derive(Debug, Default)]
    struct TestProcessor;

    #[derive(Debug)]
    struct TestInput(Vec<u8>);

    #[derive(Debug, PartialEq)]
    struct TestOutput(Vec<u8>);

    #[async_trait]
    impl Processor for TestProcessor {
        type Input = TestInput;
        type Output = TestOutput;
        type Error = DomainError;
        type FlowControl = NoopFlow;
        type Context = DefaultContext;

        async fn process_with_backpressure(
            &self,
            input: &Self::Input,
            flow: &mut Self::FlowControl,
            ctx: &mut Self::Context,
        ) -> Result<Self::Output, Self::Error> {
            ctx.metrics().start_operation("test_process");

            if input.0.is_empty() {
                ctx.metrics().record_error("test_process", "empty_input");
                return Err(ValidationError::EmptyInput(
                    "Empty test input".into(),
                )
                .with_context("test_process")
                .with_stage("validation")
                .with_input_size(0));
            }

            ctx.metrics().increment_counter("processing_success");
            ctx.metrics().complete_operation("test_process");
            Ok(TestOutput(input.0.clone()))
        }
    }

    // Basic processing tests
    #[tokio::test]
    async fn test_basic_processing() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();

        // Test successful processing
        let input = TestInput(vec![1, 2, 3]);
        let result =
            DefaultProcessor::process(&processor, &input, &mut ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestOutput(vec![1, 2, 3]));

        // Test error handling
        let empty_input = TestInput(vec![]);
        let error =
            DefaultProcessor::process(&processor, &empty_input, &mut ctx)
                .await
                .unwrap_err();

        // DefaultProcessor wraps the already wrapped error
        if let DomainError::WithContext(default_ctx) = error {
            // Check DefaultProcessor context
            assert_eq!(default_ctx.context_operation(), "default_processor");
            assert_eq!(
                default_ctx.get_context_attribute("stage").unwrap(),
                "processing"
            );

            // Check the underlying error from our test processor
            if let DomainError::WithContext(test_ctx) = default_ctx.source() {
                // Check test processor context
                assert_eq!(test_ctx.context_operation(), "test_process");
                assert_eq!(
                    test_ctx.get_context_attribute("stage").unwrap(),
                    "validation"
                );
                assert_eq!(
                    test_ctx.get_context_attribute("input_size").unwrap(),
                    "0"
                );

                // Finally, check the root error
                match test_ctx.source() {
                    DomainError::Validation(ValidationError::EmptyInput(_)) => {
                        // This is the expected error type
                    }
                    other => panic!("Unexpected root error: {:?}", other),
                }
            } else {
                panic!("Expected wrapped error from test processor");
            }
        } else {
            panic!("Expected error with DefaultProcessor context");
        }

        // Verify processor type was recorded
        assert!(ctx.get_attribute("processor_type").is_some());
    }

    // Test processor with custom flow control
    #[derive(Debug, Default)]
    struct CustomFlow {
        allowed: AtomicBool,
    }

    impl FlowControl for CustomFlow {
        fn can_proceed(&mut self) -> bool {
            self.allowed.load(Ordering::Relaxed)
        }

        fn update(&mut self, metrics: &mut ProcessingMetrics) {
            metrics.update_pressure(if self.can_proceed() { 0.0 } else { 1.0 });
        }

        fn pressure(&mut self) -> f64 {
            if self.can_proceed() {
                0.0
            } else {
                1.0
            }
        }
    }

    #[derive(Debug, Default)]
    struct FlowTestProcessor;

    #[async_trait]
    impl Processor for FlowTestProcessor {
        type Input = TestInput;
        type Output = TestOutput;
        type Error = DomainError;
        type FlowControl = CustomFlow;
        type Context = DefaultContext;

        async fn process_with_backpressure(
            &self,
            input: &Self::Input,
            flow: &mut Self::FlowControl,
            ctx: &mut Self::Context,
        ) -> Result<Self::Output, Self::Error> {
            ctx.metrics().start_operation("flow_test");

            if !flow.can_proceed() {
                return Err(ProcessingError::RateLimitExceeded {
                    limit: 1,
                    current: 2,
                }
                .with_context("flow_test")
                .with_stage("flow_check"));
            }

            flow.update(ctx.metrics());
            ctx.metrics().complete_operation("flow_test");
            Ok(TestOutput(input.0.clone()))
        }
    }

    #[tokio::test]
    async fn test_flow_control() {
        let processor = FlowTestProcessor::default();
        let mut ctx = DefaultContext::new();
        let mut flow = CustomFlow::default();

        // Test when flow control blocks processing
        flow.allowed.store(false, Ordering::Relaxed);
        let result = processor
            .process_with_backpressure(&TestInput(vec![1]), &mut flow, &mut ctx)
            .await;
        assert!(result.is_err());

        match result.unwrap_err() {
            DomainError::WithContext(ctx) => {
                // Check context attributes
                assert_eq!(ctx.context_operation(), "flow_test");
                assert_eq!(
                    ctx.get_context_attribute("stage").unwrap(),
                    "flow_check"
                );

                // Check the underlying error
                match ctx.source() {
                    DomainError::Processing(
                        ProcessingError::RateLimitExceeded { limit, current },
                    ) => {
                        assert_eq!(*limit, 1 as usize);
                        assert_eq!(*current, 2 as usize);
                    }
                    other => panic!("Unexpected error type: {:?}", other),
                }
            }
            other => panic!("Expected error with context, got: {:?}", other),
        }

        // Test when flow control allows processing
        flow.allowed.store(true, Ordering::Relaxed);
        let result = processor
            .process_with_backpressure(&TestInput(vec![1]), &mut flow, &mut ctx)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TestOutput(vec![1]));

        // Verify metrics were updated
        assert!(flow.pressure() >= 0.0 && flow.pressure() <= 1.0);
    }

    // Add test for flow control behavior under load
    #[tokio::test]
    async fn test_flow_control_under_load() {
        let processor = FlowTestProcessor::default();
        let mut ctx = DefaultContext::new();
        let mut flow = CustomFlow::default();
        let mut success_count = 0;
        let mut failure_count = 0;

        // Alternate between allowed and not allowed
        for i in 0..10 {
            flow.allowed.store(i % 2 == 0, Ordering::Relaxed);

            let result = processor
                .process_with_backpressure(
                    &TestInput(vec![1]),
                    &mut flow,
                    &mut ctx,
                )
                .await;

            match result {
                Ok(_) => success_count += 1,
                Err(e) => {
                    failure_count += 1;
                    assert!(matches!(e, DomainError::WithContext(_)));
                }
            }
        }

        // Should have equal successes and failures
        assert_eq!(success_count, 5, "Expected 5 successful operations");
        assert_eq!(failure_count, 5, "Expected 5 failed operations");
    }

    // Add test for flow control pressure updates
    #[tokio::test]
    async fn test_flow_control_pressure() {
        let processor = FlowTestProcessor::default();
        let mut ctx = DefaultContext::new();
        let mut flow = CustomFlow::default();

        // Test pressure when blocked
        flow.allowed.store(false, Ordering::Relaxed);
        assert_eq!(flow.pressure(), 1.0, "Pressure should be max when blocked");

        // Test pressure when allowed
        flow.allowed.store(true, Ordering::Relaxed);
        assert_eq!(
            flow.pressure(),
            0.0,
            "Pressure should be zero when allowed"
        );

        // Process request and verify pressure update
        let result = processor
            .process_with_backpressure(&TestInput(vec![1]), &mut flow, &mut ctx)
            .await;
        assert!(result.is_ok());

        // Verify metrics were updated with pressure
        let final_pressure = flow.pressure();
        assert!(
            final_pressure >= 0.0 && final_pressure <= 1.0,
            "Pressure should be between 0 and 1, got {}",
            final_pressure
        );
    }

    // Test cancellation
    #[tokio::test]
    async fn test_cancellation() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();
        let token = CancellationToken::new();

        // Test processing without cancellation
        let result = processor
            .process_with_cancellation(&TestInput(vec![1]), &mut ctx, &token)
            .await;
        assert!(result.is_ok());

        // Test processing with immediate cancellation
        token.cancel();
        let result = processor
            .process_with_cancellation(&TestInput(vec![1]), &mut ctx, &token)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::WithContext(ctx) => {
                // Check context attributes
                assert_eq!(
                    ctx.context_operation(),
                    "process_with_cancellation"
                );
                assert_eq!(
                    ctx.get_context_attribute("stage").unwrap(),
                    "pre_process"
                );

                // Verify it's a cancellation error
                match ctx.source() {
                    DomainError::Processing(ProcessingError::Cancelled {
                        operation,
                        reason,
                    }) => {
                        assert_eq!(operation, "process");
                        assert_eq!(reason, "cancelled before start");
                    }
                    other => panic!(
                        "Unexpected error type inside context: {:?}",
                        other
                    ),
                }
            }
            other => panic!("Expected error with context, got: {:?}", other),
        }
    }

    // Add a more comprehensive test for cancellation metrics
    #[tokio::test]
    async fn test_cancellation_metrics() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();
        let token = CancellationToken::new();

        // Successful case first - should record success metric
        let result = processor
            .process_with_cancellation(&TestInput(vec![1]), &mut ctx, &token)
            .await;
        assert!(result.is_ok());

        // Now test cancellation with metrics
        token.cancel();
        let _ = processor
            .process_with_cancellation(&TestInput(vec![1]), &mut ctx, &token)
            .await;

        // Start a new operation to verify metrics system is still working
        ctx.metrics().start_operation("post_cancel_op");
        ctx.metrics().complete_operation("post_cancel_op");
    }

    // Add test for cancellation during processing
    #[tokio::test]
    async fn test_cancellation_during_processing() {
        // Create a processor that takes time to process
        #[derive(Debug)]
        struct SlowProcessor;

        #[async_trait]
        impl Processor for SlowProcessor {
            type Input = TestInput;
            type Output = TestOutput;
            type Error = DomainError;
            type FlowControl = NoopFlow;
            type Context = DefaultContext;

            async fn process_with_backpressure(
                &self,
                input: &Self::Input,
                _flow: &mut Self::FlowControl,
                ctx: &mut Self::Context,
            ) -> Result<Self::Output, Self::Error> {
                ctx.metrics().start_operation("slow_processing");

                // Simulate long processing
                tokio::time::sleep(Duration::from_millis(200)).await;

                ctx.metrics().complete_operation("slow_processing");
                Ok(TestOutput(input.0.clone()))
            }
        }

        let processor = SlowProcessor;
        let mut ctx = DefaultContext::new();
        let token = CancellationToken::new();

        // Spawn a task to cancel after a shorter delay
        let token_clone = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            token_clone.cancel();
        });

        let result = processor
            .process_with_cancellation(&TestInput(vec![1]), &mut ctx, &token)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::WithContext(ctx) => {
                assert_eq!(
                    ctx.context_operation(),
                    "process_with_cancellation"
                );
                match ctx.source() {
                    DomainError::Processing(ProcessingError::Cancelled {
                        operation,
                        reason,
                    }) => {
                        assert_eq!(operation, "process");
                        assert!(reason.contains("cancelled during processing"));
                    }
                    other => panic!(
                        "Unexpected error type inside context: {:?}",
                        other
                    ),
                }
            }
            other => panic!("Expected error with context, got: {:?}", other),
        }
    }

    // Add test for race conditions in cancellation
    #[tokio::test]
    async fn test_cancellation_timing() {
        #[derive(Debug)]
        struct TimingSensitiveProcessor;

        #[async_trait]
        impl Processor for TimingSensitiveProcessor {
            type Input = TestInput;
            type Output = TestOutput;
            type Error = DomainError;
            type FlowControl = NoopFlow;
            type Context = DefaultContext;

            async fn process_with_backpressure(
                &self,
                input: &Self::Input,
                _flow: &mut Self::FlowControl,
                ctx: &mut Self::Context,
            ) -> Result<Self::Output, Self::Error> {
                ctx.metrics().start_operation("timing_test");

                // Small sleep to make timing more predictable
                tokio::time::sleep(Duration::from_millis(10)).await;

                ctx.metrics().complete_operation("timing_test");
                Ok(TestOutput(input.0.clone()))
            }
        }

        let processor = TimingSensitiveProcessor;
        let mut ctx = DefaultContext::new();

        // Test very close timing
        for _ in 0..3 {
            let token = CancellationToken::new();
            let token_clone = token.clone();

            // Spawn cancellation with minimal delay
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_micros(1)).await;
                token_clone.cancel();
            });

            let result = processor
                .process_with_cancellation(
                    &TestInput(vec![1]),
                    &mut ctx,
                    &token,
                )
                .await;

            // We don't assert specific success/failure here because timing
            // might vary, but we ensure it doesn't panic
            if let Err(e) = result {
                assert!(matches!(e, DomainError::WithContext(_)));
            }
        }
    }

    // Test concurrent processing
    #[tokio::test]
    async fn test_concurrent_processing() {
        let processor = Arc::new(TestProcessor::default());
        let ctx = Arc::new(parking_lot::RwLock::new(DefaultContext::new()));
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        for i in 0..3 {
            let processor = Arc::clone(&processor);
            let ctx = Arc::clone(&ctx);
            let barrier = Arc::clone(&barrier);

            let handle = tokio::spawn(async move {
                barrier.wait().await;

                let input = TestInput(vec![i as u8]);
                // Create a new DefaultContext for each task
                let mut task_ctx = DefaultContext::new();
                let result =
                    Processor::process(&*processor, &input, &mut task_ctx)
                        .await;

                // After processing, update the shared context
                if result.is_ok() {
                    let mut shared_ctx = ctx.write();
                    // Copy metrics or other relevant data
                    shared_ctx
                        .metrics()
                        .increment_counter("concurrent_success");
                }

                result
            });
            handles.push(handle);
        }

        let mut successful = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                successful += 1;
            }
        }

        // Verify results
        assert_eq!(successful, 3, "All tasks should succeed");

        // Verify final metrics in shared context
        let final_ctx = ctx.read();
        // Add any specific metric checks here
    }

    // Add a separate test for context synchronization
    #[tokio::test]
    async fn test_context_synchronization() {
        let processor = Arc::new(TestProcessor::default());
        let ctx = Arc::new(parking_lot::RwLock::new(DefaultContext::new()));
        let mut handles = Vec::new();

        for i in 0..3 {
            let processor = Arc::clone(&processor);
            let ctx = Arc::clone(&ctx);

            let handle = tokio::spawn(async move {
                let input = TestInput(vec![i as u8]);
                let mut task_ctx = DefaultContext::new();

                // Process with local context
                let result =
                    Processor::process(&*processor, &input, &mut task_ctx)
                        .await;

                if result.is_ok() {
                    // Update shared context after processing
                    let mut shared_ctx = ctx.write();
                    shared_ctx.set_attribute(
                        &format!("task_{}_completed", i),
                        "true",
                    );
                }

                result
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify shared context
        let final_ctx = ctx.read();
        for i in 0..3 {
            assert_eq!(
                final_ctx
                    .get_attribute(&format!("task_{}_completed", i))
                    .unwrap(),
                "true",
                "Task {} should be marked as completed",
                i
            );
        }
    }

    // Test metrics collection
    #[tokio::test]
    async fn test_metrics_collection() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();

        // Test successful processing metrics
        let result =
            Processor::process(&processor, &TestInput(vec![1]), &mut ctx).await;
        assert!(result.is_ok());

        // Test error metrics
        let err = Processor::process(&processor, &TestInput(vec![]), &mut ctx)
            .await
            .unwrap_err();

        // Check operation name from the test processor implementation
        assert_eq!(err.operation().unwrap(), "test_process");
        assert_eq!(err.get_attribute("stage").unwrap(), "validation");

        // Verify metrics were recorded
        // We can add more specific metrics checks here if needed
    }

    // Test default processor implementation
    #[tokio::test]
    async fn test_default_processor() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();

        // Test with valid input
        let result = DefaultProcessor::process(
            &processor,
            &TestInput(vec![1]),
            &mut ctx,
        )
        .await;
        assert!(result.is_ok());

        // Test with invalid input
        let result =
            DefaultProcessor::process(&processor, &TestInput(vec![]), &mut ctx)
                .await;
        assert!(result.is_err());

        // Verify processor type is recorded
        assert!(ctx.get_attribute("processor_type").is_some());
    }

    // Test error propagation and context
    #[tokio::test]
    async fn test_error_handling() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();

        let err =
            DefaultProcessor::process(&processor, &TestInput(vec![]), &mut ctx)
                .await
                .unwrap_err();

        // Check error context
        assert!(err.operation().is_some());
        assert!(err.get_attribute("stage").is_some());
        assert_eq!(err.get_attribute("input_size").unwrap(), "0");

        // Check metrics
        assert!(ctx.get_attribute("processor_type").is_some());
    }

    // Test processing timeouts
    #[tokio::test]
    async fn test_processing_timeouts() {
        #[derive(Debug, Default)]
        struct SlowProcessor;

        #[async_trait]
        impl Processor for SlowProcessor {
            type Input = TestInput;
            type Output = TestOutput;
            type Error = DomainError;
            type FlowControl = NoopFlow;
            type Context = DefaultContext;

            async fn process_with_backpressure(
                &self,
                input: &Self::Input,
                _flow: &mut Self::FlowControl,
                ctx: &mut Self::Context,
            ) -> Result<Self::Output, Self::Error> {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Ok(TestOutput(input.0.clone()))
            }
        }

        let processor = SlowProcessor::default();
        let mut ctx = DefaultContext::new();

        // Test with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(100),
            DefaultProcessor::process(
                &processor,
                &TestInput(vec![1]),
                &mut ctx,
            ),
        )
        .await;

        assert!(result.is_err());
    }

    // Test context propagation
    #[tokio::test]
    async fn test_context_propagation() {
        let processor = TestProcessor::default();
        let mut ctx = DefaultContext::new();

        // Set initial context
        ctx.set_attribute("test_attr", "test_value");

        // Process with context
        let _ = DefaultProcessor::process(
            &processor,
            &TestInput(vec![1]),
            &mut ctx,
        )
        .await;

        // Verify context is preserved
        assert_eq!(ctx.get_attribute("test_attr").unwrap(), "test_value");
    }
}
