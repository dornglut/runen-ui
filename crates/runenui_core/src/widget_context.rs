//! Runtime-owned widget lifecycle contexts and invalidation vocabulary.

use core::{
    future::Future,
    ops::{BitOr, BitOrAssign},
};

use crate::{
    SemanticActionTarget, SendTaskStartFailure, TimerEffect, WorkFamily, WorkKey,
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

pub struct WidgetWorkCollector<Action> {
    outputs: Vec<MountedEffect<Action>>,
}

impl<Action> WidgetWorkCollector<Action> {
    pub const fn new() -> Self {
        Self {
            outputs: Vec::new(),
        }
    }

    pub fn emit(&mut self, action: Action) {
        self.outputs.push(MountedEffect::Action(action));
    }

    pub fn local_task(&mut self, future: impl Future<Output = Option<Action>> + 'static) {
        self.outputs.push(MountedEffect::LocalTask(LocalTaskEffect {
            key: None,
            future: Box::pin(future),
        }));
    }

    pub fn keyed_local_task(
        &mut self,
        key: WorkKey,
        future: impl Future<Output = Option<Action>> + 'static,
    ) {
        self.outputs.push(MountedEffect::LocalTask(LocalTaskEffect {
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
        self.outputs.push(MountedEffect::SendTask(SendTaskEffect {
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
        self.outputs.push(MountedEffect::Timer(timer));
    }

    pub fn cancel(&mut self, family: WorkFamily, key: WorkKey) {
        self.outputs.push(MountedEffect::Cancel { family, key });
    }

    pub fn take_outputs(&mut self) -> Vec<MountedEffect<Action>> {
        core::mem::take(&mut self.outputs)
    }

    pub fn push_output(&mut self, output: MountedEffect<Action>) {
        self.outputs.push(output);
    }

    const fn len(&self) -> usize {
        self.outputs.len()
    }
}

macro_rules! work_context {
    ($name:ident $(, $field:ident : $field_ty:ty = $field_default:expr)* $(,)?) => {
        pub struct $name<Action = ()> {
            invalidation: WidgetInvalidation,
            subscription_invalidation: bool,
            work: WidgetWorkCollector<Action>,
            remaining_outputs: Option<usize>,
            overflowed: bool,
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

            /// Requests one complete owner-local subscription declaration pass.
            pub const fn invalidate_subscriptions(&mut self) {
                if !self.subscription_invalidation && self.__runtime_reserve_output() {
                    self.subscription_invalidation = true;
                }
            }

            /// Emits one action owned by the exact mounted lifetime.
            pub fn emit(&mut self, action: Action) {
                if self.__runtime_reserve_output() {
                    self.work.emit(action);
                }
            }

            pub fn local_task(&mut self, future: impl Future<Output = Option<Action>> + 'static) {
                if self.__runtime_reserve_output() {
                    self.work.local_task(future);
                }
            }

            pub fn keyed_local_task(
                &mut self,
                key: WorkKey,
                future: impl Future<Output = Option<Action>> + 'static,
            ) {
                if self.__runtime_reserve_output() {
                    self.work.keyed_local_task(key, future);
                }
            }

            pub fn send_task<Output>(
                &mut self,
                future: impl Future<Output = Output> + Send + 'static,
                map: impl FnOnce(Output) -> Action + 'static,
            ) where
                Output: Send + 'static,
            {
                if self.__runtime_reserve_output() {
                    self.work.send_task(future, map);
                }
            }

            pub fn keyed_send_task<Output>(
                &mut self,
                key: WorkKey,
                future: impl Future<Output = Output> + Send + 'static,
                map: impl FnOnce(Output) -> Action + 'static,
            ) where
                Output: Send + 'static,
            {
                if self.__runtime_reserve_output() {
                    self.work.keyed_send_task(key, future, map);
                }
            }

            pub fn send_task_with_failure<Output>(
                &mut self,
                future: impl Future<Output = Output> + Send + 'static,
                map: impl FnOnce(Output) -> Action + 'static,
                start_failure: impl FnOnce(SendTaskStartFailure) -> Action + 'static,
            ) where
                Output: Send + 'static,
            {
                if self.__runtime_reserve_output() {
                    self.work.send_task_with_failure(future, map, start_failure);
                }
            }

            pub fn timer(&mut self, timer: TimerEffect<Action>) {
                if self.__runtime_reserve_output() {
                    self.work.timer(timer);
                }
            }

            pub fn cancel(&mut self, family: WorkFamily, key: WorkKey) {
                if self.__runtime_reserve_output() {
                    self.work.cancel(family, key);
                }
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_new() -> Self {
                Self {
                    invalidation: WidgetInvalidation::NONE,
                    subscription_invalidation: false,
                    work: WidgetWorkCollector::new(),
                    remaining_outputs: None,
                    overflowed: false,
                    $($field: $field_default,)*
                }
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_new_bounded(output_allowance: usize) -> Self {
                Self {
                    invalidation: WidgetInvalidation::NONE,
                    subscription_invalidation: false,
                    work: WidgetWorkCollector::new(),
                    remaining_outputs: Some(output_allowance),
                    overflowed: false,
                    $($field: $field_default,)*
                }
            }

            #[doc(hidden)]
            pub const fn __runtime_reserve_output(&mut self) -> bool {
                let Some(remaining) = self.remaining_outputs else {
                    return true;
                };
                if remaining == 0 {
                    self.overflowed = true;
                    false
                } else {
                    self.remaining_outputs = Some(remaining - 1);
                    true
                }
            }

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_remaining_outputs(&self) -> Option<usize> {
                self.remaining_outputs
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
            pub fn __runtime_push_output(&mut self, output: MountedEffect<Action>) {
                if self.__runtime_reserve_output() {
                    self.work.push_output(output);
                }
            }
        }
    };
}

work_context!(WidgetMountContext);
work_context!(WidgetUpdateContext);
work_context!(
    WidgetActivationContext,
    semantic_target: Option<SemanticActionTarget> = None,
);

impl<Action> WidgetActivationContext<Action> {
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
    use crate::{__runtime::RuntimeNamespace, SemanticAction, SemanticActionTarget, SemanticKey};

    use super::{WidgetActivationContext, WidgetInvalidation};

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
}
