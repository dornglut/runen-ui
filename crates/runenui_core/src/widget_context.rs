//! Runtime-owned widget lifecycle contexts and invalidation vocabulary.

use core::{
    future::Future,
    ops::{BitOr, BitOrAssign},
};

use crate::{
    SendTaskStartFailure, TimerEffect, WorkFamily, WorkKey,
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
    pub const ALL: Self = Self(
        Self::INTERACTION.0
            | Self::LAYOUT.0
            | Self::PAINT.0
            | Self::SEMANTICS.0
            | Self::DIAGNOSTICS.0,
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

macro_rules! work_context {
    ($name:ident) => {
        pub struct $name<Action = ()> {
            invalidation: WidgetInvalidation,
            subscription_invalidation: bool,
            outputs: Vec<MountedEffect<Action>>,
        }

        impl<Action> core::fmt::Debug for $name<Action> {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .field("invalidation", &self.invalidation)
                    .field("subscription_invalidation", &self.subscription_invalidation)
                    .field("output_count", &self.outputs.len())
                    .finish()
            }
        }

        impl<Action> $name<Action> {
            pub fn invalidate(&mut self, invalidation: WidgetInvalidation) {
                self.invalidation |= invalidation;
            }

            /// Requests one complete owner-local subscription declaration pass.
            pub const fn invalidate_subscriptions(&mut self) {
                self.subscription_invalidation = true;
            }

            /// Emits one action owned by the exact mounted lifetime.
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
                    future: Box::pin(
                        async move { Box::new(future.await) as crate::work::SendOutput },
                    ),
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

            #[doc(hidden)]
            #[must_use]
            pub const fn __runtime_new() -> Self {
                Self {
                    invalidation: WidgetInvalidation::NONE,
                    subscription_invalidation: false,
                    outputs: Vec::new(),
                }
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
                core::mem::take(&mut self.outputs)
            }

            #[doc(hidden)]
            pub fn __runtime_push_output(&mut self, output: MountedEffect<Action>) {
                self.outputs.push(output);
            }
        }
    };
}

work_context!(WidgetMountContext);
work_context!(WidgetUpdateContext);
work_context!(WidgetActivationContext);

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
    use super::WidgetInvalidation;

    #[test]
    fn invalidation_union_and_containment_are_exact() {
        let value = WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT;
        assert!(value.contains(WidgetInvalidation::LAYOUT));
        assert!(value.contains(WidgetInvalidation::PAINT));
        assert!(!value.contains(WidgetInvalidation::SEMANTICS));
        assert!(WidgetInvalidation::NONE.is_empty());
        assert!(WidgetInvalidation::ALL.contains(value));
    }
}
