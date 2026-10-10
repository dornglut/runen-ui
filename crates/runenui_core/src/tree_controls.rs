//! Hierarchical collection authoring over the existing M4 real-focus group.
//! Expansion, selection and domain identity are exclusively application-owned.
use crate::{
    EventContext, EventPhase, FlexContainerStyle, FlexDirection, FocusGroup,
    FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, FocusGroupTypeAhead, Focusability,
    HitContribution, HitContributionContext, KeyboardCompositionState, KeyboardPhase,
    LayoutContainer, LayoutStyle, LogicalKey, SemanticAction, SemanticCollectionPosition,
    SemanticCommand, SemanticCommandEvent, SemanticContribution, SemanticContributionContext,
    SemanticHierarchyLevel, SemanticNodeContribution, SemanticRole, SemanticSelectionMode,
    SemanticState, UiEvent, WidgetActivation, WidgetActivationContext, WidgetActivationOutput,
    WidgetDiagnostic, WidgetEventOutput, WidgetInvalidation, WidgetMeasure, WidgetMeasureInput,
    WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::{ChildBearingWidget, Widget},
};
use core::{fmt, time::Duration};
use std::collections::BTreeSet;

/// Ordinary application requests retain domain identity, never runtime IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeItemRequest {
    Select(String),
    Expand(String),
    Collapse(String),
}

