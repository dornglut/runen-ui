#![allow(refining_impl_trait)]
//! #485: distinguish select-only widget claims from generic M4 keyboard defaults.
//! Characterization only; accepted M11 conformance still owns the intended policy.

use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand, UiApp,
    View, combo_box, option_item,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, MountedNodeId, PumpBudget, RuntimeConfig, TraceConfig,
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
