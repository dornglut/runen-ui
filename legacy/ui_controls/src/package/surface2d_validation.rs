//! Surface2D package validation owner path.

use std::collections::BTreeSet;

use crate::{
    ControlPackageDescriptor, ControlPackageValidationDiagnostic, ControlPackageValidationReason,
    ControlPackageValidationReport, ControlSurface2DBudgetEvidenceKind, ControlSurface2DInputMode,
    ControlSurface2DLayerKind,
};

pub(super) fn validate_surface2d_descriptors(
    package: &ControlPackageDescriptor,
    control_kind_ids: &BTreeSet<String>,
    report: &mut ControlPackageValidationReport,
) {
    for descriptor in &package.surface2d_descriptors {
        let id = descriptor.control_kind_id.as_str().to_owned();
        if !control_kind_ids.contains(&id) {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::UnresolvedSurface2DDescriptor,
                "Surface2D descriptor references no control kind in the package",
            ));
            continue;
        }
        if !descriptor.proof_required {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must require runtime/static proof evidence",
                report,
            );
        }
        if descriptor.renderer_backend_required {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must remain renderer-backend neutral",
                report,
            );
        }
        if descriptor.executes_host_commands {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must not execute host commands",
                report,
            );
        }
        if descriptor.mutates_product_state {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must not mutate product/editor/game state",
                report,
            );
        }
        if descriptor.graph_or_timeline_semantics {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must not own graph or timeline semantics",
                report,
            );
        }
        if !descriptor.accessibility.is_complete() {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must declare complete accessibility acceptance status",
                report,
            );
        }
        if !descriptor.interaction.is_complete() {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must declare complete interaction acceptance status",
                report,
            );
        }
        if descriptor.input_modes.is_empty() {
            push_unsupported(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must declare input mode support/status rows",
                report,
            );
        }
        if descriptor.layer_kinds.is_empty() {
            push_unsupported(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must declare renderer-neutral layer facts",
                report,
            );
        }
        if descriptor.budget_evidence.is_empty() {
            push_unsupported(
                descriptor.control_kind_id.clone(),
                "Surface2D descriptor must declare budget evidence rows",
                report,
            );
        }
        require_input(
            descriptor.control_kind_id.clone(),
            &descriptor.input_modes,
            ControlSurface2DInputMode::PointerCapture,
            "Surface2D descriptor must include pointer capture status",
            report,
        );
        require_input(
            descriptor.control_kind_id.clone(),
            &descriptor.input_modes,
            ControlSurface2DInputMode::KeyboardFitContent,
            "Surface2D descriptor must include keyboard fit-content status",
            report,
        );
        require_layer(
            descriptor.control_kind_id.clone(),
            &descriptor.layer_kinds,
            ControlSurface2DLayerKind::Background,
            "Surface2D descriptor must include background layer fact",
            report,
        );
        require_layer(
            descriptor.control_kind_id.clone(),
            &descriptor.layer_kinds,
            ControlSurface2DLayerKind::Grid,
            "Surface2D descriptor must include grid layer fact",
            report,
        );
        require_layer(
            descriptor.control_kind_id.clone(),
            &descriptor.layer_kinds,
            ControlSurface2DLayerKind::DiagnosticOverlay,
            "Surface2D descriptor must include diagnostic overlay fact",
            report,
        );
        require_budget(
            descriptor.control_kind_id.clone(),
            &descriptor.budget_evidence,
            ControlSurface2DBudgetEvidenceKind::TransformProjection,
            "Surface2D descriptor must include transform projection budget evidence",
            report,
        );
        require_budget(
            descriptor.control_kind_id.clone(),
            &descriptor.budget_evidence,
            ControlSurface2DBudgetEvidenceKind::PrimitiveCount,
            "Surface2D descriptor must include primitive/fact count budget evidence",
            report,
        );
    }
}

fn require_input(
    control_kind_id: crate::ControlKindId,
    modes: &[ControlSurface2DInputMode],
    required: ControlSurface2DInputMode,
    message: &'static str,
    report: &mut ControlPackageValidationReport,
) {
    if !modes.contains(&required) {
        push_unsupported(control_kind_id, message, report);
    }
}

fn require_layer(
    control_kind_id: crate::ControlKindId,
    layers: &[ControlSurface2DLayerKind],
    required: ControlSurface2DLayerKind,
    message: &'static str,
    report: &mut ControlPackageValidationReport,
) {
    if !layers.contains(&required) {
        push_unsupported(control_kind_id, message, report);
    }
}

fn require_budget(
    control_kind_id: crate::ControlKindId,
    budget: &[ControlSurface2DBudgetEvidenceKind],
    required: ControlSurface2DBudgetEvidenceKind,
    message: &'static str,
    report: &mut ControlPackageValidationReport,
) {
    if !budget.contains(&required) {
        push_unsupported(control_kind_id, message, report);
    }
}

fn push_invalid(
    control_kind_id: crate::ControlKindId,
    message: impl Into<String>,
    report: &mut ControlPackageValidationReport,
) {
    report.push(ControlPackageValidationDiagnostic::kind(
        control_kind_id,
        ControlPackageValidationReason::InvalidSurface2DDescriptor,
        message,
    ));
}

fn push_unsupported(
    control_kind_id: crate::ControlKindId,
    message: impl Into<String>,
    report: &mut ControlPackageValidationReport,
) {
    report.push(ControlPackageValidationDiagnostic::kind(
        control_kind_id,
        ControlPackageValidationReason::UnsupportedSurface2DDescriptor,
        message,
    ));
}
