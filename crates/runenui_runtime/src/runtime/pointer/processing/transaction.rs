use runenui_core::{
    __runtime::PointerCaptureRequest, CommandOrigin, HostProtocol, LogicalScrollCommand,
    PointerBoundaryEvent, PointerButton, PointerCaptureEvent, PointerPhase, SemanticCommand,
    UiEvent,
};

use super::{
    PointerCommitPlan, PointerGeometry, PointerStreamState, PointerWork, PreparedPointer,
    StreamCommitKind, pointer_default_is_cancelable,
};
use crate::{
    MountedNodeId, RuntimeTerminalReason, TracePointerCaptureRequestRejection, TraceRecordKind,
    TraceRoutedIntegrityFailure,
    mounted::TargetStatus,
    runtime::{
        CollectedRoutedOutput, MandatoryTracePlan, PointerDispatchFacts,
        ProcessApplicationActionOutcome, RoutedIngressFacts, RoutedTransaction, Runtime,
    },
    trace::TraceReservation,
};

struct PendingPointerCommit {
    work: PointerWork,
    stream: PointerStreamState,
    previous_capture_owner: Option<MountedNodeId>,
    geometry: PointerGeometry,
    routed_target: Option<MountedNodeId>,
    kind: StreamCommitKind,
    boundary_notifications: Vec<runenui_core::PointerBoundaryKind>,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn dispatch_prepared_pointer(
        &mut self,
        prepared: PreparedPointer,
    ) -> ProcessApplicationActionOutcome {
        let PreparedPointer {
            work,
            is_new,
            stream,
            previous_capture_owner,
            geometry,
            boundary_events,
            routed_target,
            parent,
        } = prepared;
        let kind = Self::stream_commit_kind(work.event.phase(), is_new);
        let boundary_events = boundary_events
            .into_iter()
            .filter(|event| self.tree.target_status(event.target()) == TargetStatus::Live)
            .collect::<Vec<_>>();
        let boundary_targets = boundary_events
            .iter()
            .map(|event| event.target().clone())
            .collect::<Vec<_>>();
        let boundary_notifications = Self::boundary_notification_kinds(&boundary_events);
        let deferred_capture_targets = previous_capture_owner
            .iter()
            .filter(|target| self.tree.target_status(target) == TargetStatus::Live)
            .cloned()
            .collect::<Vec<_>>();
        let Some(pointer_commit_trace) =
            self.plan_pointer_commit_trace(boundary_notifications.len())
        else {
            return self.pointer_runtime_outcome();
        };
        let anchor = routed_target
            .clone()
            .or_else(|| boundary_targets.first().cloned());
        let Some(anchor) = anchor else {
            return self.commit_unrouted_pointer(
                work.sequence,
                parent,
                PointerCommitPlan {
                    pointer_id: work.event.pointer_id(),
                    stream,
                    kind,
                    focus: None,
                    capture_events: Vec::new(),
                    boundary_notifications,
                    physical_target: geometry.physical_target,
                    physical_path: geometry.physical_path,
                },
            );
        };
        let facts = RoutedIngressFacts::new(
            work.sequence,
            anchor,
            CommandOrigin::__runtime_pointer(),
            work.instant,
            parent,
            TraceReservation::continuation(),
        );
        let Some(mut transaction) = self.begin_pointer_routed_transaction(
            facts,
            routed_target.is_some(),
            &boundary_targets,
            &deferred_capture_targets,
            2,
            pointer_commit_trace,
        ) else {
            return self.pointer_runtime_outcome();
        };
        if let Err((failure, current)) = self.invoke_pointer_boundary_events(
            &mut transaction,
            &work,
            &geometry,
            &boundary_events,
        ) {
            self.poison_transaction(&transaction, failure, Some(&current));
            return self.pointer_runtime_outcome();
        }
        if let Err(failure) = self.invoke_ordinary_pointer_event(
            &mut transaction,
            &work,
            &geometry,
            routed_target.is_some(),
        ) {
            let current = transaction.failure_current_target.clone();
            self.poison_transaction(&transaction, failure, current.as_ref());
            return self.pointer_runtime_outcome();
        }
        self.finish_pointer_transaction(
            transaction,
            PendingPointerCommit {
                work,
                stream,
                previous_capture_owner,
                geometry,
                routed_target,
                kind,
                boundary_notifications,
            },
        )
    }

