//! Host-neutral semantic normalization for one explicitly selected controller profile.
//!
//! This adapter deliberately accepts logical controls and phases, not raw device
//! identifiers, axes, polling samples, dead zones, or repeat timers. A concrete
//! host remains responsible for translating its supported hardware into these
//! profile values.

use runenui_core::{CommandOrigin, SemanticCommand};

/// Explicit logical controller profile understood by this adapter.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ControllerInputProfile {
    /// Accept/back plus deterministic focus navigation and directional scrolling.
    Navigation,
}

/// Logical buttons supported by the initial controller profile.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ControllerButton {
    Accept,
    Back,
    FocusNext,
    FocusPrevious,
    FocusLeft,
    FocusRight,
    FocusUp,
    FocusDown,
    /// A logical control outside the profile selected by this adapter.
    Unsupported,
}

/// Lifecycle transition normalized by the selected host profile.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ControllerTransition {
    Pressed,
    Repeated,
    Released,
    Cancelled,
}

/// Rejection for an inconsistent controller-button lifetime.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ControllerIngressDiagnostic {
    DuplicatePress,
    RepeatWithoutPress,
    ReleaseWithoutPress,
    CancelWithoutPress,
    UnsupportedProfileControl,
}

/// One semantic command normalized from a controller button press or repeat.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NormalizedControllerCommand {
    profile: ControllerInputProfile,
    command: SemanticCommand,
    repeated: bool,
}

impl NormalizedControllerCommand {
    /// Returns the profile under which this command was normalized.
    #[must_use]
    pub const fn profile(self) -> ControllerInputProfile {
        self.profile
    }

    /// Returns the existing host-neutral semantic command.
    #[must_use]
    pub const fn command(self) -> SemanticCommand {
        self.command
    }

    /// Returns the explicit controller source for submission to the runtime.
    #[must_use]
    pub const fn origin(self) -> CommandOrigin {
        CommandOrigin::controller()
    }

    /// Returns whether this command came from a host/profile repeat transition.
    #[must_use]
    pub const fn is_repeat(self) -> bool {
        self.repeated
    }
}

/// Result of normalizing one profile-level control transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerInputOutcome {
    Submit(NormalizedControllerCommand),
    Released(ControllerButton),
    Cancelled(ControllerButton),
    Suppressed(ControllerIngressDiagnostic),
}

/// Tracks held logical controls to reject duplicate, orphan-repeat, and orphan-release input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerInputState {
    profile: ControllerInputProfile,
    held: Vec<ControllerButton>,
}

impl ControllerInputState {
    /// Creates state for one explicit supported host profile.
    #[must_use]
    pub const fn new(profile: ControllerInputProfile) -> Self {
        Self {
            profile,
            held: Vec::new(),
        }
    }

    /// Returns the selected host profile.
    #[must_use]
    pub const fn profile(&self) -> ControllerInputProfile {
        self.profile
    }

    /// Normalizes a selected profile control without learning cadence or hardware identity.
    pub fn transition(
        &mut self,
        button: ControllerButton,
        transition: ControllerTransition,
    ) -> ControllerInputOutcome {
        let Some(command) = command_for(button) else {
            return ControllerInputOutcome::Suppressed(
                ControllerIngressDiagnostic::UnsupportedProfileControl,
            );
        };
        let held = self.held.iter().position(|active| *active == button);
        match transition {
            ControllerTransition::Pressed => {
                if held.is_some() {
                    return ControllerInputOutcome::Suppressed(
                        ControllerIngressDiagnostic::DuplicatePress,
                    );
                }
                self.held.push(button);
                ControllerInputOutcome::Submit(NormalizedControllerCommand {
                    profile: self.profile,
                    command,
                    repeated: false,
                })
            }
            ControllerTransition::Repeated => {
                if held.is_none() {
                    return ControllerInputOutcome::Suppressed(
                        ControllerIngressDiagnostic::RepeatWithoutPress,
                    );
                }
                ControllerInputOutcome::Submit(NormalizedControllerCommand {
                    profile: self.profile,
                    command,
                    repeated: true,
                })
            }
            ControllerTransition::Released => {
                let Some(index) = held else {
                    return ControllerInputOutcome::Suppressed(
                        ControllerIngressDiagnostic::ReleaseWithoutPress,
                    );
                };
                self.held.remove(index);
                ControllerInputOutcome::Released(button)
            }
            ControllerTransition::Cancelled => {
                let Some(index) = held else {
                    return ControllerInputOutcome::Suppressed(
                        ControllerIngressDiagnostic::CancelWithoutPress,
                    );
                };
                self.held.remove(index);
                ControllerInputOutcome::Cancelled(button)
            }
        }
    }

