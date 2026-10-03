#![allow(refining_impl_trait)]
#![allow(clippy::panic)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Axis, CommandOrigin, Element, EventContext, HitContribution, HitContributionContext,
    KeyModifiers, LayoutContainer, LayoutDimension, LayoutStyle, LogicalDelta, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, OverflowPolicy, OverflowStyle, PaintContribution,
    PaintContributionContext, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent,
    PointerId, PointerPhase, ScrollControlBinding, ScrollControlRequest, ScrollControlSnapshot,
    ScrollNormalizedValue, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, StyleEnvironment, UiApp, UiEvent, View, Widget,
    WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput,
    WidgetMeasure, WidgetMeasureInput, column, scroll_viewport,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, RuntimeConfig, RuntimeLimits,
    SurfaceBuildContext, TraceConfig, TraceRecordKind, TraceRoutedAdmissionRejection,
    TraceScrollControlBindingOutcome, TraceTargetRejection,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is finite"))
}

fn dimension(value: f32) -> LayoutDimension {
    LayoutDimension::Length(length(value))
}

fn vertical_scroll() -> OverflowStyle {
    OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll)
}

fn block_size(width: f32, height: f32, overflow: OverflowStyle) -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Block)
        .with_width(dimension(width))
        .with_height(dimension(height))
        .with_overflow(overflow)
}

fn settle<App: UiApp>(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn node_id<App: UiApp>(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored =
        runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!("valid fixture id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture node is mounted"))
        .id()
        .clone()
}

fn scroll_offset<App: UiApp>(runtime: &mut AppRuntime<App>, authored: &str) -> (f32, f32) {
    let id = node_id(runtime, authored);
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.id() == &id)
        .unwrap_or_else(|| unreachable!("fixture scroll owner is mounted"))
        .interaction()
        .scroll_offset()
}

#[derive(Debug)]
struct HitProbe;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewportAction {
    ChildActivated,
}

impl Widget<ViewportAction> for HitProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        _: &mut WidgetActivationContext<ViewportAction>,
    ) -> WidgetActivationOutput<ViewportAction> {
        WidgetActivationOutput::action(ViewportAction::ChildActivated)
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("fixture hit bounds are finite")),
        )
    }
}

struct ViewportApp;

impl UiApp for ViewportApp {
    type State = usize;
    type Action = ViewportAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        let child = Element::new(HitProbe)
            .id("viewport.child")
            .key("viewport.child")
            .with_layout(
                LayoutStyle::default()
                    .with_width(dimension(20.0))
                    .with_height(dimension(60.0)),
            );
        scroll_viewport(child, vertical_scroll())
            .id("viewport")
            .key("viewport")
            .with_layout(block_size(40.0, 30.0, vertical_scroll()))
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let ViewportAction::ChildActivated = action;
        *state += 1;
    }
}

