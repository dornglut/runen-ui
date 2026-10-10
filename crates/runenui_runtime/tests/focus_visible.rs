#![allow(refining_impl_trait)]

use runenui_core::{
    Brush, Color, CommandOrigin, Element, ElementId, HitContribution, HitContributionContext, KeyLocation,
    KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, Outline, PhysicalKey, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticAction,
    SemanticActionRequest, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, FocusScope, StyleEnvironment, StyleInteractionState,
    StyleProperties, StyleRecipe, StyleRecipeId, StyleTheme, StyleTokens, StrokeStyle, UiApp, View,
    Widget, WidgetActivation, WidgetMeasure, WidgetTextInput, button, children, column,
};
use runenui_runtime::{
    AppRuntime, InputModality, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext,
    TraceFocusRecordRole, TraceRecordKind, TraceReplay,
};

#[derive(Clone, Copy, Debug)]
enum Action {
    Activate,
}

struct App;

impl UiApp for App {
    type State = ();
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(children![
            button("Ordinary").id("ordinary").on_activate(|| Action::Activate),
            Element::new(TextInputProbe).id("editable").focusable(true),
            button("Other").id("other").on_activate(|| Action::Activate),
        ])
    }

    fn update((): &mut Self::State, Action::Activate: Self::Action) {}
}

/// Public downstream Widget text-input capability, not a built-in type identity.
#[derive(Debug)]
struct TextInputProbe;

impl Widget<Action> for TextInputProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, false)
    }

    fn measure(
        &self,
        (): &Self::State,
        _: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(64_u16), LogicalLength::from(24_u16))
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("measured size produces finite hit bounds"));
        HitContribution::single_rect(rect)
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button).with_name("Editable"),
        )
    }
}

fn settle<Application: UiApp>(runtime: &mut AppRuntime<Application>) {
    assert!(runtime.pump(PumpBudget::new(64, 64, 64, 64)).expect("pump observation").report().to_owned().is_quiescent());
}

fn mount() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(());
    settle(&mut runtime);
    runtime
}

