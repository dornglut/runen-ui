use runenui_core::{
    Element, View, Widget, WidgetActivation, WidgetSemanticProof, button, children, column, text,
};
use runenui_runtime::{ActivationResult, AppRuntime, RuntimeEvent, UiApp};

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    count: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
    Reset,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    fn root(state: &State) -> Element<Action> {
        column(children![
            text(state.count.to_string()).id("value"),
            button("+").id("increment").on_press(Action::Increment),
            button("locked")
                .id("locked")
                .on_press(Action::Increment)
                .disabled(),
            button("reset").id("reset").on_press(Action::Reset),
        ])
        .into_element()
    }
    fn update(state: &mut State, action: Action) {
        match action {
            Action::Increment => state.count += 1,
            Action::Reset => state.count = 0,
        }
    }
}

#[test]
fn dispatch_activation_disabled_and_trace_regressions_hold() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<App>::mount(State { count: 0 });
    assert_eq!(runtime.activate("locked"), ActivationResult::Disabled);
    assert_eq!(runtime.activate("increment"), ActivationResult::Dispatched);
    assert_eq!(runtime.state().count, 1);
    assert_eq!(runtime.root().children()[0].semantics().name(), "1");
    assert_eq!(
        runtime.trace().events(),
        &[
            RuntimeEvent::Mounted,
            RuntimeEvent::ActionDispatched,
            RuntimeEvent::StateUpdated,
            RuntimeEvent::RootRebuilt,
        ]
    );
    let targeted = runtime
        .trace()
        .records()
        .get(1)
        .and_then(runenui_runtime::TraceRecord::target)
        .ok_or("target")?;
    assert_eq!(
        targeted.authored_id().map(runenui_core::ElementId::as_str),
        Some("increment")
    );
    assert_eq!(runtime.activate(" "), ActivationResult::InvalidId);
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
enum NonCloneAction {
    Increment,
}

struct NonCloneApp;
impl UiApp for NonCloneApp {
    type State = i32;
    type Action = NonCloneAction;
    fn root(_: &i32) -> Element<NonCloneAction> {
        button("+")
            .on_press(NonCloneAction::Increment)
            .into_element()
    }
    fn update(state: &mut i32, _: NonCloneAction) {
        *state += 1;
    }
}

#[test]
fn non_clone_actions_support_mount_and_direct_dispatch() {
    let mut runtime = AppRuntime::<NonCloneApp>::mount(0);
    assert_eq!(
        runtime.activate_node(runtime.index().nodes()[0].id()),
        ActivationResult::Dispatched
    );
    assert_eq!(*runtime.state(), 1);
    assert_eq!(
        runtime.activate_node(runtime.index().nodes()[0].id()),
        ActivationResult::Dispatched
    );
    assert_eq!(*runtime.state(), 2);
    runtime.dispatch(NonCloneAction::Increment);
    assert_eq!(*runtime.state(), 3);
}

#[derive(Debug)]
struct ExhaustedActionSource;

impl Widget<NonCloneAction> for ExhaustedActionSource {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn activation(&self) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn activate(&mut self) -> Option<NonCloneAction> {
        None
    }
    fn semantics(&self) -> WidgetSemanticProof {
        WidgetSemanticProof::new("control", "Exhausted").with_action("activate")
    }
}

struct ExhaustedApp;
impl UiApp for ExhaustedApp {
    type State = ();
    type Action = NonCloneAction;
    fn root((): &()) -> Element<Self::Action> {
        Element::new(ExhaustedActionSource)
    }
    fn update((): &mut (), _: Self::Action) {}
}

#[test]
fn failed_runtime_extraction_does_not_change_actionable_facts() {
    let mut runtime = AppRuntime::<ExhaustedApp>::mount(());
    let node = runtime.index().nodes()[0].id();
    assert!(runtime.index().nodes()[0].is_focusable());
    assert_eq!(runtime.root().semantics().action_intent(), Some("activate"));
    assert_eq!(runtime.activate_node(node), ActivationResult::NoAction);
    assert!(runtime.index().nodes()[0].is_focusable());
    assert_eq!(runtime.root().semantics().action_intent(), Some("activate"));
}
