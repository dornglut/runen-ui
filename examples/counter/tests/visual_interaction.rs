#[path = "../src/app.rs"]
mod app;
#[path = "../src/ui.rs"]
mod ui;

use std::time::Duration;

use app::{Counter, CounterAction, CounterApp};
use runenui_core::{
    Brush, Color, ElementId, LogicalDelta, LogicalLength, LogicalPoint, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticCommand,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePublication,
};

const SCREEN_BACKGROUND: Color = Color::rgb(24, 28, 36);
const WIN_BACKGROUND: Color = Color::rgb(38, 82, 58);
const CONTROL_BACKGROUND: Color = Color::rgb(92, 106, 135);
const CONTROL_HOVER_BACKGROUND: Color = Color::rgb(78, 104, 160);
const CONTROL_ACTIVE_BACKGROUND: Color = Color::rgb(64, 80, 122);
const RESET_BACKGROUND: Color = Color::rgb(140, 92, 92);

const SURFACE_SIZE: LogicalSize = LogicalSize::new(
    match LogicalLength::new(240.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
    match LogicalLength::new(160.0) {
        Ok(value) => value,
        Err(_) => LogicalLength::ZERO,
    },
);

fn pump_all(runtime: &mut AppRuntime<CounterApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "Counter did not settle: {report:?}");
}

fn authored_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("controlled Counter id is valid"))
}

fn publish_at(
    runtime: &mut AppRuntime<CounterApp>,
    environment: &runenui_core::StyleEnvironment,
    size: LogicalSize,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::tight(environment, size))
        .unwrap_or_else(|_| unreachable!("Counter visual publication is admitted"))
}

fn publish(
    runtime: &mut AppRuntime<CounterApp>,
    environment: &runenui_core::StyleEnvironment,
) -> SurfacePublication {
    publish_at(runtime, environment, SURFACE_SIZE)
}

fn style_node<'a>(
    publication: &'a SurfacePublication,
    authored: &str,
) -> &'a runenui_runtime::SurfaceStyleNode {
    publication
        .style_report()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored Counter style node is published"))
}

fn frame_node<'a>(
    publication: &'a SurfacePublication,
    authored: &str,
) -> &'a runenui_runtime::SurfaceNode {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored Counter frame node is published"))
}

fn background(publication: &SurfacePublication, authored: &str) -> Brush {
    frame_node(publication, authored)
        .computed_style()
        .background()
        .cloned()
        .unwrap_or_else(|| unreachable!("Counter control publishes a background"))
}

fn target_point(publication: &SurfacePublication, authored: &str) -> LogicalPoint {
    let node = frame_node(publication, authored);
    let bounds = node.bounds();
    LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published Counter coordinates are finite"))
}

#[test]
fn counter_button_recipes_resolve_without_style_diagnostics() {
    let environment = ui::style_environment();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    pump_all(&mut runtime);
    let publication = publish(&mut runtime, &environment);

    for authored in ["counter.decrement", "counter.increment", "counter.reset"] {
        assert!(
            style_node(&publication, authored).is_fully_resolved(),
            "{authored} must resolve its Counter-owned recipe without diagnostics"
        );
    }

    assert_eq!(
        background(&publication, "counter.increment"),
        Brush::Solid(CONTROL_BACKGROUND)
    );
    assert_eq!(
        background(&publication, "counter.reset"),
        Brush::Solid(RESET_BACKGROUND)
    );
    assert!(
        frame_node(&publication, "counter.increment")
            .computed_style()
            .outline()
            .is_none(),
        "ordinary resting controls do not impersonate keyboard focus"
    );
}

#[test]
fn stepper_pair_stays_grouped_and_reset_wraps_on_narrow_surfaces() {
    let environment = ui::style_environment();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    pump_all(&mut runtime);

    let narrow_size = LogicalSize::new(LogicalLength::from(110_u16), LogicalLength::from(220_u16));
    let publication = publish_at(&mut runtime, &environment, narrow_size);
    let stepper = frame_node(&publication, "counter.stepper").bounds();
    let decrement = frame_node(&publication, "counter.decrement").bounds();
    let increment = frame_node(&publication, "counter.increment").bounds();
    let reset = frame_node(&publication, "counter.reset").bounds();

    for control in [decrement, increment, reset] {
        assert!(
            control.width() > 0.0 && control.height() > 0.0,
            "Counter controls retain ordinary intrinsic Button geometry"
        );
    }
    assert_eq!(decrement.y(), increment.y());
    assert!(decrement.x() >= stepper.x());
    assert!(increment.x() + increment.width() <= stepper.x() + stepper.width());
    assert!(
        reset.y() > stepper.y(),
        "Reset should wrap below the paired stepper group when the row is narrow"
    );
    let stepper_center = stepper.x() + stepper.width() / 2.0;
    let reset_center = reset.x() + reset.width() / 2.0;
    let surface_center = narrow_size.width() / 2.0;
    assert!((stepper_center - surface_center).abs() < 0.01);
    assert!((reset_center - surface_center).abs() < 0.01);
    assert!(
        reset.x() >= 0.0 && reset.x() + reset.width() <= narrow_size.width(),
        "wrapped Reset remains reachable inside the narrow viewport"
    );
}

