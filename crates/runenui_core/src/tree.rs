//! Typed host-neutral UI element tree.

use crate::{Axis, ElementId, LayoutStyle};

#[derive(Clone, Debug, PartialEq)]
pub struct Element<Action> {
    id: Option<ElementId>,
    style: LayoutStyle,
    kind: ElementKind<Action>,
}

impl<Action> Element<Action> {
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            id: None,
            style: LayoutStyle::default(),
            kind: ElementKind::Text(TextElement::new(content)),
        }
    }

    #[must_use]
    pub fn button(label: impl Into<String>) -> Self {
        Self {
            id: None,
            style: LayoutStyle::default(),
            kind: ElementKind::Button(ButtonElement::new(label)),
        }
    }

    #[must_use]
    pub fn container(axis: Axis, children: impl IntoElements<Action>) -> Self {
        Self {
            id: None,
            style: LayoutStyle::default(),
            kind: ElementKind::Container(ContainerElement::new(axis, children)),
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
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
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
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
    on_press: Option<Action>,
}

impl<Action> ButtonElement<Action> {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_press: None,
        }
    }

    #[must_use]
    pub const fn label(&self) -> &str {
        self.label.as_str()
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
pub fn button<Action>(label: impl Into<String>) -> Element<Action> {
    Element::button(label)
}

#[must_use]
pub fn column<Action>(children: impl IntoElements<Action>) -> Element<Action> {
    Element::container(Axis::Vertical, children)
}

#[must_use]
pub fn row<Action>(children: impl IntoElements<Action>) -> Element<Action> {
    Element::container(Axis::Horizontal, children)
}
