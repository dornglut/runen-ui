//! Genuine downstream consumer of only `RunenUI`'s public APIs.
//!
//! Routed command authority remains inaccessible to this downstream crate:
//!
//! ```compile_fail
//! use runenui_core::{CommandOrigin, EventSource};
//! let _ = CommandOrigin::delegated(EventSource::Controller);
//! ```
//!
//! ```compile_fail
//! use runenui_core::EventContext;
//! let _ = EventContext::<()>::new;
//! let _ = EventContext::<()>::into_output;
//! ```
//!
//! A widget can delegate only to its current callback target because the
//! public output operation accepts no target argument:
//!
//! ```compile_fail
//! use runenui_core::{EventContext, MountedNodeId, SemanticCommand};
//! fn arbitrary_target(
//!     context: &mut EventContext<'_, ()>,
//!     target: MountedNodeId,
//! ) {
//!     context.emit_command(target, SemanticCommand::OpenMenu);
//! }
//! ```
//!
//! Mounted identities expose no live runtime namespace extraction:
//!
//! ```compile_fail
//! use runenui_core::MountedNodeId;
//! fn extract_namespace(target: &MountedNodeId) {
//!     let _ = target.namespace;
//! }
//! ```
//!
//! Downstream code has no runtime callback-injection entry point, even if it
//! already holds ordinary public event borrows:
//!
//! ```compile_fail
//! use runenui_core::{EventContext, UiApp, UiEvent};
//! use runenui_runtime::AppRuntime;
//! fn inject<App: UiApp>(
//!     runtime: &mut AppRuntime<App>,
//!     event: &UiEvent,
//!     context: &mut EventContext<'_, App::Action>,
//! ) {
//!     runtime.invoke_event(event, context);
//! }
//! ```
//!
//! Composition generations are opaque runtime-issued values, not downstream
//! constructors:
//!
//! ```compile_fail
//! use runenui_core::CompositionGeneration;
//! let _ = CompositionGeneration { generation: 1 };
//! ```
//!
//! Runtime namespace internals are not part of the public core vocabulary:
//!
//! ```compile_fail
//! use runenui_core::RuntimeNamespace;
//! let _ = RuntimeNamespace::__runtime_new();
//! ```
//!
//! Raw keyboard ingress replaced the former public normalized-keyboard origin:
//!
//! ```compile_fail
//! use runenui_core::CommandOrigin;
//! let _ = CommandOrigin::keyboard();
//! ```
//!
//! Mounted inspection has no first-match automation lookup or direct activation
//! escape hatch:
//!
//! ```compile_fail
//! use runenui_runtime::MountedTreeIndex;
//! let _ = MountedTreeIndex::node_by_authored_id;
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::MountedNodeRef;
//! let _ = MountedNodeRef::activate;
//! ```

#![forbid(unsafe_code)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    task::Poll,
};

use runenui_core::{
    Axis, ChildLayout, ChildLayoutWidget, CompositionCancelReason, CompositionEvent, Container,
    EdgeInsets, Element, EventContext, EventPhase, FocusEventKind, FocusReason, IntoEffects,
    KeyboardPhase, LogicalLength, NoHostProtocol, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, SemanticState,
    SubscriptionSet, UiApp, UiEvent, View, Views, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetDiagnostic, WidgetEventOutput,
    WidgetInvalidation, WidgetMeasure, WidgetMountContext, WidgetPaintProof, WidgetTextKind,
    WidgetUnmountContext, WidgetUpdateContext, WorkKey, button, children, column, container, text,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalFocusFact {
    pub widget: &'static str,
    pub phase: EventPhase,
    pub kind: FocusEventKind,
    pub reason: FocusReason,
    pub target_is_callback_target: bool,
    pub has_related_target: bool,
}

#[derive(Debug)]
pub struct ExternalFocusWidget {
    name: &'static str,
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
    actionable: bool,
    prevent_focus_request: bool,
}

impl ExternalFocusWidget {
    #[must_use]
    pub const fn new(
        name: &'static str,
        log: Rc<RefCell<Vec<ExternalFocusFact>>>,
        actionable: bool,
    ) -> Self {
        Self {
            name,
            log,
            actionable,
            prevent_focus_request: false,
        }
    }

    #[must_use]
    pub const fn prevent_focus_request(mut self, prevent: bool) -> Self {
        self.prevent_focus_request = prevent;
        self
    }
}

impl Widget<()> for ExternalFocusWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.actionable)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if self.prevent_focus_request
            && event
                .as_semantic_command()
                .is_some_and(|event| event.command() == runenui_core::SemanticCommand::RequestFocus)
        {
            context.prevent_default();
        }
        if let Some(event) = event.as_focus() {
            self.log.borrow_mut().push(ExternalFocusFact {
                widget: self.name,
                phase: context.phase(),
                kind: event.kind(),
                reason: event.reason(),
                target_is_callback_target: event.target() == context.current_target(),
                has_related_target: context.related_target().is_some(),
            });
            context.prevent_default();
        }
        WidgetEventOutput::none()
    }
}

