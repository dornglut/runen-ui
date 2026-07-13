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

fn debug_surface(runtime: &mut AppRuntime<CounterApp>) -> String {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::tight(&tokens, EXAMPLE_SURFACE_SIZE);
    let publication = runtime.publish_surface(&context);
    render_debug_surface_frame(publication.frame())
}

fn print_debug_surface(label: &str, runtime: &mut AppRuntime<CounterApp>) {
    let surface = debug_surface(runtime);
    println!("{label}\n{surface}");
}

fn main() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

    print_debug_surface("counter.surface.initial", &mut runtime);

    for _ in 0..WIN_COUNT {
        runtime.activate("counter.increment");
    }

    print_debug_surface("counter.surface.win", &mut runtime);

    runtime.activate("counter.reset");

    print_debug_surface("counter.surface.reset", &mut runtime);

    let count = runtime.state().count;
    let trace_events = runtime.trace().events().len();

    println!("counter.count={count} trace_events={trace_events}");
}

#[cfg(test)]
mod tests {
    use runenui_core::{LogicalLength, StyleTokens};
    use runenui_runtime::{
        ActivationResult, AppRuntime, FocusTargetResult, LogicalSize, SurfaceBuildContext,
    };

    use crate::app::{Counter, CounterAction, CounterApp, WIN_COUNT};
    use crate::debug_surface;
    fn published_names(counter: Counter) -> Vec<String> {
        let mut runtime = AppRuntime::<CounterApp>::mount(counter);
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::tight(
            &tokens,
            LogicalSize::new(LogicalLength::from(240_u16), LogicalLength::from(160_u16)),
        );
        runtime
            .publish_surface(&context)
            .frame()
            .nodes()
            .iter()
            .map(|node| node.semantics().name().to_owned())
            .collect()
    }

    #[test]
    fn counter_screen_is_used_before_win_count() {
        let counter = Counter { count: 9 };

        let names = published_names(counter);
        assert_eq!(names[1], "Counter");
        assert_eq!(names[2], "9");
    }

    #[test]
    fn win_screen_is_used_at_win_count() {
        let counter = Counter { count: 10 };

        let names = published_names(counter);
        assert_eq!(names[1], "You win");
        assert_eq!(names[2], "Count: 10");
    }

