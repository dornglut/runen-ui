use core::fmt;

use crate::{
    Axis, ColorValue, ElementId, ElementKey, IntoElementId, IntoElementKey, LayoutStyle,
    LogicalLength, RadiusValue, SpacingValue, StyleIntent, WidgetActivationContext,
    WidgetInvalidation, WidgetUpdateContext,
    element::{
        AuthoredElementFields, AuthoringDiagnostic, ChildLayout, ChildLayoutWidget, Element, View,
        Views, Widget, WidgetActivation, WidgetActivationOutput, WidgetMeasure, WidgetPaintProof,
        WidgetSemanticProof, WidgetTextKind,
    },
    widget_erasure::{ChildLayoutWidgetAdapter, ErasedWidget, WidgetAdapter},
};

macro_rules! common_builder_methods {
    () => {
        #[must_use]
        pub fn id(mut self, id: impl IntoElementId) -> Self {
            assign_id(&mut self.id, &mut self.diagnostics, id);
            self
        }
        #[must_use]
        pub fn key(mut self, key: impl IntoElementKey) -> Self {
            assign_key(&mut self.key, &mut self.diagnostics, key);
            self
        }
        #[must_use]
        pub fn foreground(mut self, value: impl Into<ColorValue>) -> Self {
            self.style = self.style.with_foreground(value);
            self
        }
        #[must_use]
        pub fn background(mut self, value: impl Into<ColorValue>) -> Self {
            self.style = self.style.with_background(value);
            self
        }
        #[must_use]
        pub fn padding(mut self, value: impl Into<SpacingValue>) -> Self {
            self.style = self.style.with_padding(value);
            self
        }
        #[must_use]
        pub fn radius(mut self, value: impl Into<RadiusValue>) -> Self {
            self.style = self.style.with_radius(value);
            self
        }
    };
}

#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    content: String,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    style: StyleIntent,
    diagnostics: Vec<AuthoringDiagnostic>,
}

impl Text {
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: None,
            key: None,
            style: StyleIntent::EMPTY,
            diagnostics: Vec::new(),
        }
    }
    common_builder_methods!();
    #[must_use]
    pub const fn content(&self) -> &str {
        self.content.as_str()
    }
}

#[derive(Debug)]
struct TextWidget {
    content: String,
}

impl<Action> Widget<Action> for TextWidget {
    type State = String;
    fn create_state(&self) -> Self::State {
        self.content.clone()
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.content {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
            state.clone_from(&self.content);
        }
    }
    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.content.clone(),
            kind: WidgetTextKind::Text,
            minimum_width: LogicalLength::ZERO,
            minimum_height: LogicalLength::ZERO,
        }
    }
    fn paint(&self, _: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::new("text", self.content.clone())
    }
    fn semantics(&self, _: &Self::State) -> WidgetSemanticProof {
        WidgetSemanticProof::new("text", self.content.clone())
    }
}

impl<Action: 'static> View<Action> for Text {
    fn into_element(self) -> Element<Action> {
        Element::from_authored_parts(
            AuthoredElementFields::new(
                self.id,
                self.key,
                LayoutStyle::default(),
                self.style,
                crate::Focusability::Automatic,
                None,
            ),
            Box::new(WidgetAdapter(TextWidget {
                content: self.content,
            })),
            Vec::new(),
            self.diagnostics,
        )
    }
}

pub struct Button<Action> {
    label: String,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
    style: StyleIntent,
    diagnostics: Vec<AuthoringDiagnostic>,
}

impl<Action> fmt::Debug for Button<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Button")
            .field("label", &self.label)
            .field("id", &self.id)
            .field("key", &self.key)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .field("style", &self.style)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl<Action> Button<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            id: None,
            key: None,
            enabled: true,
            activation_factory: None,
            actionable: false,
            style: StyleIntent::EMPTY,
            diagnostics: Vec::new(),
        }
    }
    common_builder_methods!();
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
    pub fn on_activate(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.activation_factory = Some(Box::new(callback));
        self.actionable = true;
        self
    }
}

struct ButtonWidget<Action> {
    label: String,
    enabled: bool,
    activation_factory: Option<Box<dyn FnMut() -> Action>>,
    actionable: bool,
}

#[derive(Debug)]
struct ButtonWidgetState {
    label: String,
    enabled: bool,
    actionable: bool,
    activation_count: u64,
}

impl<Action> fmt::Debug for ButtonWidget<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ButtonWidget")
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_callback", &self.activation_factory.is_some())
            .finish()
    }
}

impl<Action> Widget<Action> for ButtonWidget<Action> {
    type State = ButtonWidgetState;
    fn create_state(&self) -> Self::State {
        ButtonWidgetState {
            label: self.label.clone(),
            enabled: self.enabled,
            actionable: self.actionable,
            activation_count: 0,
        }
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.label != self.label {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        if state.enabled != self.enabled || state.actionable != self.actionable {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        state.label.clone_from(&self.label);
        state.enabled = self.enabled;
        state.actionable = self.actionable;
    }
    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.actionable {
            WidgetActivation::actionable(self.enabled)
        } else {
            WidgetActivation::NONE
        }
    }
    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        if self.enabled {
            state.activation_count = state.activation_count.saturating_add(1);
            context.invalidate(WidgetInvalidation::PAINT);
            self.activation_factory
                .as_mut()
                .map_or_else(WidgetActivationOutput::changed, |factory| {
                    WidgetActivationOutput::changed_with_action(factory())
                })
        } else {
            WidgetActivationOutput::none()
        }
    }
    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.label.clone(),
            kind: WidgetTextKind::ControlLabel,
            minimum_width: LogicalLength::new(64.0).unwrap_or_default(),
            minimum_height: LogicalLength::new(32.0).unwrap_or_default(),
        }
    }
    fn paint(&self, state: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::new(
            "button",
            format!(
                "label={:?} enabled={} activations={}",
                self.label, self.enabled, state.activation_count
            ),
        )
    }
    fn semantics(&self, _: &Self::State) -> WidgetSemanticProof {
        let semantics =
            WidgetSemanticProof::new("button", self.label.clone()).with_enabled(self.enabled);
        if self.actionable {
            semantics.with_action("activate")
        } else {
            semantics
        }
    }
}