impl ChildLayoutWidget<()> for ExternalFocusWidget {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

#[must_use]
pub fn external_focus_panel(log: Rc<RefCell<Vec<ExternalFocusFact>>>) -> Element<()> {
    container(
        ExternalFocusWidget::new("root", Rc::clone(&log), false),
        vec![
            Element::new(ExternalFocusWidget::new("a", Rc::clone(&log), true))
                .id("focus.a")
                .key("a")
                .focusable(true),
            Element::new(ExternalFocusWidget::new("b", log, true))
                .id("focus.b")
                .key("b")
                .focusable(true),
        ],
    )
    .id("focus.root")
    .key("root")
    .into_element()
}

/// Redacted facts a downstream widget can observe for the M4C5 input families.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalInputKind {
    Keyboard(KeyboardPhase),
    CommittedText { bytes: usize, scalars: usize },
    CompositionStart,
    CompositionUpdate { has_range: bool },
    CompositionEnd,
    CompositionCancel(CompositionCancelReason),
}

/// One downstream callback observation without retaining raw user text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalInputFact {
    phase: EventPhase,
    kind: ExternalInputKind,
    cancelable: bool,
    prevented_before_callback: bool,
}

impl ExternalInputFact {
    #[must_use]
    pub const fn phase(&self) -> EventPhase {
        self.phase
    }

    #[must_use]
    pub const fn kind(&self) -> &ExternalInputKind {
        &self.kind
    }

    #[must_use]
    pub const fn default_is_cancelable(&self) -> bool {
        self.cancelable
    }

    #[must_use]
    pub const fn default_was_prevented(&self) -> bool {
        self.prevented_before_callback
    }
}

/// Child-owned action used to prove public action mapping retains input facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalInputAction {
    Observed(ExternalInputKind),
}

/// Downstream ancestor used to observe Capture and Bubble input phases.
#[derive(Debug)]
pub struct ExternalInputAncestor {
    facts: Rc<RefCell<Vec<ExternalInputFact>>>,
}

impl ExternalInputAncestor {
    #[must_use]
    pub const fn new(facts: Rc<RefCell<Vec<ExternalInputFact>>>) -> Self {
        Self { facts }
    }
}

impl<Action> Widget<Action> for ExternalInputAncestor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        let Some(kind) = external_input_kind(event) else {
            return WidgetEventOutput::none();
        };
        self.facts.borrow_mut().push(ExternalInputFact {
            phase: context.phase(),
            kind,
            cancelable: context.default_is_cancelable(),
            prevented_before_callback: context.default_is_prevented(),
        });
        WidgetEventOutput::none()
    }
}

impl<Action> ChildLayoutWidget<Action> for ExternalInputAncestor {
    fn child_layout(&self, (): &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

/// Genuine downstream text and composition-capable widget.
#[derive(Debug)]
pub struct ExternalInputWidget {
    facts: Rc<RefCell<Vec<ExternalInputFact>>>,
    prevent_keyboard: bool,
    prevent_text: bool,
}

impl ExternalInputWidget {
    #[must_use]
    pub const fn new(facts: Rc<RefCell<Vec<ExternalInputFact>>>) -> Self {
        Self {
            facts,
            prevent_keyboard: false,
            prevent_text: false,
        }
    }

    #[must_use]
    pub const fn prevent_keyboard(mut self, prevent: bool) -> Self {
        self.prevent_keyboard = prevent;
        self
    }

    #[must_use]
    pub const fn prevent_text(mut self, prevent: bool) -> Self {
        self.prevent_text = prevent;
        self
    }
}

impl Widget<ExternalInputAction> for ExternalInputWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ExternalInputAction>,
    ) -> WidgetEventOutput {
        if self.prevent_keyboard
            && context.phase() == EventPhase::Target
            && matches!(event, UiEvent::Keyboard(_))
        {
            context.prevent_default();
        }
        if self.prevent_text
            && context.phase() == EventPhase::Target
            && matches!(event, UiEvent::CommittedText(_))
        {
            context.prevent_default();
        }
        let Some(kind) = external_input_kind(event) else {
            return WidgetEventOutput::none();
        };
        self.facts.borrow_mut().push(ExternalInputFact {
            phase: context.phase(),
            kind: kind.clone(),
            cancelable: context.default_is_cancelable(),
            prevented_before_callback: context.default_is_prevented(),
        });
        if context.phase() == EventPhase::Target {
            context.emit(ExternalInputAction::Observed(kind));
        }
        WidgetEventOutput::none()
    }

