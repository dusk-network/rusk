# Type State Builder Pattern

The type state builder pattern is a design pattern that allows you to build types reliably and ensure that required fields are set before the type is built at compile time instead of runtime.

## Example

```rust
use log::{Level, LevelFilter};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Stage execution context with thread-safe shared state
pub struct StageContext {
    /// Unique pipeline execution ID
    pipeline_id: Uuid,
    /// Whether metrics collection is enabled
    metrics_enabled: bool,
    /// Minimum log level for this context
    log_level: LevelFilter,
    /// Shared mutable state
    state: Arc<RwLock<ContextState>>,
}

/// Mutable state shared between stages
#[derive(Default)]
struct ContextState {
    /// Stage execution metadata
    metadata: HashMap<String, String>,
}

// Type states for type-state builder pattern.
//
// When the required field `pipeline_id` is not set, the state is
// `NoPipelineId`. After setting the required field, the state transitions to
// `WithPipelineId`.
#[derive(Debug)]
pub struct NoPipelineId;
#[derive(Debug)]
pub struct WithPipelineId(Uuid);

/// Builder for StageContext
pub struct StageContextBuilder<T> {
    state: T,
    metrics_enabled: bool,
    log_level: LevelFilter,
}

impl StageContext {
    /// Create a new builder for StageContext with the `NoPipelineId` state
    pub fn builder() -> StageContextBuilder<NoPipelineId> {
        StageContextBuilder {
            state: NoPipelineId,
            metrics_enabled: false,
            log_level: LevelFilter::Info, // Default to Info level
        }
    }

    /// Get pipeline ID
    pub fn pipeline_id(&self) -> Uuid {
        self.pipeline_id
    }

    /// Record a counter metric if metrics are enabled
    pub fn increment_counter(&self, key: &str) {
        if self.metrics_enabled {
            metrics::counter!(key, 1);
        }
    }

    /// Record a gauge value if metrics are enabled
    pub fn record_gauge(&self, key: &str, value: f64) {
        if let Some(recorder) = &self.metrics {
            recorder.record_gauge(&metrics::Key::from_name(key), value);
        }
    }

    /// Record a histogram value if metrics are enabled
    pub fn record_histogram(&self, key: &str, value: f64) {
        if let Some(recorder) = &self.metrics {
            recorder.record_histogram(&metrics::Key::from_name(key), value);
        }
    }

    /// Log a message if level is enabled
    pub fn log(&self, level: Level, message: &str) {
        if level <= self.log_level {
            log::log!(
                target: &format!("pipeline_{}", self.pipeline_id),
                level,
                "{}",
                message
            );
        }
    }

    /// Get read access to shared state
    pub fn state(&self) -> std::sync::RwLockReadGuard<'_, ContextState> {
        self.state.read().unwrap()
    }

    /// Get write access to shared state
    pub fn state_mut(&self) -> std::sync::RwLockWriteGuard<'_, ContextState> {
        self.state.write().unwrap()
    }
}

// This is the type-state builder pattern. All methods except `build` are available
// because we want to be able to use them in all states. We use the generic type `T`
// to represent all possible states of the builder.
//
// We don't have a `build` method for the StageContextBuilder<T> because the
// required field `pipeline_id` is not set yet. After setting the required
// field, we return a `StageContextBuilder<WithPipelineId>` which has a `build`
// method to create the final `StageContext`. With the type-state builder
// pattern we make impossible states unrepresentable and check required fields
// at compile time instead of runtime.
impl<T> StageContextBuilder<T> {
    /// Set metrics recorder
    pub fn metrics(mut self, metrics: Arc<dyn Recorder>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Set log level
    pub fn log_level(mut self, level: LevelFilter) -> Self {
        self.log_level = level;
        self
    }

    /// Set required pipeline ID (transitions state)
    pub fn pipeline_id(self, id: Uuid) -> StageContextBuilder<WithPipelineId> {
        StageContextBuilder {
            state: WithPipelineId(id),
            metrics: self.metrics,
            log_level: self.log_level,
        }
    }
}

// The `build` method is available for the
// `StageContextBuilder<WithPipelineId>` type only that we get after setting the
// required field `pipeline_id`.
impl StageContextBuilder<WithPipelineId> {
    /// Build the final StageContext
    pub fn build(self) -> StageContext {
        StageContext {
            pipeline_id: self.state.0,
            metrics: self.metrics,
            log_level: self.log_level,
            state: Arc::new(RwLock::new(ContextState::default())),
        }
    }
}
```

## How the Builder Pattern Works

- It uses generic type parameters on the builder (`StageContextBuilder<T>`) to represent different states (e.g., `NoPipelineId`, `WithPipelineId`).
- The initial `builder()` method returns the builder in the "missing required fields" state (e.g., `StageContextBuilder<NoPipelineId>`).
- Methods for setting optional fields are implemented on the generic builder (`impl<T> StageContextBuilder<T>`). They return `self`.
- Methods for setting required fields take `self` and return a builder in the next state (e.g., `pipeline_id(self, id: Uuid) -> StageContextBuilder<WithPipelineId>`).
- The final `build()` method is implemented only on the builder state where all required fields are set (e.g., `impl StageContextBuilder<WithPipelineId>`).
  This ensures at compile time that `build()` cannot be called until all required fields are provided.
