//! Open widget protocol, capability vocabulary, and process-local widget identity.

use core::{any::TypeId, fmt};

use crate::{
    EditableContribution, EventContext, HitContribution, HitContributionContext, LogicalLength,
    LogicalSize, PaintContribution, PaintContributionContext, SemanticContribution,
    SemanticContributionContext, SubscriptionSet, UiEvent, WidgetActivationContext,
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

/// Available-space meaning supplied to a widget intrinsic measurement callback.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetAvailableSpace {
    Definite(LogicalLength),
    MinContent,
    MaxContent,
}

impl WidgetAvailableSpace {
    #[must_use]
    pub const fn definite(value: LogicalLength) -> Self {
        Self::Definite(value)
    }
}

/// Bounded, renderer-neutral request for one widget's intrinsic content size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetMeasureInput {
    known_width: Option<LogicalLength>,
    known_height: Option<LogicalLength>,
    available_width: WidgetAvailableSpace,
    available_height: WidgetAvailableSpace,
}

impl WidgetMeasureInput {
    #[must_use]
    pub const fn new(
        known_width: Option<LogicalLength>,
        known_height: Option<LogicalLength>,
        available_width: WidgetAvailableSpace,
        available_height: WidgetAvailableSpace,
    ) -> Self {
        Self {
            known_width,
            known_height,
            available_width,
            available_height,
        }
    }

    #[must_use]
    pub const fn known_width(self) -> Option<LogicalLength> {
        self.known_width
    }
    #[must_use]
    pub const fn known_height(self) -> Option<LogicalLength> {
        self.known_height
    }
    #[must_use]
    pub const fn available_width(self) -> WidgetAvailableSpace {
        self.available_width
    }
    #[must_use]
    pub const fn available_height(self) -> WidgetAvailableSpace {
        self.available_height
    }
}

/// Intrinsic content measurement returned by an open widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetMeasuredSize {
    size: LogicalSize,
    first_baseline: Option<LogicalLength>,
    last_baseline: Option<LogicalLength>,
}

impl WidgetMeasuredSize {
    #[must_use]
    pub const fn new(
        size: LogicalSize,
        first_baseline: Option<LogicalLength>,
        last_baseline: Option<LogicalLength>,
    ) -> Self {
        Self {
            size,
            first_baseline,
            last_baseline,
        }
    }

    #[must_use]
    pub const fn size(self) -> LogicalSize {
        self.size
    }
    #[must_use]
    pub const fn first_baseline(self) -> Option<LogicalLength> {
        self.first_baseline
    }
    #[must_use]
    pub const fn last_baseline(self) -> Option<LogicalLength> {
        self.last_baseline
    }
}

/// Production measurement capability contributed by a widget.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetMeasure {
    Measured(WidgetMeasuredSize),
    Text { content: String },
    Unsupported { reason: &'static str },
}

impl Default for WidgetMeasure {
    fn default() -> Self {
        Self::Measured(WidgetMeasuredSize::new(LogicalSize::ZERO, None, None))
    }
}

impl WidgetMeasure {
    /// Convenience constructor for a measured content-box size.
    #[must_use]
    pub const fn measured(width: LogicalLength, height: LogicalLength) -> Self {
        Self::Measured(WidgetMeasuredSize::new(
            LogicalSize::new(width, height),
            None,
            None,
        ))
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

/// Explicit opt-in to committed-text and composition routing.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WidgetTextInput {
    committed_text: bool,
    composition: bool,
}

impl WidgetTextInput {
    /// The default: neither text-input event family is accepted.
    pub const NONE: Self = Self {
        committed_text: false,
        composition: false,
    };
    #[must_use]
    pub const fn new(committed_text: bool, composition: bool) -> Self {
        Self {
            committed_text,
            composition,
        }
    }
    #[must_use]
    pub const fn accepts_committed_text(self) -> bool {
        self.committed_text
    }
    #[must_use]
    pub const fn accepts_composition(self) -> bool {
        self.composition
    }
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
/// Methods contribute bounded runtime behavior. Paint, physical hit, and
/// semantics are action-type-independent owner-local description contracts;
/// runtime alone composes them with mounted identity and surface placement.
pub trait Widget<Action>: fmt::Debug {
    /// Runtime-local state type stored by the mounted runtime.
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

    /// Declares host text-input protocol capability; it does not imply editing.
    fn text_input(&self, _state: &Self::State) -> WidgetTextInput {
        WidgetTextInput::NONE
    }

    /// Contributes one application-owned editable document presentation.
    ///
    /// Runtime may retain only ephemeral owner-local interaction state. The
    /// contribution remains the authoritative document snapshot and maps edit
    /// proposals into ordinary application actions.
    fn editable(&self, _state: &Self::State) -> Option<EditableContribution<Action>> {
        None
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

    /// Returns the widget's intrinsic response to one bounded layout request.
    fn measure(&self, _state: &Self::State, _input: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::default()
    }

    /// Returns this owner's renderer-neutral paint contribution in local logical coordinates.
    fn paint(&self, _state: &Self::State, _context: PaintContributionContext) -> PaintContribution {
        PaintContribution::empty()
    }

    /// Returns this owner's physical hit contribution in local logical coordinates.
    ///
    /// Empty is the default and canonical pass-through representation. The
    /// runtime injects mounted target identity only when composing a hit scene.
    fn hit_test(&self, _state: &Self::State, _context: HitContributionContext) -> HitContribution {
        HitContribution::empty()
    }

    /// Returns this mounted owner's complete action-type-independent semantic contribution.
    ///
    /// The context contains only structural child-count facts needed to satisfy
    /// the explicit mounted-child splice contract. It exposes no runtime IDs,
    /// focus, layout, or absolute coordinates.
    fn semantics(
        &self,
        _state: &Self::State,
        _context: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::empty()
    }

    /// Returns widget-owned diagnostics in deterministic order.
    fn diagnostics(&self, _state: &Self::State) -> Vec<WidgetDiagnostic> {
        Vec::new()
    }
}

/// Marker for widgets whose elements may structurally own children.
pub trait ChildBearingWidget<Action>: Widget<Action> {}

