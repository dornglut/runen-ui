use runenui_core::{
    Element, View, Widget, WidgetActivation, WidgetActivationContext, WidgetSemanticProof, button,
    children, column, text,
};
use runenui_runtime::{ActivationResult, AppRuntime, PumpBudget, TraceRecordKind, UiApp};

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Increment,
}
struct App;
impl UiApp for App {
    type State = usize;
    type Action = Action;
    fn root(state: &usize) -> Element<Action> {
        column(children![
            text(state.to_string()).id("value").key("value"),
            button("+")
                .id("increment")
                .key("increment")
                .on_activate(|| Action::Increment)
        ])
        .key("root")
        .into_element()
    }
    fn update(state: &mut usize, _: Action) {
        *state += 1;
    }
}

#[test]
fn queued_action_reconciles_without_replacing_compatible_nodes() {
    let mut runtime = AppRuntime::<App>::mount(0);
    let ids: Vec<_> = runtime
        .index()
        .nodes()
        .iter()
        .map(|n| n.id().clone())
        .collect();
    assert_eq!(runtime.reconciliation_report().generation().get(), 1);
    runtime
        .submit_action(Action::Increment)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(runtime.state(), &0);
    assert_eq!(runtime.pump(PumpBudget::new(1)).processed_envelopes(), 1);
    assert_eq!(runtime.state(), &1);
    let after: Vec<_> = runtime
        .index()
        .nodes()
        .iter()
        .map(|n| n.id().clone())
        .collect();
    assert_eq!(after, ids);
    assert_eq!(runtime.reconciliation_report().updated_count(), 3);
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::TreeReconciled))
    );
}

#[test]
fn mounted_activation_queues_fresh_non_clone_actions() {
    let mut runtime = AppRuntime::<App>::mount(0);
    assert!(matches!(
        runtime.activate("increment"),
        ActivationResult::Queued { .. }
    ));
    assert!(matches!(
        runtime.activate("increment"),
        ActivationResult::Queued { .. }
    ));
    assert_eq!(runtime.state(), &0);
    assert_eq!(runtime.pump(PumpBudget::new(2)).processed_envelopes(), 2);
    assert_eq!(runtime.state(), &2);
}

#[derive(Debug, Eq, PartialEq)]
struct NonCloneAction(String);
#[derive(Debug)]
struct NonCloneWidget(Option<NonCloneAction>);
impl Widget<NonCloneAction> for NonCloneWidget {
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
        _: &mut WidgetActivationContext,
    ) -> Option<NonCloneAction> {
        *state += 1;
        self.0.take()
    }
    fn semantics(&self, _: &Self::State) -> WidgetSemanticProof {
        WidgetSemanticProof::new("custom", "non-clone").with_action("activate")
    }
}
struct NonCloneApp;
impl UiApp for NonCloneApp {
    type State = Vec<String>;
    type Action = NonCloneAction;
    fn root(_: &Self::State) -> Element<Self::Action> {
        Element::new(NonCloneWidget(Some(NonCloneAction("owned".into())))).key("non-clone")
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        state.push(action.0);
    }
}

#[test]
fn non_clone_actions_remain_supported() {
    let mut runtime = AppRuntime::<NonCloneApp>::mount(Vec::new());
    let id = runtime.index().nodes()[0].id().clone();
    assert!(matches!(
        runtime.activate_node(&id),
        ActivationResult::Queued { .. }
    ));
    assert!(runtime.state().is_empty());
    assert_eq!(runtime.pump(PumpBudget::new(1)).processed_envelopes(), 1);
    assert_eq!(runtime.state(), &["owned"]);
}
