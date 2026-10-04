#![allow(refining_impl_trait)]

use runenui_core::{
    Axis, Color, CommandOrigin, Element, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength,
    LogicalPoint, NoHostProtocol, OverflowPolicy, OverflowStyle, PhysicalKey, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, ScrollBarVisibility,
    ScrollControlBinding, ScrollControlRequest, SemanticAction, SemanticActionRequest,
    SemanticCommand, SemanticNumber, SemanticRole, StyleEnvironment, StyleIntent, UiApp, View,
    Widget, WidgetMeasure, WidgetMeasureInput, scroll_bar, scroll_container,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is finite"))
}

fn dimension(value: f32) -> LayoutDimension {
    LayoutDimension::Length(length(value))
}

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("fixture number is finite"))
}

#[derive(Debug)]
struct Content {
    width: LogicalLength,
    height: LogicalLength,
}

impl Widget<()> for Content {
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
    type Action = ();
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
            .corner_style(
                StyleIntent::EMPTY.with_background(Color::rgba(30, 40, 50, 255)),
            );

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

    fn update(_: &mut Self::State, (): Self::Action) {}
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

fn build<'a>(environment: &'a StyleEnvironment) -> SurfaceBuildContext<'a> {
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

fn keyboard(logical: runenui_core::LogicalKey, physical: PhysicalKey, modifiers: KeyModifiers) -> KeyboardEvent {
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
    let pointer_id = PointerId::new(id)
        .unwrap_or_else(|| unreachable!("fixture pointer identity is non-zero"));
    let point = LogicalPoint::new(x, y)
        .unwrap_or_else(|_| unreachable!("fixture pointer point is finite"));
    let mut event = PointerEvent::new(pointer_id, device, phase, point, context);
    if device != PointerDeviceKind::Touch && matches!(phase, PointerPhase::Down | PointerPhase::Up)
    {
        event = event.with_changed_button(PointerButton::Primary);
    }
    if device != PointerDeviceKind::Touch && matches!(phase, PointerPhase::Down | PointerPhase::Move)
    {
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

#[test]
fn standard_scrollbar_keyboard_and_accessibility_converge_on_m10_scroll_state() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    let publication = runtime
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
        settle(&mut runtime);
        assert_eq!(offset(&mut runtime, &owner).1, expected_y);
    }

    runtime
        .submit_keyboard(keyboard(
            runenui_core::LogicalKey::ArrowDown,
            PhysicalKey::ArrowDown,
            KeyModifiers::SHIFT,
        ))
        .unwrap_or_else(|_| unreachable!("unowned modified key is still valid ingress"));
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 0.0);

    let semantics = publication.semantic_publication().snapshot();
    let scrollbar = semantics
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Vertical scroll"))
        .unwrap_or_else(|| unreachable!("standard scrollbar semantic node is published"));
    let surface = semantics.surface_id().clone();
    let semantic_id = scrollbar.id().clone();
    assert_eq!(scrollbar.role(), SemanticRole::ScrollBar);

    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface.clone(),
            semantic_id.clone(),
            SemanticAction::Increment,
        ))
        .unwrap_or_else(|_| unreachable!("scrollbar increment is published"));
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 5.0);

    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            surface.clone(),
            semantic_id.clone(),
            number(50.0),
        ))
        .unwrap_or_else(|_| unreachable!("scrollbar set-value is published"));
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 50.0);

    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            semantic_id,
            SemanticAction::Decrement,
        ))
        .unwrap_or_else(|_| unreachable!("scrollbar decrement is published"));
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 45.0);
}

#[test]
fn standard_scrollbar_track_and_thumb_use_one_shot_paging_and_captured_drag_for_mouse_and_touch() {
    let mut runtime = scrollable_vertical_runtime();
    let environment = StyleEnvironment::default();
    let first = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("standard pointer fixture publishes"));
    let owner = node_id(&mut runtime, "standard.container");
    let bar = node_id(&mut runtime, "standard.vertical");

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
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 100.0);
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
    settle(&mut runtime);

    runtime
        .submit_command(
            bar.clone(),
            SemanticCommand::ScrollControl(ScrollControlRequest::ToStart),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("standard bar accepts canonical reset request"));
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 0.0);

    let drag_start = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("reset thumb geometry republishes"));
    let drag_context = drag_start.input_context().clone();
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
    settle(&mut runtime);
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
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 50.0);

    let moved = runtime
        .publish_surface(&build(&environment))
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
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 100.0);
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
    settle(&mut runtime);

    runtime
        .submit_command(
            bar,
            SemanticCommand::ScrollControl(ScrollControlRequest::ToStart),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("standard bar resets before touch proof"));
    settle(&mut runtime);
    let touch_surface = runtime
        .publish_surface(&build(&environment))
        .unwrap_or_else(|_| unreachable!("touch thumb geometry republishes"));
    let touch_context = touch_surface.input_context().clone();
    runtime
        .submit_pointer(pointer(
            14,
            PointerDeviceKind::Touch,
            PointerPhase::Down,
            95.0,
            25.0,
            touch_context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("touch thumb down is admitted without mouse button facts"));
    settle(&mut runtime);
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
    settle(&mut runtime);
    assert_eq!(offset(&mut runtime, &owner).1, 50.0);
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
    settle(&mut runtime);
}

#[test]
fn standard_scroll_container_keeps_zero_range_bar_nonfocusable_and_composes_noninteractive_corner() {
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
    assert!(!zero_bar.supported_actions().contains(&SemanticAction::RequestFocus));
    assert!(!zero_bar.supported_actions().contains(&SemanticAction::Increment));

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
    let corner = LogicalPoint::new(95.0, 95.0)
        .unwrap_or_else(|_| unreachable!("corner probe is finite"));
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