#[test]
fn standard_viewport_hits_blank_area_without_stealing_child_and_wheel_scrolls() {
    let mut runtime = AppRuntime::<ViewportApp>::mount(0);
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("scroll viewport publishes: {error:?}"));

    let child = node_id(&mut runtime, "viewport.child");
    let viewport = node_id(&mut runtime, "viewport");
    let child_point =
        LogicalPoint::new(5.0, 5.0).unwrap_or_else(|_| unreachable!("point is finite"));
    let blank_point =
        LogicalPoint::new(35.0, 5.0).unwrap_or_else(|_| unreachable!("point is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(child_point),
        Some(&child),
        "descendant hit contribution stays above the viewport owner"
    );
    assert_eq!(
        publication.hit_test_scene().target_at(blank_point),
        Some(&viewport),
        "blank viewport area routes to the scroll owner"
    );

    let pointer_id =
        PointerId::new(200).unwrap_or_else(|| unreachable!("fixture pointer id is nonzero"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                child_point,
                publication.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("child pointer down is admitted: {error:?}"));
    settle(&mut runtime);
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                child_point,
                publication.input_context().clone(),
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("child pointer up is admitted: {error:?}"));
    settle(&mut runtime);
    assert_eq!(
        *runtime.state(),
        1,
        "the viewport owner hit shell does not steal ordinary child activation"
    );

    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("post-activation viewport republishes: {error:?}"));
    let wheel = PointerEvent::new(
        PointerId::new(201).unwrap_or_else(|| unreachable!("pointer id is nonzero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Wheel,
        blank_point,
        publication.input_context().clone(),
    )
    .with_scroll_delta(
        LogicalDelta::new(0.0, 12.0).unwrap_or_else(|_| unreachable!("wheel delta is finite")),
    );
    runtime
        .submit_pointer(wheel)
        .unwrap_or_else(|error| panic!("blank-area wheel is admitted: {error:?}"));
    settle(&mut runtime);
    assert_eq!(scroll_offset(&mut runtime, "viewport"), (0.0, 12.0));

    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("shift-wheel viewport republishes: {error:?}"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                PointerId::new(203).unwrap_or_else(|| unreachable!("pointer id is nonzero")),
                PointerDeviceKind::Mouse,
                PointerPhase::Wheel,
                blank_point,
                publication.input_context().clone(),
            )
            .with_modifiers(KeyModifiers::SHIFT)
            .with_scroll_delta(
                LogicalDelta::new(0.0, 5.0)
                    .unwrap_or_else(|_| unreachable!("wheel delta is finite")),
            ),
        )
        .unwrap_or_else(|error| panic!("shift-wheel is admitted: {error:?}"));
    settle(&mut runtime);
    assert_eq!(
        scroll_offset(&mut runtime, "viewport"),
        (0.0, 17.0),
        "Scroll A preserves the accepted exact wheel axes and does not add Shift remapping"
    );
}

#[test]
fn blank_viewport_touch_pan_uses_the_existing_m10_scroll_gesture() {
    let mut runtime = AppRuntime::<ViewportApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_trace_config(TraceConfig::new(256)),
    );
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(40.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|error| panic!("touch viewport publishes: {error:?}"));
    let viewport = node_id(&mut runtime, "viewport");
    let pointer_id =
        PointerId::new(202).unwrap_or_else(|| unreachable!("fixture pointer id is nonzero"));
    let down_point =
        LogicalPoint::new(35.0, 20.0).unwrap_or_else(|_| unreachable!("point is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(down_point),
        Some(&viewport),
        "blank viewport area is the physical touch target"
    );

    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Touch,
                PointerPhase::Down,
                down_point,
                publication.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("blank-area touch down is admitted: {error:?}"));
    settle(&mut runtime);

    for point in [(29.0, 14.0), (28.0, 13.0)] {
        runtime
            .submit_pointer(
                PointerEvent::new(
                    pointer_id,
                    PointerDeviceKind::Touch,
                    PointerPhase::Move,
                    LogicalPoint::new(point.0, point.1)
                        .unwrap_or_else(|_| unreachable!("point is finite")),
                    publication.input_context().clone(),
                )
                .with_buttons(PointerButtons::new([PointerButton::Primary])),
            )
            .unwrap_or_else(|error| panic!("blank-area touch move is admitted: {error:?}"));
        settle(&mut runtime);
    }

    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::TouchGestureWon {
            pointer_id: id,
            gesture: runenui_runtime::TraceTouchGestureKind::Scroll,
        } if *id == pointer_id
    )));
    assert!(
        scroll_offset(&mut runtime, "viewport").1 > 0.0,
        "the existing M10 touch-scroll winner mutates the viewport's canonical mounted offset"
    );
}

#[derive(Debug)]
struct Spacer;

