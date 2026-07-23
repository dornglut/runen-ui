//! Runtime queue, surface retention, and trace configuration.

use core::num::NonZeroUsize;

use crate::TraceConfig;

pub const DEFAULT_RUNTIME_LIMIT: usize = 1024;
const DEFAULT_WAITING_ENVELOPE_LIMIT: usize = DEFAULT_RUNTIME_LIMIT * 4;
const DEFAULT_SURFACE_SNAPSHOT_RETENTION: usize = 2;

/// Logical bounded capacities used by runtime-owned scheduling facilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLimits {
    waiting_envelopes: usize,
    transaction_outputs: usize,
    pointer_streams: usize,
    local_tasks: usize,
    send_tasks: usize,
    timers: usize,
    subscriptions: usize,
    host_requests: usize,
    completion_ingress: usize,
    subscription_diagnostics: usize,
}

impl RuntimeLimits {
    #[must_use]
    pub const fn with_waiting_envelopes(mut self, limit: usize) -> Self {
        self.waiting_envelopes = limit;
        self
    }

    #[must_use]
    pub const fn with_transaction_outputs(mut self, limit: usize) -> Self {
        self.transaction_outputs = limit;
        self
    }

    /// Returns these limits with a different active pointer-stream capacity.
    #[must_use]
    pub const fn with_pointer_streams(mut self, limit: usize) -> Self {
        self.pointer_streams = limit;
        self
    }

    #[must_use]
    pub const fn with_local_tasks(mut self, limit: usize) -> Self {
        self.local_tasks = limit;
        self
    }

    #[must_use]
    pub const fn with_send_tasks(mut self, limit: usize) -> Self {
        self.send_tasks = limit;
        self
    }

    #[must_use]
    pub const fn with_timers(mut self, limit: usize) -> Self {
        self.timers = limit;
        self
    }

    #[must_use]
    pub const fn with_subscriptions(mut self, limit: usize) -> Self {
        self.subscriptions = limit;
        self
    }

    #[must_use]
    pub const fn with_host_requests(mut self, limit: usize) -> Self {
        self.host_requests = limit;
        self
    }

    #[must_use]
    pub const fn with_completion_ingress(mut self, limit: usize) -> Self {
        self.completion_ingress = limit;
        self
    }

    #[must_use]
    pub const fn with_subscription_diagnostics(mut self, limit: usize) -> Self {
        self.subscription_diagnostics = limit;
        self
    }

    #[must_use]
    pub const fn waiting_envelopes(self) -> usize {
        self.waiting_envelopes
    }

    #[must_use]
    pub const fn transaction_outputs(self) -> usize {
        self.transaction_outputs
    }

    /// Returns the maximum number of active pointer streams.
    #[must_use]
    pub const fn pointer_streams(self) -> usize {
        self.pointer_streams
    }

    #[must_use]
    pub const fn local_tasks(self) -> usize {
        self.local_tasks
    }

    #[must_use]
    pub const fn send_tasks(self) -> usize {
        self.send_tasks
    }

    #[must_use]
    pub const fn timers(self) -> usize {
        self.timers
    }

    #[must_use]
    pub const fn subscriptions(self) -> usize {
        self.subscriptions
    }

    #[must_use]
    pub const fn host_requests(self) -> usize {
        self.host_requests
    }

    #[must_use]
    pub const fn completion_ingress(self) -> usize {
        self.completion_ingress
    }

    #[must_use]
    pub const fn subscription_diagnostics(self) -> usize {
        self.subscription_diagnostics
    }
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            waiting_envelopes: DEFAULT_WAITING_ENVELOPE_LIMIT,
            transaction_outputs: DEFAULT_RUNTIME_LIMIT,
            pointer_streams: DEFAULT_RUNTIME_LIMIT,
            local_tasks: DEFAULT_RUNTIME_LIMIT * 2,
            send_tasks: DEFAULT_RUNTIME_LIMIT * 2,
            timers: DEFAULT_RUNTIME_LIMIT * 2,
            subscriptions: DEFAULT_RUNTIME_LIMIT * 2,
            host_requests: DEFAULT_RUNTIME_LIMIT * 2,
            completion_ingress: DEFAULT_RUNTIME_LIMIT,
            subscription_diagnostics: DEFAULT_RUNTIME_LIMIT,
        }
    }
}