    fn finish_pointer_transaction(
        &mut self,
        mut transaction: RoutedTransaction<Action>,
        mut pending: PendingPointerCommit,
    ) -> ProcessApplicationActionOutcome {
        self.apply_pointer_capture_requests(
            pending.work.event.pointer_id(),
            &mut pending.stream,
            &mut transaction,
        );
        let default_outputs_before = transaction.default_outputs.len();
        let focus = match self.apply_pointer_defaults(
            &pending.work.event,
            &pending.geometry.physical_path,
            pending.geometry.physical_target.as_ref(),
            pending.routed_target.as_ref(),
            &mut pending.stream,
            &mut transaction,
        ) {
            Ok(focus) => focus,
            Err(failure) => {
                let current = transaction.failure_current_target.clone();
                self.poison_transaction(&transaction, failure, current.as_ref());
                return self.pointer_runtime_outcome();
            }
        };
        let default_applied = match pending.work.event.phase() {
            PointerPhase::Move | PointerPhase::Cancel => true,
            PointerPhase::Down => {
                !transaction.default_prevented
                    && pending.work.event.changed_button() == Some(PointerButton::Primary)
                    && pending
                        .geometry
                        .physical_target
                        .as_ref()
                        .is_some_and(|target| pending.stream.pressed_owner() == Some(target))
            }
            PointerPhase::Up | PointerPhase::Wheel => {
                transaction.default_outputs.len() > default_outputs_before
            }
            _ => false,
        };
        transaction.parent = self.trace.record_event(
            if default_applied {
                TraceRecordKind::PointerDefaultApplied {
                    pointer_id: pending.work.event.pointer_id(),
                    phase: pending.work.event.phase(),
                }
            } else {
                TraceRecordKind::PointerDefaultSuppressed {
                    pointer_id: pending.work.event.pointer_id(),
                    phase: pending.work.event.phase(),
                }
            },
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace.clone()),
            transaction.instant,
            &transaction.target,
            None,
            transaction.origin,
        );
        let final_capture_owner = if pending.kind == StreamCommitKind::Close {
            None
        } else {
            pending.stream.capture_owner().cloned()
        };
        let capture_events = super::notifications::plan_capture_events(
            pending.work.event.pointer_id(),
            pending.previous_capture_owner.as_ref(),
            final_capture_owner.as_ref(),
            pending.work.event.surface_context(),
        );
        self.commit_prepared_pointer_transaction(
            transaction,
            PointerCommitPlan {
                pointer_id: pending.work.event.pointer_id(),
                stream: pending.stream,
                kind: pending.kind,
                focus,
                capture_events,
                boundary_notifications: pending.boundary_notifications,
                physical_target: pending.geometry.physical_target,
                physical_path: pending.geometry.physical_path,
            },
        )
    }

    fn invoke_pointer_boundary_events(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        work: &PointerWork,
        geometry: &PointerGeometry,
        boundary_events: &[PointerBoundaryEvent],
    ) -> Result<(), (TraceRoutedIntegrityFailure, MountedNodeId)> {
        for boundary in boundary_events {
            let event = UiEvent::PointerBoundary(boundary.clone());
            let dispatch = PointerDispatchFacts::new(
                work.event.pointer_id(),
                geometry.physical_target.as_ref(),
                &geometry.physical_path,
                boundary.related_target(),
                false,
            );
            if let Err(failure) = self.invoke_target_only_pointer_callback(
                transaction,
                &event,
                dispatch,
                boundary.target(),
            ) {
                return Err((failure, boundary.target().clone()));
            }
        }
        Ok(())
    }

    fn invoke_ordinary_pointer_event(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        work: &PointerWork,
        geometry: &PointerGeometry,
        has_routed_target: bool,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        if !has_routed_target {
            return Ok(());
        }
        let event = UiEvent::Pointer(work.event.clone());
        let dispatch = PointerDispatchFacts::new(
            work.event.pointer_id(),
            geometry.physical_target.as_ref(),
            &geometry.physical_path,
            None,
            pointer_default_is_cancelable(work.event.phase()),
        );
        self.invoke_routed_callbacks(transaction, &event, Some(dispatch))
    }

    fn commit_prepared_pointer_transaction(
        &mut self,
        transaction: RoutedTransaction<Action>,
        plan: PointerCommitPlan,
    ) -> ProcessApplicationActionOutcome {
        let failure_facts = transaction.failure_facts();
        if self
            .commit_routed_transaction_with(transaction, move |runtime, transaction| {
                runtime.commit_pointer_plan(plan, transaction)
            })
            .is_err()
        {
            self.poison_routed_event(
                &failure_facts,
                TraceRoutedIntegrityFailure::CommitInvariantFailure,
                None,
            );
        }
        self.pointer_runtime_outcome()
    }

    const fn stream_commit_kind(phase: PointerPhase, is_new: bool) -> StreamCommitKind {
        if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
            StreamCommitKind::Close
        } else if is_new {
            StreamCommitKind::Register
        } else {
            StreamCommitKind::Replace
        }
    }

    fn apply_pointer_defaults(
        &mut self,
        event: &runenui_core::PointerEvent,
        physical_path: &[MountedNodeId],
        physical_target: Option<&MountedNodeId>,
        routed_target: Option<&MountedNodeId>,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Result<Option<MountedNodeId>, TraceRoutedIntegrityFailure> {
        match event.phase() {
            PointerPhase::Down => {
                if transaction.default_prevented
                    || event.changed_button() != Some(PointerButton::Primary)
                {
                    return Ok(None);
                }
                let Some(target) = physical_target else {
                    return Ok(None);
                };
                let actionable = self
                    .tree
                    .activation(target)
                    .is_ok_and(|activation| activation.enabled() && activation.is_actionable());
                if !actionable {
                    return Ok(None);
                }
                stream.set_pressed_owner(Some(target.clone()));
                stream.set_capture_owner(Some(target.clone()));
                Ok(self.validate_focus(target).then(|| target.clone()))
            }
            PointerPhase::Move => {
                let inside = stream
                    .pressed_owner()
                    .is_some_and(|owner| physical_path.iter().any(|target| target == owner));
                stream.set_pressed_inside(inside);
                Ok(None)
            }
            PointerPhase::Up => {
                if !transaction.default_prevented
                    && event.changed_button() == Some(PointerButton::Primary)
                    && let Some(owner) = stream.pressed_owner().cloned()
                    && physical_path.iter().any(|target| target == &owner)
                    && self
                        .tree
                        .activation(&owner)
                        .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
                {
                    Self::push_pointer_default(transaction, owner, SemanticCommand::Activate)?;
                }
                Ok(None)
            }
            PointerPhase::Wheel => {
                if !transaction.default_prevented
                    && let Some(routed_target) = routed_target
                {
                    Self::push_pointer_default(
                        transaction,
                        routed_target.clone(),
                        SemanticCommand::LogicalScroll(LogicalScrollCommand::__runtime_new(
                            event.pointer_id(),
                            event.scroll_delta(),
                        )),
                    )?;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn push_pointer_default(
        transaction: &mut RoutedTransaction<Action>,
        target: MountedNodeId,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        if transaction.remaining_outputs == 0 {
            transaction.failure_current_target = Some(target);
            return Err(TraceRoutedIntegrityFailure::OutputAllowanceExceeded);
        }
        transaction.remaining_outputs -= 1;
        transaction
            .default_outputs
            .push(CollectedRoutedOutput::Command {
                target,
                command,
                origin: CommandOrigin::__runtime_pointer_default(),
                causal_parent: transaction.parent,
            });
        Ok(())
    }

    fn apply_pointer_capture_requests(
        &mut self,
        pointer_id: runenui_core::PointerId,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
    ) {
        for request in core::mem::take(&mut transaction.pointer_capture_requests) {
            let (requested, target, capture) = match request {
                PointerCaptureRequest::Capture { pointer_id, target } => (pointer_id, target, true),
                PointerCaptureRequest::Release { pointer_id, target } => {
                    (pointer_id, target, false)
                }
            };
            let rejection = if requested != pointer_id {
                Some(TracePointerCaptureRequestRejection::PointerMismatch)
            } else if !transaction
                .pointer_callback_targets
                .iter()
                .any(|node| node == &target)
            {
                Some(TracePointerCaptureRequestRejection::TargetNotInTransaction)
            } else if self.tree.target_status(&target) != TargetStatus::Live {
                Some(TracePointerCaptureRequestRejection::TargetUnavailable)
            } else if capture {
                stream.set_capture_owner(Some(target.clone()));
                None
            } else if stream.capture_owner().is_some_and(|owner| owner == &target) {
                stream.set_capture_owner(None);
                None
            } else {
                Some(TracePointerCaptureRequestRejection::ReleaseNotOwner)
            };
            if let Some(outcome) = rejection {
                transaction.parent = self.trace.record_event(
                    TraceRecordKind::PointerCaptureRequestRejected {
                        pointer_id: requested,
                        outcome,
                    },
                    transaction.sequence,
                    transaction.parent,
                    Some(self.tree.trace_target(&target)),
                    transaction.instant,
                    &transaction.target,
                    Some(&target),
                    transaction.origin,
                );
            }
        }
    }

    fn commit_pointer_plan(
        &mut self,
        plan: PointerCommitPlan,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Result<(), ()> {
        let PointerCommitPlan {
            pointer_id,
            stream,
            kind,
            focus,
            capture_events,
            boundary_notifications,
            physical_target,
            physical_path,
        } = plan;
        match kind {
            StreamCommitKind::Register => {
                let registration_sequence = stream.registration_sequence().get();
                self.pointer_registry
                    .commit_registration(pointer_id, stream)
                    .map_err(map_commit_error)?;
                transaction.parent = self.trace.record(
                    TraceRecordKind::PointerStreamRegistered {
                        pointer_id,
                        registration_sequence,
                    },
                    Some(transaction.sequence),
                    transaction.parent,
                    None,
                    None,
                    None,
                );
            }
            StreamCommitKind::Replace => {
                self.pointer_registry
                    .replace(pointer_id, stream)
                    .map_err(map_commit_error)?;
            }
            StreamCommitKind::Close => {
                self.pointer_registry.close(pointer_id).ok_or(())?;
            }
        }
        self.record_pointer_commit_facts(
            transaction,
            pointer_id,
            kind,
            &capture_events,
            boundary_notifications,
        );
        if let Some(focus) = focus
            && self.validate_focus(&focus)
        {
            self.set_focus(focus);
        }
        self.invoke_pointer_capture_events(
            transaction,
            pointer_id,
            physical_target.as_ref(),
            &physical_path,
            &capture_events,
        )
    }

    fn record_pointer_commit_facts(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        pointer_id: runenui_core::PointerId,
        kind: StreamCommitKind,
        capture_events: &[PointerCaptureEvent],
        boundary_notifications: Vec<runenui_core::PointerBoundaryKind>,
    ) {
        transaction.parent = self.trace.record(
            TraceRecordKind::PointerInteractionCommitted { pointer_id },
            Some(transaction.sequence),
            transaction.parent,
            None,
            None,
            None,
        );
        for capture in capture_events {
            transaction.parent = self.trace.record(
                TraceRecordKind::PointerCaptureTransitionQueued {
                    pointer_id,
                    kind: capture.kind(),
                },
                Some(transaction.sequence),
                transaction.parent,
                None,
                None,
                Some(self.tree.trace_target(capture.target())),
            );
        }
        for kind in boundary_notifications {
            transaction.parent = self.trace.record(
                TraceRecordKind::PointerBoundaryNotificationQueued { pointer_id, kind },
                Some(transaction.sequence),
                transaction.parent,
                None,
                None,
                None,
            );
        }
        for output in &transaction.default_outputs {
            let kind = match output {
                CollectedRoutedOutput::Command {
                    command: SemanticCommand::Activate,
                    ..
                } => Some(TraceRecordKind::PointerActivateCollected { pointer_id }),
                CollectedRoutedOutput::Command {
                    command: SemanticCommand::LogicalScroll(_),
                    ..
                } => Some(TraceRecordKind::PointerLogicalScrollCollected { pointer_id }),
                _ => None,
            };
            if let Some(kind) = kind {
                transaction.parent = self.trace.record(
                    kind,
                    Some(transaction.sequence),
                    transaction.parent,
                    None,
                    None,
                    None,
                );
            }
        }
        if kind == StreamCommitKind::Close {
            transaction.parent = self.trace.record(
                TraceRecordKind::PointerStreamClosed { pointer_id },
                Some(transaction.sequence),
                transaction.parent,
                None,
                None,
                None,
            );
        }
        for output in &mut transaction.default_outputs {
            match output {
                CollectedRoutedOutput::Action { causal_parent, .. }
                | CollectedRoutedOutput::Command { causal_parent, .. } => {
                    *causal_parent = transaction.parent;
                }
            }
        }
    }

    fn invoke_pointer_capture_events(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        pointer_id: runenui_core::PointerId,
        physical_target: Option<&MountedNodeId>,
        physical_path: &[MountedNodeId],
        capture_events: &[PointerCaptureEvent],
    ) -> Result<(), ()> {
        for capture in capture_events {
            if self.tree.target_status(capture.target()) != TargetStatus::Live {
                continue;
            }
            let event = UiEvent::PointerCapture(capture.clone());
            let dispatch = PointerDispatchFacts::new(
                pointer_id,
                physical_target,
                physical_path,
                capture.related_owner(),
                false,
            );
            self.invoke_target_only_pointer_callback(
                transaction,
                &event,
                dispatch,
                capture.target(),
            )
            .map_err(|_| ())?;
            transaction.pointer_capture_requests.clear();
        }
        Ok(())
    }

    fn commit_unrouted_pointer(
        &mut self,
        sequence: runenui_core::WorkSequence,
        mut parent: Option<crate::TraceSequence>,
        plan: PointerCommitPlan,
    ) -> ProcessApplicationActionOutcome {
        let Some(pointer_commit_trace) =
            self.plan_pointer_commit_trace(plan.boundary_notifications.len())
        else {
            return self.pointer_runtime_outcome();
        };
        if !self.trace.can_admit(pointer_commit_trace) {
            let cancelled = self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::TraceSequenceExhausted,
                cancelled,
            };
        }
        let result = match plan.kind {
            StreamCommitKind::Register => {
                let registration_sequence = plan.stream.registration_sequence().get();
                self.pointer_registry
                    .commit_registration(plan.pointer_id, plan.stream)
                    .map_err(map_commit_error)
                    .map(|()| {
                        parent = self.trace.record(
                            TraceRecordKind::PointerStreamRegistered {
                                pointer_id: plan.pointer_id,
                                registration_sequence,
                            },
                            Some(sequence),
                            parent,
                            None,
                            None,
                            None,
                        );
                    })
            }
            StreamCommitKind::Replace => self
                .pointer_registry
                .replace(plan.pointer_id, plan.stream)
                .map_err(map_commit_error),
            StreamCommitKind::Close => {
                self.pointer_registry
                    .close(plan.pointer_id)
                    .ok_or(())
                    .map(|_| {
                        parent = self.trace.record(
                            TraceRecordKind::PointerStreamClosed {
                                pointer_id: plan.pointer_id,
                            },
                            Some(sequence),
                            parent,
                            None,
                            None,
                            None,
                        );
                    })
            }
        };
        if result.is_err() {
            let cancelled = self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::Poisoned,
                cancelled,
            };
        }
        parent = self.trace.record(
            TraceRecordKind::PointerInteractionCommitted {
                pointer_id: plan.pointer_id,
            },
            Some(sequence),
            parent,
            None,
            None,
            None,
        );
        for kind in plan.boundary_notifications {
            parent = self.trace.record(
                TraceRecordKind::PointerBoundaryNotificationQueued {
                    pointer_id: plan.pointer_id,
                    kind,
                },
                Some(sequence),
                parent,
                None,
                None,
                None,
            );
        }
        ProcessApplicationActionOutcome::Completed
    }

    fn plan_pointer_commit_trace(
        &mut self,
        boundary_notifications: usize,
    ) -> Option<MandatoryTracePlan> {
        let plan = MandatoryTracePlan::pointer_commit(boundary_notifications);
        if plan.is_none() {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
        plan
    }

    fn boundary_notification_kinds(
        events: &[PointerBoundaryEvent],
    ) -> Vec<runenui_core::PointerBoundaryKind> {
        events.iter().map(PointerBoundaryEvent::kind).collect()
    }
}

const fn map_commit_error(_: super::PointerCommitError) {}
