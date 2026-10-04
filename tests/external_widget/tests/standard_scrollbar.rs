#![allow(refining_impl_trait)]

use runenui_core::{
    Axis, Color, CommandOrigin, Element, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength,
    LogicalPoint, NoHostProtocol, OverflowPolicy, OverflowStyle, PhysicalKey, PointerButton,
    PointerButtons, PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    ScrollBarVisibility, ScrollControlBinding, ScrollControlRequest, SemanticAction,
    SemanticActionRequest, SemanticCommand, SemanticNumber, SemanticRole, StyleEnvironment,
    StyleIntent, UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput, scroll_bar,
    scroll_container,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SubmitSemanticActionErrorKind,
    SurfaceBuildContext, TraceRecordKind,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is finite"))
}

fn dimension(value: f32) -> LayoutDimension {
    LayoutDimension::Length(length(value))
}

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("fixture number is finite"))
}

#[derive(Clone, Copy, Debug)]
enum Action {
    SetContentHeight(f32),
    SetVertical(bool),
}

#[derive(Debug)]
struct Content {
    width: LogicalLength,
    height: LogicalLength,
}

impl Widget<Action> for Content {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug)]
struct State {
    content_width: f32,
    content_height: f32,
    horizontal: bool,
    vertical: bool,
    always: bool,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let content = Element::new(Content {
            width: length(state.content_width),
            height: length(state.content_height),
        })
        .id("standard.content")
        .key("standard.content")
        .with_layout(
            LayoutStyle::default()
                .with_width(dimension(state.content_width))
                .with_height(dimension(state.content_height)),
        );

        let overflow = OverflowStyle::new(OverflowPolicy::Scroll, OverflowPolicy::Scroll);
        let mut container = scroll_container(content, overflow)
            .id("standard.container")
            .key("standard.container")
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(dimension(100.0))
                    .with_height(dimension(100.0))
                    .with_overflow(overflow),
            )
            .corner_style(StyleIntent::EMPTY.with_background(Color::rgba(30, 40, 50, 255)));

        if state.vertical {
            let binding = ScrollControlBinding::new(Axis::Vertical, length(5.0))
                .unwrap_or_else(|_| unreachable!("vertical binding is valid"));
            let mut bar = scroll_bar("Vertical scroll", binding, length(10.0), length(20.0))
                .id("standard.vertical")
                .key("standard.vertical");
            if state.always {
                bar = bar.visibility(ScrollBarVisibility::Always);
            }
            container = container.scroll_bar(bar);
        }

        if state.horizontal {
            let binding = ScrollControlBinding::new(Axis::Horizontal, length(5.0))
                .unwrap_or_else(|_| unreachable!("horizontal binding is valid"));
            let mut bar = scroll_bar("Horizontal scroll", binding, length(10.0), length(20.0))
                .id("standard.horizontal")
                .key("standard.horizontal");
            if state.always {
                bar = bar.visibility(ScrollBarVisibility::Always);
            }
            container = container.scroll_bar(bar);
        }

        container
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::SetContentHeight(height) => state.content_height = height,
            Action::SetVertical(vertical) => state.vertical = vertical,
        }
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
}

fn build(environment: &StyleEnvironment) -> SurfaceBuildContext<'_> {
    SurfaceBuildContext::tight(
        environment,
        LogicalSize::try_new(100.0, 100.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    )
}

fn node_id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored)
        .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture node is mounted"))
        .id()
        .clone()
}

fn offset(runtime: &mut AppRuntime<App>, owner: &MountedNodeId) -> (f32, f32) {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.id() == owner)
        .unwrap_or_else(|| unreachable!("scroll owner remains mounted"))
        .interaction()
        .scroll_offset()
}

fn only_child_id(runtime: &mut AppRuntime<App>, parent: &MountedNodeId) -> MountedNodeId {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.parent() == Some(parent))
        .unwrap_or_else(|| unreachable!("standard scrollbar has one mounted thumb child"))
        .id()
        .clone()
}

fn assert_pointer_down_default_committed_on(
    runtime: &AppRuntime<App>,
    trace_start: usize,
    pointer_id: u64,
    target: &MountedNodeId,
) {
    assert!(
        runtime.trace().records().skip(trace_start).any(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::PointerDefaultApplied {
                    pointer_id: current,
                    phase: PointerPhase::Down,
                } if current.get() == pointer_id
            ) && record
                .target()
                .is_some_and(|trace_target| trace_target.mounted_node_id() == target)
        }),
        "primary Down must commit the canonical M4 pressed default on the exact thumb"
    );
}

