use runenui_core::{
    __runtime::PointerCaptureRequest, CommandOrigin, HostProtocol, LogicalDelta,
    LogicalScrollCommand, MonotonicInstant, PointerButton, PointerDeviceKind, PointerId,
    PointerPhase, SemanticCommand, TextDisplayPosition, TextSelection, UiEvent,
};

use super::super::{PointerTextSelectionGesture, TouchGestureState};
use super::{
    PointerBoundaryNotification, PointerBoundaryPlan, PointerCaptureNotification,
    PointerCapturePlan, PointerCaptureTrace, PointerCommitPlan, PointerGeometry,
    PointerIntegrityCleanupPlan, PointerStreamState, PointerWork, PreparedPointer,
    StreamCommitKind, TouchGestureKind, TouchGestureWinner, pointer_default_is_cancelable,
};
use crate::{
    MountedNodeId, RuntimeTerminalReason, TraceContext, TraceDeliveryOutcome, TraceEventContext,
    TraceEventFamily, TracePointerCaptureRequestKind, TracePointerCaptureRequestRejection,
    TracePointerCleanup, TracePointerContext, TracePointerPath, TraceRecordKind,
    TraceRouteSnapshot, TraceRoutedIntegrityFailure, TraceSequence, TraceSurfaceContext,
    TraceTargetTransition, WorkSequence,
    mounted::TargetStatus,
    runtime::{
        CollectedRoutedOutput, MandatoryTracePlan, PointerDispatchFacts,
        ProcessApplicationActionOutcome, RoutedIngressFacts, RoutedTransaction, Runtime,
    },
    trace::{TraceRecordDraft, TraceReservation},
};

struct PendingPointerCommit {
    work: PointerWork,
    stream: PointerStreamState,
    previous_capture_owner: Option<MountedNodeId>,
    geometry: PointerGeometry,
    routed_target: Option<MountedNodeId>,
    kind: StreamCommitKind,
    selection_cancelled: bool,
    selection_tracking: bool,
    touch_cancelled: Option<TouchGestureKind>,
    touch_proposal: Option<TouchGestureWinner>,
}

struct PendingUnroutedPointerCommit {
    work: PointerWork,
    parent: Option<TraceSequence>,
    boundary_plan: PointerBoundaryPlan,
    stream: PointerStreamState,
    kind: StreamCommitKind,
    geometry: PointerGeometry,
    previous_capture_owner: Option<MountedNodeId>,
    selection_cancelled: bool,
    touch_cancelled: Option<TouchGestureKind>,
}

struct PointerCaptureResolutionFacts<'a> {
    sequence: WorkSequence,
    instant: MonotonicInstant,
    pointer_id: PointerId,
    physical_path: &'a [MountedNodeId],
    plan: &'a PointerCapturePlan,
    trace: &'a PointerCaptureTrace,
}

