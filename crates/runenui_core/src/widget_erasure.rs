use crate::element::{
    AuthoredElementFields, AuthoringDiagnostic, ChildLayout, ChildLayoutWidget, Element, Widget,
    WidgetActivation, WidgetActivationOutput, WidgetDiagnostic, WidgetMeasure, WidgetPaintProof,
    WidgetStateTypeId, WidgetTextInput, WidgetTypeId,
};
use crate::{
    CommandOrigin, ElementId, ElementKey, EventContext, EventPhase, FocusScope, Focusability,
    LayoutStyle, MonotonicInstant, MountedNodeId, PointerId, SemanticContribution,
    SemanticContributionContext, StyleIntent, SubscriptionSet, UiEvent, WidgetActivationContext,
    WidgetEventOutput, WidgetMountContext, WidgetUnmountContext, WidgetUpdateContext, WorkSequence,
};
use core::{any::Any, fmt};

pub trait ErasedWidget<Action>: fmt::Debug {
    fn widget_type_id(&self) -> WidgetTypeId;
    fn widget_type_name(&self) -> &'static str;
    fn state_type_id(&self) -> WidgetStateTypeId;
    fn create_state(&self) -> Box<dyn Any>;
    fn mount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetMountContext<Action>,
    ) -> Result<(), WidgetBridgeError>;
    fn update(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUpdateContext<Action>,
    ) -> Result<(), WidgetBridgeError>;
    fn unmount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUnmountContext,
    ) -> Result<(), WidgetBridgeError>;
    fn subscriptions(
        &self,
        state: &dyn Any,
        subscriptions: &mut SubscriptionSet<Action>,
    ) -> Result<(), WidgetBridgeError>;
    fn event_bridge_matches(&self, state: &dyn Any) -> bool;
    fn event(
        &mut self,
        state: &mut dyn Any,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> Result<WidgetEventOutput, WidgetBridgeError>;
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError>;
    fn text_input(&self, state: &dyn Any) -> Result<WidgetTextInput, WidgetBridgeError>;
    fn activate(
        &mut self,
        state: &mut dyn Any,
        context: &mut WidgetActivationContext<Action>,
    ) -> Result<WidgetActivationOutput<Action>, WidgetBridgeError>;
    fn measure(&self, state: &dyn Any) -> Result<WidgetMeasure, WidgetBridgeError>;
    fn child_layout(&self, state: &dyn Any) -> Result<Option<ChildLayout>, WidgetBridgeError>;
    fn paint(&self, state: &dyn Any) -> Result<WidgetPaintProof, WidgetBridgeError>;
    fn semantics(
        &self,
        state: &dyn Any,
        context: SemanticContributionContext,
    ) -> Result<SemanticContribution, WidgetBridgeError>;
    fn diagnostics(&self, state: &dyn Any) -> Result<Vec<WidgetDiagnostic>, WidgetBridgeError>;
}

#[derive(Debug)]
pub struct WidgetAdapter<Implementation>(pub Implementation);

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

    fn mount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetMountContext<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .mount(downcast_mut::<Implementation::State>(state)?, context);
        Ok(())
    }
    fn update(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUpdateContext<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .update(downcast_mut::<Implementation::State>(state)?, context);
        Ok(())
    }
    fn unmount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUnmountContext,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .unmount(downcast_mut::<Implementation::State>(state)?, context);
        Ok(())
    }
    fn subscriptions(
        &self,
        state: &dyn Any,
        subscriptions: &mut SubscriptionSet<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .subscriptions(downcast_ref::<Implementation::State>(state)?, subscriptions);
        Ok(())
    }
    fn event_bridge_matches(&self, state: &dyn Any) -> bool {
        state.is::<Implementation::State>()
    }
    fn event(
        &mut self,
        state: &mut dyn Any,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> Result<WidgetEventOutput, WidgetBridgeError> {
        Ok(self.0.event(
            downcast_mut::<Implementation::State>(state)?,
            event,
            context,
        ))
    }
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError> {
        Ok(self
            .0
            .activation(downcast_ref::<Implementation::State>(state)?))
    }
    fn text_input(&self, state: &dyn Any) -> Result<WidgetTextInput, WidgetBridgeError> {
        Ok(self
            .0
            .text_input(downcast_ref::<Implementation::State>(state)?))
    }
    fn activate(
        &mut self,
        state: &mut dyn Any,
        context: &mut WidgetActivationContext<Action>,
    ) -> Result<WidgetActivationOutput<Action>, WidgetBridgeError> {
        Ok(self
            .0
            .activate(downcast_mut::<Implementation::State>(state)?, context))
    }
    fn measure(&self, state: &dyn Any) -> Result<WidgetMeasure, WidgetBridgeError> {
        Ok(self
            .0
            .measure(downcast_ref::<Implementation::State>(state)?))
    }
    fn child_layout(&self, _state: &dyn Any) -> Result<Option<ChildLayout>, WidgetBridgeError> {
        Ok(None)
    }
    fn paint(&self, state: &dyn Any) -> Result<WidgetPaintProof, WidgetBridgeError> {
        Ok(self.0.paint(downcast_ref::<Implementation::State>(state)?))
    }
    fn semantics(
        &self,
        state: &dyn Any,
        context: SemanticContributionContext,
    ) -> Result<SemanticContribution, WidgetBridgeError> {
        Ok(self
            .0
            .semantics(downcast_ref::<Implementation::State>(state)?, context))
    }
    fn diagnostics(&self, state: &dyn Any) -> Result<Vec<WidgetDiagnostic>, WidgetBridgeError> {
        Ok(self
            .0
            .diagnostics(downcast_ref::<Implementation::State>(state)?))
    }
}

