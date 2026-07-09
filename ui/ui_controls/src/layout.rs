//! File: domain/ui/ui_controls/src/layout.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};
use ui_layout::{
    UiContainerKind, UiContentState, UiItemIdentityRequirement, UiLargeContentBudget,
    UiLayoutDiagnostic, UiLayoutRole, UiScrollRequirement, UiSelectionIdentityRequirement,
    UiSizeConstraintKind, UiVirtualizationRequirement,
};

use crate::package::ids::ControlKindId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlLayoutDescriptor {
    pub control_kind_id: ControlKindId,
    #[serde(default)]
    pub layout_roles: Vec<UiLayoutRole>,
    #[serde(default)]
    pub container_kinds: Vec<UiContainerKind>,
    #[serde(default)]
    pub size_constraints: Vec<UiSizeConstraintKind>,
    #[serde(default)]
    pub scroll_requirements: Vec<UiScrollRequirement>,
    #[serde(default)]
    pub content_states: Vec<UiContentState>,
    #[serde(default)]
    pub item_identities: Vec<UiItemIdentityRequirement>,
    #[serde(default)]
    pub selection_identities: Vec<UiSelectionIdentityRequirement>,
    #[serde(default)]
    pub virtualization_requirements: Vec<UiVirtualizationRequirement>,
    #[serde(default)]
    pub large_content_budgets: Vec<UiLargeContentBudget>,
    #[serde(default)]
    pub diagnostics: Vec<UiLayoutDiagnostic>,
}

