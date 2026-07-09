use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CompositionDefinitionV1, CompositionDiagnosticCode as Code,
    CompositionDiagnosticRecord as Record, CompositionDiagnosticStage as Stage,
    CompositionDiagnosticSubject as Subject, MountedUnitId, RegionId, RegionKind,
};

pub(crate) fn validate_definition(definition: &CompositionDefinitionV1) -> Vec<Record> {
    let mut diagnostics = Vec::new();
    if definition.schema_version() != CompositionDefinitionV1::SCHEMA_VERSION {
        diagnostics.push(Record::error(
            Code::UnsupportedSchemaVersion,
            Stage::Formation,
            Subject::Definition(definition.id()),
            "Use composition schema version 1.",
        ));
    }
    if definition.targets().is_empty()
        || definition.roots().is_empty()
        || definition.regions().is_empty()
        || definition.mounted_units().is_empty()
    {
        diagnostics.push(Record::error(
            Code::EmptyDefinition,
            Stage::Formation,
            Subject::Definition(definition.id()),
            "Define at least one target, primary root, reachable region, and mounted unit.",
        ));
    }

    let targets = unique_map(
        definition.targets(),
        |value| value.id,
        Code::DuplicateTargetId,
        Subject::Target,
        &mut diagnostics,
    );
    let roots = unique_map(
        definition.roots(),
        |value| value.id,
        Code::DuplicateRootId,
        Subject::Root,
        &mut diagnostics,
    );
    let regions = unique_map(
        definition.regions(),
        |value| value.id,
        Code::DuplicateRegionId,
        Subject::Region,
        &mut diagnostics,
    );
    let units = unique_map(
        definition.mounted_units(),
        |value| value.id,
        Code::DuplicateMountedUnitId,
        Subject::MountedUnit,
        &mut diagnostics,
    );

    validate_roots(definition, &targets, &regions, &mut diagnostics);
    validate_regions(definition, &roots, &regions, &units, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn unique_map<'a, T, I: Copy + Ord>(
    values: &'a [T],
    id: impl Fn(&T) -> I,
    code: Code,
    subject: impl Fn(I) -> Subject,
    diagnostics: &mut Vec<Record>,
) -> BTreeMap<I, &'a T> {
    let mut result = BTreeMap::new();
    for value in values {
        let value_id = id(value);
        if result.insert(value_id, value).is_some() {
            diagnostics.push(Record::error(
                code,
                Stage::Formation,
                subject(value_id),
                "Remove the duplicate record or assign a distinct typed ID.",
            ));
        }
    }
    result
}

fn validate_roots(
    definition: &CompositionDefinitionV1,
    targets: &BTreeMap<crate::PresentationTargetId, &crate::PresentationTargetDefinition>,
    regions: &BTreeMap<RegionId, &crate::RegionDefinition>,
    diagnostics: &mut Vec<Record>,
) {
    let mut primary_counts = BTreeMap::new();
    for root in definition.roots() {
        if !targets.contains_key(&root.target) {
            diagnostics.push(Record::error(
                Code::UnknownTarget,
                Stage::Formation,
                Subject::Root(root.id),
                "Attach the root to a declared presentation target.",
            ));
        }
        if !regions.contains_key(&root.region) {
            diagnostics.push(Record::error(
                Code::UnknownRegion,
                Stage::Formation,
                Subject::Root(root.id),
                "Point the root at a declared region.",
            ));
        }
        if root.primary {
            *primary_counts.entry(root.target).or_insert(0usize) += 1;
        }
    }
    for target in definition.targets() {
        let count = primary_counts.get(&target.id).copied().unwrap_or_default();
        if count != 1 {
            diagnostics.push(
                Record::error(
                    Code::InvalidPrimaryRootCount,
                    Stage::Formation,
                    Subject::Target(target.id),
                    "Declare exactly one primary root for this target.",
                )
                .with_context("actual", count.to_string()),
            );
        }
    }
}

fn validate_regions(
    definition: &CompositionDefinitionV1,
    _roots: &BTreeMap<crate::CompositionRootId, &crate::CompositionRootDefinition>,
    regions: &BTreeMap<RegionId, &crate::RegionDefinition>,
    units: &BTreeMap<MountedUnitId, &crate::MountedUnitDefinition>,
    diagnostics: &mut Vec<Record>,
) {
    let mut child_parent_counts = BTreeMap::<RegionId, usize>::new();
    let mut root_parent_counts = BTreeMap::<RegionId, usize>::new();
    let mut unit_locations = BTreeMap::<MountedUnitId, usize>::new();

    for root in definition.roots() {
        *root_parent_counts.entry(root.region).or_default() += 1;
    }

    for region in definition.regions() {
        for child in region.kind.child_regions() {
            if !regions.contains_key(&child) {
                diagnostics.push(Record::error(
                    Code::UnknownRegion,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "Reference only declared child regions.",
                ));
            }
            *child_parent_counts.entry(child).or_default() += 1;
        }
        for unit in region.kind.mounted_units() {
            if !units.contains_key(unit) {
                diagnostics.push(Record::error(
                    Code::UnknownMountedUnit,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "Reference only declared mounted units.",
                ));
            }
            *unit_locations.entry(*unit).or_default() += 1;
        }
        validate_region_kind(region, diagnostics);
    }

    for region in definition.regions() {
        let child_count = child_parent_counts
            .get(&region.id)
            .copied()
            .unwrap_or_default();
        let root_count = root_parent_counts
            .get(&region.id)
            .copied()
            .unwrap_or_default();
        let valid = (root_count == 1 && child_count == 0) || (root_count == 0 && child_count == 1);
        if !valid {
            diagnostics.push(
                Record::error(
                    Code::InvalidRegionParentCount,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "Give every region exactly one root owner or one structural parent.",
                )
                .with_context("root_owners", root_count.to_string())
                .with_context("structural_parents", child_count.to_string()),
            );
        }
    }

    for unit in definition.mounted_units() {
        let count = unit_locations.get(&unit.id).copied().unwrap_or_default();
        if count != 1 {
            diagnostics.push(
                Record::error(
                    Code::InvalidMountedUnitLocation,
                    Stage::Formation,
                    Subject::MountedUnit(unit.id),
                    "Place every mounted unit in exactly one stack or mount point.",
                )
                .with_context("actual", count.to_string()),
            );
        }
    }

    detect_cycles(regions, diagnostics);
    detect_unreachable(definition, regions, diagnostics);
}

