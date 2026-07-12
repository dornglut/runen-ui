//! Transient views, erased elements, and the open widget protocol.

use core::{any::TypeId, fmt};
use std::{any::Any, rc::Rc};

use crate::{
    Axis, ColorValue, ElementId, ElementKey, IdentifierError, IntoElementId, IntoElementKey,
    LayoutStyle, LogicalLength, RadiusValue, SpacingValue, StyleIntent,
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

/// Lifecycle hook supplied to the isolated M2 conformance seam.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetLifecycle {
    Mount,
    Update,
    Unmount,
}

/// Narrow future-runtime request recorded by the lifecycle conformance seam.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WidgetLifecycleRequest {
    Layout,
    Paint,
    Diagnostic(WidgetDiagnostic),
}

/// Collector passed to typed lifecycle hooks.
#[derive(Debug, Default)]
pub struct WidgetLifecycleContext {
    requests: Vec<WidgetLifecycleRequest>,
}

impl WidgetLifecycleContext {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    pub fn request(&mut self, request: WidgetLifecycleRequest) {
        self.requests.push(request);
    }

    #[must_use]
    pub const fn requests(&self) -> &[WidgetLifecycleRequest] {
        self.requests.as_slice()
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

    /// Returns non-consuming activation/focus facts.
    fn activation(&self) -> WidgetActivation {
        WidgetActivation::NONE
    }

    /// Extracts the action for one accepted activation from this transient widget.
    ///
    /// Returning an action may mutate the widget's transient action source.
    /// Capability inspection remains borrowed and cannot consume it.
    fn activate(&mut self) -> Option<Action> {
        None
    }

    /// Returns current proof-level measurement behavior.
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::default()
    }

    /// Returns deterministic proof-level paint/debug facts.
    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::default()
    }

    /// Returns deterministic proof-level semantic facts.
    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::default()
    }

    /// Returns widget-owned diagnostics in deterministic order.
    fn diagnostics(&self) -> Vec<WidgetDiagnostic> {
        Vec::new()
    }

    /// Handles one lifecycle event in the isolated conformance seam.
    fn lifecycle(
        &self,
        _state: &mut Self::State,
        _event: WidgetLifecycle,
        _context: &mut WidgetLifecycleContext,
    ) {
    }
}

/// Explicit behavior contract for widgets whose elements may own children.
pub trait ChildLayoutWidget<Action>: Widget<Action> {
    /// Returns the current proof-level child arrangement policy.
    fn child_layout(&self) -> ChildLayout;
}

trait ErasedWidget<Action>: fmt::Debug {
    fn widget_type_id(&self) -> WidgetTypeId;
    fn widget_type_name(&self) -> &'static str;
    fn state_type_id(&self) -> WidgetStateTypeId;
    fn create_state(&self) -> Box<dyn Any>;
    fn activation(&self) -> WidgetActivation;
    fn activate(&mut self) -> Option<Action>;
    fn measure(&self) -> WidgetMeasure;
    fn child_layout(&self) -> Option<ChildLayout>;
    fn paint(&self) -> WidgetPaintProof;
    fn semantics(&self) -> WidgetSemanticProof;
    fn diagnostics(&self) -> Vec<WidgetDiagnostic>;
    fn lifecycle(
        &self,
        state: &mut dyn Any,
        event: WidgetLifecycle,
        context: &mut WidgetLifecycleContext,
    ) -> Result<(), WidgetStateMismatch>;
}

#[derive(Debug)]
struct WidgetAdapter<Implementation>(Implementation);

