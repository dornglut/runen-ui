use crate::element::{
    WidgetActivation, WidgetActivationOutput, WidgetDiagnostic, WidgetMeasure, WidgetMeasureInput,
    WidgetStateTypeId, WidgetTextInput, WidgetTypeId,
};
use crate::widget_erasure::{ErasedWidget, WidgetBridgeError};
use crate::{
    EventContext, HitContribution, HitContributionContext, PaintContribution,
    PaintContributionContext, SemanticContribution, SemanticContributionContext, SubscriptionSet,
    UiEvent, WidgetActivationContext, WidgetEventOutput, WidgetMountContext, WidgetUnmountContext,
    WidgetUpdateContext,
};
use core::{any::Any, fmt};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::rc::Rc;

use crate::__runtime::MountedEffect;

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
where
    ChildAction: 'static,
    ParentAction: 'static,
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
        context: &mut WidgetMountContext<ParentAction>,
    ) -> Result<(), WidgetBridgeError> {
        let mut child_context = WidgetMountContext::__runtime_new();
        self.child.mount(state, &mut child_context)?;
        transfer_context(child_context, context, &self.mapper);
        Ok(())
    }
    fn update(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUpdateContext<ParentAction>,
    ) -> Result<(), WidgetBridgeError> {
        let mut child_context = WidgetUpdateContext::__runtime_new();
        self.child.update(state, &mut child_context)?;
        transfer_context(child_context, context, &self.mapper);
        Ok(())
    }
    fn unmount(
        &self,
        state: &mut dyn Any,
        context: &mut WidgetUnmountContext,
    ) -> Result<(), WidgetBridgeError> {
        self.child.unmount(state, context)
    }
    fn subscriptions(
        &self,
        state: &dyn Any,
        subscriptions: &mut SubscriptionSet<ParentAction>,
    ) -> Result<(), WidgetBridgeError> {
        let mut child_subscriptions = SubscriptionSet::new();
        self.child.subscriptions(state, &mut child_subscriptions)?;
        for declaration in child_subscriptions.__runtime_into_declarations() {
            let crate::subscription::Subscription {
                key,
                source_type,
                revision,
                source,
            } = declaration;
            let mapper = Rc::clone(&self.mapper);
            let source = match source {
                crate::subscription::SubscriptionSource::Local(source) => {
                    crate::subscription::SubscriptionSource::Local(Box::pin(MappedLocalSource {
                        source,
                        mapper,
                    }))
                }
                crate::subscription::SubscriptionSource::Send { source, mut map } => {
                    crate::subscription::SubscriptionSource::Send {
                        source,
                        map: Box::new(move |item| mapper(map(item))),
                    }
                }
            };
            subscriptions.__runtime_push(crate::subscription::Subscription {
                key,
                source_type,
                revision,
                source,
            });
        }
        Ok(())
    }
    fn event_bridge_matches(&self, state: &dyn Any) -> bool {
        self.child.event_bridge_matches(state)
    }
    fn event(
        &mut self,
        state: &mut dyn Any,
        event: &UiEvent,
        context: &mut EventContext<'_, ParentAction>,
    ) -> Result<WidgetEventOutput, WidgetBridgeError> {
        let mut child_context = context.mapped_child();
        let output = self.child.event(state, event, &mut child_context)?;
        context.absorb_mapped(child_context.into_output(), &self.mapper);
        Ok(output)
    }
    fn activation(&self, state: &dyn Any) -> Result<WidgetActivation, WidgetBridgeError> {
        self.child.activation(state)
    }
    fn text_input(&self, state: &dyn Any) -> Result<WidgetTextInput, WidgetBridgeError> {
        self.child.text_input(state)
    }
    fn activate(
        &mut self,
        state: &mut dyn Any,
        context: &mut WidgetActivationContext<ParentAction>,
    ) -> Result<WidgetActivationOutput<ParentAction>, WidgetBridgeError> {
        let mut child_context = WidgetActivationContext::__runtime_new_with_semantic_target(
            context.semantic_action_target().cloned(),
        );
        let action = self.child.activate(state, &mut child_context)?;
        transfer_context(child_context, context, &self.mapper);
        Ok(action.map_action(self.mapper.as_ref()))
    }
    fn measure(
        &self,
        state: &dyn Any,
        input: WidgetMeasureInput,
    ) -> Result<WidgetMeasure, WidgetBridgeError> {
        self.child.measure(state, input)
    }
    fn paint(
        &self,
        state: &dyn Any,
        context: PaintContributionContext,
    ) -> Result<PaintContribution, WidgetBridgeError> {
        self.child.paint(state, context)
    }
    fn hit_test(
        &self,
        state: &dyn Any,
        context: HitContributionContext,
    ) -> Result<HitContribution, WidgetBridgeError> {
        self.child.hit_test(state, context)
    }
    fn semantics(
        &self,
        state: &dyn Any,
        context: SemanticContributionContext,
    ) -> Result<SemanticContribution, WidgetBridgeError> {
        self.child.semantics(state, context)
    }
    fn diagnostics(&self, state: &dyn Any) -> Result<Vec<WidgetDiagnostic>, WidgetBridgeError> {
        self.child.diagnostics(state)
    }
}

