//! Typed host-neutral UI element tree and builders.

use crate::{
    Axis, ColorValue, ElementId, ElementKey, IdentifierError, IntoElementId, IntoElementKey,
    LayoutStyle, LogicalLength, RadiusValue, SpacingValue, StyleIntent,
};

/// Type-erased built-in element description consumed by the current runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct Element<Action> {
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    kind: ElementKind<Action>,
    authoring_diagnostics: Vec<AuthoringDiagnostic>,
}

/// Invalid authored configuration retained for deterministic runtime reporting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoringDiagnostic {
    field: &'static str,
    value: String,
    error: IdentifierError,
}

impl AuthoringDiagnostic {
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }
    #[must_use]
    pub const fn value(&self) -> &str {
        self.value.as_str()
    }
    #[must_use]
    pub const fn error(&self) -> IdentifierError {
        self.error
    }
}

impl<Action> Element<Action> {
    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }

    #[must_use]
    pub const fn element_key(&self) -> Option<&ElementKey> {
        self.key.as_ref()
    }

    #[must_use]
    pub const fn layout(&self) -> &LayoutStyle {
        &self.layout
    }

    #[must_use]
    pub const fn style(&self) -> &StyleIntent {
        &self.style
    }

    #[must_use]
    pub const fn kind(&self) -> &ElementKind<Action> {
        &self.kind
    }

    #[must_use]
    pub const fn authoring_diagnostics(&self) -> &[AuthoringDiagnostic] {
        self.authoring_diagnostics.as_slice()
    }
}

/// Converts a typed builder or existing erased element into [`Element`].
pub trait IntoElement<Action> {
    fn into_element(self) -> Element<Action>;
}

impl<Action> IntoElement<Action> for Element<Action> {
    fn into_element(self) -> Self {
        self
    }
}

/// Typed text-element builder.
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

    #[must_use]
    pub const fn content(&self) -> &str {
        self.content.as_str()
    }
}

impl<Action> IntoElement<Action> for Text {
    fn into_element(self) -> Element<Action> {
        Element {
            id: self.id,
            key: self.key,
            layout: LayoutStyle::default(),
            style: self.style,
            kind: ElementKind::Text(TextElement {
                content: self.content,
            }),
            authoring_diagnostics: self.diagnostics,
        }
    }
}

/// Typed button-element builder.
#[derive(Clone, Debug, PartialEq)]
pub struct Button<Action> {
    label: String,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    enabled: bool,
    on_press: Option<Action>,
    style: StyleIntent,
    diagnostics: Vec<AuthoringDiagnostic>,
}

impl<Action> Button<Action> {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            id: None,
            key: None,
            enabled: true,
            on_press: None,
            style: StyleIntent::EMPTY,
            diagnostics: Vec::new(),
        }
    }

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
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    #[must_use]
    pub fn on_press(mut self, action: Action) -> Self {
        self.on_press = Some(action);
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
}

impl<Action> IntoElement<Action> for Button<Action> {
    fn into_element(self) -> Element<Action> {
        Element {
            id: self.id,
            key: self.key,
            layout: LayoutStyle::default(),
            style: self.style,
            kind: ElementKind::Button(ButtonElement {
                label: self.label,
                enabled: self.enabled,
                on_press: self.on_press,
            }),
            authoring_diagnostics: self.diagnostics,
        }
    }
}

/// Typed row/column container builder.
#[derive(Clone, Debug, PartialEq)]
pub struct Container<Action> {
    axis: Axis,
    children: Vec<Element<Action>>,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    diagnostics: Vec<AuthoringDiagnostic>,
}

impl<Action> Container<Action> {
    #[must_use]
    pub fn new(axis: Axis, children: impl IntoElements<Action>) -> Self {
        Self {
            axis,
            children: children.into_elements(),
            id: None,
            key: None,
            layout: LayoutStyle::default(),
            style: StyleIntent::EMPTY,
            diagnostics: Vec::new(),
        }
    }

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
    pub fn gap(mut self, gap: impl Into<LogicalLength>) -> Self {
        self.layout = self.layout.with_gap(gap);
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
}

impl<Action> IntoElement<Action> for Container<Action> {
    fn into_element(self) -> Element<Action> {
        Element {
            id: self.id,
            key: self.key,
            layout: self.layout,
            style: self.style,
            kind: ElementKind::Container(ContainerElement {
                axis: self.axis,
                children: self.children,
            }),
            authoring_diagnostics: self.diagnostics,
        }
    }
}

/// Closed built-in proof vocabulary. M2 owns replacement with an open widget protocol.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementKind<Action> {
    Text(TextElement),
    Button(ButtonElement<Action>),
    Container(ContainerElement<Action>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextElement {
    content: String,
}

impl TextElement {
    #[must_use]
    pub const fn content(&self) -> &str {
        self.content.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonElement<Action> {
    label: String,
    enabled: bool,
    on_press: Option<Action>,
}

impl<Action> ButtonElement<Action> {
    #[must_use]
    pub const fn label(&self) -> &str {
        self.label.as_str()
    }
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
    #[must_use]
    pub const fn on_press(&self) -> Option<&Action> {
        self.on_press.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerElement<Action> {
    axis: Axis,
    children: Vec<Element<Action>>,
}

impl<Action> ContainerElement<Action> {
    #[must_use]
    pub const fn axis(&self) -> Axis {
        self.axis
    }
    #[must_use]
    pub const fn children(&self) -> &[Element<Action>] {
        self.children.as_slice()
    }
}

/// Converts an arbitrary iterator or collection of typed builders into children.
pub trait IntoElements<Action> {
    fn into_elements(self) -> Vec<Element<Action>>;
}

impl<Action, Items, Item> IntoElements<Action> for Items
where
    Items: IntoIterator<Item = Item>,
    Item: IntoElement<Action>,
{
    fn into_elements(self) -> Vec<Element<Action>> {
        self.into_iter().map(IntoElement::into_element).collect()
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
pub fn column<Action>(children: impl IntoElements<Action>) -> Container<Action> {
    Container::new(Axis::Vertical, children)
}

#[must_use]
pub fn row<Action>(children: impl IntoElements<Action>) -> Container<Action> {
    Container::new(Axis::Horizontal, children)
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