impl<Action, Implementation> ErasedWidget<Action> for WidgetAdapter<Implementation>
where
    Implementation: Widget<Action> + 'static,
{
    fn widget_type_id(&self) -> WidgetTypeId {
        WidgetTypeId::of::<Implementation>()
    }

    fn widget_type_name(&self) -> &'static str {
        core::any::type_name::<Implementation>()
    }

    fn state_type_id(&self) -> WidgetStateTypeId {
        WidgetStateTypeId::of::<Implementation::State>()
    }

    fn create_state(&self) -> Box<dyn Any> {
        Box::new(self.0.create_state())
    }

    fn activation(&self) -> WidgetActivation {
        self.0.activation()
    }

    fn activate(&mut self) -> Option<Action> {
        self.0.activate()
    }

    fn measure(&self) -> WidgetMeasure {
        self.0.measure()
    }

    fn child_layout(&self) -> Option<ChildLayout> {
        None
    }

    fn paint(&self) -> WidgetPaintProof {
        self.0.paint()
    }

    fn semantics(&self) -> WidgetSemanticProof {
        self.0.semantics()
    }

    fn diagnostics(&self) -> Vec<WidgetDiagnostic> {
        self.0.diagnostics()
    }

    fn lifecycle(
        &self,
        state: &mut dyn Any,
        event: WidgetLifecycle,
        context: &mut WidgetLifecycleContext,
    ) -> Result<(), WidgetStateMismatch> {
        let actual = WidgetStateTypeId::of::<Implementation::State>();
        let Some(state) = state.downcast_mut::<Implementation::State>() else {
            return Err(WidgetStateMismatch::ErasedStatePayload { expected: actual });
        };
        self.0.lifecycle(state, event, context);
        Ok(())
    }
}

#[derive(Debug)]
struct ChildLayoutWidgetAdapter<Implementation>(Implementation);

impl<Action, Implementation> ErasedWidget<Action> for ChildLayoutWidgetAdapter<Implementation>
where
    Implementation: ChildLayoutWidget<Action> + 'static,
{
    fn widget_type_id(&self) -> WidgetTypeId {
        WidgetTypeId::of::<Implementation>()
    }
    fn widget_type_name(&self) -> &'static str {
        core::any::type_name::<Implementation>()
    }
    fn state_type_id(&self) -> WidgetStateTypeId {
        WidgetStateTypeId::of::<Implementation::State>()
    }
    fn create_state(&self) -> Box<dyn Any> {
        Box::new(self.0.create_state())
    }
    fn activation(&self) -> WidgetActivation {
        self.0.activation()
    }
    fn activate(&mut self) -> Option<Action> {
        self.0.activate()
    }
    fn measure(&self) -> WidgetMeasure {
        self.0.measure()
    }
    fn child_layout(&self) -> Option<ChildLayout> {
        Some(self.0.child_layout())
    }
    fn paint(&self) -> WidgetPaintProof {
        self.0.paint()
    }
    fn semantics(&self) -> WidgetSemanticProof {
        self.0.semantics()
    }
    fn diagnostics(&self) -> Vec<WidgetDiagnostic> {
        self.0.diagnostics()
    }
    fn lifecycle(
        &self,
        state: &mut dyn Any,
        event: WidgetLifecycle,
        context: &mut WidgetLifecycleContext,
    ) -> Result<(), WidgetStateMismatch> {
        let expected = WidgetStateTypeId::of::<Implementation::State>();
        let Some(state) = state.downcast_mut::<Implementation::State>() else {
            return Err(WidgetStateMismatch::ErasedStatePayload { expected });
        };
        self.0.lifecycle(state, event, context);
        Ok(())
    }
}

struct MappedWidget<ChildAction, ParentAction> {
    child: Box<dyn ErasedWidget<ChildAction>>,
    mapper: Rc<dyn Fn(ChildAction) -> ParentAction>,
}

impl<ChildAction, ParentAction> fmt::Debug for MappedWidget<ChildAction, ParentAction> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MappedWidget")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<ChildAction, ParentAction> ErasedWidget<ParentAction>
    for MappedWidget<ChildAction, ParentAction>
{
    fn widget_type_id(&self) -> WidgetTypeId {
        self.child.widget_type_id()
    }
    fn widget_type_name(&self) -> &'static str {
        self.child.widget_type_name()
    }
    fn state_type_id(&self) -> WidgetStateTypeId {
        self.child.state_type_id()
    }
    fn create_state(&self) -> Box<dyn Any> {
        self.child.create_state()
    }
    fn activation(&self) -> WidgetActivation {
        self.child.activation()
    }
    fn activate(&mut self) -> Option<ParentAction> {
        self.child.activate().map(self.mapper.as_ref())
    }
    fn measure(&self) -> WidgetMeasure {
        self.child.measure()
    }
    fn child_layout(&self) -> Option<ChildLayout> {
        self.child.child_layout()
    }
    fn paint(&self) -> WidgetPaintProof {
        self.child.paint()
    }
    fn semantics(&self) -> WidgetSemanticProof {
        self.child.semantics()
    }
    fn diagnostics(&self) -> Vec<WidgetDiagnostic> {
        self.child.diagnostics()
    }
    fn lifecycle(
        &self,
        state: &mut dyn Any,
        event: WidgetLifecycle,
        context: &mut WidgetLifecycleContext,
    ) -> Result<(), WidgetStateMismatch> {
        self.child.lifecycle(state, event, context)
    }
}

