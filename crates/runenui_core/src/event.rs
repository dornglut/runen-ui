//! Host-neutral routed event and semantic-command protocol.

use crate::{
    FocusDirection, FocusEvent, LogicalScrollCommand, PointerBoundaryEvent, PointerCaptureEvent,
    PointerEvent,
};

/// Phase of one mounted route invocation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// Normalized source of a routed event or semantic command.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EventSource {
    Programmatic,
    Automation,
    Accessibility,
    Controller,
    Pointer,
    Keyboard,
}

/// How one semantic command was derived.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandDerivation {
    Direct,
    Delegated,
    SemanticDefault,
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

    /// Creates a normalized keyboard origin without introducing raw key routing.
    #[must_use]
    pub const fn keyboard() -> Self {
        Self::direct(EventSource::Keyboard)
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

    /// Creates the direct origin used while routing canonical pointer ingress.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_pointer() -> Self {
        Self::direct(EventSource::Pointer)
    }

    /// Creates the origin used for a command emitted by pointer default policy.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_pointer_default() -> Self {
        Self {
            source: EventSource::Pointer,
            derivation: CommandDerivation::SemanticDefault,
        }
    }

    /// Creates the origin used for a command emitted by semantic default policy.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_semantic_default(source: EventSource) -> Self {
        Self {
            source,
            derivation: CommandDerivation::SemanticDefault,
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

/// Device-independent semantic command.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticCommand {
    Activate,
    CancelOrBack,
    OpenMenu,
    OpenContextMenu,
    LogicalScroll(LogicalScrollCommand),
    FocusNext,
    FocusPrevious,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    RequestFocus,
    RestoreFocus,
    LogicalFocusScroll(FocusDirection),
}

/// Immutable event delivered to one mounted widget callback.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum UiEvent {
    SemanticCommand(SemanticCommandEvent),
    Pointer(PointerEvent),
    PointerBoundary(PointerBoundaryEvent),
    PointerCapture(PointerCaptureEvent),
    Focus(FocusEvent),
}

impl UiEvent {
    /// Borrows the semantic-command payload when this is that event family.
    #[must_use]
    pub const fn as_semantic_command(&self) -> Option<&SemanticCommandEvent> {
        match self {
            Self::SemanticCommand(command) => Some(command),
            Self::Pointer(_)
            | Self::PointerBoundary(_)
            | Self::PointerCapture(_)
            | Self::Focus(_) => None,
        }
    }

    /// Borrows the ordinary pointer payload when this is that event family.
    #[must_use]
    pub const fn as_pointer(&self) -> Option<&PointerEvent> {
        match self {
            Self::Pointer(event) => Some(event),
            Self::SemanticCommand(_)
            | Self::PointerBoundary(_)
            | Self::PointerCapture(_)
            | Self::Focus(_) => None,
        }
    }

    /// Borrows the target-only pointer-boundary payload when present.
    #[must_use]
    pub const fn as_pointer_boundary(&self) -> Option<&PointerBoundaryEvent> {
        match self {
            Self::PointerBoundary(event) => Some(event),
            Self::SemanticCommand(_)
            | Self::Pointer(_)
            | Self::PointerCapture(_)
            | Self::Focus(_) => None,
        }
    }

    /// Borrows the target-only pointer-capture payload when present.
    #[must_use]
    pub const fn as_pointer_capture(&self) -> Option<&PointerCaptureEvent> {
        match self {
            Self::PointerCapture(event) => Some(event),
            Self::SemanticCommand(_)
            | Self::Pointer(_)
            | Self::PointerBoundary(_)
            | Self::Focus(_) => None,
        }
    }

    /// Borrows the routed focus notification payload when present.
    #[must_use]
    pub const fn as_focus(&self) -> Option<&FocusEvent> {
        match self {
            Self::Focus(event) => Some(event),
            Self::SemanticCommand(_)
            | Self::Pointer(_)
            | Self::PointerBoundary(_)
            | Self::PointerCapture(_) => None,
        }
    }
}

/// Immutable semantic-command event payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
    use crate::{
        __runtime::RuntimeNamespace, CommandDerivation, CommandOrigin, EventSource, LogicalDelta,
        LogicalPoint, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticCommand,
        SemanticCommandEvent, UiEvent, WidgetEventOutput,
    };

    #[test]
    fn command_origin_and_payload_are_internally_consistent() {
        let direct = CommandOrigin::automation();
        assert_eq!(direct.source(), EventSource::Automation);
        assert_eq!(direct.derivation(), CommandDerivation::Direct);
        let delegated = CommandOrigin::delegated(direct.source());
        assert_eq!(delegated.source(), direct.source());
        assert_eq!(delegated.derivation(), CommandDerivation::Delegated);
        let pointer_default = CommandOrigin::__runtime_pointer_default();
        assert_eq!(pointer_default.source(), EventSource::Pointer);
        assert_eq!(
            pointer_default.derivation(),
            CommandDerivation::SemanticDefault
        );
        let event = UiEvent::SemanticCommand(SemanticCommandEvent::__runtime_new(
            SemanticCommand::Activate,
            direct,
        ));
        let command = event
            .as_semantic_command()
            .unwrap_or_else(|| unreachable!("the test event is a semantic command"));
        assert_eq!(command.command(), SemanticCommand::Activate);
        assert_eq!(command.origin(), direct);
        assert!(event.as_pointer().is_none());
    }

    #[test]
    fn pointer_family_is_distinct_from_semantic_commands() {
        let namespace = RuntimeNamespace::__runtime_new();
        let context = namespace
            .__runtime_surface_context(namespace.__runtime_surface_id(0, 1), 1, 1)
            .unwrap_or_else(|| unreachable!("test surface shares its namespace"));
        let pointer = PointerEvent::new(
            PointerId::new(1).unwrap_or_else(|| unreachable!("test pointer is non-zero")),
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            LogicalPoint::new(2.0, 3.0).unwrap_or_else(|_| unreachable!("test point is finite")),
            context,
        )
        .with_movement_delta(
            LogicalDelta::new(1.0, 1.0).unwrap_or_else(|_| unreachable!("test delta is finite")),
        );
        let event = UiEvent::Pointer(pointer.clone());
        assert_eq!(event.as_pointer(), Some(&pointer));
        assert!(event.as_semantic_command().is_none());
        assert_eq!(pointer.pointer_id().get(), 1);
    }

    #[test]
    fn widget_event_output_reports_only_explicit_state_change() {
        assert!(!WidgetEventOutput::none().state_changed());
        assert!(!WidgetEventOutput::default().state_changed());
        assert!(WidgetEventOutput::changed().state_changed());
    }
}
