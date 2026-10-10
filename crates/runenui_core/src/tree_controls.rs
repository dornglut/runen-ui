//! Hierarchical collection authoring over the existing M4 real-focus group.
//! Expansion, selection and domain identity are exclusively application-owned.
use core::{fmt, time::Duration};
use crate::{
    EventContext, EventPhase, FlexContainerStyle, FlexDirection, FocusGroup,
    FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, FocusGroupTypeAhead, Focusability,
    KeyboardCompositionState, KeyboardPhase, LayoutContainer, LayoutStyle, LogicalKey,
    SemanticAction, SemanticCollectionPosition, SemanticCommand, SemanticCommandEvent,
    SemanticContribution, SemanticContributionContext, SemanticHierarchyLevel,
    SemanticNodeContribution, SemanticRole, SemanticSelectionMode, SemanticState, UiEvent,
    WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput,
    WidgetInvalidation, WidgetMeasure, WidgetMeasureInput, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
    widget_erasure::WidgetAdapter, widget_protocol::{ChildBearingWidget, Widget},
};

/// Ordinary application requests retain domain identity, never runtime IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TreeItemRequest {
    Select(String),
    Expand(String),
    Collapse(String),
}

pub struct TreeItem<Action> {
    domain_id: String,
    label: String,
    selected: bool,
    expanded: bool,
    enabled: bool,
    discoverable: bool,
    children: Vec<Self>,
    on_request: Option<Box<dyn FnMut(TreeItemRequest) -> Action>>,
    common: CommonNodeAuthoring,
}
impl<Action> fmt::Debug for TreeItem<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeItem").field("domain_id", &self.domain_id)
            .field("label", &self.label).field("selected", &self.selected)
            .field("expanded", &self.expanded).field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}