/// Owned erased transient UI node derived from application state.
pub struct Element<Action> {
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    widget: Box<dyn ErasedWidget<Action>>,
    children: Vec<Self>,
    authoring_diagnostics: Vec<AuthoringDiagnostic>,
}

impl<Action> fmt::Debug for Element<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Element")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("layout", &self.layout)
            .field("style", &self.style)
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
            None,
            None,
            LayoutStyle::default(),
            StyleIntent::EMPTY,
            widget,
            children,
            Vec::new(),
        )
    }

    fn from_authored_parts(
        id: Option<ElementId>,
        key: Option<ElementKey>,
        layout: LayoutStyle,
        style: StyleIntent,
        widget: Box<dyn ErasedWidget<Action>>,
        children: Vec<Self>,
        authoring_diagnostics: Vec<AuthoringDiagnostic>,
    ) -> Self {
        Self {
            id,
            key,
            layout,
            style,
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
    pub const fn children(&self) -> &[Self] {
        self.children.as_slice()
    }
    #[must_use]
    pub const fn authoring_diagnostics(&self) -> &[AuthoringDiagnostic] {
        self.authoring_diagnostics.as_slice()
    }
    #[must_use]
    pub fn widget_type_id(&self) -> WidgetTypeId {
        self.widget.widget_type_id()
    }
    #[must_use]
    pub fn widget_type_name(&self) -> &'static str {
        self.widget.widget_type_name()
    }
    #[must_use]
    pub fn widget_state_type_id(&self) -> WidgetStateTypeId {
        self.widget.state_type_id()
    }
    #[must_use]
    pub fn activation(&self) -> WidgetActivation {
        self.widget.activation()
    }
    #[must_use]
    pub fn activate(&mut self) -> Option<Action> {
        self.widget.activate()
    }

    /// Extracts an action through the temporary core/runtime preorder bridge.
    ///
    /// This is doc-hidden because it is not an authoring API. It exists only
    /// because `runenui_core` cannot depend on `runenui_runtime`. The raw index
    /// is valid only for this transient tree; it is not authored, runtime, or
    /// mounted identity. Extraction may consume a one-shot action. M3 replaces
    /// this bridge with mounted generational targeting, and direct downstream
    /// use has no compatibility guarantee.
    #[doc(hidden)]
    #[must_use]
    pub fn extract_action_at_preorder_for_runtime(&mut self, target: usize) -> Option<Action> {
        fn visit<Action>(
            element: &mut Element<Action>,
            target: usize,
            current: &mut usize,
        ) -> Option<Action> {
            if *current == target {
                return element.activate();
            }
            *current = current.saturating_add(1);
            for child in &mut element.children {
                let before = *current;
                if let Some(action) = visit(child, target, current) {
                    return Some(action);
                }
                if before == target {
                    return None;
                }
            }
            None
        }

        visit(self, target, &mut 0)
    }
    #[must_use]
    pub fn measure(&self) -> WidgetMeasure {
        self.widget.measure()
    }
    #[must_use]
    pub fn child_layout(&self) -> Option<ChildLayout> {
        self.widget.child_layout()
    }
    #[must_use]
    pub fn paint(&self) -> WidgetPaintProof {
        self.widget.paint()
    }
    #[must_use]
    pub fn semantics(&self) -> WidgetSemanticProof {
        self.widget.semantics()
    }
    #[must_use]
    pub fn widget_diagnostics(&self) -> Vec<WidgetDiagnostic> {
        self.widget.diagnostics()
    }

    /// Creates a checked, opaque state value for the M2 lifecycle seam.
    #[must_use]
    pub fn create_widget_state(&self) -> WidgetState {
        WidgetState {
            widget_type: self.widget_type_id(),
            state_type: self.widget_state_type_id(),
            value: self.widget.create_state(),
        }
    }

    /// Runs one isolated lifecycle proof against a compatible state value.
    ///
    /// # Errors
    ///
    /// Returns [`WidgetStateMismatch`] before the typed hook runs when the
    /// state was created for a different widget or state contract.
    pub fn run_lifecycle(
        &self,
        state: &mut WidgetState,
        event: WidgetLifecycle,
        context: &mut WidgetLifecycleContext,
    ) -> Result<(), WidgetStateMismatch> {
        let expected_widget = self.widget_type_id();
        if state.widget_type != expected_widget {
            return Err(WidgetStateMismatch::WidgetType {
                expected: expected_widget,
                actual: state.widget_type,
            });
        }
        let expected_state = self.widget_state_type_id();
        if state.state_type != expected_state {
            return Err(WidgetStateMismatch::StateType {
                expected: expected_state,
                actual: state.state_type,
            });
        }
        self.widget.lifecycle(state.value.as_mut(), event, context)
    }
}

