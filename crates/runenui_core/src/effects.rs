//! Ordered, inert application effect descriptions.

#![allow(clippy::double_must_use)]

use core::future::Future;

use crate::{
    HostProtocol, TimerEffect, WorkFamily, WorkKey,
    work::{LocalTaskEffect, SendTaskEffect, SendTaskStartFailure},
};

/// An opaque ordered batch of application work descriptions.
#[must_use]
pub struct Effects<Action, Protocol: HostProtocol> {
    pub(crate) items: Vec<Effect<Action, Protocol>>,
}

impl<Action, Protocol: HostProtocol> Effects<Action, Protocol> {
    #[must_use]
    pub const fn none() -> Self {
        Self { items: Vec::new() }
    }

    #[must_use]
    pub fn action(action: Action) -> Self {
        Self {
            items: vec![Effect::Action(action)],
        }
    }

    #[must_use]
    pub fn local_task(future: impl Future<Output = Option<Action>> + 'static) -> Self {
        Self {
            items: vec![Effect::LocalTask(LocalTaskEffect {
                key: None,
                future: Box::pin(future),
            })],
        }
    }

    #[must_use]
    pub fn keyed_local_task(
        key: WorkKey,
        future: impl Future<Output = Option<Action>> + 'static,
    ) -> Self {
        Self {
            items: vec![Effect::LocalTask(LocalTaskEffect {
                key: Some(key),
                future: Box::pin(future),
            })],
        }
    }

    #[must_use]
    pub fn send_task<Output>(
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::keyed_send_task_inner(None, future, map, None)
    }

    #[must_use]
    pub fn keyed_send_task<Output>(
        key: WorkKey,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::keyed_send_task_inner(Some(key), future, map, None)
    }

    #[must_use]
    pub fn send_task_with_failure<Output>(
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
        start_failure: impl FnOnce(SendTaskStartFailure) -> Action + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::keyed_send_task_inner(None, future, map, Some(Box::new(start_failure)))
    }

    fn keyed_send_task_inner<Output>(
        key: Option<WorkKey>,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
        start_failure: Option<Box<dyn FnOnce(SendTaskStartFailure) -> Action>>,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self {
            items: vec![Effect::SendTask(SendTaskEffect {
                key,
                future: Box::pin(async move {
                    Box::new(future.await) as Box<dyn core::any::Any + Send>
                }),
                map: Box::new(move |output| {
                    output
                        .downcast::<Output>()
                        .map_or_else(|_| unreachable!(), |output| map(*output))
                }),
                start_failure,
            })],
        }
    }

    #[must_use]
    pub fn timer(timer: TimerEffect<Action>) -> Self {
        Self {
            items: vec![Effect::Timer(timer)],
        }
    }

    #[must_use]
    pub fn host_request(
        key: Option<WorkKey>,
        command: Protocol::Command,
        map: impl FnOnce(Protocol::Response) -> Action + 'static,
    ) -> Self {
        Self {
            items: vec![Effect::HostRequest(HostRequestEffect {
                key,
                command,
                map: Box::new(map),
            })],
        }
    }

    #[must_use]
    pub fn cancel(family: WorkFamily, key: WorkKey) -> Self {
        Self {
            items: vec![Effect::Cancel { family, key }],
        }
    }

    #[must_use]
    pub fn redraw() -> Self {
        Self {
            items: vec![Effect::Redraw],
        }
    }

    #[must_use]
    pub fn then(mut self, next: impl IntoEffects<Action, Protocol>) -> Self {
        self.items.extend(next.into_effects().items);
        self
    }
}

impl<Action, Protocol: HostProtocol> Default for Effects<Action, Protocol> {
    fn default() -> Self {
        Self::none()
    }
}

/// Conversion into one ordered effect batch.
pub trait IntoEffects<Action, Protocol: HostProtocol> {
    fn into_effects(self) -> Effects<Action, Protocol>;
}

impl<Action, Protocol: HostProtocol> IntoEffects<Action, Protocol> for () {
    fn into_effects(self) -> Effects<Action, Protocol> {
        Effects::none()
    }
}

impl<Action, Protocol: HostProtocol> IntoEffects<Action, Protocol> for Effects<Action, Protocol> {
    fn into_effects(self) -> Self {
        self
    }
}

#[doc(hidden)]
pub enum Effect<Action, Protocol: HostProtocol> {
    Action(Action),
    LocalTask(LocalTaskEffect<Action>),
    SendTask(SendTaskEffect<Action>),
    Timer(TimerEffect<Action>),
    HostRequest(HostRequestEffect<Action, Protocol>),
    Cancel { family: WorkFamily, key: WorkKey },
    Redraw,
}

/// Inert exact-owner output collected by mounted lifecycle and activation callbacks.
#[doc(hidden)]
pub enum MountedEffect<Action> {
    Action(Action),
    LocalTask(LocalTaskEffect<Action>),
    SendTask(SendTaskEffect<Action>),
    Timer(TimerEffect<Action>),
    Cancel { family: WorkFamily, key: WorkKey },
}

#[doc(hidden)]
pub struct HostRequestEffect<Action, Protocol: HostProtocol> {
    pub key: Option<WorkKey>,
    pub command: Protocol::Command,
    pub map: Box<dyn FnOnce(Protocol::Response) -> Action>,
}

#[doc(hidden)]
impl<Action, Protocol: HostProtocol> Effects<Action, Protocol> {
    #[must_use]
    pub fn __runtime_into_items(self) -> Vec<Effect<Action, Protocol>> {
        self.items
    }
}

#[cfg(test)]
mod tests {
    use crate::{Effects, NoHostProtocol};

    #[test]
    fn composition_preserves_non_clone_action_order() {
        struct Action(u8);
        let effects = Effects::<_, NoHostProtocol>::action(Action(1))
            .then(Effects::action(Action(2)))
            .__runtime_into_items();
        let values: Vec<_> = effects
            .into_iter()
            .map(|effect| match effect {
                super::Effect::Action(action) => action.0,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(values, [1, 2]);
    }
}
