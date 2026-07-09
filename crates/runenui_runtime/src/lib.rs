//! Headless runtime for `RunenUI`.
//!
//! This crate owns typed action delivery, update calls, root rebuilding, and
//! trace recording. Input dispatch, layout, accessibility extraction, and
//! surface-frame publication remain future runtime slices.

#![forbid(unsafe_code)]

use core::marker::PhantomData;

use runenui_core::Element;

pub mod prelude;

/// Application contract used by [`AppRuntime`].
///
/// The app owns its state, action type, update logic, and root UI composition.
/// The runtime owns dispatch sequencing, update invocation, root rebuilds, and
/// trace recording.
pub trait UiApp {
    /// Application state.
    type State;

    /// Typed actions produced by UI controls or host input.
    type Action;

    /// Builds the current root UI tree from application state.
    fn root(state: &Self::State) -> Element<Self::Action>;

    /// Applies one typed action to application state.
    fn update(state: &mut Self::State, action: Self::Action);
}

/// Runtime wrapper that binds an app's root and update functions once.
pub struct AppRuntime<App>
where
    App: UiApp,
{
    runtime: Runtime<App::State, App::Action>,
    _app: PhantomData<fn() -> App>,
}

impl<App> AppRuntime<App>
where
    App: UiApp,
{
    /// Mounts an app with its initial state.
    #[must_use]
    pub fn mount(state: App::State) -> Self {
        Self {
            runtime: Runtime::mount(state, App::root),
            _app: PhantomData,
        }
    }

    /// Dispatches one typed action through the bound app update/root pair.
    pub fn dispatch(&mut self, action: App::Action) {
        self.runtime.dispatch(action, App::update, App::root);
    }

    /// Returns the current application state.
    #[must_use]
    pub const fn state(&self) -> &App::State {
        self.runtime.state()
    }

    /// Returns the current root element tree.
    #[must_use]
    pub const fn root(&self) -> &Element<App::Action> {
        self.runtime.root()
    }

    /// Returns the runtime trace.
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        self.runtime.trace()
    }

    /// Consumes this app runtime and returns the lower-level runtime.
    #[must_use]
    pub fn into_runtime(self) -> Runtime<App::State, App::Action> {
        self.runtime
    }

    /// Consumes this app runtime and returns the final application state.
    #[must_use]
    pub fn into_state(self) -> App::State {
        self.runtime.into_state()
    }
}

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