#[derive(Debug)]
pub struct ChildLayoutWidgetAdapter<Implementation>(pub Implementation);

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
    fn mount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetMountContext<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .mount(downcast_mut::<Implementation::State>(state)?, context);
        Ok(())
    }
    fn update(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUpdateContext<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .update(downcast_mut::<Implementation::State>(state)?, context);
        Ok(())
    }
    fn unmount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUnmountContext,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .unmount(downcast_mut::<Implementation::State>(state)?, context);
        Ok(())
    }
    fn subscriptions(
        &self,
        state: &dyn Any,
        subscriptions: &mut SubscriptionSet<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.0
            .subscriptions(downcast_ref::<Implementation::State>(state)?, subscriptions);
        Ok(())
    }
    fn event_bridge_matches(&self, state: &dyn Any) -> bool {
        state.is::<Implementation::State>()
    }
    fn event(
        &mut self,
        state: &mut dyn Any,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> Result<WidgetEventOutput, WidgetBridgeError> {
        Ok(self.0.event(
            downcast_mut::<Implementation::State>(state)?,
            event,
            context,
        ))
    }
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError> {
        Ok(self
            .0
            .activation(downcast_ref::<Implementation::State>(state)?))
    }
    fn text_input(&self, state: &dyn Any) -> Result<WidgetTextInput, WidgetBridgeError> {
        Ok(self
            .0
            .text_input(downcast_ref::<Implementation::State>(state)?))
    }
    fn activate(
        &mut self,
        state: &mut dyn Any,
        context: &mut WidgetActivationContext<Action>,
    ) -> Result<WidgetActivationOutput<Action>, WidgetBridgeError> {
        Ok(self
            .0
            .activate(downcast_mut::<Implementation::State>(state)?, context))
    }
    fn measure(&self, state: &dyn Any) -> Result<WidgetMeasure, WidgetBridgeError> {
        Ok(self
            .0
            .measure(downcast_ref::<Implementation::State>(state)?))
    }
    fn child_layout(&self, state: &dyn Any) -> Result<Option<ChildLayout>, WidgetBridgeError> {
        Ok(Some(self.0.child_layout(downcast_ref::<
            Implementation::State,
        >(state)?)))
    }
    fn paint(&self, state: &dyn Any) -> Result<WidgetPaintProof, WidgetBridgeError> {
        Ok(self.0.paint(downcast_ref::<Implementation::State>(state)?))
    }
    fn semantics(
        &self,
        state: &dyn Any,
        context: SemanticContributionContext,
    ) -> Result<SemanticContribution, WidgetBridgeError> {
        Ok(self
            .0
            .semantics(downcast_ref::<Implementation::State>(state)?, context))
    }
    fn diagnostics(&self, state: &dyn Any) -> Result<Vec<WidgetDiagnostic>, WidgetBridgeError> {
        Ok(self
            .0
            .diagnostics(downcast_ref::<Implementation::State>(state)?))
    }
}
fn downcast_ref<State: 'static>(state: &dyn Any) -> Result<&State, WidgetBridgeError> {
    state
        .downcast_ref::<State>()
        .ok_or(WidgetBridgeError::StatePayloadMismatch)
}

fn downcast_mut<State: 'static>(state: &mut dyn Any) -> Result<&mut State, WidgetBridgeError> {
    state
        .downcast_mut::<State>()
        .ok_or(WidgetBridgeError::StatePayloadMismatch)
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetBridgeError {
    StatePayloadMismatch,
}

