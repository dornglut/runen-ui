#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, NoHostProtocol, SemanticCommand, UiApp, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetSemanticProof, button, children, column,
    text,
};
use runenui_runtime::{AppRuntime, PumpBudget, TraceRecordKind};

#[derive(Debug, Eq, PartialEq)]
enum Action {
    Increment,
}
struct App;
impl UiApp for App {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
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
    assert_eq!(
        runtime
            .pump(PumpBudget::new(4, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        4
    );
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
fn routed_activation_queues_fresh_non_clone_actions() {
    let mut runtime = AppRuntime::<App>::mount(0);
    let authored_id = runenui_core::ElementId::new("increment")
        .unwrap_or_else(|_| unreachable!("the test identifier is valid"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored_id))
        .unwrap_or_else(|| unreachable!("the increment node is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    assert_eq!(runtime.state(), &0);
    assert_eq!(
        runtime
            .pump(PumpBudget::new(7, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        7
    );
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
        _: &mut WidgetActivationContext<NonCloneAction>,
    ) -> WidgetActivationOutput<NonCloneAction> {
        *state += 1;
        self.0.take().map_or_else(
            WidgetActivationOutput::changed,
            WidgetActivationOutput::changed_with_action,
        )
    }
    fn semantics(&self, _: &Self::State) -> WidgetSemanticProof {
        WidgetSemanticProof::new("custom", "non-clone").with_action("activate")
    }
}
struct NonCloneApp;
impl UiApp for NonCloneApp {
    type State = Vec<String>;
    type Action = NonCloneAction;
    type HostProtocol = NoHostProtocol;
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
    runtime
        .submit_command(id, SemanticCommand::Activate, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    assert!(runtime.state().is_empty());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        3
    );
    assert_eq!(runtime.state(), &["owned"]);
}
