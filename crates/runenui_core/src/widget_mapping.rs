use crate::element::{
    ChildLayout, WidgetActivation, WidgetDiagnostic, WidgetMeasure, WidgetPaintProof,
    WidgetSemanticProof, WidgetStateTypeId, WidgetTypeId,
};
use crate::widget_erasure::{ErasedWidget, WidgetBridgeError};
use crate::{
    WidgetActivationContext, WidgetMountContext, WidgetUnmountContext, WidgetUpdateContext,
};
use core::{any::Any, fmt};
use std::rc::Rc;

pub struct MappedWidget<ChildAction, ParentAction> {
    pub child: Box<dyn ErasedWidget<ChildAction>>,
    pub mapper: Rc<dyn Fn(ChildAction) -> ParentAction>,
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
    fn mount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetMountContext,
    ) -> Result<(), WidgetBridgeError> {
        self.child.mount(state, context)
    }
    fn update(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUpdateContext,
    ) -> Result<(), WidgetBridgeError> {
        self.child.update(state, context)
    }
    fn unmount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUnmountContext,
    ) -> Result<(), WidgetBridgeError> {
        self.child.unmount(state, context)
    }
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError> {
        self.child.activation(state)
    }
    fn activate(
        &mut self,
        state: &mut dyn Any,
        context: &mut WidgetActivationContext,
    ) -> Result<Option<ParentAction>, WidgetBridgeError> {
        Ok(self
            .child
            .activate(state, context)?
            .map(self.mapper.as_ref()))
    }
    fn measure(&self, state: &dyn Any) -> Result<WidgetMeasure, WidgetBridgeError> {
        self.child.measure(state)
    }
    fn child_layout(&self, state: &dyn Any) -> Result<Option<ChildLayout>, WidgetBridgeError> {
        self.child.child_layout(state)
    }
    fn paint(&self, state: &dyn Any) -> Result<WidgetPaintProof, WidgetBridgeError> {
        self.child.paint(state)
    }
    fn semantics(&self, state: &dyn Any) -> Result<WidgetSemanticProof, WidgetBridgeError> {
        self.child.semantics(state)
    }
    fn diagnostics(&self, state: &dyn Any) -> Result<Vec<WidgetDiagnostic>, WidgetBridgeError> {
        self.child.diagnostics(state)
    }
}
