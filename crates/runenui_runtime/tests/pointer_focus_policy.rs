use runenui_core::prelude::{button, column, row, text};
use runenui_runtime::prelude::{
    AppRuntime, KeyModifiers, LogicalPoint, PointerButton, PointerEvent, PointerFocusResult,
    PointerPhase, RuntimeNodeId, RuntimeNodeRef, UiApp,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    First,
    Disabled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State;

struct MixedFocusApp;

impl UiApp for MixedFocusApp {
    type State = State;
    type Action = Action;

    fn root(_state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text("Title").id("title"),
            button("First").id("first").on_press(Action::First),
            row((
                button("Disabled")
                    .id("disabled")
                    .on_press(Action::Disabled)
                    .disabled(),
                button("No action").id("no-action"),
            )),
        ))
    }

    fn update(_state: &mut Self::State, _action: Self::Action) {}
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
fn primary_press_focuses_targeted_focusable_node() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    let event = primary_press(Some(first));

    let result = runtime.handle_pointer_focus(&event);

    assert_eq!(result, PointerFocusResult::Moved(first));
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn primary_press_focuses_enabled_button_without_action() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let no_action = node_id(&runtime, "no-action")?;
    let event = primary_press(Some(no_action));

    let result = runtime.handle_pointer_focus(&event);

    assert_eq!(result, PointerFocusResult::Moved(no_action));
    assert_eq!(runtime.focus().focused_node(), Some(no_action));

    Ok(())
}

#[test]
fn primary_press_rejects_non_focusable_target_without_clearing_focus() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    let disabled = node_id(&runtime, "disabled")?;
    assert!(runtime.set_focus(first));

    let result = runtime.handle_pointer_focus(&primary_press(Some(disabled)));

    assert_eq!(result, PointerFocusResult::NotFocusable);
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn primary_press_rejects_text_target_without_clearing_focus() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    let title = node_id(&runtime, "title")?;
    assert!(runtime.set_focus(first));

    let result = runtime.handle_pointer_focus(&primary_press(Some(title)));

    assert_eq!(result, PointerFocusResult::NotFocusable);
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn primary_press_without_target_reports_no_target() {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let event = primary_press(None);

    let result = runtime.handle_pointer_focus(&event);

    assert_eq!(result, PointerFocusResult::NoTarget);
    assert_eq!(runtime.focus().focused_node(), None);
}

#[test]
fn primary_press_with_stale_target_reports_not_found_without_clearing_focus()
-> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    assert!(runtime.set_focus(first));

    let result = runtime.handle_pointer_focus(&primary_press(Some(RuntimeNodeId::from_index(99))));

    assert_eq!(result, PointerFocusResult::NotFound);
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}

#[test]
fn non_primary_or_non_pressed_pointer_events_are_ignored() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<MixedFocusApp>::mount(State);
    let first = node_id(&runtime, "first")?;
    assert!(runtime.set_focus(first));

    let secondary = pointer_event(
        PointerPhase::Pressed,
        Some(PointerButton::Secondary),
        Some(first),
    );
    let moved = pointer_event(PointerPhase::Moved, None, Some(first));
    let released = pointer_event(
        PointerPhase::Released,
        Some(PointerButton::Primary),
        Some(first),
    );

    assert_eq!(
        runtime.handle_pointer_focus(&secondary),
        PointerFocusResult::Ignored
    );
    assert_eq!(
        runtime.handle_pointer_focus(&moved),
        PointerFocusResult::Ignored
    );
    assert_eq!(
        runtime.handle_pointer_focus(&released),
        PointerFocusResult::Ignored
    );
    assert_eq!(runtime.focus().focused_node(), Some(first));

    Ok(())
}
