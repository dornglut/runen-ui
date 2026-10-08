//! Standard menu authoring over the existing focus, presentation and command authorities.
//!
//! No menu owns durable expanded, checked, selection or visibility state. The
//! application remounts menu content in response to emitted ordinary actions.

use core::{fmt, marker::PhantomData, time::Duration};

use crate::{
    ApplicationCommandId, Axis, EventContext, EventPhase, FlexContainerStyle, FlexDirection,
    FocusGroup, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, FocusGroupTypeAhead,
    Focusability, HitContribution, HitContributionContext, KeyModifiers, KeyboardPhase,
    LayoutContainer, LayoutStyle, LogicalKey, LogicalRect, LogicalSize, PresentationDismissReason,
    PresentationFocusPolicy, PresentationOutsidePointerPolicy, SemanticAction,
    SemanticCheckedState, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticOrientation, SemanticPopupKind, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState,
    SurfacePresentation, SurfacePresentationAlignment, SurfacePresentationPlacement,
    SurfacePresentationSide, UiEvent, View, Views, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation,
    WidgetMeasure, WidgetMeasureInput, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::ChildBearingWidget,
};

type ActionCallback<Action> = Box<dyn FnMut() -> Action>;
type DismissCallback<Action> = Box<dyn FnMut(PresentationDismissReason) -> Action>;

fn container_layout(axis: Axis) -> LayoutStyle {
    LayoutStyle::default().with_container(LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(match axis {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        }),
    ))
}

fn menu_type_ahead() -> FocusGroupTypeAhead {
    FocusGroupTypeAhead::new(Duration::from_millis(500))
        .unwrap_or_else(|_| unreachable!("the standard Menu timeout is bounded"))
}

/// An application-mounted, nonmodal menu surface in the ordinary mounted tree.
///
/// Children may include the standard `Separator` and a standard `ScrollContainer`
/// with a `ScrollBar` for long lists. Presentation, focus and dismissal remain
/// the existing generic M11 authorities.
pub struct Menu<Action> {
    children: Vec<Element<Action>>,
    common: CommonNodeAuthoring,
    on_dismiss: Option<DismissCallback<Action>>,
    on_back: Option<ActionCallback<Action>>,
    type_ahead: FocusGroupTypeAhead,
}

impl<Action> fmt::Debug for Menu<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Menu")
            .field("children", &self.children)
            .field("presentation", &self.common.surface_presentation)
            .field("has_dismiss_callback", &self.on_dismiss.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action> Menu<Action> {
    #[must_use]
    pub fn new(children: impl Views<Action>) -> Self {
        let presentation = SurfacePresentation::new(
            SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom)
                .with_alignment(SurfacePresentationAlignment::Start),
        )
        .with_fallback(
            SurfacePresentationPlacement::new(SurfacePresentationSide::Top)
                .with_alignment(SurfacePresentationAlignment::Start),
        )
        .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
        .dismiss_on_cancel_or_back(true)
        .with_focus_policy(PresentationFocusPolicy::EnterAndRestore);
        Self {
            children: children.into_elements(),
            common: CommonNodeAuthoring {
                layout: container_layout(Axis::Vertical),
                surface_presentation: Some(presentation),
                ..CommonNodeAuthoring::default()
            },
            on_dismiss: None,
            on_back: None,
            type_ahead: menu_type_ahead(),
        }
    }

    common_node_builder_methods!();

    /// Submenus prefer right placement and fall back to the left.
    #[must_use]
    pub fn submenu(mut self) -> Self {
        self.common.surface_presentation = Some(
            SurfacePresentation::new(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Right)
                    .with_alignment(SurfacePresentationAlignment::Start),
            )
            .with_fallback(
                SurfacePresentationPlacement::new(SurfacePresentationSide::Left)
                    .with_alignment(SurfacePresentationAlignment::Start),
            )
            .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
            .dismiss_on_cancel_or_back(true)
            .with_focus_policy(PresentationFocusPolicy::EnterAndRestore),
        );
        self
    }

    #[must_use]
    pub const fn type_ahead(mut self, policy: FocusGroupTypeAhead) -> Self {
        self.type_ahead = policy;
        self
    }

    #[must_use]
    pub fn on_dismiss(
        mut self,
        callback: impl FnMut(PresentationDismissReason) -> Action + 'static,
    ) -> Self {
        self.on_dismiss = Some(Box::new(callback));
        self
    }

    /// Optional application-owned back action when focused inside a nested submenu.
    #[must_use]
    pub fn on_back(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.on_back = Some(Box::new(callback));
        self
    }
}

