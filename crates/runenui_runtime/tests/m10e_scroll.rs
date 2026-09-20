#![allow(refining_impl_trait)]
#![allow(clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use runenui_core::{
    Brush, Color, CommandOrigin, Element, EventContext, FocusBoundaryPolicy, FocusScope,
    FocusScopePolicy, HitContribution, HitContributionContext, LayoutContainer, LayoutDimension,
    LayoutInsets, LayoutOffset, LayoutPosition, LayoutStyle, LogicalDelta, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, OverflowPolicy, OverflowStyle, PaintContribution,
    PaintContributionContext, PaintContributionItem, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SceneShape, SemanticAction,
    SemanticCommand, SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticRole, StyleEnvironment, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure, WidgetMeasureInput, children, row,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, SurfaceBuildContext, TraceConfig,
    TraceRecordKind,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is finite"))
}

fn dimension(value: f32) -> LayoutDimension {
    LayoutDimension::length(length(value))
}

#[derive(Debug)]
struct HitTarget {
    name: &'static str,
    width: f32,
    height: f32,
    color: Color,
    prevent_wheel: bool,
}

impl Widget<()> for HitTarget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if self.prevent_wheel
            && event
                .as_pointer()
                .is_some_and(|pointer| pointer.phase() == PointerPhase::Wheel)
        {
            context.prevent_default();
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(length(self.width), length(self.height))
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("fixture target bounds are valid")),
        )
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("fixture paint bounds are valid"));
        PaintContribution::single(PaintContributionItem::fill(
            SceneShape::rect(rect),
            Brush::solid(self.color),
        ))
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name(self.name)
                .with_action(SemanticAction::Activate),
        )
    }
}

struct NestedScrollApp;

impl UiApp for NestedScrollApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(prevent_wheel: &Self::State) -> Element<Self::Action> {
        let inner_content = Element::new(HitTarget {
            name: "nested content",
            width: 50.0,
            height: 50.0,
            color: Color::rgb(0xA0, 0x30, 0x30),
            prevent_wheel: *prevent_wheel,
        })
        .with_layout(
            LayoutStyle::default()
                .with_position(LayoutPosition::Absolute(LayoutInsets::new(
                    LayoutOffset::length(length(0.0)),
                    LayoutOffset::Auto,
                    LayoutOffset::Auto,
                    LayoutOffset::length(length(0.0)),
                )))
                .with_width(dimension(40.0))
                .with_height(dimension(50.0)),
        )
        .id("inner.content");
        let inner = row(children![inner_content])
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(dimension(40.0))
                    .with_height(dimension(30.0))
                    .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
            .id("inner")
            .into_element()
            .focus_scope(FocusScope::new().with_policy(FocusScopePolicy::new(
                FocusBoundaryPolicy::LogicalScroll,
                FocusBoundaryPolicy::LogicalScroll,
            )));
        let filler = Element::new(HitTarget {
            name: "outer filler",
            width: 40.0,
            height: 40.0,
            color: Color::rgb(0x30, 0xA0, 0x30),
            prevent_wheel: false,
        })
        .id("outer.filler");
        row(children![inner, filler])
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(dimension(40.0))
                    .with_height(dimension(40.0))
                    .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
            .id("outer")
            .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[derive(Clone, Debug, Default)]
struct TouchEventLog(Arc<Mutex<Vec<PointerPhase>>>);

#[derive(Debug)]
struct TouchProbe {
    events: TouchEventLog,
    capture_on_down: bool,
}

impl Widget<()> for TouchProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if let Some(pointer) = event.as_pointer() {
            self.events
                .0
                .lock()
                .unwrap_or_else(|_| unreachable!("test event log mutex is not poisoned"))
                .push(pointer.phase());
            if self.capture_on_down && pointer.phase() == PointerPhase::Down {
                context.capture_pointer();
            }
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(length(40.0), length(60.0))
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("touch probe bounds are valid")),
        )
    }
}

struct TouchScrollApp;

#[derive(Debug)]
enum TouchScrollAction {
    RemoveContent,
}

#[derive(Clone, Debug)]
struct TouchScrollState {
    events: TouchEventLog,
    content_visible: bool,
    capture_on_down: bool,
}

