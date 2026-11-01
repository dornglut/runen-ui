//! Runtime-owned widget lifecycle contexts and invalidation vocabulary.
//!
//! Output emission is fallible and owned; obsolete void methods are absent.
//! ```compile_fail
//! use runenui_core::WidgetMountContext;
//! fn obsolete(context: &mut WidgetMountContext<()>) { context.emit(()); }
//! ```

use core::{
    future::Future,
    ops::{BitOr, BitOrAssign},
};

use crate::{
    ApplicationCommandId, SemanticActionTarget, SendTaskStartFailure, TimerEffect, WorkFamily,
    WorkKey,
    effects::MountedEffect,
    work::{LocalTaskEffect, SendTaskEffect},
};

/// Widget capability invalidation requested from mounted behavior.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WidgetInvalidation(u8);

impl WidgetInvalidation {
    pub const NONE: Self = Self(0);
    pub const INTERACTION: Self = Self(1 << 0);
    pub const LAYOUT: Self = Self(1 << 1);
    pub const PAINT: Self = Self(1 << 2);
    pub const SEMANTICS: Self = Self(1 << 3);
    pub const DIAGNOSTICS: Self = Self(1 << 4);
    /// Re-evaluates this widget's owner-local physical hit contribution.
    pub const HIT_TEST: Self = Self(1 << 5);
    pub const ALL: Self = Self(
        Self::INTERACTION.0
            | Self::LAYOUT.0
            | Self::PAINT.0
            | Self::SEMANTICS.0
            | Self::DIAGNOSTICS.0
            | Self::HIT_TEST.0,
    );

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl BitOr for WidgetInvalidation {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}
impl BitOrAssign for WidgetInvalidation {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}

/// Ordering marker for provisional activation-context outputs.
#[doc(hidden)]
pub enum WidgetActivationContextOutputOrder {
    Mounted,
    ApplicationCommand(ApplicationCommandId),
}

/// Provisional activation-context outputs with large mounted effects retained
/// separately from the small ordering stream.
#[doc(hidden)]
pub struct WidgetActivationContextOutputs<Action> {
    mounted: Vec<MountedEffect<Action>>,
    order: Option<Vec<WidgetActivationContextOutputOrder>>,
}

impl<Action> WidgetActivationContextOutputs<Action> {
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_into_parts(
        self,
    ) -> (
        Vec<MountedEffect<Action>>,
        Option<Vec<WidgetActivationContextOutputOrder>>,
    ) {
        (self.mounted, self.order)
    }
}

pub struct WidgetOutputCollector<Action> {
    outputs: Vec<MountedEffect<Action>>,
    activation_order: Option<Vec<WidgetActivationContextOutputOrder>>,
}

impl<Action> WidgetOutputCollector<Action> {
    pub const fn new() -> Self {
        Self {
            outputs: Vec::new(),
            activation_order: None,
        }
    }

    fn push_mounted_output(&mut self, output: MountedEffect<Action>) {
        self.outputs.push(output);
        if let Some(order) = &mut self.activation_order {
            order.push(WidgetActivationContextOutputOrder::Mounted);
        }
    }

    pub fn emit(&mut self, action: Action) {
        self.push_mounted_output(MountedEffect::Action(action));
    }

    pub fn local_task(&mut self, future: impl Future<Output = Option<Action>> + 'static) {
        self.push_mounted_output(MountedEffect::LocalTask(LocalTaskEffect {
            key: None,
            future: Box::pin(future),
        }));
    }

    pub fn keyed_local_task(
        &mut self,
        key: WorkKey,
        future: impl Future<Output = Option<Action>> + 'static,
    ) {
        self.push_mounted_output(MountedEffect::LocalTask(LocalTaskEffect {
            key: Some(key),
            future: Box::pin(future),
        }));
    }

    pub fn send_task<Output>(
        &mut self,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
    ) where
        Output: Send + 'static,
    {
        self.push_send_task(None, future, map, None);
    }

    pub fn keyed_send_task<Output>(
        &mut self,
        key: WorkKey,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
    ) where
        Output: Send + 'static,
    {
        self.push_send_task(Some(key), future, map, None);
    }