/// Inline menu-bar container, not an OS/native global menubar.
pub struct MenuBar<Action> {
    children: Vec<Element<Action>>,
    common: CommonNodeAuthoring,
    type_ahead: FocusGroupTypeAhead,
}

impl<Action> fmt::Debug for MenuBar<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuBar")
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl<Action> MenuBar<Action> {
    #[must_use]
    pub fn new(children: impl Views<Action>) -> Self {
        Self {
            children: children.into_elements(),
            common: CommonNodeAuthoring {
                layout: container_layout(Axis::Horizontal),
                ..CommonNodeAuthoring::default()
            },
            type_ahead: menu_type_ahead(),
        }
    }

    common_node_builder_methods!();

    #[must_use]
    pub const fn type_ahead(mut self, policy: FocusGroupTypeAhead) -> Self {
        self.type_ahead = policy;
        self
    }
}

struct MenuContainerWidget<Action> {
    role: SemanticRole,
    on_dismiss: Option<DismissCallback<Action>>,
    on_back: Option<ActionCallback<Action>>,
}

impl<Action> fmt::Debug for MenuContainerWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuContainerWidget")
            .field("role", &self.role)
            .field("has_dismiss_callback", &self.on_dismiss.is_some())
            .field("has_back_callback", &self.on_back.is_some())
            .finish()
    }
}

impl<Action> Widget<Action> for MenuContainerWidget<Action> {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && let Some(command) = event.as_semantic_command()
            && let SemanticCommand::PresentationDismiss(reason) = command.command()
            && let Some(callback) = self.on_dismiss.as_mut()
        {
            context.emit(callback(reason));
            return WidgetEventOutput::none();
        }

