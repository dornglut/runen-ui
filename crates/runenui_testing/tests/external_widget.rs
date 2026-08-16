use core::num::NonZeroUsize;
use std::{cell::RefCell, rc::Rc};

use runenui_core::{CommandOrigin, Element, ElementId, NoHostProtocol, SemanticCommand, UiApp};
use runenui_runtime::{InputModality, MountedNodeId, PumpBudget, SurfaceFrame};
use runenui_testing::{SettleBudget, SettleOutcome, TestHarness};

#[path = "../../../tests/external_widget/src/lib.rs"]
mod external_fixture;

use external_fixture::{ExternalFocusFact, external_focus_panel};

struct State {
    log: Rc<RefCell<Vec<ExternalFocusFact>>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<()> {
        external_focus_panel(Rc::clone(&state.log))
    }

    fn update(_: &mut Self::State, (): ()) {}
}

const fn settle_budget() -> SettleBudget {
    SettleBudget::new(NonZeroUsize::MIN, PumpBudget::new(64, 64, 64, 64))
}

fn authored_node(frame: &SurfaceFrame, authored: &str) -> Option<MountedNodeId> {
    let Ok(authored) = ElementId::new(authored) else {
        return None;
    };
    frame
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .map(|node| node.id().clone())
}

#[test]
fn genuine_downstream_focus_widget_uses_public_harness_and_controller_commands() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut harness = TestHarness::<App>::mount(State {
        log: Rc::clone(&log),
    });

    let Some((a, b)) = (|| {
        let publication = harness.publish().ok()?;
        Some((
            authored_node(publication.frame(), "focus.a")?,
            authored_node(publication.frame(), "focus.b")?,
        ))
    })() else {
        return;
    };

    assert!(
        harness
            .submit_command(
                a.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.focus().focused_node(), Some(&a));

    log.borrow_mut().clear();
    assert!(
        harness
            .submit_command(a, SemanticCommand::FocusNext, CommandOrigin::controller())
            .is_ok()
    );
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.focus().focused_node(), Some(&b));
    assert_eq!(harness.focus().modality(), Some(InputModality::Controller));
    assert!(!log.borrow().is_empty());
}