    #[test]
    fn reset_returns_to_counter_screen() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 10 });

        assert_eq!(
            runtime.activate("counter.reset"),
            ActivationResult::Dispatched
        );

        assert_eq!(runtime.state(), &Counter { count: 0 });
        assert_eq!(published_names(runtime.into_state())[1], "Counter");
    }

    #[test]
    fn semantic_increment_activation_reaches_win_screen() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        for _ in 0..WIN_COUNT {
            assert_eq!(
                runtime.activate("counter.increment"),
                ActivationResult::Dispatched
            );
        }

        assert_eq!(runtime.state(), &Counter { count: 10 });
        assert_eq!(published_names(runtime.into_state())[1], "You win");
    }

    #[test]
    fn direct_dispatch_still_works_without_authored_ids() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        runtime
            .dispatch(CounterAction::Increment)
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(runtime.state(), &Counter { count: 1 });
    }

    #[test]
    fn generation_exhaustion_preserves_counter_and_mounted_state() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
        let authored =
            runenui_core::ElementId::new("counter.increment").unwrap_or_else(|_| unreachable!());
        let increment = runtime
            .index()
            .node_by_authored_id(&authored)
            .unwrap_or_else(|| unreachable!())
            .id()
            .clone();
        let semantic = runtime
            .index()
            .node(&increment)
            .unwrap_or_else(|| unreachable!())
            .semantic_id()
            .clone();
        assert_eq!(
            runtime.set_focus(increment.clone()),
            FocusTargetResult::Focused
        );
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::tight(&tokens, crate::EXAMPLE_SURFACE_SIZE);
        let before = runtime.publish_surface(&context);
        let report = runtime.reconciliation_report().clone();
        let trace = runtime.trace().clone();
        runtime.__seed_reconciliation_generation_for_test(u64::MAX);

        assert_eq!(
            runtime.activate_node(&increment),
            ActivationResult::RuntimeError(
                runenui_runtime::RuntimeError::ReconciliationGenerationExhausted
            )
        );
        assert_eq!(runtime.state(), &Counter::new());
        assert_eq!(runtime.focus().focused_node(), Some(&increment));
        assert_eq!(
            runtime
                .index()
                .node(&increment)
                .unwrap_or_else(|| unreachable!())
                .semantic_id(),
            &semantic
        );
        assert_eq!(runtime.reconciliation_report(), &report);
        assert_eq!(runtime.trace(), &trace);
        assert_eq!(runtime.publish_surface(&context), before);

        runtime.__seed_reconciliation_generation_for_test(1);
        assert_eq!(
            runtime.activate_node(&increment),
            ActivationResult::Dispatched
        );
        assert_eq!(runtime.state(), &Counter { count: 1 });
    }

    #[test]
    fn debug_surface_output_exposes_counter_screen() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
        let surface = debug_surface(&mut runtime);

        assert!(surface.contains("surface size=(240.0,160.0) nodes=7"));
        assert!(surface.contains("paint=text \"Counter\""));
        assert!(surface.contains("authored=counter.increment"));
        assert!(surface.contains("semantic=button \"+\" enabled=true actionable=true"));
    }

    #[test]
    fn debug_surface_output_exposes_win_screen_after_rebuild() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());

        for _ in 0..WIN_COUNT {
            runtime
                .dispatch(CounterAction::Increment)
                .unwrap_or_else(|_| unreachable!());
        }

        let surface = debug_surface(&mut runtime);

        assert!(surface.contains("surface size=(240.0,160.0) nodes=4"));
        assert!(surface.contains("paint=text \"You win\""));
        assert!(surface.contains("paint=text \"Count: 10\""));
        assert!(surface.contains("authored=counter.reset"));
    }

    #[test]
    fn mounted_identity_focus_state_and_screen_replacement_are_proven() {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
        let authored =
            runenui_core::ElementId::new("counter.increment").unwrap_or_else(|_| unreachable!());
        let increment = runtime
            .index()
            .node_by_authored_id(&authored)
            .unwrap_or_else(|| unreachable!())
            .id()
            .clone();
        let semantic = runtime
            .index()
            .node(&increment)
            .unwrap_or_else(|| unreachable!())
            .semantic_id()
            .clone();
        assert_eq!(
            runtime.set_focus(increment.clone()),
            FocusTargetResult::Focused
        );
        assert_eq!(
            runtime.activate_node(&increment),
            ActivationResult::Dispatched
        );
        assert_eq!(runtime.focus().focused_node(), Some(&increment));
        assert_eq!(
            runtime
                .index()
                .node_by_authored_id(&authored)
                .unwrap_or_else(|| unreachable!())
                .semantic_id(),
            &semantic
        );
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::tight(&tokens, crate::EXAMPLE_SURFACE_SIZE);
        assert!(
            runtime
                .publish_surface(&context)
                .frame()
                .node(&increment)
                .unwrap_or_else(|| unreachable!())
                .paint()
                .description()
                .contains("activations=1")
        );
        for _ in 1..WIN_COUNT {
            assert_eq!(
                runtime.activate("counter.increment"),
                ActivationResult::Dispatched
            );
        }
        assert_eq!(runtime.focus().focused_node(), None);
        assert_eq!(
            runtime.activate_node(&increment),
            ActivationResult::StaleTarget
        );
        assert_eq!(
            runtime.activate("counter.reset"),
            ActivationResult::Dispatched
        );
        let replacement = runtime
            .index()
            .node_by_authored_id(&authored)
            .unwrap_or_else(|| unreachable!())
            .id()
            .clone();
        assert_ne!(replacement, increment);
    }
}
