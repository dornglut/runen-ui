//! Headless counter proof for `RunenUI`.
//!
//! The example owns its state, actions, update function, and screens. The
//! runtime owns typed action dispatch, update execution, root rebuilds, semantic
//! activation, trace recording, surface-frame publication, and debug surface
//! rendering.

mod app;
mod ui;

use app::{Counter, CounterApp, WIN_COUNT};
use runenui_core::{CommandOrigin, ElementId, LogicalLength, SemanticCommand, StyleTokens};
use runenui_runtime::{
    AppRuntime, CommandSubmission, LogicalSize, MountedNodeId, PumpBudget, SubmitCommandError,
    SurfaceBuildContext, render_debug_surface_frame,
};

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

fn settle_initial_work(runtime: &mut AppRuntime<CounterApp>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn target_by_authored_id(runtime: &mut AppRuntime<CounterApp>, authored_id: &str) -> MountedNodeId {
    let authored_id = ElementId::new(authored_id).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .node_by_authored_id(&authored_id)
        .unwrap_or_else(|| unreachable!("counter command target is mounted"))
        .id()
        .clone()
}

fn submit_activate(
    runtime: &mut AppRuntime<CounterApp>,
    target: MountedNodeId,
) -> Result<CommandSubmission, SubmitCommandError> {
    runtime.submit_command(
        target,
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    )
}

fn main() {
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    settle_initial_work(&mut runtime);

    print_debug_surface("counter.surface.initial", &mut runtime);

    let increment = target_by_authored_id(&mut runtime, "counter.increment");
    for _ in 0..WIN_COUNT {
        submit_activate(&mut runtime, increment.clone()).unwrap_or_else(|_| unreachable!());
    }
    runtime.pump(PumpBudget::new(
        WIN_COUNT as usize,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    settle_initial_work(&mut runtime);

    print_debug_surface("counter.surface.win", &mut runtime);

    let reset = target_by_authored_id(&mut runtime, "counter.reset");
    submit_activate(&mut runtime, reset).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));

    print_debug_surface("counter.surface.reset", &mut runtime);

    let count = runtime.state().count;
    let trace_events = runtime.trace().len();

    println!("counter.count={count} trace_events={trace_events}");
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        CommandOrigin, LogicalDelta, LogicalLength, LogicalPoint, PointerButton, PointerButtons,
        PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticCommand, StyleTokens,
    };
    use runenui_runtime::{
        AppRuntime, LogicalSize, PumpBudget, RuntimeStatus, RuntimeTerminalReason,
        SubmitCommandErrorKind, SurfaceBuildContext,
    };

    use crate::app::{Counter, CounterAction, CounterApp, WIN_COUNT};
    use crate::{debug_surface, settle_initial_work, submit_activate, target_by_authored_id};

    fn mounted_counter(counter: Counter) -> AppRuntime<CounterApp> {
        let mut runtime = AppRuntime::<CounterApp>::mount(counter);
        settle_initial_work(&mut runtime);
        runtime
    }

    fn published_names(counter: Counter) -> Vec<String> {
        let mut runtime = mounted_counter(counter);
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

    fn primary_pointer_event(
        pointer_id: PointerId,
        phase: PointerPhase,
        point: LogicalPoint,
        context: runenui_core::SurfaceInputContext,
    ) -> PointerEvent {
        PointerEvent::new(pointer_id, PointerDeviceKind::Mouse, phase, point, context)
            .with_buttons(if phase == PointerPhase::Down {
                PointerButtons::new([PointerButton::Primary])
            } else {
                PointerButtons::default()
            })
            .with_changed_button(PointerButton::Primary)
            .with_movement_delta(LogicalDelta::ZERO)
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
        let mut runtime = mounted_counter(Counter { count: 10 });

        let reset = target_by_authored_id(&mut runtime, "counter.reset");
        submit_activate(&mut runtime, reset)
            .unwrap_or_else(|_| unreachable!("the exact live reset target is accepted"));
        assert_eq!(runtime.state(), &Counter { count: 10 });
        runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(runtime.state(), &Counter { count: 0 });
        assert_eq!(published_names(runtime.into_state())[1], "Counter");
    }

    #[test]
    fn semantic_increment_activation_reaches_win_screen() {
        let mut runtime = mounted_counter(Counter::new());

        let increment = target_by_authored_id(&mut runtime, "counter.increment");
        for _ in 0..WIN_COUNT {
            submit_activate(&mut runtime, increment.clone())
                .unwrap_or_else(|_| unreachable!("the exact live increment target is accepted"));
        }
        assert_eq!(runtime.state(), &Counter::new());
        runtime.pump(PumpBudget::new(
            (WIN_COUNT * 2) as usize,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ));
        assert_eq!(runtime.state(), &Counter { count: 10 });
        assert_eq!(published_names(runtime.into_state())[1], "You win");
    }

    #[test]
    fn physical_primary_release_inside_increments_once() {
        let mut runtime = mounted_counter(Counter::new());
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::tight(&tokens, crate::EXAMPLE_SURFACE_SIZE);
        let publication = runtime.publish_surface(&context);
        let increment = publication
            .frame()
            .nodes()
            .iter()
            .find(|node| {
                node.authored_id()
                    .is_some_and(|id| id.as_str() == "counter.increment")
            })
            .unwrap_or_else(|| unreachable!("the increment control is published"));
        let bounds = increment.bounds();
        let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
            .unwrap_or_else(|_| unreachable!("published coordinates are finite"));
        let input = publication.input_context().clone();
        let pointer_id =
            PointerId::new(1).unwrap_or_else(|| unreachable!("the pointer identity is non-zero"));

        runtime
            .submit_pointer(primary_pointer_event(
                pointer_id,
                PointerPhase::Down,
                point,
                input.clone(),
            ))
            .unwrap_or_else(|_| unreachable!("the displayed down is accepted"));
        runtime
            .submit_pointer(primary_pointer_event(
                pointer_id,
                PointerPhase::Up,
                point,
                input,
            ))
            .unwrap_or_else(|_| unreachable!("the displayed up is accepted"));
        assert_eq!(runtime.state(), &Counter::new());

        settle_initial_work(&mut runtime);

        assert_eq!(runtime.state(), &Counter { count: 1 });
    }

    #[test]
    fn submitted_action_waits_for_the_explicit_pump() {
        let mut runtime = mounted_counter(Counter::new());

        runtime
            .submit_action(CounterAction::Increment)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(runtime.state(), &Counter::new());
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(runtime.state(), &Counter { count: 1 });
    }

    #[test]
    fn generation_exhaustion_preserves_counter_and_mounted_state() {
        let mut runtime = mounted_counter(Counter::new());
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
        runtime
            .submit_command(
                increment.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("the exact live focus target is accepted"));
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
        let tokens = StyleTokens::new();
        let context = SurfaceBuildContext::tight(&tokens, crate::EXAMPLE_SURFACE_SIZE);
        let before = runtime.publish_surface(&context);
        let before_context = before.input_context().clone();
        let before_products = before.into_parts();
        let report = runtime.reconciliation_report().clone();
        runtime.__seed_reconciliation_generation_for_test(u64::MAX);

        submit_activate(&mut runtime, increment.clone())
            .unwrap_or_else(|_| unreachable!("the exact live increment target is accepted"));
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(
            runtime.status(),
            RuntimeStatus::Terminal(RuntimeTerminalReason::ReconciliationGenerationExhausted)
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
        let after = runtime.publish_surface(&context);
        assert_eq!(
            after.input_context().surface_id(),
            before_context.surface_id()
        );
        assert!(after.input_context().coordinate_revision() > before_context.coordinate_revision());
        assert!(after.input_context().hit_test_generation() > before_context.hit_test_generation());
        assert_eq!(after.into_parts(), before_products);
        assert_eq!(
            runtime
                .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
                .processed_envelopes(),
            0
        );
    }

    #[test]
    fn debug_surface_output_exposes_counter_screen() {
        let mut runtime = mounted_counter(Counter::new());
        let surface = debug_surface(&mut runtime);

        assert!(surface.contains("surface size=(240.0,160.0) nodes=7"));
        assert!(surface.contains("paint=text \"Counter\""));
        assert!(surface.contains("authored=counter.increment"));
        assert!(surface.contains("semantic=button \"+\" enabled=true actionable=true"));
    }

    #[test]
    fn debug_surface_output_exposes_win_screen_after_rebuild() {
        let mut runtime = mounted_counter(Counter::new());

        for _ in 0..WIN_COUNT {
            runtime
                .submit_action(CounterAction::Increment)
                .unwrap_or_else(|_| unreachable!());
        }
        runtime.pump(PumpBudget::new(
            WIN_COUNT as usize,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ));

        let surface = debug_surface(&mut runtime);

        assert!(surface.contains("surface size=(240.0,160.0) nodes=4"));
        assert!(surface.contains("paint=text \"You win\""));
        assert!(surface.contains("paint=text \"Count: 10\""));
        assert!(surface.contains("authored=counter.reset"));
    }

    #[test]
    fn mounted_identity_focus_state_and_screen_replacement_are_proven() {
        let mut runtime = mounted_counter(Counter::new());
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
        runtime
            .submit_command(
                increment.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("the exact live focus target is accepted"));
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
        submit_activate(&mut runtime, increment.clone())
            .unwrap_or_else(|_| unreachable!("the exact live increment target is accepted"));
        runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
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
            submit_activate(&mut runtime, increment.clone())
                .unwrap_or_else(|_| unreachable!("the exact live increment target is accepted"));
        }
        runtime.pump(PumpBudget::new(
            ((WIN_COUNT - 1) * 2) as usize,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ));
        assert_eq!(runtime.focus().focused_node(), None);
        let Err(error) = submit_activate(&mut runtime, increment.clone()) else {
            unreachable!("the replaced increment target is stale")
        };
        assert_eq!(error.kind(), SubmitCommandErrorKind::StaleTarget);
        settle_initial_work(&mut runtime);
        let reset = target_by_authored_id(&mut runtime, "counter.reset");
        submit_activate(&mut runtime, reset)
            .unwrap_or_else(|_| unreachable!("the exact live reset target is accepted"));
        runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
        let replacement = runtime
            .index()
            .node_by_authored_id(&authored)
            .unwrap_or_else(|| unreachable!())
            .id()
            .clone();
        assert_ne!(replacement, increment);
    }
}
