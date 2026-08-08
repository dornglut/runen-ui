use core::num::NonZeroUsize;

use crate::trace::{MandatoryTracePlan, TraceRecordDraft, TraceReservation};

use super::{
    Arc, AutomationSubmissionPolicy, CompletionIngress, Element, FocusState, HostProtocol,
    ManualClock, MonotonicInstant, MountedIdentityExhausted, MountedTree, PointerRegistry,
    ReconciliationGeneration, ReconciliationReport, Runtime, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, SurfacePublicationState, SurfaceTraceState, Trace, TraceRecordKind,
    UnavailableExecutor, WakeState, WorkQueue, WorkRegistry,
};

fn checked_surface_snapshot_retention(retention: usize) -> NonZeroUsize {
    NonZeroUsize::new(retention)
        .unwrap_or_else(|| unreachable!("surface snapshot retention is non-zero"))
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    #[allow(
        clippy::too_many_lines,
        reason = "runtime construction enumerates every bounded authority explicitly"
    )]
    pub(crate) fn mount(
        state: State,
        root: impl FnOnce(&State) -> Element<Action>,
        config: RuntimeConfig,
    ) -> Self {
        let transient = root(&state);
        let mounted_public_slot_limit = config.mounted_public_slot_limit();
        let mounted =
            MountedTree::mount_with_public_slot_limit(transient, mounted_public_slot_limit);
        let mount_failed = mounted.is_err();
        let (tree, reconcile_stats, generation) = match mounted {
            Ok((tree, reconcile_stats)) => (tree, reconcile_stats, 1),
            Err(MountedIdentityExhausted) => (
                MountedTree::empty(),
                crate::mounted::ReconcileStats::default(),
                0,
            ),
        };
        let surface_snapshot_retention =
            checked_surface_snapshot_retention(config.surface_snapshot_retention());
        let surface_publication =
            SurfacePublicationState::new(tree.runtime_namespace(), surface_snapshot_retention);
        let mut trace =
            Trace::new_for_runtime(config.trace_config(), tree.runtime_namespace().clone());
        let initial_trace_plan = if mount_failed {
            MandatoryTracePlan::one_fact()
        } else {
            MandatoryTracePlan::runtime_mount_and_publication()
        };
        let initial_trace_admitted = trace.can_admit(initial_trace_plan);
        let initial_redraw_trace = if !mount_failed && initial_trace_admitted {
            let mounted_trace = trace.record_draft(TraceRecordDraft::lifecycle_fact(
                TraceRecordKind::RuntimeMounted,
                MonotonicInstant::ZERO,
            ));
            trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawRequested { revision: 1 },
                    MonotonicInstant::ZERO,
                )
                .with_causal_parent(mounted_trace),
            )
        } else {
            None
        };
        let publication_reservation = if initial_trace_admitted {
            trace
                .reserve_surface_publication()
                .unwrap_or_else(|| unreachable!("initial publication trace was preflighted"))
        } else {
            TraceReservation::continuation()
        };
        let report = ReconciliationReport {
            generation: ReconciliationGeneration(generation),
            live_node_count: tree.live_count(),
            mounted_count: reconcile_stats.mounted,
            updated_count: 0,
            unmounted_count: 0,
            moved_count: 0,
            retained_focus: false,
            diagnostics: reconcile_stats.diagnostics,
        };
        let limits = config.limits();
        let wake = WakeState::new();
        let mounted_owners = reconcile_stats.mounted_owners;
        let queue = WorkQueue::new(config.queue_capacity());
        let work = WorkRegistry::new(limits);
        #[cfg(feature = "internal-test-seams")]
        let (queue, work) = {
            let mut queue = queue;
            let mut work = work;
            queue.seed_next_sequence_for_test(config.initial_next_work_sequence());
            work.seed_next_generation_for_test(config.initial_next_work_generation());
            (queue, work)
        };
        let mut runtime = Self {
            state: Some(state),
            tree,
            queue,
            trace,
            trace_action_labeler: None,
            focus: FocusState::new(),
            pointer_registry: PointerRegistry::new(limits.pointer_streams()),
            space_ownership: None,
            composition: crate::input::CompositionState::None,
            next_composition_generation: core::num::NonZeroU64::new(1),
            last_issued_composition_generation: None,
            generation,
            report,
            status: RuntimeStatus::Running,
            automation_submission_policy: AutomationSubmissionPolicy::Ordinary,
            limits,
            mounted_public_slot_limit,
            work,
            mounted_subscription_reconcile_pending: Vec::new(),
            initial_mounted_subscription_owners: mounted_owners,
            initial_mounted_outputs: reconcile_stats.mounted_outputs,
            subscriptions: Vec::new(),
            subscription_diagnostics: Vec::new(),
            clock: ManualClock::new(),
            local_tasks: Vec::new(),
            timers: Vec::new(),
            completion_ingress: CompletionIngress::new(limits.completion_ingress(), wake.handle()),
            send_executor: Box::new(UnavailableExecutor),
            send_task_mappers: Vec::new(),
            last_send_task_start_outcome: None,
            last_timer_start_outcome: None,
            last_timer_firing_outcome: None,
            host_clock: None,
            host_namespace: Arc::new(()),
            host_requests: Vec::new(),
            surface_publication,
            surface_trace: SurfaceTraceState::new(
                Some(1),
                initial_redraw_trace,
                publication_reservation,
            ),
            wake,
            #[cfg(test)]
            readiness_checkpoint_count: 0,
            #[cfg(feature = "internal-test-seams")]
            routed_callback_bridge_failure_for_test: false,
            #[cfg(feature = "internal-test-seams")]
            routed_semantic_default_failure_for_test: false,
            #[cfg(feature = "internal-test-seams")]
            routed_commit_failure_for_test: false,
        };
        if mount_failed {
            runtime.enter_terminal(RuntimeTerminalReason::MountedIdentityExhausted, 0);
        } else if !initial_trace_admitted {
            runtime.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
        }
        runtime
    }
}
