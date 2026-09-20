use super::super::{TouchGestureState, TouchTextSelectionCandidate};
use runenui_core::{
    HostProtocol, OverflowPolicy, PointerButton, PointerButtons, PointerEvent, PointerPhase,
    TextDisplayPosition, TextSelection,
};

use super::{
    PointerBoundaryPlan, PointerGeometry, PointerOwnerCleanup, PointerStreamState, PointerWork,
    StreamPreparation, TouchGestureKind, TouchGestureWinner,
};
use crate::{
    MountedNodeId, RuntimeTerminalReason, TraceContext, TraceEventContext, TraceEventFamily,
    TracePointerContext, TracePointerPath, TraceRecordKind, TraceSequence, TraceSurfaceContext,
    TraceTargetTransition,
    mounted::{RouteBuildError, TargetStatus},
    runtime::{MandatoryTracePlan, ProcessApplicationActionOutcome, Runtime},
    trace::TraceRecordDraft,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn prepare_pointer_stream(
        &mut self,
        work: &PointerWork,
    ) -> Result<StreamPreparation, ProcessApplicationActionOutcome> {
        let pointer_id = work.event.pointer_id();
        let phase = work.event.phase();
        let surface = self
            .surface_publication
            .validate_surface_identity(work.event.surface_context())
            .map_err(|error| {
                self.reject_pointer_preparation(work, super::rejection::map_surface_error(error))
            })?;
        let existing = match self.pointer_registry.validate(
            pointer_id,
            &surface,
            work.event.device_id(),
            work.event.device_kind(),
        ) {
            Ok(stream) => Some(stream.clone()),
            Err(super::PointerStreamError::Missing) => None,
            Err(error) => {
                return Err(self
                    .reject_pointer_preparation(work, super::rejection::map_stream_error(error)));
            }
        };
        if work.event.device_kind() == runenui_core::PointerDeviceKind::Touch
            && !touch_transition_is_supported(&work.event, existing.as_ref())
        {
            return Err(self.reject_pointer_preparation(
                work,
                crate::trace::TracePointerRejection::TouchProfileUnsupported,
            ));
        }
        if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) && existing.is_none() {
            return Err(self.reject_pointer_preparation(
                work,
                crate::trace::TracePointerRejection::MissingStream,
            ));
        }
        if !pointer_button_transition_is_valid(&work.event, existing.as_ref()) {
            return Err(self.reject_pointer_preparation(
                work,
                crate::trace::TracePointerRejection::ButtonTransitionMismatch,
            ));
        }
        let is_new = existing.is_none();
        let stream = match existing {
            Some(stream) => stream,
            None => self
                .pointer_registry
                .plan_registration(
                    pointer_id,
                    surface,
                    work.event.device_id(),
                    work.event.device_kind(),
                    work.event.position(),
                    work.event.buttons().clone(),
                )
                .map_err(|error| {
                    self.reject_pointer_preparation(
                        work,
                        super::rejection::map_registration_error(error),
                    )
                })?,
        };
        Ok(StreamPreparation { is_new, stream })
    }

    fn reject_pointer_preparation(
        &mut self,
        work: &PointerWork,
        outcome: crate::trace::TracePointerRejection,
    ) -> ProcessApplicationActionOutcome {
        self.reject_pointer(super::rejection::RejectedPointerFacts::new(
            work.sequence,
            work.causal_parent,
            work.trace_reservation,
            work.event.pointer_id(),
            work.event.phase(),
            outcome,
        ))
    }

    pub(super) fn resolve_pointer_geometry(
        &mut self,
        work: &PointerWork,
        stream: &super::PointerStreamState,
    ) -> Result<PointerGeometry, ProcessApplicationActionOutcome> {
        if matches!(work.event.phase(), PointerPhase::Cancel) {
            let diagnosis = self
                .surface_publication
                .resolve_pointer_point(work.event.surface_context(), work.event.position())
                .err()
                .map(super::rejection::map_surface_error);
            let physical_path = stream.physical_path().to_vec();
            return Ok(PointerGeometry {
                physical_target: physical_path.last().cloned(),
                physical_path,
                snapshot: None,
                diagnosis,
            });
        }
        let resolution = match self
            .surface_publication
            .resolve_pointer_point(work.event.surface_context(), work.event.position())
        {
            Ok(resolution) => resolution,
            Err(
                error @ (super::SurfaceSnapshotError::RetiredSurfaceContext
                | super::SurfaceSnapshotError::MissingSurfaceGeneration),
            ) if matches!(work.event.phase(), PointerPhase::Up) => {
                return Err(self.settle_unavailable_pointer_up(work, error, stream.clone()));
            }
            Err(error) => {
                return Err(
                    self.reject_pointer(super::rejection::RejectedPointerFacts::new(
                        work.sequence,
                        work.causal_parent,
                        work.trace_reservation,
                        work.event.pointer_id(),
                        work.event.phase(),
                        super::rejection::map_surface_error(error),
                    )),
                );
            }
        };
        let snapshot = super::rejection::map_snapshot_kind(resolution.snapshot_kind());
        let physical_target = resolution.into_target();
        let (physical_target, physical_path) = match physical_target {
            Some(target) => match self.tree.event_route(&target) {
                Ok(path) => (Some(target), path),
                Err(RouteBuildError::Target(TargetStatus::Stale)) => {
                    // A runtime-authored displayed snapshot may legally outlive the mounted
                    // generation it names. Down has no pre-existing routing authority, so reject
                    // it without committing a stream. Existing-stream phases retain their own
                    // live capture/pressed routing authority while the stale physical hit becomes
                    // no live physical target; never retarget through current geometry.
                    if matches!(work.event.phase(), PointerPhase::Down) {
                        return Err(self.reject_pointer(
                            super::rejection::RejectedPointerFacts::new(
                                work.sequence,
                                work.causal_parent,
                                work.trace_reservation,
                                work.event.pointer_id(),
                                work.event.phase(),
                                crate::trace::TracePointerRejection::NoTarget,
                            ),
                        ));
                    }
                    (None, Vec::new())
                }
                Err(
                    RouteBuildError::Target(
                        TargetStatus::Live | TargetStatus::Missing | TargetStatus::Foreign,
                    )
                    | RouteBuildError::BrokenTopology
                    | RouteBuildError::BridgeMismatch,
                ) => {
                    self.trace.release_reservation(work.trace_reservation);
                    let cancelled = self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return Err(ProcessApplicationActionOutcome::Terminal {
                        reason: RuntimeTerminalReason::Poisoned,
                        cancelled,
                    });
                }
            },
            None => (None, Vec::new()),
        };
        Ok(PointerGeometry {
            physical_target,
            physical_path,
            snapshot: Some(snapshot),
            diagnosis: None,
        })
    }

    pub(super) fn clear_non_live_pointer_owners(
        &self,
        stream: &mut super::PointerStreamState,
    ) -> PointerOwnerCleanup {
        if stream
            .capture_owner()
            .is_some_and(|owner| self.tree.target_status(owner) != TargetStatus::Live)
        {
            stream.set_capture_owner(None);
        }
        if stream
            .pressed_owner()
            .is_some_and(|owner| self.tree.target_status(owner) != TargetStatus::Live)
        {
            stream.set_pressed_owner(None);
        }
        let selection_cancelled = stream.text_selection().is_some_and(|selection| {
            self.tree.target_status(selection.owner()) != TargetStatus::Live
                || !self.editing.has_owner(selection.owner())
        });
        if selection_cancelled {
            stream.set_text_selection(None);
        }
        let touch_owner_lost = stream.touch_gesture().is_some_and(|gesture| {
            !gesture.cancelled()
                && (gesture
                    .origin_route()
                    .iter()
                    .any(|owner| self.tree.target_status(owner) != TargetStatus::Live)
                    || gesture
                        .scroll_candidates()
                        .iter()
                        .any(|owner| self.tree.target_status(owner) != TargetStatus::Live)
                    || gesture.selection_candidate().is_some_and(|candidate| {
                        self.tree.target_status(candidate.owner()) != TargetStatus::Live
                            || !self.editing.has_owner(candidate.owner())
                    })
                    || gesture.winner().is_some_and(|winner| {
                        winner.owner().is_some_and(|owner| {
                            self.tree.target_status(owner) != TargetStatus::Live
                        })
                    }))
        });
        let touch_cancelled = touch_owner_lost
            .then(|| stream.touch_gesture.as_mut().map(TouchGestureState::cancel))
            .flatten();
        PointerOwnerCleanup {
            selection_cancelled,
            touch_cancelled,
        }
    }

    #[allow(clippy::too_many_lines)] // One ordered arbitration compares all provisional competitors.
    pub(super) fn touch_gesture_proposal(
        &self,
        event: &PointerEvent,
        stream: &PointerStreamState,
    ) -> Option<TouchGestureWinner> {
        let gesture = stream.touch_gesture()?;
        if gesture.cancelled() || gesture.winner().is_some() {
            return None;
        }
        let thresholds = self.touch_gesture_thresholds;
        let dx = f64::from(event.position().x()) - f64::from(gesture.start_position().x());
        let dy = f64::from(event.position().y()) - f64::from(gesture.start_position().y());
        let abs_x = dx.abs();
        let abs_y = dy.abs();
        let distance = abs_x.hypot(abs_y);
        let scroll_threshold = f64::from(thresholds.scroll_movement());
        let selection_threshold = f64::from(thresholds.selection_movement());

        let scroll_owner = gesture.scroll_candidates().iter().rev().find(|owner| {
            let Some(metrics) = self.surface_publication.displayed_scroll_metrics(
                event.surface_context().hit_test_generation(),
                event.surface_context().coordinate_revision(),
                owner,
            ) else {
                return false;
            };
            let Some(node) = self.tree.node(owner) else {
                return false;
            };
            let offset = node.interaction.scroll_offset;
            let max_x = (metrics.content.width() - metrics.viewport.width()).max(0.0);
            let max_y = (metrics.content.height() - metrics.viewport.height()).max(0.0);
            let can_scroll_x = metrics.overflow.horizontal() == OverflowPolicy::Scroll
                && max_x > 0.0
                && if dx < 0.0 {
                    offset.0 < max_x
                } else {
                    offset.0 > 0.0
                };
            let can_scroll_y = metrics.overflow.vertical() == OverflowPolicy::Scroll
                && max_y > 0.0
                && if dy < 0.0 {
                    offset.1 < max_y
                } else {
                    offset.1 > 0.0
                };
            (abs_x > 0.0 && can_scroll_x) || (abs_y > 0.0 && can_scroll_y)
        });
        let scroll_candidate = (distance >= scroll_threshold)
            .then(|| scroll_owner.cloned())
            .flatten();
        let selection_candidate = gesture.selection_candidate().and_then(|candidate| {
            if distance < selection_threshold
                || self.tree.target_status(candidate.owner()) != TargetStatus::Live
                || !self.editing.has_owner(candidate.owner())
            {
                return None;
            }
            let (map, TextDisplayPosition::Document(active)) =
                self.surface_publication.text_map_position_at(
                    event.surface_context(),
                    candidate.owner(),
                    event.position(),
                )?
            else {
                return None;
            };
            let selection = TextSelection::new(candidate.anchor(), active).ok()?;
            self.editing
                .validate_selection(candidate.owner(), selection, &map)
                .ok()?;
            Some(candidate.owner().clone())
        });

        match (scroll_candidate, selection_candidate) {
            (Some(scroll), Some(selection)) => {
                let scroll_rank = gesture
                    .origin_route()
                    .iter()
                    .position(|owner| owner == &scroll)
                    .unwrap_or(0);
                let selection_rank = gesture
                    .origin_route()
                    .iter()
                    .position(|owner| owner == &selection)
                    .unwrap_or(0);
                if selection_rank >= scroll_rank {
                    Some(TouchGestureWinner::new(
                        TouchGestureKind::TextSelection,
                        Some(selection),
                    ))
                } else {
                    Some(TouchGestureWinner::new(
                        TouchGestureKind::Scroll,
                        Some(scroll),
                    ))
                }
            }
            (Some(scroll), None) => Some(TouchGestureWinner::new(
                TouchGestureKind::Scroll,
                Some(scroll),
            )),
            (None, Some(selection)) => Some(TouchGestureWinner::new(
                TouchGestureKind::TextSelection,
                Some(selection),
            )),
            (None, None)
                if (scroll_owner.is_some() && distance < scroll_threshold)
                    || (gesture.selection_candidate().is_some()
                        && distance < selection_threshold) =>
            {
                None
            }
            (None, None) if distance >= scroll_threshold.min(selection_threshold) => Some(
                TouchGestureWinner::new(TouchGestureKind::Move, gesture.origin_target().cloned()),
            ),
            (None, None) => None,
        }
    }

    pub(super) fn touch_routed_target(
        event: &PointerEvent,
        stream: &PointerStreamState,
        physical_target: Option<&MountedNodeId>,
    ) -> Option<MountedNodeId> {
        let Some(gesture) = stream.touch_gesture() else {
            return if event.phase() == PointerPhase::Cancel {
                stream
                    .capture_owner()
                    .or_else(|| stream.pressed_owner())
                    .cloned()
            } else {
                physical_target.cloned()
            };
        };
        if gesture.cancelled() {
            return None;
        }
        if let Some(winner) = gesture.winner() {
            return winner.owner().or_else(|| stream.capture_owner()).cloned();
        }
        stream
            .capture_owner()
            .or_else(|| gesture.origin_target())
            .cloned()
    }

    pub(super) fn start_touch_gesture(
        &self,
        event: &PointerEvent,
        geometry: &PointerGeometry,
    ) -> TouchGestureState {
        let selection_candidate = geometry.physical_target.as_ref().and_then(|owner| {
            if !self.editing.has_owner(owner)
                || self.tree.target_status(owner) != TargetStatus::Live
            {
                return None;
            }
            let (map, display_position) = self.surface_publication.text_map_position_at(
                event.surface_context(),
                owner,
                event.position(),
            )?;
            let TextDisplayPosition::Document(anchor) = display_position else {
                return None;
            };
            self.editing
                .validate_selection(owner, TextSelection::collapsed(anchor), &map)
                .ok()?;
            Some(TouchTextSelectionCandidate::new(owner.clone(), anchor))
        });
        let scroll_candidates = geometry
            .physical_path
            .iter()
            .filter(|owner| {
                self.surface_publication
                    .displayed_scroll_metrics(
                        event.surface_context().hit_test_generation(),
                        event.surface_context().coordinate_revision(),
                        owner,
                    )
                    .is_some_and(|metrics| {
                        metrics.content.width() > metrics.viewport.width()
                            || metrics.content.height() > metrics.viewport.height()
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        TouchGestureState::new(
            event.position(),
            geometry.physical_target.clone(),
            geometry.physical_path.clone(),
            scroll_candidates,
            selection_candidate,
            None,
        )
    }

    pub(super) fn pointer_routed_target(
        phase: PointerPhase,
        stream: &super::PointerStreamState,
        physical_target: Option<&MountedNodeId>,
    ) -> Option<MountedNodeId> {
        let capture = stream.capture_owner().cloned();
        let pressed = stream.pressed_owner().cloned();
        match phase {
            PointerPhase::Move | PointerPhase::Wheel => {
                capture.or_else(|| physical_target.cloned())
            }
            PointerPhase::Up => capture.or(pressed).or_else(|| physical_target.cloned()),
            PointerPhase::Cancel => capture.or(pressed),
            _ => physical_target.cloned(),
        }
    }

    const fn pointer_trace_context(work: &PointerWork) -> TracePointerContext {
        TracePointerContext::event(
            work.event.pointer_id(),
            work.event.device_id(),
            work.event.device_kind(),
            work.event.phase(),
        )
    }

    fn trace_pointer_path(&self, path: &[MountedNodeId]) -> TracePointerPath {
        TracePointerPath::new(
            path.iter()
                .map(|target| self.tree.trace_target(target))
                .collect(),
        )
    }

    fn record_physical_pointer_observation(
        &mut self,
        work: &PointerWork,
        geometry: &PointerGeometry,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        let Some(snapshot) = geometry.snapshot else {
            return parent;
        };
        if !self.trace.is_enabled() {
            return parent;
        }
        let context = TraceContext::pointer_observation(
            TraceEventContext::new(
                TraceEventFamily::Pointer,
                super::pointer_default_is_cancelable(work.event.phase(), work.event.device_kind()),
            ),
            TraceSurfaceContext::accepted(work.event.surface_context(), snapshot),
            Self::pointer_trace_context(work),
            self.trace_pointer_path(&geometry.physical_path),
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerPhysicalTargetResolved,
                work.instant,
                context,
            )
            .with_work_sequence(Some(work.sequence))
            .with_causal_parent(parent)
            .with_target(
                geometry
                    .physical_target
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
            ),
        )
    }

    fn record_pointer_boundary_plan(
        &mut self,
        work: &PointerWork,
        geometry: &PointerGeometry,
        boundary_plan: &PointerBoundaryPlan,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let surface = geometry
            .snapshot
            .map(|snapshot| TraceSurfaceContext::accepted(work.event.surface_context(), snapshot));
        let transition = TraceTargetTransition::new(
            boundary_plan
                .previous_target
                .as_ref()
                .map(|target| self.tree.trace_target(target)),
            boundary_plan
                .current_target
                .as_ref()
                .map(|target| self.tree.trace_target(target)),
        );
        let context = TraceContext::pointer_boundary_plan(
            surface,
            Self::pointer_trace_context(work),
            self.trace_pointer_path(&boundary_plan.previous_path),
            transition,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerBoundaryBundlePlanned {
                    notifications: boundary_plan.notifications.len(),
                },
                work.instant,
                context,
            )
            .with_work_sequence(Some(work.sequence))
            .with_causal_parent(parent)
            .with_target(
                boundary_plan
                    .current_target
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
            ),
        )
    }

    pub(super) fn record_pointer_prelude(
        &mut self,
        work: &PointerWork,
        is_new: bool,
        geometry: &PointerGeometry,
        boundary_plan: &PointerBoundaryPlan,
    ) -> Result<Option<TraceSequence>, ProcessApplicationActionOutcome> {
        if !self.trace.can_replace_reservation(
            work.trace_reservation,
            MandatoryTracePlan::pointer_processing(),
        ) {
            self.trace.release_reservation(work.trace_reservation);
            let cancelled = self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return Err(ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::TraceSequenceExhausted,
                cancelled,
            });
        }
        let pointer_id = work.event.pointer_id();
        let mut parent = self.trace.record_reserved(
            work.trace_reservation,
            TraceRecordKind::PointerIngressValidated {
                pointer_id,
                phase: work.event.phase(),
            },
            work.sequence,
            work.causal_parent,
        );
        parent = self.trace.record(
            TraceRecordKind::PointerStreamResolved {
                pointer_id,
                new_stream: is_new,
            },
            Some(work.sequence),
            parent,
            None,
            None,
            None,
        );
        if !is_new {
            parent = self.trace.record(
                TraceRecordKind::PointerStreamObserved { pointer_id },
                Some(work.sequence),
                parent,
                None,
                None,
                None,
            );
        }
        if let Some(outcome) = geometry.diagnosis {
            parent = self.trace.record(
                TraceRecordKind::PointerContextUnavailable {
                    pointer_id,
                    outcome,
                },
                Some(work.sequence),
                parent,
                None,
                None,
                None,
            );
        }
        if geometry.snapshot.is_none() && is_new {
            unreachable!("cancel requires an existing pointer stream")
        }
        parent = self.record_physical_pointer_observation(work, geometry, parent);
        parent = self.record_pointer_boundary_plan(work, geometry, boundary_plan, parent);
        Ok(parent)
    }
}

