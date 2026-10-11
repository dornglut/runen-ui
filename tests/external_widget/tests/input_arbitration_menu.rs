//! Standard focus-group controls claim keys before the separate focus-command boundary.

use runenui_core::{
    CommandOrigin, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, ListBox, LogicalKey, Menu, MenuBar, MenuButton, MenuItem, NoHostProtocol,
    OptionItem, PhysicalKey, SemanticCommand, StyleEnvironment, Tab, TabList, Toolbar, TreeItem,
    TreeView, UiApp, View, WorkSequence, button,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputPumpBatch, LogicalSize, MountedNodeId, PumpBudget,
    RuntimeConfig, SurfaceBuildContext, TraceConfig, TraceRecordKind, UiDefaultDisposition,
    UiInputClaimReason, UiInputConflict, UiInputFamily, UiInputFinality, UiInputSettlement,
};

const FULL: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Clone, Copy, Debug)]
enum Control {
    Menu,
    MenuBar,
    ListBox,
    TabList,
    Toolbar,
    TreeView,
}
const CONTROLS: [Control; 6] = [
    Control::Menu,
    Control::MenuBar,
    Control::ListBox,
    Control::TabList,
    Control::Toolbar,
    Control::TreeView,
];

struct State {
    control: Control,
    activations: usize,
}

struct App;
impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> impl View<()> {
        match state.control {
            Control::ListBox => {
                return ListBox::new([
                    OptionItem::new("Alpha", false)
                        .id("first")
                        .on_activate(|| ()),
                    OptionItem::new("Beta", false).id("last").on_activate(|| ()),
                ])
                .into_element();
            }
            Control::TabList => {
                return TabList::new([
                    Tab::new("Alpha", false).id("first").on_activate(|| ()),
                    Tab::new("Beta", false).id("last").on_activate(|| ()),
                ])
                .into_element();
            }
            Control::Toolbar => {
                return Toolbar::new(vec![
                    button("Alpha")
                        .id("first")
                        .on_activate(|| ())
                        .into_element(),
                    button("Beta").id("last").on_activate(|| ()).into_element(),
                ])
                .into_element();
            }
            Control::TreeView => {
                return TreeView::new([
                    TreeItem::new("alpha", "Alpha", [])
                        .id("first")
                        .on_request(|_| ()),
                    TreeItem::new("beta", "Beta", [])
                        .id("last")
                        .on_request(|_| ()),
                ])
                .into_element();
            }
            Control::Menu | Control::MenuBar => {}
        }
        let items = vec![
            MenuItem::new("Alpha")
                .id("first")
                .on_activate(|| ())
                .into_element(),
            MenuItem::new("Beta")
                .id("last")
                .on_activate(|| ())
                .into_element(),
        ];
        if matches!(state.control, Control::MenuBar) {
            MenuBar::new(items).id("menu").into_element()
        } else {
            MenuButton::new("File", true)
                .id("trigger")
                .with_submenu(Menu::new(items).id("menu"), true)
                .into_element()
        }
    }

    fn update(state: &mut State, (): ()) {
        state.activations += 1;
    }
}

fn target(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let id = ElementId::new(name).unwrap_or_else(|_| unreachable!("fixture ID"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("mounted menu node"))
        .id()
        .clone()
}

fn runtime(control: Control, trace_capacity: usize, focused: &str) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            control,
            activations: 0,
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("mount"));
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("retained menu geometry"));
    runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("presentation focus"));
    let focused = target(&mut runtime, focused);
    runtime
        .submit_command(
            focused.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("menu focus admission"));
    runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("menu focus"));
    assert_eq!(runtime.focus().focused_node(), Some(&focused));
    runtime
}

fn keyboard(logical: LogicalKey, modifiers: KeyModifiers, phase: KeyboardPhase) -> KeyboardEvent {
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

fn trace_bound(runtime: &AppRuntime<App>, capacity: usize) {
    match capacity {
        0 => assert_eq!(runtime.trace().len(), 0),
        1 => assert_eq!(runtime.trace().len(), 1, "trace really was truncated"),
        _ => assert!(runtime.trace().len() <= capacity),
    }
}

fn navigation_trace(
    runtime: &AppRuntime<App>,
    native: WorkSequence,
    navigation: WorkSequence,
    command: SemanticCommand,
) {
    assert!(runtime.trace().records().any(|record| record.work_sequence() == Some(native)
        && matches!(record.kind(), TraceRecordKind::DelegatedCommandCollected { command: actual } if *actual == command)));
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| record.work_sequence() == Some(native)
                && matches!(record.kind(), TraceRecordKind::DefaultPrevented))
    );
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| record.work_sequence() == Some(navigation)
                && matches!(
                    record.kind(),
                    TraceRecordKind::FocusTransitionCommitted { .. }
                ))
    );
}

fn settlement(batch: &InputPumpBatch) -> &UiInputSettlement {
    let [InputArbitrationRecord::InputSettled(input)] = batch.ordered_records() else {
        unreachable!("the native handler queues focus without changing ownership yet");
    };
    input
}

