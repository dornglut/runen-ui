//! Headless runtime for `RunenUI`.
//!
//! This crate owns typed action delivery, update calls, root rebuilding, and
//! trace recording. Input dispatch, layout, accessibility extraction, and
//! surface-frame publication remain future runtime slices.

#![forbid(unsafe_code)]

use core::marker::PhantomData;

use runenui_core::{Element, ElementId, ElementKind};

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

/// Result of semantic headless activation by authored element ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationResult {
    /// A matching button action was dispatched.
    Dispatched,
    /// No element with the requested authored or runtime ID exists in the current tree.
    NotFound,
    /// The requested element exists, but the element is not activatable.
    NotActivatable,
    /// The requested element exists on a button, but the button has no action.
    NoAction,
}

/// Generated runtime identity for an element in one built tree.
///
/// Runtime node IDs are assigned by pre-order traversal. They are stable for a
/// specific built tree and are intended for runtime internals such as hit-test,
/// focus, tracing, and renderer feedback. They are not authored public handles.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuntimeNodeId(usize);

impl RuntimeNodeId {
    /// Root node ID for a built runtime tree.
    pub const ROOT: Self = Self(0);

    /// Creates a runtime node ID from a traversal index.
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }

    /// Returns the traversal index backing this node ID.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Borrowed runtime node view into the current element tree.
pub struct RuntimeNodeRef<'a, Action> {
    id: RuntimeNodeId,
    parent: Option<RuntimeNodeId>,
    element: &'a Element<Action>,
}

impl<'a, Action> RuntimeNodeRef<'a, Action> {
    const fn new(
        id: RuntimeNodeId,
        parent: Option<RuntimeNodeId>,
        element: &'a Element<Action>,
    ) -> Self {
        Self {
            id,
            parent,
            element,
        }
    }

    /// Returns the generated runtime node ID.
    #[must_use]
    pub const fn id(&self) -> RuntimeNodeId {
        self.id
    }

    /// Returns the generated runtime parent ID, if this node is not the root.
    #[must_use]
    pub const fn parent(&self) -> Option<RuntimeNodeId> {
        self.parent
    }

    /// Returns the borrowed element for this runtime node.
    #[must_use]
    pub const fn element(&self) -> &'a Element<Action> {
        self.element
    }

    /// Returns the optional authored element ID.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&'a ElementId> {
        self.element.element_id()
    }
}

/// Indexed borrowed view over one built runtime tree.
pub struct RuntimeTreeIndex<'a, Action> {
    nodes: Vec<RuntimeNodeRef<'a, Action>>,
}

impl<'a, Action> RuntimeTreeIndex<'a, Action> {
    /// Builds an index for the provided root element tree.
    #[must_use]
    pub fn new(root: &'a Element<Action>) -> Self {
        let mut index = Self { nodes: Vec::new() };
        index.push_node(None, root);
        index
    }

    fn push_node(
        &mut self,
        parent: Option<RuntimeNodeId>,
        element: &'a Element<Action>,
    ) -> RuntimeNodeId {
        let id = RuntimeNodeId::from_index(self.nodes.len());
        self.nodes.push(RuntimeNodeRef::new(id, parent, element));

        if let ElementKind::Container(container) = element.kind() {
            for child in container.children() {
                self.push_node(Some(id), child);
            }
        }

        id
    }

    /// Returns all indexed runtime nodes in pre-order traversal order.
    #[must_use]
    pub const fn nodes(&self) -> &[RuntimeNodeRef<'a, Action>] {
        self.nodes.as_slice()
    }

    /// Returns the node with the generated runtime node ID.
    #[must_use]
    pub fn node(&self, id: RuntimeNodeId) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.nodes.get(id.as_usize())
    }

    /// Returns the first node with the matching authored element ID.
    #[must_use]
    pub fn node_by_authored_id(&self, id: impl AsRef<str>) -> Option<&RuntimeNodeRef<'a, Action>> {
        let id = id.as_ref();
        self.nodes.iter().find(
            |node| matches!(node.authored_id(), Some(element_id) if element_id.as_str() == id),
        )
    }
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
            index.node(id).map_or(ActivationLookup::NotFound, |node| {
                activation_lookup(node.element())
            })
        };

        match lookup {
            ActivationLookup::Action(action) => {
                self.dispatch(action);
                ActivationResult::Dispatched
            }
            ActivationLookup::NotFound => ActivationResult::NotFound,
            ActivationLookup::NotActivatable => ActivationResult::NotActivatable,
            ActivationLookup::NoAction => ActivationResult::NoAction,
        }
    }
}

enum ActivationLookup<Action> {
    Action(Action),
    NotFound,
    NotActivatable,
    NoAction,
}

fn activation_lookup<Action>(element: &Element<Action>) -> ActivationLookup<Action>
where
    Action: Clone,
{
    match element.kind() {
        ElementKind::Button(button) => button
            .on_press()
            .map_or(ActivationLookup::NoAction, |action| {
                ActivationLookup::Action(action.clone())
            }),
        ElementKind::Text(_) | ElementKind::Container(_) => ActivationLookup::NotActivatable,
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