fn assert_pointer_stream_closed(runtime: &AppRuntime<App>, trace_start: usize, pointer_id: u64) {
    assert!(
        runtime
            .trace()
            .records()
            .skip(trace_start)
            .any(|record| matches!(
                record.kind(),
                TraceRecordKind::PointerStreamClosed {
                    pointer_id: current,
                } if current.get() == pointer_id
            )),
        "terminal pointer input must close the canonical M4 pointer stream"
    );
}

fn current_scrollbar_semantic_target(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
    label: &str,
) -> (runenui_core::SurfaceId, runenui_core::SemanticNodeId) {
    let publication = runtime
        .publish_surface(&build(environment))
        .unwrap_or_else(|_| unreachable!("current scrollbar semantics republish"));
    let semantics = publication.semantic_publication().snapshot();
    let scrollbar = semantics
        .nodes()
        .iter()
        .find(|node| node.name() == Some(label))
        .unwrap_or_else(|| unreachable!("current scrollbar semantic node is published"));
    (semantics.surface_id().clone(), scrollbar.id().clone())
}

const fn keyboard(
    logical: runenui_core::LogicalKey,
    physical: PhysicalKey,
    modifiers: KeyModifiers,
) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        physical,
        logical,
        modifiers,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn pointer(
    id: u64,
    device: PointerDeviceKind,
    phase: PointerPhase,
    x: f32,
    y: f32,
    context: runenui_core::SurfaceInputContext,
) -> PointerEvent {
    let pointer_id =
        PointerId::new(id).unwrap_or_else(|| unreachable!("fixture pointer identity is non-zero"));
    let point =
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("fixture pointer point is finite"));
    let mut event = PointerEvent::new(pointer_id, device, phase, point, context);
    if matches!(phase, PointerPhase::Down | PointerPhase::Up) {
        event = event.with_changed_button(PointerButton::Primary);
    }
    if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
        event = event.with_buttons(PointerButtons::new([PointerButton::Primary]));
    }
    event
}

fn scrollable_vertical_runtime() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        content_width: 80.0,
        content_height: 200.0,
        horizontal: false,
        vertical: true,
        always: false,
    });
    settle(&mut runtime);
    runtime
}

fn scrollable_horizontal_runtime() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        content_width: 200.0,
        content_height: 80.0,
        horizontal: true,
        vertical: false,
        always: false,
    });
    settle(&mut runtime);
    runtime
}

fn pump_one(runtime: &mut AppRuntime<App>) {
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn assert_vertical_scrollbar_keyboard(runtime: &mut AppRuntime<App>, owner: &MountedNodeId) {
    for (logical, physical, modifiers, expected_y) in [
        (
            runenui_core::LogicalKey::ArrowDown,
            PhysicalKey::ArrowDown,
            KeyModifiers::NONE,
            5.0,
        ),
        (
            runenui_core::LogicalKey::End,
            PhysicalKey::End,
            KeyModifiers::NONE,
            100.0,
        ),
        (
            runenui_core::LogicalKey::Home,
            PhysicalKey::Home,
            KeyModifiers::NONE,
            0.0,
        ),
        (
            runenui_core::LogicalKey::Space,
            PhysicalKey::Space,
            KeyModifiers::NONE,
            100.0,
        ),
        (
            runenui_core::LogicalKey::Space,
            PhysicalKey::Space,
            KeyModifiers::SHIFT,
            0.0,
        ),
    ] {
        runtime
            .submit_keyboard(keyboard(logical, physical, modifiers))
            .unwrap_or_else(|_| unreachable!("owned scrollbar key is accepted"));
        settle(runtime);
        assert_eq!(offset(runtime, owner).1, expected_y);
    }

    runtime
        .submit_keyboard(keyboard(
            runenui_core::LogicalKey::ArrowDown,
            PhysicalKey::ArrowDown,
            KeyModifiers::SHIFT,
        ))
        .unwrap_or_else(|_| unreachable!("unowned modified key is still valid ingress"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 0.0);
}

fn assert_vertical_scrollbar_accessibility(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
    owner: &MountedNodeId,
) {
    let semantic_publication = runtime
        .publish_surface(&build(environment))
        .unwrap_or_else(|_| unreachable!("current scrollbar semantics republish"));
    let semantics = semantic_publication.semantic_publication().snapshot();
    let scrollbar = semantics
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Vertical scroll"))
        .unwrap_or_else(|| unreachable!("standard scrollbar semantic node is published"));
    let surface = semantics.surface_id().clone();
    let semantic_id = scrollbar.id().clone();
    assert_eq!(scrollbar.role(), SemanticRole::ScrollBar);
    for action in [
        SemanticAction::Increment,
        SemanticAction::Decrement,
        SemanticAction::SetValue,
    ] {
        assert!(
            scrollbar.supported_actions().contains(&action),
            "current scrollable scrollbar publishes {action:?}"
        );
    }

    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            semantic_id,
            SemanticAction::Increment,
        ))
        .unwrap_or_else(|_| unreachable!("current scrollbar increment is published"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 5.0);

    let (surface, semantic_id) =
        current_scrollbar_semantic_target(runtime, environment, "Vertical scroll");
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            surface,
            semantic_id,
            number(50.0),
        ))
        .unwrap_or_else(|_| unreachable!("scrollbar set-value is published"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 50.0);

    let (surface, semantic_id) =
        current_scrollbar_semantic_target(runtime, environment, "Vertical scroll");
    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            semantic_id,
            SemanticAction::Decrement,
        ))
        .unwrap_or_else(|_| unreachable!("scrollbar decrement is published"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 45.0);
}