/// Runtime limits, displayed-surface retention, and trace configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConfig {
    limits: RuntimeLimits,
    trace_config: TraceConfig,
    surface_snapshot_retention: usize,
    #[cfg(feature = "internal-test-seams")]
    initial_next_work_sequence: u64,
    #[cfg(feature = "internal-test-seams")]
    initial_next_work_generation: u64,
    #[cfg(feature = "internal-test-seams")]
    mounted_public_slot_limit: u64,
}

impl RuntimeConfig {
    /// Returns this configuration with a different waiting-envelope capacity.
    #[must_use]
    pub const fn with_queue_capacity(mut self, capacity: usize) -> Self {
        self.limits = self.limits.with_waiting_envelopes(capacity);
        self
    }

    #[must_use]
    pub const fn with_limits(mut self, limits: RuntimeLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Returns this configuration with different canonical trace retention.
    #[must_use]
    pub const fn with_trace_config(mut self, trace_config: TraceConfig) -> Self {
        self.trace_config = trace_config;
        self
    }

    /// Returns this configuration with a different total retained hit-test snapshot count.
    ///
    /// The count includes the current displayed generation. The default of two
    /// therefore retains the current and immediately previous generations.
    #[must_use]
    pub const fn with_surface_snapshot_retention(mut self, retention: NonZeroUsize) -> Self {
        self.surface_snapshot_retention = retention.get();
        self
    }

    /// Returns the maximum number of waiting envelopes.
    #[must_use]
    pub const fn queue_capacity(self) -> usize {
        self.limits.waiting_envelopes()
    }

    #[must_use]
    pub const fn limits(self) -> RuntimeLimits {
        self.limits
    }

    /// Returns the canonical trace configuration.
    #[must_use]
    pub const fn trace_config(self) -> TraceConfig {
        self.trace_config
    }

    /// Returns the total retained displayed hit-test snapshot count.
    #[must_use]
    pub const fn surface_snapshot_retention(self) -> usize {
        self.surface_snapshot_retention
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub const fn __with_initial_next_work_sequence_for_test(mut self, next: u64) -> Self {
        self.initial_next_work_sequence = next;
        self
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub const fn __with_initial_next_work_generation_for_test(mut self, next: u64) -> Self {
        self.initial_next_work_generation = next;
        self
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub const fn __with_mounted_public_slot_limit_for_test(mut self, limit: u64) -> Self {
        self.mounted_public_slot_limit = limit;
        self
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn initial_next_work_sequence(self) -> u64 {
        self.initial_next_work_sequence
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn initial_next_work_generation(self) -> u64 {
        self.initial_next_work_generation
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn mounted_public_slot_limit(self) -> u64 {
        self.mounted_public_slot_limit
    }

    #[cfg(not(feature = "internal-test-seams"))]
    pub(crate) const fn mounted_public_slot_limit(self) -> u64 {
        u32::MAX as u64 + 1
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            limits: RuntimeLimits::default(),
            trace_config: TraceConfig::new(DEFAULT_RUNTIME_LIMIT),
            surface_snapshot_retention: DEFAULT_SURFACE_SNAPSHOT_RETENTION,
            #[cfg(feature = "internal-test-seams")]
            initial_next_work_sequence: 1,
            #[cfg(feature = "internal-test-seams")]
            initial_next_work_generation: 1,
            #[cfg(feature = "internal-test-seams")]
            mounted_public_slot_limit: u64::from(u32::MAX) + 1,
        }
    }
}
