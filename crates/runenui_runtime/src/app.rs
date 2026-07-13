//! Application-bound mounted runtime and activation.

use core::marker::PhantomData;

use runenui_core::{Element, ElementId};

use crate::{
    FocusState, FocusTargetResult, InputEvent, Key, KeyPhase, KeyboardEvent, MountedNodeId,
    MountedTreeIndex, PointerButton, PointerEvent, PointerPhase, ReconciliationReport,
    RuntimeError, SurfaceBuildContext, SurfacePublication, Trace, TraceTarget,
    mounted::TargetStatus,
    policy::{
        InputEventResult, KeyboardActivationResult, KeyboardFocusResult, PointerActivationResult,
        PointerFocusResult,
    },
    runtime::Runtime,
    surface::{SurfaceCache, publish_mounted_surface_cached},
};

pub trait UiApp {
    type State;
    type Action;
    fn root(state: &Self::State) -> Element<Self::Action>;
    fn update(state: &mut Self::State, action: Self::Action);
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationResult {
    Dispatched,
    Activated,
    NoAction,
    NotFound,
    NotActivatable,
    Disabled,
    AmbiguousId,
    InvalidId,
    StaleTarget,
    ForeignRuntime,
    RuntimeError(RuntimeError),
}

pub struct AppRuntime<App: UiApp> {
    runtime: Runtime<App::State, App::Action>,
    surface_cache: Option<SurfaceCache>,
    phase_report: crate::SurfacePhaseReport,
    _app: PhantomData<fn() -> App>,
}

impl<App: UiApp> AppRuntime<App> {
    #[must_use]
    pub fn mount(state: App::State) -> Self {
        Self {
            runtime: Runtime::mount(state, App::root),
            surface_cache: None,
            phase_report: crate::SurfacePhaseReport::default(),
            _app: PhantomData,
        }
    }

    /// Dispatches an application action and reconciles the mounted tree.
    ///
    /// # Errors
    ///
    /// Returns an integrity error when the reconciliation generation is exhausted.
    pub fn dispatch(&mut self, action: App::Action) -> Result<&ReconciliationReport, RuntimeError> {
        let report = self.runtime.dispatch(action, App::update, App::root)?;
        self.phase_report = crate::SurfacePhaseReport::one(crate::SurfacePhase::FocusValidation);
        Ok(report)
    }

    #[must_use]
    pub fn index(&mut self) -> MountedTreeIndex<'_, App::Action> {
        self.runtime.tree.index()
    }

    pub fn set_focus(&mut self, id: MountedNodeId) -> FocusTargetResult {
        match self.runtime.tree.target_status(&id) {
            TargetStatus::Foreign => FocusTargetResult::ForeignRuntime,
            TargetStatus::Stale => FocusTargetResult::StaleTarget,
            TargetStatus::Live => {
                let activation = self.runtime.tree.activation(&id);
                if activation
                    .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
                {
                    self.runtime.set_focus(id);
                    FocusTargetResult::Focused
                } else {
                    FocusTargetResult::NotFocusable
                }
            }
        }
    }