/// Compact immutable authored facts. The values remain app-owned; bits only
/// transport the current authoring snapshot to the generic Widget protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TreeItemStatus(u8);
impl TreeItemStatus {
    const SELECTED: u8 = 1;
    const EXPANDED: u8 = 1 << 1;
    const ENABLED: u8 = 1 << 2;
    const DISCOVERABLE: u8 = 1 << 3;
    const HAS_CHILDREN: u8 = 1 << 4;
    const HAS_VISIBLE_CHILD: u8 = 1 << 5;
    const ACTIONABLE: u8 = 1 << 6;
    const DEFAULT: Self = Self(Self::ENABLED);
    const fn has(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
    const fn with(self, flag: u8, value: bool) -> Self {
        Self(if value { self.0 | flag } else { self.0 & !flag })
    }
}

pub struct TreeItem<Action> {
    domain_id: String,
    label: String,
    status: TreeItemStatus,
    children: Vec<Self>,
    on_request: Option<Box<dyn FnMut(TreeItemRequest) -> Action>>,
    common: CommonNodeAuthoring,
}
impl<Action> fmt::Debug for TreeItem<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeItem")
            .field("domain_id", &self.domain_id)
            .field("label", &self.label)
            .field("selected", &self.status.has(TreeItemStatus::SELECTED))
            .field("expanded", &self.status.has(TreeItemStatus::EXPANDED))
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}
impl<Action> TreeItem<Action> {
    #[must_use]
    pub fn new(
        domain_id: impl Into<String>,
        label: impl Into<String>,
        children: impl IntoIterator<Item = Self>,
    ) -> Self {
        Self {
            domain_id: domain_id.into(),
            label: label.into(),
            status: TreeItemStatus::DEFAULT,
            children: children.into_iter().collect(),
            on_request: None,
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_direction(FlexDirection::Column),
                )),
                ..CommonNodeAuthoring::default()
            },
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub const fn selected(mut self, value: bool) -> Self {
        self.status = self.status.with(TreeItemStatus::SELECTED, value);
        self
    }
    #[must_use]
    pub const fn expanded(mut self, value: bool) -> Self {
        self.status = self.status.with(TreeItemStatus::EXPANDED, value);
        self
    }
    #[must_use]
    pub const fn enabled(mut self, value: bool) -> Self {
        self.status = self.status.with(TreeItemStatus::ENABLED, value);
        self
    }
    #[must_use]
    pub const fn discoverable_when_disabled(mut self, value: bool) -> Self {
        self.status = self.status.with(TreeItemStatus::DISCOVERABLE, value);
        self
    }
    #[must_use]
    pub fn on_request(mut self, callback: impl FnMut(TreeItemRequest) -> Action + 'static) -> Self {
        self.on_request = Some(Box::new(callback));
        self
    }
    const fn eligible(&self) -> bool {
        self.status.has(TreeItemStatus::ENABLED) || self.status.has(TreeItemStatus::DISCOVERABLE)
    }
    fn visible_eligible(&self) -> bool {
        self.eligible()
            || (self.status.has(TreeItemStatus::EXPANDED)
                && self.children.iter().any(Self::visible_eligible))
    }
    fn visible_selected(&self) -> bool {
        (self.eligible() && self.status.has(TreeItemStatus::SELECTED))
            || (self.status.has(TreeItemStatus::EXPANDED)
                && self.children.iter().any(Self::visible_selected))
    }
    /// Check all logical IDs, including collapsed descendants whose realization
    /// is absent. Mounted keys and semantic IDs are never domain identity.
    fn validate_ids(&self, seen: &mut BTreeSet<String>) -> Option<TreeAuthoringError> {
        if self.domain_id.is_empty() {
            return Some(TreeAuthoringError::EmptyDomainId);
        }
        if !seen.insert(self.domain_id.clone()) {
            return Some(TreeAuthoringError::DuplicateDomainId);
        }
        self.children
            .iter()
            .find_map(|child| child.validate_ids(seen))
    }

    fn selected_count(&self) -> usize {
        usize::from(self.status.has(TreeItemStatus::SELECTED))
            + self
                .children
                .iter()
                .map(Self::selected_count)
                .sum::<usize>()
    }

    fn build(
        self,
        level: u32,
        position: SemanticCollectionPosition,
        prefer_selected: bool,
        preferred_used: &mut bool,
    ) -> Element<Action>
    where
        Action: 'static,
    {
        let preferred = !*preferred_used
            && self.eligible()
            && (!prefer_selected || self.status.has(TreeItemStatus::SELECTED));
        if preferred {
            *preferred_used = true;
        }
        let has_children = !self.children.is_empty();
        let has_visible_child = self.status.has(TreeItemStatus::EXPANDED)
            && self.children.iter().any(Self::visible_eligible);
        let size = u64::try_from(self.children.len())
            .unwrap_or_else(|_| unreachable!("bounded mounted tree child count"));
        // Keep the ordinary M8 label as a measured child so it reserves its
        // own Flex space before hierarchical descendants; never paint text
        // directly on a container whose children would overlap it.
        let mut children = vec![
            Element::new(TreeLabelWidget {
                label: self.label.clone(),
            })
            .with_focusability(Focusability::NotFocusable),
        ];
        if self.status.has(TreeItemStatus::EXPANDED) {
            children.extend(self.children.into_iter().enumerate().map(|(index, child)| {
                let index = u64::try_from(index).unwrap_or_else(|_| unreachable!());
                let position = SemanticCollectionPosition::new(index, Some(size))
                    .unwrap_or_else(|_| unreachable!("valid tree sibling position"));
                child.build(
                    level.checked_add(1).unwrap_or_else(|| unreachable!()),
                    position,
                    prefer_selected,
                    preferred_used,
                )
            }));
        }
        let (fields, diagnostics) = self.common.into_authored_fields(
            if self.status.has(TreeItemStatus::DISCOVERABLE) {
                Focusability::FocusableWhenDisabled
            } else {
                Focusability::Focusable
            },
            None,
        );
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(TreeItemWidget {
                domain_id: self.domain_id,
                label: self.label.clone(),
                status: self
                    .status
                    .with(TreeItemStatus::HAS_CHILDREN, has_children)
                    .with(TreeItemStatus::HAS_VISIBLE_CHILD, has_visible_child)
                    .with(TreeItemStatus::ACTIONABLE, self.on_request.is_some()),
                level: SemanticHierarchyLevel::new(level).unwrap_or_else(|_| unreachable!()),
                position,
                on_request: self.on_request,
            })),
            children,
            diagnostics,
        )
        .focus_group_preferred(preferred)
        .focus_group_search_text(self.label)
    }
}