/// Unstable, non-forgeable erased state owned by the mounted runtime.
#[doc(hidden)]
pub struct MountedWidgetState {
    value: Box<dyn Any>,
}

impl fmt::Debug for MountedWidgetState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("MountedWidgetState(..)")
    }
}

/// Unstable checked widget bridge used only by `runenui_runtime`.
#[doc(hidden)]
pub struct MountedWidget<Action> {
    inner: Box<dyn ErasedWidget<Action>>,
}

impl<Action> fmt::Debug for MountedWidget<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(formatter)
    }
}

#[allow(clippy::missing_errors_doc)]
impl<Action> MountedWidget<Action> {
    pub(crate) fn from_erased(inner: Box<dyn ErasedWidget<Action>>) -> Self {
        Self { inner }
    }
    #[must_use]
    pub fn widget_type_id(&self) -> WidgetTypeId {
        self.inner.widget_type_id()
    }
    #[must_use]
    pub fn widget_type_name(&self) -> &'static str {
        self.inner.widget_type_name()
    }
    #[must_use]
    pub fn state_type_id(&self) -> WidgetStateTypeId {
        self.inner.state_type_id()
    }
    #[must_use]
    pub fn create_state(&self) -> MountedWidgetState {
        MountedWidgetState {
            value: self.inner.create_state(),
        }
    }
    pub fn mount(
        &self,
        state: &mut MountedWidgetState,
        context: &mut WidgetMountContext<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.inner.mount(state.value.as_mut(), context)
    }
    pub fn update(
        &self,
        state: &mut MountedWidgetState,
        context: &mut WidgetUpdateContext<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.inner.update(state.value.as_mut(), context)
    }
    pub fn unmount(
        &self,
        state: &mut MountedWidgetState,
        context: &mut WidgetUnmountContext,
    ) -> Result<(), WidgetBridgeError> {
        self.inner.unmount(state.value.as_mut(), context)
    }
    pub fn subscriptions(
        &self,
        state: &MountedWidgetState,
        subscriptions: &mut SubscriptionSet<Action>,
    ) -> Result<(), WidgetBridgeError> {
        self.inner
            .subscriptions(state.value.as_ref(), subscriptions)
    }
    #[must_use]
    pub fn event_bridge_matches(&self, state: &MountedWidgetState) -> bool {
        self.inner.event_bridge_matches(state.value.as_ref())
    }
    /// Invokes one event callback through the checked core-owned context bridge.
    ///
    /// The runtime supplies invocation facts, but cannot construct or extract
    /// the borrowed [`EventContext`] itself. These opaque mounted values cannot
    /// be obtained from a live runtime by downstream code.
    #[allow(clippy::too_many_arguments)]
    pub fn event(
        &mut self,
        state: &mut MountedWidgetState,
        event: &UiEvent,
        phase: EventPhase,
        original_target: &MountedNodeId,
        current_target: &MountedNodeId,
        related_target: Option<&MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Result<
        (
            WidgetEventOutput,
            crate::event_context::EventContextOutput<Action>,
        ),
        WidgetBridgeError,
    > {
        let context = EventContext::new(
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        );
        self.event_with_context(state, event, context)
    }

    /// Invokes one pointer-family callback with immutable physical routing facts.
    #[allow(clippy::too_many_arguments)]
    pub fn pointer_event(
        &mut self,
        state: &mut MountedWidgetState,
        event: &UiEvent,
        phase: EventPhase,
        original_target: &MountedNodeId,
        current_target: &MountedNodeId,
        related_target: Option<&MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        pointer_id: PointerId,
        physical_target: Option<&MountedNodeId>,
        physical_path: &[MountedNodeId],
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Result<
        (
            WidgetEventOutput,
            crate::event_context::EventContextOutput<Action>,
        ),
        WidgetBridgeError,
    > {
        let context = EventContext::new_pointer(
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            pointer_id,
            physical_target,
            physical_path,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        );
        self.event_with_context(state, event, context)
    }

    fn event_with_context(
        &mut self,
        state: &mut MountedWidgetState,
        event: &UiEvent,
        mut context: EventContext<'_, Action>,
    ) -> Result<
        (
            WidgetEventOutput,
            crate::event_context::EventContextOutput<Action>,
        ),
        WidgetBridgeError,
    > {
        let widget = self
            .inner
            .event(state.value.as_mut(), event, &mut context)?;
        Ok((widget, context.into_output()))
    }

    pub fn activation(
        &self,
        state: &MountedWidgetState,
    ) -> Result<WidgetActivation, WidgetBridgeError> {
        self.inner.activation(state.value.as_ref())
    }
    pub fn text_input(
        &self,
        state: &MountedWidgetState,
    ) -> Result<WidgetTextInput, WidgetBridgeError> {
        self.inner.text_input(state.value.as_ref())
    }
    pub fn activate(
        &mut self,
        state: &mut MountedWidgetState,
        context: &mut WidgetActivationContext<Action>,
    ) -> Result<WidgetActivationOutput<Action>, WidgetBridgeError> {
        self.inner.activate(state.value.as_mut(), context)
    }
    pub fn measure(&self, state: &MountedWidgetState) -> Result<WidgetMeasure, WidgetBridgeError> {
        self.inner.measure(state.value.as_ref())
    }
    pub fn child_layout(
        &self,
        state: &MountedWidgetState,
    ) -> Result<Option<ChildLayout>, WidgetBridgeError> {
        self.inner.child_layout(state.value.as_ref())
    }
    pub fn paint(&self, state: &MountedWidgetState) -> Result<WidgetPaintProof, WidgetBridgeError> {
        self.inner.paint(state.value.as_ref())
    }
    pub fn semantics(
        &self,
        state: &MountedWidgetState,
        context: SemanticContributionContext,
    ) -> Result<SemanticContribution, WidgetBridgeError> {
        self.inner.semantics(state.value.as_ref(), context)
    }
    pub fn diagnostics(
        &self,
        state: &MountedWidgetState,
    ) -> Result<Vec<WidgetDiagnostic>, WidgetBridgeError> {
        self.inner.diagnostics(state.value.as_ref())
    }
}

/// Unstable consumed element parts used only by `runenui_runtime`.
#[doc(hidden)]
pub struct ElementParts<Action> {
    id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    focusability: Focusability,
    focus_scope: Option<FocusScope>,
    widget: MountedWidget<Action>,
    children: Vec<Element<Action>>,
    authoring_diagnostics: Vec<AuthoringDiagnostic>,
}

#[doc(hidden)]
pub type ElementRuntimeParts<Action> = (
    Option<ElementId>,
    Option<ElementKey>,
    LayoutStyle,
    StyleIntent,
    Focusability,
    Option<FocusScope>,
    Vec<AuthoringDiagnostic>,
    MountedWidget<Action>,
    Vec<Element<Action>>,
);

impl<Action> ElementParts<Action> {
    pub(crate) fn new(
        fields: AuthoredElementFields,
        widget: MountedWidget<Action>,
        children: Vec<Element<Action>>,
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
    pub const fn id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }
    #[must_use]
    pub const fn key(&self) -> Option<&ElementKey> {
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
    pub const fn focus_scope(&self) -> Option<FocusScope> {
        self.focus_scope
    }
    #[must_use]
    pub const fn authoring_diagnostics(&self) -> &[AuthoringDiagnostic] {
        self.authoring_diagnostics.as_slice()
    }
    #[must_use]
    pub const fn widget(&self) -> &MountedWidget<Action> {
        &self.widget
    }
    #[must_use]
    pub const fn children(&self) -> &[Element<Action>] {
        self.children.as_slice()
    }
    #[must_use]
    pub fn into_parts(self) -> ElementRuntimeParts<Action> {
        (
            self.id,
            self.key,
            self.layout,
            self.style,
            self.focusability,
            self.focus_scope,
            self.authoring_diagnostics,
            self.widget,
            self.children,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::element::{Element, Widget, WidgetActivation};

    use super::WidgetBridgeError;

    #[derive(Debug)]
    struct Probe(Rc<Cell<usize>>);

    impl Widget<()> for Probe {
        type State = u8;
        fn create_state(&self) -> Self::State {
            0
        }
        fn activation(&self, _: &Self::State) -> WidgetActivation {
            self.0.set(self.0.get() + 1);
            WidgetActivation::NONE
        }
    }

    #[test]
    fn corrupted_erased_payload_never_invokes_typed_callback() {
        let calls = Rc::new(Cell::new(0));
        let parts = Element::new(Probe(Rc::clone(&calls))).into_runtime_parts();
        let (_, _, _, _, _, _, _, widget, _) = parts.into_parts();
        let mut state = widget.create_state();
        state.value = Box::new(String::from("wrong"));
        assert_eq!(
            widget.activation(&state),
            Err(WidgetBridgeError::StatePayloadMismatch)
        );
        assert_eq!(calls.get(), 0);
    }
}
