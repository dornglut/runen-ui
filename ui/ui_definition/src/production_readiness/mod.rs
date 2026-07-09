//! Runtime-neutral production readiness evidence and inspection contracts.

use crate::{UiDefinitionDiagnosticSeverity, UiSourceLocation, identity::AuthoredId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type UiReadinessPacketId = AuthoredId;
pub type UiReadinessRequestId = AuthoredId;
pub type UiReadinessInspectionReportId = AuthoredId;
pub type UiReadinessArtifactId = AuthoredId;
pub type UiReadinessDocumentId = AuthoredId;
pub type UiReadinessTargetProfileId = AuthoredId;
pub type UiReadinessSourcePackageId = AuthoredId;
pub type UiReadinessDiagnosticRef = AuthoredId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiReadinessEvidenceKind {
    ProjectionSnapshot,
    DiagnosticInspection,
    AccessibilityReport,
    CompatibilityReport,
    PerformanceBudgetReport,
    GoldenArtifact,
    ExampleScenario,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiReadinessDiagnosticGroup {
    Composition,
    Style,
    Recipe,
    Binding,
    Preview,
    Persistence,
    Accessibility,
    Compatibility,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiReadinessCompatibilityAxis {
    SafeArea,
    InputModality,
    PlatformPrompt,
    LocalizationTextExpansion,
    Accessibility,
    Size,
    PerformanceReadability,
    ViewModelFreshness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiReadinessValidationMode {
    Inspect,
    DryRun,
    ReleaseCandidate,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiReadinessArtifactFreshness {
    Fresh,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiReadinessArtifactOwnership {
    ExternalReference,
    OwnsConcreteArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiReadinessActivationImpact {
    None,
    PreviewOnly,
    BlocksActivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiReadinessDiagnosticDomain {
    UiDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessEvidenceArtifact {
    pub id: UiReadinessArtifactId,
    pub kind: UiReadinessEvidenceKind,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiReadinessTargetProfileId>,
    pub freshness: UiReadinessArtifactFreshness,
    pub ownership: UiReadinessArtifactOwnership,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessEvidencePacket {
    pub id: UiReadinessPacketId,
    pub document: UiReadinessDocumentId,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiReadinessTargetProfileId>,
    #[serde(default)]
    pub compatibility_axes: BTreeSet<UiReadinessCompatibilityAxis>,
    #[serde(default)]
    pub required_evidence: BTreeSet<UiReadinessEvidenceKind>,
    #[serde(default)]
    pub artifacts: Vec<UiReadinessEvidenceArtifact>,
    #[serde(default)]
    pub expected_diagnostics: BTreeSet<UiReadinessDiagnosticRef>,
    pub source_package: UiReadinessSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessInspectionReport {
    pub id: UiReadinessInspectionReportId,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiReadinessTargetProfileId>,
    #[serde(default)]
    pub compatibility_axes: BTreeSet<UiReadinessCompatibilityAxis>,
    #[serde(default)]
    pub diagnostic_groups: BTreeSet<UiReadinessDiagnosticGroup>,
    pub source_package: UiReadinessSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessRequest {
    pub id: UiReadinessRequestId,
    pub evidence_packet: UiReadinessPacketId,
    #[serde(default)]
    pub inspection_report: Option<UiReadinessInspectionReportId>,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiReadinessTargetProfileId>,
    #[serde(default)]
    pub required_evidence: BTreeSet<UiReadinessEvidenceKind>,
    #[serde(default)]
    pub expected_diagnostics: BTreeSet<UiReadinessDiagnosticRef>,
    pub source_package: UiReadinessSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessLibrary {
    #[serde(default)]
    pub evidence_packets: Vec<UiReadinessEvidencePacket>,
    #[serde(default)]
    pub inspection_reports: Vec<UiReadinessInspectionReport>,
    #[serde(default)]
    pub readiness_requests: Vec<UiReadinessRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessValidationRequest {
    pub target_profile: UiReadinessTargetProfileId,
    pub mode: UiReadinessValidationMode,
    #[serde(default)]
    pub actual_diagnostics: BTreeSet<UiReadinessDiagnosticRef>,
}

impl UiReadinessValidationRequest {
    pub fn production(target_profile: impl Into<UiReadinessTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiReadinessValidationMode::Production,
            actual_diagnostics: BTreeSet::new(),
        }
    }

    pub fn inspect(target_profile: impl Into<UiReadinessTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiReadinessValidationMode::Inspect,
            actual_diagnostics: BTreeSet::new(),
        }
    }

    pub fn with_actual_diagnostic(
        mut self,
        diagnostic: impl Into<UiReadinessDiagnosticRef>,
    ) -> Self {
        self.actual_diagnostics.insert(diagnostic.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub evidence_packet: Option<UiReadinessPacketId>,
    #[serde(default)]
    pub readiness_request: Option<UiReadinessRequestId>,
    #[serde(default)]
    pub inspection_report: Option<UiReadinessInspectionReportId>,
    #[serde(default)]
    pub artifact: Option<UiReadinessArtifactId>,
    #[serde(default)]
    pub evidence_kind: Option<UiReadinessEvidenceKind>,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profile: Option<UiReadinessTargetProfileId>,
    pub owning_domain: UiReadinessDiagnosticDomain,
    #[serde(default)]
    pub source_package: Option<UiReadinessSourcePackageId>,
    #[serde(default)]
    pub expected_diagnostics: Vec<UiReadinessDiagnosticRef>,
    #[serde(default)]
    pub actual_diagnostics: Vec<UiReadinessDiagnosticRef>,
    pub activation_impact: UiReadinessActivationImpact,
    pub suggested_fix: String,
}

impl UiReadinessDiagnostic {
    fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            evidence_packet: None,
            readiness_request: None,
            inspection_report: None,
            artifact: None,
            evidence_kind: None,
            source_location: None,
            target_profile: None,
            owning_domain: UiReadinessDiagnosticDomain::UiDefinition,
            source_package: None,
            expected_diagnostics: Vec::new(),
            actual_diagnostics: Vec::new(),
            activation_impact: UiReadinessActivationImpact::BlocksActivation,
            suggested_fix: suggested_fix.into(),
        }
    }

    fn for_packet(mut self, packet: &UiReadinessEvidencePacket) -> Self {
        self.evidence_packet = Some(packet.id.clone());
        self.source_location = packet.source_location.clone();
        self.source_package = Some(packet.source_package.clone());
        self.expected_diagnostics = packet.expected_diagnostics.iter().cloned().collect();
        self
    }

    fn for_inspection(mut self, report: &UiReadinessInspectionReport) -> Self {
        self.inspection_report = Some(report.id.clone());
        self.source_location = report.source_location.clone();
        self.source_package = Some(report.source_package.clone());
        self
    }

    fn for_request(mut self, request: &UiReadinessRequest) -> Self {
        self.readiness_request = Some(request.id.clone());
        self.evidence_packet = Some(request.evidence_packet.clone());
        self.source_location = request.source_location.clone();
        self.source_package = Some(request.source_package.clone());
        self.expected_diagnostics = request.expected_diagnostics.iter().cloned().collect();
        self
    }

    fn with_artifact(mut self, artifact: &UiReadinessEvidenceArtifact) -> Self {
        self.artifact = Some(artifact.id.clone());
        self.evidence_kind = Some(artifact.kind);
        self
    }

    fn with_evidence_kind(mut self, kind: UiReadinessEvidenceKind) -> Self {
        self.evidence_kind = Some(kind);
        self
    }

    fn with_target_profile(mut self, target_profile: UiReadinessTargetProfileId) -> Self {
        self.target_profile = Some(target_profile);
        self
    }

    fn with_diagnostic_mismatch(
        mut self,
        expected: Vec<UiReadinessDiagnosticRef>,
        actual: Vec<UiReadinessDiagnosticRef>,
    ) -> Self {
        self.expected_diagnostics = expected;
        self.actual_diagnostics = actual;
        self
    }

    fn preview_only(mut self) -> Self {
        self.activation_impact = UiReadinessActivationImpact::PreviewOnly;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiReadinessValidationReport {
    #[serde(default)]
    pub diagnostics: Vec<UiReadinessDiagnostic>,
}

impl UiReadinessValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }
}

pub fn game_runtime_required_compatibility_axes() -> BTreeSet<UiReadinessCompatibilityAxis> {
    BTreeSet::from([
        UiReadinessCompatibilityAxis::SafeArea,
        UiReadinessCompatibilityAxis::InputModality,
        UiReadinessCompatibilityAxis::PlatformPrompt,
        UiReadinessCompatibilityAxis::LocalizationTextExpansion,
        UiReadinessCompatibilityAxis::Accessibility,
        UiReadinessCompatibilityAxis::Size,
        UiReadinessCompatibilityAxis::PerformanceReadability,
        UiReadinessCompatibilityAxis::ViewModelFreshness,
    ])
}

pub fn validate_production_readiness(
    library: &UiReadinessLibrary,
    request: &UiReadinessValidationRequest,
) -> UiReadinessValidationReport {
    let mut diagnostics = Vec::new();
    let packets = index_packets(library, request, &mut diagnostics);
    let inspections = index_inspections(library, request, &mut diagnostics);
    let readiness_requests = index_requests(library, request, &mut diagnostics);

    let mut validator = ProductionReadinessValidator {
        request,
        diagnostics,
    };

    validator.validate_packets(&packets);
    validator.validate_inspections(&inspections);
    validator.validate_readiness_requests(&packets, &inspections, &readiness_requests);

    UiReadinessValidationReport {
        diagnostics: validator.diagnostics,
    }
}

fn index_packets<'a>(
    library: &'a UiReadinessLibrary,
    request: &UiReadinessValidationRequest,
    diagnostics: &mut Vec<UiReadinessDiagnostic>,
) -> BTreeMap<UiReadinessPacketId, &'a UiReadinessEvidencePacket> {
    let mut packets = BTreeMap::new();
    for packet in &library.evidence_packets {
        if packets.insert(packet.id.clone(), packet).is_some() {
            diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.packet.duplicate_id",
                    format!(
                        "Readiness evidence packet '{}' is declared more than once.",
                        packet.id
                    ),
                    "Keep one readiness evidence packet for each stable packet id.",
                )
                .for_packet(packet)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    packets
}

fn index_inspections<'a>(
    library: &'a UiReadinessLibrary,
    request: &UiReadinessValidationRequest,
    diagnostics: &mut Vec<UiReadinessDiagnostic>,
) -> BTreeMap<UiReadinessInspectionReportId, &'a UiReadinessInspectionReport> {
    let mut inspections = BTreeMap::new();
    for report in &library.inspection_reports {
        if inspections.insert(report.id.clone(), report).is_some() {
            diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.inspection.duplicate_id",
                    format!(
                        "Inspection report '{}' is declared more than once.",
                        report.id
                    ),
                    "Keep one inspection report for each stable report id.",
                )
                .for_inspection(report)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    inspections
}

fn index_requests<'a>(
    library: &'a UiReadinessLibrary,
    request: &UiReadinessValidationRequest,
    diagnostics: &mut Vec<UiReadinessDiagnostic>,
) -> BTreeMap<UiReadinessRequestId, &'a UiReadinessRequest> {
    let mut requests = BTreeMap::new();
    for readiness_request in &library.readiness_requests {
        if requests
            .insert(readiness_request.id.clone(), readiness_request)
            .is_some()
        {
            diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.request.duplicate_id",
                    format!(
                        "Readiness request '{}' is declared more than once.",
                        readiness_request.id
                    ),
                    "Keep one readiness request for each stable request id.",
                )
                .for_request(readiness_request)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    requests
}

struct ProductionReadinessValidator<'a> {
    request: &'a UiReadinessValidationRequest,
    diagnostics: Vec<UiReadinessDiagnostic>,
}

impl ProductionReadinessValidator<'_> {
    fn validate_packets(
        &mut self,
        packets: &BTreeMap<UiReadinessPacketId, &UiReadinessEvidencePacket>,
    ) {
        for packet in packets.values() {
            self.validate_packet_target_profile(packet);
            self.validate_packet_game_runtime_axis_coverage(packet);
            self.validate_required_evidence(packet, &packet.required_evidence);
            self.validate_packet_artifacts(packet);
            self.validate_packet_expected_diagnostics(packet);
            self.validate_packet_preview_only(packet);
        }
    }

    fn validate_packet_target_profile(&mut self, packet: &UiReadinessEvidencePacket) {
        if !packet.target_profiles.is_empty()
            && !packet
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.packet.target_profile_unsupported",
                    format!(
                        "Readiness evidence packet '{}' does not support target profile '{}'.",
                        packet.id, self.request.target_profile
                    ),
                    "Add the target profile to the packet or validate with a compatible target profile.",
                )
                .for_packet(packet)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_required_evidence(
        &mut self,
        packet: &UiReadinessEvidencePacket,
        required_evidence: &BTreeSet<UiReadinessEvidenceKind>,
    ) {
        let present: BTreeSet<_> = packet
            .artifacts
            .iter()
            .map(|artifact| artifact.kind)
            .collect();
        for kind in required_evidence.difference(&present) {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.evidence.missing",
                    format!(
                        "Readiness evidence packet '{}' is missing required evidence '{kind:?}'.",
                        packet.id
                    ),
                    "Attach the required evidence descriptor before production readiness.",
                )
                .for_packet(packet)
                .with_evidence_kind(*kind)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_packet_game_runtime_axis_coverage(&mut self, packet: &UiReadinessEvidencePacket) {
        if self.request.target_profile.as_str() != "game.runtime" {
            return;
        }

        let applies_to_runtime = packet.target_profiles.is_empty()
            || packet
                .target_profiles
                .contains(&self.request.target_profile);
        if !applies_to_runtime {
            return;
        }

        let required = game_runtime_required_compatibility_axes();
        if !required.is_subset(&packet.compatibility_axes) {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.compatibility_axis.missing",
                    format!(
                        "Readiness evidence packet '{}' does not cover every game.runtime compatibility axis.",
                        packet.id
                    ),
                    "Attach safe-area, input, platform-prompt, localization, accessibility, size, performance, and view-model freshness compatibility axes.",
                )
                .for_packet(packet)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_packet_artifacts(&mut self, packet: &UiReadinessEvidencePacket) {
        for artifact in &packet.artifacts {
            if !artifact.target_profiles.is_empty()
                && !artifact
                    .target_profiles
                    .contains(&self.request.target_profile)
            {
                self.diagnostics.push(
                    UiReadinessDiagnostic::error(
                        "ui.readiness.artifact.target_profile_unsupported",
                        format!(
                            "Readiness artifact '{}' does not support target profile '{}'.",
                            artifact.id, self.request.target_profile
                        ),
                        "Attach target-profile-compatible evidence before production readiness.",
                    )
                    .for_packet(packet)
                    .with_artifact(artifact)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            if artifact.freshness == UiReadinessArtifactFreshness::Stale
                && self.request.mode != UiReadinessValidationMode::Inspect
            {
                self.diagnostics.push(
                    UiReadinessDiagnostic::error(
                        "ui.readiness.artifact.stale",
                        format!("Readiness artifact '{}' is stale.", artifact.id),
                        "Regenerate the evidence artifact or use inspect-only mode.",
                    )
                    .for_packet(packet)
                    .with_artifact(artifact)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            if artifact.ownership == UiReadinessArtifactOwnership::OwnsConcreteArtifact {
                self.diagnostics.push(
                    UiReadinessDiagnostic::error(
                        "ui.readiness.artifact.ownership_violation",
                        format!(
                            "Readiness artifact '{}' claims concrete artifact ownership.",
                            artifact.id
                        ),
                        "Store concrete artifacts in the owning app, runtime, renderer, provider, accessibility, or performance system and reference them from UI definitions.",
                    )
                    .for_packet(packet)
                    .with_artifact(artifact)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }

    fn validate_packet_expected_diagnostics(&mut self, packet: &UiReadinessEvidencePacket) {
        if packet.expected_diagnostics != self.request.actual_diagnostics {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.packet.expected_diagnostics_mismatch",
                    format!(
                        "Readiness evidence packet '{}' expected diagnostics do not match actual diagnostics.",
                        packet.id
                    ),
                    "Update expected diagnostics or fix the evidence that produced different diagnostics.",
                )
                .for_packet(packet)
                .with_target_profile(self.request.target_profile.clone())
                .with_diagnostic_mismatch(
                    packet.expected_diagnostics.iter().cloned().collect(),
                    self.request.actual_diagnostics.iter().cloned().collect(),
                ),
            );
        }
    }

    fn validate_packet_preview_only(&mut self, packet: &UiReadinessEvidencePacket) {
        if packet.preview_only && self.request.mode == UiReadinessValidationMode::Production {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.packet.preview_only_production",
                    format!(
                        "Readiness evidence packet '{}' is preview-only and cannot pass production readiness.",
                        packet.id
                    ),
                    "Use inspect or dry-run validation, or remove the preview-only flag before production readiness.",
                )
                .for_packet(packet)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }

    fn validate_inspections(
        &mut self,
        inspections: &BTreeMap<UiReadinessInspectionReportId, &UiReadinessInspectionReport>,
    ) {
        for inspection in inspections.values() {
            if !inspection.target_profiles.is_empty()
                && !inspection
                    .target_profiles
                    .contains(&self.request.target_profile)
            {
                self.diagnostics.push(
                    UiReadinessDiagnostic::error(
                        "ui.readiness.inspection.target_profile_unsupported",
                        format!(
                            "Inspection report '{}' does not support target profile '{}'.",
                            inspection.id, self.request.target_profile
                        ),
                        "Attach a target-profile-compatible inspection report.",
                    )
                    .for_inspection(inspection)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            self.validate_inspection_game_runtime_axis_coverage(inspection);

            if inspection.preview_only && self.request.mode == UiReadinessValidationMode::Production
            {
                self.diagnostics.push(
                    UiReadinessDiagnostic::error(
                        "ui.readiness.inspection.preview_only_production",
                        format!(
                            "Inspection report '{}' is preview-only and cannot pass production readiness.",
                            inspection.id
                        ),
                        "Use inspect or dry-run validation, or regenerate a production-capable inspection report.",
                    )
                    .for_inspection(inspection)
                    .with_target_profile(self.request.target_profile.clone())
                    .preview_only(),
                );
            }
        }
    }

    fn validate_inspection_game_runtime_axis_coverage(
        &mut self,
        inspection: &UiReadinessInspectionReport,
    ) {
        if self.request.target_profile.as_str() != "game.runtime" {
            return;
        }

        let applies_to_runtime = inspection.target_profiles.is_empty()
            || inspection
                .target_profiles
                .contains(&self.request.target_profile);
        if !applies_to_runtime {
            return;
        }

        let required = game_runtime_required_compatibility_axes();
        if !required.is_subset(&inspection.compatibility_axes) {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.inspection.compatibility_axis.missing",
                    format!(
                        "Inspection report '{}' does not cover every game.runtime compatibility axis.",
                        inspection.id
                    ),
                    "Attach safe-area, input, platform-prompt, localization, accessibility, size, performance, and view-model freshness compatibility axes.",
                )
                .for_inspection(inspection)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_readiness_requests(
        &mut self,
        packets: &BTreeMap<UiReadinessPacketId, &UiReadinessEvidencePacket>,
        inspections: &BTreeMap<UiReadinessInspectionReportId, &UiReadinessInspectionReport>,
        requests: &BTreeMap<UiReadinessRequestId, &UiReadinessRequest>,
    ) {
        for readiness_request in requests.values() {
            let packet = match packets.get(&readiness_request.evidence_packet) {
                Some(packet) => *packet,
                None => {
                    self.diagnostics.push(
                        UiReadinessDiagnostic::error(
                            "ui.readiness.request.packet_unknown",
                            format!(
                                "Readiness request '{}' references unknown evidence packet '{}'.",
                                readiness_request.id, readiness_request.evidence_packet
                            ),
                            "Create the referenced evidence packet or update the readiness request.",
                        )
                        .for_request(readiness_request)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                    continue;
                }
            };

            self.validate_request_target_profile(readiness_request);
            self.validate_required_evidence(packet, &readiness_request.required_evidence);
            self.validate_request_inspection_report(readiness_request, inspections);
            self.validate_request_expected_diagnostics(readiness_request);
            self.validate_request_preview_only(readiness_request);
        }
    }

    fn validate_request_target_profile(&mut self, readiness_request: &UiReadinessRequest) {
        if !readiness_request.target_profiles.is_empty()
            && !readiness_request
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.request.target_profile_unsupported",
                    format!(
                        "Readiness request '{}' does not support target profile '{}'.",
                        readiness_request.id, self.request.target_profile
                    ),
                    "Add the target profile to the readiness request or validate with a compatible target profile.",
                )
                .for_request(readiness_request)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_request_inspection_report(
        &mut self,
        readiness_request: &UiReadinessRequest,
        inspections: &BTreeMap<UiReadinessInspectionReportId, &UiReadinessInspectionReport>,
    ) {
        let Some(inspection_id) = &readiness_request.inspection_report else {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.request.inspection_missing",
                    format!(
                        "Readiness request '{}' does not reference a diagnostic inspection report.",
                        readiness_request.id
                    ),
                    "Attach a diagnostic inspection report before production readiness.",
                )
                .for_request(readiness_request)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        if !inspections.contains_key(inspection_id) {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.request.inspection_unknown",
                    format!(
                        "Readiness request '{}' references unknown inspection report '{}'.",
                        readiness_request.id, inspection_id
                    ),
                    "Create the referenced inspection report or update the readiness request.",
                )
                .for_request(readiness_request)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_request_expected_diagnostics(&mut self, readiness_request: &UiReadinessRequest) {
        if readiness_request.expected_diagnostics != self.request.actual_diagnostics {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.request.expected_diagnostics_mismatch",
                    format!(
                        "Readiness request '{}' expected diagnostics do not match actual diagnostics.",
                        readiness_request.id
                    ),
                    "Update expected diagnostics or fix the evidence that produced different diagnostics.",
                )
                .for_request(readiness_request)
                .with_target_profile(self.request.target_profile.clone())
                .with_diagnostic_mismatch(
                    readiness_request.expected_diagnostics.iter().cloned().collect(),
                    self.request.actual_diagnostics.iter().cloned().collect(),
                ),
            );
        }
    }

    fn validate_request_preview_only(&mut self, readiness_request: &UiReadinessRequest) {
        if readiness_request.preview_only
            && self.request.mode == UiReadinessValidationMode::Production
        {
            self.diagnostics.push(
                UiReadinessDiagnostic::error(
                    "ui.readiness.request.preview_only_production",
                    format!(
                        "Readiness request '{}' is preview-only and cannot pass production readiness.",
                        readiness_request.id
                    ),
                    "Use inspect or dry-run validation, or remove the preview-only flag before production readiness.",
                )
                .for_request(readiness_request)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> AuthoredId {
        AuthoredId::from(value)
    }

    fn target_profiles(values: &[&str]) -> BTreeSet<UiReadinessTargetProfileId> {
        values.iter().map(|value| id(value)).collect()
    }

    fn source_package() -> UiReadinessSourcePackageId {
        id("ui.package.core")
    }

    fn required_evidence() -> BTreeSet<UiReadinessEvidenceKind> {
        BTreeSet::from([
            UiReadinessEvidenceKind::ProjectionSnapshot,
            UiReadinessEvidenceKind::DiagnosticInspection,
            UiReadinessEvidenceKind::AccessibilityReport,
            UiReadinessEvidenceKind::CompatibilityReport,
            UiReadinessEvidenceKind::PerformanceBudgetReport,
            UiReadinessEvidenceKind::GoldenArtifact,
            UiReadinessEvidenceKind::ExampleScenario,
        ])
    }

    fn compatibility_axes() -> BTreeSet<UiReadinessCompatibilityAxis> {
        game_runtime_required_compatibility_axes()
    }

    fn artifact(kind: UiReadinessEvidenceKind, suffix: &str) -> UiReadinessEvidenceArtifact {
        UiReadinessEvidenceArtifact {
            id: id(&format!("artifact.{suffix}")),
            kind,
            target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
            freshness: UiReadinessArtifactFreshness::Fresh,
            ownership: UiReadinessArtifactOwnership::ExternalReference,
        }
    }

    fn packet(id_value: &str) -> UiReadinessEvidencePacket {
        UiReadinessEvidencePacket {
            id: id(id_value),
            document: id(&format!("{id_value}.document")),
            target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
            compatibility_axes: compatibility_axes(),
            required_evidence: required_evidence(),
            artifacts: vec![
                artifact(UiReadinessEvidenceKind::ProjectionSnapshot, "projection"),
                artifact(UiReadinessEvidenceKind::DiagnosticInspection, "diagnostics"),
                artifact(
                    UiReadinessEvidenceKind::AccessibilityReport,
                    "accessibility",
                ),
                artifact(
                    UiReadinessEvidenceKind::CompatibilityReport,
                    "compatibility",
                ),
                artifact(
                    UiReadinessEvidenceKind::PerformanceBudgetReport,
                    "performance",
                ),
                artifact(UiReadinessEvidenceKind::GoldenArtifact, "golden"),
                artifact(UiReadinessEvidenceKind::ExampleScenario, "example"),
            ],
            expected_diagnostics: BTreeSet::new(),
            source_package: source_package(),
            source_location: None,
            preview_only: false,
        }
    }

    fn inspection(id_value: &str) -> UiReadinessInspectionReport {
        UiReadinessInspectionReport {
            id: id(id_value),
            target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
            compatibility_axes: compatibility_axes(),
            diagnostic_groups: BTreeSet::from([
                UiReadinessDiagnosticGroup::Composition,
                UiReadinessDiagnosticGroup::Style,
                UiReadinessDiagnosticGroup::Recipe,
                UiReadinessDiagnosticGroup::Binding,
                UiReadinessDiagnosticGroup::Preview,
                UiReadinessDiagnosticGroup::Persistence,
                UiReadinessDiagnosticGroup::Accessibility,
                UiReadinessDiagnosticGroup::Compatibility,
                UiReadinessDiagnosticGroup::Performance,
            ]),
            source_package: source_package(),
            source_location: None,
            preview_only: false,
        }
    }

    fn readiness_request(packet_id: &str, inspection_id: &str) -> UiReadinessRequest {
        UiReadinessRequest {
            id: id(&format!("{packet_id}.request")),
            evidence_packet: id(packet_id),
            inspection_report: Some(id(inspection_id)),
            target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
            required_evidence: required_evidence(),
            expected_diagnostics: BTreeSet::new(),
            source_package: source_package(),
            source_location: None,
            preview_only: false,
        }
    }

    fn library() -> UiReadinessLibrary {
        UiReadinessLibrary {
            evidence_packets: vec![packet("editor.ready"), packet("runtime.ready")],
            inspection_reports: vec![
                inspection("editor.ready.inspection"),
                inspection("runtime.ready.inspection"),
            ],
            readiness_requests: vec![
                readiness_request("editor.ready", "editor.ready.inspection"),
                readiness_request("runtime.ready", "runtime.ready.inspection"),
            ],
        }
    }

    fn request(target_profile: &str) -> UiReadinessValidationRequest {
        UiReadinessValidationRequest::production(target_profile)
    }

    fn codes(report: &UiReadinessValidationReport) -> BTreeSet<&str> {
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    #[test]
    fn production_readiness_validates_editor_and_runtime_examples_without_shared_ownership() {
        let editor = validate_production_readiness(&library(), &request("editor.workbench"));
        let runtime = validate_production_readiness(&library(), &request("game.runtime"));

        assert!(
            !editor.has_errors(),
            "editor diagnostics: {:?}",
            editor.diagnostics
        );
        assert!(
            !runtime.has_errors(),
            "runtime diagnostics: {:?}",
            runtime.diagnostics
        );
    }

    #[test]
    fn production_readiness_rejects_missing_required_evidence() {
        let mut library = library();
        library.evidence_packets[0]
            .artifacts
            .retain(|artifact| artifact.kind != UiReadinessEvidenceKind::AccessibilityReport);

        let report = validate_production_readiness(&library, &request("editor.workbench"));

        assert!(codes(&report).contains("ui.readiness.evidence.missing"));
    }

    #[test]
    fn game_runtime_readiness_requires_axis_coverage_and_external_artifacts() {
        let mut library = library();
        library.evidence_packets[1]
            .compatibility_axes
            .remove(&UiReadinessCompatibilityAxis::SafeArea);
        library.inspection_reports[1]
            .compatibility_axes
            .remove(&UiReadinessCompatibilityAxis::SafeArea);
        library.evidence_packets[1].artifacts[0].ownership =
            UiReadinessArtifactOwnership::OwnsConcreteArtifact;

        let report = validate_production_readiness(&library, &request("game.runtime"));
        let codes = codes(&report);

        assert!(codes.contains("ui.readiness.compatibility_axis.missing"));
        assert!(codes.contains("ui.readiness.inspection.compatibility_axis.missing"));
        assert!(codes.contains("ui.readiness.artifact.ownership_violation"));
    }

    #[test]
    fn game_runtime_readiness_rejects_missing_compatibility_report() {
        let mut library = library();
        library.evidence_packets[1]
            .artifacts
            .retain(|artifact| artifact.kind != UiReadinessEvidenceKind::CompatibilityReport);

        let report = validate_production_readiness(&library, &request("game.runtime"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.readiness.evidence.missing"));
    }

    #[test]
    fn production_readiness_rejects_stale_evidence_outside_inspect_mode() {
        let mut library = library();
        library.evidence_packets[0].artifacts[0].freshness = UiReadinessArtifactFreshness::Stale;

        let production = validate_production_readiness(&library, &request("editor.workbench"));
        let inspect = validate_production_readiness(
            &library,
            &UiReadinessValidationRequest::inspect("editor.workbench"),
        );

        assert!(codes(&production).contains("ui.readiness.artifact.stale"));
        assert!(!codes(&inspect).contains("ui.readiness.artifact.stale"));
    }

    #[test]
    fn production_readiness_requires_diagnostic_inspection_report() {
        let mut library = library();
        library.readiness_requests[0].inspection_report = None;

        let report = validate_production_readiness(&library, &request("editor.workbench"));

        assert!(codes(&report).contains("ui.readiness.request.inspection_missing"));
    }

    #[test]
    fn production_readiness_rejects_unsupported_target_profile() {
        let report = validate_production_readiness(&library(), &request("console.runtime"));
        let codes = codes(&report);

        assert!(codes.contains("ui.readiness.packet.target_profile_unsupported"));
        assert!(codes.contains("ui.readiness.artifact.target_profile_unsupported"));
        assert!(codes.contains("ui.readiness.inspection.target_profile_unsupported"));
        assert!(codes.contains("ui.readiness.request.target_profile_unsupported"));
    }

    #[test]
    fn production_readiness_rejects_expected_diagnostic_mismatches() {
        let mut library = library();
        library.evidence_packets[0]
            .expected_diagnostics
            .insert(id("ui.readiness.expected"));
        library.readiness_requests[0]
            .expected_diagnostics
            .insert(id("ui.readiness.expected"));

        let report = validate_production_readiness(
            &library,
            &UiReadinessValidationRequest::production("editor.workbench")
                .with_actual_diagnostic("ui.readiness.actual"),
        );
        let codes = codes(&report);

        assert!(codes.contains("ui.readiness.packet.expected_diagnostics_mismatch"));
        assert!(codes.contains("ui.readiness.request.expected_diagnostics_mismatch"));
    }

    #[test]
    fn production_readiness_rejects_artifact_ownership_violations() {
        let mut library = library();
        library.evidence_packets[0].artifacts[0].ownership =
            UiReadinessArtifactOwnership::OwnsConcreteArtifact;

        let report = validate_production_readiness(&library, &request("editor.workbench"));

        assert!(codes(&report).contains("ui.readiness.artifact.ownership_violation"));
    }

    #[test]
    fn production_readiness_rejects_preview_only_production() {
        let mut library = library();
        library.evidence_packets[0].preview_only = true;
        library.inspection_reports[0].preview_only = true;
        library.readiness_requests[0].preview_only = true;

        let report = validate_production_readiness(&library, &request("editor.workbench"));
        let codes = codes(&report);

        assert!(codes.contains("ui.readiness.packet.preview_only_production"));
        assert!(codes.contains("ui.readiness.inspection.preview_only_production"));
        assert!(codes.contains("ui.readiness.request.preview_only_production"));
    }
}