    fn text_input(&self, (): &Self::State) -> runenui_core::WidgetTextInput {
        runenui_core::WidgetTextInput::new(true, true)
    }
}

fn external_input_kind(event: &UiEvent) -> Option<ExternalInputKind> {
    match event {
        UiEvent::Keyboard(event) => Some(ExternalInputKind::Keyboard(event.phase())),
        UiEvent::CommittedText(event) => Some(ExternalInputKind::CommittedText {
            bytes: event.text().len(),
            scalars: event.text().chars().count(),
        }),
        UiEvent::Composition(CompositionEvent::Start(_)) => {
            Some(ExternalInputKind::CompositionStart)
        }
        UiEvent::Composition(CompositionEvent::Update(event)) => {
            Some(ExternalInputKind::CompositionUpdate {
                has_range: event.range().is_some(),
            })
        }
        UiEvent::Composition(CompositionEvent::End(_)) => Some(ExternalInputKind::CompositionEnd),
        UiEvent::Composition(CompositionEvent::Cancel(event)) => {
            Some(ExternalInputKind::CompositionCancel(event.reason()))
        }
        _ => None,
    }
}

/// Downstream-observable mounted-subscription lifecycle facts.
#[derive(Debug, Default)]
pub struct ExternalSubscriptionLog {
    declarations: Cell<usize>,
    polled_declarations: RefCell<Vec<usize>>,
    observed_states: RefCell<Vec<usize>>,
}

impl ExternalSubscriptionLog {
    #[must_use]
    pub const fn declarations(&self) -> usize {
        self.declarations.get()
    }

    #[must_use]
    pub fn polled_declarations(&self) -> Vec<usize> {
        self.polled_declarations.borrow().clone()
    }

    #[must_use]
    pub fn observed_states(&self) -> Vec<usize> {
        self.observed_states.borrow().clone()
    }
}

pub struct ExternalActivationSubscriptionWidget<Action> {
    log: Rc<ExternalSubscriptionLog>,
    primary: Box<dyn FnMut() -> Action>,
    auxiliary: Box<dyn FnMut() -> Action>,
    updated_state: Option<usize>,
}

impl<Action> core::fmt::Debug for ExternalActivationSubscriptionWidget<Action> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ExternalActivationSubscriptionWidget")
            .finish_non_exhaustive()
    }
}

impl<Action> ExternalActivationSubscriptionWidget<Action> {
    #[must_use]
    pub fn new(
        log: Rc<ExternalSubscriptionLog>,
        primary: impl FnMut() -> Action + 'static,
        auxiliary: impl FnMut() -> Action + 'static,
    ) -> Self {
        Self {
            log,
            primary: Box::new(primary),
            auxiliary: Box::new(auxiliary),
            updated_state: None,
        }
    }

    #[must_use]
    pub const fn updated_state(mut self, updated_state: usize) -> Self {
        self.updated_state = Some(updated_state);
        self
    }
}

impl<Action> Widget<Action> for ExternalActivationSubscriptionWidget<Action> {
    type State = usize;

    fn create_state(&self) -> Self::State {
        0
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if let Some(updated_state) = self.updated_state {
            *state = updated_state;
            context.invalidate_subscriptions();
        }
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        *state += 1;
        context.invalidate_subscriptions();
        context.emit((self.auxiliary)());
        WidgetActivationOutput::changed_with_action((self.primary)())
    }

    fn subscriptions(&self, state: &Self::State, _: &mut SubscriptionSet<Action>) {
        self.log.declarations.set(self.log.declarations.get() + 1);
        self.log.observed_states.borrow_mut().push(*state);
    }
}