/// Passive ordinary text leaf, with no duplicate accessible name or focus target.
#[derive(Debug)]
struct TreeLabelWidget {
    label: String,
}
impl<Action> Widget<Action> for TreeLabelWidget {
    type State = String;
    fn create_state(&self) -> Self::State {
        self.label.clone()
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state != &self.label {
            context.invalidate(WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT);
        }
        state.clone_from(&self.label);
    }
    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(crate::TextLeafMeasure::new(self.label.clone()))
    }
    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TreeItemFacts {
    label: String,
    status: TreeItemStatus,
    level: SemanticHierarchyLevel,
    position: SemanticCollectionPosition,
}
struct TreeItemWidget<Action> {
    domain_id: String,
    label: String,
    status: TreeItemStatus,
    level: SemanticHierarchyLevel,
    position: SemanticCollectionPosition,
    on_request: Option<Box<dyn FnMut(TreeItemRequest) -> Action>>,
}
impl<Action> TreeItemWidget<Action> {
    fn facts(&self) -> TreeItemFacts {
        TreeItemFacts {
            label: self.label.clone(),
            status: self.status,
            level: self.level,
            position: self.position,
        }
    }
}
impl<Action> fmt::Debug for TreeItemWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeItemWidget")
            .field("domain_id", &self.domain_id)
            .field("facts", &self.facts())
            .finish_non_exhaustive()
    }
}
impl<Action> Widget<Action> for TreeItemWidget<Action> {
    type State = TreeItemFacts;
    fn create_state(&self) -> Self::State {
        self.facts()
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        let next = self.facts();
        if state.label != next.label {
            context.invalidate(WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT);
        }
        if *state != next {
            context.invalidate(WidgetInvalidation::INTERACTION | WidgetInvalidation::SEMANTICS);
        }
        *state = next;
    }
    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.on_request.is_some() {
            WidgetActivation::actionable(self.status.has(TreeItemStatus::ENABLED))
        } else if self.status.has(TreeItemStatus::ENABLED) {
            WidgetActivation::NONE
        } else {
            WidgetActivation::disabled()
        }
    }
    fn activate(
        &mut self,
        _: &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        if !self.status.has(TreeItemStatus::ENABLED) {
            return WidgetActivationOutput::none();
        }
        self.on_request
            .as_mut()
            .map_or_else(WidgetActivationOutput::none, |callback| {
                WidgetActivationOutput::action(callback(TreeItemRequest::Select(
                    self.domain_id.clone(),
                )))
            })
    }
    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        let Some(command) = event
            .as_semantic_command()
            .map(SemanticCommandEvent::command)
        else {
            return WidgetEventOutput::none();
        };
        if context.phase() == EventPhase::Bubble
            && command == SemanticCommand::FocusLeft
            && context.original_target() != context.current_target()
            && (self.status.has(TreeItemStatus::ENABLED)
                || self.status.has(TreeItemStatus::DISCOVERABLE))
        {
            // The bubbling ancestor is the exact parent focus target, not a guessed identity.
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(SemanticCommand::RequestFocus);
            return WidgetEventOutput::none();
        }
        if context.phase() != EventPhase::Target || !self.status.has(TreeItemStatus::ENABLED) {
            return WidgetEventOutput::none();
        }
        let request = match command {
            SemanticCommand::Expand | SemanticCommand::FocusRight
                if self.status.has(TreeItemStatus::HAS_CHILDREN)
                    && !self.status.has(TreeItemStatus::EXPANDED) =>
            {
                Some(TreeItemRequest::Expand(self.domain_id.clone()))
            }
            SemanticCommand::Collapse | SemanticCommand::FocusLeft
                if self.status.has(TreeItemStatus::HAS_CHILDREN)
                    && self.status.has(TreeItemStatus::EXPANDED) =>
            {
                Some(TreeItemRequest::Collapse(self.domain_id.clone()))
            }
            _ => None,
        };
        if let Some(request) = request {
            if let Some(callback) = self.on_request.as_mut() {
                context.emit(callback(request));
                context.prevent_default();
                context.stop_propagation();
            }
        } else if command == SemanticCommand::FocusRight {
            // A leaf or an expanded item without an eligible child cannot advance
            // into a sibling merely because generic focus-group movement exists.
            context.prevent_default();
            context.stop_propagation();
            if self.status.has(TreeItemStatus::EXPANDED)
                && self.status.has(TreeItemStatus::HAS_VISIBLE_CHILD)
            {
                context.emit_command(SemanticCommand::FocusGroupNext);
            }
        }
        WidgetEventOutput::none()
    }
    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = crate::LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("measured local TreeItem size is finite"));
        HitContribution::single_rect(rect)
    }
    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut flags = SemanticState::ENABLED
            .with_selected(state.status.has(TreeItemStatus::SELECTED))
            .with_disabled(!state.status.has(TreeItemStatus::ENABLED));
        if state.status.has(TreeItemStatus::HAS_CHILDREN) {
            flags = flags.with_expanded(state.status.has(TreeItemStatus::EXPANDED));
        }
        let mut node = SemanticNodeContribution::primary(SemanticRole::TreeItem)
            .with_name(state.label.clone())
            .with_state(flags)
            .with_hierarchy_level(state.level)
            .with_collection_position(state.position)
            .with_action(SemanticAction::RequestFocus);
        if state.status.has(TreeItemStatus::ACTIONABLE) && state.status.has(TreeItemStatus::ENABLED)
        {
            node = node.with_action(SemanticAction::Activate);
            if state.status.has(TreeItemStatus::HAS_CHILDREN) {
                node = node.with_action(if state.status.has(TreeItemStatus::EXPANDED) {
                    SemanticAction::Collapse
                } else {
                    SemanticAction::Expand
                });
            }
        }
        // The passive M8 label is a mounted child even for a logical leaf.
        // Project its empty semantics through the mandatory marker; otherwise
        // a leaf's TreeItem contribution is rejected and vanishes.
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl<Action> ChildBearingWidget<Action> for TreeItemWidget<Action> {}