    pub fn send_task_with_failure<Output>(
        &mut self,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
        start_failure: impl FnOnce(SendTaskStartFailure) -> Action + 'static,
    ) where
        Output: Send + 'static,
    {
        self.push_send_task(None, future, map, Some(Box::new(start_failure)));
    }

    fn push_send_task<Output>(
        &mut self,
        key: Option<WorkKey>,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
        start_failure: Option<Box<dyn FnOnce(SendTaskStartFailure) -> Action>>,
    ) where
        Output: Send + 'static,
    {
        self.push_mounted_output(MountedEffect::SendTask(SendTaskEffect {
            key,
            future: Box::pin(async move { Box::new(future.await) as crate::work::SendOutput }),
            map: Box::new(move |output| {
                output
                    .downcast::<Output>()
                    .map_or_else(|_| unreachable!(), |output| map(*output))
            }),
            start_failure,
        }));
    }

    pub fn timer(&mut self, timer: TimerEffect<Action>) {
        self.push_mounted_output(MountedEffect::Timer(timer));
    }

    pub fn cancel(&mut self, family: WorkFamily, key: WorkKey) {
        self.push_mounted_output(MountedEffect::Cancel { family, key });
    }

    pub fn take_outputs(&mut self) -> Vec<MountedEffect<Action>> {
        assert!(
            self.activation_order.is_none(),
            "activation command ordering must be consumed through the activation output path"
        );
        core::mem::take(&mut self.outputs)
    }

    pub fn take_activation_outputs(&mut self) -> WidgetActivationContextOutputs<Action> {
        WidgetActivationContextOutputs {
            mounted: core::mem::take(&mut self.outputs),
            order: self.activation_order.take(),
        }
    }

    pub fn push_output(&mut self, output: MountedEffect<Action>) {
        self.push_mounted_output(output);
    }

    pub fn push_application_command(&mut self, command: ApplicationCommandId) {
        let order = self.activation_order.get_or_insert_with(|| {
            let mut order = Vec::with_capacity(self.outputs.len().saturating_add(1));
            order.extend(
                core::iter::repeat_with(|| WidgetActivationContextOutputOrder::Mounted)
                    .take(self.outputs.len()),
            );
            order
        });
        order.push(WidgetActivationContextOutputOrder::ApplicationCommand(
            command,
        ));
    }

