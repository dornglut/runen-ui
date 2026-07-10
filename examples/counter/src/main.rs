//! Headless counter proof for `RunenUI`.
//!
//! The example owns its state, actions, update function, and screens. The
//! runtime owns typed action dispatch, update execution, root rebuilds, semantic
//! activation, trace recording, surface-frame publication, and debug surface
//! rendering.

use runenui_core::{
    Axis, ButtonArgs, ContainerArgs, Element, IntoElements, TextArgs, button_with, container_with,
    text_with,
};
use runenui_runtime::{AppRuntime, LogicalSize, UiApp, render_debug_surface_frame};

const WIN_COUNT: i32 = 10;
const EXAMPLE_SURFACE_SIZE: LogicalSize = LogicalSize::new(240.0, 160.0);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter {
    count: i32,
}

impl Counter {
    const fn new() -> Self {
        Self { count: 0 }
    }

    const fn has_won(&self) -> bool {
        self.count >= WIN_COUNT
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterAction {
    Decrement,
    Increment,
    Reset,
}

struct CounterApp;

impl UiApp for CounterApp {
    type State = Counter;
    type Action = CounterAction;

    fn root(state: &Self::State) -> Element<Self::Action> {
        root(state)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        update(state, action);
    }
}

struct CounterScreen;

impl CounterScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        screen_column((counter_title(), counter_value(counter), counter_controls()))
    }
}

struct WinScreen;

impl WinScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        screen_column((win_title(), win_value(counter), reset_button()))
    }
}

fn screen_column(children: impl IntoElements<CounterAction>) -> Element<CounterAction> {
    container_with(ContainerArgs::new(Axis::Vertical, children).gap(8_u16))
}

fn counter_title() -> Element<CounterAction> {
    text_with(TextArgs::new("Counter").id("counter.title"))
}

fn counter_value(counter: &Counter) -> Element<CounterAction> {
    text_with(TextArgs::new(counter.count.to_string()).id("counter.value"))
}

fn win_title() -> Element<CounterAction> {
    text_with(TextArgs::new("You win").id("counter.win.title"))
}

fn win_value(counter: &Counter) -> Element<CounterAction> {
    let count = counter.count;

    text_with(TextArgs::new(format!("Count: {count}")).id("counter.value"))
}

fn counter_controls() -> Element<CounterAction> {
    container_with(
        ContainerArgs::new(
            Axis::Horizontal,
            (
                counter_button("counter.decrement", "-", CounterAction::Decrement),
                counter_button("counter.increment", "+", CounterAction::Increment),
                reset_button(),
            ),
        )
        .id("counter.controls")
        .gap(8_u16),
    )
}

fn counter_button(
    id: &'static str,
    label: &'static str,
    action: CounterAction,
) -> Element<CounterAction> {
    button_with(ButtonArgs::new(label).id(id).on_press(action))
}

fn reset_button() -> Element<CounterAction> {
    counter_button("counter.reset", "Reset", CounterAction::Reset)
}

fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.has_won() {
        WinScreen::root(counter)
    } else {
        CounterScreen::root(counter)
    }
}

const fn update(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Decrement => counter.count -= 1,
        CounterAction::Increment => counter.count += 1,
        CounterAction::Reset => counter.count = 0,
    }
}

fn debug_surface(runtime: &AppRuntime<CounterApp>) -> String {
    let frame = runtime.surface_frame(EXAMPLE_SURFACE_SIZE);
    render_debug_surface_frame(&frame)
}

fn print_debug_surface(label: &str, runtime: &AppRuntime<CounterApp>) {
    let surface = debug_surface(runtime);
    println!("{label}\n{surface}");
}

fn main() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

    print_debug_surface("counter.surface.initial", &runtime);

    for _ in 0..WIN_COUNT {
        runtime.activate("counter.increment");
    }

    print_debug_surface("counter.surface.win", &runtime);

    runtime.activate("counter.reset");

    print_debug_surface("counter.surface.reset", &runtime);

    let count = runtime.state().count;
    let trace_events = runtime.trace().events().len();

    println!("counter.count={count} trace_events={trace_events}");
}

#[cfg(test)]
mod tests {
    use super::{
        AppRuntime, Counter, CounterAction, CounterApp, WIN_COUNT, counter_button,
        counter_controls, debug_surface, reset_button, root,
    };
    use runenui_core::ElementKind;
    use runenui_runtime::ActivationResult;

