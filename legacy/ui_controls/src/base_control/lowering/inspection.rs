//! File: domain/ui/ui_controls/src/base_control/lowering/inspection.rs
//! Crate: ui_controls

use crate::catalog::{ControlLayoutInspectionExt, ControlRenderInspectionExt};
use crate::{ControlInspectionDescriptor, ControlPackageDescriptor};

use super::super::compiler::LoweredControl;

pub(crate) fn lower_inspection(
    package: &ControlPackageDescriptor,
    control: &LoweredControl,
) -> ControlInspectionDescriptor {
    let interaction = package
        .interaction_descriptor(&control.module.kind.control_kind_id)
        .unwrap_or(&control.interaction);
    let mut descriptor =
        ControlInspectionDescriptor::from_control_kind(package, &control.module.kind)
            .with_input_summary(&control.input.summary())
            .with_interaction_summary(&interaction.summary())
            .with_state_summary(&control.state.summary())
            .with_theme_summary(&control.theme.summary())
            .with_accessibility_summary(&control.accessibility.summary())
            .with_control_layout_summary(&control.layout.summary())
            .with_control_render_summary(&control.render.summary());
    if let Some(layering) = package.overlay_descriptor(&control.module.kind.control_kind_id) {
        descriptor = descriptor.with_layering_summary(&layering.summary());
    }
    if let Some(generic_text) =
        package.generic_text_descriptor(&control.module.kind.control_kind_id)
    {
        descriptor = descriptor.with_generic_text_summary(&generic_text.summary());
    }
    if let Some(editable_text) =
        package.editable_text_descriptor(&control.module.kind.control_kind_id)
    {
        descriptor = descriptor.with_editable_text_summary(&editable_text.summary());
    }
    descriptor
}
