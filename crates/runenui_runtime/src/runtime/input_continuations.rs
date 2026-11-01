//! Connects input-owner lifetimes to the queue's conserved continuation claims.

use super::{Runtime, RuntimeStatus, RuntimeTerminalReason};
use crate::queue::{InputContinuationKey, QueueReservationError, RootQuotaId};
use runenui_core::{HostProtocol, PointerEvent, WorkSequence};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn retire_input_continuation_claims(&mut self) {
        if !matches!(self.status, RuntimeStatus::Running) {
            return;
        }
        let composition = &self.composition;
        let pointers = &self.pointer_registry;
        let space = &self.space_ownership;
        if self
            .queue
            .retire_input_continuations(|key| match key {
                InputContinuationKey::Composition(generation) => {
                    composition.generation() == Some(generation)
                }
                InputContinuationKey::Pointer(pointer, registration, button) => {
                    pointers.stream(*pointer).is_some_and(|stream| {
                        stream.registration_sequence() == *registration
                            && button.map_or_else(
                                || {
                                    stream.buttons().is_empty()
                                        && (!stream.physical_path().is_empty()
                                            || stream.pressed_owner().is_some()
                                            || stream.capture_owner().is_some())
                                },
                                |button| stream.buttons().contains(button),
                            )
                    })
                }
                InputContinuationKey::PendingPointer(_, _)
                | InputContinuationKey::PendingSpace(_) => true,
                InputContinuationKey::Space(sequence) => space
                    .as_ref()
                    .is_some_and(|owner| owner.down_sequence == *sequence),
            })
            .is_err()
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
    }

    pub(crate) fn bind_reached_space_continuation(
        &mut self,
        root: RootQuotaId,
        sequence: WorkSequence,
    ) -> Result<(), QueueReservationError> {
        if self
            .space_ownership
            .as_ref()
            .is_some_and(|owner| owner.down_sequence == sequence)
        {
            self.queue.bind_space_continuation(root, sequence)?;
        }
        Ok(())
    }

    pub(crate) fn bind_reached_pointer_continuations(
        &mut self,
        root: RootQuotaId,
        sequence: WorkSequence,
        pointer: runenui_core::PointerId,
    ) -> Result<(), QueueReservationError> {
        if let Some(stream) = self.pointer_registry.stream(pointer) {
            // An empty, unowned hover stream has no mandatory callback or
            // button retirement. Its registry identity alone does not justify
            // retaining an entire producer frontier.
            if stream.buttons().is_empty()
                && stream.physical_path().is_empty()
                && stream.pressed_owner().is_none()
                && stream.capture_owner().is_none()
            {
                return Ok(());
            }
            self.queue.bind_pointer_continuations(
                root,
                sequence,
                pointer,
                stream.registration_sequence(),
                stream.buttons(),
            )?;
        }
        Ok(())
    }

    pub(in crate::runtime) fn pointer_continuation_key(
        &self,
        event: &PointerEvent,
    ) -> Option<InputContinuationKey> {
        if !matches!(
            event.phase(),
            runenui_core::PointerPhase::Up | runenui_core::PointerPhase::Cancel
        ) {
            return None;
        }
        self.surface_publication
            .validate_surface_identity(event.surface_context())
            .ok()?;
        let current = if self.pointer_registry.stream(event.pointer_id()).is_some() {
            let stream = self
                .pointer_registry
                .validate(
                    event.pointer_id(),
                    event.surface_context().surface_id(),
                    event.device_id(),
                    event.device_kind(),
                )
                .ok()?;
            Some((stream.registration_sequence(), stream.buttons()))
        } else {
            None
        };
        self.queue
            .pointer_continuation_after_predecessors(event, current, |queued| {
                // Unavailable Down/Move/Wheel snapshots reject before changing the
                // stream. Up and Cancel still perform integrity-only retirement.
                matches!(
                    queued.phase(),
                    runenui_core::PointerPhase::Up | runenui_core::PointerPhase::Cancel
                ) || self
                    .surface_publication
                    .resolve_pointer_point(queued.surface_context(), queued.position())
                    .is_ok()
            })
    }
}