fn mounted_id<Application: UiApp>(runtime: &mut AppRuntime<Application>, authored: &str) -> MountedNodeId {
    let id = ElementId::new(authored).unwrap_or_else(|_| unreachable!("known id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("test widget mounted"))
        .id()
        .clone()
}

fn focus<Application: UiApp>(runtime: &mut AppRuntime<Application>, node: &MountedNodeId, origin: CommandOrigin) {
    assert!(runtime
        .submit_command(node.clone(), SemanticCommand::RequestFocus, origin)
        .is_ok());
    settle(runtime);
    assert_eq!(runtime.focus().focused_node(), Some(node));
}

fn pointer_focus<Application: UiApp>(runtime: &mut AppRuntime<Application>, name: &str, pointer: u64) {
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(260.0, 180.0)
        .unwrap_or_else(|_| unreachable!("finite surface"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|error| panic!("published surface: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let node = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some(name))
        .unwrap_or_else(|| unreachable!("named widget is on surface"));
    let bounds = node.bounds();
    let point = LogicalPoint::new(bounds.x() + bounds.width() / 2.0, bounds.y() + bounds.height() / 2.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let id = PointerId::new(pointer).unwrap_or_else(|| unreachable!("nonzero id"));
    let context = publication.input_context().clone();
    let down = PointerEvent::new(id, PointerDeviceKind::Mouse, PointerPhase::Down, point, context)
        .with_buttons(PointerButtons::new([PointerButton::Primary]))
        .with_changed_button(PointerButton::Primary);
    assert!(runtime.submit_pointer(down).is_ok());
    settle(runtime);
}

#[test]
fn pointer_focus_is_hidden_for_button_but_visible_for_custom_text_input() {
    let mut app = mount();
    let ordinary = mounted_id(&mut app, "ordinary");
    pointer_focus(&mut app, "Ordinary", 1);
    assert_eq!(app.focus().focused_node(), Some(&ordinary));
    assert!(!app.focus().focus_visible());

    let mut custom = mount();
    let editable = mounted_id(&mut custom, "editable");
    pointer_focus(&mut custom, "Editable", 2);
    assert_eq!(custom.focus().focused_node(), Some(&editable));
    assert!(custom.focus().focus_visible());
}

#[test]
fn source_aware_transfers_and_stable_owner_promotion_are_distinct() {
    let mut app = mount();
    let ordinary = mounted_id(&mut app, "ordinary");
    let other = mounted_id(&mut app, "other");

    // Initial programmatic acquisition is visible, then carries across transfer.
    focus(&mut app, &ordinary, CommandOrigin::programmatic());
    assert!(app.focus().focus_visible());
    focus(&mut app, &other, CommandOrigin::automation());
    assert!(app.focus().focus_visible());

    // Pointer-acquired focus is hidden and programmatic transfer inherits hidden.
    let mut hidden = mount();
    let first = mounted_id(&mut hidden, "ordinary");
    let next = mounted_id(&mut hidden, "other");
    pointer_focus(&mut hidden, "Ordinary", 3);
    assert_eq!(hidden.focus().focused_node(), Some(&first));
    assert!(!hidden.focus().focus_visible());
    focus(&mut hidden, &next, CommandOrigin::programmatic());
    assert!(!hidden.focus().focus_visible());

    // A committed controller event on that same focused owner promotes without focus transfer.
    let before = hidden.trace().records().filter(|r| matches!(r.kind(), TraceRecordKind::FocusTransitionCommitted { .. })).count();
    focus(&mut hidden, &next, CommandOrigin::controller());
    assert_eq!(hidden.focus().modality(), Some(InputModality::Controller));
    assert!(hidden.focus().focus_visible());
    let after = hidden.trace().records().filter(|r| matches!(r.kind(), TraceRecordKind::FocusTransitionCommitted { .. })).count();
    assert_eq!(before, after);
    assert!(hidden.trace().records().any(|r| {
        matches!(r.kind(), TraceRecordKind::FocusVisibilityChanged { visible: true })
            && r.context().focus_record_role() == Some(TraceFocusRecordRole::FocusVisibility)
    }));
    // Same-owner programmatic and pointer modality cannot demote a promoted owner.
    focus(&mut hidden, &next, CommandOrigin::programmatic());
    assert!(hidden.focus().focus_visible());
    let jsonl = hidden.trace().export_jsonl();
    assert!(jsonl.contains("focus_visibility_changed"));
    assert!(jsonl.contains("focus_visibility"));
    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| panic!("typed visibility trace replays: {error:?}"));
    assert!(replay
        .records()
        .any(|record| record.kind().as_str() == "focus_visibility_changed"));
}

#[test]
fn nonpointing_focus_sources_and_automation_inheritance_use_canonical_modality() {
    for origin in [CommandOrigin::controller(), CommandOrigin::accessibility()] {
        let mut app = mount();
        let ordinary = mounted_id(&mut app, "ordinary");
        focus(&mut app, &ordinary, origin);
        assert!(app.focus().focus_visible());
    }

    let mut app = mount();
    let ordinary = mounted_id(&mut app, "ordinary");
    focus(&mut app, &ordinary, CommandOrigin::automation());
    assert!(app.focus().focus_visible());

    // Automation does not invent visible indication when transferring hidden focus.
    let mut app = mount();
    let other = mounted_id(&mut app, "other");
    pointer_focus(&mut app, "Ordinary", 9);
    assert!(!app.focus().focus_visible());
    focus(&mut app, &other, CommandOrigin::automation());
    assert!(!app.focus().focus_visible());
}

#[test]
fn semantic_action_request_focus_uses_accessibility_ingress_and_typed_trace() {
    let mut app = mount();
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(260.0, 180.0)
        .unwrap_or_else(|_| unreachable!("finite surface"));
    let publication = app
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("surface publishes"));
    let snapshot = publication.semantic_publication().snapshot();
    let target = snapshot.nodes().iter().find(|n| n.name() == Some("Ordinary"))
        .unwrap_or_else(|| unreachable!("button in semantic snapshot"));
    let request = SemanticActionRequest::new(snapshot.surface_id().clone(), target.id().clone(), SemanticAction::RequestFocus);
    assert!(app.submit_semantic_action(request).is_ok());
    settle(&mut app);
    assert_eq!(app.focus().modality(), Some(InputModality::Accessibility));
    assert!(app.focus().focus_visible());
}

#[test]
fn shutdown_clears_focus_indication_with_real_focus_authority() {
    let mut app = mount();
    let ordinary = mounted_id(&mut app, "ordinary");
    focus(&mut app, &ordinary, CommandOrigin::controller());
    assert!(app.focus().focus_visible());
    app.shutdown().expect("shutdown observation").report().to_owned();
    assert!(app.focus().focused_node().is_none());
    assert!(!app.focus().focus_visible());
}


#[test]
fn keyboard_traversal_promotes_pointer_focus_and_tracks_canonical_target() {
    let mut app = mount();
    pointer_focus(&mut app, "Ordinary", 12);
    assert!(!app.focus().focus_visible());
    let event = KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Tab,
        LogicalKey::Tab,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    );
    assert!(app.submit_keyboard(event).is_ok());
    settle(&mut app);
    assert_eq!(app.focus().modality(), Some(InputModality::Keyboard));
    assert!(app.focus().focus_visible());
    assert!(app.focus().focused_node().is_some());
}

#[test]
fn pointer_activity_does_not_demote_visible_focus_without_owner_transfer() {
    let mut app = mount();
    let ordinary = mounted_id(&mut app, "ordinary");
    pointer_focus(&mut app, "Ordinary", 13);
    assert!(!app.focus().focus_visible());
    focus(&mut app, &ordinary, CommandOrigin::controller());
    assert!(app.focus().focus_visible());

    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(260.0, 180.0)
        .unwrap_or_else(|_| unreachable!("finite surface"));
    let context = app
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("updated surface publishes"))
        .input_context()
        .clone();
    let point = LogicalPoint::new(1.0, 1.0).unwrap_or_else(|_| unreachable!("finite point"));
    let move_event = PointerEvent::new(
        PointerId::new(13).unwrap_or_else(|| unreachable!("valid pointer")),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        context,
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    assert!(app.submit_pointer(move_event).is_ok());
    settle(&mut app);
    assert_eq!(app.focus().focused_node(), Some(&ordinary));
    assert!(app.focus().focus_visible());
}

struct DisabledApp;

impl UiApp for DisabledApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(enabled: &Self::State) -> impl View<Self::Action> {
        button("Retire").id("retire").enabled(*enabled).on_activate(|| ())
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        *state = false;
    }
}

#[test]
fn disablement_retires_focus_and_its_visibility_latch_together() {
    let mut app = AppRuntime::<DisabledApp>::mount(true);
    let id = ElementId::new("retire").unwrap_or_else(|_| unreachable!("known id"));
    let target = app
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("retire is mounted"))
        .id()
        .clone();
    assert!(app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    ).is_ok());
    assert!(app.pump(PumpBudget::new(64, 64, 64, 64)).expect("pump observation").report().to_owned().is_quiescent());
    assert!(app.focus().focus_visible());

    assert!(app.submit_action(()).is_ok());
    assert!(app.pump(PumpBudget::new(64, 64, 64, 64)).expect("pump observation").report().to_owned().is_quiescent());
    assert!(app.focus().focused_node().is_none());
    assert!(!app.focus().focus_visible());
}