#[test]
fn standard_scrollbar_keyboard_and_accessibility_converge_on_m10_scroll_state() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("standard scrollbar fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let bar = node_id(&mut runtime, "standard.vertical");

    runtime
        .submit_command(
            bar,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("scrollbar focus request is accepted"));
    settle(&mut runtime);

    assert_vertical_scrollbar_keyboard(&mut runtime, &owner);
    assert_vertical_scrollbar_accessibility(&mut runtime, &environment, &owner);
}

#[test]
fn standard_horizontal_scrollbar_owns_horizontal_arrows_and_pages_along_its_axis() {
    let mut runtime = scrollable_horizontal_runtime();
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("horizontal standard scrollbar publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let bar = node_id(&mut runtime, "standard.horizontal");

    runtime
        .submit_command(
            bar,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("horizontal scrollbar focus request is accepted"));
    settle(&mut runtime);

    for (logical, physical, modifiers, expected_x) in [
        (
            runenui_core::LogicalKey::ArrowRight,
            PhysicalKey::ArrowRight,
            KeyModifiers::NONE,
            5.0,
        ),
        (
            runenui_core::LogicalKey::ArrowLeft,
            PhysicalKey::ArrowLeft,
            KeyModifiers::NONE,
            0.0,
        ),
        (
            runenui_core::LogicalKey::PageDown,
            PhysicalKey::PageDown,
            KeyModifiers::NONE,
            100.0,
        ),
        (
            runenui_core::LogicalKey::PageUp,
            PhysicalKey::PageUp,
            KeyModifiers::NONE,
            0.0,
        ),
        (
            runenui_core::LogicalKey::End,
            PhysicalKey::End,
            KeyModifiers::NONE,
            100.0,
        ),
        (
            runenui_core::LogicalKey::Home,
            PhysicalKey::Home,
            KeyModifiers::NONE,
            0.0,
        ),
        (
            runenui_core::LogicalKey::Space,
            PhysicalKey::Space,
            KeyModifiers::NONE,
            100.0,
        ),
        (
            runenui_core::LogicalKey::Space,
            PhysicalKey::Space,
            KeyModifiers::SHIFT,
            0.0,
        ),
    ] {
        runtime
            .submit_keyboard(keyboard(logical, physical, modifiers))
            .unwrap_or_else(|_| unreachable!("owned horizontal scrollbar key is accepted"));
        settle(&mut runtime);
        assert_eq!(offset(&mut runtime, &owner).0, expected_x);
    }

    runtime
        .submit_keyboard(keyboard(
            runenui_core::LogicalKey::ArrowUp,
            PhysicalKey::ArrowUp,
            KeyModifiers::NONE,
        ))
        .unwrap_or_else(|_| unreachable!("unowned vertical arrow remains valid ingress"));
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).0, 0.0);
}

