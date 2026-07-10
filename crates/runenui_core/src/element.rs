//! Typed host-neutral UI element tree.

use crate::{Axis, ElementId, ElementKey, LayoutStyle};

#[derive(Clone, Debug, PartialEq)]
pub struct Element<Action> {
    id: Option<ElementId>,
    key: Option<ElementKey>,
    style: LayoutStyle,
    kind: ElementKind<Action>,
}

impl<Action> Element<Action> {
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self::text_with(TextArgs::new(content))
    }

    #[must_use]
    pub fn text_with(args: TextArgs) -> Self {
        Self {
            id: args.id,
            key: args.key,
            style: LayoutStyle::default(),
            kind: ElementKind::Text(TextElement::new(args.content)),
        }
    }

    #[must_use]
    pub fn button(label: impl Into<String>) -> Self {
        Self::button_with(ButtonArgs::new(label))
    }

    #[must_use]
    pub fn button_with(args: ButtonArgs<Action>) -> Self {
        let mut button = ButtonElement::new(args.label);
        button.enabled = args.enabled;
        button.on_press = args.on_press;

        Self {
            id: args.id,
            key: args.key,
            style: LayoutStyle::default(),
            kind: ElementKind::Button(button),
        }
    }

    #[must_use]
    pub fn container(axis: Axis, children: impl IntoElements<Action>) -> Self {
        Self::container_with(ContainerArgs::new(axis, children))
    }

    #[must_use]
    pub fn container_with(args: ContainerArgs<Action>) -> Self {
        Self {
            id: args.id,
            key: args.key,
            style: args.style,
            kind: ElementKind::Container(ContainerElement::new(args.axis, args.children)),
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    #[must_use]
    pub fn key(mut self, key: impl Into<ElementKey>) -> Self {
        self.key = Some(key.into());
        self
    }

    #[must_use]
    pub fn gap(mut self, gap: impl Into<crate::Px>) -> Self {
        self.style = self.style.with_gap(gap);
        self
    }

    #[must_use]
    pub fn on_press(mut self, action: Action) -> Self {
        if let ElementKind::Button(button) = &mut self.kind {
            button.on_press = Some(action);
        }
        self
    }

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        if let ElementKind::Button(button) = &mut self.kind {
            button.enabled = enabled;
        }
        self
    }

    #[must_use]
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }

    #[must_use]
    pub const fn element_key(&self) -> Option<&ElementKey> {
        self.key.as_ref()
    }

    #[must_use]
    pub const fn style(&self) -> &LayoutStyle {
        &self.style
    }

    #[must_use]
    pub const fn kind(&self) -> &ElementKind<Action> {
        &self.kind
    }
}

impl<Action> From<TextArgs> for Element<Action> {
    fn from(args: TextArgs) -> Self {
        Self::text_with(args)
    }
}

impl<Action> From<ButtonArgs<Action>> for Element<Action> {
    fn from(args: ButtonArgs<Action>) -> Self {
        Self::button_with(args)
    }
}

impl<Action> From<ContainerArgs<Action>> for Element<Action> {
    fn from(args: ContainerArgs<Action>) -> Self {
        Self::container_with(args)
    }
}

/// Explicit construction arguments for a text element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextArgs {
    content: String,
    id: Option<ElementId>,
    key: Option<ElementKey>,
}

impl TextArgs {
    /// Creates text arguments from text content.
    #[must_use]
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: None,
            key: None,
        }
    }

    /// Adds an authored element ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Adds a stable data key.
    #[must_use]
    pub fn key(mut self, key: impl Into<ElementKey>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Returns the text content.
    #[must_use]
    pub const fn content(&self) -> &str {
        self.content.as_str()
    }

    /// Returns the authored element ID, if present.
    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }

    /// Returns the stable data key, if present.
    #[must_use]
    pub const fn element_key(&self) -> Option<&ElementKey> {
        self.key.as_ref()
    }
}

/// Explicit construction arguments for a button element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonArgs<Action> {
    label: String,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    enabled: bool,
    on_press: Option<Action>,
}