impl UiApp for TouchScrollApp {
    type State = TouchScrollState;
    type Action = TouchScrollAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let content = Element::new(TouchProbe {
            events: state.events.clone(),
            capture_on_down: state.capture_on_down,
        })
        .with_layout(
            LayoutStyle::default()
                .with_position(LayoutPosition::Absolute(LayoutInsets::new(
                    LayoutOffset::length(length(0.0)),
                    LayoutOffset::Auto,
                    LayoutOffset::Auto,
                    LayoutOffset::length(length(0.0)),
                )))
                .with_width(dimension(40.0))
                .with_height(dimension(60.0)),
        )
        .id("touch.content")
        .map_action(|()| TouchScrollAction::RemoveContent);
        let children: Vec<Element<TouchScrollAction>> = if state.content_visible {
            children![content]
        } else {
            children![]
        };
        row::<TouchScrollAction>(children)
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(dimension(40.0))
                    .with_height(dimension(30.0))
                    .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
            )
            .id("touch.viewport")
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            TouchScrollAction::RemoveContent => state.content_visible = false,
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // One route exercises the exact nested wheel chain and resulting geometry.
fn nested_wheel_consumes_exact_remainder_and_republishes_clipped_geometry() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<NestedScrollApp>::mount_with_config(false, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let initial = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("nested scroll surface publishes: {error:?}"));
    let point =
        LogicalPoint::new(5.0, 5.0).unwrap_or_else(|_| unreachable!("wheel position is finite"));
    let initial_target = initial
        .hit_test_scene()
        .target_at(point)
        .unwrap_or_else(|| unreachable!("nested content is initially hit-testable"))
        .clone();
    assert_eq!(
        initial_target,
        initial
            .frame()
            .nodes()
            .iter()
            .find(|node| node
                .authored_id()
                .is_some_and(|id| id.as_str() == "inner.content"))
            .unwrap_or_else(|| unreachable!("nested target is in the frame"))
            .id()
            .clone()
    );

    let wheel = PointerEvent::new(
        PointerId::new(41).unwrap_or_else(|| unreachable!("fixture pointer is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Wheel,
        point,
        initial.input_context().clone(),
    )
    .with_scroll_delta(
        LogicalDelta::new(15.0, 50.0).unwrap_or_else(|_| unreachable!("wheel delta is finite")),
    );
    runtime
        .submit_pointer(wheel)
        .unwrap_or_else(|error| panic!("wheel is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let applied = runtime
        .trace()
        .records()
        .filter_map(|record| {
            let TraceRecordKind::LogicalScrollOwnerApplied {
                evaluation_order,
                offered,
                consumed,
                remainder,
                maximum,
                ..
            } = record.kind()
            else {
                return None;
            };
            Some((
                *evaluation_order,
                offered.y(),
                consumed.y(),
                remainder.y(),
                maximum.y(),
                offered.x(),
                consumed.x(),
                remainder.x(),
                maximum.x(),
                record
                    .target()
                    .and_then(|target| target.authored_id())
                    .map(|id| id.as_str().to_owned()),
            ))
        })
        .collect::<Vec<_>>();
    assert_eq!(applied.len(), 2, "each nested scroll owner is evaluated");
    assert_eq!(
        applied[0],
        (
            0,
            50.0,
            20.0,
            30.0,
            20.0,
            15.0,
            10.0,
            5.0,
            10.0,
            Some("inner".to_owned())
        )
    );
    assert_eq!(
        applied[1],
        (
            1,
            30.0,
            30.0,
            0.0,
            30.0,
            5.0,
            0.0,
            5.0,
            0.0,
            Some("outer".to_owned())
        )
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::LogicalScrollChainCompleted { remainder }
            if remainder.x().to_bits() == 5.0_f32.to_bits()
                && remainder.y().to_bits() == 0.0_f32.to_bits()
    )));
    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"name\":\"logical_scroll_owner_applied\""));
    assert!(jsonl.contains("\"evaluation_order\":0"));
    assert!(jsonl.contains("\"name\":\"logical_scroll_chain_completed\""));
    assert!(jsonl.contains("\"remainder\":{\"x\":5,\"y\":0}"));

    let scrolled = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("scrolled publication commits: {error:?}"));
    assert_ne!(initial.paint_scene(), scrolled.paint_scene());
    let scrolled_target = scrolled.hit_test_scene().target_at(point);
    assert!(scrolled_target.is_some_and(|target| {
        scrolled
            .frame()
            .nodes()
            .iter()
            .find(|node| node.id() == target)
            .and_then(|node| node.authored_id())
            .is_some_and(|id| id.as_str() == "outer.filler")
    }));
    let semantics = scrolled.semantic_publication().snapshot();
    assert!(
        semantics.nodes().iter().any(|node| {
            node.name() == Some("nested content") && !node.bounds().contains(point)
        })
    );

    runtime
        .submit_pointer(
            PointerEvent::new(
                PointerId::new(43).unwrap_or_else(|| unreachable!("fixture pointer is non-zero")),
                PointerDeviceKind::Mouse,
                PointerPhase::Wheel,
                point,
                scrolled.input_context().clone(),
            )
            .with_scroll_delta(
                LogicalDelta::new(5.0, 0.0)
                    .unwrap_or_else(|_| unreachable!("overscroll delta is finite")),
            ),
        )
        .unwrap_or_else(|error| panic!("bounded overscroll input is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let no_visual_overscroll = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::LogicalScrollOwnerApplied { .. }
            )
        })
        .last()
        .unwrap_or_else(|| unreachable!("clamped owner records its exact zero consumption"));
    assert!(matches!(
        no_visual_overscroll.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied {
            offered,
            consumed,
            remainder,
            maximum,
            ..
        } if offered.x().to_bits() == 5.0_f32.to_bits()
            && consumed.x().to_bits() == 0.0_f32.to_bits()
            && remainder.x().to_bits() == 5.0_f32.to_bits()
            && maximum.x().to_bits() == 0.0_f32.to_bits()
    ));
    let unchanged = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("bounded overscroll publication commits: {error:?}"));
    assert_eq!(scrolled.paint_scene(), unchanged.paint_scene());
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}

