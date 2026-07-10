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
    /// The requested element exists, but it is intentionally disabled.
    Disabled,
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

/// Logical position in UI coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalPoint {
    x: f32,
    y: f32,
}

impl LogicalPoint {
    /// Creates a logical point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal coordinate.
    #[must_use]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// Returns the vertical coordinate.
    #[must_use]
    pub const fn y(&self) -> f32 {
        self.y
    }
}

/// Keyboard modifier state carried by host input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyModifiers {
    bits: u8,
}

impl KeyModifiers {
    const SHIFT_BIT: u8 = 0b0001;
    const CONTROL_BIT: u8 = 0b0010;
    const ALT_BIT: u8 = 0b0100;
    const META_BIT: u8 = 0b1000;
    const ALL_BITS: u8 = Self::SHIFT_BIT | Self::CONTROL_BIT | Self::ALT_BIT | Self::META_BIT;

    /// Empty modifier state.
    pub const NONE: Self = Self { bits: 0 };
    /// Shift modifier state.
    pub const SHIFT: Self = Self {
        bits: Self::SHIFT_BIT,
    };
    /// Control modifier state.
    pub const CONTROL: Self = Self {
        bits: Self::CONTROL_BIT,
    };
    /// Alt modifier state.
    pub const ALT: Self = Self {
        bits: Self::ALT_BIT,
    };
    /// Meta, Command, or Windows modifier state.
    pub const META: Self = Self {
        bits: Self::META_BIT,
    };

    /// Creates a modifier state from raw modifier bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self {
            bits: bits & Self::ALL_BITS,
        }
    }

    /// Returns the raw modifier bits.
    #[must_use]
    pub const fn bits(&self) -> u8 {
        self.bits
    }

    /// Returns a modifier state with Shift added.
    #[must_use]
    pub const fn with_shift(self) -> Self {
        Self::from_bits(self.bits | Self::SHIFT_BIT)
    }

    /// Returns a modifier state with Control added.
    #[must_use]
    pub const fn with_control(self) -> Self {
        Self::from_bits(self.bits | Self::CONTROL_BIT)
    }

    /// Returns a modifier state with Alt added.
    #[must_use]
    pub const fn with_alt(self) -> Self {
        Self::from_bits(self.bits | Self::ALT_BIT)
    }

    /// Returns a modifier state with Meta, Command, or Windows added.
    #[must_use]
    pub const fn with_meta(self) -> Self {
        Self::from_bits(self.bits | Self::META_BIT)
    }

    /// Returns whether Shift is active.
    #[must_use]
    pub const fn shift(&self) -> bool {
        self.bits & Self::SHIFT_BIT != 0
    }

    /// Returns whether Control is active.
    #[must_use]
    pub const fn control(&self) -> bool {
        self.bits & Self::CONTROL_BIT != 0
    }

    /// Returns whether Alt is active.
    #[must_use]
    pub const fn alt(&self) -> bool {
        self.bits & Self::ALT_BIT != 0
    }

    /// Returns whether Meta, Command, or Windows is active.
    #[must_use]
    pub const fn meta(&self) -> bool {
        self.bits & Self::META_BIT != 0
    }
}

/// Pointer button reported by a host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    /// Primary activation button, usually left mouse or primary touch.
    Primary,
    /// Secondary button, usually right mouse.
    Secondary,
    /// Middle mouse button.
    Middle,
    /// Host-specific pointer button.
    Other(u16),
}

/// Pointer input phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerPhase {
    /// Pointer moved without a button state transition.
    Moved,
    /// Pointer button was pressed.
    Pressed,
    /// Pointer button was released.
    Released,
    /// Pointer stream was cancelled by the host.
    Cancelled,
}

/// Pointer input after optional host hit-test resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    phase: PointerPhase,
    position: LogicalPoint,
    button: Option<PointerButton>,
    modifiers: KeyModifiers,
    target: Option<RuntimeNodeId>,
}

impl PointerEvent {
    /// Creates a pointer event.
    #[must_use]
    pub const fn new(
        phase: PointerPhase,
        position: LogicalPoint,
        button: Option<PointerButton>,
        modifiers: KeyModifiers,
        target: Option<RuntimeNodeId>,
    ) -> Self {
        Self {
            phase,
            position,
            button,
            modifiers,
            target,
        }
    }

    /// Returns the pointer phase.
    #[must_use]
    pub const fn phase(&self) -> PointerPhase {
        self.phase
    }

    /// Returns the logical pointer position.
    #[must_use]
    pub const fn position(&self) -> LogicalPoint {
        self.position
    }

    /// Returns the pointer button for button transitions.
    #[must_use]
    pub const fn button(&self) -> Option<PointerButton> {
        self.button
    }

