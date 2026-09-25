//! Transient element and view authoring.

use core::fmt;
use std::rc::Rc;

use crate::widget_erasure::{ElementParts, ErasedWidget, MountedWidget, WidgetAdapter};
use crate::widget_mapping::MappedWidget;
use crate::widget_protocol::Widget;
use crate::{
    ElementId, ElementKey, ExplicitTimeline, FocusScope, Focusability, IdentifierError,
    IntoElementId, IntoElementKey, LayoutStyle, StyleIntent,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommonNodeAuthoring {
    pub(crate) id: Option<ElementId>,
    pub(crate) key: Option<ElementKey>,
    pub(crate) layout: LayoutStyle,
    pub(crate) style: StyleIntent,
    pub(crate) timelines: Vec<ExplicitTimeline>,
    pub(crate) diagnostics: Vec<AuthoringDiagnostic>,
}

impl Default for CommonNodeAuthoring {
    fn default() -> Self {
        Self {
            id: None,
            key: None,
            layout: LayoutStyle::default(),
            style: StyleIntent::EMPTY,
            timelines: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl CommonNodeAuthoring {
    pub(crate) fn from_authored_fields(
        fields: AuthoredElementFields,
        diagnostics: Vec<AuthoringDiagnostic>,
    ) -> (Self, Focusability, Option<FocusScope>) {
        (
            Self {
                id: fields.id,
                key: fields.key,
                layout: fields.layout,
                style: fields.style,
                timelines: fields.timelines,
                diagnostics,
            },
            fields.focusability,
            fields.focus_scope,
        )
    }

    pub(crate) fn into_authored_fields(
        self,
        focusability: Focusability,
        focus_scope: Option<FocusScope>,
    ) -> (AuthoredElementFields, Vec<AuthoringDiagnostic>) {
        (
            AuthoredElementFields::new(
                self.id,
                self.key,
                self.layout,
                self.style,
                self.timelines,
                focusability,
                focus_scope,
            ),
            self.diagnostics,
        )
    }

    pub(crate) fn assign_id(&mut self, value: impl IntoElementId) {
        match value.into_element_id() {
            Ok(id) => self.id = Some(id),
            Err((value, error)) => self.diagnostics.push(AuthoringDiagnostic {
                field: "id",
                value,
                error,
            }),
        }
    }

    pub(crate) fn assign_key(&mut self, value: impl IntoElementKey) {
        match value.into_element_key() {
            Ok(key) => self.key = Some(key),
            Err((value, error)) => self.diagnostics.push(AuthoringDiagnostic {
                field: "key",
                value,
                error,
            }),
        }
    }
}

macro_rules! common_node_builder_methods {
    () => {
        #[must_use]
        pub fn id(mut self, id: impl $crate::IntoElementId) -> Self {
            self.common.assign_id(id);
            self
        }
        #[must_use]
        pub fn key(mut self, key: impl $crate::IntoElementKey) -> Self {
            self.common.assign_key(key);
            self
        }
        #[must_use]
        pub fn with_layout(mut self, layout: $crate::LayoutStyle) -> Self {
            self.common.layout = layout;
            self
        }
        #[must_use]
        pub fn recipe(mut self, recipe: $crate::StyleRecipeId) -> Self {
            self.common.style = self.common.style.with_recipe(recipe);
            self
        }
        #[must_use]
        pub fn variant(mut self, variant: $crate::StyleVariantId) -> Self {
            self.common.style = self.common.style.with_variant(variant);
            self
        }
        #[must_use]
        pub fn foreground(mut self, value: impl Into<$crate::ColorValue>) -> Self {
            self.common.style = self.common.style.with_foreground(value);
            self
        }
        #[must_use]
        pub fn background(mut self, value: impl Into<$crate::BrushValue>) -> Self {
            self.common.style = self.common.style.with_background(value);
            self
        }
        #[must_use]
        pub fn padding(mut self, value: impl Into<$crate::SpacingValue>) -> Self {
            self.common.style = self.common.style.with_padding(value);
            self
        }
        #[must_use]
        pub fn radius(mut self, value: impl Into<$crate::RadiusValue>) -> Self {
            self.common.style = self.common.style.with_radius(value);
            self
        }
        #[must_use]
        pub fn typography(mut self, value: impl Into<$crate::TypographyValue>) -> Self {
            self.common.style = self.common.style.with_typography(value);
            self
        }
        #[must_use]
        pub fn outline(mut self, value: impl Into<$crate::OutlineValue>) -> Self {
            self.common.style = self.common.style.with_outline(value);
            self
        }
        #[must_use]
        pub fn shadows(mut self, value: impl Into<$crate::ShadowValue>) -> Self {
            self.common.style = self.common.style.with_shadows(value);
            self
        }
        #[must_use]
        pub fn opacity(mut self, value: impl Into<$crate::OpacityValue>) -> Self {
            self.common.style = self.common.style.with_opacity(value);
            self
        }
        #[must_use]
        pub fn presentation(mut self, value: impl Into<$crate::PresentationValue>) -> Self {
            self.common.style = self.common.style.with_presentation(value);
            self
        }
        /// Contributes transition policy through the ordinary style cascade.
        #[must_use]
        pub fn transition(
            mut self,
            target: $crate::MotionTarget,
            spec: $crate::TransitionSpec,
        ) -> Self {
            self.common.style = self.common.style.with_transition(target, spec);
            self
        }

        /// Explicitly disables transition for one motion target at the authored layer.
        #[must_use]
        pub fn transition_disabled(mut self, target: $crate::MotionTarget) -> Self {
            self.common.style = self.common.style.with_transition_disabled(target);
            self
        }

        /// Adds one owner-local declarative explicit timeline.
        ///
        /// Duplicate animation IDs and duplicate targets are retained here and
        /// rejected transactionally by runtime candidate planning.
        #[must_use]
        pub fn timeline(mut self, timeline: $crate::ExplicitTimeline) -> Self {
            self.common.timelines.push(timeline);
            self
        }
    };
}

pub(crate) use common_node_builder_methods;

pub struct Element<Action> {
    common: CommonNodeAuthoring,
    focusability: Focusability,
    focus_scope: Option<FocusScope>,
    widget: Box<dyn ErasedWidget<Action>>,
    children: Vec<Self>,
}

pub struct AuthoredElementFields {
    pub id: Option<ElementId>,
    pub key: Option<ElementKey>,
    pub layout: LayoutStyle,
    pub style: StyleIntent,
    pub timelines: Vec<ExplicitTimeline>,
    pub focusability: Focusability,
    pub focus_scope: Option<FocusScope>,
}

impl AuthoredElementFields {
    pub const fn new(
        id: Option<ElementId>,
        key: Option<ElementKey>,
        layout: LayoutStyle,
        style: StyleIntent,
        timelines: Vec<ExplicitTimeline>,
        focusability: Focusability,
        focus_scope: Option<FocusScope>,
    ) -> Self {
        Self {
            id,
            key,
            layout,
            style,
            timelines,
            focusability,
            focus_scope,
        }
    }
}

impl<Action> fmt::Debug for Element<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Element")
            .field("id", &self.common.id)
            .field("key", &self.common.key)
            .field("layout", &self.common.layout)
            .field("style", &self.common.style)
            .field("timelines", &self.common.timelines)
            .field("focusability", &self.focusability)
            .field("focus_scope", &self.focus_scope)
            .field("widget_type", &self.widget.widget_type_name())
            .field("children", &self.children)
            .field("authoring_diagnostics", &self.common.diagnostics)
            .finish()
    }
}

impl<Action> Element<Action> {
    /// Erases a downstream widget implementation into a transient element.
    #[must_use]
    pub fn new<Implementation>(widget: Implementation) -> Self
    where
        Implementation: Widget<Action> + 'static,
    {
        Self::from_parts(Box::new(WidgetAdapter(widget)), Vec::new())
    }

    fn from_parts(widget: Box<dyn ErasedWidget<Action>>, children: Vec<Self>) -> Self {
        Self {
            common: CommonNodeAuthoring::default(),
            focusability: Focusability::Automatic,
            focus_scope: None,
            widget,
            children,
        }
    }

    pub(crate) fn from_authored_parts(
        fields: AuthoredElementFields,
        widget: Box<dyn ErasedWidget<Action>>,
        children: Vec<Self>,
        authoring_diagnostics: Vec<AuthoringDiagnostic>,
    ) -> Self {
        let (common, focusability, focus_scope) =
            CommonNodeAuthoring::from_authored_fields(fields, authoring_diagnostics);
        Self {
            common,
            focusability,
            focus_scope,
            widget,
            children,
        }
    }

    common_node_builder_methods!();

    /// Declares explicit participation in mounted focus selection.
    #[must_use]
    pub const fn focusable(mut self, focusable: bool) -> Self {
        self.focusability = if focusable {
            Focusability::Focusable
        } else {
            Focusability::NotFocusable
        };
        self
    }

    /// Excludes this mounted node from focus selection as focus-hidden.
    ///
    /// This is a focus eligibility fact, not a renderer visibility contract.
    #[must_use]
    pub const fn focus_hidden(mut self, hidden: bool) -> Self {
        self.focusability = if hidden {
            Focusability::Hidden
        } else {
            Focusability::Automatic
        };
        self
    }

    /// Declares this mounted node as a nested focus-scope boundary.
    #[must_use]
    pub const fn focus_scope(mut self, scope: FocusScope) -> Self {
        self.focus_scope = Some(scope);
        self
    }

    /// Maps every typed widget action in this subtree into a parent action.
    #[must_use]
    pub fn map_action<ParentAction>(
        self,
        mapper: impl Fn(Action) -> ParentAction + 'static,
    ) -> Element<ParentAction>
    where
        Action: 'static,
        ParentAction: 'static,
    {
        let mapper: Rc<dyn Fn(Action) -> ParentAction> = Rc::new(mapper);
        self.map_action_shared(&mapper)
    }

    fn map_action_shared<ParentAction>(
        self,
        mapper: &Rc<dyn Fn(Action) -> ParentAction>,
    ) -> Element<ParentAction>
    where
        Action: 'static,
        ParentAction: 'static,
    {
        Element {
            common: self.common,
            focusability: self.focusability,
            focus_scope: self.focus_scope,
            widget: Box::new(MappedWidget {
                child: self.widget,
                mapper: Rc::clone(mapper),
            }),
            children: self
                .children
                .into_iter()
                .map(|child| child.map_action_shared(mapper))
                .collect(),
        }
    }

    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.common.id.as_ref()
    }
    #[must_use]
    pub const fn element_key(&self) -> Option<&ElementKey> {
        self.common.key.as_ref()
    }
    #[must_use]
    pub const fn layout(&self) -> &LayoutStyle {
        &self.common.layout
    }
    #[must_use]
    pub const fn style(&self) -> &StyleIntent {
        &self.common.style
    }
    #[must_use]
    pub const fn timelines(&self) -> &[ExplicitTimeline] {
        self.common.timelines.as_slice()
    }
    #[must_use]
    pub const fn focusability(&self) -> Focusability {
        self.focusability
    }
    #[must_use]
    pub const fn focus_scope_config(&self) -> Option<FocusScope> {
        self.focus_scope
    }
    #[must_use]
    pub const fn children(&self) -> &[Self] {
        self.children.as_slice()
    }
    #[must_use]
    pub const fn authoring_diagnostics(&self) -> &[AuthoringDiagnostic] {
        self.common.diagnostics.as_slice()
    }
    /// Consumes this transient node into unstable runtime-owned plumbing.
    #[doc(hidden)]
    #[must_use]
    pub fn into_runtime_parts(self) -> ElementParts<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(self.focusability, self.focus_scope);
        ElementParts::new(
            fields,
            MountedWidget::from_erased(self.widget),
            self.children,
            diagnostics,
        )
    }
}

/// Converts one typed transient view into its erased element.
pub trait View<Action> {
    fn into_element(self) -> Element<Action>;
}

impl<Action> View<Action> for Element<Action> {
    fn into_element(self) -> Self {
        self
    }
}

/// Converts an iterator or collection of views into erased children.
pub trait Views<Action> {
    fn into_elements(self) -> Vec<Element<Action>>;
}

impl<Action, Items, Item> Views<Action> for Items
where
    Items: IntoIterator<Item = Item>,
    Item: View<Action>,
{
    fn into_elements(self) -> Vec<Element<Action>> {
        self.into_iter().map(View::into_element).collect()
    }
}

/// Invalid authored configuration retained for deterministic runtime reporting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoringDiagnostic {
    pub(crate) field: &'static str,
    pub(crate) value: String,
    pub(crate) error: IdentifierError,
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