struct RestoreApp;

impl UiApp for RestoreApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(children![
            column(children![
                button("Remember").id("remember").on_activate(|| ()),
            ])
            .id("nested")
            .focus_scope(FocusScope::new()),
            button("Outside").id("outside").on_activate(|| ()),
        ])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn remembered_restoration_obeys_canonical_source_not_focus_reason() {
    let mut app = AppRuntime::<RestoreApp>::mount(());
    settle(&mut app);
    let inner = mounted_id(&mut app, "remember");
    let outer = mounted_id(&mut app, "outside");
    let scope = mounted_id(&mut app, "nested");
    pointer_focus(&mut app, "Remember", 27);
    assert_eq!(app.focus().focused_node(), Some(&inner));
    assert!(!app.focus().focus_visible());

    focus(&mut app, &outer, CommandOrigin::programmatic());
    assert!(!app.focus().focus_visible());
    assert!(app
        .submit_command(
            scope.clone(),
            SemanticCommand::RestoreFocus,
            CommandOrigin::programmatic(),
        )
        .is_ok());
    settle(&mut app);
    assert_eq!(app.focus().focused_node(), Some(&inner));
    assert_eq!(
        app.focus().reason(),
        Some(runenui_core::FocusReason::RememberedRestoration)
    );
    assert!(!app.focus().focus_visible());

    focus(&mut app, &outer, CommandOrigin::controller());
    assert!(app.focus().focus_visible());
    assert!(app
        .submit_command(
            scope,
            SemanticCommand::RestoreFocus,
            CommandOrigin::programmatic(),
        )
        .is_ok());
    settle(&mut app);
    assert_eq!(app.focus().focused_node(), Some(&inner));
    assert!(app.focus().focus_visible());
}


struct StyledApp;

