#![allow(refining_impl_trait)]
//! #485: distinguish select-only widget claims from generic M4 keyboard defaults.
//! Characterization only; accepted M11 conformance still owns the intended policy.

use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, LogicalPoint, NoHostProtocol, PhysicalKey,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    SceneShape, SemanticCommand, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, HitTestScene, InputArbitrationRecord,
    LogicalSize, MountedNodeId, PumpBudget, RuntimeConfig, SurfaceBuildContext, TraceConfig,
    UiDefaultDisposition, UiInputClaimReason, UiInputConflict, UiInputFinality,
};

enum Action {
    Popup(ComboBoxRequest),
    Choose(u8),
}

struct State {
    open: bool,
    chosen: u8,
}
struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        combo_box(
            "Select",
            ElementId::new("probe.popup").unwrap_or_else(|_| unreachable!("valid popup")),
            [
                option_item("One", state.chosen == 1)
                    .id("probe.one")
                    .on_activate(|| Action::Choose(1)),
                option_item("Two", state.chosen == 2)
                    .id("probe.two")
                    .on_activate(|| Action::Choose(2)),
            ],
            Action::Popup,
        )
        .id("probe.owner")
        .open(state.open)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Popup(ComboBoxRequest::Open) => state.open = true,
            Action::Popup(ComboBoxRequest::Close) => state.open = false,
            Action::Choose(value) => {
                state.chosen = value;
                state.open = false;
            }
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|error| unreachable!("canonical work settles: {error:?}"))
            .report()
            .is_quiescent()
    );
}

fn mount_focused(trace_capacity: usize) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            open: true,
            chosen: 0,
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    settle(&mut runtime);
    let authored = ElementId::new("probe.owner").unwrap_or_else(|_| unreachable!());
    let owner: MountedNodeId = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("one current owner"))
        .id()
        .clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request admitted"));
    settle(&mut runtime);
    runtime
}

#[test]
fn generic_focus_navigation_claim_is_not_an_explicit_combo_widget_claim() {
    let cases = [
        (
            KeyboardPhase::Down,
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
            Some(UiInputClaimReason::ExplicitWidgetClaim),
            UiDefaultDisposition::Prevented,
        ),
        (
            KeyboardPhase::Down,
            KeyModifiers::CONTROL,
            KeyboardCompositionState::Inactive,
            Some(UiInputClaimReason::FocusNavigation),
            UiDefaultDisposition::Queued,
        ),
        (
            KeyboardPhase::Up,
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
            None,
            UiDefaultDisposition::None,
        ),
        (
            KeyboardPhase::Down,
            KeyModifiers::NONE,
            KeyboardCompositionState::Active,
            Some(UiInputClaimReason::FocusNavigation),
            UiDefaultDisposition::Queued,
        ),
    ];
    for trace_capacity in [0, 1, 512] {
        for (phase, modifiers, composition, reason, disposition) in cases {
            let mut runtime = mount_focused(trace_capacity);
            let input = KeyboardEvent::new(
                phase,
                PhysicalKey::ArrowDown,
                LogicalKey::ArrowDown,
                modifiers,
                false,
                KeyLocation::Standard,
                composition,
                None,
            );
            let receipt = runtime
                .submit_keyboard(input)
                .unwrap_or_else(|_| unreachable!("native input admitted"));
            let batch = runtime
                .pump(PumpBudget::new(1, 0, 0, 0))
                .unwrap_or_else(|error| unreachable!("native input reached: {error:?}"));
            let [InputArbitrationRecord::InputSettled(settled)] = batch.ordered_records() else {
                unreachable!("one settled keyboard occurrence")
            };
            assert_eq!(settled.sequence(), receipt.sequence());
            let UiInputFinality::Committed(facts) = settled.finality() else {
                unreachable!("reached input commits")
            };
            assert_eq!(
                facts.reasons(),
                reason.as_slice(),
                "phase={phase:?} modifiers={modifiers:?} composition={composition:?} trace={trace_capacity}"
            );
            assert_eq!(facts.default_disposition(), disposition);
            assert_eq!(
                facts.conflict(),
                if reason.is_some() {
                    UiInputConflict::ExclusiveUi
                } else {
                    UiInputConflict::ObservedNonexclusive
                }
            );
            assert_eq!(runtime.state().chosen, 0);
        }
    }
}

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn option_two(runtime: &mut AppRuntime<App>) -> MountedNodeId {
    let id = ElementId::new("probe.two").unwrap_or_else(|_| unreachable!("valid option ID"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("mounted real OptionItem"))
        .id()
        .clone()
}

fn published(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!("finite surface")),
        ))
        .unwrap_or_else(|error| unreachable!("correlated hit publication: {error:?}"))
}

fn displayed_option_center(scene: &HitTestScene, option: &MountedNodeId) -> LogicalPoint {
    let region = scene
        .regions()
        .iter()
        .find(|region| region.target() == option && matches!(region.shape(), SceneShape::Rect(_)))
        .unwrap_or_else(|| unreachable!("option has a projected rectangular physical hit region"));
    let SceneShape::Rect(bounds) = region.shape() else {
        unreachable!("selected a rectangle")
    };
    assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
    let local = LogicalPoint::new(
        bounds.width().mul_add(0.5, bounds.x()),
        bounds.height().mul_add(0.5, bounds.y()),
    )
    .unwrap_or_else(|_| unreachable!("finite region center"));
    region
        .local_to_surface()
        .transform_point(local)
        .unwrap_or_else(|| unreachable!("non-singular accepted Option projection"))
}

#[test]
fn published_hit_region_activates_the_exact_option_without_focus_transfer() {
    let mut runtime = mount_focused(512);
    runtime
        .register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!("fixture font accepted"));
    runtime
        .set_text_generic_family_mapping(
            GenericFontFamily::SansSerif,
            &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
        )
        .unwrap_or_else(|_| unreachable!("fixture font mapping accepted"));
    settle(&mut runtime);
    let option = option_two(&mut runtime);
    let initial = published(&mut runtime);
    let owner = runtime.focus().focused_node().cloned();
    let point = displayed_option_center(initial.hit_test_scene(), &option);
    assert_eq!(
        initial.hit_test_scene().target_at(point),
        Some(&option),
        "the input authority must resolve this exact physical Option generation"
    );
    let pointer = PointerId::new(331).unwrap_or_else(|| unreachable!("nonzero pointer"));
    runtime
        .submit_pointer(PointerEvent::new(
            pointer,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            point,
            initial.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("Move admitted"));
    settle(&mut runtime);

    let hovered = published(&mut runtime);
    let down_point = displayed_option_center(hovered.hit_test_scene(), &option);
    assert_eq!(
        hovered.hit_test_scene().target_at(down_point),
        Some(&option)
    );
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                down_point,
                hovered.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("Down admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().chosen, 0);

    let pressed = published(&mut runtime);
    let up_point = displayed_option_center(pressed.hit_test_scene(), &option);
    assert_eq!(pressed.hit_test_scene().target_at(up_point), Some(&option));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                up_point,
                pressed.input_context().clone(),
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("Up admitted"));
    settle(&mut runtime);
    assert_eq!(
        runtime.state().chosen,
        2,
        "canonical Option callback must choose Two"
    );
    assert!(!runtime.state().open);
    assert_eq!(runtime.focus().focused_node(), owner.as_ref());
}
