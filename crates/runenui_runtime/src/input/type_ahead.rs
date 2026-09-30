use runenui_core::{
    FocusGroupTypeAhead, HostProtocol, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LogicalKey, MonotonicInstant, MountedNodeId,
};

use crate::{
    TraceRecordKind, TraceRoutedIntegrityFailure,
    focus::{
        FocusGroupTypeAheadContext, focus_group_type_ahead_context,
        select_focus_group_type_ahead_match,
    },
    runtime::{RoutedTransaction, Runtime},
};

const TYPE_AHEAD_MAX_BYTES: usize = 64;
const TYPE_AHEAD_MAX_SCALARS: usize = 32;

/// Private transient type-ahead state for one exact active focus group.
///
/// The literal buffer deliberately has no public inspection or Debug surface.
pub(crate) struct FocusGroupTypeAheadState {
    group: Option<MountedNodeId>,
    policy: Option<FocusGroupTypeAhead>,
    buffer: String,
    scalar_count: usize,
    last_input: Option<MonotonicInstant>,
}

impl FocusGroupTypeAheadState {
    pub(crate) const fn new() -> Self {
        Self {
            group: None,
            policy: None,
            buffer: String::new(),
            scalar_count: 0,
            last_input: None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.group = None;
        self.policy = None;
        self.buffer.clear();
        self.scalar_count = 0;
        self.last_input = None;
    }

    fn session_is_current(
        &self,
        context: &FocusGroupTypeAheadContext,
        instant: MonotonicInstant,
    ) -> bool {
        if self.group.as_ref() != Some(&context.group) || self.policy != Some(context.policy) {
            return false;
        }
        let Some(last_input) = self.last_input else {
            return false;
        };
        let Some(elapsed) = instant.as_nanos().checked_sub(last_input.as_nanos()) else {
            return false;
        };
        u128::from(elapsed) <= context.policy.timeout().as_nanos()
    }

    fn apply(&mut self, update: FocusGroupTypeAheadUpdate) {
        self.group = Some(update.group);
        self.policy = Some(update.policy);
        self.buffer = update.buffer;
        self.scalar_count = update.scalar_count;
        self.last_input = Some(update.last_input);
    }
}

pub(crate) struct FocusGroupTypeAheadUpdate {
    group: MountedNodeId,
    policy: FocusGroupTypeAhead,
    buffer: String,
    scalar_count: usize,
    last_input: MonotonicInstant,
}

impl FocusGroupTypeAheadUpdate {
    fn new(
        context: &FocusGroupTypeAheadContext,
        buffer: String,
        scalar_count: usize,
        last_input: MonotonicInstant,
    ) -> Self {
        Self {
            group: context.group.clone(),
            policy: context.policy,
            buffer,
            scalar_count,
            last_input,
        }
    }
}

fn lowercase_type_ahead_fragment(text: &str) -> Option<(String, usize)> {
    let mut output = String::new();
    let mut scalars = 0usize;
    for scalar in text.chars().flat_map(char::to_lowercase) {
        let next_scalars = scalars.checked_add(1)?;
        let next_bytes = output.len().checked_add(scalar.len_utf8())?;
        if next_scalars > TYPE_AHEAD_MAX_SCALARS || next_bytes > TYPE_AHEAD_MAX_BYTES {
            return None;
        }
        output.push(scalar);
        scalars = next_scalars;
    }
    (!output.is_empty()).then_some((output, scalars))
}


impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn apply_focus_group_type_ahead_update(
        &mut self,
        update: FocusGroupTypeAheadUpdate,
    ) {
        self.focus_group_type_ahead.apply(update);
    }

    pub(crate) fn reconcile_focus_group_type_ahead_state(&mut self) {
        let Some(focused) = self.focus.focused_node().cloned() else {
            self.focus_group_type_ahead.clear();
            return;
        };
        let Some(context) = focus_group_type_ahead_context(&self.tree, &self.focus, &focused) else {
            self.focus_group_type_ahead.clear();
            return;
        };
        if self.focus_group_type_ahead.group.as_ref() != Some(&context.group)
            || self.focus_group_type_ahead.policy != Some(context.policy)
        {
            self.focus_group_type_ahead.clear();
        }
    }

