//! File: domain/ui/ui_surface/src/diagnostics/constructors.rs
//! Purpose: ui_surface-owned diagnostic constructors.

use diagnostics::{
    Diagnostic, DiagnosticMessage, DiagnosticMetadataEntry, DiagnosticMetadataKey,
    DiagnosticMetadataValue, DiagnosticSubject, DiagnosticSubjectId, DiagnosticSubjectKind,
    Severity,
};

use crate::{
    DUPLICATE_MOUNTED_SURFACE_INSTANCE_CODE, MISSING_CAPABILITY_CODE, MountedSurfaceInstance,
    SurfaceCapability, SurfaceInstanceId, UI_SURFACE_DIAGNOSTIC_DOMAIN,
    UNKNOWN_SURFACE_DEFINITION_CODE,
};

pub fn missing_capability_diagnostic(
    surface_instance_id: SurfaceInstanceId,
    required_capability: SurfaceCapability,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        MISSING_CAPABILITY_CODE,
        UI_SURFACE_DIAGNOSTIC_DOMAIN,
        DiagnosticMessage::from_static(
            "Surface intent required a capability the adapter does not provide.",
        ),
    )
    .with_subject(surface_instance_subject(surface_instance_id))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("required_capability")
            .expect("required_capability metadata key should be valid"),
        DiagnosticMetadataValue::string(surface_capability_name(required_capability)),
    ))
}

pub fn duplicate_mounted_surface_instance_diagnostic(
    surface_instance_id: SurfaceInstanceId,
    first: MountedSurfaceInstance,
    duplicate: MountedSurfaceInstance,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        DUPLICATE_MOUNTED_SURFACE_INSTANCE_CODE,
        UI_SURFACE_DIAGNOSTIC_DOMAIN,
        DiagnosticMessage::from_static(
            "Mounted surface set contains duplicate surface instance ids.",
        ),
    )
    .with_subject(surface_instance_subject(surface_instance_id))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("surface_instance_id")
            .expect("surface_instance_id metadata key should be valid"),
        DiagnosticMetadataValue::id(surface_instance_id.raw().to_string()),
    ))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("first_definition_id")
            .expect("first_definition_id metadata key should be valid"),
        DiagnosticMetadataValue::id(first.definition_id.raw().to_string()),
    ))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("duplicate_definition_id")
            .expect("duplicate_definition_id metadata key should be valid"),
        DiagnosticMetadataValue::id(duplicate.definition_id.raw().to_string()),
    ))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("first_host_instance_id")
            .expect("first_host_instance_id metadata key should be valid"),
        DiagnosticMetadataValue::id(first.host_instance_id.raw().to_string()),
    ))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("duplicate_host_instance_id")
            .expect("duplicate_host_instance_id metadata key should be valid"),
        DiagnosticMetadataValue::id(duplicate.host_instance_id.raw().to_string()),
    ))
}

pub fn unknown_surface_definition_diagnostic(mounted: MountedSurfaceInstance) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        UNKNOWN_SURFACE_DEFINITION_CODE,
        UI_SURFACE_DIAGNOSTIC_DOMAIN,
        DiagnosticMessage::from_static("Mounted surface references an unknown surface definition."),
    )
    .with_subject(surface_instance_subject(mounted.surface_instance_id))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("surface_instance_id")
            .expect("surface_instance_id metadata key should be valid"),
        DiagnosticMetadataValue::id(mounted.surface_instance_id.raw().to_string()),
    ))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("definition_id")
            .expect("definition_id metadata key should be valid"),
        DiagnosticMetadataValue::id(mounted.definition_id.raw().to_string()),
    ))
    .with_metadata(DiagnosticMetadataEntry::new(
        DiagnosticMetadataKey::from_static("host_instance_id")
            .expect("host_instance_id metadata key should be valid"),
        DiagnosticMetadataValue::id(mounted.host_instance_id.raw().to_string()),
    ))
}

pub const fn surface_capability_name(capability: SurfaceCapability) -> &'static str {
    match capability {
        SurfaceCapability::Observe => "observe",
        SurfaceCapability::Interact => "interact",
        SurfaceCapability::RequestMutation => "request_mutation",
        SurfaceCapability::Ratify => "ratify",
    }
}

fn surface_instance_subject(surface_instance_id: SurfaceInstanceId) -> DiagnosticSubject {
    DiagnosticSubject::new(
        DiagnosticSubjectKind::from_static("surface_instance")
            .expect("surface_instance subject kind should be valid"),
    )
    .with_id(
        DiagnosticSubjectId::new(surface_instance_id.raw().to_string())
            .expect("surface instance id should be valid diagnostic subject id"),
    )
}
