//! Standard toolbar composition over the accepted real-focus group and app-command paths.
//!
//! The toolbar owns neither child activation state nor application command resolution.
use core::fmt;

use crate::{
    Axis, EventContext, EventPhase, FlexContainerStyle, FlexDirection, FocusGroup,
    FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, Focusability, KeyModifiers,
    KeyboardPhase, LayoutContainer, LayoutStyle, LogicalKey, LogicalLength, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticOrientation, SemanticRole, UiEvent, View, Views, Widget, WidgetEventOutput,
    WidgetInvalidation, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::ChildBearingWidget,
};

fn toolbar_container(orientation: Axis) -> LayoutContainer {
    LayoutContainer::Flex(
        FlexContainerStyle::default().with_direction(match orientation {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        }),
    )
}

fn toolbar_layout(orientation: Axis) -> LayoutStyle {
    LayoutStyle::default().with_container(toolbar_container(orientation))
}

/// A logical toolbar composed from ordinary public child controls.
///
/// The existing focus group supplies one external tab stop, remembered/first
/// eligible focus entry and scope-bounded real-focus movement. Children own
/// their activation, selected/pressed state and commands. A nested `RadioGroup`
/// uses `.standalone_navigation(false)` so outer arrow movement remains owned
/// by the Toolbar, not a second radio navigation handler.
pub struct Toolbar<Action> {
    children: Vec<Element<Action>>,
    common: CommonNodeAuthoring,
    orientation: Axis,
    boundary: FocusGroupBoundaryPolicy,
    accessible_name: Option<String>,
}

impl<Action> fmt::Debug for Toolbar<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Toolbar")
            .field("children", &self.children)
            .field("orientation", &self.orientation)
            .field("boundary", &self.boundary)
            .field("accessible_name", &self.accessible_name)
            .finish_non_exhaustive()
    }
}

impl<Action> Toolbar<Action> {
    #[must_use]
    pub fn new(children: impl Views<Action>) -> Self {
        Self {
            children: children.into_elements(),
            common: CommonNodeAuthoring {
                layout: toolbar_layout(Axis::Horizontal),
                ..CommonNodeAuthoring::default()
            },
            orientation: Axis::Horizontal,
            boundary: FocusGroupBoundaryPolicy::Stop,
            accessible_name: None,
        }
    }

    common_node_builder_methods!();

    #[must_use]
    pub fn accessible_name(mut self, name: impl Into<String>) -> Self {
        self.accessible_name = Some(name.into());
        self
    }

    #[must_use]
    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        let direction = match orientation {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        };
        let container = match self.common.layout.container() {
            LayoutContainer::Flex(style) => {
                LayoutContainer::Flex((*style).with_direction(direction))
            }
            _ => toolbar_container(orientation),
        };
        self.common.layout = self.common.layout.with_container(container);
        self
    }

    /// Selects ordinary Stop/Wrap ordered focus-group movement.
    #[must_use]
    pub const fn boundary(mut self, boundary: FocusGroupBoundaryPolicy) -> Self {
        self.boundary = boundary;
        self
    }

    #[must_use]
    pub fn gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.common.layout = self.common.layout.with_gap(gap);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ToolbarState {
    orientation: Axis,
    accessible_name: Option<String>,
}

#[derive(Debug)]
struct ToolbarWidget {
    orientation: Axis,
    accessible_name: Option<String>,
}

impl<Action> Widget<Action> for ToolbarWidget {
    type State = ToolbarState;

    fn create_state(&self) -> Self::State {
        ToolbarState {
            orientation: self.orientation,
            accessible_name: self.accessible_name.clone(),
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.orientation != self.orientation || state.accessible_name != self.accessible_name {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        state.orientation = self.orientation;
        state.accessible_name.clone_from(&self.accessible_name);
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Bubble || context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        let delegated = if let Some(command) = event.as_semantic_command() {
            match (self.orientation, command.command()) {
                (Axis::Horizontal, SemanticCommand::FocusLeft)
                | (Axis::Vertical, SemanticCommand::FocusUp) => {
                    Some(SemanticCommand::FocusGroupPrevious)
                }
                (Axis::Horizontal, SemanticCommand::FocusRight)
                | (Axis::Vertical, SemanticCommand::FocusDown) => {
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
        if let Some(command) = delegated {
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(command);
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let orientation = match state.orientation {
            Axis::Horizontal => SemanticOrientation::Horizontal,
            Axis::Vertical => SemanticOrientation::Vertical,
        };
        let mut node =
            SemanticNodeContribution::primary(SemanticRole::Toolbar).with_orientation(orientation);
        if let Some(name) = &state.accessible_name {
            node = node.with_name(name.clone());
        }
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl<Action> ChildBearingWidget<Action> for ToolbarWidget {}

impl<Action: 'static> View<Action> for Toolbar<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ToolbarWidget {
                orientation: self.orientation,
                accessible_name: self.accessible_name,
            })),
            self.children,
            diagnostics,
        )
        .focus_group(
            FocusGroup::new()
                .with_boundary(self.boundary)
                .with_activation(FocusGroupActivationPolicy::Manual),
        )
    }
}

/// Ordinary mixed-control toolbar; all durable child state stays application-owned.
#[must_use]
pub fn toolbar<Action>(children: impl Views<Action>) -> Toolbar<Action> {
    Toolbar::new(children)
}
