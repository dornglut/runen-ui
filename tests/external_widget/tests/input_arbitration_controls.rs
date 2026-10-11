//! Recognized range-control keys claim only their native envelope; effects remain queued.

use runenui_core::{
    Axis, CommandOrigin, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LayoutDimension, LayoutStyle, LogicalKey, NoHostProtocol,
    OverflowPolicy, OverflowStyle, PhysicalKey, ScrollControlBinding, SemanticCommand,
    SemanticNumber, Slider, Splitter, SplitterRequest, StyleEnvironment, UiApp, View, scroll_bar,
    scroll_container, text,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputPumpBatch, LogicalSize, MountedNodeId, PumpBudget,
    RuntimeConfig, SurfaceBuildContext, TraceConfig, UiInputClaimReason, UiInputConflict,
    UiInputFinality,
};

const FULL: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);
#[derive(Clone, Copy, Debug)]
enum Control {
    Slider,
    Splitter,
    ScrollBar,
}
const CONTROLS: [Control; 3] = [Control::Slider, Control::Splitter, Control::ScrollBar];
struct State {
    control: Control,
    value: f64,
    updates: usize,
}
enum Action {
    Value(SemanticNumber),
    Resize(SplitterRequest),
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &State) -> impl View<Action> {
        match state.control {
            Control::Slider => Slider::new("Volume", 0.0, 100.0, state.value, 5.0)
                .unwrap_or_else(|_| unreachable!("checked range"))
                .id("control")
                .on_change(Action::Value)
                .into_element(),
            Control::Splitter => Splitter::new("Pane", 0.0, 100.0, state.value, 5.0)
                .unwrap_or_else(|_| unreachable!("checked range"))
                .id("control")
                .on_resize(Action::Resize)
                .into_element(),
            Control::ScrollBar => {
                let overflow = OverflowStyle::new(OverflowPolicy::Scroll, OverflowPolicy::Scroll);
                scroll_container(
                    text("Content").with_layout(
                        LayoutStyle::default()
                            .with_height(LayoutDimension::Length(300_u16.into()))
                            .with_width(LayoutDimension::Length(100_u16.into())),
                    ),
                    overflow,
                )
                .id("owner")
                .with_layout(
                    LayoutStyle::default()
                        .with_height(LayoutDimension::Length(100_u16.into()))
                        .with_width(LayoutDimension::Length(100_u16.into()))
                        .with_overflow(overflow),
                )
                .scroll_bar(
                    scroll_bar(
                        "Vertical",
                        ScrollControlBinding::new(Axis::Vertical, 5_u16.into())
                            .unwrap_or_else(|_| unreachable!("checked binding")),
                        10_u16,
                        20_u16,
                    )
                    .id("control"),
                )
                .into_element()
            }
        }
    }
    fn update(state: &mut State, action: Action) {
        state.value = match action {
            Action::Value(value) | Action::Resize(SplitterRequest::SetValue(value)) => value.get(),
            Action::Resize(_) => unreachable!("Home/End emit absolute proposals"),
        };
        state.updates += 1;
    }
}
fn target(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted fixture"))
        .id()
        .clone()
}
fn runtime(control: Control, trace_capacity: usize) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            control,
            value: 50.0,
            updates: 0,
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::new(320_u16.into(), 240_u16.into()),
        ))
        .unwrap_or_else(|_| unreachable!("retained metrics"));
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let control = target(&mut runtime, "control");
    runtime
        .submit_command(
            control.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert_eq!(runtime.focus().focused_node(), Some(&control));
    runtime
}
fn key(logical: LogicalKey, modifiers: KeyModifiers, phase: KeyboardPhase) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        PhysicalKey::Code(format!("{logical:?}")),
        logical,
        modifiers,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}