/// Opaque checked state created from a transient widget contract.
pub struct WidgetState {
    widget_type: WidgetTypeId,
    state_type: WidgetStateTypeId,
    value: Box<dyn Any>,
}

impl fmt::Debug for WidgetState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WidgetState")
            .field("widget_type", &self.widget_type)
            .field("state_type", &self.state_type)
            .finish_non_exhaustive()
    }
}

impl WidgetState {
    #[must_use]
    pub const fn widget_type_id(&self) -> WidgetTypeId {
        self.widget_type
    }
    #[must_use]
    pub const fn state_type_id(&self) -> WidgetStateTypeId {
        self.state_type
    }
}

/// Deterministic state compatibility failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetStateMismatch {
    WidgetType {
        expected: WidgetTypeId,
        actual: WidgetTypeId,
    },
    StateType {
        expected: WidgetStateTypeId,
        actual: WidgetStateTypeId,
    },
    ErasedStatePayload {
        expected: WidgetStateTypeId,
    },
}

impl WidgetStateMismatch {
    #[must_use]
    pub const fn expected_widget_type(&self) -> Option<WidgetTypeId> {
        match self {
            Self::WidgetType { expected, .. } => Some(*expected),
            Self::StateType { .. } | Self::ErasedStatePayload { .. } => None,
        }
    }
    #[must_use]
    pub const fn actual_widget_type(&self) -> Option<WidgetTypeId> {
        match self {
            Self::WidgetType { actual, .. } => Some(*actual),
            Self::StateType { .. } | Self::ErasedStatePayload { .. } => None,
        }
    }
    #[must_use]
    pub const fn expected_state_type(&self) -> Option<WidgetStateTypeId> {
        match self {
            Self::WidgetType { .. } => None,
            Self::StateType { expected, .. } | Self::ErasedStatePayload { expected } => {
                Some(*expected)
            }
        }
    }
    #[must_use]
    pub const fn actual_state_type(&self) -> Option<WidgetStateTypeId> {
        match self {
            Self::StateType { actual, .. } => Some(*actual),
            Self::WidgetType { .. } | Self::ErasedStatePayload { .. } => None,
        }
    }
}

impl fmt::Display for WidgetStateMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WidgetType { .. } => {
                formatter.write_str("widget implementation type does not match widget state")
            }
            Self::StateType { .. } => {
                formatter.write_str("widget state type does not match widget contract")
            }
            Self::ErasedStatePayload { .. } => formatter
                .write_str("erased widget state payload does not match its declared state type"),
        }
    }
}

impl std::error::Error for WidgetStateMismatch {}

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

/// Typed built-in text authored view.
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
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.content.clone(),
            kind: WidgetTextKind::Text,
            minimum_width: LogicalLength::ZERO,
            minimum_height: LogicalLength::ZERO,
        }
    }
    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::new("text", self.content.clone())
    }
    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::new("text", self.content.clone())
    }
}

impl<Action: 'static> View<Action> for Text {
    fn into_element(self) -> Element<Action> {
        let Self {
            content,
            id,
            key,
            style,
            diagnostics,
        } = self;
        Element::from_authored_parts(
            id,
            key,
            LayoutStyle::default(),
            style,
            Box::new(WidgetAdapter(TextWidget { content })),
            Vec::new(),
            diagnostics,
        )
    }
}

