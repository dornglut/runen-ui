//! Transient element and view authoring.

use core::fmt;
use std::rc::Rc;

use crate::widget_erasure::{ElementParts, ErasedWidget, MountedWidget, WidgetAdapter};
use crate::widget_mapping::MappedWidget;
use crate::widget_protocol::Widget;
use crate::{
    BrushValue, ColorValue, ElementId, ElementKey, ExplicitTimeline, FocusScope, Focusability,
    IdentifierError, IntoElementId, IntoElementKey, LayoutStyle, MotionTarget, OpacityValue,
    OutlineValue, RadiusValue, ShadowValue, SpacingValue, StyleIntent, StyleRecipeId,
    StyleVariantId, TransitionSpec, TypographyValue,
};

pub struct Element<Action> {
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    timelines: Vec<ExplicitTimeline>,
    focusability: Focusability,
    focus_scope: Option<FocusScope>,
    widget: Box<dyn ErasedWidget<Action>>,
    children: Vec<Self>,
    authoring_diagnostics: Vec<AuthoringDiagnostic>,
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
            .field("id", &self.id)
            .field("key", &self.key)
            .field("layout", &self.layout)
            .field("style", &self.style)
            .field("timelines", &self.timelines)
            .field("focusability", &self.focusability)
            .field("focus_scope", &self.focus_scope)
            .field("widget_type", &self.widget.widget_type_name())
            .field("children", &self.children)
            .field("authoring_diagnostics", &self.authoring_diagnostics)
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
        Self::from_authored_parts(
            AuthoredElementFields::new(
                None,
                None,
                LayoutStyle::default(),
                StyleIntent::EMPTY,
                Vec::new(),
                Focusability::Automatic,
                None,
            ),
            widget,
            children,
            Vec::new(),
        )
    }

    pub(crate) fn from_authored_parts(
        fields: AuthoredElementFields,
        widget: Box<dyn ErasedWidget<Action>>,
        children: Vec<Self>,
        authoring_diagnostics: Vec<AuthoringDiagnostic>,
    ) -> Self {
        Self {
            id: fields.id,
            key: fields.key,
            layout: fields.layout,
            style: fields.style,
            timelines: fields.timelines,
            focusability: fields.focusability,
            focus_scope: fields.focus_scope,
            widget,
            children,
            authoring_diagnostics,
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl IntoElementId) -> Self {
        assign_id(&mut self.id, &mut self.authoring_diagnostics, id);
        self
    }

    #[must_use]
    pub fn key(mut self, key: impl IntoElementKey) -> Self {
        assign_key(&mut self.key, &mut self.authoring_diagnostics, key);
        self
    }

    #[must_use]
    pub fn with_layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = layout;
        self
    }

    #[must_use]
    pub fn recipe(mut self, recipe: StyleRecipeId) -> Self {
        self.style = self.style.with_recipe(recipe);
        self
    }

    #[must_use]
    pub fn variant(mut self, variant: StyleVariantId) -> Self {
        self.style = self.style.with_variant(variant);
        self
    }

    #[must_use]
    pub fn foreground(mut self, value: impl Into<ColorValue>) -> Self {
        self.style = self.style.with_foreground(value);
        self
    }

    #[must_use]
    pub fn background(mut self, value: impl Into<BrushValue>) -> Self {
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
    pub fn typography(mut self, value: impl Into<TypographyValue>) -> Self {
        self.style = self.style.with_typography(value);
        self
    }

    #[must_use]
    pub fn outline(mut self, value: impl Into<OutlineValue>) -> Self {
        self.style = self.style.with_outline(value);
        self
    }

    #[must_use]
    pub fn shadows(mut self, value: impl Into<ShadowValue>) -> Self {
        self.style = self.style.with_shadows(value);
        self
    }

    #[must_use]
    pub fn opacity(mut self, value: impl Into<OpacityValue>) -> Self {
        self.style = self.style.with_opacity(value);
        self
    }

    /// Contributes transition policy through the ordinary style cascade.
    #[must_use]
    pub fn transition(mut self, target: MotionTarget, spec: TransitionSpec) -> Self {
        self.style = self.style.with_transition(target, spec);
        self
    }

    /// Explicitly disables transition for one motion target at the authored layer.
    #[must_use]
    pub fn transition_disabled(mut self, target: MotionTarget) -> Self {
        self.style = self.style.with_transition_disabled(target);
        self
    }

    /// Adds one owner-local declarative explicit timeline.
    ///
    /// Duplicate animation IDs and duplicate targets are retained here and
    /// rejected transactionally by runtime candidate planning.
    #[must_use]
    pub fn timeline(mut self, timeline: ExplicitTimeline) -> Self {
        self.timelines.push(timeline);
        self
    }

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
            id: self.id,
            key: self.key,
            layout: self.layout,
            style: self.style,
            timelines: self.timelines,
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
            authoring_diagnostics: self.authoring_diagnostics,
        }
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
    pub const fn layout(&self) -> &LayoutStyle {
        &self.layout
    }
    #[must_use]
    pub const fn style(&self) -> &StyleIntent {
        &self.style
    }
    #[must_use]
    pub const fn timelines(&self) -> &[ExplicitTimeline] {
        self.timelines.as_slice()
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
        self.authoring_diagnostics.as_slice()
    }
    /// Consumes this transient node into unstable runtime-owned plumbing.
    #[doc(hidden)]
    #[must_use]
    pub fn into_runtime_parts(self) -> ElementParts<Action> {
        ElementParts::new(
            AuthoredElementFields::new(
                self.id,
                self.key,
                self.layout,
                self.style,
                self.timelines,
                self.focusability,
                self.focus_scope,
            ),
            MountedWidget::from_erased(self.widget),
            self.children,
            self.authoring_diagnostics,
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
