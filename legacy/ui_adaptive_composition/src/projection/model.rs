//! Derived adaptive region projection. Canonical composition remains untouched.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ui_composition::{
    CompositionDefinitionId, CompositionSnapshot, MountedUnitId, PresentationTargetId, RegionId,
    RegionKind, SplitAxis, StateRevision,
};
use ui_math::UiRect;

use crate::{
    AdaptiveAccessibilityNode, AdaptiveAccessibilityProjection, AdaptiveCompositionRejection,
    AdaptiveDiagnosticCode as Code, AdaptiveDiagnosticRecord as Record,
    AdaptiveDiagnosticStage as Stage, AdaptiveDiagnosticSubject as Subject, AdaptiveInspectionRole,
    AdaptiveProjectionPolicy, CompactBehavior,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdaptivePresentationMode {
    Normal,
    Drawer,
    Overflow,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedRegion {
    pub target: PresentationTargetId,
    pub region: RegionId,
    pub bounds: UiRect,
    pub mode: AdaptivePresentationMode,
    pub priority: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedMountedUnit {
    pub target: PresentationTargetId,
    pub region: RegionId,
    pub mounted_unit: MountedUnitId,
    pub bounds: UiRect,
    pub mode: AdaptivePresentationMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveProjectionState {
    definition_id: CompositionDefinitionId,
    source_revision: StateRevision,
    regions: Arc<[ProjectedRegion]>,
    mounted_units: Arc<[ProjectedMountedUnit]>,
    accessibility: AdaptiveAccessibilityProjection,
    diagnostics: Arc<[Record]>,
}

impl AdaptiveProjectionState {
    pub fn derive(
        snapshot: CompositionSnapshot<'_>,
        policy: &AdaptiveProjectionPolicy,
    ) -> Result<Self, AdaptiveCompositionRejection> {
        let mut regions = Vec::new();
        let mut mounted_units = Vec::new();
        let mut diagnostics = Vec::new();
        let mut visiting = BTreeSet::new();
        for root in snapshot.roots() {
            let constraints = policy.target(root.target).ok_or_else(|| {
                AdaptiveCompositionRejection::single(Record::error(
                    Code::TargetMissing,
                    Stage::Projection,
                    Subject::Target(root.target),
                    "Provide adaptive target constraints for every composition root target.",
                ))
            })?;
            project_region(
                snapshot,
                policy,
                root.target,
                root.region,
                constraints.bounds,
                &mut visiting,
                &mut regions,
                &mut mounted_units,
            )?;
        }
        regions.sort_by_key(|value| (value.target, value.region));
        mounted_units.sort_by_key(|value| (value.target, value.mounted_unit));
        let accessibility = accessibility_projection(&regions, &mounted_units, policy);
        if !accessibility.is_complete() {
            diagnostics.push(Record::error(
                Code::AccessibilityIncomplete,
                Stage::Accessibility,
                Subject::General("adaptive_projection".to_owned()),
                "Provide complete labels, focus facts, text scale, contrast, motion, and touch metadata.",
            ));
        }
        Ok(Self {
            definition_id: snapshot.definition_id(),
            source_revision: snapshot.revision(),
            regions: regions.into(),
            mounted_units: mounted_units.into(),
            accessibility,
            diagnostics: diagnostics.into(),
        })
    }

    pub const fn definition_id(&self) -> CompositionDefinitionId {
        self.definition_id
    }
    pub const fn source_revision(&self) -> StateRevision {
        self.source_revision
    }
    pub fn regions(&self) -> &[ProjectedRegion] {
        &self.regions
    }
    pub fn mounted_units(&self) -> &[ProjectedMountedUnit] {
        &self.mounted_units
    }
    pub fn accessibility(&self) -> &AdaptiveAccessibilityProjection {
        &self.accessibility
    }
    pub fn diagnostics(&self) -> &[Record] {
        &self.diagnostics
    }
    pub fn shared_regions(&self) -> Arc<[ProjectedRegion]> {
        Arc::clone(&self.regions)
    }
    pub fn region(&self, id: RegionId) -> Option<ProjectedRegion> {
        self.regions
            .iter()
            .find(|value| value.region == id)
            .copied()
    }
}

#[allow(clippy::too_many_arguments)]
fn project_region(
    snapshot: CompositionSnapshot<'_>,
    policy: &AdaptiveProjectionPolicy,
    target: PresentationTargetId,
    region_id: RegionId,
    bounds: UiRect,
    visiting: &mut BTreeSet<RegionId>,
    output: &mut Vec<ProjectedRegion>,
    mounted: &mut Vec<ProjectedMountedUnit>,
) -> Result<(), AdaptiveCompositionRejection> {
    if !visiting.insert(region_id) {
        return Err(reject(
            Code::RegionCycle,
            Subject::Region(region_id),
            "Project only acyclic composition regions.",
        ));
    }
    let region = snapshot.region(region_id).ok_or_else(|| {
        reject(
            Code::RegionMissing,
            Subject::Region(region_id),
            "Reference only regions present in the composition snapshot.",
        )
    })?;
    let region_policy = policy.region(region_id);
    let constrained =
        bounds.width < region_policy.minimum_width || bounds.height < region_policy.minimum_height;
    let mode = if constrained {
        match region_policy.compact_behavior {
            CompactBehavior::Preserve => AdaptivePresentationMode::Normal,
            CompactBehavior::Drawer => AdaptivePresentationMode::Drawer,
            CompactBehavior::Overflow => AdaptivePresentationMode::Overflow,
            CompactBehavior::HideWhenExplicitlyAllowed => AdaptivePresentationMode::Hidden,
        }
    } else {
        AdaptivePresentationMode::Normal
    };
    output.push(ProjectedRegion {
        target,
        region: region_id,
        bounds,
        mode,
        priority: region_policy.priority,
    });
    match &region.kind {
        RegionKind::Split {
            axis,
            fraction,
            first,
            second,
        } => {
            let ratio = f32::from(fraction.basis_points()) / 10_000.0;
            let (first_bounds, second_bounds) = split(bounds, *axis, ratio);
            project_region(
                snapshot,
                policy,
                target,
                *first,
                first_bounds,
                visiting,
                output,
                mounted,
            )?;
            project_region(
                snapshot,
                policy,
                target,
                *second,
                second_bounds,
                visiting,
                output,
                mounted,
            )?;
        }
        RegionKind::Stack { ordered_units, .. } => {
            mounted.extend(ordered_units.iter().map(|unit| ProjectedMountedUnit {
                target,
                region: region_id,
                mounted_unit: *unit,
                bounds,
                mode,
            }));
        }
        RegionKind::Overlay {
            base,
            ordered_overlays,
        } => {
            project_region(
                snapshot, policy, target, *base, bounds, visiting, output, mounted,
            )?;
            for overlay in ordered_overlays {
                project_region(
                    snapshot, policy, target, *overlay, bounds, visiting, output, mounted,
                )?;
            }
        }
        RegionKind::MountPoint { mounted_unit } => mounted.push(ProjectedMountedUnit {
            target,
            region: region_id,
            mounted_unit: *mounted_unit,
            bounds,
            mode,
        }),
    }
    visiting.remove(&region_id);
    Ok(())
}

fn split(bounds: UiRect, axis: SplitAxis, ratio: f32) -> (UiRect, UiRect) {
    match axis {
        SplitAxis::Horizontal => {
            let first = bounds.width * ratio;
            (
                UiRect::new(bounds.x, bounds.y, first, bounds.height),
                UiRect::new(
                    bounds.x + first,
                    bounds.y,
                    bounds.width - first,
                    bounds.height,
                ),
            )
        }
        SplitAxis::Vertical => {
            let first = bounds.height * ratio;
            (
                UiRect::new(bounds.x, bounds.y, bounds.width, first),
                UiRect::new(
                    bounds.x,
                    bounds.y + first,
                    bounds.width,
                    bounds.height - first,
                ),
            )
        }
    }
}

fn accessibility_projection(
    regions: &[ProjectedRegion],
    mounted: &[ProjectedMountedUnit],
    policy: &AdaptiveProjectionPolicy,
) -> AdaptiveAccessibilityProjection {
    let by_region = mounted.iter().fold(
        BTreeMap::<RegionId, MountedUnitId>::new(),
        |mut map, unit| {
            map.entry(unit.region).or_insert(unit.mounted_unit);
            map
        },
    );
    AdaptiveAccessibilityProjection::new(
        regions
            .iter()
            .enumerate()
            .filter_map(|(index, region)| {
                let target = policy.target(region.target)?;
                Some(AdaptiveAccessibilityNode {
                    target: region.target,
                    region: region.region,
                    mounted_unit: by_region.get(&region.region).copied(),
                    role: match region.mode {
                        AdaptivePresentationMode::Drawer => AdaptiveInspectionRole::Drawer,
                        _ if by_region.contains_key(&region.region) => {
                            AdaptiveInspectionRole::Panel
                        }
                        _ => AdaptiveInspectionRole::Region,
                    },
                    label: format!("Composition region {}", region.region.raw()),
                    focus_order: index,
                    focus_visible: true,
                    high_contrast: target.high_contrast,
                    text_scale: target.text_scale,
                    minimum_touch_target: target.minimum_touch_target.max(24.0),
                    transition_duration_ms: if target.reduced_motion { 0 } else { 120 },
                })
            })
            .collect(),
    )
}

fn reject(code: Code, subject: Subject, message: &'static str) -> AdaptiveCompositionRejection {
    AdaptiveCompositionRejection::single(Record::error(code, Stage::Projection, subject, message))
}