impl Widget<()> for Spacer {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

#[derive(Debug)]
struct SnapshotProbe {
    hit: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    paint: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    semantics: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    event: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
}

impl Widget<()> for SnapshotProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(length(20.0), length(100.0))
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        _: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if let Some(snapshot) = context.scroll_control_snapshot() {
            self.event.borrow_mut().push(snapshot);
        }
        WidgetEventOutput::none()
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        if let Some(snapshot) = context.scroll_control_snapshot() {
            self.hit.borrow_mut().push(snapshot);
        }
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("bound-control hit bounds are finite")),
        )
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        if let Some(snapshot) = context.scroll_control_snapshot() {
            self.paint.borrow_mut().push(snapshot);
        }
        PaintContribution::empty()
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        if let Some(snapshot) = context.scroll_control_snapshot() {
            self.semantics.borrow_mut().push(snapshot);
        }
        SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Generic))
    }
}

#[derive(Clone, Copy, Debug)]
enum BoundAction {
    ResizeInner,
    EnableInnerScroll,
    DisableInnerScroll,
    ReplaceBound,
}

#[derive(Debug)]
struct BoundState {
    inner_height: f32,
    inner_scrollable: bool,
    replaced: bool,
    hit: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    paint: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    semantics: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    event: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
}

struct BoundApp;

impl UiApp for BoundApp {
    type State = BoundState;
    type Action = BoundAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let binding = ScrollControlBinding::new(Axis::Vertical, length(5.0))
            .unwrap_or_else(|_| unreachable!("positive fixture step is valid"));
        let bound = Element::new(SnapshotProbe {
            hit: Rc::clone(&state.hit),
            paint: Rc::clone(&state.paint),
            semantics: Rc::clone(&state.semantics),
            event: Rc::clone(&state.event),
        })
        .id("bound")
        .key(if state.replaced {
            "bound.replaced"
        } else {
            "bound.original"
        })
        .with_layout(
            LayoutStyle::default()
                .with_width(dimension(20.0))
                .with_height(dimension(100.0)),
        )
        .scroll_control(binding)
        .map_action(|()| BoundAction::ReplaceBound);

        let inner_overflow = if state.inner_scrollable {
            vertical_scroll()
        } else {
            OverflowStyle::all(OverflowPolicy::Clip)
        };
        let inner = scroll_viewport(bound, inner_overflow)
            .id("inner")
            .key("inner")
            .with_layout(block_size(30.0, state.inner_height, inner_overflow))
            .into_element();
        let filler = Element::new(Spacer)
            .id("outer.filler")
            .key("outer.filler")
            .with_layout(
                LayoutStyle::default()
                    .with_width(dimension(30.0))
                    .with_height(dimension(60.0)),
            )
            .map_action(|()| BoundAction::ReplaceBound);
        let outer_content = column(vec![inner, filler])
            .key("outer.content")
            .into_element();
        scroll_viewport(outer_content, vertical_scroll())
            .id("outer")
            .key("outer")
            .with_layout(block_size(40.0, 40.0, vertical_scroll()))
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            BoundAction::ResizeInner => state.inner_height = 40.0,
            BoundAction::EnableInnerScroll => state.inner_scrollable = true,
            BoundAction::DisableInnerScroll => state.inner_scrollable = false,
            BoundAction::ReplaceBound => state.replaced = true,
        }
    }
}

fn bound_runtime_with_scrollability(inner_scrollable: bool) -> AppRuntime<BoundApp> {
    AppRuntime::<BoundApp>::mount_with_config(
        BoundState {
            inner_height: 30.0,
            inner_scrollable,
            replaced: false,
            hit: Rc::new(RefCell::new(Vec::new())),
            paint: Rc::new(RefCell::new(Vec::new())),
            semantics: Rc::new(RefCell::new(Vec::new())),
            event: Rc::new(RefCell::new(Vec::new())),
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(1024)),
    )
}

fn bound_runtime() -> AppRuntime<BoundApp> {
    bound_runtime_with_scrollability(true)
}

fn bound_build<'a>(environment: &'a StyleEnvironment) -> SurfaceBuildContext<'a> {
    SurfaceBuildContext::tight(
        environment,
        LogicalSize::try_new(40.0, 40.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    )
}

