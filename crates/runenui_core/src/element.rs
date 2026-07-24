//! Transient views, erased elements, and the open widget protocol.

use core::{any::TypeId, fmt};
use std::rc::Rc;

use crate::widget_erasure::{ElementParts, ErasedWidget, MountedWidget, WidgetAdapter};
use crate::widget_mapping::MappedWidget;
use crate::{
    Axis, ColorValue, ElementId, ElementKey, EventContext, FocusScope, Focusability,
    IdentifierError, IntoElementId, IntoElementKey, LayoutStyle, LogicalLength, RadiusValue,
    SpacingValue, StyleIntent, SubscriptionSet, UiEvent, WidgetActivationContext,
    WidgetEventOutput, WidgetInvalidation, WidgetMountContext, WidgetUnmountContext,
    WidgetUpdateContext,
};

/// Process-local identity of a concrete widget implementation type.
///
/// This wraps [`TypeId`] for reconciliation and checked state access. It is not
/// authored identity and must not be serialized or compared across builds.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetTypeId(TypeId);

impl WidgetTypeId {
    #[must_use]
    pub const fn of<Widget: 'static>() -> Self {
        Self(TypeId::of::<Widget>())
    }
}

impl fmt::Debug for WidgetTypeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WidgetTypeId(..)")
    }
}

/// Process-local identity of a widget's declared runtime-local state type.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WidgetStateTypeId(TypeId);

impl WidgetStateTypeId {
    #[must_use]
    pub const fn of<State: 'static>() -> Self {
        Self(TypeId::of::<State>())
    }
}

impl fmt::Debug for WidgetStateTypeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("WidgetStateTypeId(..)")
    }
}

/// Current proof-level measurement behavior contributed by a widget.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetMeasure {
    /// A fixed renderer-neutral logical size.
    Fixed {
        width: LogicalLength,
        height: LogicalLength,
    },
    /// Text measured through the current measurement-provider seam.
    Text {
        content: String,
        kind: WidgetTextKind,
        minimum_width: LogicalLength,
        minimum_height: LogicalLength,
    },
    /// A capability this runtime version must report rather than interpret.
    Unsupported { reason: &'static str },
}

/// Current proof-level policy for arranging a widget's owned children.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChildLayout {
    /// Children composed in authored order along one axis.
    Linear { axis: Axis },
}

impl Default for WidgetMeasure {
    fn default() -> Self {
        Self::Fixed {
            width: LogicalLength::ZERO,
            height: LogicalLength::ZERO,
        }
    }
}

/// Text category used only by the current measurement proof.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetTextKind {
    Text,
    ControlLabel,
}

/// Deterministic renderer-neutral contribution used to prove paint participation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WidgetPaintProof {
    category: String,
    description: String,
}

impl WidgetPaintProof {
    #[must_use]
    pub fn new(category: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            category: category.into(),
            description: description.into(),
        }
    }

    #[must_use]
    pub const fn category(&self) -> &str {
        self.category.as_str()
    }

    #[must_use]
    pub const fn description(&self) -> &str {
        self.description.as_str()
    }
}

impl Default for WidgetPaintProof {
    fn default() -> Self {
        Self::new("none", "")
    }
}

/// Minimal renderer-independent semantic facts used only by the M2 proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WidgetSemanticProof {
    role: String,
    name: String,
    enabled: bool,
    actionable: bool,
    action_intent: Option<String>,
}

impl WidgetSemanticProof {
    #[must_use]
    pub fn new(role: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            name: name.into(),
            enabled: true,
            actionable: false,
            action_intent: None,
        }
    }

    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn with_action(mut self, intent: impl Into<String>) -> Self {
        self.actionable = true;
        self.action_intent = Some(intent.into());
        self
    }

    #[must_use]
    pub const fn role(&self) -> &str {
        self.role.as_str()
    }

    #[must_use]
    pub const fn name(&self) -> &str {
        self.name.as_str()
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn actionable(&self) -> bool {
        self.actionable
    }

    #[must_use]
    pub fn action_intent(&self) -> Option<&str> {
        self.action_intent.as_deref()
    }
}

impl Default for WidgetSemanticProof {
    fn default() -> Self {
        Self::new("generic", "")
    }
}

/// Deterministic widget-authored or capability diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WidgetDiagnostic {
    code: String,
    message: String,
}

impl WidgetDiagnostic {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn code(&self) -> &str {
        self.code.as_str()
    }

    #[must_use]
    pub const fn message(&self) -> &str {
        self.message.as_str()
    }
}

/// Non-consuming activation facts for runtime focus and inspection policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WidgetActivation {
    enabled: bool,
    actionable: bool,
}

/// Explicit semantic result of one accepted mutable widget activation.
///
/// The action and persistent-state mutation facts are independent: an
/// activation may produce either, both, or neither.
#[must_use]
pub struct WidgetActivationOutput<Action> {
    action: Option<Action>,
    state_changed: bool,
}

impl<Action> WidgetActivationOutput<Action> {
    /// Reports that the callback committed no primary action or persistent state change.
    pub const fn none() -> Self {
        Self {
            action: None,
            state_changed: false,
        }
    }

