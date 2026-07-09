use ui_controls::{
    ControlCatalogMetadata, ControlKindAuthoringSpec, ControlModuleAuthoringBuilder,
    ControlMountEligibility, ControlPackageAuthoringBuilder, ControlPackageRegistry,
    ControlPackageRegistryError, ControlPackageValidationReason, ControlPackageVersion,
    ControlTargetProfileRef,
};
use ui_program::RouteCapability;
use ui_schema::{UiSchema, UiSchemaShape};

const PACKAGE_ID: &str = "runenwerk.ui.controls.authoring_test";
const TARGET_PROFILE: &str = "runenwerk.ui.target.editor";

#[test]
fn control_authoring_builds_valid_control_module() {
    let module = ControlModuleAuthoringBuilder::new(authored_spec("button")).build();
    let report = module.validate_contract();

    assert!(report.is_valid(), "{:?}", report.diagnostics);
    assert_eq!(module.schemas.len(), 3);
    assert_eq!(module.kernels.len(), 5);
    assert_eq!(module.fixtures.len(), 1);
    assert_eq!(module.diagnostics.len(), 1);
    assert_eq!(module.migrations.len(), 1);
    assert_eq!(module.stories.len(), 1);
}

#[test]
fn control_authoring_builds_valid_package() {
    let package = authored_package();
    let report = package.validate_contract();

    assert!(report.is_valid(), "{:?}", report.diagnostics);
    assert_eq!(package.control_kinds.len(), 1);
    assert_eq!(package.property_schemas.len(), 1);
    assert_eq!(package.state_schemas.len(), 1);
    assert_eq!(package.event_payload_schemas.len(), 1);
    assert_eq!(package.kernels.len(), 5);
    assert_eq!(package.fixtures.len(), 1);
    assert_eq!(package.diagnostics.len(), 1);
    assert_eq!(package.migrations.len(), 1);
    assert_eq!(package.stories.len(), 1);
}

#[test]
fn control_authoring_output_remains_not_mount_eligible() {
    let package = authored_package();
    let kind = &package.control_kinds[0];

    match &kind.mount_eligibility {
        ControlMountEligibility::NotEligible { reason } => {
            assert!(reason.contains("runtime mount eligibility requires future story"));
        }
        ControlMountEligibility::RequiresEvidence { .. } => {
            panic!("authoring kit must not claim mount eligibility");
        }
    }

    assert!(!kind.compatibility.supports_runtime_mount);
}

#[test]
fn control_authoring_invalid_output_still_fails_closed() {
    let mut package = authored_package();
    package.property_schemas.clear();

    let report = package.validate_contract();
    assert!(!report.is_valid());
    assert!(report.has_reason(ControlPackageValidationReason::MissingSchema));

    let mut registry = ControlPackageRegistry::new();
    assert!(matches!(
        registry.register(package),
        Err(ControlPackageRegistryError::InvalidPackage { .. })
    ));
}

fn authored_package() -> ui_controls::ControlPackageDescriptor {
    ControlPackageAuthoringBuilder::new(PACKAGE_ID, ControlPackageVersion::new(1))
        .with_display_name("Authoring Test Controls")
        .with_description("Descriptor-only controls assembled through the authoring kit.")
        .with_category("authoring-test")
        .with_tag("control-package")
        .with_target_profile(ControlTargetProfileRef::new(TARGET_PROFILE))
        .with_catalog_metadata(ControlCatalogMetadata::new(PACKAGE_ID, "Authoring Test"))
        .with_authored_kind(authored_spec("button"))
        .build()
}

fn authored_spec(kind_suffix: &str) -> ControlKindAuthoringSpec {
    ControlKindAuthoringSpec::new(
        PACKAGE_ID,
        kind_suffix,
        "Authoring Test Button",
        "Reusable descriptor-only button authored through the authoring kit.",
        ControlTargetProfileRef::new(TARGET_PROFILE),
        UiSchema::object(format!("{PACKAGE_ID}.{kind_suffix}.properties"), 1)
            .with_required_field("label", UiSchemaShape::String),
        UiSchema::object(format!("{PACKAGE_ID}.{kind_suffix}.state"), 1)
            .with_optional_field("disabled", UiSchemaShape::Bool),
        UiSchema::object(format!("{PACKAGE_ID}.{kind_suffix}.event.activate"), 1)
            .with_required_field("route", UiSchemaShape::RouteRef),
        RouteCapability::new("runenwerk.ui.authoring.write"),
    )
    .with_tag("authoring-test")
}