    fn root_text(counter: &Counter) -> Result<String, &'static str> {
        let root = root(counter);
        let ElementKind::Container(container) = root.kind() else {
            return Err("expected root container");
        };
        let Some(title) = container.children().first() else {
            return Err("expected title element");
        };
        let ElementKind::Text(text) = title.kind() else {
            return Err("expected title text");
        };
        Ok(text.content().to_owned())
    }

    fn value_text(counter: &Counter) -> Result<String, &'static str> {
        let root = root(counter);
        let ElementKind::Container(container) = root.kind() else {
            return Err("expected root container");
        };
        let Some(value) = container.children().get(1) else {
            return Err("expected value element");
        };
        let ElementKind::Text(text) = value.kind() else {
            return Err("expected value text");
        };
        Ok(text.content().to_owned())
    }

    #[test]
    fn counter_screen_is_used_before_win_count() -> Result<(), &'static str> {
        let counter = Counter { count: 9 };

        assert_eq!(root_text(&counter)?, "Counter");
        assert_eq!(value_text(&counter)?, "9");
        Ok(())
    }

    #[test]
    fn win_screen_is_used_at_win_count() -> Result<(), &'static str> {
        let counter = Counter { count: 10 };

        assert_eq!(root_text(&counter)?, "You win");
        assert_eq!(value_text(&counter)?, "Count: 10");
        Ok(())
    }

    #[test]
    fn reset_returns_to_counter_screen() -> Result<(), &'static str> {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 10 });

        assert_eq!(
            runtime.activate("counter.reset"),
            ActivationResult::Dispatched
        );

        assert_eq!(runtime.state(), &Counter { count: 0 });
        assert_eq!(root_text(runtime.state())?, "Counter");
        Ok(())
    }

    #[test]
    fn semantic_increment_activation_reaches_win_screen() -> Result<(), &'static str> {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        for _ in 0..WIN_COUNT {
            assert_eq!(
                runtime.activate("counter.increment"),
                ActivationResult::Dispatched
            );
        }

        assert_eq!(runtime.state(), &Counter { count: 10 });
        assert_eq!(root_text(runtime.state())?, "You win");
        Ok(())
    }

    #[test]
    fn direct_dispatch_still_works_without_authored_ids() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        runtime.dispatch(CounterAction::Increment);

        assert_eq!(runtime.state(), &Counter { count: 1 });
    }

    #[test]
    fn debug_surface_output_exposes_counter_screen() {
        let runtime = AppRuntime::<CounterApp>::mount(Counter::new());
        let surface = debug_surface(&runtime);

        assert!(surface.contains("surface size=(240.0,160.0) nodes=7"));
        assert!(surface.contains("kind=text \"Counter\""));
        assert!(surface.contains("authored=counter.increment"));
        assert!(surface.contains("kind=button \"+\" enabled=true"));
    }

    #[test]
    fn debug_surface_output_exposes_win_screen_after_rebuild() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        for _ in 0..WIN_COUNT {
            runtime.dispatch(CounterAction::Increment);
        }

        let surface = debug_surface(&runtime);

        assert!(surface.contains("surface size=(240.0,160.0) nodes=4"));
        assert!(surface.contains("kind=text \"You win\""));
        assert!(surface.contains("kind=text \"Count: 10\""));
        assert!(surface.contains("authored=counter.reset"));
    }

    #[test]
    fn button_component_sets_id_label_and_action() -> Result<(), &'static str> {
        let button = counter_button("counter.increment", "+", CounterAction::Increment);

        assert_eq!(
            button.element_id().map(runenui_core::ElementId::as_str),
            Some("counter.increment")
        );

        let ElementKind::Button(button) = button.kind() else {
            return Err("expected button element");
        };

        assert_eq!(button.label(), "+");
        assert_eq!(button.on_press(), Some(&CounterAction::Increment));
        Ok(())
    }

    #[test]
    fn controls_component_reuses_reset_button() -> Result<(), &'static str> {
        let controls = counter_controls();
        let reset = reset_button();

        let ElementKind::Container(container) = controls.kind() else {
            return Err("expected controls container");
        };

        assert_eq!(container.children().len(), 3);
        assert_eq!(container.children()[2], reset);
        Ok(())
    }
}
