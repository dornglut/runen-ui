//! Read-only projection of the **existing** live focus/input/presentation authority.

use runenui_core::{__runtime::RuntimeNamespace, HostProtocol, WidgetTextInput};

use crate::{
    InputArbitrationScope, InputObservationError, InputOwnershipRevision, InputOwnershipSnapshot,
    KeyboardInputOwnership, PointerInputOwnership, SurfaceInputOwnership,
};

use super::{
    Runtime,
    pointer::{PointerStreamState, TouchGestureState},
};

/// Derived cached projection for change detection; never a writable focus/input authority.
pub(super) struct InputObservationState {
    scope: InputArbitrationScope,
    revision: InputOwnershipRevision,
    last: Option<InputOwnershipSnapshot>,
    /// Transaction-local external pointer result, never retained across pump boundaries.
    pointer_finality: Option<crate::UiInputFinality>,
    external_pointer_active: bool,
    retired_recorded: bool,
    /// Each committed outside-pump ownership boundary is a separate witness.
    pending_direct_boundaries: u64,
    retirement_cause: Option<crate::InputScopeRetirementReason>,
    #[cfg(test)]
    fail_reservation_after: std::cell::Cell<Option<usize>>,
}

impl InputObservationState {
    pub(super) const fn new(namespace: RuntimeNamespace) -> Self {
        Self {
            scope: InputArbitrationScope::new(namespace),
            revision: InputOwnershipRevision::new(1),
            last: None,
            pointer_finality: None,
            external_pointer_active: false,
            retired_recorded: false,
            pending_direct_boundaries: 0,
            retirement_cause: None,
            #[cfg(test)]
            fail_reservation_after: std::cell::Cell::new(None),
        }
    }
}

/// Owned vectors reserved before a canonical input/readiness/shutdown boundary.
/// Reusing these buffers removes observation allocation from the post-commit path.
struct SnapshotBuffer {
    pointers: Vec<PointerInputOwnership>,
    surfaces: Vec<SurfaceInputOwnership>,
}

impl SnapshotBuffer {
    fn reserved(pointer_bound: usize) -> Result<Self, InputObservationError> {
        let mut pointers = Vec::new();
        pointers
            .try_reserve_exact(pointer_bound)
            .map_err(|_| InputObservationError::Capacity)?;
        let mut surfaces = Vec::new();
        surfaces
            .try_reserve_exact(1)
            .map_err(|_| InputObservationError::Capacity)?;
        Ok(Self { pointers, surfaces })
    }

    fn clone_facts(mut self, source: &InputOwnershipSnapshot) -> InputOwnershipSnapshot {
        debug_assert!(self.pointers.capacity() >= source.pointers.len());
        debug_assert!(self.surfaces.capacity() >= source.surfaces.len());
        self.pointers.extend_from_slice(&source.pointers);
        self.surfaces.extend_from_slice(&source.surfaces);
        InputOwnershipSnapshot {
            scope: source.scope.clone(),
            revision: source.revision,
            status: source.status,
            keyboard: source.keyboard.clone(),
            surfaces: self.surfaces,
            pointers: self.pointers,
        }
    }
}

/// Reserved once per impending canonical checkpoint or work envelope.
/// This is not a retained input-ownership registry or an alternative queue.
pub(crate) struct InputSnapshotReservation {
    ids: Vec<runenui_core::PointerId>,
    projected: SnapshotBuffer,
    retained: SnapshotBuffer,
    result: SnapshotBuffer,
    transition: SnapshotBuffer,
}

impl InputSnapshotReservation {
    fn new(pointer_bound: usize) -> Result<Self, InputObservationError> {
        let mut ids = Vec::new();
        ids.try_reserve_exact(pointer_bound)
            .map_err(|_| InputObservationError::Capacity)?;
        Ok(Self {
            ids,
            projected: SnapshotBuffer::reserved(pointer_bound)?,
            retained: SnapshotBuffer::reserved(pointer_bound)?,
            result: SnapshotBuffer::reserved(pointer_bound)?,
            transition: SnapshotBuffer::reserved(pointer_bound)?,
        })
    }
}