impl<Action: 'static> View<Action> for Button<Action> {
    fn into_element(self) -> Element<Action> {
        Element::from_authored_parts(
            AuthoredElementFields::new(
                self.id,
                self.key,
                LayoutStyle::default(),
                self.style,
                crate::Focusability::Automatic,
                None,
            ),
            Box::new(WidgetAdapter(ButtonWidget {
                label: self.label,
                enabled: self.enabled,
                activation_factory: self.activation_factory,
                actionable: self.actionable,
            })),
            Vec::new(),
            self.diagnostics,
        )
    }
}

pub struct Container<Action> {
    widget: Box<dyn ErasedWidget<Action>>,
    children: Vec<Element<Action>>,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    diagnostics: Vec<AuthoringDiagnostic>,
}

impl<Action> fmt::Debug for Container<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Container")
            .field("widget", &self.widget)
            .field("children", &self.children)
            .field("id", &self.id)
            .field("key", &self.key)
            .field("layout", &self.layout)
            .field("style", &self.style)
            .field("diagnostics", &self.diagnostics)
            .finish()
    }
}

impl<Action> Container<Action> {
    #[must_use]
    pub fn new<Implementation>(widget: Implementation, children: impl Views<Action>) -> Self
    where
        Implementation: ChildLayoutWidget<Action> + 'static,
    {
        Self {
            widget: Box::new(ChildLayoutWidgetAdapter(widget)),
            children: children.into_elements(),
            id: None,
            key: None,
            layout: LayoutStyle::default(),
            style: StyleIntent::EMPTY,
            diagnostics: Vec::new(),
        }
    }
    common_builder_methods!();
    #[must_use]
    pub fn gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.layout = self.layout.with_gap(gap);
        self
    }
}

#[derive(Debug)]
struct LinearContainerWidget {
    axis: Axis,
}

impl<Action> Widget<Action> for LinearContainerWidget {
    type State = Axis;
    fn create_state(&self) -> Self::State {
        self.axis
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.axis {
            context.invalidate(WidgetInvalidation::LAYOUT);
            *state = self.axis;
        }
    }
    fn paint(&self, _: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::new("container", format!("axis={:?}", self.axis))
    }
    fn semantics(&self, _: &Self::State) -> WidgetSemanticProof {
        WidgetSemanticProof::new("group", "")
    }
}

impl<Action> ChildLayoutWidget<Action> for LinearContainerWidget {
    fn child_layout(&self, _: &Self::State) -> ChildLayout {
        ChildLayout::Linear { axis: self.axis }
    }
}

impl<Action: 'static> View<Action> for Container<Action> {
    fn into_element(self) -> Element<Action> {
        Element::from_authored_parts(
            AuthoredElementFields::new(
                self.id,
                self.key,
                self.layout,
                self.style,
                crate::Focusability::Automatic,
                None,
            ),
            self.widget,
            self.children,
            self.diagnostics,
        )
    }
}

#[must_use]
pub fn text(content: impl Into<String>) -> Text {
    Text::new(content)
}
#[must_use]
pub fn button<Action>(label: impl Into<String>) -> Button<Action> {
    Button::new(label)
}
#[must_use]
pub fn container<Action, Implementation>(
    widget: Implementation,
    children: impl Views<Action>,
) -> Container<Action>
where
    Implementation: ChildLayoutWidget<Action> + 'static,
{
    Container::new(widget, children)
}
#[must_use]
pub fn column<Action>(children: impl Views<Action>) -> Container<Action> {
    Container::new(
        LinearContainerWidget {
            axis: Axis::Vertical,
        },
        children,
    )
}
#[must_use]
pub fn row<Action>(children: impl Views<Action>) -> Container<Action> {
    Container::new(
        LinearContainerWidget {
            axis: Axis::Horizontal,
        },
        children,
    )
}

fn assign_id(
    slot: &mut Option<ElementId>,
    diagnostics: &mut Vec<AuthoringDiagnostic>,
    value: impl IntoElementId,
) {
    match value.into_element_id() {
        Ok(id) => *slot = Some(id),
        Err((value, error)) => diagnostics.push(AuthoringDiagnostic {
            field: "id",
            value,
            error,
        }),
    }
}

fn assign_key(
    slot: &mut Option<ElementKey>,
    diagnostics: &mut Vec<AuthoringDiagnostic>,
    value: impl IntoElementKey,
) {
    match value.into_element_key() {
        Ok(key) => *slot = Some(key),
        Err((value, error)) => diagnostics.push(AuthoringDiagnostic {
            field: "key",
            value,
            error,
        }),
    }
}