#[test]
fn touch_scroll_commits_once_without_retargeting_the_threshold_crossing_move() {
    let state = TouchScrollState {
        events: TouchEventLog::default(),
        content_visible: true,
        capture_on_down: false,
    };
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<TouchScrollApp>::mount_with_config(state.clone(), config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("touch scroll surface publishes: {error:?}"));
    let pointer_id =
        PointerId::new(51).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Touch,
        PointerPhase::Down,
        LogicalPoint::new(20.0, 20.0).unwrap_or_else(|_| unreachable!("touch position is finite")),
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(down)
        .unwrap_or_else(|error| panic!("touch down is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let crossing_move = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Touch,
        PointerPhase::Move,
        LogicalPoint::new(14.0, 14.0).unwrap_or_else(|_| unreachable!("touch position is finite")),
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime
        .submit_pointer(crossing_move)
        .unwrap_or_else(|error| panic!("touch threshold move is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let later_move = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Touch,
        PointerPhase::Move,
        LogicalPoint::new(13.0, 13.0).unwrap_or_else(|_| unreachable!("touch position is finite")),
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime
        .submit_pointer(later_move)
        .unwrap_or_else(|error| panic!("touch winner move is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(
        state
            .events
            .0
            .lock()
            .unwrap_or_else(|_| unreachable!("test event log mutex is not poisoned"))
            .as_slice(),
        [PointerPhase::Down, PointerPhase::Move]
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::TouchGestureWon {
            pointer_id: id,
            gesture: runenui_runtime::TraceTouchGestureKind::Scroll,
        } if *id == pointer_id
    )));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied { .. }
    )));
    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"name\":\"touch_gesture_provisional\""));
    assert!(jsonl.contains("\"name\":\"touch_gesture_won\""));
    assert!(jsonl.contains("\"gesture\":\"scroll\""));
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}

#[test]
fn touch_gesture_is_cancelled_once_when_its_origin_is_unmounted() {
    let state = TouchScrollState {
        events: TouchEventLog::default(),
        content_visible: true,
        capture_on_down: false,
    };
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<TouchScrollApp>::mount_with_config(state, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("touch scroll surface publishes: {error:?}"));
    let pointer_id =
        PointerId::new(52).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Touch,
        PointerPhase::Down,
        LogicalPoint::new(5.0, 20.0).unwrap_or_else(|_| unreachable!("touch position is finite")),
        publication.input_context().clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(down)
        .unwrap_or_else(|error| panic!("touch down is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_action(TouchScrollAction::RemoveContent)
        .unwrap_or_else(|error| panic!("content removal action is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let cancellations = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id: id,
                    gesture: runenui_runtime::TraceTouchGestureKind::Tap,
                } if *id == pointer_id
            )
        })
        .count();
    assert_eq!(cancellations, 1);
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}