struct MappedLocalSource<ChildAction, ParentAction> {
    source: Pin<Box<dyn crate::LocalSubscriptionSource<ChildAction>>>,
    mapper: Rc<dyn Fn(ChildAction) -> ParentAction>,
}

impl<ChildAction: 'static, ParentAction: 'static> crate::LocalSubscriptionSource<ParentAction>
    for MappedLocalSource<ChildAction, ParentAction>
{
    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<ParentAction>> {
        let this = self.get_mut();
        this.source
            .as_mut()
            .poll_next(context)
            .map(|item| item.map(this.mapper.as_ref()))
    }
}

trait TransferContext<Action> {
    fn take_invalidation(&mut self) -> crate::WidgetInvalidation;
    fn take_subscription_invalidation(&mut self) -> bool;
    fn take_outputs(&mut self) -> Vec<MountedEffect<Action>>;
}

macro_rules! transfer_context_impl {
    ($context:ident) => {
        impl<Action> TransferContext<Action> for $context<Action> {
            fn take_invalidation(&mut self) -> crate::WidgetInvalidation {
                self.__runtime_take_invalidation()
            }
            fn take_subscription_invalidation(&mut self) -> bool {
                self.__runtime_take_subscription_invalidation()
            }
            fn take_outputs(&mut self) -> Vec<MountedEffect<Action>> {
                self.__runtime_take_outputs()
            }
        }
    };
}

transfer_context_impl!(WidgetMountContext);
transfer_context_impl!(WidgetUpdateContext);
transfer_context_impl!(WidgetActivationContext);

fn transfer_context<ChildAction: 'static, ParentAction: 'static>(
    mut child: impl TransferContext<ChildAction>,
    parent: &mut impl ParentContext<ParentAction>,
    mapper: &Rc<dyn Fn(ChildAction) -> ParentAction>,
) {
    parent.invalidate(child.take_invalidation());
    if child.take_subscription_invalidation() {
        parent.invalidate_subscriptions();
    }
    for output in child.take_outputs() {
        parent.push_output(map_output(output, mapper));
    }
}

trait ParentContext<Action> {
    fn invalidate(&mut self, invalidation: crate::WidgetInvalidation);
    fn invalidate_subscriptions(&mut self);
    fn push_output(&mut self, output: MountedEffect<Action>);
}

macro_rules! parent_context_impl {
    ($context:ident) => {
        impl<Action> ParentContext<Action> for $context<Action> {
            fn invalidate(&mut self, invalidation: crate::WidgetInvalidation) {
                self.invalidate(invalidation);
            }
            fn invalidate_subscriptions(&mut self) {
                self.invalidate_subscriptions();
            }
            fn push_output(&mut self, output: MountedEffect<Action>) {
                self.__runtime_push_output(output);
            }
        }
    };
}

parent_context_impl!(WidgetMountContext);
parent_context_impl!(WidgetUpdateContext);
parent_context_impl!(WidgetActivationContext);

pub fn map_output<ChildAction: 'static, ParentAction: 'static>(
    output: MountedEffect<ChildAction>,
    mapper: &Rc<dyn Fn(ChildAction) -> ParentAction>,
) -> MountedEffect<ParentAction> {
    match output {
        MountedEffect::Action(action) => MountedEffect::Action(mapper(action)),
        MountedEffect::LocalTask(task) => {
            let mapper = Rc::clone(mapper);
            MountedEffect::LocalTask(crate::work::LocalTaskEffect {
                key: task.key,
                future: Box::pin(async move { task.future.await.map(mapper.as_ref()) }),
            })
        }
        MountedEffect::SendTask(task) => {
            let mapper_for_success = Rc::clone(mapper);
            let mapper_for_failure = Rc::clone(mapper);
            MountedEffect::SendTask(crate::work::SendTaskEffect {
                key: task.key,
                future: task.future,
                map: Box::new(move |output| mapper_for_success((task.map)(output))),
                start_failure: task.start_failure.map(|failure| {
                    Box::new(move |error| mapper_for_failure(failure(error))) as Box<_>
                }),
            })
        }
        MountedEffect::Timer(timer) => {
            let (key, delay, interval, mut action) = timer.__runtime_into_parts();
            let mapper = Rc::clone(mapper);
            let mut timer_effect = if let Some(interval) = interval {
                crate::TimerEffect::repeating(interval, move || mapper(action()))
            } else {
                crate::TimerEffect::once(delay, move || mapper(action()))
            };
            if let Some(key) = key {
                timer_effect = timer_effect.keyed(key);
            }
            MountedEffect::Timer(timer_effect)
        }
        MountedEffect::Cancel { family, key } => MountedEffect::Cancel { family, key },
    }
}