struct RejectedPointerCaptureRequest {
    requested_pointer: PointerId,
    target: MountedNodeId,
    request: TracePointerCaptureRequestKind,
    outcome: TracePointerCaptureRequestRejection,
    previous_owner: Option<MountedNodeId>,
    requested_owner: Option<MountedNodeId>,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    #[allow(clippy::too_many_lines)] // This is the pointer transaction's ordered commit pipeline.
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
            boundary_plan,
            routed_target,
            parent,
            selection_cancelled,
            selection_tracking,
            touch_cancelled,
            touch_proposal,
        } = prepared;
        let kind = Self::stream_commit_kind(&work.event, is_new);
        let boundary_targets = boundary_plan.delivered_targets();
        let deferred_capture_targets = previous_capture_owner
            .iter()
            .filter(|target| self.tree.target_status(target) == TargetStatus::Live)
            .cloned()
            .collect::<Vec<_>>();
        let anchor = routed_target
            .clone()
            .or_else(|| boundary_targets.first().cloned());
        let Some(anchor) = anchor else {
            return self.commit_unrouted_pointer(PendingUnroutedPointerCommit {
                work,
                parent,
                boundary_plan,
                stream,
                kind,
                geometry,
                previous_capture_owner,
                selection_cancelled,
                touch_cancelled,
            });
        };
        let Some(pointer_commit_trace) =
            self.plan_pointer_commit_trace(boundary_plan.notifications.len())
        else {
            return self.pointer_runtime_outcome();
        };
        let facts = RoutedIngressFacts::new(
            work.sequence,
            anchor,
            CommandOrigin::__runtime_pointer(),
            work.instant,
            pointer_event_context(
                routed_target.is_some(),
                work.event.phase(),
                work.event.device_kind(),
            ),
            parent,
            TraceReservation::continuation(),
        );
        let Some(mut transaction) = self.begin_pointer_routed_transaction(
            facts,
            routed_target.is_some(),
            &boundary_targets,
            &deferred_capture_targets,
            2 + usize::from(work.event.drag_drop().is_some()),
            pointer_commit_trace,
            matches!(work.event.phase(), PointerPhase::Down),
        ) else {
            return self.pointer_runtime_outcome();
        };
        transaction
            .pointer_cursor_target
            .clone_from(&geometry.physical_target);
        transaction.pointer_surface_context = Some(work.event.surface_context().clone());
        transaction.pointer_id = Some(work.event.pointer_id());
        transaction.drag_drop_offer = work.event.drag_drop();
        if let Err((failure, current)) =
            self.invoke_pointer_boundary_events(&mut transaction, &work, &geometry, &boundary_plan)
        {
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
                selection_cancelled,
                selection_tracking,
                touch_cancelled,
                touch_proposal,
            },
        )
    }

    #[allow(clippy::too_many_lines)] // Default ordering and integrity settlement must remain explicit.
    fn finish_pointer_transaction(
        &mut self,
        mut transaction: RoutedTransaction<Action>,
        mut pending: PendingPointerCommit,
    ) -> ProcessApplicationActionOutcome {
        let integrity_cleanup = (pending.work.event.phase() == PointerPhase::Up
            && pending.work.event.changed_button() == Some(PointerButton::Primary))
        .then(|| PointerIntegrityCleanupPlan::from_primary_release(&pending.stream))
        .flatten();
        let explicit_capture_request_applied = self.apply_pointer_capture_requests(
            &pending.work,
            &pending.geometry,
            &mut pending.stream,
            &mut transaction,
        );
        if pending.selection_cancelled {
            transaction.pointer_selection_transition =
                Some(crate::runtime::routed::PointerSelectionTransition::Cancelled);
        }
        let text_focus = self.apply_pointer_text_selection_default(
            &pending.work.event,
            &pending.geometry,
            &mut pending.stream,
            &mut transaction,
            pending.selection_tracking && !pending.selection_cancelled,
            explicit_capture_request_applied,
        );
        let default_outputs_before = transaction.default_outputs.len();
        let pointer_focus = match self.apply_pointer_defaults(
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
        let touch_focus = match self.apply_touch_gesture_default(
            &pending.work.event,
            &pending.geometry,
            &mut pending.stream,
            &mut transaction,
            pending.touch_proposal.as_ref(),
            pending.touch_cancelled,
        ) {
            Ok(focus) => focus,
            Err(failure) => {
                let current = transaction.failure_current_target.clone();
                self.poison_transaction(&transaction, failure, current.as_ref());
                return self.pointer_runtime_outcome();
            }
        };
        let focus = pointer_focus.or(touch_focus).or(text_focus);
        if pending.kind == StreamCommitKind::Close
            && pending.stream.text_selection().is_some()
            && transaction.pointer_selection_transition.is_none()
        {
            pending.stream.set_text_selection(None);
            transaction.pointer_selection_transition =
                Some(crate::runtime::routed::PointerSelectionTransition::Cancelled);
        }
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
        let capture_plan = super::notifications::plan_capture_transition(
            pending.work.event.pointer_id(),
            pending.previous_capture_owner.as_ref(),
            final_capture_owner.as_ref(),
            pending.work.event.surface_context(),
            |target| self.tree.target_status(target) == TargetStatus::Live,
        );
        let capture_trace = PointerCaptureTrace {
            device_id: pending.work.event.device_id(),
            device_kind: pending.work.event.device_kind(),
            phase: pending.work.event.phase(),
            surface_context: pending.work.event.surface_context().clone(),
            surface_snapshot: pending.geometry.snapshot,
        };
        self.commit_prepared_pointer_transaction(
            transaction,
            PointerCommitPlan {
                pointer_id: pending.work.event.pointer_id(),
                stream: pending.stream,
                kind: pending.kind,
                focus,
                integrity_cleanup,
                capture_plan,
                capture_trace,
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
        plan: &PointerBoundaryPlan,
    ) -> Result<(), (TraceRoutedIntegrityFailure, MountedNodeId)> {
        for notification in &plan.notifications {
            if notification.delivery == TraceDeliveryOutcome::Suppressed {
                transaction.parent = self.record_pointer_boundary_resolution(
                    work,
                    geometry,
                    plan,
                    notification,
                    transaction.parent,
                );
                continue;
            }
            let boundary = &notification.event;
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
            transaction.parent = self.record_pointer_boundary_resolution(
                work,
                geometry,
                plan,
                notification,
                transaction.parent,
            );
        }
        Ok(())
    }

    fn record_pointer_boundary_resolution(
        &mut self,
        work: &PointerWork,
        geometry: &PointerGeometry,
        plan: &PointerBoundaryPlan,
        notification: &PointerBoundaryNotification,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        let Some(snapshot) = geometry.snapshot else {
            unreachable!("boundary notifications require accepted displayed geometry")
        };
        if !self.trace.is_enabled() {
            return parent;
        }
        let boundary = &notification.event;
        let target = self.tree.trace_target(boundary.target());
        let related_target = boundary
            .related_target()
            .map(|related| self.tree.trace_target(related));
        let route = TraceRouteSnapshot::new(vec![target.clone()], related_target);
        let physical_path = TracePointerPath::new(
            geometry
                .physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let transition = TraceTargetTransition::new(
            plan.previous_target
                .as_ref()
                .map(|node| self.tree.trace_target(node)),
            plan.current_target
                .as_ref()
                .map(|node| self.tree.trace_target(node)),
        );
        let pointer = TracePointerContext::event(
            work.event.pointer_id(),
            work.event.device_id(),
            work.event.device_kind(),
            work.event.phase(),
        );
        let context = TraceContext::pointer_boundary_notification(
            TraceSurfaceContext::accepted(boundary.surface_context(), snapshot),
            pointer,
            route,
            physical_path,
            transition,
            notification.delivery,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerBoundaryNotificationResolved {
                    kind: boundary.kind(),
                },
                work.instant,
                context,
            )
            .with_work_sequence(Some(work.sequence))
            .with_causal_parent(parent)
            .with_target(Some(target)),
        )
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
            pointer_default_is_cancelable(work.event.phase(), work.event.device_kind()),
        );
        self.invoke_routed_callbacks(transaction, &event, Some(dispatch))?;
        if let Some(drop_event) = work.event.drag_drop() {
            let event = UiEvent::DragDrop(drop_event);
            self.invoke_routed_callbacks(transaction, &event, Some(dispatch))?;
        }
        Ok(())
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

    const fn stream_commit_kind(
        event: &runenui_core::PointerEvent,
        is_new: bool,
    ) -> StreamCommitKind {
        if matches!(event.phase(), PointerPhase::Cancel)
            || (matches!(event.phase(), PointerPhase::Up) && event.buttons().is_empty())
        {
            StreamCommitKind::Close
        } else if is_new {
            StreamCommitKind::Register
        } else {
            StreamCommitKind::Replace
        }
    }

    fn apply_pointer_text_selection_default(
        &mut self,
        event: &runenui_core::PointerEvent,
        geometry: &PointerGeometry,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
        tracking: bool,
        explicit_capture_request_applied: bool,
    ) -> Option<MountedNodeId> {
        if !tracking {
            return None;
        }
        let touch_selection_winner = event.device_kind() == PointerDeviceKind::Touch
            && stream
                .touch_gesture()
                .and_then(TouchGestureState::winner)
                .is_some_and(|winner| winner.kind() == TouchGestureKind::TextSelection);
        if event.device_kind() == PointerDeviceKind::Touch && !touch_selection_winner {
            return None;
        }
        match event.phase() {
            PointerPhase::Down => self.start_pointer_text_selection(
                event,
                geometry,
                stream,
                transaction,
                explicit_capture_request_applied,
            ),
            PointerPhase::Move | PointerPhase::Up => {
                self.update_pointer_text_selection(event, stream, transaction)
            }
            PointerPhase::Cancel => {
                if stream.text_selection().is_some() {
                    cancel_pointer_selection(stream, transaction);
                }
                None
            }
            _ => None,
        }
    }

    fn start_pointer_text_selection(
        &mut self,
        event: &runenui_core::PointerEvent,
        geometry: &PointerGeometry,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
        explicit_capture_request_applied: bool,
    ) -> Option<MountedNodeId> {
        if event.device_kind() == PointerDeviceKind::Touch
            || transaction.default_prevented
            || event.changed_button() != Some(PointerButton::Primary)
            || stream.text_selection().is_some()
        {
            return None;
        }
        let owner = geometry.physical_target.as_ref()?;
        if stream
            .capture_owner()
            .is_some_and(|capture_owner| capture_owner != owner)
            || (explicit_capture_request_applied && stream.capture_owner().is_none())
            || !self.editing.has_owner(owner)
            || self.tree.target_status(owner) != TargetStatus::Live
        {
            return None;
        }
        let (caret_map, TextDisplayPosition::Document(hit)) = self
            .surface_publication
            .text_hit_position_at(event.surface_context(), owner, event.position())?
        else {
            return None;
        };
        let selection = if event.modifiers().shift() {
            let current = self.editing.stable_selection_for_map(owner, &caret_map)?;
            TextSelection::new(current.anchor(), hit).ok()?
        } else {
            TextSelection::collapsed(hit)
        };
        let anchor = selection.anchor();
        if self
            .editing
            .validate_selection(owner, selection, &caret_map)
            .is_err()
        {
            return None;
        }
        if stream.capture_owner().is_none() {
            stream.set_capture_owner(Some(owner.clone()));
        }
        stream.set_text_selection(Some(PointerTextSelectionGesture::new(
            owner.clone(),
            anchor,
        )));
        transaction.pointer_selection_update =
            Some(crate::runtime::routed::PointerSelectionUpdate {
                owner: owner.clone(),
                selection,
                caret_map,
            });
        transaction.pointer_selection_transition =
            Some(crate::runtime::routed::PointerSelectionTransition::Started);
        self.validate_focus(owner).then(|| owner.clone())
    }

    fn update_pointer_text_selection(
        &self,
        event: &runenui_core::PointerEvent,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Option<MountedNodeId> {
        let gesture = stream.text_selection().cloned()?;
        let ends_gesture = event.phase() == PointerPhase::Up
            && event.changed_button() == Some(PointerButton::Primary);
        let owns_selection = stream.capture_owner() == Some(gesture.owner())
            || (event.device_kind() == PointerDeviceKind::Touch
                && stream
                    .touch_gesture()
                    .and_then(TouchGestureState::winner)
                    .is_some_and(|winner| winner.owner() == Some(gesture.owner())));
        if !owns_selection
            || self.tree.target_status(gesture.owner()) != TargetStatus::Live
            || !self.editing.has_owner(gesture.owner())
        {
            cancel_pointer_selection(stream, transaction);
            return None;
        }
        if !transaction.default_prevented
            && let Some((caret_map, TextDisplayPosition::Document(active))) =
                self.surface_publication.captured_text_position_at(
                    event.surface_context(),
                    gesture.owner(),
                    event.position(),
                )
        {
            match TextSelection::new(gesture.anchor(), active) {
                Ok(selection)
                    if self
                        .editing
                        .validate_selection(gesture.owner(), selection, &caret_map)
                        .is_ok() =>
                {
                    transaction.pointer_selection_update =
                        Some(crate::runtime::routed::PointerSelectionUpdate {
                            owner: gesture.owner().clone(),
                            selection,
                            caret_map,
                        });
                }
                Ok(_) => {
                    cancel_pointer_selection(stream, transaction);
                    return None;
                }
                Err(_) => {}
            }
        }
        if ends_gesture {
            stream.set_text_selection(None);
            transaction.pointer_selection_transition =
                Some(crate::runtime::routed::PointerSelectionTransition::Ended);
        } else if event.phase() == PointerPhase::Move
            && transaction.pointer_selection_update.is_some()
        {
            transaction.pointer_selection_transition =
                Some(crate::runtime::routed::PointerSelectionTransition::Updated);
        }
        None
    }

    #[allow(clippy::too_many_lines)] // Provisional, winning, and terminal touch phases form one state machine.
    fn apply_touch_gesture_default(
        &mut self,
        event: &runenui_core::PointerEvent,
        geometry: &PointerGeometry,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
        proposal: Option<&TouchGestureWinner>,
        touch_cancelled: Option<TouchGestureKind>,
    ) -> Result<Option<MountedNodeId>, TraceRoutedIntegrityFailure> {
        if event.device_kind() != PointerDeviceKind::Touch {
            return Ok(None);
        }
        if let Some(gesture) = touch_cancelled {
            self.record_touch_gesture_fact(
                transaction,
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id: event.pointer_id(),
                    gesture: gesture.trace_kind(),
                },
                None,
            );
        }
        match event.phase() {
            PointerPhase::Down => {
                if event.changed_button() != Some(PointerButton::Primary) {
                    return Ok(None);
                }
                let state = self.start_touch_gesture(event, geometry);
                let thresholds = self.touch_gesture_thresholds;
                self.record_touch_gesture_fact(
                    transaction,
                    TraceRecordKind::TouchGestureProvisional {
                        pointer_id: event.pointer_id(),
                        thresholds,
                        scroll_candidates: state.scroll_candidates().len(),
                        selection_candidate: state.selection_candidate().is_some(),
                    },
                    state.origin_target(),
                );
                stream.set_touch_gesture(Some(state));
                if let Some(owner) = stream.capture_owner().cloned() {
                    let winner = TouchGestureWinner::new(TouchGestureKind::Capture, Some(owner));
                    self.resolve_touch_gesture_winner(event, stream, transaction, winner)?;
                }
                Ok(None)
            }
            PointerPhase::Move => {
                let Some(state) = stream.touch_gesture().cloned() else {
                    return Ok(None);
                };
                if state.cancelled() {
                    return Ok(None);
                }
                let existing_winner = state.winner().cloned();
                if existing_winner.is_none() {
                    let capture_winner = stream.capture_owner().cloned().map(|owner| {
                        TouchGestureWinner::new(TouchGestureKind::Capture, Some(owner))
                    });
                    let next = capture_winner.or_else(|| proposal.cloned());
                    if let Some(winner) = next
                        && (winner.kind() == TouchGestureKind::Capture
                            || !transaction.default_prevented)
                    {
                        let focus =
                            self.resolve_touch_gesture_winner(event, stream, transaction, winner)?;
                        if let Some(gesture) = stream.touch_gesture.as_mut() {
                            gesture.advance(event.position());
                        }
                        return Ok(focus);
                    }
                } else if !transaction.default_prevented
                    && existing_winner
                        .as_ref()
                        .is_some_and(|winner| winner.kind() == TouchGestureKind::Scroll)
                    && let Some(delta) = touch_delta(state.last_position(), event.position())
                {
                    self.apply_logical_scroll_default(
                        transaction,
                        negate_delta(delta),
                        event.surface_context().hit_test_generation(),
                        event.surface_context().coordinate_revision(),
                    );
                }
                if let Some(gesture) = stream.touch_gesture.as_mut() {
                    gesture.advance(event.position());
                }
                Ok(None)
            }
            PointerPhase::Up => {
                let Some(state) = stream.touch_gesture() else {
                    return Ok(None);
                };
                if !state.cancelled() {
                    let winner = state.winner().cloned().unwrap_or_else(|| {
                        TouchGestureWinner::new(
                            TouchGestureKind::Tap,
                            state.origin_target().cloned(),
                        )
                    });
                    if state.winner().is_none() {
                        self.record_touch_winner_and_losers(
                            event.pointer_id(),
                            transaction,
                            state,
                            &winner,
                        );
                    }
                    self.record_touch_gesture_fact(
                        transaction,
                        TraceRecordKind::TouchGestureCompleted {
                            pointer_id: event.pointer_id(),
                            gesture: winner.kind().trace_kind(),
                        },
                        winner.owner(),
                    );
                }
                Ok(None)
            }
            PointerPhase::Cancel => {
                let Some(state) = stream.touch_gesture().cloned() else {
                    return Ok(None);
                };
                if !state.cancelled() {
                    if let Some(winner) = state.winner() {
                        self.record_touch_gesture_fact(
                            transaction,
                            TraceRecordKind::TouchGestureCancelled {
                                pointer_id: event.pointer_id(),
                                gesture: winner.kind().trace_kind(),
                            },
                            winner.owner(),
                        );
                    } else {
                        self.record_touch_gesture_fact(
                            transaction,
                            TraceRecordKind::TouchGestureCancelled {
                                pointer_id: event.pointer_id(),
                                gesture: TouchGestureKind::Tap.trace_kind(),
                            },
                            state.origin_target(),
                        );
                        if !state.scroll_candidates().is_empty() {
                            self.record_touch_gesture_fact(
                                transaction,
                                TraceRecordKind::TouchGestureCancelled {
                                    pointer_id: event.pointer_id(),
                                    gesture: TouchGestureKind::Scroll.trace_kind(),
                                },
                                state.scroll_candidates().last(),
                            );
                        }
                        if let Some(candidate) = state.selection_candidate() {
                            self.record_touch_gesture_fact(
                                transaction,
                                TraceRecordKind::TouchGestureCancelled {
                                    pointer_id: event.pointer_id(),
                                    gesture: TouchGestureKind::TextSelection.trace_kind(),
                                },
                                Some(candidate.owner()),
                            );
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn resolve_touch_gesture_winner(
        &mut self,
        event: &runenui_core::PointerEvent,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
        mut winner: TouchGestureWinner,
    ) -> Result<Option<MountedNodeId>, TraceRoutedIntegrityFailure> {
        let Some(mut state) = stream.touch_gesture().cloned() else {
            return Ok(None);
        };
        if state.cancelled() || state.winner().is_some() {
            return Ok(None);
        }
        if winner.kind() == TouchGestureKind::TextSelection {
            let Some(candidate) = state.selection_candidate() else {
                return Ok(None);
            };
            let Some((caret_map, TextDisplayPosition::Document(active))) =
                self.surface_publication.captured_text_position_at(
                    event.surface_context(),
                    candidate.owner(),
                    event.position(),
                )
            else {
                winner =
                    TouchGestureWinner::new(TouchGestureKind::Move, state.origin_target().cloned());
                state.set_winner(winner.clone());
                stream.set_pressed_owner(None);
                self.record_touch_winner_and_losers(
                    event.pointer_id(),
                    transaction,
                    &state,
                    &winner,
                );
                stream.set_touch_gesture(Some(state));
                return Ok(None);
            };
            let selection = TextSelection::new(candidate.anchor(), active)
                .map_err(|_| TraceRoutedIntegrityFailure::SemanticDefaultFailure)?;
            if self
                .editing
                .validate_selection(candidate.owner(), selection, &caret_map)
                .is_err()
            {
                winner =
                    TouchGestureWinner::new(TouchGestureKind::Move, state.origin_target().cloned());
                state.set_winner(winner.clone());
                stream.set_pressed_owner(None);
                self.record_touch_winner_and_losers(
                    event.pointer_id(),
                    transaction,
                    &state,
                    &winner,
                );
                stream.set_touch_gesture(Some(state));
                return Ok(None);
            }
            let owner = candidate.owner().clone();
            stream.set_text_selection(Some(PointerTextSelectionGesture::new(
                owner.clone(),
                candidate.anchor(),
            )));
            transaction.pointer_selection_update =
                Some(crate::runtime::routed::PointerSelectionUpdate {
                    owner,
                    selection,
                    caret_map,
                });
            transaction.pointer_selection_transition =
                Some(crate::runtime::routed::PointerSelectionTransition::Started);
        }
        state.set_winner(winner.clone());
        if winner.kind() != TouchGestureKind::Capture {
            stream.set_pressed_owner(None);
        }
        self.record_touch_winner_and_losers(event.pointer_id(), transaction, &state, &winner);
        let focus = if winner.kind() == TouchGestureKind::TextSelection {
            winner
                .owner()
                .filter(|owner| self.validate_focus(owner))
                .cloned()
        } else {
            None
        };
        if winner.kind() == TouchGestureKind::Scroll
            && !transaction.default_prevented
            && let Some(total) = touch_delta(state.start_position(), event.position())
        {
            self.apply_logical_scroll_default(
                transaction,
                negate_delta(total),
                event.surface_context().hit_test_generation(),
                event.surface_context().coordinate_revision(),
            );
        }
        state.advance(event.position());
        stream.set_touch_gesture(Some(state));
        Ok(focus)
    }

    fn record_touch_winner_and_losers(
        &mut self,
        pointer_id: PointerId,
        transaction: &mut RoutedTransaction<Action>,
        state: &TouchGestureState,
        winner: &TouchGestureWinner,
    ) {
        if winner.kind() != TouchGestureKind::Tap {
            self.record_touch_gesture_fact(
                transaction,
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id,
                    gesture: TouchGestureKind::Tap.trace_kind(),
                },
                state.origin_target(),
            );
        }
        if winner.kind() != TouchGestureKind::Scroll && !state.scroll_candidates().is_empty() {
            self.record_touch_gesture_fact(
                transaction,
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id,
                    gesture: TouchGestureKind::Scroll.trace_kind(),
                },
                state.scroll_candidates().last(),
            );
        }
        if winner.kind() != TouchGestureKind::TextSelection
            && let Some(candidate) = state.selection_candidate()
        {
            self.record_touch_gesture_fact(
                transaction,
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id,
                    gesture: TouchGestureKind::TextSelection.trace_kind(),
                },
                Some(candidate.owner()),
            );
        }
        self.record_touch_gesture_fact(
            transaction,
            TraceRecordKind::TouchGestureWon {
                pointer_id,
                gesture: winner.kind().trace_kind(),
            },
            winner.owner(),
        );
    }

    fn record_touch_gesture_fact(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        kind: TraceRecordKind,
        owner: Option<&MountedNodeId>,
    ) {
        let target = owner.cloned().unwrap_or_else(|| transaction.target.clone());
        transaction.parent = self.trace.record_event(
            kind,
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(&target)),
            transaction.instant,
            &transaction.target,
            Some(&target),
            transaction.origin,
        );
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
                if event.device_kind() != PointerDeviceKind::Touch {
                    stream.set_capture_owner(Some(target.clone()));
                }
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
                let primary_release = event.changed_button() == Some(PointerButton::Primary);
                if !transaction.default_prevented
                    && primary_release
                    && let Some(owner) = stream.pressed_owner().cloned()
                    && physical_path.iter().any(|target| target == &owner)
                    && self
                        .tree
                        .activation(&owner)
                        .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
                {
                    Self::push_pointer_default(transaction, owner, SemanticCommand::Activate)?;
                }
                if primary_release {
                    stream.set_pressed_owner(None);
                    stream.set_capture_owner(None);
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
                            event.surface_context(),
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
        work: &PointerWork,
        geometry: &PointerGeometry,
        stream: &mut PointerStreamState,
        transaction: &mut RoutedTransaction<Action>,
    ) -> bool {
        let mut explicit_request_applied = false;
        for request in core::mem::take(&mut transaction.pointer_capture_requests) {
            let previous_owner = stream.capture_owner().cloned();
            let (requested_pointer, target, request_kind, requested_owner) = match request {
                PointerCaptureRequest::Capture { pointer_id, target } => (
                    pointer_id,
                    target.clone(),
                    TracePointerCaptureRequestKind::Capture,
                    Some(target),
                ),
                PointerCaptureRequest::Release { pointer_id, target } => (
                    pointer_id,
                    target,
                    TracePointerCaptureRequestKind::Release,
                    None,
                ),
            };
            let rejection = if requested_pointer != work.event.pointer_id() {
                Some(TracePointerCaptureRequestRejection::PointerMismatch)
            } else if !transaction
                .pointer_callback_targets
                .iter()
                .any(|node| node == &target)
            {
                Some(TracePointerCaptureRequestRejection::TargetNotInTransaction)
            } else if self.tree.target_status(&target) != TargetStatus::Live {
                Some(TracePointerCaptureRequestRejection::TargetUnavailable)
            } else if work.event.device_kind() == PointerDeviceKind::Touch
                && stream.touch_gesture().is_some_and(|gesture| {
                    gesture
                        .winner()
                        .is_some_and(|winner| winner.owner() != Some(&target))
                })
            {
                Some(TracePointerCaptureRequestRejection::TouchGestureCommitted)
            } else if request_kind == TracePointerCaptureRequestKind::Capture {
                stream.set_capture_owner(Some(target.clone()));
                explicit_request_applied = true;
                None
            } else if stream.capture_owner().is_some_and(|owner| owner == &target) {
                stream.set_capture_owner(None);
                explicit_request_applied = true;
                None
            } else {
                Some(TracePointerCaptureRequestRejection::ReleaseNotOwner)
            };
            if let Some(outcome) = rejection {
                transaction.parent = self.record_pointer_capture_request_rejection(
                    work,
                    geometry,
                    RejectedPointerCaptureRequest {
                        requested_pointer,
                        target,
                        request: request_kind,
                        outcome,
                        previous_owner,
                        requested_owner,
                    },
                    transaction.parent,
                );
            }
        }
        explicit_request_applied
    }

    fn record_pointer_capture_request_rejection(
        &mut self,
        work: &PointerWork,
        geometry: &PointerGeometry,
        rejected: RejectedPointerCaptureRequest,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let RejectedPointerCaptureRequest {
            requested_pointer,
            target,
            request,
            outcome,
            previous_owner,
            requested_owner,
        } = rejected;
        let target = self.tree.trace_target(&target);
        let physical_path = TracePointerPath::new(
            geometry
                .physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let transition = TraceTargetTransition::new(
            previous_owner
                .as_ref()
                .map(|owner| self.tree.trace_target(owner)),
            requested_owner
                .as_ref()
                .map(|owner| self.tree.trace_target(owner)),
        );
        let event_pointer = TracePointerContext::event(
            work.event.pointer_id(),
            work.event.device_id(),
            work.event.device_kind(),
            work.event.phase(),
        );
        let surface = Some(geometry.snapshot.map_or_else(
            || TraceSurfaceContext::requested(work.event.surface_context()),
            |snapshot| TraceSurfaceContext::accepted(work.event.surface_context(), snapshot),
        ));
        let context = TraceContext::pointer_capture_request_rejection(
            surface,
            event_pointer,
            requested_pointer,
            physical_path,
            transition,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerCaptureRequestRejected { request, outcome },
                work.instant,
                context,
            )
            .with_work_sequence(Some(work.sequence))
            .with_causal_parent(parent)
            .with_target(Some(target)),
        )
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
            integrity_cleanup,
            capture_plan,
            capture_trace,
            physical_target,
            physical_path,
        } = plan;
        self.commit_pending_modality(transaction);
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
        if let Some(cleanup) = integrity_cleanup.as_ref() {
            self.record_pointer_integrity_cleanup(
                transaction,
                pointer_id,
                cleanup,
                &capture_trace,
                &physical_path,
            );
        }
        self.record_pointer_commit_facts(transaction, pointer_id, kind);
        if let Some(focus) = focus
            && self.validate_focus(&focus)
        {
            self.commit_focus_transition(
                transaction,
                Some(focus),
                runenui_core::FocusReason::Pointer,
            )
            .map_err(|_| ())?;
        }
        self.invoke_pointer_capture_events(
            transaction,
            pointer_id,
            physical_target.as_ref(),
            &physical_path,
            &capture_plan,
            &capture_trace,
        )
    }

    fn record_pointer_integrity_cleanup(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        pointer_id: PointerId,
        cleanup: &PointerIntegrityCleanupPlan,
        trace: &PointerCaptureTrace,
        physical_path: &[MountedNodeId],
    ) {
        if !self.trace.is_enabled() {
            return;
        }
        let physical_path = TracePointerPath::new(
            physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let pressed_owner = cleanup
            .pressed_owner
            .as_ref()
            .map(|owner| TraceTargetTransition::new(Some(self.tree.trace_target(owner)), None));
        let capture_owner = cleanup
            .capture_owner
            .as_ref()
            .map(|owner| TraceTargetTransition::new(Some(self.tree.trace_target(owner)), None));
        let pointer =
            TracePointerContext::event(pointer_id, trace.device_id, trace.device_kind, trace.phase);
        let surface = Some(trace.surface_snapshot.map_or_else(
            || TraceSurfaceContext::requested(&trace.surface_context),
            |snapshot| TraceSurfaceContext::accepted(&trace.surface_context, snapshot),
        ));
        let context = TraceContext::pointer_integrity_cleanup(
            surface,
            pointer,
            physical_path,
            TracePointerCleanup::new(pressed_owner, capture_owner, false),
        );
        transaction.parent = self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerIntegrityCleanupCommitted,
                transaction.instant,
                context,
            )
            .with_work_sequence(Some(transaction.sequence))
            .with_causal_parent(transaction.parent),
        );
    }

    fn record_pointer_commit_facts(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        pointer_id: PointerId,
        kind: StreamCommitKind,
    ) {
        transaction.parent = self.trace.record(
            TraceRecordKind::PointerInteractionCommitted { pointer_id },
            Some(transaction.sequence),
            transaction.parent,
            None,
            None,
            None,
        );
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
                | CollectedRoutedOutput::EditAction { causal_parent, .. }
                | CollectedRoutedOutput::Command { causal_parent, .. } => {
                    *causal_parent = transaction.parent;
                }
            }
        }
    }

    fn invoke_pointer_capture_events(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        pointer_id: PointerId,
        physical_target: Option<&MountedNodeId>,
        physical_path: &[MountedNodeId],
        plan: &PointerCapturePlan,
        trace: &PointerCaptureTrace,
    ) -> Result<(), ()> {
        let resolution = PointerCaptureResolutionFacts {
            sequence: transaction.sequence,
            instant: transaction.instant,
            pointer_id,
            physical_path,
            plan,
            trace,
        };
        for notification in &plan.notifications {
            if notification.delivery == TraceDeliveryOutcome::Suppressed {
                transaction.parent = self.record_pointer_capture_resolution(
                    &resolution,
                    notification,
                    transaction.parent,
                );
                continue;
            }
            let capture = &notification.event;
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
            transaction.parent = self.record_pointer_capture_resolution(
                &resolution,
                notification,
                transaction.parent,
            );
        }
        Ok(())
    }

    fn record_pointer_capture_resolution(
        &mut self,
        facts: &PointerCaptureResolutionFacts<'_>,
        notification: &PointerCaptureNotification,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let capture = &notification.event;
        let target = self.tree.trace_target(capture.target());
        let related_target = capture
            .related_owner()
            .map(|related| self.tree.trace_target(related));
        let route = TraceRouteSnapshot::new(vec![target.clone()], related_target);
        let physical_path = TracePointerPath::new(
            facts
                .physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let transition = TraceTargetTransition::new(
            facts
                .plan
                .previous_owner
                .as_ref()
                .map(|owner| self.tree.trace_target(owner)),
            facts
                .plan
                .current_owner
                .as_ref()
                .map(|owner| self.tree.trace_target(owner)),
        );
        let pointer = TracePointerContext::event(
            facts.pointer_id,
            facts.trace.device_id,
            facts.trace.device_kind,
            facts.trace.phase,
        );
        let surface = Some(facts.trace.surface_snapshot.map_or_else(
            || TraceSurfaceContext::requested(&facts.trace.surface_context),
            |snapshot| TraceSurfaceContext::accepted(&facts.trace.surface_context, snapshot),
        ));
        let context = TraceContext::pointer_capture_notification(
            surface,
            pointer,
            route,
            physical_path,
            transition,
            notification.delivery,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerCaptureNotificationResolved {
                    kind: capture.kind(),
                },
                facts.instant,
                context,
            )
            .with_work_sequence(Some(facts.sequence))
            .with_causal_parent(parent)
            .with_target(Some(target)),
        )
    }

    #[allow(clippy::too_many_lines)] // Handles close-only stream commits without creating a routed transaction.
    fn commit_unrouted_pointer(
        &mut self,
        pending: PendingUnroutedPointerCommit,
    ) -> ProcessApplicationActionOutcome {
        let PendingUnroutedPointerCommit {
            work,
            mut parent,
            boundary_plan,
            stream,
            kind,
            geometry,
            previous_capture_owner,
            selection_cancelled,
            touch_cancelled,
        } = pending;
        let Some(pointer_commit_trace) =
            self.plan_unrouted_pointer_commit_trace(boundary_plan.notifications.len())
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
        for notification in &boundary_plan.notifications {
            debug_assert_eq!(notification.delivery, TraceDeliveryOutcome::Suppressed);
            parent = self.record_pointer_boundary_resolution(
                &work,
                &geometry,
                &boundary_plan,
                notification,
                parent,
            );
        }
        let pointer_id = work.event.pointer_id();
        let final_capture_owner = if kind == StreamCommitKind::Close {
            None
        } else {
            stream.capture_owner().cloned()
        };
        let capture_plan = super::notifications::plan_capture_transition(
            pointer_id,
            previous_capture_owner.as_ref(),
            final_capture_owner.as_ref(),
            work.event.surface_context(),
            |target| self.tree.target_status(target) == TargetStatus::Live,
        );
        let capture_trace = PointerCaptureTrace {
            device_id: work.event.device_id(),
            device_kind: work.event.device_kind(),
            phase: work.event.phase(),
            surface_context: work.event.surface_context().clone(),
            surface_snapshot: geometry.snapshot,
        };
        let pointer_interaction_before = self.pointer_registry.surface_interaction_projection(None);
        if self
            .commit_unrouted_pointer_stream(pointer_id, stream, kind, work.sequence, &mut parent)
            .is_err()
        {
            let cancelled = self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::Poisoned,
                cancelled,
            };
        }
        parent = self.trace.record(
            TraceRecordKind::PointerInteractionCommitted { pointer_id },
            Some(work.sequence),
            parent,
            None,
            None,
            None,
        );
        if selection_cancelled {
            parent = self.trace.record(
                TraceRecordKind::PointerTextSelectionCancelled { pointer_id },
                Some(work.sequence),
                parent,
                None,
                None,
                None,
            );
        }
        if let Some(gesture) = touch_cancelled {
            parent = self.trace.record(
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id,
                    gesture: gesture.trace_kind(),
                },
                Some(work.sequence),
                parent,
                None,
                None,
                None,
            );
        }
        let resolution = PointerCaptureResolutionFacts {
            sequence: work.sequence,
            instant: work.instant,
            pointer_id,
            physical_path: &geometry.physical_path,
            plan: &capture_plan,
            trace: &capture_trace,
        };
        for notification in &capture_plan.notifications {
            debug_assert_eq!(notification.delivery, TraceDeliveryOutcome::Suppressed);
            parent = self.record_pointer_capture_resolution(&resolution, notification, parent);
        }
        if pointer_interaction_before
            .content_differs(&self.pointer_registry.surface_interaction_projection(None))
        {
            self.request_redraw(parent, work.instant);
        }
        ProcessApplicationActionOutcome::Completed
    }

    fn commit_unrouted_pointer_stream(
        &mut self,
        pointer_id: PointerId,
        stream: PointerStreamState,
        kind: StreamCommitKind,
        sequence: WorkSequence,
        parent: &mut Option<TraceSequence>,
    ) -> Result<(), ()> {
        match kind {
            StreamCommitKind::Register => {
                let registration_sequence = stream.registration_sequence().get();
                self.pointer_registry
                    .commit_registration(pointer_id, stream)
                    .map_err(map_commit_error)?;
                *parent = self.trace.record(
                    TraceRecordKind::PointerStreamRegistered {
                        pointer_id,
                        registration_sequence,
                    },
                    Some(sequence),
                    *parent,
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
                *parent = self.trace.record(
                    TraceRecordKind::PointerStreamClosed { pointer_id },
                    Some(sequence),
                    *parent,
                    None,
                    None,
                    None,
                );
            }
        }
        Ok(())
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

    fn plan_unrouted_pointer_commit_trace(
        &mut self,
        boundary_notifications: usize,
    ) -> Option<MandatoryTracePlan> {
        let plan = MandatoryTracePlan::pointer_unrouted_commit(boundary_notifications);
        if plan.is_none() {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
        plan
    }
}

fn pointer_event_context(
    has_routed_target: bool,
    phase: PointerPhase,
    device_kind: PointerDeviceKind,
) -> TraceEventContext {
    if has_routed_target {
        TraceEventContext::new(
            TraceEventFamily::Pointer,
            pointer_default_is_cancelable(phase, device_kind),
        )
    } else {
        TraceEventContext::new(TraceEventFamily::PointerBoundary, false)
    }
}

fn touch_delta(
    from: runenui_core::LogicalPoint,
    to: runenui_core::LogicalPoint,
) -> Option<LogicalDelta> {
    LogicalDelta::new(to.x() - from.x(), to.y() - from.y()).ok()
}

fn cancel_pointer_selection<Action>(
    stream: &mut PointerStreamState,
    transaction: &mut RoutedTransaction<Action>,
) {
    stream.set_text_selection(None);
    transaction.pointer_selection_transition =
        Some(crate::runtime::routed::PointerSelectionTransition::Cancelled);
}

fn negate_delta(delta: LogicalDelta) -> LogicalDelta {
    LogicalDelta::new(-delta.x(), -delta.y())
        .unwrap_or_else(|_| unreachable!("finite pointer deltas remain finite when negated"))
}

const fn map_commit_error(_: super::PointerCommitError) {}