    /// Cancels all held controls in deterministic press order.
    pub fn cancel_all(&mut self) -> Vec<ControllerButton> {
        core::mem::take(&mut self.held)
    }

    /// Returns the currently held logical controls.
    #[must_use]
    pub fn held(&self) -> &[ControllerButton] {
        &self.held
    }
}

impl Default for ControllerInputState {
    fn default() -> Self {
        Self::new(ControllerInputProfile::Navigation)
    }
}

const fn command_for(button: ControllerButton) -> Option<SemanticCommand> {
    Some(match button {
        ControllerButton::Accept => SemanticCommand::Activate,
        ControllerButton::Back => SemanticCommand::CancelOrBack,
        ControllerButton::FocusNext => SemanticCommand::FocusNext,
        ControllerButton::FocusPrevious => SemanticCommand::FocusPrevious,
        ControllerButton::FocusLeft => SemanticCommand::FocusLeft,
        ControllerButton::FocusRight => SemanticCommand::FocusRight,
        ControllerButton::FocusUp => SemanticCommand::FocusUp,
        ControllerButton::FocusDown => SemanticCommand::FocusDown,
        ControllerButton::Unsupported => return None,
    })
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use runenui_core::{
        CommandDerivation, EventSource, IntoUpdateOutput, NoHostProtocol, SemanticCommand, UiApp,
        View, button,
    };
    use runenui_runtime::{AppRuntime, InputModality, PumpBudget, RuntimeConfig, TraceConfig};

    use super::{
        ControllerButton, ControllerIngressDiagnostic, ControllerInputOutcome,
        ControllerInputProfile, ControllerInputState, ControllerTransition,
    };

    #[derive(Debug)]
    enum Action {
        Activated,
    }

    struct App;

    impl UiApp for App {
        type State = usize;
        type Action = Action;
        type HostProtocol = NoHostProtocol;

        fn root(_: &Self::State) -> impl View<Self::Action> {
            button("accept")
                .id("accept")
                .on_activate(|| Action::Activated)
        }

        fn update(
            state: &mut Self::State,
            action: Self::Action,
        ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
            match action {
                Action::Activated => *state += 1,
            }
        }
    }

    #[test]
    fn accepted_buttons_map_to_existing_controller_origin_commands() {
        let mut state = ControllerInputState::default();
        let ControllerInputOutcome::Submit(command) =
            state.transition(ControllerButton::Accept, ControllerTransition::Pressed)
        else {
            unreachable!("a first press produces one semantic command");
        };
        assert_eq!(command.command(), SemanticCommand::Activate);
        assert_eq!(command.profile(), ControllerInputProfile::Navigation);
        assert_eq!(state.profile(), ControllerInputProfile::Navigation);
        assert_eq!(command.origin().source(), EventSource::Controller);
        assert_eq!(command.origin().derivation(), CommandDerivation::Direct);
        assert!(!command.is_repeat());

        let ControllerInputOutcome::Submit(repeated) =
            state.transition(ControllerButton::Accept, ControllerTransition::Repeated)
        else {
            unreachable!("a held control may repeat");
        };
        assert_eq!(repeated.command(), SemanticCommand::Activate);
        assert!(repeated.is_repeat());
        assert_eq!(
            state.transition(ControllerButton::FocusDown, ControllerTransition::Pressed),
            ControllerInputOutcome::Submit(super::NormalizedControllerCommand {
                profile: ControllerInputProfile::Navigation,
                command: SemanticCommand::FocusDown,
                repeated: false,
            })
        );
    }

    #[test]
    fn invalid_lifetimes_suppress_and_cancellation_releases_without_a_command() {
        let mut state = ControllerInputState::default();
        assert_eq!(
            state.transition(ControllerButton::Back, ControllerTransition::Repeated),
            ControllerInputOutcome::Suppressed(ControllerIngressDiagnostic::RepeatWithoutPress)
        );
        assert!(matches!(
            state.transition(ControllerButton::Back, ControllerTransition::Pressed),
            ControllerInputOutcome::Submit(_)
        ));
        assert_eq!(
            state.transition(ControllerButton::Back, ControllerTransition::Pressed),
            ControllerInputOutcome::Suppressed(ControllerIngressDiagnostic::DuplicatePress)
        );
        assert_eq!(
            state.transition(ControllerButton::Back, ControllerTransition::Cancelled),
            ControllerInputOutcome::Cancelled(ControllerButton::Back)
        );
        assert_eq!(state.held(), &[]);
        assert_eq!(
            state.transition(ControllerButton::Back, ControllerTransition::Released),
            ControllerInputOutcome::Suppressed(ControllerIngressDiagnostic::ReleaseWithoutPress)
        );
        for button in [ControllerButton::Accept, ControllerButton::FocusNext] {
            let _ = state.transition(button, ControllerTransition::Pressed);
        }
        assert_eq!(
            state.cancel_all(),
            [ControllerButton::Accept, ControllerButton::FocusNext]
        );
        assert!(state.held().is_empty());
    }

    #[test]
    fn unsupported_profile_controls_are_diagnosed_without_entering_the_runtime_contract() {
        let mut state = ControllerInputState::default();
        assert_eq!(
            state.transition(ControllerButton::Unsupported, ControllerTransition::Pressed),
            ControllerInputOutcome::Suppressed(
                ControllerIngressDiagnostic::UnsupportedProfileControl
            )
        );
        assert!(state.held().is_empty());
    }

    #[test]
    fn normalized_controller_commands_use_canonical_runtime_source_and_modality() {
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(128));
        let mut runtime = AppRuntime::<App>::mount_with_config(0, config);
        let target_id = runenui_core::ElementId::new("accept")
            .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
        let target = runtime
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&target_id))
            .unwrap_or_else(|| unreachable!("controller target is mounted"))
            .id()
            .clone();
        let mut input = ControllerInputState::default();

        let ControllerInputOutcome::Submit(pressed) =
            input.transition(ControllerButton::Accept, ControllerTransition::Pressed)
        else {
            unreachable!("accepted profile press normalizes to a command");
        };
        assert!(!pressed.is_repeat());
        runtime
            .submit_command(target.clone(), pressed.command(), pressed.origin())
            .unwrap_or_else(|error| panic!("normalized command enters the runtime: {error:?}"));
        runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

        let ControllerInputOutcome::Submit(repeated) =
            input.transition(ControllerButton::Accept, ControllerTransition::Repeated)
        else {
            unreachable!("held profile input normalizes repeats");
        };
        assert!(repeated.is_repeat());
        runtime
            .submit_command(target, repeated.command(), repeated.origin())
            .unwrap_or_else(|error| panic!("normalized repeat enters the runtime: {error:?}"));
        runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

        assert_eq!(
            input.transition(ControllerButton::Accept, ControllerTransition::Cancelled),
            ControllerInputOutcome::Cancelled(ControllerButton::Accept)
        );
        assert_eq!(*runtime.state(), 2);
        assert_eq!(runtime.focus().modality(), Some(InputModality::Controller));
        assert_eq!(
            runtime
                .trace()
                .records()
                .filter(|record| {
                    matches!(
                        record.kind(),
                        runenui_runtime::TraceRecordKind::RoutedEventStarted
                    )
                })
                .map(runenui_runtime::TraceRecord::command_origin)
                .collect::<Vec<_>>(),
            [Some(runenui_core::CommandOrigin::controller()); 2]
        );
        assert!(
            runtime
                .trace()
                .export_jsonl()
                .contains("\"source\":\"controller\"")
        );
        assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
    }
}
