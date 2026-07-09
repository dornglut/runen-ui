use std::collections::BTreeSet;

use crate::{
    ControlPackageDescriptor, ControlPackageValidationDiagnostic, ControlPackageValidationReason,
    ControlPackageValidationReport,
};

pub(super) fn validate_overlay_descriptors(
    package: &ControlPackageDescriptor,
    control_kind_ids: &BTreeSet<String>,
    report: &mut ControlPackageValidationReport,
) {
    let mut seen = BTreeSet::new();
    for descriptor in &package.overlay_descriptors {
        let id = descriptor.control_kind_id.as_str().to_owned();
        if !control_kind_ids.contains(&id) {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::UnresolvedOverlayDescriptor,
                "overlay descriptor references no control kind in the package",
            ));
        }
        if !seen.insert(id.clone()) {
            report.push(ControlPackageValidationDiagnostic::package(
                package.package_id.clone(),
                ControlPackageValidationReason::DuplicateOverlayDescriptor,
                format!("duplicate overlay descriptor for control kind {id}"),
            ));
        }
        if descriptor.requirements.is_empty() {
            report.push(ControlPackageValidationDiagnostic::kind(
                descriptor.control_kind_id.clone(),
                ControlPackageValidationReason::InvalidOverlayDescriptor,
                "overlay descriptor must contain at least one requirement",
            ));
        }
        for requirement in &descriptor.requirements {
            if requirement.anchor_role.trim().is_empty()
                || requirement.content_role.trim().is_empty()
            {
                report.push(ControlPackageValidationDiagnostic::kind(
                    descriptor.control_kind_id.clone(),
                    ControlPackageValidationReason::InvalidOverlayDescriptor,
                    "overlay requirement must name non-empty anchor and content roles",
                ));
            }
        }
    }
}
