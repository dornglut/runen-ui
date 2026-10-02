#[path = "../src/app.rs"]
mod app;
#[path = "../src/ui.rs"]
mod ui;

use std::time::Duration;

use app::{Counter, CounterAction, CounterApp};
use runenui_core::{Brush, Color};
use runenui_runtime::{
    AppRuntime, LogicalSize, ManualClock, PumpBudget, SurfaceBuildContext, SurfacePublication,
};

fn surface_size() -> LogicalSize {
    LogicalSize::try_new(240.0, 160.0)
        .unwrap_or_else(|_| unreachable!("Counter proof surface size is finite"))
}

fn pump_all(runtime: &mut AppRuntime<CounterApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

fn count_background(publication: &SurfacePublication) -> Brush {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "counter.value")
        })
        .and_then(|node| node.computed_style().background())
        .cloned()
        .unwrap_or_else(|| unreachable!("Counter publishes a background for the count value"))
}

#[test]
fn count_background_transition_uses_host_provided_monotonic_time() {
    let host_clock = ManualClock::new();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    runtime.set_monotonic_clock(host_clock.clone());
    pump_all(&mut runtime);

    let style_environment = ui::style_environment();
    let context = SurfaceBuildContext::tight(&style_environment, surface_size());
    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial Counter publication is admitted"));
    let initial_background = count_background(&initial);
    assert_eq!(initial_background, Brush::Solid(Color::rgb(40, 56, 104)));

    runtime
        .submit_action(CounterAction::Increment)
        .unwrap_or_else(|_| unreachable!("Counter increment action is admitted"));
    pump_all(&mut runtime);

    let started = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("Counter transition start is admitted"));
    assert_eq!(
        count_background(&started),
        initial_background,
        "the accepted transition starts from the previously presented count background"
    );

    host_clock
        .advance(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("Counter host-clock midpoint advance is bounded"));
    pump_all(&mut runtime);
    let middle = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("Counter transition midpoint is admitted"));
    let middle_background = count_background(&middle);
    let target_background = Brush::Solid(Color::rgb(52, 56, 104));
    assert_ne!(middle_background, initial_background);
    assert_ne!(middle_background, target_background);

    host_clock
        .advance(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("Counter host-clock terminal advance is bounded"));
    pump_all(&mut runtime);
    let terminal = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("Counter transition terminal sample is admitted"));
    assert_eq!(count_background(&terminal), target_background);
    assert_eq!(runtime.state(), &Counter { count: 1 });
}

#[test]
fn negative_count_transitions_through_zero_without_palette_wrap() {
    let host_clock = ManualClock::new();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    runtime.set_monotonic_clock(host_clock.clone());
    pump_all(&mut runtime);

    let style_environment = ui::style_environment();
    let context = SurfaceBuildContext::tight(&style_environment, surface_size());
    let neutral = Brush::Solid(Color::rgb(40, 56, 104));
    let negative = Brush::Solid(Color::rgb(40, 56, 116));
    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("initial count publication is admitted"));
    assert_eq!(count_background(&initial), neutral);

    runtime
        .submit_action(CounterAction::Decrement)
        .unwrap_or_else(|_| unreachable!("Counter decrement action is admitted"));
    pump_all(&mut runtime);
    let started = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("negative transition start is admitted"));
    assert_eq!(count_background(&started), neutral);

    host_clock
        .advance(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("negative transition midpoint time is bounded"));
    pump_all(&mut runtime);
    let midpoint = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("negative transition midpoint is admitted"));
    assert_ne!(count_background(&midpoint), neutral);
    assert_ne!(count_background(&midpoint), negative);

    host_clock
        .advance(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("negative transition terminal time is bounded"));
    pump_all(&mut runtime);
    let terminal = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("negative transition terminal is admitted"));
    assert_eq!(count_background(&terminal), negative);
    assert_eq!(runtime.state(), &Counter { count: -1 });

    runtime
        .submit_action(CounterAction::Increment)
        .unwrap_or_else(|_| unreachable!("Counter return-to-zero action is admitted"));
    pump_all(&mut runtime);
    let returning = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("return-to-zero transition start is admitted"));
    assert_eq!(count_background(&returning), negative);

    host_clock
        .advance(Duration::from_millis(200))
        .unwrap_or_else(|_| unreachable!("return-to-zero terminal time is bounded"));
    pump_all(&mut runtime);
    let zero = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("return-to-zero terminal is admitted"));
    assert_eq!(count_background(&zero), neutral);
    assert_eq!(runtime.state(), &Counter::new());
}

#[test]
fn count_color_clamps_at_palette_limits() {
    let style_environment = ui::style_environment();
    let context = SurfaceBuildContext::tight(&style_environment, surface_size());
    for count in [-9, -10, i32::MIN] {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count });
        pump_all(&mut runtime);
        let publication = runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("extreme count publication is admitted"));
        assert_eq!(
            count_background(&publication),
            Brush::Solid(Color::rgb(40, 56, 212)),
            "negative count {count} must remain within the bounded palette"
        );
    }
    for count in [9, 10, i32::MAX] {
        let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count });
        pump_all(&mut runtime);
        let publication = runtime
            .publish_surface(&context)
            .unwrap_or_else(|_| unreachable!("extreme count publication is admitted"));
        assert_eq!(
            count_background(&publication),
            Brush::Solid(Color::rgb(148, 56, 104)),
            "positive count {count} must remain within the bounded palette"
        );
    }
}
