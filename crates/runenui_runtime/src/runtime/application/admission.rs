//! Conservative whole-turn trace demand, before irreversible application code.

use super::{HostProtocol, MandatoryTracePlan, Runtime, RuntimeTerminalReason};
use runenui_core::ProducerBudget;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn application_identity_budget_fits(
        &self,
        profile: ProducerBudget,
    ) -> bool {
        profile
            .retained_updates()
            .checked_add(profile.mounted_nodes())
            .and_then(|peak| u64::try_from(peak).ok())
            .is_some_and(|peak| peak <= self.mounted_public_slot_limit)
    }

    pub(in crate::runtime) fn preflight_application_turn_identity(
        &self,
        profile: ProducerBudget,
    ) -> Result<(), RuntimeTerminalReason> {
        self.tree
            .preflight_producer_identity(profile, self.mounted_public_slot_limit)
            .map_err(|_| RuntimeTerminalReason::MountedIdentityExhausted)?;
        self.work
            .preflight_rooted_callback_generations(profile.work_starts())
            .map_err(|_| RuntimeTerminalReason::WorkGenerationExhausted)
    }

    pub(in crate::runtime) fn application_turn_trace_plan(
        &self,
        profile: ProducerBudget,
        edit_resolution: bool,
    ) -> Option<MandatoryTracePlan> {
        // The old tree plus every declared new mount covers old and new routes.
        // No callback runs here and no post-update topology is predicted.
        let nodes = self
            .tree
            .live_count()
            .checked_add(profile.mounted_nodes())?;
        let pointers = self.pointer_registry.len();
        let route_invocations = nodes.checked_mul(2)?;
        // At most composition cancellation and FocusOut, plus each pointer's
        // cancel, leave, enter, capture-lost, capture-gained and re-hit paths.
        // Target-only paths are conservatively charged a complete C/T/B route.
        let routed_passes = pointers.checked_mul(6)?.checked_add(2)?;
        // Preserve each full configured route ceiling. Boundary notifications
        // and cursor/IME/default work are additional, never stolen from it.
        let default_outputs = route_invocations.checked_add(3)?;
        let route_outputs = self
            .limits
            .transaction_outputs()
            .checked_add(default_outputs)?;
        let routed = MandatoryTracePlan::routed_event(route_invocations, route_outputs)?
            .checked_add(MandatoryTracePlan::focus_commit())?
            .checked_add(MandatoryTracePlan::composition_cleanup())?
            .checked_add(MandatoryTracePlan::pointer_processing())?
            .checked_add(MandatoryTracePlan::input_processing())?
            .checked_add(MandatoryTracePlan::pointer_commit(route_invocations)?)?
            .checked_mul(routed_passes)?;
        // The scheduler's exact planner charges 2 per invalidation, 2 per
        // start, one more per subscription start, actions/redraws, and commit.
        // Same-batch replacements may invalidate every newly authored start.
        let starts = profile
            .work_starts()
            .total()?
            .checked_add(profile.application_subscriptions())?
            .checked_add(nodes.checked_mul(profile.mounted_subscriptions_per_owner())?)?;
        let resident = self.work.live_record_count();
        let invalidated = resident.checked_add(starts)?;
        let scheduler_records = invalidated
            .checked_mul(2)?
            .checked_add(starts.checked_mul(3)?)?
            .checked_add(profile.application_outputs().checked_mul(2)?)?
            // Each possible presentation restoration has acceptance, future
            // outcome and a resolution fact; none runs its command recursively.
            .checked_add(nodes.checked_mul(3)?)?
            .checked_add(1)?;
        MandatoryTracePlan::application_action_base(
            self.focus.focused_node().is_some(),
            edit_resolution,
        )
        .checked_add(routed)?
        .checked_add(MandatoryTracePlan::lifecycle_invalidations(resident)?)?
        .checked_add(MandatoryTracePlan::planned_scheduler_transaction(
            scheduler_records,
        ))
    }
}
