//! File: domain/ui/ui_tree/src/inspection/mod.rs
//! Purpose: Backend-neutral retained widget inspection contracts.

use std::collections::{BTreeMap, BTreeSet};

use crate::{ComputedLayoutMap, UiNode, UiNodeKind, WidgetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleWidgetScanRequirement {
    pub require_layout_bounds: bool,
    pub require_accessible_labels: bool,
    pub require_focus_reachability: bool,
    pub require_overflow_policy: bool,
    pub required_states: BTreeSet<VisibleWidgetState>,
}

impl VisibleWidgetScanRequirement {
    pub fn strict_interactive(
        required_states: impl IntoIterator<Item = VisibleWidgetState>,
    ) -> Self {
        Self {
            require_layout_bounds: true,
            require_accessible_labels: true,
            require_focus_reachability: true,
            require_overflow_policy: true,
            required_states: required_states.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleWidgetScanEvidence {
    pub layout_bounds: ComputedLayoutMap,
    pub accessible_labels: BTreeSet<WidgetId>,
    pub focus_targets: BTreeSet<WidgetId>,
    pub overflow_policies: BTreeSet<WidgetId>,
    pub state_coverage: BTreeMap<WidgetId, BTreeSet<VisibleWidgetState>>,
}

impl VisibleWidgetScanEvidence {
    pub fn new(layout_bounds: ComputedLayoutMap) -> Self {
        Self {
            layout_bounds,
            accessible_labels: BTreeSet::new(),
            focus_targets: BTreeSet::new(),
            overflow_policies: BTreeSet::new(),
            state_coverage: BTreeMap::new(),
        }
    }

    pub fn mark_accessible_label(mut self, widget_id: WidgetId) -> Self {
        self.accessible_labels.insert(widget_id);
        self
    }

    pub fn mark_focus_target(mut self, widget_id: WidgetId) -> Self {
        self.focus_targets.insert(widget_id);
        self
    }

    pub fn mark_overflow_policy(mut self, widget_id: WidgetId) -> Self {
        self.overflow_policies.insert(widget_id);
        self
    }

    pub fn mark_state_coverage(
        mut self,
        widget_id: WidgetId,
        states: impl IntoIterator<Item = VisibleWidgetState>,
    ) -> Self {
        self.state_coverage
            .entry(widget_id)
            .or_default()
            .extend(states);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleWidgetScan {
    pub records: Vec<VisibleWidgetRecord>,
    pub issues: Vec<VisibleWidgetScanIssue>,
}

impl VisibleWidgetScan {
    pub fn passed(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleWidgetRecord {
    pub widget_id: WidgetId,
    pub kind: VisibleWidgetKind,
    pub interactive: bool,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisibleWidgetKind {
    Panel,
    Popup,
    RadialMenu,
    OverlayAdornment,
    Label,
    Button,
    TextInput,
    Toggle,
    NumericInput,
    Tabs,
    Select,
    Table,
    Tree,
    Spacer,
    Divider,
    Image,
    ProductSurface,
    GraphCanvas,
    ViewportSurfaceEmbed,
    Scroll,
    Stack,
    Split,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisibleWidgetState {
    Default,
    Hovered,
    Focused,
    Disabled,
    Selected,
    Checked,
    Error,
    Empty,
    Loading,
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleWidgetScanIssue {
    pub widget_id: WidgetId,
    pub kind: VisibleWidgetScanIssueKind,
    pub message: String,
}

impl VisibleWidgetScanIssue {
    fn new(
        widget_id: WidgetId,
        kind: VisibleWidgetScanIssueKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            widget_id,
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleWidgetScanIssueKind {
    MissingLayoutBounds,
    InvalidLayoutBounds,
    MissingAccessibleLabel,
    UnreachableFocusTarget,
    MissingOverflowPolicy,
    MissingStateCoverage,
}

pub fn scan_visible_widgets(
    root: &UiNode,
    evidence: &VisibleWidgetScanEvidence,
    requirement: &VisibleWidgetScanRequirement,
) -> VisibleWidgetScan {
    let mut records = Vec::new();
    let mut issues = Vec::new();
    scan_node(root, evidence, requirement, &mut records, &mut issues);
    VisibleWidgetScan { records, issues }
}

fn scan_node(
    node: &UiNode,
    evidence: &VisibleWidgetScanEvidence,
    requirement: &VisibleWidgetScanRequirement,
    records: &mut Vec<VisibleWidgetRecord>,
    issues: &mut Vec<VisibleWidgetScanIssue>,
) {
    let kind = VisibleWidgetKind::from_node_kind(&node.kind);
    let interactive = node_is_interactive(&node.kind);
    let label = node_accessible_label(&node.kind);

    records.push(VisibleWidgetRecord {
        widget_id: node.id,
        kind,
        interactive,
        label: label.clone(),
    });

    if requirement.require_layout_bounds {
        match evidence.layout_bounds.get(&node.id) {
            Some(layout) if layout.bounds.width > 0.0 && layout.bounds.height > 0.0 => {}
            Some(_) => issues.push(VisibleWidgetScanIssue::new(
                node.id,
                VisibleWidgetScanIssueKind::InvalidLayoutBounds,
                "visible widget has non-positive layout bounds",
            )),
            None => issues.push(VisibleWidgetScanIssue::new(
                node.id,
                VisibleWidgetScanIssueKind::MissingLayoutBounds,
                "visible widget is missing computed layout bounds",
            )),
        }
    }

    if interactive && requirement.require_accessible_labels {
        let has_label = label
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || evidence.accessible_labels.contains(&node.id);
        if !has_label {
            issues.push(VisibleWidgetScanIssue::new(
                node.id,
                VisibleWidgetScanIssueKind::MissingAccessibleLabel,
                "interactive widget is missing an accessible label",
            ));
        }
    }

    if interactive
        && requirement.require_focus_reachability
        && !evidence.focus_targets.contains(&node.id)
    {
        issues.push(VisibleWidgetScanIssue::new(
            node.id,
            VisibleWidgetScanIssueKind::UnreachableFocusTarget,
            "interactive widget is not reachable by focus traversal",
        ));
    }

    if requirement.require_overflow_policy
        && node_requires_overflow_policy(&node.kind)
        && !evidence.overflow_policies.contains(&node.id)
    {
        issues.push(VisibleWidgetScanIssue::new(
            node.id,
            VisibleWidgetScanIssueKind::MissingOverflowPolicy,
            "widget with scrollable or clipped content is missing overflow policy evidence",
        ));
    }

    if interactive && !requirement.required_states.is_empty() {
        let covered = evidence.state_coverage.get(&node.id);
        for required_state in &requirement.required_states {
            if !covered.is_some_and(|states| states.contains(required_state)) {
                issues.push(VisibleWidgetScanIssue::new(
                    node.id,
                    VisibleWidgetScanIssueKind::MissingStateCoverage,
                    format!("interactive widget is missing {required_state:?} state coverage"),
                ));
            }
        }
    }

    for child in &node.children {
        scan_node(child, evidence, requirement, records, issues);
    }
}

impl VisibleWidgetKind {
    fn from_node_kind(kind: &UiNodeKind) -> Self {
        match kind {
            UiNodeKind::Panel(_) => Self::Panel,
            UiNodeKind::Popup(_) => Self::Popup,
            UiNodeKind::RadialMenu(_) => Self::RadialMenu,
            UiNodeKind::OverlayAdornment(_) => Self::OverlayAdornment,
            UiNodeKind::Label(_) => Self::Label,
            UiNodeKind::Button(_) => Self::Button,
            UiNodeKind::TextInput(_) => Self::TextInput,
            UiNodeKind::Toggle(_) => Self::Toggle,
            UiNodeKind::NumericInput(_) => Self::NumericInput,
            UiNodeKind::Tabs(_) => Self::Tabs,
            UiNodeKind::Select(_) => Self::Select,
            UiNodeKind::Table(_) => Self::Table,
            UiNodeKind::Tree(_) => Self::Tree,
            UiNodeKind::Spacer(_) => Self::Spacer,
            UiNodeKind::Divider(_) => Self::Divider,
            UiNodeKind::Image(_) => Self::Image,
            UiNodeKind::ProductSurface(_) => Self::ProductSurface,
            UiNodeKind::GraphCanvas(_) => Self::GraphCanvas,
            UiNodeKind::ViewportSurfaceEmbed(_) => Self::ViewportSurfaceEmbed,
            UiNodeKind::Scroll(_) => Self::Scroll,
            UiNodeKind::Stack(_) => Self::Stack,
            UiNodeKind::Split(_) => Self::Split,
        }
    }
}

fn node_is_interactive(kind: &UiNodeKind) -> bool {
    match kind {
        UiNodeKind::Button(button) => button.enabled,
        UiNodeKind::TextInput(input) => input.editable,
        UiNodeKind::Toggle(toggle) => toggle.enabled,
        UiNodeKind::NumericInput(input) => input.enabled,
        UiNodeKind::Tabs(tabs) => !tabs.labels.is_empty(),
        UiNodeKind::Select(select) => select.enabled,
        UiNodeKind::Table(table) => !table.rows.is_empty(),
        UiNodeKind::Tree(tree) => tree.rows.iter().any(|row| row.enabled),
        UiNodeKind::GraphCanvas(canvas) => canvas.focusable,
        _ => false,
    }
}

fn node_requires_overflow_policy(kind: &UiNodeKind) -> bool {
    matches!(
        kind,
        UiNodeKind::Scroll(_)
            | UiNodeKind::Table(_)
            | UiNodeKind::Tree(_)
            | UiNodeKind::ProductSurface(_)
            | UiNodeKind::GraphCanvas(_)
            | UiNodeKind::ViewportSurfaceEmbed(_)
    )
}

fn node_accessible_label(kind: &UiNodeKind) -> Option<String> {
    match kind {
        UiNodeKind::Label(label) => Some(label.text.clone()),
        UiNodeKind::Button(button) => button
            .accessible_label
            .clone()
            .or_else(|| Some(button.label.clone())),
        UiNodeKind::TextInput(input) if !input.placeholder.trim().is_empty() => {
            Some(input.placeholder.clone())
        }
        UiNodeKind::TextInput(input) => Some(input.value.clone()),
        UiNodeKind::Toggle(toggle) => Some(toggle.label.clone()),
        UiNodeKind::NumericInput(input) => Some(format!("numeric value {}", input.value)),
        UiNodeKind::Tabs(tabs) => Some(tabs.labels.join(", ")),
        UiNodeKind::Select(select) => select
            .selected_index
            .and_then(|index| select.options.get(index).cloned())
            .or_else(|| Some(select.placeholder.clone())),
        UiNodeKind::Table(table) => Some(
            table
                .columns
                .iter()
                .map(|column| column.label.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        UiNodeKind::Tree(tree) => Some(
            tree.rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        UiNodeKind::GraphCanvas(_) => Some("Graph Canvas".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ButtonNode, ComputedLayout, UiNode, UiNodeKind};
    use ui_math::{UiRect, UiSize};
    use ui_text::TextStyle;
    use ui_theme::ThemeTokens;

    #[test]
    fn visible_widget_scan_reports_missing_interactive_evidence() {
        let root = UiNode::new(
            WidgetId(1),
            UiNodeKind::Button(ButtonNode::new(
                "",
                TextStyle::default(),
                ThemeTokens::default(),
            )),
        );
        let evidence = VisibleWidgetScanEvidence::new(ComputedLayoutMap::new());
        let requirement = VisibleWidgetScanRequirement::strict_interactive([
            VisibleWidgetState::Default,
            VisibleWidgetState::Focused,
        ]);

        let scan = scan_visible_widgets(&root, &evidence, &requirement);

        assert!(!scan.passed());
        assert!(scan.issues.iter().any(|issue| {
            issue.kind == VisibleWidgetScanIssueKind::MissingLayoutBounds
                && issue.widget_id == WidgetId(1)
        }));
        assert!(scan.issues.iter().any(|issue| {
            issue.kind == VisibleWidgetScanIssueKind::MissingAccessibleLabel
                && issue.widget_id == WidgetId(1)
        }));
        assert!(scan.issues.iter().any(|issue| {
            issue.kind == VisibleWidgetScanIssueKind::UnreachableFocusTarget
                && issue.widget_id == WidgetId(1)
        }));
        assert_eq!(
            scan.issues
                .iter()
                .filter(|issue| issue.kind == VisibleWidgetScanIssueKind::MissingStateCoverage)
                .count(),
            2
        );
    }

    #[test]
    fn visible_widget_scan_accepts_complete_interactive_evidence() {
        let root = UiNode::new(
            WidgetId(1),
            UiNodeKind::Button(ButtonNode::new(
                "Apply",
                TextStyle::default(),
                ThemeTokens::default(),
            )),
        );
        let mut layouts = ComputedLayoutMap::new();
        layouts.insert(
            WidgetId(1),
            ComputedLayout::new(
                UiRect::new(0.0, 0.0, 96.0, 24.0),
                UiRect::new(0.0, 0.0, 96.0, 24.0),
                UiSize::new(96.0, 24.0),
            ),
        );
        let evidence = VisibleWidgetScanEvidence::new(layouts)
            .mark_focus_target(WidgetId(1))
            .mark_state_coverage(
                WidgetId(1),
                [VisibleWidgetState::Default, VisibleWidgetState::Focused],
            );
        let requirement = VisibleWidgetScanRequirement::strict_interactive([
            VisibleWidgetState::Default,
            VisibleWidgetState::Focused,
        ]);

        let scan = scan_visible_widgets(&root, &evidence, &requirement);

        assert!(scan.passed(), "{:?}", scan.issues);
        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.records[0].kind, VisibleWidgetKind::Button);
    }
}