fn submit_scroll(
    runtime: &mut AppRuntime<BoundApp>,
    target: MountedNodeId,
    request: ScrollControlRequest,
) {
    runtime
        .submit_command(
            target,
            SemanticCommand::ScrollControl(request),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("scroll-control request is admitted: {error:?}"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn last_snapshot(values: &Rc<RefCell<Vec<ScrollControlSnapshot>>>) -> ScrollControlSnapshot {
    *values
        .borrow()
        .last()
        .unwrap_or_else(|| unreachable!("fixture contribution observed a scroll snapshot"))
}

#[test]
fn bound_control_uses_nearest_owner_projects_snapshots_and_revalidates_processing_metrics() {
    let mut runtime = bound_runtime();
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("bound scroll surface publishes: {error:?}"));
    let bound = node_id(&mut runtime, "bound");

    let hit = last_snapshot(&runtime.state().hit);
    let paint = last_snapshot(&runtime.state().paint);
    let semantics = last_snapshot(&runtime.state().semantics);
    assert_eq!(hit, paint);
    assert_eq!(paint, semantics);
    assert_eq!(paint.axis(), Axis::Vertical);
    assert_eq!(paint.offset().get(), 0.0);
    assert_eq!(paint.maximum_offset().get(), 70.0);
    assert_eq!(paint.viewport_extent().get(), 30.0);
    assert_eq!(paint.content_extent().get(), 100.0);
    assert_eq!(paint.normalized_position(), ScrollNormalizedValue::ZERO);
    assert_eq!(
        paint.visible_fraction(),
        ScrollNormalizedValue::new(0.3)
            .unwrap_or_else(|_| unreachable!("fixture fraction is normalized"))
    );

    runtime
        .submit_action(BoundAction::ResizeInner)
        .unwrap_or_else(|_| unreachable!("resize action is admitted"));
    runtime
        .submit_command(
            bound.clone(),
            SemanticCommand::ScrollControl(ScrollControlRequest::PageForward),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("queued page request is admitted before resize"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("resized scroll surface publishes: {error:?}"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(
        scroll_offset(&mut runtime, "inner"),
        (0.0, 40.0),
        "queued request re-reads the resized viewport at processing time"
    );
    let event = last_snapshot(&runtime.state().event);
    assert_eq!(event.viewport_extent().get(), 40.0);
    assert_eq!(event.maximum_offset().get(), 60.0);
    assert_eq!(
        scroll_offset(&mut runtime, "outer"),
        (0.0, 0.0),
        "nearest eligible ancestor owns the request"
    );

    submit_scroll(&mut runtime, bound.clone(), ScrollControlRequest::ToStart);
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::SmallStepForward,
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 5.0));
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::SmallStepBackward,
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    submit_scroll(&mut runtime, bound.clone(), ScrollControlRequest::ToEnd);
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 60.0));
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::SmallStepForward,
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 60.0));
    let clamped = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::LogicalScrollOwnerApplied { .. }
            )
        })
        .last()
        .unwrap_or_else(|| {
            unreachable!("clamped scroll request retains the canonical owner trace")
        });
    assert!(matches!(
        clamped.kind(),
        TraceRecordKind::LogicalScrollOwnerApplied {
            offered,
            consumed,
            remainder,
            ..
        } if offered.y().to_bits() == 5.0_f32.to_bits()
            && consumed.y().to_bits() == 0.0_f32.to_bits()
            && remainder.y().to_bits() == 5.0_f32.to_bits()
    ));
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::SetNormalized(
            ScrollNormalizedValue::new(0.5).unwrap_or_else(|_| unreachable!("half is normalized")),
        ),
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 30.0));
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::PageBackward,
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    submit_scroll(&mut runtime, bound, ScrollControlRequest::PageForward);
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 40.0));

    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("scrolled surface republishes: {error:?}"));
    let latest = last_snapshot(&runtime.state().paint);
    let latest_hit = last_snapshot(&runtime.state().hit);
    let latest_semantics = last_snapshot(&runtime.state().semantics);
    assert_eq!(latest_hit, latest);
    assert_eq!(latest_semantics, latest);
    assert_eq!(latest.offset().get(), 40.0);
    assert_eq!(latest.maximum_offset().get(), 60.0);
    assert_eq!(latest.viewport_extent().get(), 40.0);
    assert_eq!(latest.content_extent().get(), 100.0);
    assert_eq!(
        latest.normalized_position(),
        ScrollNormalizedValue::new(2.0 / 3.0)
            .unwrap_or_else(|_| unreachable!("fixture position is normalized"))
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated {
            outcome: TraceScrollControlBindingOutcome::Resolved,
            axis: Some(Axis::Vertical),
            ..
        }
    )));
    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"name\":\"scroll_control_binding_evaluated\""));
    assert!(jsonl.contains("\"operation\":\"page_forward\""));
    assert!(jsonl.contains("\"axis\":\"vertical\""));
    assert!(jsonl.contains("\"outcome\":\"resolved\""));
}

