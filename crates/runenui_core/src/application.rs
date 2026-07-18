//! Host-neutral application and host-request protocol.

use crate::{IntoEffects, SubscriptionSet, View};

/// The sole application contract consumed by the `RunenUI` runtime.
pub trait UiApp {
    type State;
    type Action;
    type HostProtocol: HostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action>;

    fn initial_effects(_state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        {}
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol>;

    fn subscriptions(_state: &Self::State, _subscriptions: &mut SubscriptionSet<Self::Action>) {}
}

/// Closed application-defined command and response protocol used by host effects.
pub trait HostProtocol {
    type Command;
    type Response: 'static;
    type ResponseKind: Copy + Eq + 'static;

    fn expected_response(command: &Self::Command) -> Self::ResponseKind;
    fn response_kind(response: &Self::Response) -> Self::ResponseKind;
}

/// Explicit protocol for applications which issue no host requests.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NoHostProtocol;

/// Uninhabited command type for [`NoHostProtocol`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NoHostCommand {}

/// Uninhabited response type for [`NoHostProtocol`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NoHostResponse {}

/// Uninhabited response-kind type for [`NoHostProtocol`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NoHostResponseKind {}

impl HostProtocol for NoHostProtocol {
    type Command = NoHostCommand;
    type Response = NoHostResponse;
    type ResponseKind = NoHostResponseKind;

    fn expected_response(command: &Self::Command) -> Self::ResponseKind {
        let _ = command;
        unreachable!("NoHostCommand is uninhabited")
    }

    fn response_kind(response: &Self::Response) -> Self::ResponseKind {
        let _ = response;
        unreachable!("NoHostResponse is uninhabited")
    }
}
