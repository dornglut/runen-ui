//! File: domain/ui/ui_surface/src/validation/mount_ratification.rs
//! Purpose: Mounted-surface ratification reports backed by foundation/ratification.

use std::collections::BTreeMap;

use foundation_ratification::{RatificationIssue, RatificationReport, Ratifier};

use crate::{
    MountedSurfaceInstance, SurfaceDefinitionId, SurfaceDefinitionRegistry, SurfaceInstanceId,
};

/// ui_surface-owned ratification issue codes.
///
/// These codes are domain-owned and intentionally separate from foundation
/// ratification. Foundation owns the report shape; ui_surface owns the meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UiSurfaceRatificationCode {
    DuplicateMountedSurfaceInstance,
    UnknownSurfaceDefinition,
}

/// ui_surface-owned ratification subjects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UiSurfaceRatificationSubject {
    SurfaceInstance(SurfaceInstanceId),
    SurfaceDefinition(SurfaceDefinitionId),
}

/// Concrete candidate for Phase 2 ratification.
///
/// This keeps the first consumer small: one mounted-surface set can be checked
/// against the known surface definitions without mutating the candidate.
#[derive(Debug, Clone, Copy)]
pub struct MountedSurfaceSetCandidate<'a> {
    mounted_surfaces: &'a [MountedSurfaceInstance],
}

impl<'a> MountedSurfaceSetCandidate<'a> {
    pub const fn new(mounted_surfaces: &'a [MountedSurfaceInstance]) -> Self {
        Self { mounted_surfaces }
    }

    pub const fn mounted_surfaces(self) -> &'a [MountedSurfaceInstance] {
        self.mounted_surfaces
    }
}

/// Ratifies mounted surface sets against ui_surface invariants.
#[derive(Debug, Clone, Copy)]
pub struct UiSurfaceMountRatifier<'a> {
    definitions: &'a SurfaceDefinitionRegistry,
}

impl<'a> UiSurfaceMountRatifier<'a> {
    pub const fn new(definitions: &'a SurfaceDefinitionRegistry) -> Self {
        Self { definitions }
    }
}

impl<'ratifier, 'candidate> Ratifier<MountedSurfaceSetCandidate<'candidate>>
    for UiSurfaceMountRatifier<'ratifier>
{
    type Code = UiSurfaceRatificationCode;
    type Subject = UiSurfaceRatificationSubject;

    fn ratify(
        &self,
        candidate: &MountedSurfaceSetCandidate<'candidate>,
    ) -> RatificationReport<Self::Code, Self::Subject> {
        ratify_mounted_surface_set(*candidate, self.definitions)
    }
}