pub struct TreeView<Action> {
    children: Vec<TreeItem<Action>>,
    common: CommonNodeAuthoring,
    selection_follows_focus: bool,
    type_ahead: FocusGroupTypeAhead,
}
impl<Action> fmt::Debug for TreeView<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeView")
            .field("children", &self.children.len())
            .field("selection_follows_focus", &self.selection_follows_focus)
            .finish_non_exhaustive()
    }
}
impl<Action> TreeView<Action> {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = TreeItem<Action>>) -> Self {
        Self {
            children: children.into_iter().collect(),
            selection_follows_focus: false,
            type_ahead: FocusGroupTypeAhead::new(Duration::from_millis(500))
                .unwrap_or_else(|_| unreachable!()),
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_direction(FlexDirection::Column),
                )),
                ..CommonNodeAuthoring::default()
            },
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub const fn selection_follows_focus(mut self, value: bool) -> Self {
        self.selection_follows_focus = value;
        self
    }
    #[must_use]
    pub const fn type_ahead(mut self, value: FocusGroupTypeAhead) -> Self {
        self.type_ahead = value;
        self
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TreeAuthoringError {
    EmptyDomainId,
    DuplicateDomainId,
    MultipleSelected,
}

#[derive(Debug)]
struct TreeViewWidget {
    invalid: Option<TreeAuthoringError>,
}
impl<Action> Widget<Action> for TreeViewWidget {
    type State = Option<TreeAuthoringError>;
    fn create_state(&self) -> Self::State {
        self.invalid
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.invalid {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::SEMANTICS
                    | WidgetInvalidation::DIAGNOSTICS,
            );
        }
        *state = self.invalid;
    }
    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if self.invalid.is_some()
            || context.phase() != EventPhase::Bubble
            || context.default_is_prevented()
        {
            return WidgetEventOutput::none();
        }
        let delegated = event
            .as_semantic_command()
            .map(SemanticCommandEvent::command)
            .and_then(|command| match command {
                SemanticCommand::FocusUp => Some(SemanticCommand::FocusGroupPrevious),
                SemanticCommand::FocusDown => Some(SemanticCommand::FocusGroupNext),
                _ => None,
            })
            .or_else(|| {
                let key = event.as_keyboard()?;
                if key.phase() != KeyboardPhase::Down
                    || key.modifiers() != crate::KeyModifiers::NONE
                    || key.composition_state() != KeyboardCompositionState::Inactive
                {
                    return None;
                }
                match key.logical_key() {
                    LogicalKey::Home => Some(SemanticCommand::FocusGroupFirst),
                    LogicalKey::End => Some(SemanticCommand::FocusGroupLast),
                    _ => None,
                }
            });
        if let Some(command) = delegated {
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(command);
        } else if event
            .as_semantic_command()
            .is_some_and(|event| event.command() == SemanticCommand::FocusLeft)
        {
            // Root has no higher tree parent; do not escape by directional fallback.
            context.prevent_default();
            context.stop_propagation();
        }
        WidgetEventOutput::none()
    }
    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        if state.is_some() {
            return SemanticContribution::empty();
        }
        let mut node = SemanticNodeContribution::primary(SemanticRole::Tree)
            .with_selection_mode(SemanticSelectionMode::Single);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
    fn diagnostics(&self, state: &Self::State) -> Vec<WidgetDiagnostic> {
        match state {
            None => Vec::new(),
            Some(TreeAuthoringError::EmptyDomainId) => vec![WidgetDiagnostic::new(
                "runenui.control.tree.empty-domain-id",
                "TreeItem domain identity must not be empty",
            )],
            Some(TreeAuthoringError::DuplicateDomainId) => vec![WidgetDiagnostic::new(
                "runenui.control.tree.duplicate-domain-id",
                "TreeItem domain identities must be unique even in collapsed subtrees",
            )],
            Some(TreeAuthoringError::MultipleSelected) => vec![WidgetDiagnostic::new(
                "runenui.control.tree.multiple-selected",
                "single-selection TreeView cannot author multiple selected TreeItems",
            )],
        }
    }
}
impl<Action> ChildBearingWidget<Action> for TreeViewWidget {}
impl<Action: 'static> View<Action> for TreeView<Action> {
    fn into_element(self) -> Element<Action> {
        let mut ids = BTreeSet::new();
        let invalid = self
            .children
            .iter()
            .find_map(|item| item.validate_ids(&mut ids))
            .or_else(|| {
                (self
                    .children
                    .iter()
                    .map(TreeItem::selected_count)
                    .sum::<usize>()
                    > 1)
                .then_some(TreeAuthoringError::MultipleSelected)
            });
        if invalid.is_some() {
            let (fields, diagnostics) = self
                .common
                .into_authored_fields(Focusability::NotFocusable, None);
            return Element::from_authored_parts(
                fields,
                Box::new(WidgetAdapter(TreeViewWidget { invalid })),
                Vec::new(),
                diagnostics,
            );
        }
        let prefer_selected = self.children.iter().any(TreeItem::visible_selected);
        let size = u64::try_from(self.children.len()).unwrap_or_else(|_| unreachable!());
        let mut used = false;
        let children = self
            .children
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let index = u64::try_from(index).unwrap_or_else(|_| unreachable!());
                let position = SemanticCollectionPosition::new(index, Some(size))
                    .unwrap_or_else(|_| unreachable!());
                item.build(1, position, prefer_selected, &mut used)
            })
            .collect();
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(TreeViewWidget { invalid: None })),
            children,
            diagnostics,
        )
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Stop)
                .with_activation(if self.selection_follows_focus {
                    FocusGroupActivationPolicy::ActivateTarget
                } else {
                    FocusGroupActivationPolicy::Manual
                })
                .with_type_ahead(self.type_ahead),
        )
    }
}
#[must_use]
pub fn tree_item<Action>(
    domain_id: impl Into<String>,
    label: impl Into<String>,
    children: impl IntoIterator<Item = TreeItem<Action>>,
) -> TreeItem<Action> {
    TreeItem::new(domain_id, label, children)
}
#[must_use]
pub fn tree_view<Action>(children: impl IntoIterator<Item = TreeItem<Action>>) -> TreeView<Action> {
    TreeView::new(children)
}
