use std::collections::BTreeSet;

use crate::{
    AuthorizedTransaction, CompositionCommand, CompositionCommandKind, CompositionDefinitionV1,
    CompositionDiagnosticCode as Code, CompositionDiagnosticRecord as Record,
    CompositionDiagnosticStage as Stage, CompositionDiagnosticSubject as Subject,
    CompositionPolicies, CompositionPolicyDecision, CompositionRejection, CompositionState,
    CompositionTransaction, DefinitionRevision, MountedUnitId, RegionId, RegionKind, StateRevision,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositionCommit {
    pub transaction: crate::CompositionTransactionId,
    pub revision: StateRevision,
}

impl CompositionState {
    pub fn authorize(
        &self,
        transaction: CompositionTransaction,
        policies: CompositionPolicies<'_>,
    ) -> Result<AuthorizedTransaction, CompositionRejection> {
        self.authorize_mode(transaction, policies, false)
    }

    pub(crate) fn authorize_history(
        &self,
        transaction: CompositionTransaction,
        policies: CompositionPolicies<'_>,
    ) -> Result<AuthorizedTransaction, CompositionRejection> {
        self.authorize_mode(transaction, policies, true)
    }

    fn authorize_mode(
        &self,
        transaction: CompositionTransaction,
        policies: CompositionPolicies<'_>,
        allows_history_restore: bool,
    ) -> Result<AuthorizedTransaction, CompositionRejection> {
        let mut diagnostics =
            basic_transaction_diagnostics(self, &transaction, allows_history_restore);
        for decision in [
            policies.lifecycle.evaluate(self.snapshot(), &transaction),
            policies.capability.evaluate(self.snapshot(), &transaction),
            policies.target.evaluate(self.snapshot(), &transaction),
        ] {
            if let CompositionPolicyDecision::Rejected(mut rejected) = decision {
                diagnostics.append(&mut rejected);
            }
        }
        if !diagnostics.is_empty() {
            return Err(CompositionRejection::new(diagnostics));
        }
        Ok(AuthorizedTransaction {
            authorized_revision: self.revision,
            transaction,
            allows_history_restore,
        })
    }

    pub fn transact(
        &mut self,
        transaction: CompositionTransaction,
        policies: CompositionPolicies<'_>,
    ) -> Result<CompositionCommit, CompositionRejection> {
        let authorized = self.authorize(transaction, policies)?;
        self.apply_authorized(authorized)
    }

    pub fn apply_authorized(
        &mut self,
        authorized: AuthorizedTransaction,
    ) -> Result<CompositionCommit, CompositionRejection> {
        self.apply_authorized_mode(authorized, true)
    }

    pub(crate) fn apply_authorized_mode(
        &mut self,
        authorized: AuthorizedTransaction,
        record_history: bool,
    ) -> Result<CompositionCommit, CompositionRejection> {
        let transaction = authorized.transaction;
        let diagnostics =
            basic_transaction_diagnostics(self, &transaction, authorized.allows_history_restore);
        if authorized.authorized_revision != self.revision || !diagnostics.is_empty() {
            let mut all = diagnostics;
            if authorized.authorized_revision != self.revision {
                all.push(Record::error(
                    Code::StaleRevision,
                    Stage::Transaction,
                    Subject::Transaction(transaction.id()),
                    "Reauthorize the transaction against the current composition revision.",
                ));
            }
            return Err(CompositionRejection::new(all));
        }

        let mut candidate = self.definition.clone();
        apply_commands(&mut candidate, transaction.commands())
            .map_err(CompositionRejection::single)?;
        let diagnostics = if resize_commands_preserve_global_invariants(transaction.commands()) {
            Vec::new()
        } else {
            crate::validation::validate_transaction_candidate(&candidate)
        };
        if !diagnostics.is_empty() {
            return Err(CompositionRejection::new(diagnostics));
        }

        let Some(next_raw) = self.revision.raw().checked_add(1) else {
            return Err(CompositionRejection::single(Record::error(
                Code::RevisionOverflow,
                Stage::Transaction,
                Subject::Transaction(transaction.id()),
                "Promote or reload the composition into a fresh revision sequence before retrying.",
            )));
        };
        let next = StateRevision::new(next_raw);
        candidate.set_revision(DefinitionRevision::new(next_raw));
        candidate = candidate.normalized();
        let before = std::mem::replace(&mut self.definition, candidate);
        self.revision = next;
        self.applied_transactions.insert(transaction.id());
        if record_history {
            self.history.record(crate::CompositionJournalEntry::new(
                transaction.clone(),
                vec![CompositionCommand::restore_definition(before)],
                next,
            ));
        }
        Ok(CompositionCommit {
            transaction: transaction.id(),
            revision: next,
        })
    }
}

fn basic_transaction_diagnostics(
    state: &CompositionState,
    transaction: &CompositionTransaction,
    allows_history_restore: bool,
) -> Vec<Record> {
    let mut diagnostics = Vec::new();
    if transaction.commands().is_empty() {
        diagnostics.push(Record::error(
            Code::EmptyTransaction,
            Stage::Transaction,
            Subject::Transaction(transaction.id()),
            "Add at least one typed structural command.",
        ));
    }
    if transaction.expected_revision() != state.revision {
        diagnostics.push(Record::error(
            Code::StaleRevision,
            Stage::Transaction,
            Subject::Transaction(transaction.id()),
            "Rebuild the transaction against the current revision.",
        ));
    }
    if state.applied_transactions.contains(&transaction.id()) {
        diagnostics.push(Record::error(
            Code::DuplicateTransactionId,
            Stage::Transaction,
            Subject::Transaction(transaction.id()),
            "Submit the operation with a new transaction ID.",
        ));
    }
    if !allows_history_restore
        && transaction
            .commands()
            .iter()
            .any(CompositionCommand::is_history_restore)
    {
        diagnostics.push(Record::error(
            Code::InvalidCommand,
            Stage::History,
            Subject::Transaction(transaction.id()),
            "Use CompositionState::undo or CompositionState::redo for structural history operations.",
        ));
    }
    if state.revision.raw() == u64::MAX {
        diagnostics.push(Record::error(
            Code::RevisionOverflow,
            Stage::Transaction,
            Subject::Transaction(transaction.id()),
            "Promote or reload the composition into a fresh revision sequence before retrying.",
        ));
    }
    diagnostics
}

fn invalid(subject: Subject, message: impl Into<String>) -> Record {
    Record::error(Code::InvalidCommand, Stage::Transaction, subject, message)
}

fn apply_commands(
    definition: &mut CompositionDefinitionV1,
    commands: &[CompositionCommand],
) -> Result<(), Record> {
    if resize_commands_preserve_global_invariants(commands) {
        return apply_resize_split_commands(definition, commands);
    }
    for command in commands {
        apply_command(definition, command)?;
    }
    Ok(())
}

fn resize_commands_preserve_global_invariants(commands: &[CompositionCommand]) -> bool {
    // Ratified input plus locally validated resize commands cannot change graph topology or ownership.
    !commands.is_empty()
        && commands
            .iter()
            .all(|command| matches!(command.kind(), CompositionCommandKind::ResizeSplit { .. }))
}

fn apply_resize_split_commands(
    definition: &mut CompositionDefinitionV1,
    commands: &[CompositionCommand],
) -> Result<(), Record> {
    let (_, _, regions, _) = definition.parts_mut();
    for command in commands {
        let CompositionCommandKind::ResizeSplit { region, fraction } = command.kind() else {
            unreachable!("resize batch classification accepts only resize commands");
        };
        let index = regions
            .binary_search_by_key(region, |value| value.id)
            .map_err(|_| invalid(Subject::Region(*region), "Split does not exist."))?;
        let RegionKind::Split {
            fraction: current, ..
        } = &mut regions[index].kind
        else {
            return Err(invalid(Subject::Region(*region), "Region is not a split."));
        };
        *current = *fraction;
    }
    Ok(())
}

fn apply_command(
    definition: &mut CompositionDefinitionV1,
    command: &CompositionCommand,
) -> Result<(), Record> {
    if let CompositionCommandKind::RestoreDefinition {
        definition: restored,
    } = command.kind()
    {
        *definition = restored.clone();
        return Ok(());
    }
    let (targets, roots, regions, units) = definition.parts_mut();
    match command.kind() {
        CompositionCommandKind::MountUnit {
            unit,
            stack,
            ordinal,
        } => {
            if units.iter().any(|value| value.id == unit.id) {
                return Err(invalid(
                    Subject::MountedUnit(unit.id),
                    "Mounted unit ID already exists.",
                ));
            }
            let region = regions
                .iter_mut()
                .find(|value| value.id == *stack)
                .ok_or_else(|| {
                    invalid(Subject::Region(*stack), "Destination stack does not exist.")
                })?;
            let RegionKind::Stack {
                ordered_units,
                active_unit,
            } = &mut region.kind
            else {
                return Err(invalid(
                    Subject::Region(*stack),
                    "Destination region is not a stack.",
                ));
            };
            let index = (*ordinal).min(ordered_units.len());
            ordered_units.insert(index, unit.id);
            if ordered_units.len() == 1 {
                *active_unit = unit.id;
            }
            units.push(unit.clone());
        }
        CompositionCommandKind::UnmountUnit { unit } => {
            remove_unit_from_regions(regions, *unit)?;
            let Some(index) = units.iter().position(|value| value.id == *unit) else {
                return Err(invalid(
                    Subject::MountedUnit(*unit),
                    "Mounted unit does not exist.",
                ));
            };
            units.remove(index);
        }
        CompositionCommandKind::ActivateUnit { stack, unit } => {
            let region = regions
                .iter_mut()
                .find(|value| value.id == *stack)
                .ok_or_else(|| invalid(Subject::Region(*stack), "Stack does not exist."))?;
            let RegionKind::Stack {
                ordered_units,
                active_unit,
            } = &mut region.kind
            else {
                return Err(invalid(Subject::Region(*stack), "Region is not a stack."));
            };
            if !ordered_units.contains(unit) {
                return Err(invalid(
                    Subject::MountedUnit(*unit),
                    "Unit does not belong to the stack.",
                ));
            }
            *active_unit = *unit;
        }
        CompositionCommandKind::MoveUnit {
            unit,
            stack,
            ordinal,
        } => {
            remove_unit_from_regions(regions, *unit)?;
            let destination = regions
                .iter_mut()
                .find(|value| value.id == *stack)
                .ok_or_else(|| {
                    invalid(Subject::Region(*stack), "Destination stack does not exist.")
                })?;
            let RegionKind::Stack {
                ordered_units,
                active_unit,
            } = &mut destination.kind
            else {
                return Err(invalid(
                    Subject::Region(*stack),
                    "Destination region is not a stack.",
                ));
            };
            ordered_units.insert((*ordinal).min(ordered_units.len()), *unit);
            if ordered_units.len() == 1 {
                *active_unit = *unit;
            }
        }
        CompositionCommandKind::ReorderStack {
            stack,
            ordered_units,
            active_unit,
        } => {
            let region = regions
                .iter_mut()
                .find(|value| value.id == *stack)
                .ok_or_else(|| invalid(Subject::Region(*stack), "Stack does not exist."))?;
            region.kind = RegionKind::Stack {
                ordered_units: ordered_units.clone(),
                active_unit: *active_unit,
            };
        }
        CompositionCommandKind::SplitRegion {
            region,
            preserved_child,
            new_region,
            axis,
            fraction,
            new_region_first,
        } => {
            if regions
                .iter()
                .any(|value| value.id == *preserved_child || value.id == new_region.id)
            {
                return Err(invalid(
                    Subject::Region(*region),
                    "Split child IDs must be new.",
                ));
            }
            let existing = regions
                .iter_mut()
                .find(|value| value.id == *region)
                .ok_or_else(|| {
                    invalid(Subject::Region(*region), "Region to split does not exist.")
                })?;
            let preserved = crate::RegionDefinition::new(
                *preserved_child,
                existing.profile.clone(),
                existing.kind.clone(),
            );
            let (first, second) = if *new_region_first {
                (new_region.id, *preserved_child)
            } else {
                (*preserved_child, new_region.id)
            };
            existing.kind = RegionKind::Split {
                axis: *axis,
                fraction: *fraction,
                first,
                second,
            };
            regions.push(preserved);
            regions.push(new_region.clone());
        }
        CompositionCommandKind::SplitRegionWithMovedUnit {
            region,
            preserved_child,
            moved_region,
            unit,
            axis,
            fraction,
            moved_region_first,
        } => {
            if regions
                .iter()
                .any(|value| value.id == *preserved_child || value.id == moved_region.id)
            {
                return Err(invalid(
                    Subject::Region(*region),
                    "Split child IDs must be new.",
                ));
            }
            let RegionKind::Stack {
                ordered_units,
                active_unit,
            } = &moved_region.kind
            else {
                return Err(invalid(
                    Subject::Region(moved_region.id),
                    "Moved-unit split destination must be a stack.",
                ));
            };
            if ordered_units.as_slice() != [*unit] || *active_unit != *unit {
                return Err(invalid(
                    Subject::Region(moved_region.id),
                    "Moved-unit split destination must contain exactly the moved unit as active.",
                ));
            }
            if !regions.iter().any(|value| value.id == *region) {
                return Err(invalid(
                    Subject::Region(*region),
                    "Region to split does not exist.",
                ));
            }
            remove_unit_from_regions(regions, *unit)?;
            let existing = regions
                .iter_mut()
                .find(|value| value.id == *region)
                .expect("split destination presence was checked before unit removal");
            if matches!(
                &existing.kind,
                RegionKind::Stack { ordered_units, .. } if ordered_units.is_empty()
            ) {
                return Err(invalid(
                    Subject::Region(*region),
                    "Splitting the unit's own single-unit stack would leave an empty preserved region.",
                ));
            }
            let preserved = crate::RegionDefinition::new(
                *preserved_child,
                existing.profile.clone(),
                existing.kind.clone(),
            );
            let (first, second) = if *moved_region_first {
                (moved_region.id, *preserved_child)
            } else {
                (*preserved_child, moved_region.id)
            };
            existing.kind = RegionKind::Split {
                axis: *axis,
                fraction: *fraction,
                first,
                second,
            };
            regions.push(preserved);
            regions.push(moved_region.clone());
        }
        CompositionCommandKind::ResizeSplit { region, fraction } => {
            let found = regions
                .iter_mut()
                .find(|value| value.id == *region)
                .ok_or_else(|| invalid(Subject::Region(*region), "Split does not exist."))?;
            let RegionKind::Split {
                fraction: current, ..
            } = &mut found.kind
            else {
                return Err(invalid(Subject::Region(*region), "Region is not a split."));
            };
            *current = *fraction;
        }
        CompositionCommandKind::MergeSplit {
            region,
            retained_child,
        } => merge_split(regions, *region, *retained_child)?,
        CompositionCommandKind::CreateRoot {
            root,
            regions: new_regions,
        } => {
            roots.push(root.clone());
            regions.extend(new_regions.clone());
        }
        CompositionCommandKind::CreateRootWithMovedUnit { root, region, unit } => {
            if roots.iter().any(|value| value.id == root.id) {
                return Err(invalid(Subject::Root(root.id), "Root ID already exists."));
            }
            if regions.iter().any(|value| value.id == region.id) {
                return Err(invalid(
                    Subject::Region(region.id),
                    "Root region ID already exists.",
                ));
            }
            if root.region != region.id {
                return Err(invalid(
                    Subject::Root(root.id),
                    "Moved-unit root must reference its supplied region.",
                ));
            }
            let RegionKind::Stack {
                ordered_units,
                active_unit,
            } = &region.kind
            else {
                return Err(invalid(
                    Subject::Region(region.id),
                    "Moved-unit root region must be a stack.",
                ));
            };
            if ordered_units.as_slice() != [*unit] || *active_unit != *unit {
                return Err(invalid(
                    Subject::Region(region.id),
                    "Moved-unit root stack must contain exactly the moved unit as active.",
                ));
            }
            remove_unit_from_regions(regions, *unit)?;
            roots.push(root.clone());
            regions.push(region.clone());
        }
        CompositionCommandKind::MoveRoot {
            root,
            target,
            primary,
        } => {
            let found = roots
                .iter_mut()
                .find(|value| value.id == *root)
                .ok_or_else(|| invalid(Subject::Root(*root), "Root does not exist."))?;
            found.target = *target;
            found.primary = *primary;
        }
        CompositionCommandKind::CloseRoot { root } => close_root(roots, regions, *root)?,
        CompositionCommandKind::AttachTarget { target } => targets.push(target.clone()),
        CompositionCommandKind::DetachTarget { target } => {
            if roots.iter().any(|root| root.target == *target) {
                return Err(invalid(
                    Subject::Target(*target),
                    "Close or move every target root before detaching the target.",
                ));
            }
            let Some(index) = targets.iter().position(|value| value.id == *target) else {
                return Err(invalid(Subject::Target(*target), "Target does not exist."));
            };
            targets.remove(index);
        }
        CompositionCommandKind::RatifyExtensionState => {}
        CompositionCommandKind::RestoreDefinition { .. } => {
            return Err(invalid(
                Subject::Definition(definition.id()),
                "History restoration must be applied as the complete candidate definition.",
            ));
        }
    }
    Ok(())
}

fn remove_unit_from_regions(
    regions: &mut [crate::RegionDefinition],
    unit: MountedUnitId,
) -> Result<(), Record> {
    for region in regions {
        match &mut region.kind {
            RegionKind::Stack {
                ordered_units,
                active_unit,
            } if ordered_units.contains(&unit) => {
                ordered_units.retain(|value| *value != unit);
                if *active_unit == unit
                    && let Some(first) = ordered_units.first()
                {
                    *active_unit = *first;
                }
                return Ok(());
            }
            RegionKind::MountPoint { mounted_unit } if *mounted_unit == unit => {
                region.kind = RegionKind::Stack {
                    ordered_units: Vec::new(),
                    active_unit: unit,
                };
                return Ok(());
            }
            _ => {}
        }
    }
    Err(invalid(
        Subject::MountedUnit(unit),
        "Mounted unit has no structural location.",
    ))
}

fn subtree_ids(regions: &[crate::RegionDefinition], start: RegionId) -> BTreeSet<RegionId> {
    let mut result = BTreeSet::new();
    let mut pending = vec![start];
    while let Some(id) = pending.pop() {
        if !result.insert(id) {
            continue;
        }
        if let Some(region) = regions.iter().find(|value| value.id == id) {
            pending.extend(region.kind.child_regions());
        }
    }
    result
}

fn close_root(
    roots: &mut Vec<crate::CompositionRootDefinition>,
    regions: &mut Vec<crate::RegionDefinition>,
    root: crate::CompositionRootId,
) -> Result<(), Record> {
    let Some(index) = roots.iter().position(|value| value.id == root) else {
        return Err(invalid(Subject::Root(root), "Root does not exist."));
    };
    let root_region = roots[index].region;
    let subtree = subtree_ids(regions, root_region);
    if regions
        .iter()
        .filter(|value| subtree.contains(&value.id))
        .any(|value| !value.kind.mounted_units().is_empty())
    {
        return Err(invalid(
            Subject::Root(root),
            "Move or unmount every unit before closing the root.",
        ));
    }
    roots.remove(index);
    regions.retain(|value| !subtree.contains(&value.id));
    Ok(())
}

fn merge_split(
    regions: &mut Vec<crate::RegionDefinition>,
    split_id: RegionId,
    retained: RegionId,
) -> Result<(), Record> {
    let split = regions
        .iter()
        .find(|value| value.id == split_id)
        .cloned()
        .ok_or_else(|| invalid(Subject::Region(split_id), "Split does not exist."))?;
    let RegionKind::Split { first, second, .. } = split.kind else {
        return Err(invalid(Subject::Region(split_id), "Region is not a split."));
    };
    if retained != first && retained != second {
        return Err(invalid(
            Subject::Region(retained),
            "Retained region is not a split child.",
        ));
    }
    let discarded = if retained == first { second } else { first };
    let discarded_ids = subtree_ids(regions, discarded);
    if regions
        .iter()
        .filter(|value| discarded_ids.contains(&value.id))
        .any(|value| !value.kind.mounted_units().is_empty())
    {
        return Err(invalid(
            Subject::Region(discarded),
            "Move or unmount every unit before merging the split.",
        ));
    }
    let retained_region = regions
        .iter()
        .find(|value| value.id == retained)
        .cloned()
        .ok_or_else(|| invalid(Subject::Region(retained), "Retained region does not exist."))?;
    let Some(target) = regions.iter_mut().find(|value| value.id == split_id) else {
        return Err(invalid(
            Subject::Region(split_id),
            "Split disappeared while applying the merge.",
        ));
    };
    target.profile = retained_region.profile;
    target.kind = retained_region.kind;
    regions.retain(|value| value.id != retained && !discarded_ids.contains(&value.id));
    Ok(())
}