/// Typed built-in button authored view.
pub struct Button<Action> {
    label: String,
    id: Option<ElementId>,
    key: Option<ElementKey>,
    enabled: bool,
    on_press: Option<Action>,
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
            .field("has_action", &self.on_press.is_some())
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
            on_press: None,
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
    pub fn on_press(mut self, action: Action) -> Self {
        self.on_press = Some(action);
        self.actionable = true;
        self
    }
}

struct ButtonWidget<Action> {
    label: String,
    enabled: bool,
    on_press: Option<Action>,
    actionable: bool,
}

impl<Action> fmt::Debug for ButtonWidget<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ButtonWidget")
            .field("label", &self.label)
            .field("enabled", &self.enabled)
            .field("actionable", &self.actionable)
            .field("has_action", &self.on_press.is_some())
            .finish()
    }
}

impl<Action> Widget<Action> for ButtonWidget<Action> {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn activation(&self) -> WidgetActivation {
        if self.actionable {
            WidgetActivation::actionable(self.enabled)
        } else {
            WidgetActivation::NONE
        }
    }
    fn activate(&mut self) -> Option<Action> {
        if self.enabled {
            self.on_press.take()
        } else {
            None
        }
    }
    fn measure(&self) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.label.clone(),
            kind: WidgetTextKind::ControlLabel,
            minimum_width: LogicalLength::new(64.0).unwrap_or_default(),
            minimum_height: LogicalLength::new(32.0).unwrap_or_default(),
        }
    }
    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::new(
            "button",
            format!("label={:?} enabled={}", self.label, self.enabled),
        )
    }
    fn semantics(&self) -> WidgetSemanticProof {
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
        let Self {
            label,
            id,
            key,
            enabled,
            on_press,
            actionable,
            style,
            diagnostics,
        } = self;
        Element::from_authored_parts(
            id,
            key,
            LayoutStyle::default(),
            style,
            Box::new(WidgetAdapter(ButtonWidget {
                label,
                enabled,
                on_press,
                actionable,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

/// Canonical typed authored view for a child-layout widget.
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
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn paint(&self) -> WidgetPaintProof {
        WidgetPaintProof::new("container", format!("axis={:?}", self.axis))
    }
    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::new("group", "")
    }
}

impl<Action> ChildLayoutWidget<Action> for LinearContainerWidget {
    fn child_layout(&self) -> ChildLayout {
        ChildLayout::Linear { axis: self.axis }
    }
}

impl<Action: 'static> View<Action> for Container<Action> {
    fn into_element(self) -> Element<Action> {
        Element::from_authored_parts(
            self.id,
            self.key,
            self.layout,
            self.style,
            self.widget,
            self.children,
            self.diagnostics,
        )
    }
}

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
use common_builder_methods;

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

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::*;

    #[derive(Debug)]
    struct StatelessA;

    impl Widget<()> for StatelessA {
        type State = ();
        fn create_state(&self) -> Self::State {}
    }

    #[derive(Debug)]
    struct StatelessB;

    impl Widget<()> for StatelessB {
        type State = ();
        fn create_state(&self) -> Self::State {}
    }

    #[derive(Debug)]
    struct Stateful {
        calls: Rc<Cell<usize>>,
    }

    impl Widget<()> for Stateful {
        type State = u8;
        fn create_state(&self) -> Self::State {
            0
        }
        fn lifecycle(
            &self,
            state: &mut Self::State,
            _event: WidgetLifecycle,
            _context: &mut WidgetLifecycleContext,
        ) {
            self.calls.set(self.calls.get() + 1);
            *state += 1;
        }
    }

    #[test]
    fn lifecycle_compatibility_reports_widget_identity_before_state_identity() {
        let element = Element::new(StatelessA);
        let mut other_state = Element::new(StatelessB).create_widget_state();
        let result = element.run_lifecycle(
            &mut other_state,
            WidgetLifecycle::Mount,
            &mut WidgetLifecycleContext::new(),
        );
        let mismatch = WidgetStateMismatch::WidgetType {
            expected: WidgetTypeId::of::<StatelessA>(),
            actual: WidgetTypeId::of::<StatelessB>(),
        };
        assert_eq!(result, Err(mismatch));
        assert_eq!(
            mismatch.expected_widget_type(),
            Some(WidgetTypeId::of::<StatelessA>())
        );
        assert_eq!(
            mismatch.actual_widget_type(),
            Some(WidgetTypeId::of::<StatelessB>())
        );
        assert_eq!(mismatch.expected_state_type(), None);
        assert_eq!(mismatch.actual_state_type(), None);
        assert_eq!(
            mismatch.to_string(),
            "widget implementation type does not match widget state"
        );

        let calls = Rc::new(Cell::new(0));
        let different_state = Element::new(Stateful {
            calls: Rc::clone(&calls),
        });
        let mut different_state = different_state.create_widget_state();
        let mismatch = element.run_lifecycle(
            &mut different_state,
            WidgetLifecycle::Mount,
            &mut WidgetLifecycleContext::new(),
        );
        assert!(matches!(
            mismatch,
            Err(WidgetStateMismatch::WidgetType { .. })
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn lifecycle_compatibility_reports_state_and_erased_payload_failures_truthfully() {
        let calls = Rc::new(Cell::new(0));
        let element = Element::new(Stateful {
            calls: Rc::clone(&calls),
        });
        let mut state = element.create_widget_state();
        state.state_type = WidgetStateTypeId::of::<String>();
        let state_result = element.run_lifecycle(
            &mut state,
            WidgetLifecycle::Mount,
            &mut WidgetLifecycleContext::new(),
        );
        let state_mismatch = WidgetStateMismatch::StateType {
            expected: WidgetStateTypeId::of::<u8>(),
            actual: WidgetStateTypeId::of::<String>(),
        };
        assert_eq!(state_result, Err(state_mismatch));
        assert_eq!(state_mismatch.expected_widget_type(), None);
        assert_eq!(state_mismatch.actual_widget_type(), None);
        assert_eq!(
            state_mismatch.expected_state_type(),
            Some(WidgetStateTypeId::of::<u8>())
        );
        assert_eq!(
            state_mismatch.actual_state_type(),
            Some(WidgetStateTypeId::of::<String>())
        );
        assert_eq!(
            state_mismatch.to_string(),
            "widget state type does not match widget contract"
        );
        assert_eq!(calls.get(), 0);

        state.state_type = WidgetStateTypeId::of::<u8>();
        state.value = Box::new(String::new());
        let payload_result = element.run_lifecycle(
            &mut state,
            WidgetLifecycle::Mount,
            &mut WidgetLifecycleContext::new(),
        );
        let payload_mismatch = WidgetStateMismatch::ErasedStatePayload {
            expected: WidgetStateTypeId::of::<u8>(),
        };
        assert_eq!(payload_result, Err(payload_mismatch));
        assert_eq!(
            payload_mismatch.to_string(),
            "erased widget state payload does not match its declared state type"
        );
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn lifecycle_state_remains_compatible_across_equivalent_action_mapping() {
        let calls = Rc::new(Cell::new(0));
        let original: Element<()> = Element::new(Stateful {
            calls: Rc::clone(&calls),
        });
        let mut before_mapping = original.create_widget_state();
        let mapped = original.map_action(|()| 1_u8);
        assert_eq!(
            mapped.run_lifecycle(
                &mut before_mapping,
                WidgetLifecycle::Mount,
                &mut WidgetLifecycleContext::new(),
            ),
            Ok(())
        );

        let mut compatible = equivalent_state(&calls);
        let equivalent = Element::new(Stateful {
            calls: Rc::clone(&calls),
        });
        assert_eq!(
            equivalent.run_lifecycle(
                &mut compatible,
                WidgetLifecycle::Update,
                &mut WidgetLifecycleContext::new(),
            ),
            Ok(())
        );

        let mapped_source: Element<()> = Element::new(Stateful {
            calls: Rc::clone(&calls),
        });
        let mapped_source = mapped_source.map_action(|()| 1_u8);
        let mut after_mapping = mapped_source.create_widget_state();
        let equivalent: Element<()> = Element::new(Stateful {
            calls: Rc::clone(&calls),
        });
        assert_eq!(
            equivalent.run_lifecycle(
                &mut after_mapping,
                WidgetLifecycle::Update,
                &mut WidgetLifecycleContext::new(),
            ),
            Ok(())
        );
        assert_eq!(calls.get(), 3);
    }

    fn equivalent_state(calls: &Rc<Cell<usize>>) -> WidgetState {
        Element::<()>::new(Stateful {
            calls: Rc::clone(calls),
        })
        .create_widget_state()
    }
}
