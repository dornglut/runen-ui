//! Editable-text package validation owner path.

use std::collections::BTreeSet;

use crate::{
    ControlEditableTextMode, ControlPackageDescriptor, ControlPackageValidationDiagnostic,
    ControlPackageValidationReason, ControlPackageValidationReport,
};

pub(super) fn validate_editable_text_descriptors(
    package: &ControlPackageDescriptor,
    control_kind_ids: &BTreeSet<String>,
    report: &mut ControlPackageValidationReport,
) {
    for descriptor in &package.editable_text_descriptors {
        let id = descriptor.control_kind_id.as_str().to_owned();
        if !control_kind_ids.contains(&id) {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::UnresolvedEditableTextDescriptor,
                "editable-text descriptor references no control kind in the package",
            ));
        }
        if package
            .interaction_descriptor(&descriptor.control_kind_id)
            .is_none()
        {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "editable-text descriptor requires a matching interaction descriptor",
                report,
            );
        }
        if descriptor.supported_intents.is_empty() {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "editable-text descriptor must support at least one edit intent",
                report,
            );
        }
        for intent in &descriptor.supported_intents {
            if intent.requires_selection() && !descriptor.selection_policy.supports_ranges() {
                push_invalid(
                    descriptor.control_kind_id.clone(),
                    format!(
                        "intent {} requires range selection support",
                        intent.as_str()
                    ),
                    report,
                );
            }
            if intent.requires_composition() && !descriptor.composition_policy.supports_preedit() {
                push_invalid(
                    descriptor.control_kind_id.clone(),
                    format!(
                        "intent {} requires preedit composition support",
                        intent.as_str()
                    ),
                    report,
                );
            }
            if descriptor.mode == ControlEditableTextMode::ReadOnlySelectable
                && intent.mutates_transient_text()
            {
                push_invalid(
                    descriptor.control_kind_id.clone(),
                    format!(
                        "read-only selectable text must not declare mutating intent {}",
                        intent.as_str()
                    ),
                    report,
                );
            }
        }
        if !descriptor.proof_required {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "editable-text descriptor must require proof evidence",
                report,
            );
        }
        if !descriptor.host_owned_mutation {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "editable-text mutation must remain host-owned",
                report,
            );
        }
    }
}

fn push_invalid(
    control_kind_id: crate::ControlKindId,
    message: impl Into<String>,
    report: &mut ControlPackageValidationReport,
) {
    report.push(ControlPackageValidationDiagnostic::kind(
        control_kind_id,
        ControlPackageValidationReason::InvalidEditableTextDescriptor,
        message,
    ));
}
