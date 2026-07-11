//! Headless counter proof for `RunenUI`.
//!
//! The example owns its state, actions, update function, and screens. The
//! runtime owns typed action dispatch, update execution, root rebuilds, semantic
//! activation, trace recording, surface-frame publication, and debug surface
//! rendering.

mod app;
mod ui;

use app::{Counter, CounterApp, WIN_COUNT};
use runenui_core::{LogicalLength, StyleTokens};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext, render_debug_surface_frame};

const EXAMPLE_SURFACE_SIZE: LogicalSize = LogicalSize::new(
    match LogicalLength::new(240.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
    match LogicalLength::new(160.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
);

fn debug_surface(runtime: &AppRuntime<CounterApp>) -> String {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::tight(&tokens, EXAMPLE_SURFACE_SIZE);
    let publication = runtime.publish_surface(&context);
    render_debug_surface_frame(publication.frame())
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
    use runenui_core::ElementKind;
    use runenui_runtime::{ActivationResult, AppRuntime};

    use crate::app::{Counter, CounterAction, CounterApp, WIN_COUNT};
    use crate::debug_surface;
    use crate::ui::root;

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
}