impl ControlLayoutDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            layout_roles: Vec::new(),
            container_kinds: Vec::new(),
            size_constraints: Vec::new(),
            scroll_requirements: Vec::new(),
            content_states: Vec::new(),
            item_identities: Vec::new(),
            selection_identities: Vec::new(),
            virtualization_requirements: Vec::new(),
            large_content_budgets: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_layout_role(mut self, role: UiLayoutRole) -> Self {
        self.layout_roles.push(role);
        self.layout_roles.sort();
        self.layout_roles.dedup();
        self
    }

    pub fn with_container_kind(mut self, kind: UiContainerKind) -> Self {
        self.container_kinds.push(kind);
        self.container_kinds.sort();
        self.container_kinds.dedup();
        self
    }

    pub fn with_size_constraint(mut self, constraint: UiSizeConstraintKind) -> Self {
        self.size_constraints.push(constraint);
        self.size_constraints.sort();
        self.size_constraints.dedup();
        self
    }

    pub fn with_scroll_requirement(mut self, requirement: UiScrollRequirement) -> Self {
        self.scroll_requirements.push(requirement);
        self.scroll_requirements.sort();
        self.scroll_requirements.dedup();
        self
    }

    pub fn with_content_state(mut self, state: UiContentState) -> Self {
        self.content_states.push(state);
        self.content_states.sort();
        self.content_states.dedup();
        self
    }

    pub fn with_item_identity(mut self, identity: UiItemIdentityRequirement) -> Self {
        self.item_identities.push(identity);
        self.item_identities
            .sort_by(|left, right| left.identity_id.cmp(&right.identity_id));
        self.item_identities
            .dedup_by(|left, right| left.identity_id == right.identity_id);
        self
    }

    pub fn with_selection_identity(mut self, identity: UiSelectionIdentityRequirement) -> Self {
        self.selection_identities.push(identity);
        self.selection_identities
            .sort_by(|left, right| left.identity_id.cmp(&right.identity_id));
        self.selection_identities
            .dedup_by(|left, right| left.identity_id == right.identity_id);
        self
    }

    pub fn with_virtualization_requirement(
        mut self,
        requirement: UiVirtualizationRequirement,
    ) -> Self {
        self.virtualization_requirements.push(requirement);
        self.virtualization_requirements.sort();
        self.virtualization_requirements.dedup();
        self
    }

    pub fn with_large_content_budget(mut self, budget: UiLargeContentBudget) -> Self {
        self.large_content_budgets.push(budget);
        self.large_content_budgets
            .sort_by(|left, right| left.budget_id.cmp(&right.budget_id));
        self.large_content_budgets
            .dedup_by(|left, right| left.budget_id == right.budget_id);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: UiLayoutDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self.diagnostics
            .sort_by(|left, right| left.diagnostic_id.cmp(&right.diagnostic_id));
        self.diagnostics
            .dedup_by(|left, right| left.diagnostic_id == right.diagnostic_id);
        self
    }

    pub fn summary(&self) -> ControlLayoutCapabilitySummary {
        ControlLayoutCapabilitySummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlLayoutCapabilitySummary {
    pub control_kind_id: ControlKindId,
    pub layout_roles: Vec<String>,
    pub container_kinds: Vec<String>,
    pub size_constraints: Vec<String>,
    pub scroll_requirements: Vec<String>,
    pub content_states: Vec<String>,
    pub item_identities: Vec<String>,
    pub selection_identities: Vec<String>,
    pub virtualization_requirements: Vec<String>,
    pub large_content_budgets: Vec<String>,
    pub diagnostics: Vec<String>,
    pub expected_failures: Vec<String>,
    pub has_runtime_layout_behavior: bool,
}

impl ControlLayoutCapabilitySummary {
    pub fn from_descriptor(descriptor: &ControlLayoutDescriptor) -> Self {
        let mut layout_roles = descriptor
            .layout_roles
            .iter()
            .map(|role| role.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut container_kinds = descriptor
            .container_kinds
            .iter()
            .map(|kind| kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut size_constraints = descriptor
            .size_constraints
            .iter()
            .map(|constraint| constraint.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut scroll_requirements = descriptor
            .scroll_requirements
            .iter()
            .map(|requirement| requirement.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut content_states = descriptor
            .content_states
            .iter()
            .map(|state| state.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut item_identities = descriptor
            .item_identities
            .iter()
            .map(|identity| identity.identity_id.clone())
            .collect::<Vec<_>>();
        let mut selection_identities = descriptor
            .selection_identities
            .iter()
            .map(|identity| identity.identity_id.clone())
            .collect::<Vec<_>>();
        let mut virtualization_requirements = descriptor
            .virtualization_requirements
            .iter()
            .map(|requirement| requirement.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut large_content_budgets = descriptor
            .large_content_budgets
            .iter()
            .map(|budget| budget.budget_id.clone())
            .collect::<Vec<_>>();
        let mut diagnostics = descriptor
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut expected_failures = descriptor
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind.as_str() == "expected-failure")
            .map(|diagnostic| diagnostic.diagnostic_id.clone())
            .collect::<Vec<_>>();

        sort_dedup(&mut layout_roles);
        sort_dedup(&mut container_kinds);
        sort_dedup(&mut size_constraints);
        sort_dedup(&mut scroll_requirements);
        sort_dedup(&mut content_states);
        sort_dedup(&mut item_identities);
        sort_dedup(&mut selection_identities);
        sort_dedup(&mut virtualization_requirements);
        sort_dedup(&mut large_content_budgets);
        sort_dedup(&mut diagnostics);
        sort_dedup(&mut expected_failures);

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            layout_roles,
            container_kinds,
            size_constraints,
            scroll_requirements,
            content_states,
            item_identities,
            selection_identities,
            virtualization_requirements,
            large_content_budgets,
            diagnostics,
            expected_failures,
            has_runtime_layout_behavior: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlLayoutInspectionFact> {
        vec![
            ControlLayoutInspectionFact::new("layout_roles", self.layout_roles.join(",")),
            ControlLayoutInspectionFact::new("container_kinds", self.container_kinds.join(",")),
            ControlLayoutInspectionFact::new("size_constraints", self.size_constraints.join(",")),
            ControlLayoutInspectionFact::new(
                "scroll_requirements",
                self.scroll_requirements.join(","),
            ),
            ControlLayoutInspectionFact::new("content_states", self.content_states.join(",")),
            ControlLayoutInspectionFact::new("item_identities", self.item_identities.join(",")),
            ControlLayoutInspectionFact::new(
                "selection_identities",
                self.selection_identities.join(","),
            ),
            ControlLayoutInspectionFact::new(
                "virtualization_requirements",
                self.virtualization_requirements.join(","),
            ),
            ControlLayoutInspectionFact::new(
                "large_content_budgets",
                self.large_content_budgets.join(","),
            ),
            ControlLayoutInspectionFact::new("diagnostics", self.diagnostics.join(",")),
            ControlLayoutInspectionFact::new("expected_failures", self.expected_failures.join(",")),
            ControlLayoutInspectionFact::new(
                "has_runtime_layout_behavior",
                bool_string(self.has_runtime_layout_behavior),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlLayoutInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlLayoutInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