        if context.phase() != EventPhase::Bubble || context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        if self.role == SemanticRole::Menu
            && let Some(command) = event.as_semantic_command()
            && command.command() == SemanticCommand::FocusLeft
            && let Some(callback) = self.on_back.as_mut()
        {
            context.prevent_default();
            context.stop_propagation();
            context.emit(callback());
            return WidgetEventOutput::none();
        }
        let delegated = if let Some(command) = event.as_semantic_command() {
            match (self.role, command.command()) {
                (SemanticRole::Menu, SemanticCommand::FocusUp)
                | (SemanticRole::MenuBar, SemanticCommand::FocusLeft) => {
                    Some(SemanticCommand::FocusGroupPrevious)
                }
                (SemanticRole::Menu, SemanticCommand::FocusDown)
                | (SemanticRole::MenuBar, SemanticCommand::FocusRight) => {
                    Some(SemanticCommand::FocusGroupNext)
                }
                _ => None,
            }
        } else if let Some(keyboard) = event.as_keyboard() {
            if keyboard.phase() == KeyboardPhase::Down && keyboard.modifiers() == KeyModifiers::NONE
            {
                match keyboard.logical_key() {
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
        if let Some(delegated) = delegated {
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(delegated);
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let orientation = if self.role == SemanticRole::Menu {
            SemanticOrientation::Vertical
        } else {
            SemanticOrientation::Horizontal
        };
        let mut node = SemanticNodeContribution::primary(self.role).with_orientation(orientation);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl<Action> ChildBearingWidget<Action> for MenuContainerWidget<Action> {}

fn menu_container_element<Action: 'static>(
    common: CommonNodeAuthoring,
    children: Vec<Element<Action>>,
    role: SemanticRole,
    on_dismiss: Option<DismissCallback<Action>>,
    on_back: Option<ActionCallback<Action>>,
    type_ahead: FocusGroupTypeAhead,
) -> Element<Action> {
    let (fields, diagnostics) = common.into_authored_fields(Focusability::Automatic, None);
    Element::from_authored_parts(
        fields,
        Box::new(WidgetAdapter(MenuContainerWidget {
            role,
            on_dismiss,
            on_back,
        })),
        children,
        diagnostics,
    )
    .focus_group(
        FocusGroup::new()
            .with_boundary(FocusGroupBoundaryPolicy::Stop)
            .with_activation(FocusGroupActivationPolicy::Manual)
            .with_type_ahead(type_ahead),
    )
}

impl<Action: 'static> View<Action> for Menu<Action> {
    fn into_element(self) -> Element<Action> {
        let mut common = self.common;
        common.surface_presentation = common
            .surface_presentation
            .map(|presentation| presentation.modal(false));
        menu_container_element(
            common,
            self.children,
            SemanticRole::Menu,
            self.on_dismiss,
            self.on_back,
            self.type_ahead,
        )
    }
}

impl<Action: 'static> View<Action> for MenuBar<Action> {
    fn into_element(self) -> Element<Action> {
        menu_container_element(
            self.common,
            self.children,
            SemanticRole::MenuBar,
            None,
            None,
            self.type_ahead,
        )
    }
}

/// Public marker separating ordinary menu items from checkbox, radio and
/// menu-button authoring without creating multiple behavior backends.
pub struct PlainMenuItem;
pub struct CheckboxMenuItem;
pub struct RadioMenuItem;
pub struct MenuButtonItem;

/// An ordinary item inside `Menu` or `MenuBar`.
///
/// Selected/checked/expanded facts are authored anew by the application.
pub struct MenuEntry<Action, Kind> {
    label: String,
    role: SemanticRole,
    checked: Option<SemanticCheckedState>,
    expanded: Option<bool>,
    common: CommonNodeAuthoring,
    enabled: bool,
    discoverable_when_disabled: bool,
    activation: Option<ActionCallback<Action>>,
    command: Option<ApplicationCommandId>,
    on_expand: Option<ActionCallback<Action>>,
    on_collapse: Option<ActionCallback<Action>>,
    submenu: Option<Element<Action>>,
    marker: PhantomData<Kind>,
}

/// `MenuButton` is a semantic `Button` with popup state, not a separate role.
pub type MenuItem<Action> = MenuEntry<Action, PlainMenuItem>;
pub type MenuButton<Action> = MenuEntry<Action, MenuButtonItem>;
pub type MenuItemCheckbox<Action> = MenuEntry<Action, CheckboxMenuItem>;
pub type MenuItemRadio<Action> = MenuEntry<Action, RadioMenuItem>;

impl<Action, Kind> fmt::Debug for MenuEntry<Action, Kind> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuItem")
            .field("label", &self.label)
            .field("role", &self.role)
            .field("checked", &self.checked)
            .field("expanded", &self.expanded)
            .field("enabled", &self.enabled)
            .field("has_submenu", &self.submenu.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action, Kind> MenuEntry<Action, Kind> {
    fn with_role(label: impl Into<String>, role: SemanticRole) -> Self {
        Self {
            label: label.into(),
            role,
            checked: None,
            expanded: None,
            common: CommonNodeAuthoring::default(),
            enabled: true,
            discoverable_when_disabled: true,
            activation: None,
            command: None,
            on_expand: None,
            on_collapse: None,
            submenu: None,
            marker: PhantomData,
        }
    }

    common_node_builder_methods!();

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    #[must_use]
    pub const fn discoverable_when_disabled(mut self, discoverable: bool) -> Self {
        self.discoverable_when_disabled = discoverable;
        self
    }

    #[must_use]
    pub fn on_activate(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.activation = Some(Box::new(callback));
        self
    }

    /// Resolves through the accepted scoped queued application-command backend.
    #[must_use]
    pub fn command(mut self, command: ApplicationCommandId) -> Self {
        self.command = Some(command);
        self
    }

    /// Declares application-authored expanded state even when the submenu
    /// itself is not mounted.
    #[must_use]
    pub const fn submenu_expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// An app-conditionally mounted submenu under the owning item.
    /// The submenu's authored ID is used for the existing Controls relationship.
    #[must_use]
    pub fn with_submenu(mut self, submenu: impl View<Action>, expanded: bool) -> Self
    where
        Action: 'static,
    {
        let element = submenu.into_element();
        self.submenu = expanded.then_some(element);
        self.expanded = Some(expanded);
        self
    }

    /// App-owned submenu expansion: no framework visibility mutation occurs.
    #[must_use]
    pub fn on_expand(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.on_expand = Some(Box::new(callback));
        self
    }

    #[must_use]
    pub fn on_collapse(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.on_collapse = Some(Box::new(callback));
        self
    }
}

impl<Action> MenuEntry<Action, PlainMenuItem> {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self::with_role(label, SemanticRole::MenuItem)
    }
}

impl<Action> MenuEntry<Action, CheckboxMenuItem> {
    #[must_use]
    pub fn new(label: impl Into<String>, checked: SemanticCheckedState) -> Self {
        let mut item = Self::with_role(label, SemanticRole::MenuItemCheckbox);
        item.checked = Some(checked);
        item
    }
}

impl<Action> MenuEntry<Action, RadioMenuItem> {
    #[must_use]
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let mut item = Self::with_role(label, SemanticRole::MenuItemRadio);
        item.checked = Some(if checked {
            SemanticCheckedState::Checked
        } else {
            SemanticCheckedState::Unchecked
        });
        item
    }
}

impl<Action> MenuEntry<Action, MenuButtonItem> {
    #[must_use]
    pub fn new(label: impl Into<String>, expanded: bool) -> Self {
        let mut item = Self::with_role(label, SemanticRole::Button);
        item.expanded = Some(expanded);
        item
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MenuItemState {
    label: String,
    role: SemanticRole,
    checked: Option<SemanticCheckedState>,
    expanded: Option<bool>,
    enabled: bool,
    actionable: bool,
    submenu_id: Option<crate::ElementId>,
}

struct MenuItemWidget<Action> {
    authored: MenuItemState,
    activation: Option<ActionCallback<Action>>,
    command: Option<ApplicationCommandId>,
    on_expand: Option<ActionCallback<Action>>,
    on_collapse: Option<ActionCallback<Action>>,
}

impl<Action> fmt::Debug for MenuItemWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuItemWidget")
            .field("authored", &self.authored)
            .finish_non_exhaustive()
    }
}

impl<Action> Widget<Action> for MenuItemWidget<Action> {
    type State = MenuItemState;