/// Rejected-context release still owns UI's pre-release press/capture lifetime.
/// No hit retarget, default action or new callback is inferred from cleanup.
pub(crate) fn integrity_only_pointer_finality(
    before: Option<&PointerStreamState>,
    staged: Option<&PointerStreamState>,
) -> crate::UiInputFinality {
    let streams = [before, staged];
    let mut reasons = Vec::new();
    if streams
        .iter()
        .flatten()
        .any(|s| s.pressed_owner().is_some())
    {
        reasons.push(crate::UiInputClaimReason::PointerPress);
    }
    if streams
        .iter()
        .flatten()
        .any(|s| s.capture_owner().is_some())
    {
        reasons.push(crate::UiInputClaimReason::PointerCapture);
    }
    if streams
        .iter()
        .flatten()
        .any(|s| s.text_selection().is_some())
    {
        reasons.push(crate::UiInputClaimReason::PointerSelection);
    }
    if streams
        .iter()
        .flatten()
        .any(|s| s.presentation_barrier().is_some())
    {
        reasons.push(crate::UiInputClaimReason::ModalBarrier);
    }
    if streams.iter().flatten().any(|s| {
        s.touch_gesture()
            .and_then(TouchGestureState::winner)
            .is_some()
    }) {
        reasons.push(crate::UiInputClaimReason::TouchGesture);
    }
    crate::UiInputFinality::Committed(crate::UiInputRoutingFacts {
        conflict: if reasons.is_empty() {
            crate::UiInputConflict::Unclaimed
        } else {
            crate::UiInputConflict::ExclusiveUi
        },
        reasons,
        route: crate::UiInputRoute::Unrouted,
        propagation_stopped: false,
        default_prevented: false,
        default_disposition: crate::UiDefaultDisposition::None,
    })
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    /// Records *existing* synchronous public mutations that occur without a
    /// canonical FIFO envelope. This is only a revision witness: the existing
    /// focus/composition/surface authorities still own all actual state.
    /// Checked before a public synchronous operation that would create an
    /// additional input ownership revision without any canonical pump call.
    /// Leave space for a truthful terminal scope invalidation if admission
    /// itself must fail on revision exhaustion.
    pub(crate) fn can_admit_direct_input_ownership_boundary(&self) -> bool {
        self.input_observation.last.is_none()
            || self
                .input_observation
                .pending_direct_boundaries
                .checked_add(1)
                .and_then(|pending| self.input_observation.revision.get().checked_add(pending))
                .and_then(|revision| revision.checked_add(1))
                .is_some()
    }

    pub(crate) const fn note_direct_input_ownership_boundary(&mut self) {
        self.input_observation.pending_direct_boundaries = self
            .input_observation
            .pending_direct_boundaries
            .saturating_add(1);
    }

    pub(crate) const fn note_input_terminal_retirement(
        &mut self,
        reason: crate::RuntimeTerminalReason,
    ) {
        self.input_observation.retirement_cause =
            Some(crate::InputScopeRetirementReason::Terminal(reason));
    }

    pub(crate) const fn note_input_shutdown_retirement(&mut self) {
        if self.input_observation.retirement_cause.is_none() {
            self.input_observation.retirement_cause =
                Some(crate::InputScopeRetirementReason::Shutdown);
        }
    }

    pub(crate) fn input_retirement_record(&mut self) -> Option<crate::InputScopeRetirement> {
        if self.input_observation.retired_recorded {
            return None;
        }
        let reason = match self.input_observation.retirement_cause {
            Some(reason) => reason,
            None => match self.status {
                crate::RuntimeStatus::Closed => crate::InputScopeRetirementReason::Shutdown,
                crate::RuntimeStatus::Terminal(reason) => {
                    crate::InputScopeRetirementReason::Terminal(reason)
                }
                crate::RuntimeStatus::Running => return None,
            },
        };
        self.input_observation.retired_recorded = true;
        Some(crate::InputScopeRetirement {
            scope: self.input_observation.scope.clone(),
            reason,
        })
    }

    /// One atomic shutdown-observation adapter over the *existing* cleanup law.
    /// This is not a second runtime lifecycle; the public clean cutover remains gated
    /// by #429's native host writer and complete capacity/exhaustion conformance.
    ///
    /// The records are reserved before the canonical shutdown begins. Snapshot
    /// allocation and checked-revision exhaustion still require a full reserved
    /// projection before this may become the accepted public return path.
    #[allow(dead_code)] // Draft-only: public shutdown cutover waits for native host #429 ownership.
    pub(crate) fn shutdown_observed(
        &mut self,
    ) -> Result<crate::InputShutdownBatch, InputObservationError> {
        let mut records = Vec::new();
        records
            .try_reserve_exact(2)
            .map_err(|_| InputObservationError::Capacity)?;
        let before = self.input_ownership()?;
        // Explicit close changes the externally observable runtime status.
        // Do not consume the final revision and then panic or return Err
        // after irreversible cleanup. Refuse this close beforehand instead.
        if !matches!(self.status, crate::RuntimeStatus::Closed)
            && before.revision().get() == u64::MAX
        {
            return Err(InputObservationError::RevisionExhausted);
        }
        let reserved = self.reserve_input_observation()?;
        let report = self.shutdown();
        let (after, transition) = self.input_ownership_reserved(reserved);
        if after.revision() != before.revision() {
            records.push(crate::InputArbitrationRecord::OwnershipChanged(
                crate::InputOwnershipTransition {
                    before_revision: before.revision(),
                    after: transition,
                },
            ));
        }
        if let Some(retirement) = self.input_retirement_record() {
            records.push(crate::InputArbitrationRecord::ScopeRetired(retirement));
        }
        Ok(crate::InputShutdownBatch {
            report,
            ordered_records: records,
            final_ownership: after,
        })
    }

    pub(crate) fn begin_external_pointer_input(&mut self) {
        debug_assert!(!self.input_observation.external_pointer_active);
        self.input_observation.pointer_finality = None;
        self.input_observation.external_pointer_active = true;
    }

    pub(crate) fn note_external_pointer_finality(&mut self, finality: crate::UiInputFinality) {
        if self.input_observation.external_pointer_active {
            debug_assert!(self.input_observation.pointer_finality.is_none());
            self.input_observation.pointer_finality = Some(finality);
        }
    }

    pub(crate) fn finish_external_pointer_input(&mut self) -> crate::UiInputFinality {
        self.input_observation.external_pointer_active = false;
        self.input_observation
            .pointer_finality
            .take()
            .unwrap_or(match self.status {
                crate::RuntimeStatus::Terminal(reason) => {
                    crate::UiInputFinality::Aborted(crate::UiInputAbortReason::Terminal(reason))
                }
                crate::RuntimeStatus::Running | crate::RuntimeStatus::Closed => {
                    crate::UiInputFinality::Aborted(crate::UiInputAbortReason::RuntimeIntegrity)
                }
            })
    }

    /// Fallibly reserves every vector needed for the next ownership projection
    /// *before* a canonical mutation boundary.
    #[cfg(test)]
    pub(crate) fn seed_input_revision_for_test(&mut self, revision: u64) {
        let revision = InputOwnershipRevision::new(revision);
        self.input_observation.revision = revision;
        let retained = self
            .input_observation
            .last
            .as_mut()
            .unwrap_or_else(|| unreachable!("seed only after an observed snapshot"));
        retained.revision = revision;
    }

    #[cfg(test)]
    pub(crate) fn inject_input_reservation_failure_after(&self, successful: usize) {
        self.input_observation
            .fail_reservation_after
            .set(Some(successful));
    }

    pub(crate) fn reserve_input_observation(
        &self,
    ) -> Result<InputSnapshotReservation, InputObservationError> {
        #[cfg(test)]
        if let Some(remaining) = self.input_observation.fail_reservation_after.get() {
            if remaining == 0 {
                self.input_observation.fail_reservation_after.set(None);
                return Err(InputObservationError::Capacity);
            }
            self.input_observation
                .fail_reservation_after
                .set(Some(remaining - 1));
        }
        // One canonical envelope can register at most one new pointer stream;
        // checkpoint/derived re-hit only operates on already registered streams.
        // Reserve the current population plus one, clamped to the existing
        // registry limit, rather than allocating for every permitted slot.
        let bound = self
            .pointer_registry
            .len()
            .saturating_add(1)
            .min(self.limits.pointer_streams());
        // Account for every already-committed direct public boundary, even
        // if intermediate ownership facts were later coalesced. Keep one
        // additional checked revision for a terminal invalidation.
        if let Some(previous) = &self.input_observation.last {
            let needed = if matches!(self.status, crate::RuntimeStatus::Running)
                || previous.status != self.status
            {
                self.input_observation.pending_direct_boundaries.max(1)
            } else {
                // A terminal/closed scope with an already observed status is
                // immutable. Re-reading that final snapshot uses no revision.
                self.input_observation.pending_direct_boundaries
            };
            let remaining_for_terminal =
                u64::from(matches!(self.status, crate::RuntimeStatus::Running));
            if self
                .input_observation
                .revision
                .get()
                .checked_add(needed)
                .and_then(|revision| revision.checked_add(remaining_for_terminal))
                .is_none()
            {
                return Err(InputObservationError::RevisionExhausted);
            }
        }
        InputSnapshotReservation::new(bound)
    }

    pub(crate) fn reserve_input_terminal_projection(
        &self,
    ) -> Result<InputSnapshotReservation, InputObservationError> {
        let bound = self
            .pointer_registry
            .len()
            .saturating_add(1)
            .min(self.limits.pointer_streams());
        InputSnapshotReservation::new(bound)
    }

    /// Infallible post-commit ownership publication using only preallocated
    /// vectors. The second returned projection is for an ordered transition
    /// record and is never synthesized by replaying a callback.
    pub(crate) fn input_ownership_reserved(
        &mut self,
        reservation: InputSnapshotReservation,
    ) -> (InputOwnershipSnapshot, InputOwnershipSnapshot) {
        let InputSnapshotReservation {
            mut ids,
            projected,
            retained,
            result,
            transition,
        } = reservation;
        let focused_node = self.focus.focused_node().cloned();
        let text_input_capability = focused_node
            .as_ref()
            .and_then(|target| self.tree.text_input_probe(target).ok())
            .unwrap_or(WidgetTextInput::NONE);
        let keyboard = KeyboardInputOwnership {
            focused_node,
            text_input_capability,
            composition_generation: self.composition.generation().cloned(),
            composition_device_id: self.composition.device_id(),
            space_activation_owner: self.space_ownership.as_ref().map(|x| x.target.clone()),
            space_activation_device_id: self.space_ownership.as_ref().and_then(|x| x.device_id),
        };
        let mut pointers = projected.pointers;
        self.pointer_registry.ordered_pointer_ids_into(&mut ids);
        debug_assert!(pointers.capacity() >= ids.len());
        for id in ids {
            let stream = self
                .pointer_registry
                .stream(id)
                .unwrap_or_else(|| unreachable!("pre-reserved pointer stream is live"));
            pointers.push(PointerInputOwnership {
                pointer_id: id,
                device_id: stream.device_id(),
                surface_id: stream.surface().clone(),
                pressed_owner: stream.pressed_owner().cloned(),
                capture_owner: stream.capture_owner().cloned(),
            });
        }

        // This is the retained logical RunenUI scene, never native Present.
        let mut surfaces = projected.surfaces;
        surfaces.push(SurfaceInputOwnership {
            surface_id: self.surface_publication.surface_id().clone(),
            latest_retained_context: self.surface_publication.current_surface_input_context(),
            modal_blocker: self.surface_publication.current_modal_presentation_root(),
        });
        let mut current = InputOwnershipSnapshot {
            scope: self.input_observation.scope.clone(),
            revision: self.input_observation.revision,
            status: self.status,
            keyboard,
            surfaces,
            pointers,
        };
        let changed = self
            .input_observation
            .last
            .as_ref()
            .is_some_and(|previous| !previous.same_ownership_facts(&current));
        if self.input_observation.last.is_some()
            && (changed || self.input_observation.pending_direct_boundaries != 0)
        {
            // Each committed direct boundary must remain detectable as a
            // revision gap after the host's later synchronous query.
            let steps = self.input_observation.pending_direct_boundaries.max(1);
            let next = self
                .input_observation
                .revision
                .get()
                .checked_add(steps)
                .unwrap_or_else(|| unreachable!("revision was preflighted"));
            current.revision = InputOwnershipRevision::new(next);
        }

        let retained = retained.clone_facts(&current);
        let result = result.clone_facts(&current);
        let transition = transition.clone_facts(&current);
        self.input_observation.revision = current.revision;
        self.input_observation.last = Some(retained);
        self.input_observation.pending_direct_boundaries = 0;
        (result, transition)
    }

    pub(crate) fn input_ownership(
        &mut self,
    ) -> Result<InputOwnershipSnapshot, InputObservationError> {
        let reserved = self.reserve_input_observation()?;
        Ok(self.input_ownership_reserved(reserved).0)
    }
}
