//! File: domain/ui/ui_surface/src/validation/mount.rs
//! Purpose: Mounted-surface validation diagnostics.

use std::collections::BTreeMap;

use diagnostics::DiagnosticReport;

use crate::{
    MountedSurfaceInstance, SurfaceDefinitionRegistry, SurfaceInstanceId,
    duplicate_mounted_surface_instance_diagnostic, unknown_surface_definition_diagnostic,
};

pub fn validate_mounted_surface_set(
    mounted_surfaces: impl IntoIterator<Item = MountedSurfaceInstance>,
) -> DiagnosticReport {
    let mut report = DiagnosticReport::new();
    let mut first_by_surface_id = BTreeMap::<SurfaceInstanceId, MountedSurfaceInstance>::new();

    for mounted in mounted_surfaces {
        if let Some(first) = first_by_surface_id
            .get(&mounted.surface_instance_id)
            .copied()
        {
            report.push(duplicate_mounted_surface_instance_diagnostic(
                mounted.surface_instance_id,
                first,
                mounted,
            ));

            continue;
        }

        first_by_surface_id.insert(mounted.surface_instance_id, mounted);
    }

    report
}

pub fn validate_mounted_surface_definitions(
    mounted_surfaces: impl IntoIterator<Item = MountedSurfaceInstance>,
    definitions: &SurfaceDefinitionRegistry,
) -> DiagnosticReport {
    let mut report = DiagnosticReport::new();

    for mounted in mounted_surfaces {
        if definitions.definition(mounted.definition_id).is_none() {
            report.push(unknown_surface_definition_diagnostic(mounted));
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SurfaceDefinition, SurfaceDefinitionId, SurfaceHostInstanceId};

    #[test]
    fn validation_reports_duplicate_surface_instance_id() {
        let report = validate_mounted_surface_set([
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
        ]);

        assert_eq!(report.len(), 1);

        let diagnostic = &report.diagnostics()[0];

        assert_eq!(
            diagnostic.code().as_str(),
            "ui_surface.mount.duplicate_surface_instance"
        );
        assert_eq!(diagnostic.domain().as_str(), "ui_surface");
        assert_eq!(diagnostic.severity(), diagnostics::Severity::Error);
        assert_eq!(
            diagnostic.subject().unwrap().kind().as_str(),
            "surface_instance"
        );
        assert_eq!(diagnostic.subject().unwrap().id().unwrap().as_str(), "10");

        let metadata = diagnostic.metadata().entries();

        assert_eq!(metadata[0].key().as_str(), "surface_instance_id");
        assert_eq!(
            metadata[0].value(),
            &diagnostics::DiagnosticMetadataValue::Id("10".to_string())
        );

        assert_eq!(metadata[1].key().as_str(), "first_definition_id");
        assert_eq!(
            metadata[1].value(),
            &diagnostics::DiagnosticMetadataValue::Id("1".to_string())
        );

        assert_eq!(metadata[2].key().as_str(), "duplicate_definition_id");
        assert_eq!(
            metadata[2].value(),
            &diagnostics::DiagnosticMetadataValue::Id("2".to_string())
        );

        assert_eq!(metadata[3].key().as_str(), "first_host_instance_id");
        assert_eq!(
            metadata[3].value(),
            &diagnostics::DiagnosticMetadataValue::Id("100".to_string())
        );

        assert_eq!(metadata[4].key().as_str(), "duplicate_host_instance_id");
        assert_eq!(
            metadata[4].value(),
            &diagnostics::DiagnosticMetadataValue::Id("200".to_string())
        );
    }

    #[test]
    fn validation_allows_unique_surface_instance_ids() {
        let report = validate_mounted_surface_set([
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(1),
                SurfaceHostInstanceId::new(100),
            ),
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(11),
                SurfaceDefinitionId::new(1),
                SurfaceHostInstanceId::new(100),
            ),
        ]);

        assert!(report.is_empty());
    }

    #[test]
    fn validation_reports_each_duplicate_after_first_occurrence() {
        let report = validate_mounted_surface_set([
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
            MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(3),
                SurfaceHostInstanceId::new(300),
            ),
        ]);

        assert_eq!(report.len(), 2);

        assert_eq!(
            report.diagnostics()[0].code().as_str(),
            "ui_surface.mount.duplicate_surface_instance"
        );
        assert_eq!(
            report.diagnostics()[1].code().as_str(),
            "ui_surface.mount.duplicate_surface_instance"
        );
    }

    #[test]
    fn validation_reports_unknown_surface_definition() {
        let mut definitions = SurfaceDefinitionRegistry::default();
        definitions.register(SurfaceDefinition::new(
            SurfaceDefinitionId::new(1),
            "editor.tool_surface.viewport",
            "Viewport",
        ));

        let report = validate_mounted_surface_definitions(
            [
                MountedSurfaceInstance::new(
                    SurfaceInstanceId::new(10),
                    SurfaceDefinitionId::new(1),
                    SurfaceHostInstanceId::new(100),
                ),
                MountedSurfaceInstance::new(
                    SurfaceInstanceId::new(11),
                    SurfaceDefinitionId::new(99),
                    SurfaceHostInstanceId::new(100),
                ),
            ],
            &definitions,
        );

        assert_eq!(report.len(), 1);

        let diagnostic = &report.diagnostics()[0];

        assert_eq!(
            diagnostic.code().as_str(),
            "ui_surface.mount.unknown_definition"
        );
        assert_eq!(diagnostic.domain().as_str(), "ui_surface");
        assert_eq!(diagnostic.severity(), diagnostics::Severity::Error);
        assert_eq!(
            diagnostic.subject().unwrap().kind().as_str(),
            "surface_instance"
        );
        assert_eq!(diagnostic.subject().unwrap().id().unwrap().as_str(), "11");

        let metadata = diagnostic.metadata().entries();

        assert_eq!(metadata[0].key().as_str(), "surface_instance_id");
        assert_eq!(
            metadata[0].value(),
            &diagnostics::DiagnosticMetadataValue::Id("11".to_string())
        );

        assert_eq!(metadata[1].key().as_str(), "definition_id");
        assert_eq!(
            metadata[1].value(),
            &diagnostics::DiagnosticMetadataValue::Id("99".to_string())
        );

        assert_eq!(metadata[2].key().as_str(), "host_instance_id");
        assert_eq!(
            metadata[2].value(),
            &diagnostics::DiagnosticMetadataValue::Id("100".to_string())
        );
    }

    #[test]
    fn validation_allows_known_surface_definitions() {
        let mut definitions = SurfaceDefinitionRegistry::default();
        definitions.register(SurfaceDefinition::new(
            SurfaceDefinitionId::new(1),
            "editor.tool_surface.viewport",
            "Viewport",
        ));

        let report = validate_mounted_surface_definitions(
            [MountedSurfaceInstance::new(
                SurfaceInstanceId::new(10),
                SurfaceDefinitionId::new(1),
                SurfaceHostInstanceId::new(100),
            )],
            &definitions,
        );

        assert!(report.is_empty());
    }
}