fn reset_vertical_scrollbar(
    runtime: &mut AppRuntime<App>,
    bar: &MountedNodeId,
    owner: &MountedNodeId,
) {
    runtime
        .submit_command(
            bar.clone(),
            SemanticCommand::ScrollControl(ScrollControlRequest::ToStart),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("standard bar accepts canonical reset request"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 0.0);
}

fn assert_mouse_scrollbar_track_and_thumb(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
    owner: &MountedNodeId,
    bar: &MountedNodeId,
) {
    let first = runtime
        .publish_surface(&build(environment))
        .unwrap_or_else(|_| unreachable!("standard pointer fixture publishes"));
    let first_context = first.input_context().clone();
    runtime
        .submit_pointer(pointer(
            11,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            95.0,
            75.0,
            first_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("track pointer down is admitted"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 100.0);
    runtime
        .submit_pointer(pointer(
            11,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            95.0,
            75.0,
            first_context,
        ))
        .unwrap_or_else(|_| unreachable!("track pointer up is admitted"));
    settle(runtime);

    reset_vertical_scrollbar(runtime, bar, owner);
    let drag_start = runtime
        .publish_surface(&build(environment))
        .unwrap_or_else(|_| unreachable!("reset thumb geometry republishes"));
    let drag_context = drag_start.input_context().clone();
    let thumb = only_child_id(runtime, bar);
    let mouse_down_trace = runtime.trace().len();
    runtime
        .submit_pointer(pointer(
            12,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            95.0,
            25.0,
            drag_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("thumb mouse down is admitted"));
    settle(runtime);
    assert_pointer_down_default_committed_on(runtime, mouse_down_trace, 12, &thumb);
    runtime
        .submit_pointer(pointer(
            12,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            95.0,
            50.0,
            drag_context,
        ))
        .unwrap_or_else(|_| unreachable!("captured thumb move is admitted"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 50.0);

    let moved = runtime
        .publish_surface(&build(environment))
        .unwrap_or_else(|_| unreachable!("mid-drag geometry republishes"));
    let moved_context = moved.input_context().clone();
    runtime
        .submit_pointer(pointer(
            12,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            95.0,
            75.0,
            moved_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("captured move uses current thumb geometry"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 100.0);
    let mouse_up_trace = runtime.trace().len();
    runtime
        .submit_pointer(pointer(
            12,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            95.0,
            75.0,
            moved_context,
        ))
        .unwrap_or_else(|_| unreachable!("captured mouse up is admitted"));
    settle(runtime);
    assert_pointer_stream_closed(runtime, mouse_up_trace, 12);
}

fn assert_touch_scrollbar_thumb(
    runtime: &mut AppRuntime<App>,
    environment: &StyleEnvironment,
    owner: &MountedNodeId,
    bar: &MountedNodeId,
) {
    reset_vertical_scrollbar(runtime, bar, owner);
    let touch_surface = runtime
        .publish_surface(&build(environment))
        .unwrap_or_else(|_| unreachable!("touch thumb geometry republishes"));
    let touch_context = touch_surface.input_context().clone();
    let thumb = only_child_id(runtime, bar);
    let touch_down_trace = runtime.trace().len();
    runtime
        .submit_pointer(pointer(
            14,
            PointerDeviceKind::Touch,
            PointerPhase::Down,
            95.0,
            25.0,
            touch_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("canonical touch thumb down is admitted"));
    settle(runtime);
    assert_pointer_down_default_committed_on(runtime, touch_down_trace, 14, &thumb);
    runtime
        .submit_pointer(pointer(
            14,
            PointerDeviceKind::Touch,
            PointerPhase::Move,
            95.0,
            50.0,
            touch_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("captured touch move is admitted"));
    settle(runtime);
    assert_eq!(offset(runtime, owner).1, 50.0);
    let touch_cancel_trace = runtime.trace().len();
    runtime
        .submit_pointer(pointer(
            14,
            PointerDeviceKind::Touch,
            PointerPhase::Cancel,
            95.0,
            50.0,
            touch_context,
        ))
        .unwrap_or_else(|_| unreachable!("touch cancel is admitted"));
    settle(runtime);
    assert_pointer_stream_closed(runtime, touch_cancel_trace, 14);
}

#[test]
fn standard_scrollbar_track_and_thumb_use_one_shot_paging_and_captured_drag_for_mouse_and_touch() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    let owner = node_id(&mut runtime, "standard.container");
    let bar = node_id(&mut runtime, "standard.vertical");

    assert_mouse_scrollbar_track_and_thumb(&mut runtime, &environment, &owner, &bar);
    assert_touch_scrollbar_thumb(&mut runtime, &environment, &owner, &bar);
}

#[test]
fn standard_track_click_pages_once_toward_pointer_without_warping() {
    let mut runtime = AppRuntime::<App>::mount(State {
        content_width: 80.0,
        content_height: 300.0,
        horizontal: false,
        vertical: true,
        always: false,
    });
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    let initial = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("track paging fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");

    runtime
        .submit_pointer(pointer(
            51,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            95.0,
            90.0,
            initial.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("after-thumb track down is admitted"));
    settle(&mut runtime);
    assert_eq!(
        offset(&mut runtime, &owner).1,
        100.0,
        "track click advances exactly one page instead of warping toward the pointer"
    );
    runtime
        .submit_pointer(pointer(
            51,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            95.0,
            90.0,
            initial.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("after-thumb track up is admitted"));
    settle(&mut runtime);

    let advanced = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("advanced track geometry republishes"));
    runtime
        .submit_pointer(pointer(
            52,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            95.0,
            5.0,
            advanced.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("before-thumb track down is admitted"));
    settle(&mut runtime);
    assert_eq!(
        offset(&mut runtime, &owner).1,
        0.0,
        "track click before the thumb moves exactly one page backward"
    );
    runtime
        .submit_pointer(pointer(
            52,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            95.0,
            5.0,
            advanced.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("before-thumb track up is admitted"));
    settle(&mut runtime);
}

#[test]
fn standard_thumb_capture_is_cleared_on_removal_and_does_not_retarget_after_replacement() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    let first = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("capture-removal fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let first_context = first.input_context().clone();

    runtime
        .submit_pointer(pointer(
            41,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            95.0,
            25.0,
            first_context,
        ))
        .unwrap_or_else(|_| unreachable!("thumb down is admitted before removal"));
    settle(&mut runtime);

    let trace_start = runtime.trace().len();
    runtime
        .submit_action(Action::SetVertical(false))
        .unwrap_or_else(|_| unreachable!("bar removal enters the FIFO"));
    settle(&mut runtime);
    assert!(
        runtime
            .trace()
            .records()
            .skip(trace_start)
            .any(|record| matches!(
                record.kind(),
                TraceRecordKind::PointerCaptureNotificationResolved {
                    kind: PointerCaptureKind::Lost,
                }
            )),
        "removing the captured thumb resolves the existing capture lifetime exactly once"
    );

    let without_bar = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("bar removal republishes"));
    assert_eq!(
        without_bar
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::ScrollBar)
            .count(),
        0
    );
    assert_eq!(offset(&mut runtime, &owner), (0.0, 0.0));

    runtime
        .submit_action(Action::SetVertical(true))
        .unwrap_or_else(|_| unreachable!("bar replacement enters the FIFO"));
    settle(&mut runtime);
    let replacement = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("bar replacement republishes"));
    assert_eq!(
        replacement
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::ScrollBar)
            .count(),
        1
    );

    runtime
        .submit_pointer(pointer(
            41,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            95.0,
            25.0,
            replacement.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("post-replacement move is admitted"));
    settle(&mut runtime);
    assert_eq!(
        offset(&mut runtime, &owner),
        (0.0, 0.0),
        "the retired drag cannot transfer to the replacement thumb"
    );

    runtime
        .submit_pointer(pointer(
            41,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            95.0,
            25.0,
            replacement.input_context().clone(),
        ))
        .unwrap_or_else(|_| unreachable!("post-replacement pointer stream closes"));
    settle(&mut runtime);
}

#[test]
fn standard_scrollbar_revalidates_current_metrics_before_semantic_scroll_and_set_value_range() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("semantic revalidation fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let scrollbar = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Vertical scroll"))
        .unwrap_or_else(|| unreachable!("standard scrollbar semantic node is published"));
    let surface = publication
        .semantic_publication()
        .snapshot()
        .surface_id()
        .clone();
    let semantic_id = scrollbar.id().clone();

    let out_of_range = runtime.submit_semantic_action(SemanticActionRequest::set_value(
        surface.clone(),
        semantic_id.clone(),
        number(101.0),
    ));
    let Err(out_of_range) = out_of_range else {
        unreachable!("out-of-range scrollbar SetValue must reject");
    };
    assert_eq!(
        out_of_range.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );

    runtime
        .submit_action(Action::SetContentHeight(100.0))
        .unwrap_or_else(|_| unreachable!("zero-range reconfiguration enters FIFO"));
    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            semantic_id,
            SemanticAction::Increment,
        ))
        .unwrap_or_else(|_| unreachable!("increment admits against current scrollable semantics"));

    pump_one(&mut runtime);
    assert_eq!(runtime.state().content_height, 100.0);
    runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("zero-range semantics republish"));

    pump_one(&mut runtime);
    assert_eq!(
        offset(&mut runtime, &owner),
        (0.0, 0.0),
        "stale semantic increment cannot create a scroll request after current range disappears"
    );
}

#[test]
fn standard_thumb_drag_recomputes_current_geometry_after_content_extent_changes() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    let first = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("dynamic drag fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let first_context = first.input_context().clone();

    runtime
        .submit_pointer(pointer(
            31,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            95.0,
            25.0,
            first_context,
        ))
        .unwrap_or_else(|_| unreachable!("dynamic drag thumb down is admitted"));
    settle(&mut runtime);

    runtime
        .submit_action(Action::SetContentHeight(300.0))
        .unwrap_or_else(|_| unreachable!("content extent update enters FIFO"));
    settle(&mut runtime);
    let resized = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("resized content republishes during capture"));
    let resized_context = resized.input_context().clone();

    runtime
        .submit_pointer(pointer(
            31,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            95.0,
            50.0,
            resized_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("captured move after metric change is admitted"));
    settle(&mut runtime);
    assert!(
        (offset(&mut runtime, &owner).1 - 75.0).abs() <= 1.0e-4,
        "drag uses current 300px content / 100px viewport geometry, not stale 200px geometry"
    );

    runtime
        .submit_pointer(pointer(
            31,
            PointerDeviceKind::Mouse,
            PointerPhase::Cancel,
            95.0,
            50.0,
            resized_context,
        ))
        .unwrap_or_else(|_| unreachable!("dynamic drag cancel is admitted"));
    settle(&mut runtime);
}

#[test]
fn automatic_reserved_bars_reach_the_two_axis_fixed_point_when_one_bar_causes_the_other() {
    let mut runtime = AppRuntime::<App>::mount(State {
        content_width: 95.0,
        content_height: 110.0,
        horizontal: true,
        vertical: true,
        always: false,
    });
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("automatic two-axis fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let layout = publication
        .layout_report()
        .nodes()
        .iter()
        .find(|node| node.id() == &owner)
        .unwrap_or_else(|| unreachable!("scroll owner layout is published"));

    assert_eq!(layout.scroll_viewport_extent().width(), 90.0);
    assert_eq!(layout.scroll_viewport_extent().height(), 90.0);
    assert_eq!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::ScrollBar)
            .count(),
        2,
        "vertical overflow introduces the reserved width loss that makes the automatic horizontal bar necessary"
    );
}

#[test]
fn standard_scroll_container_keeps_zero_range_bar_nonfocusable_and_composes_noninteractive_corner()
{
    let mut zero = AppRuntime::<App>::mount(State {
        content_width: 80.0,
        content_height: 100.0,
        horizontal: false,
        vertical: true,
        always: true,
    });
    settle(&mut zero);
    let environment = StyleEnvironment::default();
    let zero_publication = zero
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("zero-range standard scrollbar publishes"));
    let vertical = node_id(&mut zero, "standard.vertical");
    assert!(
        zero.index()
            .node(&vertical)
            .is_some_and(|node| !node.is_focusable())
    );
    let zero_semantics = zero_publication.semantic_publication().snapshot();
    let zero_bar = zero_semantics
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Vertical scroll"))
        .unwrap_or_else(|| unreachable!("Always zero-range bar remains semantic"));
    assert_eq!(zero_bar.role(), SemanticRole::ScrollBar);
    assert!(
        !zero_bar
            .supported_actions()
            .contains(&SemanticAction::RequestFocus)
    );
    assert!(
        !zero_bar
            .supported_actions()
            .contains(&SemanticAction::Increment)
    );

    let mut both = AppRuntime::<App>::mount(State {
        content_width: 200.0,
        content_height: 200.0,
        horizontal: true,
        vertical: true,
        always: true,
    });
    settle(&mut both);
    let both_publication = both
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("two-axis standard container publishes"));
    let root = node_id(&mut both, "standard.container");
    assert_eq!(
        both.index().nodes().len(),
        7,
        "root + content + two tracks + two thumbs + one corner"
    );
    let corner =
        LogicalPoint::new(95.0, 95.0).unwrap_or_else(|_| unreachable!("corner probe is finite"));
    assert_eq!(
        both_publication.hit_test_scene().target_at(corner),
        Some(&root),
        "reserved corner contributes no interactive hit target"
    );
    assert_eq!(
        both_publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::ScrollBar)
            .count(),
        2
    );
}