struct PerAxisApp;

impl UiApp for PerAxisApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        let target = Element::new(Spacer)
            .id("axis.bound")
            .key("axis.bound")
            .with_layout(
                LayoutStyle::default()
                    .with_width(dimension(80.0))
                    .with_height(dimension(60.0)),
            )
            .scroll_control(
                ScrollControlBinding::new(Axis::Horizontal, length(5.0))
                    .unwrap_or_else(|_| unreachable!("fixture binding is valid")),
            );
        let inner = scroll_viewport(
            target,
            OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll),
        )
        .id("axis.inner")
        .key("axis.inner")
        .with_layout(block_size(
            80.0,
            30.0,
            OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Scroll),
        ))
        .into_element();
        scroll_viewport(
            inner,
            OverflowStyle::new(OverflowPolicy::Scroll, OverflowPolicy::Clip),
        )
        .id("axis.outer")
        .key("axis.outer")
        .with_layout(block_size(
            40.0,
            30.0,
            OverflowStyle::new(OverflowPolicy::Scroll, OverflowPolicy::Clip),
        ))
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn bound_request_fails_closed_until_changed_layout_metrics_are_republished() {
    let mut runtime = bound_runtime();
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("stale-metrics fixture publishes: {error:?}"));
    let bound = node_id(&mut runtime, "bound");

    runtime
        .submit_action(BoundAction::ResizeInner)
        .unwrap_or_else(|_| unreachable!("resize action is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::PageForward,
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated {
            outcome: TraceScrollControlBindingOutcome::MetricsUnavailable,
            axis: Some(Axis::Vertical),
            ..
        }
    )));

    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("changed scroll metrics republish: {error:?}"));
    submit_scroll(&mut runtime, bound, ScrollControlRequest::PageForward);
    assert_eq!(
        scroll_offset(&mut runtime, "inner"),
        (0.0, 40.0),
        "the same request uses the new viewport extent only after publication"
    );
}

#[test]
fn per_axis_binding_skips_nearer_nonmatching_scroll_owner() {
    let mut runtime = AppRuntime::<PerAxisApp>::mount(());
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(40.0, 30.0)
                .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("per-axis fixture publishes: {error:?}"));
    let bound = node_id(&mut runtime, "axis.bound");
    runtime
        .submit_command(
            bound,
            SemanticCommand::ScrollControl(ScrollControlRequest::PageForward),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("per-axis scroll request is admitted"));
    settle(&mut runtime);

    assert_eq!(
        scroll_offset(&mut runtime, "axis.inner"),
        (0.0, 0.0),
        "a nearer vertical-only viewport is ineligible for a horizontal binding"
    );
    assert_eq!(
        scroll_offset(&mut runtime, "axis.outer"),
        (40.0, 0.0),
        "the nearest horizontal-eligible ancestor owns the request"
    );
}

#[derive(Clone, Copy, Debug)]
enum FailMode {
    Missing,
    NonScrollable,
}

struct FailApp;

