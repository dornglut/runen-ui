#![allow(clippy::redundant_pub_crate)]

/// Checked admission requirement for one mutation boundary.
///
/// Constructors describe either the exact mandatory records for an operation or
/// the documented maximum path that must be admitted before user code runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MandatoryTracePlan {
    pub(super) records: usize,
}

impl MandatoryTracePlan {
    pub(super) const fn exact(records: usize) -> Self {
        Self { records }
    }

    pub(crate) const fn none() -> Self {
        Self::exact(0)
    }

    pub(crate) const fn runtime_mount_and_publication() -> Self {
        Self::exact(3)
    }

    /// Exact sequence commitments for one surface publication attempt after the
    /// retained publication reservation is released. The fixed two are the
    /// `SurfacePublished` record and the next publication reservation.
    pub(crate) const fn surface_publication(redraw: bool, stationary_rehit: bool) -> Self {
        Self::exact(2 + if redraw { 2 } else { 0 } + if stationary_rehit { 1 } else { 0 })
    }

    pub(crate) const fn action_acceptance() -> Self {
        Self::exact(1)
    }

    pub(crate) const fn command_acceptance() -> Self {
        Self::exact(1)
    }

    pub(crate) const fn pointer_acceptance() -> Self {
        Self::exact(1)
    }

    pub(crate) const fn input_acceptance() -> Self {
        Self::exact(1)
    }

    /// Validation, family-specific outcomes, and the possible derived command
    /// lineage recorded while processing one admitted raw input envelope.
    pub(crate) const fn input_processing() -> Self {
        Self::exact(6)
    }

    /// Allocation plus pending binding are committed before a composition
    /// start envelope enters the canonical FIFO.
    pub(crate) const fn composition_start_acceptance() -> Self {
        Self::exact(2)
    }

    /// Cancellation records cancellation and retirement before the focus or
    /// reconciliation transition that caused it.
    pub(crate) const fn composition_cleanup() -> Self {
        Self::exact(3)
    }

    pub(crate) const fn automation_resolution() -> Self {
        Self::exact(1)
    }

    /// Maximum stream/context/target outcome facts before routed callbacks begin.
    pub(crate) const fn pointer_processing() -> Self {
        Self::exact(5)
    }

    /// Selection, boundary/restoration, transition, focus-within, and two
    /// notification-queue facts. Phase/output facts are covered separately by
    /// the routed-event maximum.
    pub(crate) const fn focus_commit() -> Self {
        Self::exact(10)
    }

    /// Maximum default/interaction/notification/output/close facts committed
    /// after pointer callbacks.
    pub(crate) fn pointer_commit(boundary_notifications: usize) -> Option<Self> {
        Self::exact(9).checked_add(Self::exact(boundary_notifications))
    }

    pub(crate) const fn surface_command_acceptance() -> Self {
        Self::exact(3)
    }

    pub(crate) const fn one_fact() -> Self {
        Self::exact(1)
    }

    pub(crate) const fn work_cancellation() -> Self {
        Self::exact(2)
    }

    pub(crate) const fn send_completion() -> Self {
        Self::exact(3)
    }

    pub(crate) const fn host_completion() -> Self {
        Self::exact(4)
    }

    pub(crate) const fn callback_with_action() -> Self {
        Self::exact(3)
    }

    pub(crate) const fn typed_start_refusal_with_action() -> Self {
        Self::exact(1)
    }

    pub(crate) const fn work_start(host_request: bool) -> Self {
        Self::exact(if host_request { 3 } else { 2 })
    }

    /// Update summary records plus the mandatory redraw request caused by the
    /// committed reconciliation.
    pub(crate) const fn application_action_base(has_focus: bool) -> Self {
        Self::exact(if has_focus { 7 } else { 4 })
    }

    pub(crate) fn lifecycle_invalidations(count: usize) -> Option<Self> {
        Self::exact(2).checked_mul(count)
    }

    pub(crate) fn routed_event(route_invocations: usize, max_outputs: usize) -> Option<Self> {
        // The fixed maximum includes the final redraw request when routed
        // invalidation dirties publication.
        Self::exact(8)
            .checked_add(Self::exact(6).checked_mul(route_invocations)?)
            .and_then(|plan| plan.checked_add(Self::exact(6).checked_mul(max_outputs)?))
            .and_then(|plan| {
                // Every collected output may be a delegated command. Its accepted
                // envelope must retain one future processing-outcome sequence.
                plan.checked_add(Self::exact(max_outputs))
            })
    }

    pub(crate) const fn planned_scheduler_transaction(records: usize) -> Self {
        Self::exact(records)
    }

    pub(crate) fn checked_add(self, other: Self) -> Option<Self> {
        self.records.checked_add(other.records).map(Self::exact)
    }

    pub(crate) fn checked_mul(self, count: usize) -> Option<Self> {
        self.records.checked_mul(count).map(Self::exact)
    }
}

/// Private authority retained by one accepted routed ingress envelope for
/// exactly one future processing outcome. Disabled tracing carries a no-op
/// reservation so behavior remains identical when canonical retention is disabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TraceReservation {
    active: bool,
}

impl TraceReservation {
    pub(super) const DISABLED: Self = Self { active: false };
    pub(super) const ACTIVE: Self = Self { active: true };

    /// Creates an unreserved continuation after an earlier processing reservation
    /// was consumed. The next mutation boundary must still perform full admission.
    pub(crate) const fn continuation() -> Self {
        Self::DISABLED
    }

    pub(crate) const fn is_active(self) -> bool {
        self.active
    }
}