#[test]
#[allow(clippy::too_many_lines)] // Two independent streams are driven through a complete competitor resolution.
fn independent_touch_streams_resolve_scroll_and_tap_without_cross_ownership() {
    let state = TouchScrollState {
        events: TouchEventLog::default(),
        content_visible: true,
        capture_on_down: false,
    };
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<TouchScrollApp>::mount_with_config(state, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("touch scroll surface publishes: {error:?}"));
    let scroll_pointer =
        PointerId::new(61).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let tap_pointer =
        PointerId::new(62).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let point =
        LogicalPoint::new(5.0, 20.0).unwrap_or_else(|_| unreachable!("touch position is finite"));
    for pointer_id in [scroll_pointer, tap_pointer] {
        runtime
            .submit_pointer(
                PointerEvent::new(
                    pointer_id,
                    PointerDeviceKind::Touch,
                    PointerPhase::Down,
                    point,
                    publication.input_context().clone(),
                )
                .with_buttons(PointerButtons::new([PointerButton::Primary]))
                .with_changed_button(PointerButton::Primary),
            )
            .unwrap_or_else(|error| panic!("independent touch down is admitted: {error:?}"));
        runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    }
    runtime
        .submit_pointer(
            PointerEvent::new(
                scroll_pointer,
                PointerDeviceKind::Touch,
                PointerPhase::Move,
                LogicalPoint::new(5.0, 9.0)
                    .unwrap_or_else(|_| unreachable!("touch position is finite")),
                publication.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|error| panic!("scrolling touch move is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_pointer(
            PointerEvent::new(
                scroll_pointer,
                PointerDeviceKind::Touch,
                PointerPhase::Up,
                LogicalPoint::new(5.0, 9.0)
                    .unwrap_or_else(|_| unreachable!("touch position is finite")),
                publication.input_context().clone(),
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("scrolling touch release is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_pointer(
            PointerEvent::new(
                tap_pointer,
                PointerDeviceKind::Touch,
                PointerPhase::Up,
                point,
                publication.input_context().clone(),
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("tap release is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let trace = runtime.trace();
    let scroll_completed = trace.records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::TouchGestureCompleted {
                pointer_id,
                gesture: runenui_runtime::TraceTouchGestureKind::Scroll,
            } if *pointer_id == scroll_pointer
        )
    });
    let tap_completed = trace.records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::TouchGestureCompleted {
                pointer_id,
                gesture: runenui_runtime::TraceTouchGestureKind::Tap,
            } if *pointer_id == tap_pointer
        )
    });
    assert!(scroll_completed && tap_completed);
    assert!(trace.records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::TouchGestureWon {
            pointer_id,
            gesture: runenui_runtime::TraceTouchGestureKind::Tap,
        } if *pointer_id == tap_pointer
    )));
    assert_eq!(
        trace
            .records()
            .filter(|record| matches!(
                record.kind(),
                TraceRecordKind::TouchGestureCancelled {
                    pointer_id,
                    gesture: runenui_runtime::TraceTouchGestureKind::Scroll,
                } if *pointer_id == tap_pointer
            ))
            .count(),
        1
    );
    assert_eq!(
        trace
            .records()
            .filter(|record| matches!(
                record.kind(),
                TraceRecordKind::TouchGestureWon {
                    pointer_id,
                    gesture: runenui_runtime::TraceTouchGestureKind::Scroll,
                } if *pointer_id == scroll_pointer
            ))
            .count(),
        1
    );
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}

#[test]
fn committed_touch_capture_beats_scroll_and_retains_its_original_owner() {
    let state = TouchScrollState {
        events: TouchEventLog::default(),
        content_visible: true,
        capture_on_down: true,
    };
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<TouchScrollApp>::mount_with_config(state, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("touch scroll surface publishes: {error:?}"));
    let pointer_id =
        PointerId::new(63).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let point =
        LogicalPoint::new(5.0, 20.0).unwrap_or_else(|_| unreachable!("touch position is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Touch,
                PointerPhase::Down,
                point,
                publication.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("capturing touch down is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Touch,
                PointerPhase::Move,
                LogicalPoint::new(5.0, 9.0)
                    .unwrap_or_else(|_| unreachable!("touch position is finite")),
                publication.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|error| panic!("captured touch move is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::TouchGestureWon {
            pointer_id: id,
            gesture: runenui_runtime::TraceTouchGestureKind::Capture,
        } if *id == pointer_id
    )));
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied { .. }
    )));
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}

