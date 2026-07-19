//! Host-neutral routed semantic-command protocol.

/// Phase of one mounted route invocation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// Normalized source of a semantic command.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventSource {
    Programmatic,
    Automation,
    Accessibility,
    Controller,
}

/// Whether a command entered directly or was delegated by a routed widget.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandDerivation {
    Direct,
    Delegated,
}

/// One internally consistent semantic-command origin.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CommandOrigin {
    source: EventSource,
    derivation: CommandDerivation,
}

impl CommandOrigin {
    #[must_use]
    pub const fn programmatic() -> Self {
        Self::direct(EventSource::Programmatic)
    }

    #[must_use]
    pub const fn automation() -> Self {
        Self::direct(EventSource::Automation)
    }

    #[must_use]
    pub const fn accessibility() -> Self {
        Self::direct(EventSource::Accessibility)
    }

    #[must_use]
    pub const fn controller() -> Self {
        Self::direct(EventSource::Controller)
    }

    const fn direct(source: EventSource) -> Self {
        Self {
            source,
            derivation: CommandDerivation::Direct,
        }
    }

    #[must_use]
    pub(crate) const fn delegated(source: EventSource) -> Self {
        Self {
            source,
            derivation: CommandDerivation::Delegated,
        }
    }

    #[must_use]
    pub const fn source(self) -> EventSource {
        self.source
    }

    #[must_use]
    pub const fn derivation(self) -> CommandDerivation {
        self.derivation
    }
}

/// Device-independent semantic command implemented by M4C1.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticCommand {
    Activate,
    CancelOrBack,
    OpenMenu,
    OpenContextMenu,
}

/// Immutable event delivered to one mounted widget callback.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiEvent {
    SemanticCommand(SemanticCommandEvent),
}

impl UiEvent {
    /// Borrows the semantic-command payload when this is that event family.
    ///
    /// Callers must retain the `Option` branch because later milestones add
    /// other routed event families to this non-exhaustive enum.
    #[must_use]
    pub const fn as_semantic_command(&self) -> Option<&SemanticCommandEvent> {
        match self {
            Self::SemanticCommand(command) => Some(command),
        }
    }
}

/// Immutable semantic-command event payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCommandEvent {
    command: SemanticCommand,
    origin: CommandOrigin,
}

impl SemanticCommandEvent {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(command: SemanticCommand, origin: CommandOrigin) -> Self {
        Self { command, origin }
    }

    #[must_use]
    pub const fn command(&self) -> SemanticCommand {
        self.command
    }

    #[must_use]
    pub const fn origin(&self) -> CommandOrigin {
        self.origin
    }
}

/// Explicit persistent-state result of one widget event callback.
#[must_use]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WidgetEventOutput {
    state_changed: bool,
}

impl WidgetEventOutput {
    pub const fn none() -> Self {
        Self {
            state_changed: false,
        }
    }

    pub const fn changed() -> Self {
        Self {
            state_changed: true,
        }
    }

    #[must_use]
    pub const fn state_changed(&self) -> bool {
        self.state_changed
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandDerivation, CommandOrigin, EventSource, SemanticCommand, SemanticCommandEvent,
        UiEvent, WidgetEventOutput,
    };

    #[test]
    fn command_origin_and_payload_are_internally_consistent() {
        let direct = CommandOrigin::automation();
        assert_eq!(direct.source(), EventSource::Automation);
        assert_eq!(direct.derivation(), CommandDerivation::Direct);
        let delegated = CommandOrigin::delegated(direct.source());
        assert_eq!(delegated.source(), direct.source());
        assert_eq!(delegated.derivation(), CommandDerivation::Delegated);
        let event = UiEvent::SemanticCommand(SemanticCommandEvent::__runtime_new(
            SemanticCommand::Activate,
            direct,
        ));
        let command = event
            .as_semantic_command()
            .unwrap_or_else(|| unreachable!("the test event is a semantic command"));
        assert_eq!(command.command(), SemanticCommand::Activate);
        assert_eq!(command.origin(), direct);
        let UiEvent::SemanticCommand(matched) = &event;
        assert_eq!(matched, command);
    }

    #[test]
    fn widget_event_output_reports_only_explicit_state_change() {
        assert!(!WidgetEventOutput::none().state_changed());
        assert!(!WidgetEventOutput::default().state_changed());
        assert!(WidgetEventOutput::changed().state_changed());
    }
}
