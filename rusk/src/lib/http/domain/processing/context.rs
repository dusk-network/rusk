use tracing::debug;

use crate::http::domain::ProcessingMetrics;
use std::collections::HashMap;

/// Processing context that tracks state and metrics during processing
/// operations.
///
/// The context provides two key features:
///
/// 1. **Metrics Collection** (`metrics` field):
///    - Operation timing and duration tracking
///    - Counter tracking (operations, errors)
///    - Pressure monitoring for backpressure
///    - Thread-safe through `parking_lot::RwLock`
///
/// 2. **Contextual Attributes** (`attributes` field): Attributes provide a way
///    to:
///    - Track processing state and metadata across pipeline stages
///    - Pass contextual information between processors
///    - Collect debugging and monitoring information
///    - Enable tracing and correlation of related operations
///
/// # Common Attribute Use Cases
///
/// - Request correlation IDs
/// - Processing timestamps
/// - Operation metadata
/// - Debug flags
/// - Performance metrics
/// - Resource usage tracking
///
/// # Thread Safety
///
/// Both metrics and attributes are protected by `parking_lot::RwLock` for
/// thread safety. The choice of `parking_lot::RwLock` over
/// `tokio::sync::RwLock` is made because:
/// - Operations are typically very short-duration (microseconds)
/// - High-frequency operations benefit from `parking_lot`'s optimized locking
/// - No need for async lock semantics as operations never block for long
/// - Lower memory overhead compared to async locks
pub trait ProcessingContext: Send + Sync {
    /// Get mutable reference to metrics collection
    fn metrics(&mut self) -> &mut ProcessingMetrics;

    /// Get attribute value
    fn get_attribute(&self, key: &str) -> Option<String>;

    /// Set attribute value
    fn set_attribute(&self, key: impl Into<String>, value: impl Into<String>);

    /// Remove attribute
    fn remove_attribute(&self, key: &str) -> Option<String>;

    /// Check if attribute exists
    fn has_attribute(&self, key: &str) -> bool;
}

/// Default processing context implementation.
#[derive(Debug)]
pub struct DefaultContext {
    /// Processing metrics
    metrics: ProcessingMetrics,
    /// Context attributes
    attributes: parking_lot::RwLock<HashMap<String, String>>,
}

impl DefaultContext {
    /// Creates new default context
    pub fn new() -> Self {
        Self {
            metrics: ProcessingMetrics::new(),
            attributes: parking_lot::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for DefaultContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingContext for DefaultContext {
    fn metrics(&mut self) -> &mut ProcessingMetrics {
        &mut self.metrics
    }

    fn get_attribute(&self, key: &str) -> Option<String> {
        let result = self.attributes.read().get(key).cloned();
        if result.is_none() {
            debug!("Attribute not found: {}", key);
        }
        result
    }

    fn set_attribute(&self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        debug!("Setting attribute: {} = {}", key, value);
        self.attributes.write().insert(key, value);
    }

    fn remove_attribute(&self, key: &str) -> Option<String> {
        let result = self.attributes.write().remove(key);
        if result.is_some() {
            debug!("Removed attribute: {}", key);
        }
        result
    }

    fn has_attribute(&self, key: &str) -> bool {
        self.attributes.read().contains_key(key)
    }
}
