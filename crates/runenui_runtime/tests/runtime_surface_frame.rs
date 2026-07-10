use runenui_core::prelude::{button, column, text};
use runenui_runtime::prelude::{
    AppRuntime, LogicalRect, LogicalSize, RuntimeNodeId, SurfaceLayoutMetrics, SurfaceNodeKind,
    UiApp,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Increment,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct State {
    count: i32,
}

struct CounterApp;

impl UiApp for CounterApp {
    type State = State;
    type Action = Action;

    fn root(state: &Self::State) -> runenui_core::Element<Self::Action> {
        column((
            text(format!("Count: {}", state.count)).id("counter.value"),
            button("+")
                .id("counter.increment")
                .on_press(Action::Increment),
        ))
        .id("counter.root")
        .gap(4.0)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Increment => state.count += 1,
        }
    }
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "expected {expected}, got {actual}",
    );
}

fn assert_rect_eq(actual: LogicalRect, expected: LogicalRect) {
    assert_f32_eq(actual.x(), expected.x());
    assert_f32_eq(actual.y(), expected.y());
    assert_f32_eq(actual.width(), expected.width());
    assert_f32_eq(actual.height(), expected.height());
}

#[test]
fn app_runtime_surface_frame_lays_out_current_root() -> Result<(), &'static str> {
    let runtime = AppRuntime::<CounterApp>::mount(State::default());

    let frame = runtime.surface_frame(LogicalSize::new(200.0, 100.0));
    let root = frame.root().ok_or("expected root surface node")?;
    let value = frame
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected value surface node")?;
    let increment = frame
        .node(RuntimeNodeId::from_index(2))
        .ok_or("expected increment surface node")?;

    assert_eq!(frame.nodes().len(), 3);
    assert_rect_eq(
        root.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 200.0, 100.0),
    );
    assert_eq!(
        root.authored_id().map(runenui_core::ElementId::as_str),
        Some("counter.root")
    );
    assert_eq!(value.kind(), &SurfaceNodeKind::text("Count: 0"));
    assert_rect_eq(value.bounds(), LogicalRect::from_xywh(0.0, 0.0, 64.0, 20.0));
    assert_eq!(increment.kind(), &SurfaceNodeKind::button("+", true));
    assert_rect_eq(
        increment.bounds(),
        LogicalRect::from_xywh(0.0, 24.0, 64.0, 32.0),
    );

    Ok(())
}

#[test]
fn app_runtime_surface_frame_reflects_rebuilt_root_after_dispatch() -> Result<(), &'static str> {
    let mut runtime = AppRuntime::<CounterApp>::mount(State::default());

    runtime.dispatch(Action::Increment);
    let frame = runtime.surface_frame(LogicalSize::new(200.0, 100.0));
    let value = frame
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected value surface node")?;

    assert_eq!(runtime.state().count, 1);
    assert_eq!(value.kind(), &SurfaceNodeKind::text("Count: 1"));

    Ok(())
}

#[test]
fn app_runtime_surface_frame_accepts_explicit_metrics() -> Result<(), &'static str> {
    let runtime = AppRuntime::<CounterApp>::mount(State::default());
    let metrics = SurfaceLayoutMetrics::new(10.0, 18.0, 9.0, 5.0, 22.0, 30.0);

    let frame = runtime.surface_frame_with_metrics(LogicalSize::new(200.0, 100.0), metrics);
    let value = frame
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected value surface node")?;
    let increment = frame
        .node(RuntimeNodeId::from_index(2))
        .ok_or("expected increment surface node")?;

    assert_rect_eq(value.bounds(), LogicalRect::from_xywh(0.0, 0.0, 80.0, 18.0));
    assert_rect_eq(
        increment.bounds(),
        LogicalRect::from_xywh(0.0, 22.0, 30.0, 22.0),
    );

    Ok(())
}