impl UiApp for FailApp {
    type State = FailMode;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(mode: &Self::State) -> Element<Self::Action> {
        let mut target = Element::new(Spacer)
            .id("fail.target")
            .key("fail.target")
            .with_layout(
                LayoutStyle::default()
                    .with_width(dimension(20.0))
                    .with_height(dimension(60.0)),
            );
        if matches!(mode, FailMode::NonScrollable) {
            target = target.scroll_control(
                ScrollControlBinding::new(Axis::Vertical, length(5.0))
                    .unwrap_or_else(|_| unreachable!("fixture binding is valid")),
            );
        }
        scroll_viewport(
            target,
            OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Clip),
        )
        .id("fail.owner")
        .key("fail.owner")
        .with_layout(block_size(
            30.0,
            30.0,
            OverflowStyle::new(OverflowPolicy::Clip, OverflowPolicy::Clip),
        ))
        .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn bound_request_without_published_metrics_fails_closed() {
    let mut runtime = bound_runtime();
    settle(&mut runtime);
    let bound = node_id(&mut runtime, "bound");

    submit_scroll(&mut runtime, bound, ScrollControlRequest::PageForward);

    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert_eq!(scroll_offset(&mut runtime, "outer"), (0.0, 0.0));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated {
            outcome: TraceScrollControlBindingOutcome::MetricsUnavailable,
            axis: Some(Axis::Vertical),
            ..
        }
    )));
}

#[test]
fn missing_and_non_scrollable_bindings_fail_closed() {
    for (mode, expected) in [
        (
            FailMode::Missing,
            TraceScrollControlBindingOutcome::MissingBinding,
        ),
        (
            FailMode::NonScrollable,
            TraceScrollControlBindingOutcome::NonScrollable,
        ),
    ] {
        let mut runtime = AppRuntime::<FailApp>::mount_with_config(
            mode,
            RuntimeConfig::default().with_trace_config(TraceConfig::new(256)),
        );
        settle(&mut runtime);
        let environment = StyleEnvironment::default();
        runtime
            .publish_surface(&SurfaceBuildContext::tight(
                &environment,
                LogicalSize::try_new(30.0, 30.0)
                    .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
            ))
            .unwrap_or_else(|error| panic!("fail-closed fixture publishes: {error:?}"));
        let target = node_id(&mut runtime, "fail.target");
        runtime
            .submit_command(
                target,
                SemanticCommand::ScrollControl(ScrollControlRequest::PageForward),
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!("live fail-closed target is admitted"));
        settle(&mut runtime);
        assert_eq!(scroll_offset(&mut runtime, "fail.owner"), (0.0, 0.0));
        assert!(runtime.trace().records().any(|record| matches!(
            record.kind(),
            TraceRecordKind::ScrollControlBindingEvaluated { outcome, .. }
                if *outcome == expected
        )));
    }
}

#[test]
fn accepted_bound_owner_becoming_non_scrollable_fails_closed_without_outer_fallback() {
    let mut runtime = bound_runtime();
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("binding fixture publishes: {error:?}"));
    let bound = node_id(&mut runtime, "bound");

    runtime
        .submit_action(BoundAction::DisableInnerScroll)
        .unwrap_or_else(|_| unreachable!("owner policy update is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    submit_scroll(&mut runtime, bound, ScrollControlRequest::PageForward);

    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert_eq!(
        scroll_offset(&mut runtime, "outer"),
        (0.0, 0.0),
        "an invalidated exact binding must not retarget to the outer scroll owner"
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated {
            outcome: TraceScrollControlBindingOutcome::Stale,
            axis: Some(Axis::Vertical),
            ..
        }
    )));
}

#[test]
fn newly_nearer_scroll_owner_requires_republication_before_binding_switches() {
    let mut runtime = bound_runtime_with_scrollability(false);
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("outer-bound fixture publishes: {error:?}"));
    let bound = node_id(&mut runtime, "bound");

    runtime
        .submit_action(BoundAction::EnableInnerScroll)
        .unwrap_or_else(|_| unreachable!("inner-scrollability update is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    submit_scroll(
        &mut runtime,
        bound.clone(),
        ScrollControlRequest::PageForward,
    );

    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert_eq!(scroll_offset(&mut runtime, "outer"), (0.0, 0.0));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated {
            outcome: TraceScrollControlBindingOutcome::Stale,
            axis: Some(Axis::Vertical),
            ..
        }
    )));

    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("updated nearest-owner publication commits: {error:?}"));
    submit_scroll(&mut runtime, bound, ScrollControlRequest::PageForward);
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 30.0));
    assert_eq!(scroll_offset(&mut runtime, "outer"), (0.0, 0.0));
}