fn validate_region_kind(region: &crate::RegionDefinition, diagnostics: &mut Vec<Record>) {
    match &region.kind {
        RegionKind::Split { first, second, .. } if first == second => {
            diagnostics.push(Record::error(
                Code::IdenticalSplitChildren,
                Stage::Formation,
                Subject::Region(region.id),
                "Use two distinct child regions for a split.",
            ))
        }
        RegionKind::Stack {
            ordered_units,
            active_unit,
        } => {
            if ordered_units.is_empty() {
                diagnostics.push(Record::error(
                    Code::EmptyStack,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "Add at least one mounted unit or remove the stack.",
                ));
            }
            let unique = ordered_units.iter().copied().collect::<BTreeSet<_>>();
            if unique.len() != ordered_units.len() {
                diagnostics.push(Record::error(
                    Code::DuplicateStackUnit,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "List each mounted unit once in stack order.",
                ));
            }
            if !ordered_units.contains(active_unit) {
                diagnostics.push(Record::error(
                    Code::InvalidActiveUnit,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "Choose an active unit that belongs to the stack.",
                ));
            }
        }
        RegionKind::Overlay {
            base,
            ordered_overlays,
        } => {
            let unique = ordered_overlays.iter().copied().collect::<BTreeSet<_>>();
            if unique.len() != ordered_overlays.len() {
                diagnostics.push(Record::error(
                    Code::DuplicateOverlayRegion,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "List each overlay region once.",
                ));
            }
            if ordered_overlays.contains(base) {
                diagnostics.push(Record::error(
                    Code::OverlayContainsBase,
                    Stage::Formation,
                    Subject::Region(region.id),
                    "Do not repeat the base region in overlay order.",
                ));
            }
        }
        RegionKind::Split { .. } | RegionKind::MountPoint { .. } => {}
    }
}

fn detect_cycles(
    regions: &BTreeMap<RegionId, &crate::RegionDefinition>,
    diagnostics: &mut Vec<Record>,
) {
    let mut colors = BTreeMap::<RegionId, u8>::new();
    let mut cyclic = BTreeSet::new();
    for id in regions.keys().copied() {
        visit_cycle(id, regions, &mut colors, &mut cyclic);
    }
    for id in cyclic {
        diagnostics.push(Record::error(
            Code::RegionCycle,
            Stage::Formation,
            Subject::Region(id),
            "Break the region cycle before formation.",
        ));
    }
}

fn visit_cycle(
    id: RegionId,
    regions: &BTreeMap<RegionId, &crate::RegionDefinition>,
    colors: &mut BTreeMap<RegionId, u8>,
    cyclic: &mut BTreeSet<RegionId>,
) {
    match colors.get(&id).copied().unwrap_or_default() {
        1 => {
            cyclic.insert(id);
            return;
        }
        2 => return,
        _ => {}
    }
    colors.insert(id, 1);
    if let Some(region) = regions.get(&id) {
        for child in region.kind.child_regions() {
            if colors.get(&child) == Some(&1) {
                cyclic.insert(child);
                cyclic.insert(id);
            }
            visit_cycle(child, regions, colors, cyclic);
        }
    }
    colors.insert(id, 2);
}

fn detect_unreachable(
    definition: &CompositionDefinitionV1,
    regions: &BTreeMap<RegionId, &crate::RegionDefinition>,
    diagnostics: &mut Vec<Record>,
) {
    let mut reachable = BTreeSet::new();
    let mut pending = definition
        .roots()
        .iter()
        .map(|root| root.region)
        .collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if !reachable.insert(id) {
            continue;
        }
        if let Some(region) = regions.get(&id) {
            pending.extend(region.kind.child_regions());
        }
    }
    for id in regions.keys().copied() {
        if !reachable.contains(&id) {
            diagnostics.push(Record::error(
                Code::UnreachableRegion,
                Stage::Formation,
                Subject::Region(id),
                "Connect every declared region to a root.",
            ));
        }
    }
}
