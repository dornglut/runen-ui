//! Generic Text package validation owner path.

use std::collections::BTreeSet;

use crate::{
    ControlGenericTextOverflowPolicy, ControlPackageDescriptor, ControlPackageValidationDiagnostic,
    ControlPackageValidationReason, ControlPackageValidationReport,
};

pub(super) fn validate_generic_text_descriptors(
    package: &ControlPackageDescriptor,
    control_kind_ids: &BTreeSet<String>,
    report: &mut ControlPackageValidationReport,
) {
    for descriptor in &package.generic_text_descriptors {
        let id = descriptor.control_kind_id.as_str().to_owned();
        if !control_kind_ids.contains(&id) {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::UnresolvedGenericTextDescriptor,
                "generic-text descriptor references no control kind in the package",
            ));
            continue;
        }
        if !descriptor.proof_required {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "generic-text descriptor must require proof evidence",
                report,
            );
        }
        if descriptor.roles.is_empty() {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "generic-text descriptor must declare at least one text role",
                report,
            );
        }
        let mut role_ids = BTreeSet::new();
        for role in &descriptor.roles {
            if role.role_id.trim().is_empty() || !role.role_id.contains('.') {
                report.push(ControlPackageValidationDiagnostic::kind(
                    descriptor.control_kind_id.clone(),
                    ControlPackageValidationReason::InvalidGenericTextRole,
                    "generic-text role id must be non-empty and namespace-like",
                ));
            }
            if !role_ids.insert(role.role_id.clone()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    descriptor.control_kind_id.clone(),
                    ControlPackageValidationReason::InvalidGenericTextRole,
                    format!("duplicate generic-text role id {}", role.role_id),
                ));
            }
            if role.max_lines == Some(0) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    descriptor.control_kind_id.clone(),
                    ControlPackageValidationReason::UnsupportedGenericTextLayoutPolicy,
                    "generic-text role max_lines must be absent or greater than zero",
                ));
            }
        }
        if descriptor.layout_support.renderer_backend_required {
            push_invalid(
                descriptor.control_kind_id.clone(),
                "generic-text descriptor must remain renderer-backend neutral",
                report,
            );
        }
        if descriptor.layout_support.wrap_policies.is_empty()
            || descriptor.layout_support.overflow_policies.is_empty()
            || descriptor.layout_support.alignment_policies.is_empty()
        {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::UnsupportedGenericTextLayoutPolicy,
                "generic-text descriptor must declare wrap, overflow, and alignment support",
            ));
        }
        if descriptor
            .layout_support
            .overflow_policies
            .iter()
            .any(|policy| {
                matches!(
                    *policy,
                    ControlGenericTextOverflowPolicy::StartEllipsisModeled
                        | ControlGenericTextOverflowPolicy::MiddleEllipsisModeled
                )
            })
            && !descriptor.layout_support.glyph_evidence
        {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::UnsupportedGenericTextLayoutPolicy,
                "modeled ellipsis placement requires glyph evidence support",
            ));
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
        ControlPackageValidationReason::InvalidGenericTextDescriptor,
        message,
    ));
}