impl<Action> TreeItem<Action> {
    #[must_use]
    pub fn new(domain_id: impl Into<String>, label: impl Into<String>,
        children: impl IntoIterator<Item = Self>) -> Self {
        Self {
            domain_id: domain_id.into(), label: label.into(), selected: false,
            expanded: false, enabled: true, discoverable: false,
            children: children.into_iter().collect(), on_request: None,
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_direction(FlexDirection::Column),
                )), ..CommonNodeAuthoring::default()
            },
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub const fn selected(mut self, value: bool) -> Self { self.selected = value; self }
    #[must_use]
    pub const fn expanded(mut self, value: bool) -> Self { self.expanded = value; self }
    #[must_use]
    pub const fn enabled(mut self, value: bool) -> Self { self.enabled = value; self }
    #[must_use]
    pub const fn discoverable_when_disabled(mut self, value: bool) -> Self {
        self.discoverable = value; self
    }
    #[must_use]
    pub fn on_request(mut self, callback: impl FnMut(TreeItemRequest) -> Action + 'static) -> Self {
        self.on_request = Some(Box::new(callback)); self
    }
    fn eligible(&self) -> bool { self.enabled || self.discoverable }
    fn visible_eligible(&self) -> bool {
        self.eligible() || (self.expanded && self.children.iter().any(Self::visible_eligible))
    }
    fn visible_selected(&self) -> bool {
        (self.eligible() && self.selected)
            || (self.expanded && self.children.iter().any(Self::visible_selected))
    }
    fn build(self, level: u32, position: SemanticCollectionPosition,
        prefer_selected: bool, preferred_used: &mut bool) -> Element<Action>
    where Action: 'static {
        let preferred = !*preferred_used && self.eligible()
            && (!prefer_selected || self.selected);
        if preferred { *preferred_used = true; }
        let has_children = !self.children.is_empty();
        let has_visible_child = self.expanded
            && self.children.iter().any(Self::visible_eligible);
        let size = u64::try_from(self.children.len())
            .unwrap_or_else(|_| unreachable!("bounded mounted tree child count"));
        let children = if self.expanded {
            self.children.into_iter().enumerate().map(|(index, child)| {
                let index = u64::try_from(index).unwrap_or_else(|_| unreachable!());
                let position = SemanticCollectionPosition::new(index, Some(size))
                    .unwrap_or_else(|_| unreachable!("valid tree sibling position"));
                child.build(level.checked_add(1).unwrap_or_else(|| unreachable!()),
                    position, prefer_selected, preferred_used)
            }).collect()
        } else { Vec::new() };
        let (fields, diagnostics) = self.common.into_authored_fields(
            if self.discoverable { Focusability::FocusableWhenDisabled }
            else { Focusability::Focusable }, None,
        );
        Element::from_authored_parts(fields, Box::new(WidgetAdapter(TreeItemWidget {
            domain_id: self.domain_id, label: self.label.clone(), selected: self.selected,
            expanded: self.expanded, enabled: self.enabled, discoverable: self.discoverable,
            has_children, has_visible_child,
            level: SemanticHierarchyLevel::new(level).unwrap_or_else(|_| unreachable!()),
            position, on_request: self.on_request,
        })), children, diagnostics)
            .focus_group_preferred(preferred).focus_group_search_text(self.label)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TreeItemFacts {
    label: String, selected: bool, expanded: bool, enabled: bool, discoverable: bool,
    has_children: bool, has_visible_child: bool,
    level: SemanticHierarchyLevel, position: SemanticCollectionPosition, actionable: bool,
}
struct TreeItemWidget<Action> {
    domain_id: String, label: String, selected: bool, expanded: bool,
    enabled: bool, discoverable: bool, has_children: bool, has_visible_child: bool,
    level: SemanticHierarchyLevel, position: SemanticCollectionPosition,
    on_request: Option<Box<dyn FnMut(TreeItemRequest) -> Action>>,
}
impl<Action> TreeItemWidget<Action> {
    fn facts(&self) -> TreeItemFacts {
        TreeItemFacts {
            label: self.label.clone(), selected: self.selected, expanded: self.expanded,
            enabled: self.enabled, discoverable: self.discoverable,
            has_children: self.has_children, has_visible_child: self.has_visible_child,
            level: self.level, position: self.position, actionable: self.on_request.is_some(),
        }
    }
}
impl<Action> fmt::Debug for TreeItemWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeItemWidget").field("domain_id", &self.domain_id)
            .field("facts", &self.facts()).finish_non_exhaustive()
    }
}
impl<Action> Widget<Action> for TreeItemWidget<Action> {
    type State = TreeItemFacts;
    fn create_state(&self) -> Self::State { self.facts() }
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
        if self.on_request.is_some() { WidgetActivation::actionable(self.enabled) }
        else if self.enabled { WidgetActivation::NONE }
        else { WidgetActivation::disabled() }
    }
    fn activate(&mut self, _: &mut Self::State, _: &mut WidgetActivationContext<Action>)
        -> WidgetActivationOutput<Action> {
        if !self.enabled { return WidgetActivationOutput::none(); }
        self.on_request.as_mut().map_or_else(WidgetActivationOutput::none, |callback| {
            WidgetActivationOutput::action(callback(TreeItemRequest::Select(self.domain_id.clone())))
        })
    }
    fn event(&mut self, _: &mut Self::State, event: &UiEvent,
        context: &mut EventContext<'_, Action>) -> WidgetEventOutput {
        if context.default_is_prevented() { return WidgetEventOutput::none(); }
        let Some(command) = event.as_semantic_command().map(SemanticCommandEvent::command)
        else { return WidgetEventOutput::none(); };
        if context.phase() == EventPhase::Bubble
            && command == SemanticCommand::FocusLeft
            && context.original_target() != context.current_target()
            && (self.enabled || self.discoverable)
        {
            // The bubbling ancestor is the exact parent focus target, not a guessed identity.
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(SemanticCommand::RequestFocus);
            return WidgetEventOutput::none();
        }
        if context.phase() != EventPhase::Target || !self.enabled {
            return WidgetEventOutput::none();
        }
        let request = match command {
            SemanticCommand::Expand | SemanticCommand::FocusRight
                if self.has_children && !self.expanded
                => Some(TreeItemRequest::Expand(self.domain_id.clone())),
            SemanticCommand::Collapse | SemanticCommand::FocusLeft
                if self.has_children && self.expanded
                => Some(TreeItemRequest::Collapse(self.domain_id.clone())),
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
            if self.expanded && self.has_visible_child {
                context.emit_command(SemanticCommand::FocusGroupNext);
            }
        }
        WidgetEventOutput::none()
    }
    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(crate::TextLeafMeasure::new(self.label.clone()))
    }
    fn semantics(&self, state: &Self::State, context: SemanticContributionContext)
        -> SemanticContribution {
        let mut flags = SemanticState::ENABLED.with_selected(state.selected)
            .with_disabled(!state.enabled);
        if state.has_children { flags = flags.with_expanded(state.expanded); }
        let mut node = SemanticNodeContribution::primary(SemanticRole::TreeItem)
            .with_name(state.label.clone()).with_state(flags)
            .with_hierarchy_level(state.level).with_collection_position(state.position)
            .with_action(SemanticAction::RequestFocus);
        if state.actionable && state.enabled {
            node = node.with_action(SemanticAction::Activate);
            if state.has_children {
                node = node.with_action(if state.expanded {
                    SemanticAction::Collapse
                } else { SemanticAction::Expand });
            }
        }
        if context.has_mounted_children() { node = node.with_mounted_children(); }
        SemanticContribution::single(node)
    }
}
impl<Action> ChildBearingWidget<Action> for TreeItemWidget<Action> {}

