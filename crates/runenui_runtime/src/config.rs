//! Runtime queue and trace configuration.

use crate::TraceConfig;

/// Runtime limits and trace configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConfig {
    queue_capacity: usize,
    trace_config: TraceConfig,
}

impl RuntimeConfig {
    /// Returns this configuration with a different waiting-envelope capacity.
    #[must_use]
    pub const fn with_queue_capacity(mut self, capacity: usize) -> Self {
        self.queue_capacity = capacity;
        self
    }

    /// Returns this configuration with different canonical trace retention.
    #[must_use]
    pub const fn with_trace_config(mut self, trace_config: TraceConfig) -> Self {
        self.trace_config = trace_config;
        self
    }

    /// Returns the maximum number of waiting envelopes.
    #[must_use]
    pub const fn queue_capacity(self) -> usize {
        self.queue_capacity
    }

    /// Returns the canonical trace configuration.
    #[must_use]
    pub const fn trace_config(self) -> TraceConfig {
        self.trace_config
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 1024,
            trace_config: TraceConfig::new(1024),
        }
    }
}