#[test]
fn replaced_bound_target_is_rejected_without_retargeting() {
    let mut runtime = bound_runtime();
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("stale-target fixture publishes: {error:?}"));
    let stale = node_id(&mut runtime, "bound");

    runtime
        .submit_action(BoundAction::ReplaceBound)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted"));
    runtime
        .submit_command(
            stale,
            SemanticCommand::ScrollControl(ScrollControlRequest::PageForward),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("still-live target is admitted before replacement"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );

    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::CommandProcessingRejected {
            outcome: TraceTargetRejection::Stale
        }
    )));
}

#[test]
fn scroll_control_waiting_queue_pressure_rejects_before_callback_or_offset_mutation() {
    const QUEUE_CAPACITY: usize = 16;
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(QUEUE_CAPACITY)
        .with_transaction_outputs(1);
    let mut runtime = AppRuntime::<BoundApp>::mount_with_config(
        BoundState {
            inner_height: 30.0,
            inner_scrollable: true,
            replaced: false,
            hit: Rc::new(RefCell::new(Vec::new())),
            paint: Rc::new(RefCell::new(Vec::new())),
            semantics: Rc::new(RefCell::new(Vec::new())),
            event: Rc::new(RefCell::new(Vec::new())),
        },
        RuntimeConfig::default()
            .with_limits(limits)
            .with_trace_config(TraceConfig::new(512)),
    );
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("queue-pressure fixture publishes: {error:?}"));
    let bound = node_id(&mut runtime, "bound");
    runtime.state().event.borrow_mut().clear();

    runtime
        .submit_command(
            bound,
            SemanticCommand::ScrollControl(ScrollControlRequest::PageForward),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("scroll-control command ingress is admitted"));
    for _ in 0..(QUEUE_CAPACITY - 1) {
        runtime
            .submit_action(BoundAction::ResizeInner)
            .unwrap_or_else(|_| unreachable!("filler action is admitted"));
    }

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert!(
        runtime.state().event.borrow().is_empty(),
        "routed admission rejects before the bound widget callback"
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::WaitingEnvelopes
        }
    )));
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated { .. }
            | TraceRecordKind::LogicalScrollOwnerApplied { .. }
    )));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn scroll_control_trace_admission_rejects_before_scroll_mutation() {
    let limits = RuntimeLimits::default().with_transaction_outputs(1);
    let mut runtime = AppRuntime::<BoundApp>::mount_with_config(
        BoundState {
            inner_height: 30.0,
            inner_scrollable: true,
            replaced: false,
            hit: Rc::new(RefCell::new(Vec::new())),
            paint: Rc::new(RefCell::new(Vec::new())),
            semantics: Rc::new(RefCell::new(Vec::new())),
            event: Rc::new(RefCell::new(Vec::new())),
        },
        RuntimeConfig::default()
            .with_limits(limits)
            .with_trace_config(TraceConfig::new(256)),
    );
    settle(&mut runtime);
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&bound_build(&environment))
        .unwrap_or_else(|error| panic!("admission fixture publishes: {error:?}"));
    let bound = node_id(&mut runtime, "bound");

    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 2);
    runtime
        .submit_command(
            bound,
            SemanticCommand::ScrollControl(ScrollControlRequest::PageForward),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("command ingress retains enough trace capacity"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(scroll_offset(&mut runtime, "inner"), (0.0, 0.0));
    assert_eq!(
        runtime.status(),
        runenui_runtime::RuntimeStatus::Terminal(
            runenui_runtime::RuntimeTerminalReason::TraceSequenceExhausted
        )
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::TraceSequenceExhausted
        }
    )));
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ScrollControlBindingEvaluated { .. }
    )));
}
