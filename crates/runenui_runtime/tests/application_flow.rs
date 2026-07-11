use runenui_core::{Element, ElementKind, IntoElement, button, children, column, text};
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
    let ElementKind::Container(root) = runtime.root().kind() else {
        return Err("root");
    };
    let ElementKind::Text(value) = root.children()[0].kind() else {
        return Err("value");
    };
    assert_eq!(value.content(), "1");
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
    runtime.dispatch(NonCloneAction::Increment);
    assert_eq!(*runtime.state(), 1);
}