impl<Action> ButtonArgs<Action> {
    /// Creates button arguments from a button label.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            id: None,
            key: None,
            enabled: true,
            on_press: None,
        }
    }

    /// Adds an authored element ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Adds a stable data key.
    #[must_use]
    pub fn key(mut self, key: impl Into<ElementKey>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Sets whether the button is enabled.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disables the button.
    #[must_use]
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Sets the action dispatched when the button is pressed.
    #[must_use]
    pub fn on_press(mut self, action: Action) -> Self {
        self.on_press = Some(action);
        self
    }

    /// Returns the button label.
    #[must_use]
    pub const fn label(&self) -> &str {
        self.label.as_str()
    }

    /// Returns the authored element ID, if present.
    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }

    /// Returns the stable data key, if present.
    #[must_use]
    pub const fn element_key(&self) -> Option<&ElementKey> {
        self.key.as_ref()
    }

    /// Returns whether the button is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the action dispatched on press, if present.
    #[must_use]
    pub const fn on_press_action(&self) -> Option<&Action> {
        self.on_press.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerArgs<Action> {
    axis: Axis,
    children: Vec<Element<Action>>,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    style: LayoutStyle,
}

impl<Action> ContainerArgs<Action> {
    /// Creates container arguments from an axis and children.
    #[must_use]
    pub fn new(axis: Axis, children: impl IntoElements<Action>) -> Self {
        Self {
            axis,
            children: children.into_elements(),
            id: None,
            key: None,
            style: LayoutStyle::default(),
        }
    }

    /// Adds an authored element ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Adds a stable data key.
    #[must_use]
    pub fn key(mut self, key: impl Into<ElementKey>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Sets the container gap.
    #[must_use]
    pub fn gap(mut self, gap: impl Into<crate::Px>) -> Self {
        self.style = self.style.with_gap(gap);
        self
    }

    /// Returns the container axis.
    #[must_use]
    pub const fn axis(&self) -> Axis {
        self.axis
    }

    /// Returns the container children.
    #[must_use]
    pub const fn children(&self) -> &[Element<Action>] {
        self.children.as_slice()
    }

    /// Returns the authored element ID, if present.
    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }

    /// Returns the stable data key, if present.
    #[must_use]
    pub const fn element_key(&self) -> Option<&ElementKey> {
        self.key.as_ref()
    }

    /// Returns the layout style.
    #[must_use]
    pub const fn style(&self) -> &LayoutStyle {
        &self.style
    }
}

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
    fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

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
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            on_press: None,
        }
    }

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
    fn new(axis: Axis, children: impl IntoElements<Action>) -> Self {
        Self {
            axis,
            children: children.into_elements(),
        }
    }

    #[must_use]
    pub const fn axis(&self) -> Axis {
        self.axis
    }

    #[must_use]
    pub const fn children(&self) -> &[Element<Action>] {
        self.children.as_slice()
    }
}

pub trait IntoElements<Action> {
    fn into_elements(self) -> Vec<Element<Action>>;
}

impl<Action> IntoElements<Action> for Vec<Element<Action>> {
    fn into_elements(self) -> Self {
        self
    }
}

impl<Action> IntoElements<Action> for Element<Action> {
    fn into_elements(self) -> Vec<Self> {
        vec![self]
    }
}

impl<Action, const N: usize> IntoElements<Action> for [Element<Action>; N] {
    fn into_elements(self) -> Vec<Element<Action>> {
        Vec::from(self)
    }
}

macro_rules! impl_into_elements_tuple {
    ($($name:ident),+ $(,)?) => {
        impl<Action, $($name),+> IntoElements<Action> for ($($name,)+)
        where
            $($name: Into<Element<Action>>,)+
        {
            fn into_elements(self) -> Vec<Element<Action>> {
                #[allow(non_snake_case)]
                let ($($name,)+) = self;
                vec![$($name.into(),)+]
            }
        }
    };
}

impl_into_elements_tuple!(A);
impl_into_elements_tuple!(A, B);
impl_into_elements_tuple!(A, B, C);
impl_into_elements_tuple!(A, B, C, D);
impl_into_elements_tuple!(A, B, C, D, E);
impl_into_elements_tuple!(A, B, C, D, E, F);
impl_into_elements_tuple!(A, B, C, D, E, F, G);
impl_into_elements_tuple!(A, B, C, D, E, F, G, H);

#[must_use]
pub fn text<Action>(content: impl Into<String>) -> Element<Action> {
    Element::text(content)
}

#[must_use]
pub fn text_with<Action>(args: TextArgs) -> Element<Action> {
    Element::text_with(args)
}

#[must_use]
pub fn button<Action>(label: impl Into<String>) -> Element<Action> {
    Element::button(label)
}

#[must_use]
pub fn button_with<Action>(args: ButtonArgs<Action>) -> Element<Action> {
    Element::button_with(args)
}

#[must_use]
pub fn container_with<Action>(args: ContainerArgs<Action>) -> Element<Action> {
    Element::container_with(args)
}

#[must_use]
pub fn column<Action>(children: impl IntoElements<Action>) -> Element<Action> {
    Element::container(Axis::Vertical, children)
}

#[must_use]
pub fn row<Action>(children: impl IntoElements<Action>) -> Element<Action> {
    Element::container(Axis::Horizontal, children)
}
