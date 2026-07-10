use runenui_core::prelude::{button, column, row, text};
use runenui_runtime::prelude::{
    ActivationResult, AppRuntime, KeyModifiers, LogicalPoint, PointerActivationResult,
    PointerButton, PointerEvent, PointerPhase, RuntimeEvent, RuntimeNodeId, RuntimeNodeRef, UiApp,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State {
    count: i32,
}

struct MixedActivationApp;

impl UiApp for MixedActivationApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title").id("title"),
            button("Increment")
                .id("increment")
                .on_press(Action::Increment),
            row((
                button("Disabled")
                    .id("disabled")
                    .on_press(Action::Disabled)
                    .disabled(),
                button("No action").id("no-action"),
            )),
        ))
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Increment => state.count += 1,
            Action::Disabled => state.count += 100,
        }
    }
}

const fn pointer_event(
    phase: PointerPhase,
    button: Option<PointerButton>,
    target: Option<RuntimeNodeId>,
) -> PointerEvent {
    PointerEvent::new(
        phase,
        LogicalPoint::new(10.0, 20.0),
        button,
        KeyModifiers::NONE,
        target,
    )
}

const fn primary_press(target: Option<RuntimeNodeId>) -> PointerEvent {
    pointer_event(PointerPhase::Pressed, Some(PointerButton::Primary), target)
}

fn node_id<App>(runtime: &AppRuntime<App>, id: &str) -> Result<RuntimeNodeId, &'static str>
where
    App: UiApp,
{
    runtime
        .index()
        .node_by_authored_id(id)
        .map(RuntimeNodeRef::id)
        .ok_or("expected authored node")
}

#[test]
fn primary_press_activates_targeted_button() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;
    let event = primary_press(Some(increment));

    let result = runtime.handle_pointer_activation(&event);

    assert_eq!(
        result,
        PointerActivationResult::Handled(ActivationResult::Dispatched)
    );
    assert_eq!(runtime.state().count, 1);
    assert_eq!(
        runtime.trace().events(),
        &[
            RuntimeEvent::Mounted,
            RuntimeEvent::ActionDispatched,
            RuntimeEvent::StateUpdated,
            RuntimeEvent::RootRebuilt,
        ]
    );

    Ok(())
}

#[test]
fn primary_press_without_target_reports_no_target() {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let event = primary_press(None);

    let result = runtime.handle_pointer_activation(&event);

    assert_eq!(result, PointerActivationResult::NoTarget);
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn primary_press_with_stale_target_reports_not_found() {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let event = primary_press(Some(RuntimeNodeId::from_index(99)));

    let result = runtime.handle_pointer_activation(&event);

    assert_eq!(
        result,
        PointerActivationResult::Handled(ActivationResult::NotFound)
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);
}

#[test]
fn primary_press_with_disabled_target_reports_disabled() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let disabled = node_id(&runtime, "disabled")?;
    let event = primary_press(Some(disabled));

    let result = runtime.handle_pointer_activation(&event);

    assert_eq!(
        result,
        PointerActivationResult::Handled(ActivationResult::Disabled)
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);

    Ok(())
}

#[test]
fn primary_press_with_no_action_target_reports_no_action() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let no_action = node_id(&runtime, "no-action")?;
    let event = primary_press(Some(no_action));

    let result = runtime.handle_pointer_activation(&event);

    assert_eq!(
        result,
        PointerActivationResult::Handled(ActivationResult::NoAction)
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);

    Ok(())
}

#[test]
fn primary_press_with_text_target_reports_not_activatable() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let title = node_id(&runtime, "title")?;
    let event = primary_press(Some(title));

    let result = runtime.handle_pointer_activation(&event);

    assert_eq!(
        result,
        PointerActivationResult::Handled(ActivationResult::NotActivatable)
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);

    Ok(())
}

#[test]
fn non_primary_or_non_pressed_pointer_events_are_ignored() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedActivationApp>::mount(State::default());
    let increment = node_id(&runtime, "increment")?;

    let secondary = pointer_event(
        PointerPhase::Pressed,
        Some(PointerButton::Secondary),
        Some(increment),
    );
    let moved = pointer_event(PointerPhase::Moved, None, Some(increment));
    let released = pointer_event(
        PointerPhase::Released,
        Some(PointerButton::Primary),
        Some(increment),
    );

    assert_eq!(
        runtime.handle_pointer_activation(&secondary),
        PointerActivationResult::Ignored
    );
    assert_eq!(
        runtime.handle_pointer_activation(&moved),
        PointerActivationResult::Ignored
    );
    assert_eq!(
        runtime.handle_pointer_activation(&released),
        PointerActivationResult::Ignored
    );
    assert_eq!(runtime.state().count, 0);
    assert_eq!(runtime.trace().events(), &[RuntimeEvent::Mounted]);

    Ok(())
}