    pub fn focus_first(&mut self) -> Option<MountedNodeId> {
        let id = self.index().first_focusable_node().map(|n| n.id().clone());
        self.apply_focus_result(id)
    }
    pub fn focus_last(&mut self) -> Option<MountedNodeId> {
        let id = self.index().last_focusable_node().map(|n| n.id().clone());
        self.apply_focus_result(id)
    }
    pub fn focus_next(&mut self) -> Option<MountedNodeId> {
        let current = self.focus().focused_node().cloned();
        let id = {
            let index = self.index();
            current.as_ref().map_or_else(
                || index.first_focusable_node().map(|n| n.id().clone()),
                |current| {
                    index
                        .next_focusable_after(current)
                        .or_else(|| index.first_focusable_node())
                        .map(|n| n.id().clone())
                },
            )
        };
        self.apply_focus_result(id)
    }
    pub fn focus_previous(&mut self) -> Option<MountedNodeId> {
        let current = self.focus().focused_node().cloned();
        let id = {
            let index = self.index();
            current.as_ref().map_or_else(
                || index.last_focusable_node().map(|n| n.id().clone()),
                |current| {
                    index
                        .previous_focusable_before(current)
                        .or_else(|| index.last_focusable_node())
                        .map(|n| n.id().clone())
                },
            )
        };
        self.apply_focus_result(id)
    }
    fn apply_focus_result(&mut self, id: Option<MountedNodeId>) -> Option<MountedNodeId> {
        if let Some(id) = id {
            self.runtime.set_focus(id.clone());
            Some(id)
        } else {
            self.runtime.clear_focus();
            None
        }
    }
    pub fn clear_focus(&mut self) {
        self.runtime.clear_focus();
    }
    #[must_use]
    pub const fn focus(&self) -> &FocusState {
        self.runtime.focus()
    }
    #[must_use]
    pub const fn state(&self) -> &App::State {
        self.runtime.state()
    }
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        self.runtime.trace()
    }
    #[must_use]
    pub const fn reconciliation_report(&self) -> &ReconciliationReport {
        self.runtime.report()
    }
    #[must_use]
    pub fn publish_surface(&mut self, context: &SurfaceBuildContext<'_>) -> SurfacePublication {
        let (publication, report) = publish_mounted_surface_cached(
            &mut self.runtime.tree,
            context,
            &mut self.surface_cache,
        );
        self.phase_report = report;
        publication
    }
    #[must_use]
    pub const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        &self.phase_report
    }
    #[must_use]
    pub fn into_state(self) -> App::State {
        self.runtime.into_state()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __seed_reconciliation_generation_for_test(&mut self, generation: u64) {
        self.runtime.seed_generation_for_test(generation);
    }

    pub fn activate(&mut self, id: impl AsRef<str>) -> ActivationResult {
        let Ok(id) = ElementId::new(id.as_ref()) else {
            return ActivationResult::InvalidId;
        };
        let node_id = {
            let index = self.index();
            if index.diagnostics().iter().any(|d| {
                d.kind() == crate::DuplicateIdentityKind::ElementId && d.value() == id.as_str()
            }) {
                return ActivationResult::AmbiguousId;
            }
            index.node_by_authored_id(&id).map(|n| n.id().clone())
        };
        node_id.map_or(ActivationResult::NotFound, |id| self.activate_node(&id))
    }

    pub fn activate_node(&mut self, id: &MountedNodeId) -> ActivationResult {
        match self.runtime.tree.target_status(id) {
            TargetStatus::Foreign => return ActivationResult::ForeignRuntime,
            TargetStatus::Stale => return ActivationResult::StaleTarget,
            TargetStatus::Live => {}
        }
        let (activation, target) = {
            let Ok(activation) = self.runtime.tree.activation_probe(id) else {
                return ActivationResult::RuntimeError(RuntimeError::WidgetStatePayloadMismatch);
            };
            let authored = self
                .runtime
                .tree
                .node(id)
                .and_then(|n| n.authored_id.clone());
            (activation, TraceTarget::new(id.clone(), authored))
        };
        if !activation.is_actionable() {
            return ActivationResult::NotActivatable;
        }
        if !activation.enabled() {
            return ActivationResult::Disabled;
        }
        if let Err(error) = self.runtime.preflight_reconciliation_generation() {
            return ActivationResult::RuntimeError(error);
        }
        let Ok((action, invalidation)) = self.runtime.tree.activate(id) else {
            return ActivationResult::RuntimeError(RuntimeError::WidgetStatePayloadMismatch);
        };
        if let Some(action) = action {
            return match self.runtime.dispatch_with_target(
                action,
                App::update,
                App::root,
                Some(target),
            ) {
                Ok(_) => ActivationResult::Dispatched,
                Err(error) => ActivationResult::RuntimeError(error),
            };
        }
        if invalidation.is_empty() {
            ActivationResult::NoAction
        } else {
            if invalidation.contains(runenui_core::WidgetInvalidation::INTERACTION) {
                let focused = self.runtime.focus().focused_node().cloned();
                if focused.as_ref().is_some_and(|focused| {
                    self.runtime
                        .tree
                        .activation(focused)
                        .map_or(true, |activation| {
                            !activation.enabled() || !activation.is_actionable()
                        })
                }) {
                    self.runtime.clear_focus();
                }
                self.runtime.tree.finish_focus_validation();
                self.phase_report =
                    crate::SurfacePhaseReport::one(crate::SurfacePhase::FocusValidation);
            }
            ActivationResult::Activated
        }
    }

    pub fn handle_keyboard_focus(&mut self, event: &KeyboardEvent) -> KeyboardFocusResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Tab) {
            return KeyboardFocusResult::Ignored;
        }
        let id = if event.modifiers().shift() {
            self.focus_previous()
        } else {
            self.focus_next()
        };
        id.map_or(
            KeyboardFocusResult::NoFocusableNode,
            KeyboardFocusResult::Moved,
        )
    }
    pub fn handle_pointer_focus(&mut self, event: &PointerEvent) -> PointerFocusResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerFocusResult::Ignored;
        }
        let Some(id) = event.target().cloned() else {
            return PointerFocusResult::NoTarget;
        };
        match self.set_focus(id.clone()) {
            FocusTargetResult::Focused => PointerFocusResult::Moved(id),
            FocusTargetResult::NotFocusable => PointerFocusResult::NotFocusable,
            FocusTargetResult::StaleTarget | FocusTargetResult::ForeignRuntime => {
                PointerFocusResult::NotFound
            }
        }
    }
    pub fn handle_keyboard_activation(
        &mut self,
        event: &KeyboardEvent,
    ) -> KeyboardActivationResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Enter | Key::Space) {
            return KeyboardActivationResult::Ignored;
        }
        let Some(id) = self.focus().focused_node().cloned() else {
            return KeyboardActivationResult::NoFocusedNode;
        };
        KeyboardActivationResult::Handled(self.activate_node(&id))
    }
    pub fn handle_pointer_activation(&mut self, event: &PointerEvent) -> PointerActivationResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerActivationResult::Ignored;
        }
        let Some(id) = event.target().cloned() else {
            return PointerActivationResult::NoTarget;
        };
        PointerActivationResult::Handled(self.activate_node(&id))
    }
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

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use runenui_core::{
        Element, StyleTokens, Widget, WidgetActivation, WidgetActivationContext,
        WidgetInvalidation, WidgetPaintProof,
    };

    use crate::{
        ActivationResult, AppRuntime, LayoutConstraints, RuntimeError, SurfaceBuildContext, UiApp,
    };

    #[derive(Clone, Copy, Debug)]
    enum Action {
        Fire,
    }

    #[derive(Debug)]
    struct State {
        updates: usize,
        mutable_calls: Rc<Cell<usize>>,
    }

    #[derive(Debug)]
    struct OneShot {
        action: Option<Action>,
        mutable_calls: Rc<Cell<usize>>,
    }

    impl Widget<Action> for OneShot {
        type State = usize;

        fn create_state(&self) -> Self::State {
            0
        }

        fn activation(&self, _: &Self::State) -> WidgetActivation {
            WidgetActivation::actionable(true)
        }

        fn activate(
            &mut self,
            state: &mut Self::State,
            context: &mut WidgetActivationContext,
        ) -> Option<Action> {
            self.mutable_calls.set(self.mutable_calls.get() + 1);
            *state += 1;
            context.invalidate(WidgetInvalidation::PAINT);
            self.action.take()
        }

        fn paint(&self, state: &Self::State) -> WidgetPaintProof {
            WidgetPaintProof::new("one-shot", state.to_string())
        }
    }

    struct IntegrityApp;

    impl UiApp for IntegrityApp {
        type State = State;
        type Action = Action;

        fn root(state: &Self::State) -> Element<Self::Action> {
            Element::new(OneShot {
                action: Some(Action::Fire),
                mutable_calls: Rc::clone(&state.mutable_calls),
            })
            .key("root")
        }

        fn update(state: &mut Self::State, action: Self::Action) {
            match action {
                Action::Fire => {}
            }
            state.updates += 1;
        }
    }

    #[derive(Debug)]
    struct SlotWidget;

    impl Widget<()> for SlotWidget {
        type State = usize;

        fn create_state(&self) -> Self::State {
            0
        }

        fn activation(&self, _: &Self::State) -> WidgetActivation {
            WidgetActivation::actionable(true)
        }

        fn activate(
            &mut self,
            state: &mut Self::State,
            context: &mut WidgetActivationContext,
        ) -> Option<()> {
            *state += 1;
            context.invalidate(WidgetInvalidation::PAINT);
            None
        }
    }

    struct SlotApp;

    impl UiApp for SlotApp {
        type State = usize;
        type Action = ();

        fn root(_: &Self::State) -> Element<Self::Action> {
            Element::new(SlotWidget).id("slot").key("slot")
        }

        fn update(state: &mut Self::State, (): Self::Action) {
            *state += 1;
        }
    }

    fn assert_non_default_slots(runtime: &mut AppRuntime<SlotApp>, id: &crate::MountedNodeId) {
        let index = runtime.index();
        let interaction = index
            .node(id)
            .unwrap_or_else(|| unreachable!("compatible lifetime remains live"))
            .interaction();
        assert!(interaction.hovered());
        assert!(interaction.pressed());
        assert!(interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (23.0, 37.0));
    }

    #[test]
    fn ordinary_dispatch_and_state_only_activation_retain_every_interaction_slot() {
        let mut runtime = AppRuntime::<SlotApp>::mount(0);
        let id = runtime.index().nodes()[0].id().clone();
        runtime
            .runtime
            .tree
            .set_interaction_for_test(&id, true, true, true, (23.0, 37.0));

        runtime.dispatch(()).unwrap_or_else(|_| unreachable!());
        assert_eq!(runtime.index().nodes()[0].id(), &id);
        assert_non_default_slots(&mut runtime, &id);

        assert_eq!(runtime.activate_node(&id), ActivationResult::Activated);
        assert_eq!(runtime.index().nodes()[0].id(), &id);
        assert_non_default_slots(&mut runtime, &id);
    }

    #[test]
    fn exhausted_generation_rejects_activation_before_every_mutation() {
        let mutable_calls = Rc::new(Cell::new(0));
        let mut runtime = AppRuntime::<IntegrityApp>::mount(State {
            updates: 0,
            mutable_calls: Rc::clone(&mutable_calls),
        });
        let id = runtime.index().nodes()[0].id().clone();
        let semantic = runtime.index().nodes()[0].semantic_id().clone();
        runtime
            .runtime
            .tree
            .set_interaction_for_test(&id, true, true, true, (17.0, 29.0));
        assert_eq!(
            runtime.set_focus(id.clone()),
            crate::FocusTargetResult::Focused
        );
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let before_publication = runtime.publish_surface(&context);
        let before_report = runtime.reconciliation_report().clone();
        let before_trace = runtime.trace().clone();
        let before_phase_report = runtime.last_surface_phase_report().clone();
        runtime.runtime.seed_generation_for_test(u64::MAX);

        assert_eq!(
            runtime.activate_node(&id),
            ActivationResult::RuntimeError(RuntimeError::ReconciliationGenerationExhausted)
        );
        assert_eq!(runtime.state().updates, 0);
        assert_eq!(mutable_calls.get(), 0);
        assert_eq!(runtime.index().nodes()[0].id(), &id);
        assert_eq!(runtime.index().nodes()[0].semantic_id(), &semantic);
        assert_eq!(runtime.focus().focused_node(), Some(&id));
        let interaction = runtime.index().nodes()[0].interaction();
        assert!(interaction.hovered());
        assert!(interaction.pressed());
        assert!(interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (17.0, 29.0));
        assert_eq!(runtime.reconciliation_report(), &before_report);
        assert_eq!(runtime.trace(), &before_trace);
        assert_eq!(runtime.last_surface_phase_report(), &before_phase_report);
        assert_eq!(runtime.publish_surface(&context), before_publication);

        runtime.runtime.seed_generation_for_test(1);
        assert_eq!(runtime.activate_node(&id), ActivationResult::Dispatched);
        assert_eq!(runtime.state().updates, 1);
        assert_eq!(mutable_calls.get(), 1);
    }

    #[test]
    fn corrupted_capabilities_remain_integrity_errors_and_reconcile_to_fresh_state() {
        let mutable_calls = Rc::new(Cell::new(0));
        let mut runtime = AppRuntime::<IntegrityApp>::mount(State {
            updates: 0,
            mutable_calls: Rc::clone(&mutable_calls),
        });
        let old = runtime.index().nodes()[0].id().clone();
        runtime.runtime.tree.corrupt_state_for_test(&old);

        assert_eq!(
            runtime.activate_node(&old),
            ActivationResult::RuntimeError(RuntimeError::WidgetStatePayloadMismatch)
        );
        assert_eq!(mutable_calls.get(), 0);
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());
        let publication = runtime.publish_surface(&context);
        assert_eq!(
            publication
                .frame()
                .root()
                .unwrap_or_else(|| unreachable!())
                .diagnostics()[0]
                .code(),
            "runenui.runtime.state-payload-mismatch"
        );
        assert_eq!(mutable_calls.get(), 0);

        runtime
            .dispatch(Action::Fire)
            .unwrap_or_else(|_| unreachable!());
        let fresh = runtime.index().nodes()[0].id().clone();
        assert_ne!(fresh, old);
        assert_eq!(runtime.state().updates, 1);
        assert_eq!(runtime.reconciliation_report().mounted_count(), 1);
        assert_eq!(runtime.reconciliation_report().unmounted_count(), 1);
        assert_eq!(
            runtime.reconciliation_report().diagnostics(),
            &[crate::ReconciliationDiagnostic::StatePayloadMismatch {
                path: "root".to_owned(),
            }]
        );
        let publication = runtime.publish_surface(&context);
        assert_eq!(
            publication
                .frame()
                .root()
                .unwrap_or_else(|| unreachable!())
                .paint()
                .description(),
            "0"
        );
    }
}
