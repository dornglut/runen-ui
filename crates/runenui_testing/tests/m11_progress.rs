use core::num::NonZeroUsize;

use runenui_core::{
    NoHostProtocol, SemanticAction, SemanticNumber, SemanticRole, UiApp, View, progress,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Debug)]
struct Model {
    current: Option<SemanticNumber>,
    value_text: Option<String>,
}
enum Action {
    SetCurrent(Option<SemanticNumber>),
    SetValueText(Option<String>),
}
struct ProgressApp;

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("fixture finite"))
}

impl UiApp for ProgressApp {
    type State = Model;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(model: &Self::State) -> impl View<Self::Action> {
        let mut control = progress(number(0.0), number(100.0), model.current)
            .unwrap_or_else(|_| unreachable!("finite in-range values"))
            .accessible_name("Download")
            .id("progress");
        if let Some(value_text) = &model.value_text {
            control = control.with_value_text(value_text.clone())
                .unwrap_or_else(|_| unreachable!("value text needs determinate progress"));
        }
        control
    }

    fn update(model: &mut Self::State, action: Self::Action) {
        match action {
            Action::SetCurrent(current) => {
                model.current = current;
                if current.is_none() {
                    model.value_text = None;
                }
            }
            Action::SetValueText(text) => model.value_text = text,
        }
    }
}

fn settle(harness: &mut TestHarness<ProgressApp>) {
    let budget = SettleBudget::new(
        NonZeroUsize::new(12).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    );
    assert_eq!(harness.run_until_idle(budget).outcome(), SettleOutcome::Idle);
}

fn current(h: &TestHarness<ProgressApp>) -> Option<SemanticNumber> {
    let t = h.unique_semantic_target(
        &SemanticQuery::new().with_role(SemanticRole::Progress).with_name("Download"),
    ).unwrap_or_else(|error| unreachable!("one progress node: {error:?}"));
    let snapshot = h.semantic_snapshot().unwrap_or_else(|_| unreachable!("published"));
    let n = snapshot.node(t.node_id()).unwrap_or_else(|| unreachable!("node published"));
    assert!(n.supported_actions().is_empty());
    assert!(!n.state().disabled());
    n.range().and_then(runenui_core::SemanticRange::current)
}

#[test]
fn progress_is_application_owned_and_indeterminate_has_no_fabricated_numeric_value() {
    let mut h = TestHarness::<ProgressApp>::mount(Model {
        current: Some(number(25.0)),
        value_text: Some("One quarter".to_owned()),
    });
    assert!(h.publish().is_ok());
    assert_eq!(current(&h), Some(number(25.0)));
    let initial_scene = h.publish().unwrap_or_else(|_| unreachable!("publish"))
        .paint_scene().items().len();
    assert_eq!(initial_scene, 2);
    let revision = h.semantic_snapshot().unwrap_or_else(|_| unreachable!()).revision();
    let target = h.unique_semantic_target(&SemanticQuery::new().with_role(SemanticRole::Progress))
        .unwrap_or_else(|_| unreachable!("Progress target"));
    assert!(h.submit_semantic_action(&target, SemanticAction::Activate).is_err());
    settle(&mut h);

    assert!(h.submit_action(Action::SetValueText(Some("25 percent".to_owned()))).is_ok());
    settle(&mut h);
    assert!(h.publish().is_ok());
    assert_eq!(h.publish().unwrap_or_else(|_| unreachable!("publish")).paint_scene().items().len(), 2);
    assert!(h.semantic_snapshot().unwrap_or_else(|_| unreachable!()).revision() > revision);
    assert_eq!(current(&h), Some(number(25.0)));

    assert!(h.submit_action(Action::SetCurrent(None)).is_ok());
    settle(&mut h);
    assert!(h.publish().is_ok());
    assert_eq!(current(&h), None);
    assert_eq!(h.publish().unwrap_or_else(|_| unreachable!("publish")).paint_scene().items().len(), 1);

    assert!(h.submit_action(Action::SetCurrent(Some(number(75.0)))).is_ok());
    settle(&mut h);
    assert!(h.publish().is_ok());
    assert_eq!(current(&h), Some(number(75.0)));
    assert_eq!(h.publish().unwrap_or_else(|_| unreachable!("publish")).paint_scene().items().len(), 2);
}
