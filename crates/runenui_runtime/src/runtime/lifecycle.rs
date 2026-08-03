//! Owner revocation, terminal transition, shutdown, and scheduling closure.

use runenui_core::CommandOrigin;

use super::{
    CompletionIngress, HostProtocol, LiveHostRequest, LiveSubscription, LocalTask, Runtime,
    RuntimeStatus, RuntimeTerminalReason, SendTaskMapper, ShutdownReport, Timer, TraceRecordKind,
    TraceSequence, WorkCancellationCounts, WorkOwner, WorkRegistry,
    focus::InputLifetimeCleanupCause,
};
use crate::TraceSpaceCleanupReason;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn cancel_owner_work(&mut self, owner: &WorkOwner) {
        let generations = self.work.generations_for_owner(owner);
        for generation in generations {
            self.revoke_generation(generation);
        }
        if let WorkOwner::Mounted(owner) = owner {
            self.mounted_subscription_reconcile_pending
                .retain(|pending| pending != owner);
        }
    }

    pub(crate) fn enter_terminal(
        &mut self,
        reason: RuntimeTerminalReason,
        additional_cancelled: usize,
    ) -> usize {
        if !matches!(self.status, RuntimeStatus::Running) {
            return 0;
        }
        // Stop ordinary mutation first. Cleanup still uses the checked live
        // route; if that route or its bounded admission is irrecoverable, the
        // exact lifetime is retired without falsely claiming callback delivery.
        self.status = RuntimeStatus::Terminal(reason);
        let cleanup_cause = InputLifetimeCleanupCause::new(
            None,
            None,
            self.now(),
            CommandOrigin::programmatic(),
        );
        if !self.cancel_composition_while_live(
            runenui_core::CompositionCancelReason::Shutdown,
            cleanup_cause,
        ) {
            self.suppress_composition_cleanup(
                runenui_core::CompositionCancelReason::Shutdown,
                cleanup_cause,
            );
        }
        let space_parent = self.revoke_space_ownership_with_cause(
            TraceSpaceCleanupReason::Terminal,
            cleanup_cause,
        );
        let (cancelled_queued, cancelled_live, pointer_parent) = self.close_scheduling_authority();
        let cancelled = cancelled_queued
            .saturating_add(cancelled_live.total())
            .saturating_add(additional_cancelled);
        let terminal = self.trace.record(
            TraceRecordKind::RuntimeTerminal { reason },
            None,
            pointer_parent.or(space_parent),
            None,
            None,
            None,
        );
        if cancelled > 0 {
            self.trace.record(
                TraceRecordKind::QueuedWorkCancelled { count: cancelled },
                None,
                terminal,
                None,
                None,
                None,
            );
        }
        cancelled
    }

    pub(super) fn close_scheduling_authority(
        &mut self,
    ) -> (usize, WorkCancellationCounts, Option<TraceSequence>) {
        self.completion_ingress.close();
        self.wake.close();
        let cancelled_queue = self.queue.cancel_all();
        self.trace.release_reservations(
            cancelled_queue
                .command_trace_reservations
                .saturating_add(cancelled_queue.pointer_trace_reservations)
                .saturating_add(cancelled_queue.input_trace_reservations),
        );
        let cancelled_queued = cancelled_queue.envelopes;
        let cancelled_live = self.work.cancel_all_counts();
        self.local_tasks.clear();
        self.timers.clear();
        self.subscriptions.clear();
        self.send_task_mappers.clear();
        self.host_requests.clear();
        self.mounted_subscription_reconcile_pending.clear();
        self.initial_mounted_subscription_owners.clear();
        self.initial_mounted_outputs.clear();
        let (_, pointer_parent) = self.close_pointer_lifetimes(None);
        (cancelled_queued, cancelled_live, pointer_parent)
    }

    pub(crate) fn shutdown(&mut self) -> ShutdownReport {
        if matches!(self.status, RuntimeStatus::Closed) {
            return ShutdownReport {
                already_complete: true,
                cancelled_queued_envelopes: 0,
                unmounted_lifetimes: 0,
                cancelled_live_work: WorkCancellationCounts::default(),
            };
        }
        let cleanup_cause = InputLifetimeCleanupCause::new(
            None,
            None,
            self.now(),
            CommandOrigin::programmatic(),
        );
        if !self.cancel_composition_while_live(
            runenui_core::CompositionCancelReason::Shutdown,
            cleanup_cause,
        ) {
            self.suppress_composition_cleanup(
                runenui_core::CompositionCancelReason::Shutdown,
                cleanup_cause,
            );
            if matches!(self.status, RuntimeStatus::Running) {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            }
        }
        // Pointer cleanup is independent of queue closure, but its accepted
        // M4C3 trace/lifetime ordering precedes shutdown focus suppression.
        // Closing the registry here leaves actual scheduling authority open
        // until all input and focus cleanup is complete.
        let (_, pointer_parent) = self.close_pointer_lifetimes(None);
        let space_parent = self.revoke_space_ownership_with_cause(
            TraceSpaceCleanupReason::Shutdown,
            InputLifetimeCleanupCause::new(
                None,
                pointer_parent,
                self.now(),
                CommandOrigin::programmatic(),
            ),
        );
        let shutdown_parent = self.clear_focus_for_shutdown(space_parent);
        let (cancelled_queued_envelopes, cancelled_live_work, _) =
            self.close_scheduling_authority();
        let stats = self.tree.shutdown();
        self.surface_publication.clear_cache();
        self.trace.record(
            TraceRecordKind::RuntimeShutdown {
                cancelled_queued: cancelled_queued_envelopes,
                unmounted_lifetimes: stats.unmounted,
            },
            None,
            shutdown_parent,
            None,
            None,
            None,
        );
        self.status = RuntimeStatus::Closed;
        ShutdownReport {
            already_complete: false,
            cancelled_queued_envelopes,
            unmounted_lifetimes: stats.unmounted,
            cancelled_live_work,
        }
    }

    pub(crate) fn into_state(mut self) -> State {
        self.shutdown();
        self.state
            .take()
            .unwrap_or_else(|| unreachable!("state is returned exactly once"))
    }

    pub(super) fn invalidate_generation_now(&mut self, generation: crate::work::WorkGeneration) {
        self.revoke_generation(generation);
    }

    pub(super) fn revoke_generation(&mut self, generation: crate::work::WorkGeneration) {
        revoke_generation_authority(
            generation,
            &mut self.work,
            &self.completion_ingress,
            &mut self.local_tasks,
            &mut self.timers,
            &mut self.send_task_mappers,
            &mut self.subscriptions,
            &mut self.host_requests,
        );
    }
}

impl<State, Action, Protocol: HostProtocol> Drop for Runtime<State, Action, Protocol> {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::runtime) fn revoke_generation_authority<Action, Protocol: HostProtocol>(
    generation: crate::work::WorkGeneration,
    work: &mut WorkRegistry<Action, Protocol>,
    completion_ingress: &CompletionIngress,
    local_tasks: &mut Vec<LocalTask<Action>>,
    timers: &mut Vec<Timer<Action>>,
    send_task_mappers: &mut Vec<SendTaskMapper<Action>>,
    subscriptions: &mut Vec<LiveSubscription<Action>>,
    host_requests: &mut Vec<LiveHostRequest<Action, Protocol>>,
) {
    let _ = work.invalidate(generation);
    let _ = completion_ingress.revoke_generation(generation);
    local_tasks.retain(|task| task.generation != generation);
    timers.retain(|timer| timer.generation != generation);
    send_task_mappers.retain(|mapper| mapper.generation != generation);
    subscriptions.retain(|subscription| subscription.generation != generation);
    host_requests.retain(|request| request.generation != generation);
}
