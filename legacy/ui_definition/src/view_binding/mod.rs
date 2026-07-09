//! Runtime-neutral view-model binding and intent proposal contracts.

use crate::{UiDefinitionDiagnosticSeverity, UiNodeId, UiSourceLocation, identity::AuthoredId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type UiBindingId = AuthoredId;
pub type UiIntentId = AuthoredId;
pub type UiViewModelPackageId = AuthoredId;
pub type UiViewModelFieldId = AuthoredId;
pub type UiCapabilityId = AuthoredId;
pub type UiBindingTargetProfileId = AuthoredId;
pub type UiBindingSourcePackageId = AuthoredId;
pub type UiBindingFormatterId = AuthoredId;
pub type UiIntentParameterId = AuthoredId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiBindingValueType {
    Text,
    Bool,
    Number,
    Availability,
    Collection,
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiBindingValidationMode {
    Preview,
    DryRun,
    Activate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiBindingActivationImpact {
    None,
    PreviewOnly,
    BlocksActivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiBindingDiagnosticDomain {
    UiDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiViewModelPackageStatus {
    Available,
    Missing,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiBindingMissingDataPolicy {
    BlockActivation,
    AllowStale,
    UseFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiViewModelFieldDefinition {
    pub id: UiViewModelFieldId,
    pub value_type: UiBindingValueType,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiViewModelPackageDefinition {
    pub id: UiViewModelPackageId,
    pub source_package: UiBindingSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiBindingTargetProfileId>,
    #[serde(default)]
    pub fields: Vec<UiViewModelFieldDefinition>,
    #[serde(default)]
    pub provided_capabilities: BTreeSet<UiCapabilityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBindingDeclaration {
    pub id: UiBindingId,
    #[serde(default)]
    pub node: Option<UiNodeId>,
    pub package: UiViewModelPackageId,
    pub field: UiViewModelFieldId,
    pub value_type: UiBindingValueType,
    #[serde(default)]
    pub required_capabilities: BTreeSet<UiCapabilityId>,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiBindingTargetProfileId>,
    #[serde(default)]
    pub formatter: Option<UiBindingFormatterId>,
    pub missing_data_policy: UiBindingMissingDataPolicy,
    pub source_package: UiBindingSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiIntentDescriptorRef {
    EditorCommand(AuthoredId),
    GameIntent(AuthoredId),
    AdapterIntent(AuthoredId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiIntentTriggerSource {
    Button(UiNodeId),
    Menu(AuthoredId),
    Shortcut(String),
    Selection(UiNodeId),
    Focus(UiNodeId),
    PointerGesture(UiNodeId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiIntentPayloadBinding {
    pub parameter: UiIntentParameterId,
    pub binding: UiBindingId,
    pub value_type: UiBindingValueType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiIntentDeclaration {
    pub id: UiIntentId,
    #[serde(default)]
    pub node: Option<UiNodeId>,
    pub trigger: UiIntentTriggerSource,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiBindingTargetProfileId>,
    #[serde(default)]
    pub required_capabilities: BTreeSet<UiCapabilityId>,
    pub descriptor: UiIntentDescriptorRef,
    #[serde(default)]
    pub payload_bindings: Vec<UiIntentPayloadBinding>,
    pub source_package: UiBindingSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
    pub direct_mutation: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBindingLibrary {
    #[serde(default)]
    pub view_model_packages: Vec<UiViewModelPackageDefinition>,
    #[serde(default)]
    pub bindings: Vec<UiBindingDeclaration>,
    #[serde(default)]
    pub intents: Vec<UiIntentDeclaration>,
    #[serde(default)]
    pub known_capabilities: BTreeSet<UiCapabilityId>,
    #[serde(default)]
    pub known_intent_descriptors: BTreeSet<UiIntentDescriptorRef>,
    #[serde(default)]
    pub known_formatters: BTreeSet<UiBindingFormatterId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBindingValidationRequest {
    pub target_profile: UiBindingTargetProfileId,
    pub mode: UiBindingValidationMode,
    #[serde(default)]
    pub package_status: BTreeMap<UiViewModelPackageId, UiViewModelPackageStatus>,
    #[serde(default)]
    pub denied_capabilities: BTreeSet<UiCapabilityId>,
}

impl UiBindingValidationRequest {
    pub fn activate(target_profile: impl Into<UiBindingTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiBindingValidationMode::Activate,
            package_status: BTreeMap::new(),
            denied_capabilities: BTreeSet::new(),
        }
    }

    pub fn preview(target_profile: impl Into<UiBindingTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiBindingValidationMode::Preview,
            package_status: BTreeMap::new(),
            denied_capabilities: BTreeSet::new(),
        }
    }

    pub fn with_package_status(
        mut self,
        package: impl Into<UiViewModelPackageId>,
        status: UiViewModelPackageStatus,
    ) -> Self {
        self.package_status.insert(package.into(), status);
        self
    }

    pub fn with_denied_capability(mut self, capability: impl Into<UiCapabilityId>) -> Self {
        self.denied_capabilities.insert(capability.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBindingDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub binding: Option<UiBindingId>,
    #[serde(default)]
    pub intent: Option<UiIntentId>,
    #[serde(default)]
    pub node: Option<UiNodeId>,
    #[serde(default)]
    pub trigger: Option<UiIntentTriggerSource>,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profile: Option<UiBindingTargetProfileId>,
    pub owning_domain: UiBindingDiagnosticDomain,
    #[serde(default)]
    pub source_package: Option<UiBindingSourcePackageId>,
    #[serde(default)]
    pub required_capabilities: Vec<UiCapabilityId>,
    #[serde(default)]
    pub denied_capabilities: Vec<UiCapabilityId>,
    pub activation_impact: UiBindingActivationImpact,
    pub suggested_fix: String,
}

impl UiBindingDiagnostic {
    fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            binding: None,
            intent: None,
            node: None,
            trigger: None,
            source_location: None,
            target_profile: None,
            owning_domain: UiBindingDiagnosticDomain::UiDefinition,
            source_package: None,
            required_capabilities: Vec::new(),
            denied_capabilities: Vec::new(),
            activation_impact: UiBindingActivationImpact::BlocksActivation,
            suggested_fix: suggested_fix.into(),
        }
    }

    fn for_binding(mut self, binding: &UiBindingDeclaration) -> Self {
        self.binding = Some(binding.id.clone());
        self.node = binding.node.clone();
        self.source_location = binding.source_location.clone();
        self.source_package = Some(binding.source_package.clone());
        self.required_capabilities = binding.required_capabilities.iter().cloned().collect();
        self
    }

    fn for_intent(mut self, intent: &UiIntentDeclaration) -> Self {
        self.intent = Some(intent.id.clone());
        self.node = intent.node.clone();
        self.trigger = Some(intent.trigger.clone());
        self.source_location = intent.source_location.clone();
        self.source_package = Some(intent.source_package.clone());
        self.required_capabilities = intent.required_capabilities.iter().cloned().collect();
        self
    }

    fn with_target_profile(mut self, target_profile: UiBindingTargetProfileId) -> Self {
        self.target_profile = Some(target_profile);
        self
    }

    fn with_denied_capabilities(mut self, capabilities: Vec<UiCapabilityId>) -> Self {
        self.denied_capabilities = capabilities;
        self
    }

    fn preview_only(mut self) -> Self {
        self.activation_impact = UiBindingActivationImpact::PreviewOnly;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiBindingValidationReport {
    #[serde(default)]
    pub diagnostics: Vec<UiBindingDiagnostic>,
}

impl UiBindingValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }
}

pub fn validate_ui_bindings(
    library: &UiBindingLibrary,
    request: &UiBindingValidationRequest,
) -> UiBindingValidationReport {
    let mut diagnostics = Vec::new();
    let packages = index_packages(library, request, &mut diagnostics);
    let bindings = index_bindings(library, request, &mut diagnostics);

    let mut validator = BindingValidator {
        library,
        request,
        diagnostics,
    };

    validator.validate_bindings(&packages);
    validator.validate_intents(&bindings);

    UiBindingValidationReport {
        diagnostics: validator.diagnostics,
    }
}

fn index_packages<'a>(
    library: &'a UiBindingLibrary,
    request: &UiBindingValidationRequest,
    diagnostics: &mut Vec<UiBindingDiagnostic>,
) -> BTreeMap<UiViewModelPackageId, &'a UiViewModelPackageDefinition> {
    let mut packages = BTreeMap::new();
    for package in &library.view_model_packages {
        if packages.insert(package.id.clone(), package).is_some() {
            diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.view_model_package.duplicate_id",
                    format!(
                        "View-model package '{}' is declared more than once.",
                        package.id
                    ),
                    "Keep one view-model package declaration for each stable package id.",
                )
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    packages
}

fn index_bindings<'a>(
    library: &'a UiBindingLibrary,
    request: &UiBindingValidationRequest,
    diagnostics: &mut Vec<UiBindingDiagnostic>,
) -> BTreeMap<UiBindingId, &'a UiBindingDeclaration> {
    let mut bindings = BTreeMap::new();
    for binding in &library.bindings {
        if bindings.insert(binding.id.clone(), binding).is_some() {
            diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.duplicate_id",
                    format!("Binding '{}' is declared more than once.", binding.id),
                    "Keep one binding declaration for each stable binding id.",
                )
                .for_binding(binding)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    bindings
}

struct BindingValidator<'a> {
    library: &'a UiBindingLibrary,
    request: &'a UiBindingValidationRequest,
    diagnostics: Vec<UiBindingDiagnostic>,
}

impl BindingValidator<'_> {
    fn validate_bindings(
        &mut self,
        packages: &BTreeMap<UiViewModelPackageId, &UiViewModelPackageDefinition>,
    ) {
        for binding in &self.library.bindings {
            self.validate_binding_target_profile(binding);
            self.validate_preview_only_binding(binding);
            self.validate_capabilities_for_binding(binding);
            self.validate_formatter(binding);
            self.validate_package_status(binding);
            self.validate_binding_package_and_field(binding, packages);
        }
    }

    fn validate_binding_target_profile(&mut self, binding: &UiBindingDeclaration) {
        if !binding.target_profiles.is_empty()
            && !binding
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.target_profile.unsupported",
                    format!(
                        "Binding '{}' does not support target profile '{}'.",
                        binding.id, self.request.target_profile
                    ),
                    "Add target-profile support to the binding or choose a compatible binding.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_preview_only_binding(&mut self, binding: &UiBindingDeclaration) {
        if binding.preview_only && self.request.mode == UiBindingValidationMode::Activate {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.preview_only_activation",
                    format!(
                        "Binding '{}' is preview-only and cannot activate.",
                        binding.id
                    ),
                    "Use preview validation or remove the preview-only flag before activation.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }

    fn validate_capabilities_for_binding(&mut self, binding: &UiBindingDeclaration) {
        let unknown = unknown_capabilities(&binding.required_capabilities, self.library);
        if !unknown.is_empty() {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.capability.unknown",
                    format!("Binding '{}' references unknown capabilities.", binding.id),
                    "Register each required capability in the binding library or remove it.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }

        let denied = denied_capabilities(&binding.required_capabilities, self.request);
        if !denied.is_empty() {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.capability.denied",
                    format!("Binding '{}' requires denied capabilities.", binding.id),
                    "Request an allowed capability or disable the binding for this target profile.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone())
                .with_denied_capabilities(denied),
            );
        }
    }

    fn validate_formatter(&mut self, binding: &UiBindingDeclaration) {
        if binding
            .formatter
            .as_ref()
            .is_some_and(|formatter| !self.library.known_formatters.contains(formatter))
        {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.formatter.unknown",
                    format!("Binding '{}' references an unknown formatter.", binding.id),
                    "Register the formatter or remove the formatter reference.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_package_status(&mut self, binding: &UiBindingDeclaration) {
        match self
            .request
            .package_status
            .get(&binding.package)
            .copied()
            .unwrap_or(UiViewModelPackageStatus::Missing)
        {
            UiViewModelPackageStatus::Available => {}
            UiViewModelPackageStatus::Missing => {
                if binding.missing_data_policy == UiBindingMissingDataPolicy::BlockActivation {
                    self.diagnostics.push(
                        UiBindingDiagnostic::error(
                            "ui.binding.view_model_package.missing",
                            format!(
                                "Binding '{}' requires missing view-model package '{}'.",
                                binding.id, binding.package
                            ),
                            "Provide the package or choose a missing-data policy that is valid for preview.",
                        )
                        .for_binding(binding)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                }
            }
            UiViewModelPackageStatus::Stale => {
                if binding.missing_data_policy != UiBindingMissingDataPolicy::AllowStale {
                    self.diagnostics.push(
                        UiBindingDiagnostic::error(
                            "ui.binding.view_model_package.stale",
                            format!(
                                "Binding '{}' cannot activate with stale view-model package '{}'.",
                                binding.id, binding.package
                            ),
                            "Refresh the package or explicitly allow stale data for this binding.",
                        )
                        .for_binding(binding)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                }
            }
        }
    }

    fn validate_binding_package_and_field(
        &mut self,
        binding: &UiBindingDeclaration,
        packages: &BTreeMap<UiViewModelPackageId, &UiViewModelPackageDefinition>,
    ) {
        let Some(package) = packages.get(&binding.package).copied() else {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.view_model_package.unknown",
                    format!(
                        "Binding '{}' references unknown view-model package '{}'.",
                        binding.id, binding.package
                    ),
                    "Add the view-model package declaration or remove the binding.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        if !package.target_profiles.is_empty()
            && !package
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.view_model_package.target_profile_unsupported",
                    format!(
                        "View-model package '{}' does not support target profile '{}'.",
                        package.id, self.request.target_profile
                    ),
                    "Provide a compatible view-model package for the target profile.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }

        let Some(field) = package
            .fields
            .iter()
            .find(|field| field.id == binding.field)
        else {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.field.unknown",
                    format!(
                        "Binding '{}' references unknown field '{}'.",
                        binding.id, binding.field
                    ),
                    "Add the field to the view-model package or update the binding reference.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        if field.value_type != binding.value_type {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.value_type.mismatch",
                    format!(
                        "Binding '{}' expects {:?} but field '{}' provides {:?}.",
                        binding.id, binding.value_type, field.id, field.value_type
                    ),
                    "Change the binding value type or bind to a field with the expected type.",
                )
                .for_binding(binding)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_intents(&mut self, bindings: &BTreeMap<UiBindingId, &UiBindingDeclaration>) {
        let mut intent_ids = BTreeSet::new();
        let mut triggers: BTreeMap<UiIntentTriggerSource, UiIntentId> = BTreeMap::new();

        for intent in &self.library.intents {
            if !intent_ids.insert(intent.id.clone()) {
                self.diagnostics.push(
                    UiBindingDiagnostic::error(
                        "ui.binding.intent.duplicate_id",
                        format!("Intent '{}' is declared more than once.", intent.id),
                        "Keep one intent declaration for each stable intent id.",
                    )
                    .for_intent(intent)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            let applies_to_target = intent.target_profiles.is_empty()
                || intent
                    .target_profiles
                    .contains(&self.request.target_profile);
            if applies_to_target
                && let Some(existing) = triggers.insert(intent.trigger.clone(), intent.id.clone())
            {
                self.diagnostics.push(
                    UiBindingDiagnostic::error(
                        "ui.binding.intent.trigger_conflict",
                        format!(
                            "Intent '{}' conflicts with '{}' on the same trigger.",
                            intent.id, existing
                        ),
                        "Move one intent to a different trigger or add a target-profile-specific override.",
                    )
                    .for_intent(intent)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            self.validate_intent_target_profile(intent);
            self.validate_preview_only_intent(intent);
            self.validate_intent_capabilities(intent);
            self.validate_intent_descriptor(intent);
            self.validate_direct_mutation(intent);
            self.validate_payload_bindings(intent, bindings);
        }
    }

    fn validate_intent_target_profile(&mut self, intent: &UiIntentDeclaration) {
        if !intent.target_profiles.is_empty()
            && !intent
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.target_profile.unsupported",
                    format!(
                        "Intent '{}' does not support target profile '{}'.",
                        intent.id, self.request.target_profile
                    ),
                    "Add target-profile support to the intent or choose a compatible intent.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_preview_only_intent(&mut self, intent: &UiIntentDeclaration) {
        if intent.preview_only && self.request.mode == UiBindingValidationMode::Activate {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.preview_only_activation",
                    format!(
                        "Intent '{}' is preview-only and cannot activate.",
                        intent.id
                    ),
                    "Use preview validation or remove the preview-only flag before activation.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }

    fn validate_intent_capabilities(&mut self, intent: &UiIntentDeclaration) {
        let unknown = unknown_capabilities(&intent.required_capabilities, self.library);
        if !unknown.is_empty() {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.capability.unknown",
                    format!("Intent '{}' references unknown capabilities.", intent.id),
                    "Register each required capability in the binding library or remove it.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }

        let denied = denied_capabilities(&intent.required_capabilities, self.request);
        if !denied.is_empty() {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.capability.denied",
                    format!("Intent '{}' requires denied capabilities.", intent.id),
                    "Request an allowed capability or disable the intent for this target profile.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone())
                .with_denied_capabilities(denied),
            );
        }
    }

    fn validate_intent_descriptor(&mut self, intent: &UiIntentDeclaration) {
        if !self
            .library
            .known_intent_descriptors
            .contains(&intent.descriptor)
        {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.descriptor.unknown",
                    format!("Intent '{}' references an unknown descriptor.", intent.id),
                    "Register the domain-owned command or game intent descriptor.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }

        let applies_to_target = intent.target_profiles.is_empty()
            || intent
                .target_profiles
                .contains(&self.request.target_profile);
        if self.request.target_profile.as_str() == "game.runtime"
            && applies_to_target
            && matches!(intent.descriptor, UiIntentDescriptorRef::EditorCommand(_))
        {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.descriptor.editor_command_for_runtime",
                    format!(
                        "Intent '{}' uses an editor command descriptor for game.runtime.",
                        intent.id
                    ),
                    "Use a game intent or adapter intent descriptor for game.runtime declarations.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_direct_mutation(&mut self, intent: &UiIntentDeclaration) {
        if intent.direct_mutation {
            self.diagnostics.push(
                UiBindingDiagnostic::error(
                    "ui.binding.intent.direct_mutation",
                    format!("Intent '{}' encodes direct mutation.", intent.id),
                    "Replace direct mutation with a domain-owned command or game intent descriptor reference.",
                )
                .for_intent(intent)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_payload_bindings(
        &mut self,
        intent: &UiIntentDeclaration,
        bindings: &BTreeMap<UiBindingId, &UiBindingDeclaration>,
    ) {
        let mut parameters = BTreeSet::new();
        for payload in &intent.payload_bindings {
            if !parameters.insert(payload.parameter.clone()) {
                self.diagnostics.push(
                    UiBindingDiagnostic::error(
                        "ui.binding.intent.payload.duplicate_parameter",
                        format!(
                            "Intent '{}' repeats payload parameter '{}'.",
                            intent.id, payload.parameter
                        ),
                        "Keep one binding source for each payload parameter.",
                    )
                    .for_intent(intent)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            let Some(binding) = bindings.get(&payload.binding).copied() else {
                self.diagnostics.push(
                    UiBindingDiagnostic::error(
                        "ui.binding.intent.payload.unknown_binding",
                        format!(
                            "Intent '{}' references unknown payload binding '{}'.",
                            intent.id, payload.binding
                        ),
                        "Add the binding declaration or remove the payload parameter.",
                    )
                    .for_intent(intent)
                    .with_target_profile(self.request.target_profile.clone()),
                );
                continue;
            };

            if binding.value_type != payload.value_type {
                self.diagnostics.push(
                    UiBindingDiagnostic::error(
                        "ui.binding.intent.payload.value_type_mismatch",
                        format!(
                            "Intent '{}' payload '{}' expects {:?} but binding '{}' provides {:?}.",
                            intent.id,
                            payload.parameter,
                            payload.value_type,
                            binding.id,
                            binding.value_type
                        ),
                        "Change the payload value type or bind to a compatible value.",
                    )
                    .for_intent(intent)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }
}

fn unknown_capabilities(
    required: &BTreeSet<UiCapabilityId>,
    library: &UiBindingLibrary,
) -> Vec<UiCapabilityId> {
    required
        .iter()
        .filter(|capability| !library.known_capabilities.contains(*capability))
        .cloned()
        .collect()
}

fn denied_capabilities(
    required: &BTreeSet<UiCapabilityId>,
    request: &UiBindingValidationRequest,
) -> Vec<UiCapabilityId> {
    required
        .iter()
        .filter(|capability| request.denied_capabilities.contains(*capability))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> AuthoredId {
        AuthoredId::from(value)
    }

    fn profiles(values: &[&str]) -> BTreeSet<UiBindingTargetProfileId> {
        values.iter().copied().map(AuthoredId::from).collect()
    }

    fn capabilities(values: &[&str]) -> BTreeSet<UiCapabilityId> {
        values.iter().copied().map(AuthoredId::from).collect()
    }

    fn package(package_id: &str, profiles: &[&str]) -> UiViewModelPackageDefinition {
        UiViewModelPackageDefinition {
            id: id(package_id),
            source_package: id("test.package"),
            source_location: None,
            target_profiles: self::profiles(profiles),
            fields: vec![
                UiViewModelFieldDefinition {
                    id: id("title"),
                    value_type: UiBindingValueType::Text,
                    required: true,
                },
                UiViewModelFieldDefinition {
                    id: id("enabled"),
                    value_type: UiBindingValueType::Bool,
                    required: true,
                },
            ],
            provided_capabilities: capabilities(&["view.read"]),
        }
    }

    fn binding(binding_id: &str, package: &str, field: &str) -> UiBindingDeclaration {
        UiBindingDeclaration {
            id: id(binding_id),
            node: Some(id("title-node")),
            package: id(package),
            field: id(field),
            value_type: UiBindingValueType::Text,
            required_capabilities: capabilities(&["view.read"]),
            target_profiles: profiles(&["editor.workbench", "game.runtime"]),
            formatter: None,
            missing_data_policy: UiBindingMissingDataPolicy::BlockActivation,
            source_package: id("test.package"),
            source_location: None,
            preview_only: false,
        }
    }

    fn intent(intent_id: &str) -> UiIntentDeclaration {
        UiIntentDeclaration {
            id: id(intent_id),
            node: Some(id("button-node")),
            trigger: UiIntentTriggerSource::Button(id("button-node")),
            target_profiles: profiles(&["editor.workbench", "game.runtime"]),
            required_capabilities: capabilities(&["intent.emit"]),
            descriptor: UiIntentDescriptorRef::EditorCommand(id("open.asset")),
            payload_bindings: vec![UiIntentPayloadBinding {
                parameter: id("label"),
                binding: id("title.binding"),
                value_type: UiBindingValueType::Text,
            }],
            source_package: id("test.package"),
            source_location: None,
            preview_only: false,
            direct_mutation: false,
        }
    }

    fn library() -> UiBindingLibrary {
        UiBindingLibrary {
            view_model_packages: vec![
                package("asset.browser", &["editor.workbench"]),
                package("combat.hud", &["game.runtime"]),
            ],
            bindings: vec![binding("title.binding", "asset.browser", "title")],
            intents: vec![intent("open.intent")],
            known_capabilities: capabilities(&["view.read", "intent.emit"]),
            known_intent_descriptors: [UiIntentDescriptorRef::EditorCommand(id("open.asset"))]
                .into_iter()
                .collect(),
            known_formatters: BTreeSet::new(),
        }
    }

    fn request(target: &str, package: &str) -> UiBindingValidationRequest {
        UiBindingValidationRequest::activate(target)
            .with_package_status(package, UiViewModelPackageStatus::Available)
    }

    fn codes(report: &UiBindingValidationReport) -> BTreeSet<&str> {
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect()
    }

    #[test]
    fn view_binding_validates_editor_and_runtime_examples_without_shared_authority() {
        let mut runtime_library = library();
        runtime_library.bindings = vec![binding("title.binding", "combat.hud", "title")];
        runtime_library.intents[0].descriptor = UiIntentDescriptorRef::GameIntent(id("use.item"));
        runtime_library.known_intent_descriptors =
            [UiIntentDescriptorRef::GameIntent(id("use.item"))]
                .into_iter()
                .collect();

        let editor =
            validate_ui_bindings(&library(), &request("editor.workbench", "asset.browser"));
        let runtime =
            validate_ui_bindings(&runtime_library, &request("game.runtime", "combat.hud"));

        assert!(!editor.has_errors(), "{:?}", editor.diagnostics);
        assert!(!runtime.has_errors(), "{:?}", runtime.diagnostics);
    }

    #[test]
    fn view_binding_rejects_value_type_mismatch() {
        let mut library = library();
        library.bindings[0].field = id("enabled");

        let report = validate_ui_bindings(&library, &request("editor.workbench", "asset.browser"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.value_type.mismatch"));
    }

    #[test]
    fn view_binding_rejects_missing_view_model_package() {
        let report = validate_ui_bindings(
            &library(),
            &UiBindingValidationRequest::activate("editor.workbench"),
        );

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.view_model_package.missing"));
    }

    #[test]
    fn game_runtime_rejects_missing_or_stale_view_model_packages() {
        let mut runtime_library = library();
        runtime_library.bindings = vec![binding("title.binding", "combat.hud", "title")];
        runtime_library.intents[0].descriptor = UiIntentDescriptorRef::GameIntent(id("use.item"));
        runtime_library.known_intent_descriptors =
            [UiIntentDescriptorRef::GameIntent(id("use.item"))]
                .into_iter()
                .collect();

        let missing = validate_ui_bindings(
            &runtime_library,
            &UiBindingValidationRequest::activate("game.runtime"),
        );
        let stale = validate_ui_bindings(
            &runtime_library,
            &request("game.runtime", "combat.hud")
                .with_package_status("combat.hud", UiViewModelPackageStatus::Stale),
        );

        assert!(codes(&missing).contains("ui.binding.view_model_package.missing"));
        assert!(codes(&stale).contains("ui.binding.view_model_package.stale"));
    }

    #[test]
    fn view_binding_rejects_denied_capability() {
        let report = validate_ui_bindings(
            &library(),
            &request("editor.workbench", "asset.browser").with_denied_capability("view.read"),
        );

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.capability.denied"));
    }

    #[test]
    fn view_binding_rejects_unsupported_target_profile() {
        let mut library = library();
        library.bindings[0].target_profiles = profiles(&["editor.workbench"]);

        let report = validate_ui_bindings(&library, &request("game.runtime", "asset.browser"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.target_profile.unsupported"));
        assert!(
            codes(&report).contains("ui.binding.view_model_package.target_profile_unsupported")
        );
    }

    #[test]
    fn game_runtime_rejects_editor_command_descriptor() {
        let mut runtime_library = library();
        runtime_library.bindings = vec![binding("title.binding", "combat.hud", "title")];

        let report = validate_ui_bindings(&runtime_library, &request("game.runtime", "combat.hud"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.intent.descriptor.editor_command_for_runtime"));
    }

    #[test]
    fn view_binding_intent_emits_proposals_not_direct_mutation() {
        let mut library = library();
        library.intents[0].direct_mutation = true;

        let report = validate_ui_bindings(&library, &request("editor.workbench", "asset.browser"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.intent.direct_mutation"));
    }

    #[test]
    fn view_binding_rejects_intent_payload_type_mismatch() {
        let mut library = library();
        library.intents[0].payload_bindings[0].value_type = UiBindingValueType::Number;

        let report = validate_ui_bindings(&library, &request("editor.workbench", "asset.browser"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.intent.payload.value_type_mismatch"));
    }

    #[test]
    fn view_binding_rejects_trigger_conflicts() {
        let mut library = library();
        library.intents.push(intent("duplicate.intent"));

        let report = validate_ui_bindings(&library, &request("editor.workbench", "asset.browser"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.intent.trigger_conflict"));
    }

    #[test]
    fn view_binding_allows_shared_triggers_across_disjoint_target_profiles() {
        let mut library = library();
        library.intents[0].target_profiles = profiles(&["editor.workbench"]);
        let mut runtime_intent = intent("runtime.intent");
        runtime_intent.target_profiles = profiles(&["game.runtime"]);
        library.intents.push(runtime_intent);

        let report = validate_ui_bindings(&library, &request("editor.workbench", "asset.browser"));

        assert!(!codes(&report).contains("ui.binding.intent.trigger_conflict"));
    }

    #[test]
    fn view_binding_rejects_preview_only_activation() {
        let mut library = library();
        library.bindings[0].preview_only = true;
        library.intents[0].preview_only = true;

        let report = validate_ui_bindings(&library, &request("editor.workbench", "asset.browser"));

        assert!(report.has_errors());
        assert!(codes(&report).contains("ui.binding.preview_only_activation"));
        assert!(codes(&report).contains("ui.binding.intent.preview_only_activation"));
    }
}
