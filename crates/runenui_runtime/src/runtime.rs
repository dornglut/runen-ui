//! Headless runtime state machine.

use runenui_core::Element;

use crate::{FocusState, RuntimeEvent, RuntimeNodeId, Trace, TraceTarget};

/// Headless UI runtime state machine.
pub struct Runtime<State, Action> {
    state: State,
    root: Element<Action>,
    trace: Trace,
    focus: FocusState,
}

impl<State, Action> Runtime<State, Action> {
    /// Mounts an initial state and builds the first root element tree.
    #[must_use]
    pub fn mount(state: State, root: impl FnOnce(&State) -> Element<Action>) -> Self {
        let root = root(&state);
        let mut trace = Trace::new();
        trace.record(RuntimeEvent::Mounted);

        Self {
            state,
            root,
            trace,
            focus: FocusState::new(),
        }
    }

    /// Dispatches one typed action, runs update, and rebuilds the root tree.
    pub fn dispatch(
        &mut self,
        action: Action,
        update: impl FnOnce(&mut State, Action),
        root: impl FnOnce(&State) -> Element<Action>,
    ) {
        self.dispatch_with_target(action, update, root, None);
    }

    pub(crate) fn dispatch_with_target(
        &mut self,
        action: Action,
        update: impl FnOnce(&mut State, Action),
        root: impl FnOnce(&State) -> Element<Action>,
        target: Option<TraceTarget>,
    ) {
        self.trace
            .record_with_target(RuntimeEvent::ActionDispatched, target.clone());
        update(&mut self.state, action);
        self.trace
            .record_with_target(RuntimeEvent::StateUpdated, target.clone());
        self.root = root(&self.state);
        self.focus.clear();
        self.trace
            .record_with_target(RuntimeEvent::RootRebuilt, target);
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

    /// Returns the runtime focus state.
    #[must_use]
    pub const fn focus(&self) -> &FocusState {
        &self.focus
    }

    /// Sets focus to the provided runtime node ID without validating the ID.
    pub const fn set_focus(&mut self, id: RuntimeNodeId) {
        self.focus.set(id);
    }

    /// Clears the current focus target.
    pub const fn clear_focus(&mut self) {
        self.focus.clear();
    }

    /// Consumes the runtime and returns the final application state.
    #[must_use]
    pub fn into_state(self) -> State {
        self.state
    }
}
