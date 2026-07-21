//! Live typed host requests and opaque runtime-local request tokens.

use std::sync::Arc;

use runenui_core::{__runtime::HostRequestEffect, HostProtocol};

use super::WorkGeneration;

/// Opaque runtime-local token for one exact live host request generation.
#[derive(Clone)]
pub struct HostRequestToken {
    pub(crate) namespace: Arc<()>,
    pub(crate) generation: WorkGeneration,
}

impl core::fmt::Debug for HostRequestToken {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("HostRequestToken(..)")
    }
}

/// Read-only host integration view of one command exposed after its start.
pub struct HostRequestRef<'a, Protocol: HostProtocol> {
    pub(crate) token: HostRequestToken,
    pub(crate) command: &'a Protocol::Command,
}

impl<'a, Protocol: HostProtocol> HostRequestRef<'a, Protocol> {
    #[must_use]
    pub fn token(&self) -> HostRequestToken {
        self.token.clone()
    }

    #[must_use]
    pub const fn command(&self) -> &'a Protocol::Command {
        self.command
    }
}

pub(crate) struct LiveHostRequest<Action, Protocol: HostProtocol> {
    pub(crate) generation: WorkGeneration,
    pub(crate) command: Protocol::Command,
    pub(crate) expected: Protocol::ResponseKind,
    pub(crate) map: Box<dyn FnOnce(Protocol::Response) -> Action>,
}

impl<Action, Protocol: HostProtocol> LiveHostRequest<Action, Protocol> {
    pub(crate) fn new(
        generation: WorkGeneration,
        request: HostRequestEffect<Action, Protocol>,
    ) -> Self {
        let expected = Protocol::expected_response(&request.command);
        Self {
            generation,
            command: request.command,
            expected,
            map: request.map,
        }
    }
}
