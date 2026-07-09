//! Headless runtime for `RunenUI`.
//!
//! This crate owns typed action delivery, update calls, root rebuilding, and
//! trace recording. Input dispatch, layout, accessibility extraction, and
//! surface-frame publication remain future runtime slices.

#![forbid(unsafe_code)]

use runenui_core::Element;

pub mod prelude;

/// Coarse runtime trace events emitted by the headless loop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    /// The runtime was mounted with initial state and root UI.
    Mounted,
    /// A typed action was accepted for dispatch.
    ActionDispatched,
    /// The application update function returned.
    StateUpdated,
    /// The root UI was rebuilt from the latest state.
    RootRebuilt,
}

/// Ordered runtime trace log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    events: Vec<RuntimeEvent>,
}

impl Trace {
    /// Creates an empty trace log.
    #[must_use]
    pub const fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Appends one runtime event.
    pub fn record(&mut self, event: RuntimeEvent) {
        self.events.push(event);
    }

    /// Returns the recorded event sequence.
    #[must_use]
    pub const fn events(&self) -> &[RuntimeEvent] {
        self.events.as_slice()
    }
}

/// Headless UI runtime state machine.
pub struct Runtime<State, Action> {
    state: State,
    root: Element<Action>,
    trace: Trace,
}

impl<State, Action> Runtime<State, Action> {
    /// Mounts an initial state and builds the first root element tree.
    #[must_use]
    pub fn mount(state: State, root: impl FnOnce(&State) -> Element<Action>) -> Self {
        let root = root(&state);
        let mut trace = Trace::new();
        trace.record(RuntimeEvent::Mounted);

        Self { state, root, trace }
    }

    /// Dispatches one typed action, runs update, and rebuilds the root tree.
    pub fn dispatch(
        &mut self,
        action: Action,
        update: impl FnOnce(&mut State, Action),
        root: impl FnOnce(&State) -> Element<Action>,
    ) {
        self.trace.record(RuntimeEvent::ActionDispatched);
        update(&mut self.state, action);
        self.trace.record(RuntimeEvent::StateUpdated);
        self.root = root(&self.state);
        self.trace.record(RuntimeEvent::RootRebuilt);
    }

    /// Returns the current application state.
    #[must_use]
    pub const fn state(&self) -> &State {
        &self.state
    }

    /// Returns the current root element tree.
    #[must_use]
    pub const fn root(&self) -> &Element<Action> {
        &self.root
    }

    /// Returns the runtime trace.
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        &self.trace
    }

    /// Consumes the runtime and returns the final application state.
    #[must_use]
    pub fn into_state(self) -> State {
        self.state
    }
}
