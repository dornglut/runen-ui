use core::num::NonZeroUsize;

use crate::app::{InitialAdmissionError, InitialAdmissionErrorKind};
use crate::trace::{MandatoryTracePlan, TraceRecordDraft, TraceReservation};
use runenui_core::ProducerBudget;

use super::{
    Arc, AutomationSubmissionPolicy, CompletionIngress, Element, FocusState, HostProtocol,
    ManualClock, MonotonicInstant, MountedTree, PointerRegistry, PreparedInitialApplicationWork,
    ReconciliationGeneration, ReconciliationReport, Runtime, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, SurfacePublicationState, SurfaceTraceState, TextSystem, Trace,
    TraceRecordKind, UnavailableExecutor, WakeState, WorkQueue, WorkRegistry,
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
        initial_producer_budget: ProducerBudget,
        prepare_initial_application_work: impl FnOnce(
            &State,
            ProducerBudget,
        ) -> Result<
            PreparedInitialApplicationWork<Action, Protocol>,
            InitialAdmissionErrorKind,
        >,
    ) -> Result<Self, InitialAdmissionError<State>> {
        if initial_producer_budget.queue_credits() == 0
            || initial_producer_budget.mounted_nodes() == 0
            || initial_producer_budget.dirty_subscription_owners() == 0
            || initial_producer_budget.application_outputs() > initial_producer_budget.outputs()
            || initial_producer_budget.application_outputs() > config.limits().transaction_outputs()
        {
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::InvalidProducerProfile,
            ));
        }
        if u64::try_from(initial_producer_budget.mounted_nodes())
            .map_or(true, |nodes| nodes > config.mounted_public_slot_limit())
        {
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::MountedIdentityExhausted,
            ));
        }
        let mut queue = WorkQueue::new(config.queue_capacity());
        let work = WorkRegistry::new(config.limits());
        #[cfg(feature = "internal-test-seams")]
        let work = {
            let mut work = work;
            work.seed_next_generation_for_test(config.initial_next_work_generation());
            work
        };
        // Generation feasibility is part of initial admission. Checking the
        // actual collected outputs later would already have invoked arbitrary
        // initial effects, root construction and widget callbacks.
        if initial_producer_budget.work_starts().total().is_none() {
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::InvalidProducerProfile,
            ));
        }
        if work
            .preflight_rooted_callback_generations(initial_producer_budget.work_starts())
            .is_err()
        {
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::WorkGenerationExhausted,
            ));
        }
        if let Err(family) = work.preflight_producer_residency(&queue, initial_producer_budget) {
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::WorkCapacity(family),
            ));
        }
        #[cfg(feature = "internal-test-seams")]
        queue.seed_next_sequence_for_test(config.initial_next_work_sequence());
        let mut initial_root_reservation = match queue.reserve_for_producer(initial_producer_budget)
        {
            Ok(reservation) => Some(reservation),
            Err(error) => {
                let kind = match error {
                    crate::queue::QueueReservationError::Capacity
                    | crate::queue::QueueReservationError::Closed => {
                        InitialAdmissionErrorKind::Capacity
                    }
                    crate::queue::QueueReservationError::WorkCapacity(family) => {
                        InitialAdmissionErrorKind::WorkCapacity(family)
                    }
                    crate::queue::QueueReservationError::SequenceExhausted
                    | crate::queue::QueueReservationError::Exhausted => {
                        InitialAdmissionErrorKind::SequenceExhausted
                    }
                    crate::queue::QueueReservationError::Foreign
                    | crate::queue::QueueReservationError::Stale => {
                        InitialAdmissionErrorKind::InvalidProducerProfile
                    }
                };
                return Err(InitialAdmissionError::new(state, config, kind));
            }
        };
        let mounted_public_slot_limit = config.mounted_public_slot_limit();
        let (tree, reconcile_stats) = match MountedTree::mount_with_public_slot_limit(
            root(&state),
            mounted_public_slot_limit,
            initial_producer_budget,
            0,
            runenui_core::WorkStartBudget::default(),
        ) {
            Ok(mounted) => mounted,
            Err(error) => {
                if let Some(reservation) = initial_root_reservation.take() {
                    let _ = queue.release_reservation(&reservation);
                }
                let kind = match error {
                    crate::mounted::InitialTreeMountError::IdentityExhausted => {
                        InitialAdmissionErrorKind::MountedIdentityExhausted
                    }
                    crate::mounted::InitialTreeMountError::ProducerViolation(cause) => {
                        InitialAdmissionErrorKind::ProducerContractViolation(cause)
                    }
                    crate::mounted::InitialTreeMountError::ProducerProfileExceeded => {
                        InitialAdmissionErrorKind::InvalidProducerProfile
                    }
                };
                return Err(InitialAdmissionError::new(state, config, kind));
            }
        };
        if let Some(cause) = reconcile_stats.producer_violation {
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::ProducerContractViolation(cause),
            ));
        }
        let initial_application_work =
            match prepare_initial_application_work(&state, initial_producer_budget) {
                Ok(work) => work,
                Err(kind) => {
                    if let Some(reservation) = initial_root_reservation.take() {
                        let _ = queue.release_reservation(&reservation);
                    }
                    return Err(InitialAdmissionError::new(state, config, kind));
                }
            };
        if let Err(reason) = initial_application_work.application.validate_returned(
            &crate::transaction::ReturnedEffectsPreflight {
                budget: initial_producer_budget,
                phase: runenui_core::ProducerPhase::InitialEffects,
                work: &work,
                queue: &queue,
                root: initial_root_reservation
                    .as_ref()
                    .map(crate::queue::QueueReservation::quota),
                state: &state,
                action_budgeter: None,
            },
        ) {
            let kind = match reason {
                RuntimeTerminalReason::ProducerContractViolation(cause) => {
                    InitialAdmissionErrorKind::ProducerContractViolation(cause)
                }
                _ => InitialAdmissionErrorKind::InvalidProducerProfile,
            };
            return Err(InitialAdmissionError::new(state, config, kind));
        }
        let initial_effect_start_budget = initial_application_work.application.work_start_budget();
        let initial_frontier_minimum = initial_application_work
            .application
            .len()
            .checked_add(initial_application_work.subscriptions.len())
            .and_then(|credits| {
                credits.checked_add(initial_producer_budget.dirty_subscription_owners())
            })
            .and_then(|credits| {
                credits.checked_add(
                    initial_effect_start_budget?.for_family(runenui_core::WorkFamily::HostRequest),
                )
            })
            .and_then(|credits| {
                credits.checked_add(
                    initial_effect_start_budget?.for_family(runenui_core::WorkFamily::LocalTask),
                )
            })
            .and_then(|credits| {
                credits.checked_add(
                    initial_effect_start_budget?.for_family(runenui_core::WorkFamily::SendTask),
                )
            })
            .and_then(|credits| {
                credits.checked_add(
                    initial_effect_start_budget?.for_family(runenui_core::WorkFamily::Timer),
                )
            })
            .and_then(|credits| {
                credits.checked_add(
                    initial_effect_start_budget?
                        .for_family(runenui_core::WorkFamily::FrameworkService)
                        .checked_mul(2)?,
                )
            })
            .and_then(|credits| {
                credits.checked_add(
                    initial_application_work
                        .application
                        .repeating_timer_start_count()?,
                )
            });
        let Some(initial_frontier_minimum) = initial_frontier_minimum else {
            if let Some(reservation) = initial_root_reservation.take() {
                let _ = queue.release_reservation(&reservation);
            }
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::InvalidProducerProfile,
            ));
        };
        if initial_frontier_minimum > initial_producer_budget.queue_credits() {
            if let Some(reservation) = initial_root_reservation.take() {
                let _ = queue.release_reservation(&reservation);
            }
            return Err(InitialAdmissionError::new(
                state,
                config,
                InitialAdmissionErrorKind::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::InitialEffects,
                        component: runenui_core::BudgetComponent::QueueFrontier,
                        declared: initial_producer_budget.queue_credits(),
                        attempted: Some(initial_frontier_minimum),
                    },
                ),
            )
            .with_frontier_requirement(
                initial_frontier_minimum,
                initial_producer_budget.queue_credits(),
            ));
        }
        let generation = 1;
        let surface_snapshot_retention =
            checked_surface_snapshot_retention(config.surface_snapshot_retention());
        let surface_publication =
            SurfacePublicationState::new(tree.runtime_namespace(), surface_snapshot_retention);
        let text_system = TextSystem::new(config.text_font_source_policy());
        let mut trace = Trace::new_for_runtime(config.trace_config(), tree.runtime_namespace());
        let initial_trace_plan = MandatoryTracePlan::runtime_mount_and_publication();
        let initial_trace_admitted = trace.can_admit(initial_trace_plan);
        let initial_redraw_trace = if initial_trace_admitted {
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
        let initial_mount_producer_overrun = reconcile_stats.producer_overrun;
        let input_observation =
            super::input_arbitration::InputObservationState::new(tree.runtime_namespace());
        let mut runtime = Self {
            state: Some(state),
            tree,
            queue,
            trace,
            trace_action_labeler: None,
            action_producer_budget: None,
            interaction_producer_budget: None,
            initial_producer_budget,
            initial_application_work: Some(initial_application_work),
            initial_root_reservation,
            text_system,
            focus: FocusState::new(),
            input_observation,
            focus_group_type_ahead: crate::input::FocusGroupTypeAheadState::new(),
            presentation_lifecycle: super::presentation::PresentationLifecycleState::new(),
            pointer_registry: PointerRegistry::new(limits.pointer_streams()),
            space_ownership: None,
            composition: crate::input::CompositionState::None,
            editing: crate::editing::EditingRegistry::new(
                limits.editing_sessions(),
                limits.pending_edits(),
            ),
            next_composition_generation: core::num::NonZeroU64::new(1),
            last_issued_composition_generation: None,
            generation,
            report,
            status: RuntimeStatus::Running,
            automation_submission_policy: AutomationSubmissionPolicy::Ordinary,
            limits,
            touch_gesture_thresholds: config.touch_gesture_thresholds(),
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
            framework_service_namespace: Arc::new(()),
            framework_services: Vec::new(),
            framework_ime_may_be_enabled: false,
            framework_service_satisfied: Vec::new(),
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
            test_seams: super::model::RuntimeTestSeams::default(),
        };
        let contributions = runtime.tree.editable_contributions();
        let namespace = runtime.tree.runtime_namespace();
        if contributions.is_err()
            || runtime
                .editing
                .initial_reconcile(&namespace, contributions.unwrap_or_else(|_| unreachable!()))
                .is_err()
        {
            runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
        if initial_mount_producer_overrun {
            runtime.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
        if !initial_trace_admitted {
            runtime.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
        }
        Ok(runtime)
    }
}
