//! Lifetime-owned ingress claims in the canonical FIFO credit ledger.

use core::num::NonZeroU64;
use runenui_core::{CompositionGeneration, PointerButton, PointerId};

use super::{
    CancelledQueue, QueueReservation, QueueReservationError, RootQuotaId, WorkEnvelope, WorkQueue,
    WorkSequence,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum InputContinuationKey {
    Composition(CompositionGeneration),
    PendingSpace(WorkSequence),
    Space(WorkSequence),
    PendingPointer(WorkSequence, usize),
    Pointer(PointerId, NonZeroU64, Option<PointerButton>),
}

impl<Action> WorkQueue<Action> {
    pub(crate) fn has_pointer_continuation(
        &self,
        pointer: PointerId,
        registration: NonZeroU64,
    ) -> bool {
        let matches = |key: &InputContinuationKey| {
            matches!(key,
            InputContinuationKey::Pointer(owner, generation, _) if *owner == pointer && *generation == registration)
        };
        self.input_continuations.keys().any(matches)
            || self
                .claimed_input_continuations
                .values()
                .any(|(key, _)| matches(key))
    }
    pub(crate) fn space_continuation_after_predecessors(
        &self,
        target: &crate::MountedNodeId,
        binding: Option<&super::FocusInputBinding>,
        device: Option<runenui_core::InputDeviceId>,
        current: Option<InputContinuationKey>,
    ) -> Option<InputContinuationKey> {
        use runenui_core::{KeyboardCompositionState, KeyboardPhase, PhysicalKey};
        let mut selected = current;
        let mut retirement_pending = false;
        for envelope in &self.waiting {
            let WorkEnvelope::Input(input) = envelope else {
                continue;
            };
            let super::InputEnvelopePayload::Keyboard(event) = &input.payload else {
                continue;
            };
            if &input.target != target
                || input.focus_binding.as_ref() != binding
                || event.device_id() != device
                || event.physical_key() != &PhysicalKey::Space
            {
                continue;
            }
            match event.phase() {
                KeyboardPhase::Down
                    if !event.is_repeat()
                        && event.composition_state() == KeyboardCompositionState::Inactive
                        && (selected.is_none() || retirement_pending) =>
                {
                    selected = Some(InputContinuationKey::PendingSpace(input.sequence));
                    retirement_pending = false;
                }
                // A queued release does not prove actual retirement: Up can be
                // prevented and either occurrence can be processing-rejected.
                // A later fresh Down has its own admitted generation/frontier.
                KeyboardPhase::Up
                    if !event.is_repeat()
                        || event.composition_state() == KeyboardCompositionState::Active =>
                {
                    retirement_pending = true;
                }
                KeyboardPhase::Cancel => {
                    retirement_pending = true;
                }
                _ => {}
            }
        }
        selected.filter(|key| self.input_continuation_root(key).is_some())
    }

    pub(crate) fn bind_space_continuation(
        &mut self,
        root: RootQuotaId,
        sequence: WorkSequence,
    ) -> Result<(), QueueReservationError> {
        let pending = InputContinuationKey::PendingSpace(sequence);
        for (key, _) in self.claimed_input_continuations.values_mut() {
            if key == &pending {
                *key = InputContinuationKey::Space(sequence);
            }
        }
        if self.input_continuations.contains_key(&pending) {
            self.remove_input_continuation(&pending)?;
            self.retain_input_continuation(root, InputContinuationKey::Space(sequence))?;
        }
        Ok(())
    }
    /// Inspect accepted button transitions without consuming them or predicting
    /// routing/callback outcomes. The pointer authority's same pure validator
    /// controls both this claim selection and actual processing. A terminal
    /// predecessor ends the lineage; a later Down uses its own pending claims.
    #[allow(
        clippy::too_many_lines,
        reason = "one bounded read-only fold over canonical pointer predecessors"
    )]
    pub(crate) fn pointer_continuation_after_predecessors(
        &self,
        event: &runenui_core::PointerEvent,
        current: Option<(NonZeroU64, &runenui_core::PointerButtons)>,
        context_can_process: impl Fn(&runenui_core::PointerEvent) -> bool,
    ) -> Option<InputContinuationKey> {
        use runenui_core::PointerPhase;
        let mut buttons = current.map(|(_, buttons)| buttons.clone());
        let mut claims = std::collections::BTreeMap::new();
        if let Some((registration, current)) = current {
            if current.is_empty() {
                claims.insert(
                    None,
                    InputContinuationKey::Pointer(event.pointer_id(), registration, None),
                );
            } else {
                for button in current.iter() {
                    claims.insert(
                        Some(button),
                        InputContinuationKey::Pointer(
                            event.pointer_id(),
                            registration,
                            Some(button),
                        ),
                    );
                }
            }
        }
        for envelope in &self.waiting {
            let WorkEnvelope::Pointer(super::PointerEnvelope {
                sequence,
                payload: super::PointerEnvelopePayload::Event(queued),
                ..
            }) = envelope
            else {
                continue;
            };
            if queued.pointer_id() != event.pointer_id()
                || queued.device_id() != event.device_id()
                || queued.device_kind() != event.device_kind()
                || queued.surface_context().surface_id() != event.surface_context().surface_id()
                || !context_can_process(queued)
                || (queued.device_kind() == runenui_core::PointerDeviceKind::Touch
                    && !crate::runtime::touch_transition_is_supported(queued, buttons.is_some()))
                || !crate::runtime::button_transition_is_valid(
                    queued.phase(),
                    queued.changed_button(),
                    queued.buttons(),
                    buttons.as_ref(),
                )
            {
                continue;
            }
            match queued.phase() {
                PointerPhase::Up => {
                    claims.remove(&queued.changed_button());
                    buttons = (!queued.buttons().is_empty()).then(|| queued.buttons().clone());
                    if buttons.is_none() {
                        claims.clear();
                    }
                }
                PointerPhase::Cancel => {
                    buttons = None;
                    claims.clear();
                }
                PointerPhase::Down | PointerPhase::Move | PointerPhase::Wheel => {
                    if buttons.is_none() {
                        claims.clear();
                        if queued.buttons().is_empty() {
                            claims.insert(None, InputContinuationKey::PendingPointer(*sequence, 0));
                        } else {
                            for (index, button) in queued.buttons().iter().enumerate() {
                                claims.insert(
                                    Some(button),
                                    InputContinuationKey::PendingPointer(*sequence, index),
                                );
                            }
                        }
                    } else if queued.phase() == PointerPhase::Down {
                        let button = queued.changed_button()?;
                        let index = queued
                            .buttons()
                            .iter()
                            .position(|candidate| candidate == button)?;
                        claims.insert(
                            Some(button),
                            InputContinuationKey::PendingPointer(*sequence, index),
                        );
                    }
                    buttons = Some(queued.buttons().clone());
                }
                _ => {}
            }
        }
        if event.device_kind() == runenui_core::PointerDeviceKind::Touch
            && !crate::runtime::touch_transition_is_supported(event, buttons.is_some())
        {
            return None;
        }
        crate::runtime::button_transition_is_valid(
            event.phase(),
            event.changed_button(),
            event.buttons(),
            buttons.as_ref(),
        )
        .then_some(())?;
        let key = match event.phase() {
            PointerPhase::Up => claims.get(&event.changed_button()).cloned(),
            PointerPhase::Cancel => claims
                .values()
                .find(|key| self.input_continuations.contains_key(*key))
                .cloned(),
            _ => None,
        }?;
        self.input_continuations.contains_key(&key).then_some(key)
    }

    /// The queued source and every required continuation must coexist. These
    /// claims are part of the declared frontier, never additional queue space.
    pub(crate) fn preflight_input_continuations(
        &self,
        reservation: &QueueReservation,
        count: usize,
    ) -> Result<(), QueueReservationError> {
        self.validate_reservation(reservation)?;
        let required = count
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        self.preflight_root_commit(reservation.quota(), required)
    }

    pub(crate) fn retain_input_continuation(
        &mut self,
        root: RootQuotaId,
        key: InputContinuationKey,
    ) -> Result<(), QueueReservationError> {
        if self.input_continuations.contains_key(&key) {
            return Err(QueueReservationError::Stale);
        }
        if self.root_remaining(root)? == 0 {
            return Err(QueueReservationError::Exhausted);
        }
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        quota.input_continuations = quota
            .input_continuations
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        self.input_continuations.insert(key, root);
        Ok(())
    }

    pub(crate) fn input_continuation(
        &self,
        key: &InputContinuationKey,
    ) -> Option<QueueReservation> {
        let quota = self.input_continuation_root(key)?;
        (!self.closed).then(|| QueueReservation {
            quota,
            owner: self.identity.clone(),
            epoch: self.reservation_epoch.clone(),
        })
    }

    /// Capacity ownership survives a queued retirement attempt. This lookup
    /// conveys no semantic permission: the actual owner still validates input.
    pub(crate) fn input_continuation_root(
        &self,
        key: &InputContinuationKey,
    ) -> Option<RootQuotaId> {
        self.input_continuations.get(key).copied().or_else(|| {
            self.claimed_input_continuations
                .values()
                .find_map(|(claimed, root)| (claimed == key).then_some(*root))
        })
    }

    pub(crate) fn preflight_input_continuation(
        &self,
        key: &InputContinuationKey,
    ) -> Result<(), QueueReservationError> {
        let root = self
            .input_continuation_root(key)
            .ok_or(QueueReservationError::Stale)?;
        if self
            .claimed_input_continuations
            .values()
            .any(|(claimed, _)| claimed == key)
        {
            // Overlap consumes spare credits of this same finite frontier and
            // retains one guard for the still-live owner. It may not spend the
            // guard again or make unlimited duplicate input fit.
            let profile = self
                .root_profile(root)
                .ok_or(QueueReservationError::Stale)?;
            // Popping the earlier occurrence returns one slot. Preserve its
            // declared output allowance as well as the rearmed lifetime guard;
            // an overlapping attempt must not turn an admitted callback into
            // a capacity violation.
            let required = profile
                .outputs()
                .max(1)
                .checked_add(usize::from(!self.input_continuations.contains_key(key)))
                .ok_or(QueueReservationError::Capacity)?;
            if self.root_remaining(root)? < required {
                return Err(QueueReservationError::Capacity);
            }
            return self.preflight_root_sequence(root, required);
        }
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let quota = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?;
        let owned = super::future_credit_count(quota)
            .and_then(|future| quota.remaining.checked_add(future))
            .and_then(|owned| owned.checked_sub(quota.input_continuations))
            .and_then(|owned| owned.checked_add(1))
            .ok_or(QueueReservationError::Stale)?;
        let required = self
            .reserved
            .checked_sub(owned)
            .and_then(|other_claims| other_claims.checked_add(1))
            .ok_or(QueueReservationError::Stale)?;
        self.preflight_sequence_capacity(required)
    }

    pub(crate) fn push_input_continuation(
        &mut self,
        key: &InputContinuationKey,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.preflight_input_continuation(key)?;
        let root = self
            .input_continuation_root(key)
            .ok_or(QueueReservationError::Stale)?;
        if self
            .claimed_input_continuations
            .values()
            .any(|(claimed, _)| claimed == key)
        {
            if !self.input_continuations.contains_key(key) {
                self.retain_input_continuation(root, key.clone())?;
            }
            let sequence = self.push_rooted(root, envelope)?;
            self.claimed_input_continuations
                .insert(sequence, (key.clone(), root));
            return Ok(sequence);
        }
        self.remove_input_continuation(key)?;
        match self.push_rooted(root, envelope) {
            Ok(sequence) => {
                self.claimed_input_continuations
                    .insert(sequence, (key.clone(), root));
                Ok(sequence)
            }
            Err(error) => {
                self.retain_input_continuation(root, key.clone())?;
                Err(error)
            }
        }
    }

    // A reached release may be prevented or processing-rejected. Keep its
    // lifetime funded until the actual semantic owner retires; admission alone
    // cannot assert terminality. The popped envelope returns this same credit.
    pub(super) fn rearm_reached_input_continuation(
        &mut self,
        sequence: WorkSequence,
    ) -> Result<(), QueueReservationError> {
        if let Some((key, root)) = self.claimed_input_continuations.get(&sequence).cloned() {
            if !self.input_continuations.contains_key(&key) {
                self.retain_input_continuation(root, key)?;
            }
            self.claimed_input_continuations.remove(&sequence);
        }
        Ok(())
    }

    fn remove_input_continuation(
        &mut self,
        key: &InputContinuationKey,
    ) -> Result<RootQuotaId, QueueReservationError> {
        let root = *self
            .input_continuations
            .get(key)
            .ok_or(QueueReservationError::Stale)?;
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        quota.input_continuations = quota
            .input_continuations
            .checked_sub(1)
            .ok_or(QueueReservationError::Stale)?;
        self.input_continuations.remove(key);
        Ok(root)
    }

    /// Rebind only claims already funded by the reached source. The pointer
    /// registry supplies the exact committed registration and buttons.
    pub(crate) fn bind_pointer_continuations(
        &mut self,
        root: RootQuotaId,
        sequence: WorkSequence,
        pointer: PointerId,
        registration: NonZeroU64,
        buttons: &runenui_core::PointerButtons,
    ) -> Result<(), QueueReservationError> {
        // Validate the complete source's pending button cardinality before
        // rebinding any claim. A stale index must not partially rewrite the
        // accepted pointer lifetime and lose the original owner.
        if !buttons.is_empty() {
            let button_count = buttons.iter().len();
            let invalid = |key: &InputContinuationKey| {
                matches!(
                    key,
                    InputContinuationKey::PendingPointer(source, index)
                        if *source == sequence && *index >= button_count
                )
            };
            if self
                .claimed_input_continuations
                .values()
                .any(|(key, _)| invalid(key))
                || self.input_continuations.keys().any(invalid)
            {
                return Err(QueueReservationError::Stale);
            }
        }
        for (key, _) in self.claimed_input_continuations.values_mut() {
            if let InputContinuationKey::PendingPointer(source, index) = key
                && *source == sequence
            {
                let button = if buttons.is_empty() {
                    None
                } else {
                    Some(
                        buttons
                            .iter()
                            .nth(*index)
                            .ok_or(QueueReservationError::Stale)?,
                    )
                };
                *key = InputContinuationKey::Pointer(pointer, registration, button);
            }
        }
        let pending = self
            .input_continuations
            .keys()
            .filter(|key| {
                matches!(
                    key, InputContinuationKey::PendingPointer(source, _) if *source == sequence
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        for pending in pending {
            let InputContinuationKey::PendingPointer(_, index) = pending else {
                unreachable!();
            };
            let button = if buttons.is_empty() {
                None
            } else {
                Some(
                    buttons
                        .iter()
                        .nth(index)
                        .ok_or(QueueReservationError::Stale)?,
                )
            };
            self.remove_input_continuation(&pending)?;
            let key = InputContinuationKey::Pointer(pointer, registration, button);
            if !self.input_continuations.contains_key(&key)
                && !self
                    .claimed_input_continuations
                    .values()
                    .any(|(claimed, _)| claimed == &key)
            {
                self.retain_input_continuation(root, key)?;
            }
        }
        Ok(())
    }

    pub(crate) fn retire_input_continuations(
        &mut self,
        mut is_live: impl FnMut(&InputContinuationKey) -> bool,
    ) -> Result<(), QueueReservationError> {
        let retired = self
            .input_continuations
            .keys()
            .filter(|key| match key {
                InputContinuationKey::PendingPointer(sequence, _)
                | InputContinuationKey::PendingSpace(sequence) => {
                    !self.has_pending_input_source(*sequence)
                }
                _ => !is_live(key),
            })
            .cloned()
            .collect::<Vec<_>>();
        for key in retired {
            let root = self.remove_input_continuation(&key)?;
            self.close_idle_root(root)?;
        }
        Ok(())
    }

    pub(crate) fn has_pending_input_source(&self, sequence: WorkSequence) -> bool {
        self.waiting
            .iter()
            .any(|envelope| envelope.sequence() == sequence)
    }

    /// Shutdown cancels, rather than processes, this root's queued descendants
    /// before its synchronous live-owner callback. Return their occupied slots
    /// to the same root; unrelated FIFO entries and every live-work credit stay
    /// untouched. No cancelled input receives a fabricated processing receipt.
    pub(crate) fn cancel_root_pending_for_shutdown(
        &mut self,
        root: RootQuotaId,
    ) -> Result<CancelledQueue, QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let mut cancelled = CancelledQueue {
            envelopes: 0,
            command_trace_reservations: 0,
            pointer_trace_reservations: 0,
            input_trace_reservations: 0,
        };
        for envelope in self
            .waiting
            .iter()
            .filter(|work| work.root_quota() == Some(root))
        {
            cancelled.envelopes += 1;
            match envelope {
                WorkEnvelope::SemanticCommand(work) if work.trace_reservation.is_active() => {
                    cancelled.command_trace_reservations += 1;
                }
                WorkEnvelope::ApplicationCommand(work) if work.trace_reservation.is_active() => {
                    cancelled.command_trace_reservations += 1;
                }
                WorkEnvelope::Pointer(work) if work.trace_reservation.is_active() => {
                    cancelled.pointer_trace_reservations += 1;
                }
                WorkEnvelope::Input(work) if work.trace_reservation.is_active() => {
                    cancelled.input_trace_reservations += 1;
                }
                _ => {}
            }
        }
        let quota = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?;
        let remaining = quota
            .remaining
            .checked_add(cancelled.envelopes)
            .ok_or(QueueReservationError::Exhausted)?;
        let reserved = self
            .reserved
            .checked_add(cancelled.envelopes)
            .ok_or(QueueReservationError::Exhausted)?;
        if quota.pending_envelopes != cancelled.envelopes
            || super::future_credit_count(quota)
                .and_then(|future| remaining.checked_add(future))
                .is_none_or(|occupied| occupied > quota.limit)
        {
            return Err(QueueReservationError::Stale);
        }
        self.waiting.retain(|work| work.root_quota() != Some(root));
        self.claimed_input_continuations
            .retain(|_, (_, owner)| *owner != root);
        let quota = self
            .root_quotas
            .get_mut(&root)
            .unwrap_or_else(|| unreachable!("shutdown root was preflighted"));
        quota.pending_envelopes = 0;
        quota.remaining = remaining;
        quota.timer_callback_actions.clear();
        self.reserved = reserved;
        Ok(cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::{ApplicationActionEnvelope, ApplicationActionOrigin};

    fn envelope(sequence: WorkSequence, root_quota: RootQuotaId) -> WorkEnvelope<()> {
        WorkEnvelope::ApplicationAction(ApplicationActionEnvelope {
            sequence,
            root_quota,
            action: (),
            producer_budget: runenui_core::ProducerBudget::new(1, 0),
            causal_parent: None,
            target: None,
            origin: ApplicationActionOrigin::Ordinary,
        })
    }

    #[test]
    fn invalid_later_pointer_button_index_preserves_earlier_claim_and_quota() {
        let mut queue = WorkQueue::<()>::new(3);
        let reservation = queue.reserve(3).expect("finite root fits");
        let source = queue
            .push_reserved_with_remainder(&reservation, envelope)
            .expect("source committed");
        let claimed = InputContinuationKey::PendingPointer(source, 0);
        let later = InputContinuationKey::PendingPointer(source, 2);
        queue
            .claimed_input_continuations
            .insert(source, (claimed.clone(), reservation.quota()));
        queue
            .retain_input_continuation(reservation.quota(), later.clone())
            .expect("pending later button guard fits");

        let pointer = PointerId::new(1).expect("valid pointer");
        let registration = NonZeroU64::new(1).expect("valid registration");
        let buttons = runenui_core::PointerButtons::new([runenui_core::PointerButton::Primary]);
        let before_remaining = queue.root_quotas[&reservation.quota()].remaining;
        assert_eq!(
            queue.bind_pointer_continuations(
                reservation.quota(),
                source,
                pointer,
                registration,
                &buttons,
            ),
            Err(QueueReservationError::Stale),
        );
        assert_eq!(
            queue.claimed_input_continuations.get(&source),
            Some(&(claimed, reservation.quota())),
        );
        assert!(queue.input_continuations.contains_key(&later));
        assert_eq!(queue.root_quotas[&reservation.quota()].input_continuations, 1);
        assert_eq!(queue.root_quotas[&reservation.quota()].remaining, before_remaining);
    }

    #[test]
    fn failed_rearm_preserves_the_original_reached_claim() {
        let mut queue = WorkQueue::<()>::new(1);
        let reservation = queue.reserve(1).expect("one root credit");
        let source = queue
            .push_reserved_with_remainder(&reservation, envelope)
            .expect("source committed");
        let claimed = InputContinuationKey::PendingSpace(source);
        queue
            .claimed_input_continuations
            .insert(source, (claimed.clone(), reservation.quota()));
        // Deliberately inject an inconsistent reserved guard count: a failed
        // rearm must still preserve the reached source's claim.
        queue
            .root_quotas
            .get_mut(&reservation.quota())
            .expect("live root")
            .input_continuations = 1;
        assert_eq!(
            queue.rearm_reached_input_continuation(source),
            Err(QueueReservationError::Exhausted),
        );
        assert_eq!(
            queue.claimed_input_continuations.get(&source),
            Some(&(claimed, reservation.quota())),
        );
    }

    #[test]
    fn overlap_cannot_displace_the_earlier_callbacks_declared_outputs() {
        for limit in [4, 5] {
            let mut queue = WorkQueue::new(limit);
            let root = queue
                .reserve_for_producer(runenui_core::ProducerBudget::new(limit, 3))
                .unwrap_or_else(|_| unreachable!());
            let source = queue
                .push_reserved_with_remainder(&root, envelope)
                .unwrap_or_else(|_| unreachable!());
            let key = InputContinuationKey::Space(source);
            queue
                .retain_input_continuation(root.quota(), key.clone())
                .unwrap_or_else(|_| unreachable!());
            queue.pop().unwrap_or_else(|| unreachable!());
            queue
                .finish_root_envelope(root.quota())
                .unwrap_or_else(|_| unreachable!());
            queue
                .push_input_continuation(&key, envelope)
                .unwrap_or_else(|_| unreachable!());
            let next = queue.next_sequence();
            let overlap = queue.push_input_continuation(&key, envelope);
            if limit == 4 {
                assert_eq!(overlap, Err(QueueReservationError::Capacity));
                assert_eq!(queue.next_sequence(), next);
            } else {
                assert!(overlap.is_ok());
            }
            queue.pop().unwrap_or_else(|| unreachable!());
            assert!(queue.preflight_root_commit(root.quota(), 3).is_ok());
        }
    }

    #[test]
    fn overlapping_retirement_keeps_one_guard_and_returns_every_credit() {
        let mut queue = WorkQueue::new(3);
        let root = queue.reserve(3).unwrap_or_else(|_| unreachable!());
        let source = queue
            .push_reserved_with_remainder(&root, envelope)
            .unwrap_or_else(|_| unreachable!());
        let key = InputContinuationKey::Space(source);
        queue
            .retain_input_continuation(root.quota(), key.clone())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        let up = queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        let cancel = queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        assert!(up < cancel);
        let next = queue.next_sequence();
        assert_eq!(
            queue.preflight_input_continuation(&key),
            Err(QueueReservationError::Capacity)
        );
        assert_eq!(queue.next_sequence(), next);
        assert_eq!(queue.root_quotas[&root.quota()].input_continuations, 1);
        for expected in [up, cancel] {
            assert_eq!(
                queue.pop().unwrap_or_else(|| unreachable!()).sequence(),
                expected
            );
            assert_eq!(queue.root_quotas[&root.quota()].input_continuations, 1);
            queue
                .finish_root_envelope(root.quota())
                .unwrap_or_else(|_| unreachable!());
        }
        queue
            .retire_input_continuations(|_| false)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(queue.reserved, 0);
        assert!(queue.root_quotas.is_empty());
        assert!(queue.claimed_input_continuations.is_empty());
        assert!(queue.input_continuations.is_empty());
        assert!(queue.reserve(3).is_ok());
    }

    #[test]
    fn overlapping_retirement_preserves_the_guards_final_sequence() {
        let mut queue = WorkQueue::new(3);
        queue.next_sequence = NonZeroU64::new(u64::MAX - 3);
        let root = queue.reserve(3).unwrap_or_else(|_| unreachable!());
        let source = queue
            .push_reserved_with_remainder(&root, envelope)
            .unwrap_or_else(|_| unreachable!());
        let key = InputContinuationKey::Space(source);
        queue
            .retain_input_continuation(root.quota(), key.clone())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        assert_eq!(
            queue.push_rooted(root.quota(), envelope),
            Err(QueueReservationError::SequenceExhausted)
        );
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        let final_attempt = queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(final_attempt.get(), u64::MAX);
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        queue
            .retire_input_continuations(|_| false)
            .unwrap_or_else(|_| unreachable!());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(queue.reserved, 0);
    }

    #[test]
    fn shutdown_returns_cancelled_slots_only_to_their_original_root() {
        let mut queue = WorkQueue::new(4);
        let root = queue.reserve(3).unwrap_or_else(|_| unreachable!());
        let source = queue
            .push_reserved_with_remainder(&root, envelope)
            .unwrap_or_else(|_| unreachable!());
        let key = InputContinuationKey::Space(source);
        queue
            .retain_input_continuation(root.quota(), key.clone())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        let other = queue.reserve(1).unwrap_or_else(|_| unreachable!());
        let older = queue
            .push_reserved_with_remainder(&other, envelope)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            queue
                .cancel_root_pending_for_shutdown(root.quota())
                .unwrap_or_else(|_| unreachable!())
                .envelopes,
            2
        );
        assert!(
            queue.is_full(),
            "discarding work does not give this root's credits to another producer"
        );
        assert!(queue.claimed_input_continuations.is_empty());
        queue
            .preflight_root_commit(root.quota(), 2)
            .unwrap_or_else(|_| unreachable!());
        let cleanup = queue
            .push_rooted(root.quota(), envelope)
            .unwrap_or_else(|_| unreachable!());
        assert!(older < cleanup);
        for expected in [older, cleanup] {
            let reached = queue.pop().unwrap_or_else(|| unreachable!());
            assert_eq!(reached.sequence(), expected);
            queue
                .finish_root_envelope(reached.root_quota().unwrap_or_else(|| unreachable!()))
                .unwrap_or_else(|_| unreachable!());
        }
        queue
            .retire_input_continuations(|_| false)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(queue.reserved, 0);
        assert!(queue.root_quotas.is_empty());
    }

    #[test]
    fn descendants_cannot_spend_the_last_continuation_slot_or_overtake_it() {
        let mut queue = WorkQueue::new(4);
        let root = queue.reserve(3).unwrap_or_else(|_| unreachable!());
        let source = queue
            .push_reserved_with_remainder(&root, envelope)
            .unwrap_or_else(|_| unreachable!());
        let key = InputContinuationKey::PendingPointer(source, 0);
        queue
            .retain_input_continuation(root.quota(), key.clone())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        let a = queue
            .push_rooted(root.quota(), envelope)
            .unwrap_or_else(|_| unreachable!());
        let b = queue
            .push_rooted(root.quota(), envelope)
            .unwrap_or_else(|_| unreachable!());
        assert!(queue.push_rooted(root.quota(), envelope).is_err());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        let other = queue.reserve(1).unwrap_or_else(|_| unreachable!());
        let c = queue
            .push_reserved_with_remainder(&other, envelope)
            .unwrap_or_else(|_| unreachable!());
        assert!(queue.is_full());
        let terminal = queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        assert!(a < b && b < c && c < terminal);
        for expected in [a, b, c, terminal] {
            let reached = queue.pop().unwrap_or_else(|| unreachable!());
            assert_eq!(reached.sequence(), expected);
            queue
                .retire_input_continuations(|_| false)
                .unwrap_or_else(|_| unreachable!());
            queue
                .finish_root_envelope(reached.root_quota().unwrap_or_else(|| unreachable!()))
                .unwrap_or_else(|_| unreachable!());
        }
        assert_eq!(queue.reserved, 0);
        assert!(queue.root_quotas.is_empty());
        assert!(queue.input_continuations.is_empty());
    }

    #[test]
    fn unary_descendants_cannot_spend_the_continuations_final_sequence() {
        let mut queue = WorkQueue::new(2);
        queue.next_sequence = NonZeroU64::new(u64::MAX - 2);
        let root = queue.reserve(2).unwrap_or_else(|_| unreachable!());
        let source = queue
            .push_reserved_with_remainder(&root, envelope)
            .unwrap_or_else(|_| unreachable!());
        let key = InputContinuationKey::PendingPointer(source, 0);
        queue
            .retain_input_continuation(root.quota(), key.clone())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        queue
            .push_rooted(root.quota(), envelope)
            .unwrap_or_else(|_| unreachable!());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        queue.pop().unwrap_or_else(|| unreachable!());
        assert_eq!(
            queue.push_rooted(root.quota(), envelope),
            Err(QueueReservationError::SequenceExhausted)
        );
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        let terminal = queue
            .push_input_continuation(&key, envelope)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(terminal.get(), u64::MAX);
        queue.pop().unwrap_or_else(|| unreachable!());
        queue
            .retire_input_continuations(|_| false)
            .unwrap_or_else(|_| unreachable!());
        queue
            .finish_root_envelope(root.quota())
            .unwrap_or_else(|_| unreachable!());
        assert!(queue.root_quotas.is_empty());
    }
}
