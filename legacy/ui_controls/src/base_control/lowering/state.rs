//! File: domain/ui/ui_controls/src/base_control/lowering/state.rs
//! Crate: ui_controls

use crate::{
    ControlHostIntentProposal, ControlKindId, ControlRouteCapabilityDecision,
    ControlStateBindingKind, ControlStateBindingRequirement, ControlStateBucket,
    ControlStateBucketRequirement, ControlStateDescriptor, ControlValidationState,
    RUNENWERK_CONTROL_PACKAGE_ID,
};

use super::super::{ControlDef, ControlPreset};

pub(crate) fn lower_state(def: &ControlDef, kind_id: ControlKindId) -> ControlStateDescriptor {
    let route_id = route_id(def.kind_suffix());
    let descriptor = ControlStateDescriptor::new(kind_id)
        .with_bucket(ControlStateBucketRequirement::new(
            ControlStateBucket::HostFed,
        ))
        .with_bucket(ControlStateBucketRequirement::new(ControlStateBucket::Transient).optional())
        .with_bucket(ControlStateBucketRequirement::new(ControlStateBucket::Focus).optional())
        .with_validation_state(ControlValidationState::Clean)
        .with_validation_state(ControlValidationState::ReadOnly)
        .with_host_intent(
            ControlHostIntentProposal::new(
                format!(
                    "{RUNENWERK_CONTROL_PACKAGE_ID}.{}.host-intent",
                    def.kind_suffix()
                ),
                route_id.clone(),
                def.route_schema_version().value(),
            )
            .with_capability(def.route_capability().as_str()),
        )
        .with_route_decision(ControlRouteCapabilityDecision::not_evaluated(route_id));

    match def.preset() {
        ControlPreset::Label => {
            add_bindings(descriptor, &[("label.text", ControlStateBindingKind::Read)])
        }
        ControlPreset::Button => add_bindings(
            descriptor.with_bucket(
                ControlStateBucketRequirement::new(ControlStateBucket::Preview).optional(),
            ),
            &[("button.activated", ControlStateBindingKind::Option)],
        ),
        ControlPreset::InspectorField => add_bindings(
            descriptor
                .with_bucket(ControlStateBucketRequirement::new(
                    ControlStateBucket::Preview,
                ))
                .with_bucket(ControlStateBucketRequirement::new(
                    ControlStateBucket::Committed,
                ))
                .with_validation_state(ControlValidationState::Dirty)
                .with_validation_state(ControlValidationState::Invalid),
            &[
                ("inspector-field.value", ControlStateBindingKind::Read),
                (
                    "inspector-field.proposed-value",
                    ControlStateBindingKind::Write,
                ),
            ],
        ),
        ControlPreset::ColorPicker => add_bindings(
            descriptor
                .with_bucket(ControlStateBucketRequirement::new(
                    ControlStateBucket::Preview,
                ))
                .with_bucket(ControlStateBucketRequirement::new(
                    ControlStateBucket::Committed,
                ))
                .with_validation_state(ControlValidationState::Dirty),
            &[("color-picker.rgba", ControlStateBindingKind::Write)],
        ),
        ControlPreset::ActionPrompt => add_bindings(
            descriptor.with_validation_state(ControlValidationState::PendingValidation),
            &[("action-prompt.answer", ControlStateBindingKind::Option)],
        ),
        ControlPreset::ListView => add_bindings(
            descriptor,
            &[
                ("list-view.items", ControlStateBindingKind::Collection),
                ("list-view.selection", ControlStateBindingKind::Selection),
            ],
        ),
        ControlPreset::TreeView => add_bindings(
            descriptor,
            &[
                ("tree-view.roots", ControlStateBindingKind::Collection),
                ("tree-view.selection", ControlStateBindingKind::Selection),
            ],
        ),
        ControlPreset::TableView => add_bindings(
            descriptor,
            &[
                ("table-view.rows", ControlStateBindingKind::Collection),
                ("table-view.selection", ControlStateBindingKind::Selection),
            ],
        ),
        ControlPreset::Surface2D => add_bindings(
            descriptor
                .with_bucket(
                    ControlStateBucketRequirement::new(ControlStateBucket::Hover).optional(),
                )
                .with_bucket(
                    ControlStateBucketRequirement::new(ControlStateBucket::Drag).optional(),
                ),
            &[
                ("surface2d.navigation", ControlStateBindingKind::Read),
                (
                    "surface2d.hover-coordinate",
                    ControlStateBindingKind::Option,
                ),
                ("surface2d.selection-box", ControlStateBindingKind::Option),
                ("surface2d.pointer-capture", ControlStateBindingKind::Option),
            ],
        ),
    }
}

fn add_bindings(
    mut descriptor: ControlStateDescriptor,
    values: &[(&str, ControlStateBindingKind)],
) -> ControlStateDescriptor {
    for (binding_id, kind) in values {
        descriptor =
            descriptor.with_binding(ControlStateBindingRequirement::new(*binding_id, *kind));
    }
    descriptor
}

fn route_id(kind_suffix: &str) -> String {
    format!("{RUNENWERK_CONTROL_PACKAGE_ID}.{kind_suffix}.intent")
}