    /// Returns keyboard modifiers active during the pointer event.
    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// Returns the resolved runtime target, if the host already hit-tested it.
    #[must_use]
    pub const fn target(&self) -> Option<RuntimeNodeId> {
        self.target
    }
}

/// Keyboard key identity reported by a host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Key {
    /// Enter or Return.
    Enter,
    /// Space.
    Space,
    /// Tab.
    Tab,
    /// Escape.
    Escape,
    /// Text-producing character.
    Character(char),
    /// Host-specific named key.
    Named(String),
}

/// Keyboard input phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyPhase {
    /// Key was pressed.
    Pressed,
    /// Key was released.
    Released,
}

/// Keyboard input after optional focus target resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardEvent {
    phase: KeyPhase,
    key: Key,
    modifiers: KeyModifiers,
    target: Option<RuntimeNodeId>,
}

impl KeyboardEvent {
    /// Creates a keyboard event.
    #[must_use]
    pub const fn new(
        phase: KeyPhase,
        key: Key,
        modifiers: KeyModifiers,
        target: Option<RuntimeNodeId>,
    ) -> Self {
        Self {
            phase,
            key,
            modifiers,
            target,
        }
    }

    /// Returns the keyboard phase.
    #[must_use]
    pub const fn phase(&self) -> KeyPhase {
        self.phase
    }

    /// Returns the key identity.
    #[must_use]
    pub const fn key(&self) -> &Key {
        &self.key
    }

    /// Returns keyboard modifiers active during the key event.
    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// Returns the resolved runtime target, if the host already assigned one.
    #[must_use]
    pub const fn target(&self) -> Option<RuntimeNodeId> {
        self.target
    }
}

/// Raw host input event accepted by the runtime boundary.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// Pointer input.
    Pointer(PointerEvent),
    /// Keyboard input.
    Keyboard(KeyboardEvent),
}

/// Runtime-level intent resolved from input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputIntent {
    /// Activate an element by generated runtime node identity.
    ActivateNode(RuntimeNodeId),
}

impl InputIntent {
    /// Creates an activation intent for a runtime node.
    #[must_use]
    pub const fn activate_node(id: RuntimeNodeId) -> Self {
        Self::ActivateNode(id)
    }
}

/// Trace target for runtime events caused by a specific element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceTarget {
    runtime_node_id: RuntimeNodeId,
    authored_id: Option<ElementId>,
}

impl TraceTarget {
    /// Creates a trace target from generated runtime identity and optional authored identity.
    #[must_use]
    pub const fn new(runtime_node_id: RuntimeNodeId, authored_id: Option<ElementId>) -> Self {
        Self {
            runtime_node_id,
            authored_id,
        }
    }

    /// Returns the generated runtime node ID for this target.
    #[must_use]
    pub const fn runtime_node_id(&self) -> RuntimeNodeId {
        self.runtime_node_id
    }

    /// Returns the optional authored element ID for this target.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
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

    fn trace_target(&self) -> TraceTarget {
        TraceTarget::new(self.id, self.authored_id().cloned())
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

/// One runtime trace record, with optional element target details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRecord {
    event: RuntimeEvent,
    target: Option<TraceTarget>,
}

impl TraceRecord {
    const fn new(event: RuntimeEvent, target: Option<TraceTarget>) -> Self {
        Self { event, target }
    }

    /// Returns the coarse runtime event.
    #[must_use]
    pub const fn event(&self) -> RuntimeEvent {
        self.event
    }

    /// Returns target details for events caused by a specific element.
    #[must_use]
    pub const fn target(&self) -> Option<&TraceTarget> {
        self.target.as_ref()
    }
}

/// Ordered runtime trace log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    events: Vec<RuntimeEvent>,
    records: Vec<TraceRecord>,
}

impl Trace {
    /// Creates an empty trace log.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: Vec::new(),
            records: Vec::new(),
        }
    }

    /// Appends one untargeted runtime event.
    pub fn record(&mut self, event: RuntimeEvent) {
        self.record_with_target(event, None);
    }

    /// Appends one runtime event with optional target details.
    pub fn record_with_target(&mut self, event: RuntimeEvent, target: Option<TraceTarget>) {
        self.events.push(event);
        self.records.push(TraceRecord::new(event, target));
    }

    /// Returns the recorded coarse event sequence.
    #[must_use]
    pub const fn events(&self) -> &[RuntimeEvent] {
        self.events.as_slice()
    }

    /// Returns the recorded event sequence with target details.
    #[must_use]
    pub const fn records(&self) -> &[TraceRecord] {
        self.records.as_slice()
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
        self.dispatch_with_target(action, update, root, None);
    }

    fn dispatch_with_target(
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

    /// Consumes the runtime and returns the final application state.
    #[must_use]
    pub fn into_state(self) -> State {
        self.state
    }
}