/// Genuine downstream widget that declares one owner-local subscription.
#[derive(Debug)]
pub struct ExternalSubscriptionWidget {
    log: Rc<ExternalSubscriptionLog>,
    revision: u64,
    enabled: bool,
    duplicate: bool,
}

impl ExternalSubscriptionWidget {
    #[must_use]
    pub const fn new(log: Rc<ExternalSubscriptionLog>) -> Self {
        Self {
            log,
            revision: 1,
            enabled: true,
            duplicate: false,
        }
    }

    #[must_use]
    pub const fn revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn duplicate(mut self, duplicate: bool) -> Self {
        self.duplicate = duplicate;
        self
    }
}

impl<Action> Widget<Action> for ExternalSubscriptionWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn update(&self, (): &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        context.invalidate_subscriptions();
    }

    fn subscriptions(&self, (): &Self::State, subscriptions: &mut SubscriptionSet<Action>) {
        if !self.enabled {
            return;
        }
        let declaration = self.log.declarations.get() + 1;
        self.log.declarations.set(declaration);
        let log = Rc::clone(&self.log);
        subscriptions.local(
            WorkKey::new("external.subscription").unwrap_or_else(|_| unreachable!()),
            self.revision,
            move |_: &mut std::task::Context<'_>| {
                log.polled_declarations.borrow_mut().push(declaration);
                Poll::Pending
            },
        );
        if self.duplicate {
            subscriptions.local(
                WorkKey::new("external.subscription").unwrap_or_else(|_| unreachable!()),
                self.revision,
                |_: &mut std::task::Context<'_>| Poll::Pending,
            );
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ChildAction {
    Pulse,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ParentAction {
    Child(ChildAction),
    Reset,
}

#[derive(Debug, Default)]
pub struct PulseState {
    lifecycle_count: usize,
}

#[derive(Debug)]
pub struct PulseButton {
    label: String,
    enabled: bool,
}

impl PulseButton {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }

    #[must_use]
    pub const fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

impl Widget<ChildAction> for PulseButton {
    type State = PulseState;

    fn create_state(&self) -> Self::State {
        PulseState::default()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut WidgetActivationContext<ChildAction>,
    ) -> WidgetActivationOutput<ChildAction> {
        if self.enabled {
            state.lifecycle_count += 1;
            context.invalidate(WidgetInvalidation::PAINT | WidgetInvalidation::SEMANTICS);
            WidgetActivationOutput::changed_with_action(ChildAction::Pulse)
        } else {
            WidgetActivationOutput::none()
        }
    }

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(80.0).unwrap_or_default(),
            height: LogicalLength::new(24.0).unwrap_or_default(),
        }
    }

    fn paint(&self, _state: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::new("pulse", format!("label={:?}", self.label))
    }

    fn semantics(
        &self,
        _state: &Self::State,
        _context: SemanticContributionContext,
    ) -> SemanticContribution {
        let node = SemanticNodeContribution::primary(SemanticRole::Button)
            .with_name(self.label.clone())
            .with_state(SemanticState::ENABLED.with_disabled(!self.enabled))
            .with_action(SemanticAction::Activate);
        SemanticContribution::single(node)
    }

    fn diagnostics(&self, _state: &Self::State) -> Vec<WidgetDiagnostic> {
        vec![WidgetDiagnostic::new(
            "external.pulse.ready",
            format!("{} is ready", self.label),
        )]
    }

    fn mount(&self, state: &mut Self::State, context: &mut WidgetMountContext<ChildAction>) {
        state.lifecycle_count += 1;
        context.invalidate(WidgetInvalidation::DIAGNOSTICS);
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<ChildAction>) {
        state.lifecycle_count += 1;
        context.invalidate(WidgetInvalidation::DIAGNOSTICS);
    }

    fn unmount(&self, state: &mut Self::State, _context: &mut WidgetUnmountContext) {
        state.lifecycle_count += 1;
    }
}

#[must_use]
pub fn child_component() -> Element<ChildAction> {
    Element::new(PulseButton::new("Pulse"))
        .id("external.pulse")
        .key("pulse-key")
        .padding(EdgeInsets::all(LogicalLength::from(2_u16)))
}

#[must_use]
pub fn parent_view() -> Element<ParentAction> {
    custom_column(children![
        text("External widget"),
        child_component().map_action(ParentAction::Child),
        button("Reset")
            .id("external.reset")
            .on_activate(|| ParentAction::Reset),
    ])
    .id("external.panel")
    .key("panel-key")
    .into_element()
}

#[derive(Debug)]
pub struct CustomColumn;