fn assert_settlement(batch: &InputPumpBatch, exclusive: bool) {
    let [InputArbitrationRecord::InputSettled(input)] = batch.ordered_records() else {
        unreachable!("one native envelope without ownership change");
    };
    let UiInputFinality::Committed(facts) = input.finality() else {
        unreachable!()
    };
    assert_eq!(
        facts.conflict(),
        if exclusive {
            UiInputConflict::ExclusiveUi
        } else {
            UiInputConflict::ObservedNonexclusive
        }
    );
    assert_eq!(facts.default_prevented(), exclusive);
    assert_eq!(facts.propagation_stopped(), exclusive);
    assert_eq!(
        facts.reasons(),
        if exclusive {
            &[UiInputClaimReason::ExplicitWidgetClaim][..]
        } else {
            &[]
        }
    );
}
fn finish(runtime: &mut AppRuntime<App>, capacity: usize) {
    assert_eq!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .ordered_records(),
        []
    );
    match capacity {
        0 => assert_eq!(runtime.trace().len(), 0),
        1 => assert_eq!(runtime.trace().len(), 1),
        _ => assert!(runtime.trace().len() <= capacity),
    }
}
#[test]
fn range_control_claims_precede_the_separate_value_or_scroll_command() {
    for capacity in [0, 1, 512] {
        for control in CONTROLS {
            for logical in [LogicalKey::Home, LogicalKey::End] {
                let expected = if logical == LogicalKey::Home {
                    0.0
                } else {
                    100.0
                };
                let mut runtime = runtime(control, capacity);
                let before = runtime.input_ownership().unwrap_or_else(|_| unreachable!());
                let receipt = runtime
                    .submit_keyboard(key(logical, KeyModifiers::NONE, KeyboardPhase::Down))
                    .unwrap_or_else(|_| unreachable!());
                let native = runtime.pump(ONE).unwrap_or_else(|_| unreachable!());
                assert_settlement(&native, true);
                assert_eq!(native.final_ownership(), &before);
                assert_eq!(native.processed_through(), Some(receipt.sequence()));
                assert_eq!(native.report().remaining_queued_envelopes(), 1);
                assert_eq!(runtime.state().value, 50.0);
                assert_eq!(runtime.state().updates, 0);
                let effect = runtime.pump(ONE).unwrap_or_else(|_| unreachable!());
                assert_eq!(effect.ordered_records(), []);
                assert!(
                    effect
                        .processed_through()
                        .is_some_and(|seq| seq > receipt.sequence())
                );
                if matches!(control, Control::ScrollBar) {
                    let owner = target(&mut runtime, "owner");
                    let offset = runtime
                        .index()
                        .nodes()
                        .iter()
                        .find(|node| node.id() == &owner)
                        .unwrap_or_else(|| unreachable!())
                        .interaction()
                        .scroll_offset()
                        .1;
                    assert_eq!(offset > 0.0, expected > 0.0);
                    assert_eq!(runtime.state().updates, 0);
                } else {
                    assert_eq!(runtime.state().value, expected);
                    assert_eq!(runtime.state().updates, 1);
                }
                finish(&mut runtime, capacity);
                assert_eq!(
                    native.final_ownership(),
                    &before,
                    "retained batch stays immutable"
                );
            }
        }
    }
}
#[test]
fn unsupported_range_keys_do_not_inherit_the_claim_or_queue_an_effect() {
    for capacity in [0, 1, 512] {
        for control in CONTROLS {
            for (modifiers, phase) in [
                (KeyModifiers::CONTROL, KeyboardPhase::Down),
                (KeyModifiers::SHIFT, KeyboardPhase::Down),
                (KeyModifiers::ALT, KeyboardPhase::Down),
                (KeyModifiers::META, KeyboardPhase::Down),
                (KeyModifiers::NONE, KeyboardPhase::Up),
            ] {
                let mut runtime = runtime(control, capacity);
                runtime
                    .submit_keyboard(key(LogicalKey::Home, modifiers, phase))
                    .unwrap_or_else(|_| unreachable!());
                let native = runtime.pump(ONE).unwrap_or_else(|_| unreachable!());
                assert_settlement(&native, false);
                assert_eq!(native.report().remaining_queued_envelopes(), 0);
                assert_eq!(runtime.state().updates, 0);
                finish(&mut runtime, capacity);
            }
        }
    }
}
