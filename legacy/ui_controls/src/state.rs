//! File: domain/ui/ui_controls/src/state.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlStateBucket {
    Transient,
    Preview,
    Committed,
    Focus,
    Hover,
    Drag,
    Animation,
    HostFed,
    PackageOwned,
}

impl ControlStateBucket {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Preview => "preview",
            Self::Committed => "committed",
            Self::Focus => "focus",
            Self::Hover => "hover",
            Self::Drag => "drag",
            Self::Animation => "animation",
            Self::HostFed => "host-fed",
            Self::PackageOwned => "package-owned",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStateBucketRequirement {
    pub bucket: ControlStateBucket,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlStateBucketRequirement {
    pub fn new(bucket: ControlStateBucket) -> Self {
        Self {
            bucket,
            required: true,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlStateBindingKind {
    Read,
    Write,
    Collection,
    Option,
    Selection,
}

impl ControlStateBindingKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Collection => "collection",
            Self::Option => "option",
            Self::Selection => "selection",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStateBindingRequirement {
    pub binding_id: String,
    pub kind: ControlStateBindingKind,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlStateBindingRequirement {
    pub fn new(binding_id: impl Into<String>, kind: ControlStateBindingKind) -> Self {
        Self {
            binding_id: binding_id.into(),
            kind,
            required: true,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlEditLifecycle {
    LiveEdit,
    CommitEdit,
    CancelEdit,
    RollbackEdit,
}

impl ControlEditLifecycle {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LiveEdit => "live-edit",
            Self::CommitEdit => "commit-edit",
            Self::CancelEdit => "cancel-edit",
            Self::RollbackEdit => "rollback-edit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlValidationState {
    Clean,
    Dirty,
    ReadOnly,
    Invalid,
    Warning,
    PendingValidation,
}

impl ControlValidationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Dirty => "dirty",
            Self::ReadOnly => "read-only",
            Self::Invalid => "invalid",
            Self::Warning => "warning",
            Self::PendingValidation => "pending-validation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlHostIntentKind {
    Proposal,
}

impl ControlHostIntentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposal => "proposal",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlHostIntentProposal {
    pub intent_id: String,
    pub kind: ControlHostIntentKind,
    pub route_id: String,
    pub route_schema_version: u32,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

impl ControlHostIntentProposal {
    pub fn new(
        intent_id: impl Into<String>,
        route_id: impl Into<String>,
        route_schema_version: u32,
    ) -> Self {
        Self {
            intent_id: intent_id.into(),
            kind: ControlHostIntentKind::Proposal,
            route_id: route_id.into(),
            route_schema_version,
            required_capabilities: Vec::new(),
            notes: String::new(),
        }
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self.required_capabilities.sort();
        self.required_capabilities.dedup();
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlRouteCapabilityDecisionKind {
    NotEvaluated,
    Allowed,
    Blocked,
}

impl ControlRouteCapabilityDecisionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotEvaluated => "not-evaluated",
            Self::Allowed => "allowed",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRouteCapabilityDecision {
    pub route_id: String,
    pub decision: ControlRouteCapabilityDecisionKind,
    #[serde(default)]
    pub blocked_reason: Option<String>,
}

impl ControlRouteCapabilityDecision {
    pub fn not_evaluated(route_id: impl Into<String>) -> Self {
        Self {
            route_id: route_id.into(),
            decision: ControlRouteCapabilityDecisionKind::NotEvaluated,
            blocked_reason: None,
        }
    }

    pub fn allowed(route_id: impl Into<String>) -> Self {
        Self {
            route_id: route_id.into(),
            decision: ControlRouteCapabilityDecisionKind::Allowed,
            blocked_reason: None,
        }
    }

    pub fn blocked(route_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            route_id: route_id.into(),
            decision: ControlRouteCapabilityDecisionKind::Blocked,
            blocked_reason: Some(reason.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStateDescriptor {
    pub control_kind_id: ControlKindId,
    #[serde(default)]
    pub buckets: Vec<ControlStateBucketRequirement>,
    #[serde(default)]
    pub bindings: Vec<ControlStateBindingRequirement>,
    #[serde(default)]
    pub edit_lifecycle: Vec<ControlEditLifecycle>,
    #[serde(default)]
    pub validation_states: Vec<ControlValidationState>,
    #[serde(default)]
    pub host_intents: Vec<ControlHostIntentProposal>,
    #[serde(default)]
    pub route_decisions: Vec<ControlRouteCapabilityDecision>,
}

impl ControlStateDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            buckets: Vec::new(),
            bindings: Vec::new(),
            edit_lifecycle: Vec::new(),
            validation_states: Vec::new(),
            host_intents: Vec::new(),
            route_decisions: Vec::new(),
        }
    }

    pub fn with_bucket(mut self, bucket: ControlStateBucketRequirement) -> Self {
        self.buckets.push(bucket);
        self.buckets.sort_by_key(|bucket| bucket.bucket);
        self.buckets.dedup_by_key(|bucket| bucket.bucket);
        self
    }

    pub fn with_binding(mut self, binding: ControlStateBindingRequirement) -> Self {
        self.bindings.push(binding);
        self.bindings
            .sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
        self.bindings
            .dedup_by(|left, right| left.binding_id == right.binding_id);
        self
    }

    pub fn with_edit_lifecycle(mut self, lifecycle: ControlEditLifecycle) -> Self {
        self.edit_lifecycle.push(lifecycle);
        self.edit_lifecycle.sort();
        self.edit_lifecycle.dedup();
        self
    }

    pub fn with_validation_state(mut self, state: ControlValidationState) -> Self {
        self.validation_states.push(state);
        self.validation_states.sort();
        self.validation_states.dedup();
        self
    }

    pub fn with_host_intent(mut self, intent: ControlHostIntentProposal) -> Self {
        self.host_intents.push(intent);
        self.host_intents
            .sort_by(|left, right| left.intent_id.cmp(&right.intent_id));
        self.host_intents
            .dedup_by(|left, right| left.intent_id == right.intent_id);
        self
    }

    pub fn with_route_decision(mut self, decision: ControlRouteCapabilityDecision) -> Self {
        self.route_decisions.push(decision);
        self.route_decisions
            .sort_by(|left, right| left.route_id.cmp(&right.route_id));
        self.route_decisions
            .dedup_by(|left, right| left.route_id == right.route_id);
        self
    }

    pub fn summary(&self) -> ControlStateCapabilitySummary {
        ControlStateCapabilitySummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStateCapabilitySummary {
    pub control_kind_id: ControlKindId,
    pub required_buckets: Vec<String>,
    pub optional_buckets: Vec<String>,
    pub binding_kinds: Vec<String>,
    pub edit_lifecycle: Vec<String>,
    pub validation_states: Vec<String>,
    pub host_intents: Vec<String>,
    pub route_ids: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub host_decisions: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub mutates_host_state: bool,
}

impl ControlStateCapabilitySummary {
    pub fn from_descriptor(descriptor: &ControlStateDescriptor) -> Self {
        let mut required_buckets = descriptor
            .buckets
            .iter()
            .filter(|bucket| bucket.required)
            .map(|bucket| bucket.bucket.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut optional_buckets = descriptor
            .buckets
            .iter()
            .filter(|bucket| !bucket.required)
            .map(|bucket| bucket.bucket.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut binding_kinds = descriptor
            .bindings
            .iter()
            .map(|binding| binding.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut edit_lifecycle = descriptor
            .edit_lifecycle
            .iter()
            .map(|lifecycle| lifecycle.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut validation_states = descriptor
            .validation_states
            .iter()
            .map(|state| state.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut host_intents = descriptor
            .host_intents
            .iter()
            .map(|intent| intent.intent_id.clone())
            .collect::<Vec<_>>();
        let mut route_ids = descriptor
            .host_intents
            .iter()
            .map(|intent| intent.route_id.clone())
            .chain(
                descriptor
                    .route_decisions
                    .iter()
                    .map(|decision| decision.route_id.clone()),
            )
            .collect::<Vec<_>>();
        let mut required_capabilities = descriptor
            .host_intents
            .iter()
            .flat_map(|intent| intent.required_capabilities.iter().cloned())
            .collect::<Vec<_>>();
        let mut host_decisions = descriptor
            .route_decisions
            .iter()
            .map(|decision| decision.decision.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut blocked_reasons = descriptor
            .route_decisions
            .iter()
            .filter_map(|decision| decision.blocked_reason.clone())
            .collect::<Vec<_>>();

        required_buckets.sort();
        optional_buckets.sort();
        binding_kinds.sort();
        binding_kinds.dedup();
        edit_lifecycle.sort();
        validation_states.sort();
        host_intents.sort();
        route_ids.sort();
        route_ids.dedup();
        required_capabilities.sort();
        required_capabilities.dedup();
        host_decisions.sort();
        host_decisions.dedup();
        blocked_reasons.sort();
        blocked_reasons.dedup();

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            required_buckets,
            optional_buckets,
            binding_kinds,
            edit_lifecycle,
            validation_states,
            host_intents,
            route_ids,
            required_capabilities,
            host_decisions,
            blocked_reasons,
            mutates_host_state: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlStateInspectionFact> {
        vec![
            ControlStateInspectionFact::new("required_buckets", self.required_buckets.join(",")),
            ControlStateInspectionFact::new("optional_buckets", self.optional_buckets.join(",")),
            ControlStateInspectionFact::new("binding_kinds", self.binding_kinds.join(",")),
            ControlStateInspectionFact::new("edit_lifecycle", self.edit_lifecycle.join(",")),
            ControlStateInspectionFact::new("validation_states", self.validation_states.join(",")),
            ControlStateInspectionFact::new("host_intents", self.host_intents.join(",")),
            ControlStateInspectionFact::new("route_ids", self.route_ids.join(",")),
            ControlStateInspectionFact::new(
                "required_capabilities",
                self.required_capabilities.join(","),
            ),
            ControlStateInspectionFact::new("host_decisions", self.host_decisions.join(",")),
            ControlStateInspectionFact::new("blocked_reasons", self.blocked_reasons.join(",")),
            ControlStateInspectionFact::new(
                "mutates_host_state",
                bool_string(self.mutates_host_state),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStateInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlStateInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn default_required() -> bool {
    true
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