impl<Action> Widget<Action> for CustomColumn {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn paint(&self, _state: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::new("external-panel", "axis=Vertical")
    }
    fn semantics(
        &self,
        _state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node =
            SemanticNodeContribution::primary(SemanticRole::Group).with_name("External panel");
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
    fn diagnostics(&self, _state: &Self::State) -> Vec<WidgetDiagnostic> {
        vec![WidgetDiagnostic::new(
            "external.panel.ready",
            "external child-layout widget is ready",
        )]
    }
}

impl<Action> ChildLayoutWidget<Action> for CustomColumn {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[must_use]
pub fn custom_column<Action>(children: impl Views<Action>) -> Container<Action> {
    container(CustomColumn, children)
}

#[derive(Debug)]
pub struct CustomRow;

impl<Action> Widget<Action> for CustomRow {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn paint(&self, _state: &Self::State) -> WidgetPaintProof {
        WidgetPaintProof::new("external-row", "axis=Horizontal")
    }
    fn semantics(
        &self,
        _state: &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node =
            SemanticNodeContribution::primary(SemanticRole::Group).with_name("External row");
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl<Action> ChildLayoutWidget<Action> for CustomRow {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

#[derive(Debug)]
pub struct MinimumPanel;

impl<Action> Widget<Action> for MinimumPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(180.0).unwrap_or_default(),
            height: LogicalLength::new(60.0).unwrap_or_default(),
        }
    }
}

impl<Action> ChildLayoutWidget<Action> for MinimumPanel {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[derive(Debug)]
pub struct TextMinimumPanel;

impl<Action> Widget<Action> for TextMinimumPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: "external text intrinsic minimum".to_owned(),
            kind: WidgetTextKind::Text,
            minimum_width: LogicalLength::ZERO,
            minimum_height: LogicalLength::ZERO,
        }
    }
}

impl<Action> ChildLayoutWidget<Action> for TextMinimumPanel {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Horizontal,
        }
    }
}

#[derive(Debug)]
pub struct UnsupportedMinimumPanel;

impl<Action> Widget<Action> for UnsupportedMinimumPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Unsupported {
            reason: "external child-layout intrinsic proof",
        }
    }
}

impl<Action> ChildLayoutWidget<Action> for UnsupportedMinimumPanel {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        ChildLayout::Linear {
            axis: Axis::Vertical,
        }
    }
}

#[must_use]
pub fn custom_row<Action>(children: impl Views<Action>) -> Container<Action> {
    container(CustomRow, children)
}

#[must_use]
pub fn minimum_panel<Action>(children: impl Views<Action>) -> Container<Action> {
    container(MinimumPanel, children)
}

#[must_use]
pub fn text_minimum_panel<Action>(children: impl Views<Action>) -> Container<Action> {
    container(TextMinimumPanel, children)
}

#[must_use]
pub fn unsupported_minimum_panel<Action>(children: impl Views<Action>) -> Container<Action> {
    container(UnsupportedMinimumPanel, children)
}

