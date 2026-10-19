//! Public temporal discriminator for #474: an Escape-owned presentation must not
//! redirect its delayed CancelOrBack to an unrelated later presentation.

use runenui_core::{
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    Menu, MenuButton, MenuItem, NoHostProtocol, PhysicalKey, StyleEnvironment, UiApp, View, column,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, LogicalSize, PumpBudget, SurfaceBuildContext,
    UiInputFinality,
};

const FULL: PumpBudget = PumpBudget::new(128, 128, 128, 128);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Clone, Copy, Debug)]
enum Action {
    ShowQ,
    DismissP,
    DismissQ,
    Noop,
}

#[derive(Clone, Debug)]
struct State {
    p_open: bool,
    q_open: bool,
    p_dismissals: usize,
    q_dismissals: usize,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> impl View<Action> {
        let mut p = MenuButton::new("P", state.p_open).id("p-trigger");
        if state.p_open {
            p = p.with_submenu(
                Menu::new(vec![
                    MenuItem::new("P item")
                        .id("p-item")
                        .on_activate(|| Action::Noop)
                        .into_element(),
                ])
                .id("p-menu")
                .on_dismiss(|_| Action::DismissP),
                true,
            );
        }
        let mut children = vec![p.into_element()];
        if state.q_open {
            children.push(
                MenuButton::new("Q", true)
                    .id("q-trigger")
                    .with_submenu(
                        Menu::new(vec![
                            MenuItem::new("Q item")
                                .id("q-item")
                                .on_activate(|| Action::Noop)
                                .into_element(),
                        ])
                        .id("q-menu")
                        .on_dismiss(|_| Action::DismissQ),
                        true,
                    )
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut State, action: Action) {
        match action {
            Action::ShowQ => state.q_open = true,
            Action::DismissP => {
                state.p_open = false;
                state.p_dismissals += 1;
            }
            Action::DismissQ => {
                state.q_open = false;
                state.q_dismissals += 1;
            }
            Action::Noop => {}
        }
    }
}

#[test]
fn delayed_escape_from_p_must_not_dismiss_newly_published_q() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    );
    let mut runtime = AppRuntime::<App>::mount(State {
        p_open: true,
        q_open: false,
        p_dismissals: 0,
        q_dismissals: 0,
    });
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("P publication"));
    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("initial focus"))
            .report()
            .is_quiescent()
    );
    let p_item = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "p-item"))
        .unwrap_or_else(|| unreachable!("published P item"))
        .id()
        .clone();
    assert_eq!(runtime.focus().focused_node(), Some(&p_item));

    let escape = KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Escape,
        LogicalKey::Escape,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    );
    let receipt = runtime
        .submit_keyboard(escape)
        .unwrap_or_else(|_| unreachable!("focused Escape admission"));
    runtime
        .submit_action(Action::ShowQ)
        .unwrap_or_else(|_| unreachable!("Q appearance accepted behind Escape"));

    let key_batch = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one keyboard envelope"));
    assert_eq!(key_batch.report().processed_envelopes(), 1);
    assert!(key_batch.ordered_records().iter().any(|record| {
        matches!(
            record,
            InputArbitrationRecord::InputSettled(settlement)
                if settlement.sequence() == receipt.sequence()
                    && matches!(settlement.finality(), UiInputFinality::Committed(_))
        )
    }));
    assert!(!runtime.state().q_open);

    let action_batch = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("one Q application action"));
    assert_eq!(action_batch.report().processed_envelopes(), 1);
    assert!(runtime.state().p_open);
    assert!(runtime.state().q_open);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("Q publication precedes delayed default"));

    assert!(
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("drain delayed semantic command"))
            .report()
            .is_quiescent()
    );
    assert!(
        runtime.state().q_open,
        "an Escape admitted under P must not dismiss unrelated Q published before its delayed CancelOrBack"
    );
    assert_eq!(
        runtime.state().q_dismissals,
        0,
        "Q must not receive a P-origin dismissal"
    );
}