fn pointer_button_transition_is_valid(
    event: &PointerEvent,
    existing: Option<&PointerStreamState>,
) -> bool {
    button_transition_is_valid(
        event.phase(),
        event.changed_button(),
        event.buttons(),
        existing.map(PointerStreamState::buttons),
    )
}

fn touch_transition_is_supported(
    event: &PointerEvent,
    existing: Option<&PointerStreamState>,
) -> bool {
    let primary = runenui_core::PointerButton::Primary;
    match event.phase() {
        PointerPhase::Down => {
            existing.is_none()
                && event.changed_button() == Some(primary)
                && event.buttons().iter().eq([primary])
        }
        PointerPhase::Move => {
            existing.is_some()
                && event.changed_button().is_none()
                && event.buttons().iter().eq([primary])
        }
        PointerPhase::Up => {
            existing.is_some()
                && event.changed_button() == Some(primary)
                && event.buttons().is_empty()
        }
        PointerPhase::Cancel => existing.is_some(),
        _ => false,
    }
}

fn button_transition_is_valid(
    phase: PointerPhase,
    changed_button: Option<PointerButton>,
    buttons: &PointerButtons,
    previous_buttons: Option<&PointerButtons>,
) -> bool {
    match phase {
        PointerPhase::Down => {
            let Some(changed) = changed_button else {
                return false;
            };
            if !buttons.contains(changed) {
                return false;
            }
            let Some(previous) = previous_buttons else {
                return true;
            };
            if previous.contains(changed) {
                return false;
            }
            let expected = PointerButtons::new(previous.iter().chain(core::iter::once(changed)));
            &expected == buttons
        }
        PointerPhase::Up => {
            let Some(previous) = previous_buttons else {
                return false;
            };
            let Some(changed) = changed_button else {
                return false;
            };
            if !previous.contains(changed) || buttons.contains(changed) {
                return false;
            }
            let expected = PointerButtons::new(previous.iter().filter(|button| *button != changed));
            &expected == buttons
        }
        PointerPhase::Move | PointerPhase::Wheel => {
            changed_button.is_none() && previous_buttons.is_none_or(|previous| previous == buttons)
        }
        PointerPhase::Cancel => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::button_transition_is_valid;
    use runenui_core::{PointerButton, PointerButtons, PointerPhase};

    fn buttons(values: impl IntoIterator<Item = PointerButton>) -> PointerButtons {
        PointerButtons::new(values)
    }

    #[test]
    fn button_transition_contract_accepts_exact_chord_changes() {
        let empty = PointerButtons::default();
        let primary = buttons([PointerButton::Primary]);
        let secondary = buttons([PointerButton::Secondary]);
        let chord = buttons([PointerButton::Primary, PointerButton::Secondary]);

        assert!(button_transition_is_valid(
            PointerPhase::Down,
            Some(PointerButton::Primary),
            &primary,
            None,
        ));
        assert!(button_transition_is_valid(
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            &chord,
            Some(&primary),
        ));
        assert!(button_transition_is_valid(
            PointerPhase::Move,
            None,
            &chord,
            Some(&chord),
        ));
        assert!(button_transition_is_valid(
            PointerPhase::Wheel,
            None,
            &chord,
            Some(&chord),
        ));
        assert!(button_transition_is_valid(
            PointerPhase::Up,
            Some(PointerButton::Primary),
            &secondary,
            Some(&chord),
        ));
        assert!(button_transition_is_valid(
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            &empty,
            Some(&secondary),
        ));
    }

    #[test]
    fn button_transition_contract_rejects_inexact_changes() {
        let empty = PointerButtons::default();
        let primary = buttons([PointerButton::Primary]);
        let secondary = buttons([PointerButton::Secondary]);
        let chord = buttons([PointerButton::Primary, PointerButton::Secondary]);

        assert!(!button_transition_is_valid(
            PointerPhase::Down,
            None,
            &primary,
            Some(&empty),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Down,
            Some(PointerButton::Primary),
            &primary,
            Some(&primary),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            &primary,
            Some(&primary),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Move,
            None,
            &chord,
            Some(&primary),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Wheel,
            Some(PointerButton::Primary),
            &primary,
            Some(&primary),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Up,
            Some(PointerButton::Primary),
            &chord,
            Some(&chord),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Up,
            Some(PointerButton::Primary),
            &empty,
            Some(&chord),
        ));
        assert!(!button_transition_is_valid(
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            &secondary,
            Some(&primary),
        ));
    }
}
