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

    pub(crate) const fn action_acceptance() -> Self {
        Self::exact(1)
    }

    pub(crate) const fn command_acceptance() -> Self {
        Self::exact(1)
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

    pub(crate) const fn application_action_base(has_focus: bool) -> Self {
        Self::exact(if has_focus { 4 } else { 3 })
    }

    pub(crate) fn lifecycle_invalidations(count: usize) -> Option<Self> {
        Self::exact(2).checked_mul(count)
    }

    pub(crate) fn routed_event(route_invocations: usize, max_outputs: usize) -> Option<Self> {
        Self::exact(6)
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

/// Private authority retained by one accepted command for exactly one future
/// processing outcome. Disabled tracing carries a no-op reservation so command
/// behavior remains identical when canonical retention is disabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TraceReservation {
    active: bool,
}

impl TraceReservation {
    pub(super) const DISABLED: Self = Self { active: false };
    pub(super) const ACTIVE: Self = Self { active: true };

    pub(crate) const fn is_active(self) -> bool {
        self.active
    }
}
