//! App-bound runtime wrapper and activation.

use core::marker::PhantomData;

use runenui_core::{Element, ElementId};

use crate::{
    FocusState, InputEvent, Key, KeyPhase, KeyboardEvent, PointerButton, PointerEvent,
    PointerPhase, RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex, SurfaceBuildContext,
    SurfacePublication, Trace, TraceTarget,
    policy::{
        InputEventResult, KeyboardActivationResult, KeyboardFocusResult, PointerActivationResult,
        PointerFocusResult,
    },
    publish_surface,
    runtime::Runtime,
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
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationResult {
    /// The matching actionable widget's action was dispatched.
    Dispatched,
    /// No element with the requested authored or runtime ID exists in the current tree.
    NotFound,
    /// The requested element exists, but the element is not activatable.
    NotActivatable,
    /// The requested element exists, but it is intentionally disabled.
    Disabled,
    /// The requested actionable widget has no remaining widget action.
    NoAction,
    /// More than one element has the requested authored ID.
    AmbiguousId,
    /// The requested authored ID was invalid.
    InvalidId,
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

    /// Sets focus to a focusable runtime node in the current tree.
    ///
    /// Returns `false` when the node ID is not present or is not focusable.
    pub fn set_focus(&mut self, id: RuntimeNodeId) -> bool {
        if matches!(self.index().node(id), Some(node) if node.is_focusable()) {
            self.runtime.set_focus(id);
            true
        } else {
            false
        }
    }

    /// Focuses the first focusable runtime node in traversal order.
    pub fn focus_first(&mut self) -> Option<RuntimeNodeId> {
        let node_id = self.index().first_focusable_node().map(RuntimeNodeRef::id);
        self.apply_focus_result(node_id)
    }

    /// Focuses the last focusable runtime node in traversal order.
    pub fn focus_last(&mut self) -> Option<RuntimeNodeId> {
        let node_id = self.index().last_focusable_node().map(RuntimeNodeRef::id);
        self.apply_focus_result(node_id)
    }

    /// Focuses the next focusable runtime node, wrapping to the first node.
    ///
    /// If there is no focused node, this focuses the first focusable node.
    pub fn focus_next(&mut self) -> Option<RuntimeNodeId> {
        let node_id = {
            let index = self.index();
            self.focus().focused_node().map_or_else(
                || index.first_focusable_node().map(RuntimeNodeRef::id),
                |current| {
                    index
                        .next_focusable_after(current)
                        .or_else(|| index.first_focusable_node())
                        .map(RuntimeNodeRef::id)
                },
            )
        };

        self.apply_focus_result(node_id)
    }

    /// Focuses the previous focusable runtime node, wrapping to the last node.
    ///
    /// If there is no focused node, this focuses the last focusable node.
    pub fn focus_previous(&mut self) -> Option<RuntimeNodeId> {
        let node_id = {
            let index = self.index();
            self.focus().focused_node().map_or_else(
                || index.last_focusable_node().map(RuntimeNodeRef::id),
                |current| {
                    index
                        .previous_focusable_before(current)
                        .or_else(|| index.last_focusable_node())
                        .map(RuntimeNodeRef::id)
                },
            )
        };

        self.apply_focus_result(node_id)
    }

    /// Applies keyboard focus policy to one keyboard event.
    ///
    /// Pressed Tab moves to the next focusable node. Pressed Shift+Tab moves to
    /// the previous focusable node. Other keyboard events are ignored.
    pub fn handle_keyboard_focus(&mut self, event: &KeyboardEvent) -> KeyboardFocusResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Tab) {
            return KeyboardFocusResult::Ignored;
        }

        let node_id = if event.modifiers().shift() {
            self.focus_previous()
        } else {
            self.focus_next()
        };

        node_id.map_or(
            KeyboardFocusResult::NoFocusableNode,
            KeyboardFocusResult::Moved,
        )
    }

    /// Applies pointer focus policy to one already-targeted pointer event.
    ///
    /// Pressed primary pointer events focus the resolved target when that target
    /// is present and focusable. Other pointer events are ignored.
    pub fn handle_pointer_focus(&mut self, event: &PointerEvent) -> PointerFocusResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerFocusResult::Ignored;
        }

        let Some(node_id) = event.target() else {
            return PointerFocusResult::NoTarget;
        };

        let focusable = {
            let index = self.index();
            index.node(node_id).map(RuntimeNodeRef::is_focusable)
        };

        match focusable {
            Some(true) => {
                self.runtime.set_focus(node_id);
                PointerFocusResult::Moved(node_id)
            }
            Some(false) => PointerFocusResult::NotFocusable,
            None => PointerFocusResult::NotFound,
        }
    }

    const fn apply_focus_result(
        &mut self,
        node_id: Option<RuntimeNodeId>,
    ) -> Option<RuntimeNodeId> {
        if let Some(node_id) = node_id {
            self.runtime.set_focus(node_id);
            Some(node_id)
        } else {
            self.runtime.clear_focus();
            None
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

    /// Publishes aligned renderer-facing, style, and layout-diagnostic products.
    #[must_use]
    pub fn publish_surface(&self, context: &SurfaceBuildContext<'_>) -> SurfacePublication {
        publish_surface(self.root(), context)
    }

    /// Returns the runtime trace.
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        self.runtime.trace()
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
{
    /// Activates the element with the matching authored ID in the current tree.
    ///
    /// This is a semantic headless activation path for tests, tools, and host
    /// automation. Renderer hit testing should eventually resolve to internal
    /// runtime node identity and call [`Self::activate_node`].
    pub fn activate(&mut self, id: impl AsRef<str>) -> ActivationResult {
        let Ok(id) = ElementId::new(id.as_ref()) else {
            return ActivationResult::InvalidId;
        };
        let node_id = {
            let index = self.index();
            if index.diagnostics().iter().any(|diagnostic| {
                diagnostic.kind() == crate::DuplicateIdentityKind::ElementId
                    && diagnostic.value() == id.as_str()
            }) {
                return ActivationResult::AmbiguousId;
            }
            index.node_by_authored_id(&id).map(RuntimeNodeRef::id)
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
            ActivationLookup::Ready { target } => {
                let action = self
                    .runtime
                    .root_mut()
                    .extract_action_at_preorder_for_runtime(id.as_usize());
                if let Some(action) = action {
                    self.runtime
                        .dispatch_with_target(action, App::update, App::root, Some(target));
                    ActivationResult::Dispatched
                } else {
                    ActivationResult::NoAction
                }
            }
            ActivationLookup::NotFound => ActivationResult::NotFound,
            ActivationLookup::NotActivatable => ActivationResult::NotActivatable,
            ActivationLookup::Disabled => ActivationResult::Disabled,
        }
    }

    /// Applies keyboard activation policy to one keyboard event.
    ///
    /// Pressed Enter or Space activates the currently focused runtime node.
    /// Other keyboard events are ignored.
    pub fn handle_keyboard_activation(
        &mut self,
        event: &KeyboardEvent,
    ) -> KeyboardActivationResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Enter | Key::Space) {
            return KeyboardActivationResult::Ignored;
        }

        let Some(node_id) = self.focus().focused_node() else {
            return KeyboardActivationResult::NoFocusedNode;
        };

        KeyboardActivationResult::Handled(self.activate_node(node_id))
    }

    /// Applies pointer activation policy to one already-targeted pointer event.
    ///
    /// Pressed primary pointer events activate the resolved target. Other pointer
    /// events are ignored.
    pub fn handle_pointer_activation(&mut self, event: &PointerEvent) -> PointerActivationResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerActivationResult::Ignored;
        }

        let Some(node_id) = event.target() else {
            return PointerActivationResult::NoTarget;
        };

        PointerActivationResult::Handled(self.activate_node(node_id))
    }

    /// Applies the default runtime policy for one already-targeted input event.
    ///
    /// Pointer events run focus policy first and activation policy second.
    /// Keyboard events route Tab to focus policy and Enter/Space to activation
    /// policy. Other input events are ignored.
    pub fn handle_input_event(&mut self, event: &InputEvent) -> InputEventResult {
        match event {
            InputEvent::Pointer(event) => {
                let focus = self.handle_pointer_focus(event);
                let activation = self.handle_pointer_activation(event);

                if focus == PointerFocusResult::Ignored
                    && activation == PointerActivationResult::Ignored
                {
                    InputEventResult::Ignored
                } else {
                    InputEventResult::Pointer { focus, activation }
                }
            }
            InputEvent::Keyboard(event) => {
                let focus = self.handle_keyboard_focus(event);
                if focus != KeyboardFocusResult::Ignored {
                    return InputEventResult::KeyboardFocus(focus);
                }

                let activation = self.handle_keyboard_activation(event);
                if activation == KeyboardActivationResult::Ignored {
                    InputEventResult::Ignored
                } else {
                    InputEventResult::KeyboardActivation(activation)
                }
            }
        }
    }
}

enum ActivationLookup {
    Ready { target: TraceTarget },
    NotFound,
    NotActivatable,
    Disabled,
}

fn activation_lookup<Action>(node: &RuntimeNodeRef<'_, Action>) -> ActivationLookup {
    let activation = node.element().activation();
    if !activation.is_actionable() {
        return ActivationLookup::NotActivatable;
    }
    if !activation.enabled() {
        return ActivationLookup::Disabled;
    }

    ActivationLookup::Ready {
        target: node.trace_target(),
    }
}
