//! App-bound runtime wrapper and activation.

use core::marker::PhantomData;

use runenui_core::{Element, ElementKind};

use crate::{
    FocusState, Runtime, RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex, Trace, TraceTarget,
};

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

/// Result of semantic headless activation by authored element ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationResult {
    /// A matching button action was dispatched.
    Dispatched,
    /// No element with the requested authored or runtime ID exists in the current tree.
    NotFound,
    /// The requested element exists, but the element is not activatable.
    NotActivatable,
    /// The requested element exists, but it is intentionally disabled.
    Disabled,
    /// The requested element exists on a button, but the button has no action.
    NoAction,
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

    /// Returns an indexed borrowed view of the current root element tree.
    #[must_use]
    pub fn index(&self) -> RuntimeTreeIndex<'_, App::Action> {
        RuntimeTreeIndex::new(self.root())
    }

    /// Sets focus to an existing runtime node in the current tree.
    ///
    /// Returns `false` when the node ID is not present in the current tree.
    pub fn set_focus(&mut self, id: RuntimeNodeId) -> bool {
        if self.index().node(id).is_some() {
            self.runtime.set_focus(id);
            true
        } else {
            false
        }
    }

    /// Clears the current focus target.
    pub const fn clear_focus(&mut self) {
        self.runtime.clear_focus();
    }

    /// Returns the current runtime focus state.
    #[must_use]
    pub const fn focus(&self) -> &FocusState {
        self.runtime.focus()
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

impl<App> AppRuntime<App>
where
    App: UiApp,
    App::Action: Clone,
{
    /// Activates the element with the matching authored ID in the current tree.
    ///
    /// This is a semantic headless activation path for tests, tools, and host
    /// automation. Renderer hit testing should eventually resolve to internal
    /// runtime node identity and call [`Self::activate_node`].
    pub fn activate(&mut self, id: impl AsRef<str>) -> ActivationResult {
        let node_id = {
            let index = self.index();
            index
                .node_by_authored_id(id.as_ref())
                .map(RuntimeNodeRef::id)
        };

        node_id.map_or(ActivationResult::NotFound, |node_id| {
            self.activate_node(node_id)
        })
    }

    /// Activates the element with the matching generated runtime node ID.
    ///
    /// This is the renderer-facing activation seam: future hit testing can
    /// resolve pointer/focus targets to [`RuntimeNodeId`] and call this method
    /// without requiring authored element IDs.
    pub fn activate_node(&mut self, id: RuntimeNodeId) -> ActivationResult {
        let lookup = {
            let index = self.index();
            index
                .node(id)
                .map_or(ActivationLookup::NotFound, |node| activation_lookup(node))
        };

        match lookup {
            ActivationLookup::Action { action, target } => {
                self.runtime
                    .dispatch_with_target(action, App::update, App::root, Some(target));
                ActivationResult::Dispatched
            }
            ActivationLookup::NotFound => ActivationResult::NotFound,
            ActivationLookup::NotActivatable => ActivationResult::NotActivatable,
            ActivationLookup::Disabled => ActivationResult::Disabled,
            ActivationLookup::NoAction => ActivationResult::NoAction,
        }
    }
}

enum ActivationLookup<Action> {
    Action { action: Action, target: TraceTarget },
    NotFound,
    NotActivatable,
    Disabled,
    NoAction,
}

fn activation_lookup<Action>(node: &RuntimeNodeRef<'_, Action>) -> ActivationLookup<Action>
where
    Action: Clone,
{
    match node.element().kind() {
        ElementKind::Button(button) => {
            if !button.enabled() {
                return ActivationLookup::Disabled;
            }

            button
                .on_press()
                .map_or(ActivationLookup::NoAction, |action| {
                    ActivationLookup::Action {
                        action: action.clone(),
                        target: node.trace_target(),
                    }
                })
        }
        ElementKind::Text(_) | ElementKind::Container(_) => ActivationLookup::NotActivatable,
    }
}