    fn create_state(&self) -> Self::State {
        self.authored.clone()
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.label != self.authored.label {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        if state != &self.authored {
            context.invalidate(WidgetInvalidation::SEMANTICS);
            if state.enabled != self.authored.enabled
                || state.actionable != self.authored.actionable
            {
                context.invalidate(WidgetInvalidation::INTERACTION | WidgetInvalidation::HIT_TEST);
            }
        }
        state.clone_from(&self.authored);
    }

    fn activation(&self, state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(state.enabled && state.actionable)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        if !state.enabled {
            return WidgetActivationOutput::none();
        }
        if let Some(command) = self.command.clone() {
            context.emit_application_command(command);
        }
        if let Some(callback) = self.activation.as_mut() {
            return WidgetActivationOutput::action(callback());
        }
        let fallback = if state.expanded == Some(true) {
            &mut self.on_collapse
        } else {
            &mut self.on_expand
        };
        fallback
            .as_mut()
            .map_or_else(WidgetActivationOutput::none, |callback| {
                WidgetActivationOutput::action(callback())
            })
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if !state.enabled || context.phase() != EventPhase::Target || context.default_is_prevented()
        {
            return WidgetEventOutput::none();
        }
        let Some(command) = event.as_semantic_command() else {
            return WidgetEventOutput::none();
        };
        let opening = matches!(
            command.command(),
            SemanticCommand::Expand | SemanticCommand::OpenMenu
        ) || (state.role == SemanticRole::Button
            && command.command() == SemanticCommand::FocusDown)
            || (state.role != SemanticRole::Button
                && command.command() == SemanticCommand::FocusRight);
        let closing = command.command() == SemanticCommand::Collapse
            || (state.role != SemanticRole::Button
                && command.command() == SemanticCommand::FocusLeft);
        let callback = if opening && state.expanded == Some(false) {
            &mut self.on_expand
        } else if closing && state.expanded == Some(true) {
            &mut self.on_collapse
        } else {
            return WidgetEventOutput::none();
        };
        if let Some(callback) = callback.as_mut() {
            context.prevent_default();
            context.stop_propagation();
            context.emit(callback());
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, state: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: state.label.clone(),
        }
    }

    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        if !state.actionable {
            return HitContribution::empty();
        }
        let size: LogicalSize = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("a validated local size is finite")),
        )
    }

    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut semantic_state = SemanticState::ENABLED.with_disabled(!state.enabled);
        if let Some(checked) = state.checked {
            semantic_state = semantic_state.with_checked(checked);
        }
        if let Some(expanded) = state.expanded {
            semantic_state = semantic_state.with_expanded(expanded);
        }
        let mut node = SemanticNodeContribution::primary(state.role)
            .with_name(state.label.clone())
            .with_state(semantic_state)
            .with_action(SemanticAction::RequestFocus);
        if state.expanded.is_some() {
            node = node.with_popup(SemanticPopupKind::Menu);
            if state.enabled {
                node = node.with_action(SemanticAction::OpenMenu);
                if self.on_expand.is_some() {
                    node = node.with_action(SemanticAction::Expand);
                }
                if self.on_collapse.is_some() {
                    node = node.with_action(SemanticAction::Collapse);
                }
            }
        }
        if let Some(id) = &state.submenu_id {
            node = node.with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Authored {
                    element_id: id.clone(),
                    semantic_key: None,
                },
            ));
        }
        if state.enabled && state.actionable {
            node = node.with_action(SemanticAction::Activate);
        }
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl<Action> ChildBearingWidget<Action> for MenuItemWidget<Action> {}

impl<Action: 'static, Kind: 'static> View<Action> for MenuEntry<Action, Kind> {
    fn into_element(self) -> Element<Action> {
        let submenu_id = self
            .submenu
            .as_ref()
            .and_then(|element| element.element_id().cloned());
        let authored = MenuItemState {
            label: self.label.clone(),
            role: self.role,
            checked: self.checked,
            expanded: self.expanded,
            enabled: self.enabled,
            actionable: self.activation.is_some()
                || self.command.is_some()
                || self.on_expand.is_some()
                || self.on_collapse.is_some(),
            submenu_id,
        };
        let (fields, diagnostics) = self.common.into_authored_fields(
            if self.discoverable_when_disabled {
                Focusability::FocusableWhenDisabled
            } else {
                Focusability::Focusable
            },
            None,
        );
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(MenuItemWidget {
                authored,
                activation: self.activation,
                command: self.command,
                on_expand: self.on_expand,
                on_collapse: self.on_collapse,
            })),
            self.submenu.into_iter().collect(),
            diagnostics,
        )
        .focus_group_search_text(self.label)
    }
}
