//! Host-neutral application-command identity and enabled-state vocabulary.

use crate::ApplicationCommandId;

/// Terminal disposition reported by one contextual application-command scope.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ApplicationCommandDisposition {
    Resolved,
    Disabled,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ApplicationCommand {
    id: ApplicationCommandId,
    enabled: bool,
}
impl ApplicationCommand {
    #[must_use]
    pub const fn new(id: ApplicationCommandId, enabled: bool) -> Self {
        Self { id, enabled }
    }
    #[must_use]
    pub const fn id(&self) -> &ApplicationCommandId {
        &self.id
    }
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}