impl UiApp for StyledApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let recipe = StyleRecipeId::new("control.focus-indicator")
            .unwrap_or_else(|_| unreachable!("recipe identity is valid"));
        column(children![
            button("Focus indicator")
                .id("styled")
                .recipe(recipe.clone())
                .on_activate(|| ()),
            Element::new(TextInputProbe)
                .id("styled-custom")
                .focusable(true)
                .recipe(recipe),
        ])
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn stable_owner_promotion_republishes_focus_visible_recipe_without_focus_transfer() {
    let mut recipe = StyleRecipe::new(StyleProperties::EMPTY);
    recipe
        .define_interaction(
            StyleInteractionState::FocusVisible,
            StyleProperties::EMPTY.with_outline(Outline::new(
                Brush::solid(Color::BLACK),
                StrokeStyle::new(LogicalLength::from(2_u16)),
            )),
        )
        .unwrap_or_else(|_| unreachable!("focus-visible recipe defined once"));
    let mut theme = StyleTheme::new(StyleTokens::new());
    theme
        .define_recipe(
            StyleRecipeId::new("control.focus-indicator")
                .unwrap_or_else(|_| unreachable!("recipe identity is valid")),
            recipe,
        )
        .unwrap_or_else(|_| unreachable!("recipe defined once"));
    let environment = StyleEnvironment::new(theme);
    let size = LogicalSize::try_new(240.0, 100.0)
        .unwrap_or_else(|_| unreachable!("finite publication"));
    let context = SurfaceBuildContext::tight(&environment, size);
    let mut app = AppRuntime::<StyledApp>::mount(());
    settle(&mut app);

    let initial = app.publish_surface(&context)
        .unwrap_or_else(|error| panic!("initial styled publication: {error:?}"));
    let styled = initial.frame().nodes().iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "styled"))
        .unwrap_or_else(|| unreachable!("styled button publishes"));
    assert!(styled.computed_style().outline().is_none());
    let target = styled.id().clone();
    let bounds = styled.bounds();
    let point = LogicalPoint::new(bounds.x() + bounds.width() / 2.0, bounds.y() + bounds.height() / 2.0)
        .unwrap_or_else(|_| unreachable!("finite geometry"));
    let down = PointerEvent::new(
        PointerId::new(77).unwrap_or_else(|| unreachable!("nonzero pointer")),
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        initial.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    assert!(app.submit_pointer(down).is_ok());
    settle(&mut app);
    assert_eq!(app.focus().focused_node(), Some(&target));
    assert!(!app.focus().focus_visible());
    let pointer_publication = app.publish_surface(&context)
        .unwrap_or_else(|error| panic!("pointer publication: {error:?}"));
    assert!(pointer_publication.frame().nodes().iter()
        .find(|node| node.id() == &target)
        .unwrap_or_else(|| unreachable!("focused button published"))
        .computed_style().outline().is_none());

    let focus_transitions = app.trace().records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::FocusTransitionCommitted { .. }))
        .count();
    assert!(app.submit_command(
        target.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::controller(),
    ).is_ok());
    settle(&mut app);
    assert_eq!(app.focus().focused_node(), Some(&target));
    assert!(app.focus().focus_visible());
    assert_eq!(focus_transitions, app.trace().records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::FocusTransitionCommitted { .. }))
        .count());
    let after = app.publish_surface(&context)
        .unwrap_or_else(|error| panic!("promoted publication: {error:?}"));
    assert!(after.frame().nodes().iter()
        .find(|node| node.id() == &target)
        .unwrap_or_else(|| unreachable!("unchanged owner remains published"))
        .computed_style().outline().is_some());
    assert!(app.trace().records().any(|record| {
        matches!(record.kind(), TraceRecordKind::FocusVisibilityChanged { visible: true })
    }));

    // The same public recipe also works for a non-built-in Widget with text-input capability.
    let custom = after.frame().nodes().iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "styled-custom"))
        .unwrap_or_else(|| unreachable!("downstream styled widget is published"));
    let custom_id = custom.id().clone();
    let bounds = custom.bounds();
    let point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    ).unwrap_or_else(|_| unreachable!("published custom bounds are finite"));
    let down = PointerEvent::new(
        PointerId::new(78).unwrap_or_else(|| unreachable!("distinct pointer")),
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        after.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    assert!(app.submit_pointer(down).is_ok());
    settle(&mut app);
    assert_eq!(app.focus().focused_node(), Some(&custom_id));
    assert!(app.focus().focus_visible());
    let downstream = app.publish_surface(&context)
        .unwrap_or_else(|error| panic!("downstream styled publication: {error:?}"));
    assert!(downstream.frame().nodes().iter()
        .find(|node| node.id() == &custom_id)
        .unwrap_or_else(|| unreachable!("custom widget remains published"))
        .computed_style().outline().is_some());
}
