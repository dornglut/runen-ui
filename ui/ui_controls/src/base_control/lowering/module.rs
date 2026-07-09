//! File: domain/ui/ui_controls/src/base_control/lowering/module.rs
//! Crate: ui_controls

use ui_schema::UiSchema;

use crate::{
    ControlKindAuthoringSpec, ControlModuleAuthoringBuilder, ControlModuleDescriptor,
    RUNENWERK_CONTROL_PACKAGE_ID,
};

use super::super::{ControlDef, ControlFieldGroupRole};

pub(crate) fn lower_module(def: &ControlDef) -> ControlModuleDescriptor {
    let mut spec = ControlKindAuthoringSpec::new(
        RUNENWERK_CONTROL_PACKAGE_ID,
        def.kind_suffix().to_owned(),
        def.display_name().to_owned(),
        def.description().to_owned(),
        def.target_profile().clone(),
        lower_schema(def, ControlFieldGroupRole::Properties),
        lower_schema(def, ControlFieldGroupRole::State),
        lower_schema(def, ControlFieldGroupRole::EventPayload),
        def.route_capability().clone(),
    )
    .with_category(def.category())
    .with_mount_ineligible_reason(def.mount_ineligible_reason());
    spec.route_schema_version = def.route_schema_version();

    for tag in def.tags() {
        spec = spec.with_tag(tag.clone());
    }

    ControlModuleAuthoringBuilder::new(spec).build()
}

fn lower_schema(def: &ControlDef, role: ControlFieldGroupRole) -> UiSchema {
    let mut schema = UiSchema::object(
        format!(
            "{RUNENWERK_CONTROL_PACKAGE_ID}.{}.{}",
            def.kind_suffix(),
            role.schema_suffix()
        ),
        1,
    );

    for field in def
        .field_groups()
        .iter()
        .filter(|group| group.role == role)
        .flat_map(|group| group.fields.iter())
    {
        schema = if field.required {
            schema.with_required_field(field.name.clone(), field.shape.clone())
        } else {
            schema.with_optional_field(field.name.clone(), field.shape.clone())
        };
    }

    schema
}
