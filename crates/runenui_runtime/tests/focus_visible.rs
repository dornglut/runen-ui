#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, ElementId, HitContribution, HitContributionContext, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticAction,
    SemanticActionRequest, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, SemanticCommand, StyleEnvironment, UiApp, View,
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

fn settle(runtime: &mut AppRuntime<App>) {
    assert!(runtime.pump(PumpBudget::new(64, 64, 64, 64)).is_quiescent());
}

fn mount() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(());
    settle(&mut runtime);
    runtime
}

fn mounted_id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
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

fn focus(runtime: &mut AppRuntime<App>, node: &MountedNodeId, origin: CommandOrigin) {
    assert!(runtime
        .submit_command(node.clone(), SemanticCommand::RequestFocus, origin)
        .is_ok());
    settle(runtime);
    assert_eq!(runtime.focus().focused_node(), Some(node));
}

fn pointer_focus(runtime: &mut AppRuntime<App>, name: &str, pointer: u64) {
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
    app.shutdown();
    assert!(app.focus().focused_node().is_none());
    assert!(!app.focus().focus_visible());
}