#[test]
fn counter_centers_with_free_height_and_scrolls_from_top_when_height_is_tight() {
    let environment = ui::style_environment();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    pump_all(&mut runtime);

    let normal = publish(&mut runtime, &environment);
    let content = frame_node(&normal, "counter.content").bounds();
    let normal_center = content.y() + content.height() / 2.0;
    assert!(
        (normal_center - SURFACE_SIZE.height() / 2.0).abs() < 0.01,
        "collapsible flex space centers Counter content while vertical room is available"
    );

    let tight_size = LogicalSize::new(LogicalLength::from(240_u16), LogicalLength::from(72_u16));
    let tight = publish_at(&mut runtime, &environment, tight_size);
    let root_layout = tight
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!("Counter tight surface has a layout root"));
    assert!(
        root_layout.scrollable_extent().height() > root_layout.layout_extent().height(),
        "tight Counter height must produce real runtime-owned vertical overflow"
    );

    let initial_value_y = frame_node(&tight, "counter.value").bounds().y();
    let point = target_point(&tight, "counter.title");
    runtime
        .submit_pointer(
            PointerEvent::new(
                PointerId::new(7).unwrap_or_else(|| unreachable!("pointer id is non-zero")),
                PointerDeviceKind::Mouse,
                PointerPhase::Wheel,
                point,
                tight.input_context().clone(),
            )
            .with_scroll_delta(
                LogicalDelta::new(0.0, 40.0)
                    .unwrap_or_else(|_| unreachable!("Counter wheel delta is finite")),
            ),
        )
        .unwrap_or_else(|_| unreachable!("Counter tight-surface wheel input is admitted"));
    pump_all(&mut runtime);

    let scrolled = publish_at(&mut runtime, &environment, tight_size);
    assert!(
        frame_node(&scrolled, "counter.value").bounds().y() < initial_value_y,
        "runtime scroll offset moves clipped Counter content into reach"
    );
}

#[test]
fn canonical_hover_focus_and_active_facts_drive_counter_visual_feedback() {
    let environment = ui::style_environment();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter::new());
    pump_all(&mut runtime);

    let initial = publish(&mut runtime, &environment);
    let point = target_point(&initial, "counter.increment");
    let pointer_id = PointerId::new(1).unwrap_or_else(|| unreachable!("pointer id is non-zero"));

    runtime
        .submit_pointer(PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            point,
            initial.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("Counter hover ingress is admitted"));
    pump_all(&mut runtime);

    let hover_start = publish(&mut runtime, &environment);
    assert_eq!(
        background(&hover_start, "counter.increment"),
        Brush::Solid(CONTROL_BACKGROUND),
        "hover background transition starts from the presented resting value"
    );
    assert!(
        frame_node(&hover_start, "counter.increment")
            .computed_style()
            .outline()
            .is_none()
    );

    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("Counter hover duration is bounded"));
    let hovered = publish(&mut runtime, &environment);
    assert_eq!(
        background(&hovered, "counter.increment"),
        Brush::Solid(CONTROL_HOVER_BACKGROUND)
    );

    runtime
        .submit_automation_command(
            authored_id("counter.increment"),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("Counter focus target resolves"));
    pump_all(&mut runtime);
    let focused_hover = publish(&mut runtime, &environment);
    assert_eq!(
        background(&focused_hover, "counter.increment"),
        Brush::Solid(CONTROL_HOVER_BACKGROUND),
        "focus adds its own cue without erasing hover feedback"
    );
    assert!(
        frame_node(&focused_hover, "counter.increment")
            .computed_style()
            .outline()
            .is_some(),
        "focus is immediately visible through a persistent outline"
    );

    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                point,
                focused_hover.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("Counter primary press is admitted"));
    pump_all(&mut runtime);

    let active_start = publish(&mut runtime, &environment);
    assert_eq!(
        background(&active_start, "counter.increment"),
        Brush::Solid(CONTROL_HOVER_BACKGROUND)
    );
    assert!(
        frame_node(&active_start, "counter.increment")
            .computed_style()
            .outline()
            .is_some(),
        "active background feedback must not erase the focus cue"
    );

    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("Counter active duration is bounded"));
    let active = publish(&mut runtime, &environment);
    assert_eq!(
        background(&active, "counter.increment"),
        Brush::Solid(CONTROL_ACTIVE_BACKGROUND)
    );
    assert!(
        frame_node(&active, "counter.increment")
            .computed_style()
            .outline()
            .is_some()
    );

    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                point,
                active.input_context().clone(),
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("Counter primary release is admitted"));
    pump_all(&mut runtime);
    assert_eq!(
        runtime.state().count,
        1,
        "visual polish preserves ordinary Button activation semantics"
    );

    let release_start = publish(&mut runtime, &environment);
    assert_eq!(
        background(&release_start, "counter.increment"),
        Brush::Solid(CONTROL_ACTIVE_BACKGROUND)
    );

    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("Counter release duration is bounded"));
    let released = publish(&mut runtime, &environment);
    assert_eq!(
        background(&released, "counter.increment"),
        Brush::Solid(CONTROL_BACKGROUND),
        "an empty-button primary Up closes the accepted pointer stream, so hover resumes on the next native pointer move rather than being fabricated by Counter"
    );
    assert!(
        frame_node(&released, "counter.increment")
            .computed_style()
            .outline()
            .is_some(),
        "actual runtime focus remains visible after the pointer stream closes"
    );
}

