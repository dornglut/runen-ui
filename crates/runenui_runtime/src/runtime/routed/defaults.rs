use runenui_core::{
    __runtime::{FrameworkServiceBinding, FrameworkServiceEffect, MountedEffect},
    ClipboardWritePurpose, FocusDirection, FrameworkServiceRequest, HostProtocol, LogicalDelta,
    OverflowPolicy, SemanticActionData, SemanticCommand, TextSensitivity,
};

use super::{
    super::{CollectedRoutedOutput, Runtime, ingress::trace_semantic_action_rejection},
    transaction::RoutedTransaction,
};
use crate::{TraceRecordKind, TraceRoutedIntegrityFailure, TraceSemanticActionRejection};

const MAX_CLIPBOARD_BYTES: usize = 1_048_576;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    #[allow(clippy::too_many_lines)] // Keeps semantic default suppression, rejection, and dispatch ordered.
    pub(super) fn apply_semantic_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        self.commit_pending_modality(transaction);
        if transaction.default_prevented {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::SemanticDefaultSuppressed { command },
                transaction.sequence,
                transaction.parent,
                Some(transaction.target_trace.clone()),
                transaction.instant,
                &transaction.target,
                Some(&transaction.target),
                transaction.origin,
            );
            return Ok(());
        }
        if let Some(outcome) = self.semantic_default_target_rejection(transaction, command) {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::SemanticDefaultTargetInvalidated { command, outcome },
                transaction.sequence,
                transaction.parent,
                Some(transaction.target_trace.clone()),
                transaction.instant,
                &transaction.target,
                Some(&transaction.target),
                transaction.origin,
            );
            return Ok(());
        }
        if matches!(
            command,
            SemanticCommand::Copy | SemanticCommand::Cut | SemanticCommand::Paste
        ) {
            if !self.request_clipboard_default(transaction, command) {
                transaction.parent = self.trace.record_event(
                    TraceRecordKind::EditingDefaultUnavailable { command },
                    transaction.sequence,
                    transaction.parent,
                    Some(transaction.target_trace.clone()),
                    transaction.instant,
                    &transaction.target,
                    Some(&transaction.target),
                    transaction.origin,
                );
            }
            return Ok(());
        }
        if let SemanticCommand::LogicalScroll(scroll) = command {
            self.record_semantic_default_applied(transaction, command);
            self.apply_logical_scroll_default(
                transaction,
                scroll.delta(),
                scroll.hit_test_generation(),
                scroll.coordinate_revision(),
            );
            return Ok(());
        }
        if let SemanticCommand::LogicalFocusScroll(direction) = command {
            self.record_semantic_default_applied(transaction, command);
            self.apply_logical_focus_scroll_default(transaction, direction);
            return Ok(());
        }
        if command == SemanticCommand::ScrollIntoView {
            self.record_semantic_default_applied(transaction, command);
            self.apply_scroll_into_view_default(transaction);
            return Ok(());
        }
        self.record_semantic_default_applied(transaction, command);
        if super::is_editing_command(command) {
            return self.apply_editing_default(transaction, command);
        }
        if command != SemanticCommand::Activate {
            return self.apply_focus_default(transaction, command);
        }
        transaction.failure_current_target = Some(transaction.target.clone());
        #[cfg(feature = "internal-test-seams")]
        if self.test_seams.routed_semantic_default_failure {
            return Err(TraceRoutedIntegrityFailure::SemanticDefaultFailure);
        }
        self.invoke_activation_default(transaction)
    }

    fn record_semantic_default_applied(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) {
        transaction.parent = self.trace.record_event(
            TraceRecordKind::SemanticDefaultApplied { command },
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace.clone()),
            transaction.instant,
            &transaction.target,
            Some(&transaction.target),
            transaction.origin,
        );
    }

    pub(in crate::runtime) fn apply_logical_scroll_default(
        &self,
        transaction: &mut RoutedTransaction<Action>,
        offered: LogicalDelta,
        hit_test_generation: u64,
        coordinate_revision: u64,
    ) {
        let mut remainder = offered;
        let mut evaluation_order = 0;
        for owner in transaction.route.iter().rev() {
            let Some(metrics) = self.surface_publication.displayed_scroll_metrics(
                hit_test_generation,
                coordinate_revision,
                owner,
            ) else {
                continue;
            };
            let Some(node) = self.tree.node(owner) else {
                continue;
            };
            let horizontal = metrics.overflow.horizontal() == OverflowPolicy::Scroll;
            let vertical = metrics.overflow.vertical() == OverflowPolicy::Scroll;
            let maximum = (
                if horizontal {
                    (metrics.content.width() - metrics.viewport.width()).max(0.0)
                } else {
                    0.0
                },
                if vertical {
                    (metrics.content.height() - metrics.viewport.height()).max(0.0)
                } else {
                    0.0
                },
            );
            let before = node.interaction.scroll_offset;
            let after = (
                if horizontal {
                    bounded_scroll_offset(before.0, remainder.x(), maximum.0)
                } else {
                    0.0
                },
                if vertical {
                    bounded_scroll_offset(before.1, remainder.y(), maximum.1)
                } else {
                    0.0
                },
            );
            let consumed = LogicalDelta::new(after.0 - before.0, after.1 - before.1)
                .unwrap_or_else(|_| unreachable!("bounded scroll offsets remain finite"));
            let next_remainder =
                LogicalDelta::new(remainder.x() - consumed.x(), remainder.y() - consumed.y())
                    .unwrap_or_else(|_| {
                        unreachable!("clamped scroll consumption cannot exceed offered delta")
                    });
            if after != before {
                transaction
                    .scroll_updates
                    .push(super::transaction::ScrollOffsetUpdate {
                        owner: owner.clone(),
                        offset: after,
                    });
            }
            transaction
                .scroll_consumptions
                .push(super::transaction::ScrollOwnerConsumption {
                    owner: owner.clone(),
                    evaluation_order,
                    offered: remainder,
                    consumed,
                    remainder: next_remainder,
                    offset: LogicalDelta::new(after.0, after.1)
                        .unwrap_or_else(|_| unreachable!("bounded scroll offsets remain finite")),
                    maximum: LogicalDelta::new(maximum.0, maximum.1)
                        .unwrap_or_else(|_| unreachable!("scroll ranges remain finite")),
                });
            remainder = next_remainder;
            evaluation_order += 1;
        }
        transaction.scroll_chain_remainder = Some(remainder);
    }

    fn apply_logical_focus_scroll_default(
        &self,
        transaction: &mut RoutedTransaction<Action>,
        direction: FocusDirection,
    ) {
        let mut remainder = LogicalDelta::ZERO;
        for owner in transaction.route.iter().rev() {
            let Some(metrics) = self.surface_publication.current_scroll_metrics(owner) else {
                continue;
            };
            let Some(node) = self.tree.node(owner) else {
                continue;
            };
            let (horizontal, sign) = match direction {
                FocusDirection::Left => (true, -1.0),
                FocusDirection::Right => (true, 1.0),
                FocusDirection::Up => (false, -1.0),
                FocusDirection::Down => (false, 1.0),
                _ => continue,
            };
            let policy = if horizontal {
                metrics.overflow.horizontal()
            } else {
                metrics.overflow.vertical()
            };
            if policy != OverflowPolicy::Scroll {
                continue;
            }
            let (viewport, before, maximum) = if horizontal {
                (
                    metrics.viewport.width(),
                    node.interaction.scroll_offset.0,
                    (metrics.content.width() - metrics.viewport.width()).max(0.0),
                )
            } else {
                (
                    metrics.viewport.height(),
                    node.interaction.scroll_offset.1,
                    (metrics.content.height() - metrics.viewport.height()).max(0.0),
                )
            };
            if maximum == 0.0 {
                continue;
            }
            let offered_scalar = sign * viewport;
            let after = bounded_scroll_offset(before, offered_scalar, maximum);
            if after.to_bits() == before.to_bits() {
                continue;
            }
            let (offset, offered) = if horizontal {
                (
                    (after, node.interaction.scroll_offset.1),
                    LogicalDelta::new(offered_scalar, 0.0)
                        .unwrap_or_else(|_| unreachable!("viewport scroll delta is finite")),
                )
            } else {
                (
                    (node.interaction.scroll_offset.0, after),
                    LogicalDelta::new(0.0, offered_scalar)
                        .unwrap_or_else(|_| unreachable!("viewport scroll delta is finite")),
                )
            };
            transaction
                .scroll_updates
                .push(super::transaction::ScrollOffsetUpdate {
                    owner: owner.clone(),
                    offset,
                });
            let consumed = if horizontal {
                LogicalDelta::new(after - before, 0.0)
                    .unwrap_or_else(|_| unreachable!("clamped focus scroll remains finite"))
            } else {
                LogicalDelta::new(0.0, after - before)
                    .unwrap_or_else(|_| unreachable!("clamped focus scroll remains finite"))
            };
            remainder = LogicalDelta::new(offered.x() - consumed.x(), offered.y() - consumed.y())
                .unwrap_or_else(|_| unreachable!("clamped focus remainder remains finite"));
            let offset = LogicalDelta::new(offset.0, offset.1)
                .unwrap_or_else(|_| unreachable!("bounded scroll offset remains finite"));
            let maximum = if horizontal {
                LogicalDelta::new(maximum, 0.0)
            } else {
                LogicalDelta::new(0.0, maximum)
            }
            .unwrap_or_else(|_| unreachable!("bounded focus scroll range remains finite"));
            transaction
                .scroll_consumptions
                .push(super::transaction::ScrollOwnerConsumption {
                    owner: owner.clone(),
                    evaluation_order: 0,
                    offered,
                    consumed,
                    remainder,
                    offset,
                    maximum,
                });
            break;
        }
        transaction.scroll_chain_remainder = Some(remainder);
    }

    fn apply_scroll_into_view_default(&self, transaction: &mut RoutedTransaction<Action>) {
        let target = &transaction.target;
        for owner in transaction.route.iter().rev() {
            let Some(metrics) = self.surface_publication.current_scroll_metrics(owner) else {
                continue;
            };
            let Some((bounds, viewport)) = self
                .surface_publication
                .scroll_target_geometry(target, owner)
            else {
                continue;
            };
            let Some(node) = self.tree.node(owner) else {
                continue;
            };
            let before = node.interaction.scroll_offset;
            let maximum = (
                (metrics.content.width() - metrics.viewport.width()).max(0.0),
                (metrics.content.height() - metrics.viewport.height()).max(0.0),
            );
            let after = (
                if metrics.overflow.horizontal() == OverflowPolicy::Scroll {
                    reveal_axis(
                        bounds.x(),
                        bounds.max_x(),
                        before.0,
                        viewport.width(),
                        maximum.0,
                    )
                } else {
                    before.0
                },
                if metrics.overflow.vertical() == OverflowPolicy::Scroll {
                    reveal_axis(
                        bounds.y(),
                        bounds.max_y(),
                        before.1,
                        viewport.height(),
                        maximum.1,
                    )
                } else {
                    before.1
                },
            );
            if after == before {
                continue;
            }
            let delta = LogicalDelta::new(after.0 - before.0, after.1 - before.1)
                .unwrap_or_else(|_| unreachable!("bounded scroll-to-target delta is finite"));
            transaction
                .scroll_updates
                .push(super::transaction::ScrollOffsetUpdate {
                    owner: owner.clone(),
                    offset: after,
                });
            transaction
                .scroll_consumptions
                .push(super::transaction::ScrollOwnerConsumption {
                    owner: owner.clone(),
                    evaluation_order: 0,
                    offered: delta,
                    consumed: delta,
                    remainder: LogicalDelta::ZERO,
                    offset: LogicalDelta::new(after.0, after.1)
                        .unwrap_or_else(|_| unreachable!("bounded scroll offset is finite")),
                    maximum: LogicalDelta::new(maximum.0, maximum.1)
                        .unwrap_or_else(|_| unreachable!("bounded scroll maximum is finite")),
                });
            transaction.scroll_chain_remainder = Some(LogicalDelta::ZERO);
            break;
        }
    }

    fn request_clipboard_default(
        &self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> bool {
        let owner = transaction.target.clone();
        if self.focus.focused_node() != Some(&owner) {
            return false;
        }
        let Some(context) = self.editing.framework_service_context(&owner) else {
            return false;
        };
        let is_write = matches!(command, SemanticCommand::Copy | SemanticCommand::Cut);
        if is_write
            && (context.sensitivity != TextSensitivity::Public
                || context.selection.is_collapsed()
                || !clipboard_write_payload_is_bounded(&context.selected_text)
                || (command == SemanticCommand::Cut && (context.read_only || context.disabled)))
        {
            return false;
        }
        let Some(surface_context) = self.surface_publication.current_surface_input_context() else {
            return false;
        };
        let request = match command {
            SemanticCommand::Copy => FrameworkServiceRequest::ClipboardWriteText {
                text: context.selected_text,
                purpose: ClipboardWritePurpose::Copy,
            },
            SemanticCommand::Cut => FrameworkServiceRequest::ClipboardWriteText {
                text: context.selected_text,
                purpose: ClipboardWritePurpose::Cut,
            },
            SemanticCommand::Paste => FrameworkServiceRequest::ClipboardReadText {
                max_bytes: MAX_CLIPBOARD_BYTES,
            },
            _ => return false,
        };
        if transaction.consume_mandatory_default_command().is_err() {
            return false;
        }
        let binding = FrameworkServiceBinding::__runtime_new(
            owner.clone(),
            self.surface_publication.surface_id().clone(),
            Some(surface_context),
            Some(context.session),
            Some(context.snapshot),
            Some(context.selection),
            self.composition.generation().cloned(),
        );
        transaction.mounted_work.push((
            owner,
            MountedEffect::FrameworkService(FrameworkServiceEffect::__runtime_new(
                request, binding,
            )),
        ));
        true
    }

    #[allow(clippy::too_many_lines)]
    fn apply_editing_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let target = transaction.target.clone();
        if !self.editing.has_owner(&target) || self.focus.focused_node() != Some(&target) {
            return Ok(());
        }
        if command == SemanticCommand::SetSelection {
            let Some(SemanticActionData::Selection(selection)) = transaction
                .semantic_target
                .as_ref()
                .and_then(runenui_core::SemanticActionTarget::data)
            else {
                return Ok(());
            };
            let Ok((snapshot, source)) = self.editing.caret_source(&target) else {
                return Ok(());
            };
            let Ok(map) = self
                .surface_publication
                .text_caret_map(&target, snapshot, &source)
            else {
                return Ok(());
            };
            if self
                .editing
                .set_selection(&target, *selection, &map)
                .is_err()
            {
                return Ok(());
            }
            transaction.invalidation |= runenui_core::WidgetInvalidation::SEMANTICS;
            self.tree.mark_runtime_semantic_product_dirty();
            return Ok(());
        }
        if matches!(
            command,
            SemanticCommand::DeleteBackward
                | SemanticCommand::DeleteForward
                | SemanticCommand::Undo
                | SemanticCommand::Redo
                | SemanticCommand::ReplaceSelection
        ) {
            transaction.consume_mandatory_default_command()?;
        }
        let namespace = self.tree.runtime_namespace();
        if command == SemanticCommand::ReplaceSelection {
            let Some(SemanticActionData::ReplacementText(text)) = transaction
                .semantic_target
                .as_ref()
                .and_then(runenui_core::SemanticActionTarget::data)
            else {
                return Ok(());
            };
            let prepared = match self
                .editing
                .prepare_replace_selection(&namespace, &target, text)
            {
                Ok(prepared) => prepared,
                Err(
                    crate::editing::EditPrepareError::MissingOwner
                    | crate::editing::EditPrepareError::Unavailable
                    | crate::editing::EditPrepareError::InvalidCoordinates,
                ) => return Ok(()),
                Err(
                    crate::editing::EditPrepareError::PendingCapacity
                    | crate::editing::EditPrepareError::RequestExhausted,
                ) => return Err(TraceRoutedIntegrityFailure::CommitInvariantFailure),
            };
            transaction.invalidation |= runenui_core::WidgetInvalidation::PAINT
                | runenui_core::WidgetInvalidation::SEMANTICS;
            self.tree.mark_runtime_semantic_product_dirty();
            transaction
                .default_outputs
                .push(CollectedRoutedOutput::EditAction {
                    action: prepared.action,
                    origin: prepared.origin,
                    causal_parent: transaction.parent,
                    current_target: target,
                });
            return Ok(());
        }
        let caret_map = if matches!(
            command,
            SemanticCommand::MoveBackward
                | SemanticCommand::MoveForward
                | SemanticCommand::MoveUp
                | SemanticCommand::MoveDown
                | SemanticCommand::ExtendBackward
                | SemanticCommand::ExtendForward
                | SemanticCommand::ExtendUp
                | SemanticCommand::ExtendDown
                | SemanticCommand::SelectAll
                | SemanticCommand::DeleteBackward
                | SemanticCommand::DeleteForward
        ) {
            let Ok((snapshot, source)) = self.editing.caret_source(&target) else {
                return Ok(());
            };
            let Ok(map) = self
                .surface_publication
                .text_caret_map(&target, snapshot, &source)
            else {
                return Ok(());
            };
            Some(map)
        } else {
            None
        };
        let prepared =
            match self
                .editing
                .prepare_command(&namespace, &target, command, caret_map.as_ref())
            {
                Ok(prepared) => prepared,
                Err(
                    crate::editing::EditPrepareError::MissingOwner
                    | crate::editing::EditPrepareError::Unavailable
                    | crate::editing::EditPrepareError::InvalidCoordinates,
                ) => return Ok(()),
                Err(
                    crate::editing::EditPrepareError::PendingCapacity
                    | crate::editing::EditPrepareError::RequestExhausted,
                ) => return Err(TraceRoutedIntegrityFailure::CommitInvariantFailure),
            };
        transaction.invalidation |=
            runenui_core::WidgetInvalidation::PAINT | runenui_core::WidgetInvalidation::SEMANTICS;
        self.tree.mark_runtime_semantic_product_dirty();
        if let Some(prepared) = prepared {
            transaction
                .default_outputs
                .push(CollectedRoutedOutput::EditAction {
                    action: prepared.action,
                    origin: prepared.origin,
                    causal_parent: transaction.parent,
                    current_target: target,
                });
        }
        Ok(())
    }

    fn semantic_default_target_rejection(
        &self,
        transaction: &RoutedTransaction<Action>,
        _command: SemanticCommand,
    ) -> Option<TraceSemanticActionRejection> {
        let semantic_target = transaction.semantic_target.as_ref()?;
        match self.revalidate_semantic_action_target(semantic_target) {
            Ok(owner) if owner == transaction.target => None,
            Ok(_) => Some(TraceSemanticActionRejection::OwnerChanged),
            Err(kind) => Some(trace_semantic_action_rejection(kind)),
        }
    }

    fn invoke_activation_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let activation = self
            .tree
            .activation_probe(&transaction.target)
            .map_err(|_| TraceRoutedIntegrityFailure::SemanticDefaultFailure)?;
        let requires_actionable = transaction
            .semantic_target
            .as_ref()
            .is_none_or(|target| target.semantic_key().is_primary());
        if !activation.enabled() || (requires_actionable && !activation.is_actionable()) {
            return Ok(());
        }
        let target = transaction.target.clone();
        let subscription_credit = transaction.subscription_credit(&target);
        let activation = self
            .tree
            .activate(
                &target,
                transaction.output_allowance(&target),
                transaction.semantic_target.clone(),
            )
            .map_err(|_| TraceRoutedIntegrityFailure::SemanticDefaultFailure)?;
        transaction.remaining_outputs = activation.remaining_outputs;
        if activation.overflowed {
            return Err(TraceRoutedIntegrityFailure::OutputAllowanceExceeded);
        }
        self.record_event_mutation(
            transaction,
            &target,
            activation.state_changed,
            activation.invalidation,
            activation.subscription_invalidation,
            subscription_credit,
        );
        for effect in activation.outputs {
            match effect {
                MountedEffect::Action(action) => {
                    transaction.parent = self.trace.record_event(
                        TraceRecordKind::RoutedActionCollected,
                        transaction.sequence,
                        transaction.parent,
                        Some(transaction.target_trace.clone()),
                        transaction.instant,
                        &transaction.target,
                        Some(&transaction.target),
                        transaction.origin,
                    );
                    transaction
                        .default_outputs
                        .push(CollectedRoutedOutput::Action {
                            action,
                            causal_parent: transaction.parent,
                            current_target: transaction.target.clone(),
                        });
                }
                effect => transaction
                    .mounted_work
                    .push((transaction.target.clone(), effect)),
            }
        }
        Ok(())
    }
}

fn bounded_scroll_offset(current: f32, delta: f32, maximum: f32) -> f32 {
    (current + delta).clamp(0.0, maximum)
}

fn reveal_axis(
    target_start: f32,
    target_end: f32,
    current: f32,
    viewport: f32,
    maximum: f32,
) -> f32 {
    let next = if target_start < current {
        target_start
    } else if target_end > current + viewport {
        target_end - viewport
    } else {
        current
    };
    next.clamp(0.0, maximum)
}

const fn clipboard_write_payload_is_bounded(text: &str) -> bool {
    text.len() <= MAX_CLIPBOARD_BYTES
}

#[cfg(test)]
mod tests {
    use super::{MAX_CLIPBOARD_BYTES, clipboard_write_payload_is_bounded};

    #[test]
    fn native_clipboard_writes_are_bounded_before_host_work_is_staged() {
        let within_limit = "x".repeat(MAX_CLIPBOARD_BYTES);
        let over_limit = "x".repeat(MAX_CLIPBOARD_BYTES + 1);

        assert!(clipboard_write_payload_is_bounded(&within_limit));
        assert!(!clipboard_write_payload_is_bounded(&over_limit));
    }
}
