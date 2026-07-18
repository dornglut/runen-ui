use crate::element::{
    AuthoringDiagnostic, ChildLayout, ChildLayoutWidget, Element, Widget, WidgetActivation,
    WidgetActivationOutput, WidgetDiagnostic, WidgetMeasure, WidgetPaintProof, WidgetSemanticProof,
    WidgetStateTypeId, WidgetTypeId,
};
use crate::{
    ElementId, ElementKey, LayoutStyle, StyleIntent, SubscriptionSet, WidgetActivationContext,
    WidgetMountContext, WidgetUnmountContext, WidgetUpdateContext,
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
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError>;
    fn activate(
        &mut self,
        state: &mut dyn Any,
        context: &mut WidgetActivationContext<Action>,
    ) -> Result<WidgetActivationOutput<Action>, WidgetBridgeError>;
    fn measure(&self, state: &dyn Any) -> Result<WidgetMeasure, WidgetBridgeError>;
    fn child_layout(&self, state: &dyn Any) -> Result<Option<ChildLayout>, WidgetBridgeError>;
    fn paint(&self, state: &dyn Any) -> Result<WidgetPaintProof, WidgetBridgeError>;
    fn semantics(&self, state: &dyn Any) -> Result<WidgetSemanticProof, WidgetBridgeError>;
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
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError> {
        Ok(self
            .0
            .activation(downcast_ref::<Implementation::State>(state)?))
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
    fn semantics(&self, state: &dyn Any) -> Result<WidgetSemanticProof, WidgetBridgeError> {
        Ok(self
            .0
            .semantics(downcast_ref::<Implementation::State>(state)?))
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
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError> {
        Ok(self
            .0
            .activation(downcast_ref::<Implementation::State>(state)?))
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
    fn semantics(&self, state: &dyn Any) -> Result<WidgetSemanticProof, WidgetBridgeError> {
        Ok(self
            .0
            .semantics(downcast_ref::<Implementation::State>(state)?))
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
    pub fn activation(
        &self,
        state: &MountedWidgetState,
    ) -> Result<WidgetActivation, WidgetBridgeError> {
        self.inner.activation(state.value.as_ref())
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
    ) -> Result<WidgetSemanticProof, WidgetBridgeError> {
        self.inner.semantics(state.value.as_ref())
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
    Vec<AuthoringDiagnostic>,
    MountedWidget<Action>,
    Vec<Element<Action>>,
);

impl<Action> ElementParts<Action> {
    pub(crate) const fn new(
        id: Option<ElementId>,
        key: Option<ElementKey>,
        layout: LayoutStyle,
        style: StyleIntent,
        widget: MountedWidget<Action>,
        children: Vec<Element<Action>>,
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
        let (_, _, _, _, _, widget, _) = parts.into_parts();
        let mut state = widget.create_state();
        state.value = Box::new(String::from("wrong"));
        assert_eq!(
            widget.activation(&state),
            Err(WidgetBridgeError::StatePayloadMismatch)
        );
        assert_eq!(calls.get(), 0);
    }
}