#[test]
fn standard_controls_home_end_claim_the_native_key_before_queued_focus_changes() {
    for trace_capacity in [0, 1, 512] {
        for control in CONTROLS {
            for (logical, from, to) in [
                (LogicalKey::Home, "last", "first"),
                (LogicalKey::End, "first", "last"),
            ] {
                let mut runtime = runtime(control, trace_capacity, from);
                let command = if logical == LogicalKey::Home {
                    SemanticCommand::FocusGroupFirst
                } else {
                    SemanticCommand::FocusGroupLast
                };
                let destination = target(&mut runtime, to);
                let before = runtime
                    .input_ownership()
                    .unwrap_or_else(|_| unreachable!("before"));
                let receipt = runtime
                    .submit_keyboard(keyboard(logical, KeyModifiers::NONE, KeyboardPhase::Down))
                    .unwrap_or_else(|_| unreachable!("native admission"));
                let input = runtime
                    .pump(ONE)
                    .unwrap_or_else(|_| unreachable!("native boundary"));
                let settled = settlement(&input);
                assert_eq!(settled.scope(), before.scope());
                assert_eq!(settled.sequence(), receipt.sequence());
                assert_eq!(settled.family(), UiInputFamily::Keyboard);
                assert_eq!(settled.device_id(), None);
                assert_eq!(settled.ownership_revision(), before.revision());
                let UiInputFinality::Committed(facts) = settled.finality() else {
                    unreachable!("ordinary menu input commits");
                };
                assert_eq!(
                    facts.conflict(),
                    UiInputConflict::ExclusiveUi,
                    "{control:?}"
                );
                assert_eq!(facts.reasons(), [UiInputClaimReason::ExplicitWidgetClaim]);
                assert!(facts.propagation_stopped() && facts.default_prevented());
                assert_eq!(facts.default_disposition(), UiDefaultDisposition::Prevented);
                assert_eq!(input.final_ownership(), &before);
                assert_eq!(input.processed_through(), Some(receipt.sequence()));
                assert_eq!(input.report().remaining_queued_envelopes(), 1);
                assert_eq!(runtime.state().activations, 0);
                let navigation = runtime
                    .pump(ONE)
                    .unwrap_or_else(|_| unreachable!("queued focus boundary"));
                assert!(
                    matches!(navigation.ordered_records(), [InputArbitrationRecord::OwnershipChanged(change)]
                    if change.before_revision() == before.revision()
                        && change.after().keyboard().focused_node() == Some(&destination))
                );
                assert!(
                    navigation
                        .processed_through()
                        .is_some_and(|sequence| sequence > receipt.sequence())
                );
                if trace_capacity == 512 {
                    navigation_trace(
                        &runtime,
                        receipt.sequence(),
                        navigation
                            .processed_through()
                            .unwrap_or_else(|| unreachable!("navigation processed")),
                        command,
                    );
                }
                assert_eq!(runtime.state().activations, 0);
                assert_eq!(
                    runtime
                        .pump(FULL)
                        .unwrap_or_else(|_| unreachable!("no replay"))
                        .ordered_records(),
                    []
                );
                trace_bound(&runtime, trace_capacity);
            }
        }
    }
}

#[test]
fn modified_and_key_up_home_end_do_not_inherit_standard_navigation_claims() {
    for trace_capacity in [0, 1, 512] {
        for control in CONTROLS {
            for logical in [LogicalKey::Home, LogicalKey::End] {
                for (modifiers, phase) in [
                    (KeyModifiers::CONTROL, KeyboardPhase::Down),
                    (KeyModifiers::SHIFT, KeyboardPhase::Down),
                    (KeyModifiers::ALT, KeyboardPhase::Down),
                    (KeyModifiers::META, KeyboardPhase::Down),
                    (KeyModifiers::NONE, KeyboardPhase::Up),
                ] {
                    let mut runtime = runtime(control, trace_capacity, "last");
                    let before = runtime
                        .input_ownership()
                        .unwrap_or_else(|_| unreachable!("before"));
                    let receipt = runtime
                        .submit_keyboard(keyboard(logical.clone(), modifiers, phase))
                        .unwrap_or_else(|_| unreachable!("native admission"));
                    let batch = runtime
                        .pump(ONE)
                        .unwrap_or_else(|_| unreachable!("unhandled boundary"));
                    let settled = settlement(&batch);
                    assert_eq!(settled.scope(), before.scope());
                    assert_eq!(settled.sequence(), receipt.sequence());
                    assert!(
                        matches!(settled.finality(), UiInputFinality::Committed(facts)
                    if facts.conflict() == UiInputConflict::ObservedNonexclusive
                        && facts.reasons().is_empty()
                        && !facts.default_prevented() && !facts.propagation_stopped()
                        && facts.default_disposition() == UiDefaultDisposition::None)
                    );
                    assert_eq!(batch.final_ownership(), &before);
                    assert_eq!(batch.report().remaining_queued_envelopes(), 0);
                    assert_eq!(runtime.state().activations, 0);
                    assert_eq!(
                        runtime
                            .pump(FULL)
                            .unwrap_or_else(|_| unreachable!("no replay"))
                            .ordered_records(),
                        []
                    );
                    trace_bound(&runtime, trace_capacity);
                }
            }
        }
    }
}