    /// Reports one primary action without a persistent state change.
    pub const fn action(action: Action) -> Self {
        Self {
            action: Some(action),
            state_changed: false,
        }
    }

    /// Reports a persistent state change without a primary action.
    pub const fn changed() -> Self {
        Self {
            action: None,
            state_changed: true,
        }
    }

    /// Reports both a persistent state change and one primary action.
    pub const fn changed_with_action(action: Action) -> Self {
        Self {
            action: Some(action),
            state_changed: true,
        }
    }

    /// Borrows the primary action when one was produced.
    pub const fn action_ref(&self) -> Option<&Action> {
        self.action.as_ref()
    }

    /// Consumes the output and returns its primary action.
    pub fn into_action(self) -> Option<Action> {
        self.action
    }

    /// Returns whether persistent widget state changed.
    pub const fn state_changed(&self) -> bool {
        self.state_changed
    }

    /// Maps the primary action while preserving the state-change fact.
    pub fn map_action<ParentAction>(
        self,
        mapper: impl FnOnce(Action) -> ParentAction,
    ) -> WidgetActivationOutput<ParentAction> {
        WidgetActivationOutput {
            action: self.action.map(mapper),
            state_changed: self.state_changed,
        }
    }
}

impl<Action> Default for WidgetActivationOutput<Action> {
    fn default() -> Self {
        Self::none()
    }
}

impl<Action> fmt::Debug for WidgetActivationOutput<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WidgetActivationOutput")
            .field("has_action", &self.action.is_some())
            .field("state_changed", &self.state_changed)
            .finish()
    }
}

impl Default for WidgetActivation {
    fn default() -> Self {
        Self::NONE
    }
}

impl WidgetActivation {
    pub const NONE: Self = Self {
        enabled: true,
        actionable: false,
    };

    /// Deterministic unavailable fallback used after an internal state mismatch.
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            actionable: false,
        }
    }

    #[must_use]
    pub const fn actionable(enabled: bool) -> Self {
        Self {
            enabled,
            actionable: true,
        }
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn is_actionable(&self) -> bool {
        self.actionable
    }
}

/// Public downstream widget implementation contract.
///
/// Methods contribute bounded M2 proof behavior. Later milestones replace or
/// expand the corresponding production event, layout, paint, and semantic APIs.
pub trait Widget<Action>: fmt::Debug {
    /// Runtime-local state type to be stored by the future mounted runtime.
    type State: 'static;

    /// Creates initial runtime-local state. Stateless widgets explicitly declare
    /// `State = ()` and return `()` here.
    fn create_state(&self) -> Self::State;

    fn mount(&self, _state: &mut Self::State, _context: &mut WidgetMountContext<Action>) {}

    fn update(&self, _state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        context.invalidate(WidgetInvalidation::ALL);
    }

    fn unmount(&self, _state: &mut Self::State, _context: &mut WidgetUnmountContext) {}

    /// Declares the complete desired subscription set for this mounted state.
    fn subscriptions(&self, _state: &Self::State, _subscriptions: &mut SubscriptionSet<Action>) {}

    /// Participates once in the current mounted route phase.
    fn event(
        &mut self,
        _state: &mut Self::State,
        _event: &UiEvent,
        _context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        WidgetEventOutput::none()
    }

    /// Returns non-consuming activation/focus facts.
    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    /// Reports the action and persistent-state effects of one accepted activation.
    ///
    /// Repeatable controls create a fresh owned action on each invocation;
    /// state mutation is reported independently even when there is no action.
    /// Capability inspection remains borrowed and cannot invoke the callback.
    fn activate(
        &mut self,
        _state: &mut Self::State,
        _context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::none()
    }

    /// Returns current proof-level measurement behavior.
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::default()
    }

    /// Returns deterministic proof-level paint/debug facts.
    fn paint(&self, _state: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::default()
    }

    /// Returns deterministic proof-level semantic facts.
    fn semantics(&self, _state: &Self::State) -> WidgetSemanticProof {
        WidgetSemanticProof::default()
    }

    /// Returns widget-owned diagnostics in deterministic order.
    fn diagnostics(&self, _state: &Self::State) -> Vec<WidgetDiagnostic> {
        Vec::new()
    }
}

/// Explicit behavior contract for widgets whose elements may own children.
pub trait ChildLayoutWidget<Action>: Widget<Action> {
    /// Returns the current proof-level child arrangement policy.
    fn child_layout(&self, state: &Self::State) -> ChildLayout;
}

pub struct Element<Action> {
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
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
    pub focusability: Focusability,
    pub focus_scope: Option<FocusScope>,
}

impl AuthoredElementFields {
    pub const fn new(
        id: Option<ElementId>,
        key: Option<ElementKey>,
        layout: LayoutStyle,
        style: StyleIntent,
        focusability: Focusability,
        focus_scope: Option<FocusScope>,
    ) -> Self {
        Self {
            id,
            key,
            layout,
            style,
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