#[test]
fn win_screen_reuses_the_shell_and_transitions_its_background() {
    let environment = ui::style_environment();
    let mut runtime = AppRuntime::<CounterApp>::mount(Counter { count: 9 });
    pump_all(&mut runtime);

    let initial = publish(&mut runtime, &environment);
    let initial_root = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("Counter screen has a root"));
    let root_id = initial_root.id().clone();
    let content_id = frame_node(&initial, "counter.content").id().clone();
    let title_id = frame_node(&initial, "counter.title").id().clone();
    let count_id = frame_node(&initial, "counter.value").id().clone();
    let reset_id = frame_node(&initial, "counter.reset").id().clone();
    assert_eq!(
        initial_root.computed_style().background(),
        Some(&Brush::Solid(SCREEN_BACKGROUND))
    );

    runtime
        .submit_action(CounterAction::Increment)
        .unwrap_or_else(|_| unreachable!("Counter increment action is admitted"));
    pump_all(&mut runtime);
    assert_eq!(runtime.state().count, 10);

    let started = publish(&mut runtime, &environment);
    let started_root = started
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("Counter win screen has a root"));
    assert_eq!(started_root.id(), &root_id);
    assert_eq!(
        frame_node(&started, "counter.content").id(),
        &content_id,
        "the keyed content shell reconciles in place"
    );
    assert_eq!(
        frame_node(&started, "counter.content")
            .computed_style()
            .opacity()
            .get(),
        0.0,
        "incoming win content starts fully transparent instead of popping in"
    );
    assert_eq!(
        frame_node(&started, "counter.win.title").id(),
        &title_id,
        "the keyed title reconciles in place while its authored label/id changes"
    );
    assert_eq!(
        frame_node(&started, "counter.value").id(),
        &count_id,
        "the keyed count presentation reconciles in place"
    );
    assert_eq!(
        frame_node(&started, "counter.reset").id(),
        &reset_id,
        "Reset stays mounted in the stable controls row while the stepper pair is removed"
    );
    assert_eq!(
        started_root.computed_style().background(),
        Some(&Brush::Solid(SCREEN_BACKGROUND)),
        "the win transition starts from the previously presented screen background"
    );

    runtime
        .advance_time(Duration::from_millis(90))
        .unwrap_or_else(|_| unreachable!("Counter screen midpoint is bounded"));
    let midpoint = publish(&mut runtime, &environment);
    let midpoint_background = midpoint
        .frame()
        .root()
        .and_then(|node| node.computed_style().background())
        .cloned()
        .unwrap_or_else(|| unreachable!("Counter midpoint keeps a root background"));
    assert_ne!(midpoint_background, Brush::Solid(SCREEN_BACKGROUND));
    assert_ne!(midpoint_background, Brush::Solid(WIN_BACKGROUND));
    let midpoint_opacity = frame_node(&midpoint, "counter.content")
        .computed_style()
        .opacity()
        .get();
    assert!(midpoint_opacity > 0.0 && midpoint_opacity < 1.0);

    runtime
        .advance_time(Duration::from_millis(90))
        .unwrap_or_else(|_| unreachable!("Counter screen terminal time is bounded"));
    let terminal = publish(&mut runtime, &environment);
    assert_eq!(
        terminal
            .frame()
            .root()
            .and_then(|node| node.computed_style().background()),
        Some(&Brush::Solid(WIN_BACKGROUND))
    );
    assert_eq!(
        frame_node(&terminal, "counter.content")
            .computed_style()
            .opacity()
            .get(),
        1.0
    );
}