    fn keyboard_type_ahead_context(
        &self,
        event: &KeyboardEvent,
        target: &MountedNodeId,
    ) -> Option<FocusGroupTypeAheadContext> {
        if event.phase() != KeyboardPhase::Down
            || event.composition_state() != KeyboardCompositionState::Inactive
        {
            return None;
        }
        let LogicalKey::Character(character) = event.logical_key() else {
            return None;
        };
        if character.is_empty() {
            return None;
        }
        let modifiers = event.modifiers();
        if modifiers.control() || modifiers.alt() || modifiers.meta() {
            return None;
        }
        focus_group_type_ahead_context(&self.tree, &self.focus, target)
    }

    fn collect_focus_group_type_ahead_default(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        event: &KeyboardEvent,
        target: &MountedNodeId,
    ) -> Result<bool, crate::TraceRoutedIntegrityFailure> {
        let Some(context) = self.keyboard_type_ahead_context(event, target) else {
            return Ok(false);
        };
        let LogicalKey::Character(character) = event.logical_key() else {
            return Ok(false);
        };
        let Some((fragment, fragment_scalars)) = lowercase_type_ahead_fragment(character) else {
            self.record_focus_group_type_ahead_evaluation(
                transaction,
                self.focus_group_type_ahead.scalar_count,
                false,
                true,
                &context.group,
            );
            return Ok(true);
        };

        let session_current = self
            .focus_group_type_ahead
            .session_is_current(&context, transaction.instant);
        let (base, base_scalars) = if session_current {
            (
                self.focus_group_type_ahead.buffer.clone(),
                self.focus_group_type_ahead.scalar_count,
            )
        } else {
            (String::new(), 0)
        };
        let repeated_single = base_scalars == 1
            && fragment_scalars == 1
            && base == fragment;
        let (query, query_scalars) = if repeated_single || base.is_empty() {
            (fragment.clone(), fragment_scalars)
        } else {
            let Some(query_scalars) = base_scalars.checked_add(fragment_scalars) else {
                self.record_focus_group_type_ahead_evaluation(
                    transaction,
                    self.focus_group_type_ahead.scalar_count,
                    false,
                    true,
                    &context.group,
                );
                return Ok(true);
            };
            let Some(query_bytes) = base.len().checked_add(fragment.len()) else {
                self.record_focus_group_type_ahead_evaluation(
                    transaction,
                    self.focus_group_type_ahead.scalar_count,
                    false,
                    true,
                    &context.group,
                );
                return Ok(true);
            };
            if query_scalars > TYPE_AHEAD_MAX_SCALARS || query_bytes > TYPE_AHEAD_MAX_BYTES {
                self.record_focus_group_type_ahead_evaluation(
                    transaction,
                    self.focus_group_type_ahead.scalar_count,
                    false,
                    true,
                    &context.group,
                );
                return Ok(true);
            }
            let mut combined = base;
            combined.push_str(&fragment);
            (combined, query_scalars)
        };

        let mut retained_query = query;
        let mut retained_scalars = query_scalars;
        let mut matched = select_focus_group_type_ahead_match(
            &mut self.tree,
            &self.focus,
            &context.group,
            &retained_query,
        );
        if matched.is_none()
            && session_current
            && retained_query != fragment
        {
            retained_query = fragment;
            retained_scalars = fragment_scalars;
            matched = select_focus_group_type_ahead_match(
                &mut self.tree,
                &self.focus,
                &context.group,
                &retained_query,
            );
        }

        transaction.focus_group_type_ahead_update = Some(FocusGroupTypeAheadUpdate::new(
            &context,
            retained_query,
            retained_scalars,
            transaction.instant,
        ));
        let observation_target = matched.as_ref().unwrap_or(&context.group);
        self.record_focus_group_type_ahead_evaluation(
            transaction,
            retained_scalars,
            matched.is_some(),
            false,
            observation_target,
        );
        if let Some(destination) = matched {
            self.apply_focus_group_destination(transaction, destination, context.activation)?;
        }
        Ok(true)
    }

    fn record_focus_group_type_ahead_evaluation(
        &mut self,
        transaction: &mut crate::runtime::RoutedTransaction<Action>,
        buffer_scalars: usize,
        matched: bool,
        capacity_rejected: bool,
        observation_target: &MountedNodeId,
    ) {
        transaction.parent = self.trace.record_event(
            TraceRecordKind::FocusGroupTypeAheadEvaluated {
                buffer_scalars,
                matched,
                capacity_rejected,
            },
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(observation_target)),
            transaction.instant,
            &transaction.target,
            Some(observation_target),
            transaction.origin,
        );
    }

}
