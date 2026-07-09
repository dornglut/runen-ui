//! File: domain/ui/ui_program_lowering/src/entrypoint.rs
//! Crate: ui_program_lowering
//!
//! Public UiProgram formation entrypoints.

use ui_definition::{AuthoredUiNodePath, UiNodeDefinition};
use ui_program::{
    UiProgram, UiProgramId, UiProgramSource, UiProgramSourceId, UiProgramSourceMapEntry,
    UiProgramTargetId, UiProgramVersion,
};

use crate::catalog::UiProgramFormationControlCatalog;
use crate::lower::lower_control_nodes;
use crate::report::UiProgramFormationReport;

pub fn form_ui_program_report_from_node_with_registry_snapshot(
    program_id: impl Into<String>,
    source_id: impl Into<String>,
    root: &UiNodeDefinition,
    snapshot: &ui_controls::ControlPackageRegistrySnapshot,
) -> UiProgramFormationReport {
    let catalog_report =
        UiProgramFormationControlCatalog::derive_from_control_package_registry_snapshot(snapshot);

    let program =
        form_program_from_node_with_catalog(program_id, source_id, root, &catalog_report.catalog);

    UiProgramFormationReport::from_program_and_catalog_report(program, catalog_report)
}

fn form_program_from_node_with_catalog(
    program_id: impl Into<String>,
    source_id: impl Into<String>,
    root: &UiNodeDefinition,
    catalog: &UiProgramFormationControlCatalog,
) -> UiProgram {
    let program_id = program_id.into();
    let source_id = source_id.into();

    let mut program = UiProgram::new(
        UiProgramId::new(program_id.clone()),
        UiProgramVersion::new(1),
    )
    .with_source(UiProgramSource::authored(
        UiProgramSourceId::new(source_id.clone()),
        "authored UI definition",
    ))
    .with_source_map_entry(UiProgramSourceMapEntry::new(
        UiProgramSourceId::new(source_id),
        UiProgramTargetId::new(format!("{program_id}.root")),
    ));

    lower_control_nodes(
        root,
        &AuthoredUiNodePath::root(root.id()),
        catalog,
        &mut program,
    );

    program
}