pub fn ratify_mounted_surface_set(
    candidate: MountedSurfaceSetCandidate<'_>,
    definitions: &SurfaceDefinitionRegistry,
) -> RatificationReport<UiSurfaceRatificationCode, UiSurfaceRatificationSubject> {
    let mut report = RatificationReport::accepted();
    let mut first_by_surface_id = BTreeMap::<SurfaceInstanceId, MountedSurfaceInstance>::new();

    for mounted in candidate.mounted_surfaces() {
        if let Some(first) = first_by_surface_id
            .get(&mounted.surface_instance_id)
            .copied()
        {
            report.push(RatificationIssue::error(
                UiSurfaceRatificationCode::DuplicateMountedSurfaceInstance,
                UiSurfaceRatificationSubject::SurfaceInstance(mounted.surface_instance_id),
                format!(
                    "mounted surface instance {} appears more than once; first definition {}, duplicate definition {}",
                    mounted.surface_instance_id.raw(),
                    first.definition_id.raw(),
                    mounted.definition_id.raw(),
                ),
            ));
            continue;
        }

        first_by_surface_id.insert(mounted.surface_instance_id, *mounted);

        if definitions.definition(mounted.definition_id).is_none() {
            report.push(RatificationIssue::error(
                UiSurfaceRatificationCode::UnknownSurfaceDefinition,
                UiSurfaceRatificationSubject::SurfaceDefinition(mounted.definition_id),
                format!(
                    "mounted surface instance {} references unknown surface definition {}",
                    mounted.surface_instance_id.raw(),
                    mounted.definition_id.raw(),
                ),
            ));
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SurfaceDefinition, SurfaceHostInstanceId};

    #[test]
    fn mount_ratifier_accepts_known_unique_mounted_surface_set() {
        let mut definitions = SurfaceDefinitionRegistry::default();
        definitions.register(SurfaceDefinition::new(
            SurfaceDefinitionId::new(1),
            "editor.tool_surface.viewport",
            "Viewport",
        ));

        let mounted = [MountedSurfaceInstance::new(
            SurfaceInstanceId::new(10),
            SurfaceDefinitionId::new(1),
            SurfaceHostInstanceId::new(100),
        )];

        let ratifier = UiSurfaceMountRatifier::new(&definitions);
        let candidate = MountedSurfaceSetCandidate::new(&mounted);
        let report = ratifier.ratify(&candidate);

        assert!(report.is_accepted());
        assert!(report.is_clean());
        assert!(report.is_empty());
    }

    #[test]
    fn mount_ratifier_rejects_unknown_surface_definition() {
        let definitions = SurfaceDefinitionRegistry::default();
        let mounted = [MountedSurfaceInstance::new(
            SurfaceInstanceId::new(10),
            SurfaceDefinitionId::new(99),
            SurfaceHostInstanceId::new(100),
        )];

        let report =
            ratify_mounted_surface_set(MountedSurfaceSetCandidate::new(&mounted), &definitions);

        assert!(report.is_rejected());
        assert!(report.has_blocking_issues());
        assert_eq!(report.len(), 1);

        let issue = &report.issues()[0];
        assert_eq!(
            issue.code(),
            &UiSurfaceRatificationCode::UnknownSurfaceDefinition
        );
        assert_eq!(
            issue.subject(),
            &UiSurfaceRatificationSubject::SurfaceDefinition(SurfaceDefinitionId::new(99))
        );
        assert_eq!(
            issue.severity(),
            foundation_ratification::RatificationSeverity::Error
        );
    }

    #[test]
    fn mount_ratifier_rejects_duplicate_surface_instance_ids() {
        let mut definitions = SurfaceDefinitionRegistry::default();
        definitions.register(SurfaceDefinition::new(
            SurfaceDefinitionId::new(1),
            "editor.tool_surface.outliner",
            "Outliner",
        ));
        definitions.register(SurfaceDefinition::new(
            SurfaceDefinitionId::new(2),
            "editor.tool_surface.viewport",
            "Viewport",
        ));

        let mounted = [
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(1),
                SurfaceHostInstanceId::new(100),
            ),
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(2),
                SurfaceHostInstanceId::new(200),
            ),
        ];

        let report =
            ratify_mounted_surface_set(MountedSurfaceSetCandidate::new(&mounted), &definitions);

        assert!(report.is_rejected());
        assert!(report.has_blocking_issues());
        assert_eq!(report.len(), 1);

        let issue = &report.issues()[0];
        assert_eq!(
            issue.code(),
            &UiSurfaceRatificationCode::DuplicateMountedSurfaceInstance
        );
        assert_eq!(
            issue.subject(),
            &UiSurfaceRatificationSubject::SurfaceInstance(SurfaceInstanceId::new(10))
        );
    }

    #[test]
    fn mount_ratifier_preserves_multiple_blocking_issues_in_order() {
        let definitions = SurfaceDefinitionRegistry::default();
        let mounted = [
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(99),
                SurfaceHostInstanceId::new(100),
            ),
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(100),
                SurfaceHostInstanceId::new(200),
            ),
        ];

        let report =
            ratify_mounted_surface_set(MountedSurfaceSetCandidate::new(&mounted), &definitions);

        assert!(report.is_rejected());
        assert_eq!(report.len(), 2);
        assert_eq!(
            report.issues()[0].code(),
            &UiSurfaceRatificationCode::UnknownSurfaceDefinition
        );
        assert_eq!(
            report.issues()[1].code(),
            &UiSurfaceRatificationCode::DuplicateMountedSurfaceInstance
        );
    }
}