    const fn len(&self) -> usize {
        match &self.activation_order {
            Some(order) => order.len(),
            None => self.outputs.len(),
        }
    }
}

macro_rules! work_context {
    ($name:ident, $phase:expr $(, $field:ident : $field_ty:ty = $field_default:expr)* $(,)?) => {
        pub struct $name<Action = ()> {
            invalidation: WidgetInvalidation,
            subscription_invalidation: bool,
            work: WidgetOutputCollector<Action>,
            remaining_outputs: Option<usize>,
            used_outputs: usize,
            overflowed: bool,
            producer_violation: Option<crate::ProducerContractViolation>,
            output_limit: Option<usize>,
            work_starts: Option<crate::WorkStartBudget>,
            work_used: [usize; 5],
            $($field: $field_ty,)*
        }

        impl<Action> core::fmt::Debug for $name<Action> {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .field("invalidation", &self.invalidation)
                    .field("subscription_invalidation", &self.subscription_invalidation)
                    .field("output_count", &self.work.len())
                    .finish()
            }
        }

        impl<Action> $name<Action> {
            pub fn invalidate(&mut self, invalidation: WidgetInvalidation) {
                self.invalidation |= invalidation;
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_new() -> Self {
                Self {
                    invalidation: WidgetInvalidation::NONE,
                    subscription_invalidation: false,
                    work: WidgetOutputCollector::new(),
                    remaining_outputs: None,
                    used_outputs: 0,
                    overflowed: false,
                    producer_violation: None,
                    output_limit: None,
                    work_starts: None,
                    work_used: [0; 5],
                    $($field: $field_default,)*
                }
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_new_bounded(output_allowance: usize) -> Self {
                Self {
                    invalidation: WidgetInvalidation::NONE,
                    subscription_invalidation: false,
                    work: WidgetOutputCollector::new(),
                    remaining_outputs: Some(output_allowance),
                    used_outputs: 0,
                    overflowed: false,
                    producer_violation: None,
                    output_limit: Some(output_allowance),
                    work_starts: None,
                    work_used: [0; 5],
                    $($field: $field_default,)*
                }
            }

            /// Returns the first scoped producer integrity violation, even if the
            /// author ignores the individual operation's fallible result.
            #[must_use]
            pub const fn producer_violation(&self) -> Option<crate::ProducerContractViolation> {
                self.producer_violation
            }

            #[doc(hidden)]
            pub const fn __runtime_try_reserve_output(
                &mut self,
            ) -> Result<(), crate::ProducerContractViolation> {
                if let Some(cause) = self.producer_violation {
                    return Err(cause);
                }
                let Some(remaining) = self.remaining_outputs else {
                    self.used_outputs += 1;
                    return Ok(());
                };
                if remaining == 0 {
                    let cause = crate::ProducerContractViolation::output(
                        crate::ProducerPhase::Widget($phase),
                        match self.output_limit { Some(limit) => limit, None => 0 },
                    );
                    self.producer_violation = Some(cause);
                    self.overflowed = true;
                    return Err(cause);
                }
                self.remaining_outputs = Some(remaining - 1);
                self.used_outputs += 1;
                Ok(())
            }

            #[doc(hidden)]
            pub const fn __runtime_set_work_starts(&mut self, budget: crate::WorkStartBudget) {
                self.work_starts = Some(budget);
            }

            fn try_reserve_work(&mut self, family: WorkFamily) -> Result<(), crate::ProducerContractViolation> {
                if let Some(cause) = self.producer_violation { return Err(cause); }
                let index = crate::producer_violation::family_index(family);
                if let Some(budget) = self.work_starts {
                    let declared = budget.for_family(family);
                    if self.work_used[index] >= declared {
                        let cause = crate::ProducerContractViolation::work_start(
                            crate::ProducerPhase::Widget($phase),
                            family,
                            declared,
                            self.work_used[index],
                        );
                        self.producer_violation = Some(cause);
                        self.overflowed = true;
                        return Err(cause);
                    }
                }
                self.__runtime_try_reserve_output()?;
                self.work_used[index] += 1;
                Ok(())
            }

            // Mapping lends the same invocation budget to its child. The parent
            // is exclusively borrowed until the consumed child is absorbed.
            pub(crate) const fn mapped_child<Child>(&self) -> $name<Child> {
                let mut child = $name::__runtime_new();
                child.remaining_outputs = self.remaining_outputs;
                child.output_limit = self.output_limit;
                child.used_outputs = self.used_outputs;
                child.work_starts = self.work_starts;
                child.work_used = self.work_used;
                child.producer_violation = self.producer_violation;
                child.overflowed = self.overflowed;
                child.subscription_invalidation = self.subscription_invalidation;
                child
            }

            pub(crate) fn absorb_budget<Child>(&mut self, child: &$name<Child>) -> bool {
                self.remaining_outputs = child.remaining_outputs;
                self.used_outputs = child.used_outputs;
                self.work_used = child.work_used;
                self.producer_violation = self.producer_violation.or(child.producer_violation);
                self.overflowed |= child.overflowed;
                self.subscription_invalidation |= child.subscription_invalidation;
                self.producer_violation.is_none()
            }

            /// Provisional owned action emission; the exact attempted action is
            /// returned before erasure when the declared allowance is exceeded.
            ///
            /// # Errors
            /// Returns the original value and a sticky producer violation.
            pub fn try_emit(&mut self, action: Action) -> Result<(), crate::EmitError<Action>> {
                if let Err(cause) = self.__runtime_try_reserve_output() {
                    return Err(crate::EmitError::new(action, cause));
                }
                self.work.emit(action);
                Ok(())
            }

            /// # Errors
            /// Returns the original concrete future before erasure.
            pub fn try_local_task<F: Future<Output = Option<Action>> + 'static>(
                &mut self, future: F,
            ) -> Result<(), crate::EmitError<F>> {
                if let Err(cause) = self.try_reserve_work(WorkFamily::LocalTask) {
                    return Err(crate::EmitError::new(future, cause));
                }
                self.work.local_task(future);
                Ok(())
            }

            /// # Errors
            /// Returns the original key/future tuple before erasure.
            pub fn try_keyed_local_task<F: Future<Output = Option<Action>> + 'static>(
                &mut self, key: WorkKey, future: F,
            ) -> Result<(), crate::EmitError<(WorkKey, F)>> {
                if let Err(cause) = self.try_reserve_work(WorkFamily::LocalTask) {
                    return Err(crate::EmitError::new((key, future), cause));
                }
                self.work.keyed_local_task(key, future);
                Ok(())
            }

            /// # Errors
            /// Returns the original concrete future and mapper before erasure.
            pub fn try_send_task<I, F, M>(
                &mut self, future: F, map: M,
            ) -> Result<(), crate::EmitError<(F, M)>>
            where
                I: Send + 'static,
                F: Future<Output = I> + Send + 'static,
                M: FnOnce(I) -> Action + 'static,
            {
                if let Err(cause) = self.try_reserve_work(WorkFamily::SendTask) {
                    return Err(crate::EmitError::new((future, map), cause));
                }
                self.work.send_task(future, map);
                Ok(())
            }

            /// # Errors
            /// Returns the original key, future and mapper before erasure.
            pub fn try_keyed_send_task<I, F, M>(
                &mut self, key: WorkKey, future: F, map: M,
            ) -> Result<(), crate::EmitError<(WorkKey, F, M)>>
            where
                I: Send + 'static,
                F: Future<Output = I> + Send + 'static,
                M: FnOnce(I) -> Action + 'static,
            {
                if let Err(cause) = self.try_reserve_work(WorkFamily::SendTask) {
                    return Err(crate::EmitError::new((key, future, map), cause));
                }
                self.work.keyed_send_task(key, future, map);
                Ok(())
            }

            /// # Errors
            /// Returns the original future, mapper and failure mapper.
            pub fn try_send_task_with_failure<I, F, M, E>(
                &mut self, future: F, map: M, start_failure: E,
            ) -> Result<(), crate::EmitError<(F, M, E)>>
            where
                I: Send + 'static,
                F: Future<Output = I> + Send + 'static,
                M: FnOnce(I) -> Action + 'static,
                E: FnOnce(SendTaskStartFailure) -> Action + 'static,
            {
                if let Err(cause) = self.try_reserve_work(WorkFamily::SendTask) {
                    return Err(crate::EmitError::new((future, map, start_failure), cause));
                }
                self.work.send_task_with_failure(future, map, start_failure);
                Ok(())
            }

            /// # Errors
            /// Returns the original timer descriptor before erasure.
            pub fn try_timer(
                &mut self, timer: TimerEffect<Action>,
            ) -> Result<(), crate::EmitError<TimerEffect<Action>>> {
                if let Err(cause) = self.try_reserve_work(WorkFamily::Timer) {
                    return Err(crate::EmitError::new(timer, cause));
                }
                self.work.timer(timer);
                Ok(())
            }

            /// # Errors
            /// Returns the original cancellation family/key on allowance failure.
            pub fn try_cancel(
                &mut self, family: WorkFamily, key: WorkKey,
            ) -> Result<(), crate::EmitError<(WorkFamily, WorkKey)>> {
                if let Err(cause) = self.__runtime_try_reserve_output() {
                    return Err(crate::EmitError::new((family, key), cause));
                }
                self.work.cancel(family, key);
                Ok(())
            }

            /// # Errors
            /// Returns an integrity violation on insufficient declaration credits.
            pub const fn try_invalidate_subscriptions(
                &mut self,
            ) -> Result<(), crate::ProducerContractViolation> {
                if let Some(cause) = self.producer_violation { return Err(cause); }
                if self.subscription_invalidation { return Ok(()); }
                match self.__runtime_try_reserve_output() {
                    Ok(()) => {
                        self.subscription_invalidation = true;
                        Ok(())
                    }
                    Err(cause) => Err(cause),
                }
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_remaining_outputs(&self) -> Option<usize> {
                self.remaining_outputs
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_used_outputs(&self) -> usize {
                self.used_outputs
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_overflowed(&self) -> bool {
                self.overflowed
            }

            #[doc(hidden)]
            #[must_use]
            pub fn __runtime_take_invalidation(&mut self) -> WidgetInvalidation {
                core::mem::take(&mut self.invalidation)
            }

            #[doc(hidden)]
            pub const fn __runtime_take_subscription_invalidation(&mut self) -> bool {
                let invalidated = self.subscription_invalidation;
                self.subscription_invalidation = false;
                invalidated
            }

            #[doc(hidden)]
            #[must_use]
            pub fn __runtime_take_outputs(&mut self) -> Vec<MountedEffect<Action>> {
                self.work.take_outputs()
            }

            #[doc(hidden)]
            pub(super) fn __runtime_push_output(&mut self, output: MountedEffect<Action>) {
                self.work.push_output(output);
            }
        }
    };
}

work_context!(WidgetMountContext, crate::WidgetProducerPhase::Mount);
work_context!(WidgetUpdateContext, crate::WidgetProducerPhase::Update);
work_context!(
    WidgetActivationContext, crate::WidgetProducerPhase::Activation,
    semantic_target: Option<SemanticActionTarget> = None,
);

impl<Action> WidgetActivationContext<Action> {
    pub(crate) fn set_semantic_target(&mut self, target: Option<SemanticActionTarget>) {
        self.semantic_target = target;
    }

    /// # Errors
    /// Returns the original application command on declared allowance overflow.
    pub fn try_emit_application_command(
        &mut self,
        command: ApplicationCommandId,
    ) -> Result<(), crate::EmitError<ApplicationCommandId>> {
        if let Err(cause) = self.__runtime_try_reserve_output() {
            return Err(crate::EmitError::new(command, cause));
        }
        self.work.push_application_command(command);
        Ok(())
    }

    /// Borrows the exact semantic target when this activation originated from
    /// an admitted semantic-node action request.
    ///
    /// Ordinary activation has no semantic target metadata.
    #[must_use]
    pub const fn semantic_action_target(&self) -> Option<&SemanticActionTarget> {
        self.semantic_target.as_ref()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_new_with_semantic_target(
        semantic_target: Option<SemanticActionTarget>,
    ) -> Self {
        let mut context = Self::__runtime_new();
        context.semantic_target = semantic_target;
        context
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_new_bounded_with_semantic_target(
        output_allowance: usize,
        semantic_target: Option<SemanticActionTarget>,
    ) -> Self {
        let mut context = Self::__runtime_new_bounded(output_allowance);
        context.semantic_target = semantic_target;
        context
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_take_activation_outputs(&mut self) -> WidgetActivationContextOutputs<Action> {
        self.work.take_activation_outputs()
    }

    #[doc(hidden)]
    pub(super) fn __runtime_push_application_command(&mut self, command: ApplicationCommandId) {
        self.work.push_application_command(command);
    }
}

/// Why a mounted widget lifetime is ending.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WidgetUnmountReason {
    Removed,
    Replaced,
    RuntimeShutdown,
}

/// Read-only context supplied to a widget unmount hook.
#[derive(Debug)]
pub struct WidgetUnmountContext {
    reason: WidgetUnmountReason,
}

impl WidgetUnmountContext {
    #[must_use]
    pub const fn reason(&self) -> WidgetUnmountReason {
        self.reason
    }
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(reason: WidgetUnmountReason) -> Self {
        Self { reason }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        __runtime::{MountedEffect, RuntimeNamespace},
        ApplicationCommandId, SemanticAction, SemanticActionTarget, SemanticKey,
    };

    use super::{
        WidgetActivationContext, WidgetActivationContextOutputOrder, WidgetInvalidation,
        WidgetMountContext, WidgetUpdateContext,
    };

    #[test]
    fn invalidation_union_and_containment_are_exact() {
        let value = WidgetInvalidation::LAYOUT | WidgetInvalidation::HIT_TEST;
        assert!(value.contains(WidgetInvalidation::LAYOUT));
        assert!(value.contains(WidgetInvalidation::HIT_TEST));
        assert!(!value.contains(WidgetInvalidation::PAINT));
        assert!(!value.contains(WidgetInvalidation::SEMANTICS));
        assert!(WidgetInvalidation::NONE.is_empty());
        assert!(WidgetInvalidation::ALL.contains(value));
    }

    #[test]
    fn activation_context_commands_share_the_bounded_output_allowance() {
        let command = ApplicationCommandId::from_static("document.save")
            .unwrap_or_else(|_| unreachable!("static command id is valid"));
        let mut context = WidgetActivationContext::__runtime_new_bounded(2);

        let _ = context.try_emit(7_u8);
        let _ = context.try_emit_application_command(command.clone());
        let _ = context.try_emit(9_u8);

        assert_eq!(context.__runtime_remaining_outputs(), Some(0));
        assert!(context.__runtime_overflowed());
        let outputs = context.__runtime_take_activation_outputs();
        let (mounted, order) = outputs.__runtime_into_parts();
        assert_eq!(mounted.len(), 1);
        assert!(matches!(&mounted[0], MountedEffect::Action(7)));
        let order = order.unwrap_or_else(|| unreachable!("command emission records ordering"));
        assert_eq!(order.len(), 2);
        assert!(matches!(
            order[0],
            WidgetActivationContextOutputOrder::Mounted
        ));
        assert!(matches!(
            &order[1],
            WidgetActivationContextOutputOrder::ApplicationCommand(observed)
                if observed == &command
        ));
    }

    #[test]
    fn activation_context_exposes_only_runtime_supplied_semantic_target() {
        let ordinary = WidgetActivationContext::<()>::__runtime_new();
        assert!(ordinary.semantic_action_target().is_none());

        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let node = namespace.__runtime_semantic_id(4, 2);
        let key = SemanticKey::from_static("virtual")
            .unwrap_or_else(|_| unreachable!("test key is valid"));
        let target = SemanticActionTarget::__runtime_new(
            surface.clone(),
            node.clone(),
            key.clone(),
            SemanticAction::Activate,
            None,
        );
        let context =
            WidgetActivationContext::<()>::__runtime_new_with_semantic_target(Some(target));
        let observed = context
            .semantic_action_target()
            .unwrap_or_else(|| unreachable!("semantic target was supplied"));
        assert_eq!(observed.surface_id(), &surface);
        assert_eq!(observed.target(), &node);
        assert_eq!(observed.semantic_key(), &key);
        assert_eq!(observed.action(), &SemanticAction::Activate);
    }

    #[test]
    fn bounded_mount_fallible_emission_returns_identical_nonclone_action_and_latches() {
        struct NonClone {
            marker: std::rc::Rc<()>,
        }
        let marker = std::rc::Rc::new(());
        let mut context = WidgetMountContext::<NonClone>::__runtime_new_bounded(1);
        context
            .try_emit(NonClone {
                marker: marker.clone(),
            })
            .unwrap_or_else(|_| unreachable!("first output is within limit"));
        let rejected = NonClone {
            marker: marker.clone(),
        };
        let (recovered, cause) = context
            .try_emit(rejected)
            .err()
            .unwrap_or_else(|| unreachable!("second output exceeds limit"))
            .into_parts();
        assert!(std::rc::Rc::ptr_eq(&recovered.marker, &marker));
        assert_eq!(
            cause.phase,
            crate::ProducerPhase::Widget(crate::WidgetProducerPhase::Mount)
        );
        assert_eq!(cause.declared, 1);
        assert_eq!(cause.attempted, Some(2));
        assert_eq!(context.producer_violation(), Some(cause));
        assert!(context.__runtime_overflowed());
        // Handling the error cannot clear terminal producer integrity.
        assert_eq!(context.try_invalidate_subscriptions(), Err(cause));
        assert_eq!(context.__runtime_take_outputs().len(), 1);
    }

    #[test]
    fn ignored_fallible_update_result_does_not_clear_first_failure() {
        let mut context = WidgetUpdateContext::<usize>::__runtime_new_bounded(0);
        drop(context.try_emit(3));
        let cause = context
            .producer_violation()
            .unwrap_or_else(|| unreachable!("zero allowance must latch"));
        let (recovered, second_cause) = context
            .try_emit(4)
            .err()
            .unwrap_or_else(|| unreachable!("subsequent output must fail"))
            .into_parts();
        assert_eq!(recovered, 4);
        assert_eq!(second_cause, cause);
        assert_eq!(
            cause.phase,
            crate::ProducerPhase::Widget(crate::WidgetProducerPhase::Update)
        );
        assert!(context.__runtime_take_outputs().is_empty());
    }

    #[test]
    fn activation_fallible_command_and_primary_output_share_one_credit_pool() {
        let mut context = WidgetActivationContext::<u8>::__runtime_new_bounded(1);
        let command = ApplicationCommandId::from_static("document.save")
            .unwrap_or_else(|_| unreachable!("static id valid"));
        context
            .try_emit(7)
            .unwrap_or_else(|_| unreachable!("in allowance"));
        let (recovered, cause) = context
            .try_emit_application_command(command.clone())
            .err()
            .unwrap_or_else(|| unreachable!("command exceeds allowance"))
            .into_parts();
        assert_eq!(recovered, command);
        assert_eq!(
            cause.phase,
            crate::ProducerPhase::Widget(crate::WidgetProducerPhase::Activation)
        );
        assert!(
            context.__runtime_try_reserve_output().is_err(),
            "primary activation output cannot bypass sticky failure"
        );
        assert_eq!(context.producer_violation(), Some(cause));
        let (mounted, order) = context
            .__runtime_take_activation_outputs()
            .__runtime_into_parts();
        assert_eq!(mounted.len(), 1);
        assert!(matches!(&mounted[0], MountedEffect::Action(7)));
        assert!(order.is_none());
    }
    #[test]
    fn every_lifecycle_family_reserves_output_and_start_credits_atomically() {
        macro_rules! check {
            ($context:ident,$phase:ident) => {
                for family in [
                    crate::WorkFamily::LocalTask,
                    crate::WorkFamily::SendTask,
                    crate::WorkFamily::Timer,
                ] {
                    let mut context = $context::<u8>::__runtime_new_bounded(1);
                    context.__runtime_set_work_starts(crate::WorkStartBudget::default());
                    let cause = match family {
                        crate::WorkFamily::LocalTask => context
                            .try_local_task(std::future::pending())
                            .err()
                            .unwrap_or_else(|| unreachable!())
                            .violation(),
                        crate::WorkFamily::SendTask => context
                            .try_send_task(std::future::pending::<u8>(), |item| item)
                            .err()
                            .unwrap_or_else(|| unreachable!())
                            .violation(),
                        crate::WorkFamily::Timer => context
                            .try_timer(crate::TimerEffect::once(std::time::Duration::ZERO, || 7))
                            .err()
                            .unwrap_or_else(|| unreachable!())
                            .violation(),
                        _ => unreachable!(),
                    };
                    assert_eq!(
                        cause.phase,
                        crate::ProducerPhase::Widget(crate::WidgetProducerPhase::$phase)
                    );
                    assert_eq!(cause.component, crate::BudgetComponent::WorkStarts(family));
                    assert_eq!(context.__runtime_remaining_outputs(), Some(1));
                    assert_eq!(context.__runtime_used_outputs(), 0);
                    assert_eq!(context.work_used, [0; 5]);
                    assert_eq!(
                        context
                            .try_emit(9)
                            .err()
                            .unwrap_or_else(|| unreachable!())
                            .violation(),
                        cause
                    );
                }
                let mut context = $context::<u8>::__runtime_new_bounded(0);
                context.__runtime_set_work_starts(crate::WorkStartBudget::new(1, 0, 0, 0, 0));
                assert_eq!(
                    context
                        .try_local_task(std::future::pending())
                        .err()
                        .unwrap_or_else(|| unreachable!())
                        .violation()
                        .component,
                    crate::BudgetComponent::Outputs
                );
                assert_eq!(context.work_used, [0; 5]);
            };
        }
        check!(WidgetMountContext, Mount);
        check!(WidgetUpdateContext, Update);
        check!(WidgetActivationContext, Activation);
    }
}