#[test]
fn touch_wheel_is_diagnosed_instead_of_creating_a_parallel_stream_profile() {
    let state = TouchScrollState {
        events: TouchEventLog::default(),
        content_visible: true,
        capture_on_down: false,
    };
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<TouchScrollApp>::mount_with_config(state, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("touch scroll surface publishes: {error:?}"));
    let pointer_id =
        PointerId::new(64).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Touch,
                PointerPhase::Wheel,
                LogicalPoint::new(5.0, 5.0)
                    .unwrap_or_else(|_| unreachable!("touch position is finite")),
                publication.input_context().clone(),
            )
            .with_scroll_delta(
                LogicalDelta::new(0.0, 10.0)
                    .unwrap_or_else(|_| unreachable!("touch wheel delta is finite")),
            ),
        )
        .unwrap_or_else(|error| {
            panic!("unsupported touch event is admitted for diagnosis: {error:?}")
        });
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerIngressRejected {
            pointer_id: id,
            phase: PointerPhase::Wheel,
            outcome: runenui_runtime::TracePointerRejection::TouchProfileUnsupported,
        } if *id == pointer_id
    )));
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerStreamRegistered { pointer_id: id, .. }
            if *id == pointer_id
    )));
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
}

#[test]
fn prevented_nested_wheel_does_not_partially_commit_scroll() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<NestedScrollApp>::mount_with_config(true, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let initial = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("nested scroll surface publishes: {error:?}"));
    let point =
        LogicalPoint::new(5.0, 5.0).unwrap_or_else(|_| unreachable!("wheel position is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                PointerId::new(42).unwrap_or_else(|| unreachable!("fixture pointer is non-zero")),
                PointerDeviceKind::Mouse,
                PointerPhase::Wheel,
                point,
                initial.input_context().clone(),
            )
            .with_scroll_delta(
                LogicalDelta::new(0.0, 50.0)
                    .unwrap_or_else(|_| unreachable!("wheel delta is finite")),
            ),
        )
        .unwrap_or_else(|error| panic!("wheel is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied { .. }
            | TraceRecordKind::LogicalScrollChainCompleted { .. }
    )));
    let after = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("unchanged publication commits: {error:?}"));
    assert_eq!(initial.paint_scene(), after.paint_scene());
    assert_eq!(
        initial.hit_test_scene().target_at(point),
        after.hit_test_scene().target_at(point)
    );
}

#[test]
fn logical_focus_scroll_moves_nearest_scroll_owner_and_commits_a_new_publication() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<NestedScrollApp>::mount_with_config(false, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let initial = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("nested scroll surface publishes: {error:?}"));
    let target = initial
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "inner.content")
        })
        .unwrap_or_else(|| unreachable!("focus target is published"))
        .id()
        .clone();
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("focus target accepts focus request: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::FocusDown,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("focus boundary request is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let applied = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::LogicalScrollOwnerApplied { .. }
            )
        })
        .unwrap_or_else(|| unreachable!("logical focus scroll records its committed owner"));
    assert_eq!(
        applied
            .target()
            .and_then(|target| target.authored_id())
            .map(runenui_core::ElementId::as_str),
        Some("inner")
    );
    assert!(matches!(
        applied.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied {
            consumed,
            maximum,
            ..
        } if consumed.y().to_bits() == 20.0_f32.to_bits()
            && maximum.y().to_bits() == 20.0_f32.to_bits()
    ));
    let scrolled = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("focus scroll republishes geometry: {error:?}"));
    assert_ne!(initial.paint_scene(), scrolled.paint_scene());
    assert_eq!(runtime.focus().focused_node(), Some(&target));
}

#[test]
fn scroll_into_view_uses_the_exact_target_and_clamps_to_the_nearest_scroll_owner() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<NestedScrollApp>::mount_with_config(false, config);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture surface size is finite")),
    );
    let initial = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("nested scroll surface publishes: {error:?}"));
    let target = initial
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "inner.content")
        })
        .unwrap_or_else(|| unreachable!("scroll target is published"))
        .id()
        .clone();

    runtime
        .submit_command(
            target,
            SemanticCommand::ScrollIntoView,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("scroll-to-target command is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let applied = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::LogicalScrollOwnerApplied { .. }
            )
        })
        .unwrap_or_else(|| unreachable!("scroll-to-target records its committed owner"));
    assert_eq!(
        applied
            .target()
            .and_then(|target| target.authored_id())
            .map(runenui_core::ElementId::as_str),
        Some("inner")
    );
    assert!(matches!(
        applied.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied {
            consumed,
            maximum,
            ..
        } if consumed.y().to_bits() == 20.0_f32.to_bits()
            && maximum.y().to_bits() == 20.0_f32.to_bits()
    ));
    let scrolled = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("scroll-to-target republishes geometry: {error:?}"));
    assert_ne!(initial.paint_scene(), scrolled.paint_scene());
}