pub struct TreeView<Action> {
    children: Vec<TreeItem<Action>>, common: CommonNodeAuthoring,
    selection_follows_focus: bool, type_ahead: FocusGroupTypeAhead,
}
impl<Action> fmt::Debug for TreeView<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeView").field("children", &self.children.len())
            .field("selection_follows_focus", &self.selection_follows_focus)
            .finish_non_exhaustive()
    }
}
impl<Action> TreeView<Action> {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = TreeItem<Action>>) -> Self {
        Self {
            children: children.into_iter().collect(), selection_follows_focus: false,
            type_ahead: FocusGroupTypeAhead::new(Duration::from_millis(500))
                .unwrap_or_else(|_| unreachable!()),
            common: CommonNodeAuthoring {
                layout: LayoutStyle::default().with_container(LayoutContainer::Flex(
                    FlexContainerStyle::default().with_direction(FlexDirection::Column),
                )), ..CommonNodeAuthoring::default()
            },
        }
    }
    common_node_builder_methods!();
    #[must_use]
    pub const fn selection_follows_focus(mut self, value: bool) -> Self {
        self.selection_follows_focus = value; self
    }
    #[must_use]
    pub const fn type_ahead(mut self, value: FocusGroupTypeAhead) -> Self {
        self.type_ahead = value; self
    }
}
#[derive(Debug)]
struct TreeViewWidget;
impl<Action> Widget<Action> for TreeViewWidget {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Bubble || context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        let delegated = if let Some(command) = event
            .as_semantic_command()
            .map(SemanticCommandEvent::command)
        {
            match command {
                SemanticCommand::FocusUp => Some(SemanticCommand::FocusGroupPrevious),
                SemanticCommand::FocusDown => Some(SemanticCommand::FocusGroupNext),
                _ => None,
            }
        } else if let Some(key) = event.as_keyboard() {
            if key.phase() == KeyboardPhase::Down
                && key.modifiers() == crate::KeyModifiers::NONE
                && key.composition_state() == KeyboardCompositionState::Inactive
            {
                match key.logical_key() {
                    LogicalKey::Home => Some(SemanticCommand::FocusGroupFirst),
                    LogicalKey::End => Some(SemanticCommand::FocusGroupLast),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };
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
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Tree)
            .with_selection_mode(SemanticSelectionMode::Single);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl<Action> ChildBearingWidget<Action> for TreeViewWidget {}
impl<Action: 'static> View<Action> for TreeView<Action> {
    fn into_element(self) -> Element<Action> {
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
            Box::new(WidgetAdapter(TreeViewWidget)),
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