#[must_use]
pub fn diagnostic_panel() -> Element<ParentAction> {
    custom_column(children![
        text("first").id("external.duplicate").key("duplicate-key"),
        custom_column(children![
            text("nested").id("external.duplicate"),
            button("Nested action")
                .id("external.nested")
                .on_activate(|| ChildAction::Pulse)
                .into_element()
                .map_action(ParentAction::Child),
        ])
        .key("duplicate-key"),
        text("third"),
        text("fourth"),
    ])
    .id("external.diagnostic-panel")
    .into_element()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutCase {
    BuiltInColumn,
    ExternalColumn,
    ExternalRow,
    FixedMinimum,
    TextMinimum,
    UnsupportedMinimum,
    NestedExternal,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LayoutAction {
    Activate,
}

#[derive(Debug)]
pub struct LayoutState {
    pub case: LayoutCase,
    pub activations: usize,
}

fn layout_children() -> Vec<Element<LayoutAction>> {
    vec![
        text("layout child").id("layout.label").into_element(),
        button("Activate")
            .id("layout.action")
            .key("action-key")
            .on_activate(|| LayoutAction::Activate)
            .into_element(),
        text("tail").id("layout.tail").into_element(),
    ]
}

#[must_use]
pub fn layout_case_view(case: LayoutCase) -> Element<LayoutAction> {
    match case {
        LayoutCase::BuiltInColumn => column(layout_children())
            .id("layout.root")
            .gap(3_u16)
            .into_element(),
        LayoutCase::ExternalColumn => custom_column(layout_children())
            .id("layout.root")
            .gap(5_u16)
            .into_element(),
        LayoutCase::ExternalRow => custom_row(layout_children())
            .id("layout.root")
            .gap(7_u16)
            .into_element(),
        LayoutCase::FixedMinimum => minimum_panel(layout_children())
            .id("layout.root")
            .gap(4_u16)
            .into_element(),
        LayoutCase::TextMinimum => text_minimum_panel(layout_children())
            .id("layout.root")
            .gap(6_u16)
            .into_element(),
        LayoutCase::UnsupportedMinimum => unsupported_minimum_panel(layout_children())
            .id("layout.root")
            .gap(8_u16)
            .into_element(),
        LayoutCase::NestedExternal => custom_column(children![
            text("nested head").id("layout.label"),
            custom_row(children![
                button("Activate")
                    .id("layout.action")
                    .key("action-key")
                    .on_activate(|| LayoutAction::Activate),
                text("nested tail").id("layout.tail"),
            ])
            .id("layout.nested")
            .gap(11_u16),
        ])
        .id("layout.root")
        .gap(13_u16)
        .into_element(),
    }
}

pub struct LayoutConformanceApp;

impl UiApp for LayoutConformanceApp {
    type State = LayoutState;
    type Action = LayoutAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        layout_case_view(state.case)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            LayoutAction::Activate => state.activations += 1,
        }
    }
}

pub struct ConformanceApp;

impl UiApp for ConformanceApp {
    type State = usize;
    type Action = ParentAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        parent_view()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            ParentAction::Child(ChildAction::Pulse) => *state += 1,
            ParentAction::Reset => *state = 0,
        }
    }
}

#[derive(Debug)]
pub struct GenericWidget<T>(pub T);

impl<T: core::fmt::Debug + 'static> Widget<()> for GenericWidget<T> {
    type State = ();
    fn create_state(&self) -> Self::State {}
}

#[derive(Debug)]
pub struct CountingLayoutPanel {
    measure_calls: Rc<Cell<usize>>,
    child_layout_calls: Rc<Cell<usize>>,
}

impl Widget<()> for CountingLayoutPanel {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        self.measure_calls.set(self.measure_calls.get() + 1);
        WidgetMeasure::default()
    }
}

impl ChildLayoutWidget<()> for CountingLayoutPanel {
    fn child_layout(&self, _state: &Self::State) -> ChildLayout {
        let call = self.child_layout_calls.get() + 1;
        self.child_layout_calls.set(call);
        ChildLayout::Linear {
            axis: if call % 2 == 1 {
                Axis::Vertical
            } else {
                Axis::Horizontal
            },
        }
    }
}

#[derive(Debug)]
struct CountingText {
    calls: Rc<Cell<usize>>,
}

impl Widget<()> for CountingText {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        self.calls.set(self.calls.get() + 1);
        WidgetMeasure::Text {
            content: "counted descriptor".to_owned(),
            kind: WidgetTextKind::ControlLabel,
            minimum_width: LogicalLength::ZERO,
            minimum_height: LogicalLength::ZERO,
        }
    }
}

#[derive(Debug)]
struct CountingFixed {
    calls: Rc<Cell<usize>>,
}

impl Widget<()> for CountingFixed {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        self.calls.set(self.calls.get() + 1);
        WidgetMeasure::Fixed {
            width: LogicalLength::new(20.0).unwrap_or_default(),
            height: LogicalLength::new(7.0).unwrap_or_default(),
        }
    }
}

#[must_use]
pub fn counting_measurement_tree(
    panel_calls: Rc<Cell<usize>>,
    child_layout_calls: Rc<Cell<usize>>,
    text_calls: Rc<Cell<usize>>,
    fixed_calls: Rc<Cell<usize>>,
) -> Element<()> {
    container(
        CountingLayoutPanel {
            measure_calls: panel_calls,
            child_layout_calls,
        },
        children![
            Element::new(CountingText { calls: text_calls }).id("external.counted-text"),
            Element::new(CountingFixed { calls: fixed_calls }).id("external.counted-fixed"),
        ],
    )
    .id("external.counting-panel")
    .into_element()
}

#[derive(Debug)]
pub struct UnsupportedMeasure;

impl Widget<()> for UnsupportedMeasure {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Unsupported {
            reason: "external proof capability",
        }
    }
}
