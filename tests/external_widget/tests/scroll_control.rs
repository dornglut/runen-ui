#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Axis, Brush, ChildBearingWidget, Color, CommandOrigin, Element, EventContext, EventPhase,
    Focusability, HitContribution, HitContributionContext, LayoutContainer, LayoutDimension,
    LayoutStyle, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, OverflowPolicy,
    OverflowStyle, PaintContribution, PaintContributionContext, PaintContributionItem,
    PaintPrimitive, SceneShape, ScrollBarLayout, ScrollBarPlacement, ScrollBarVisibility,
    ScrollChrome, ScrollControlBinding, ScrollControlRequest, ScrollControlSnapshot,
    ScrollNormalizedValue, SemanticAction, SemanticActionRequest, SemanticCommand,
    SemanticCommandEvent, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticOrientation, SemanticRelationshipKind, SemanticRole,
    StyleEnvironment, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetEventOutput,
    WidgetMeasure, WidgetMeasureInput, children, container,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!("fixture length is finite"))
}

fn dimension(value: f32) -> LayoutDimension {
    LayoutDimension::Length(length(value))
}

fn node_id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
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

#[derive(Debug)]
struct ExternalViewport;

impl Widget<()> for ExternalViewport {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("viewport hit bounds are finite")),
        )
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Group);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<()> for ExternalViewport {}

#[derive(Debug)]
struct ExternalScrollControl {
    observed: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    events: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    height: LogicalLength,
}

impl Widget<()> for ExternalScrollControl {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(length(20.0), self.height)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }
        let Some(command) = event
            .as_semantic_command()
            .map(SemanticCommandEvent::command)
        else {
            return WidgetEventOutput::none();
        };
        let request = match command {
            SemanticCommand::Activate => ScrollControlRequest::PageForward,
            SemanticCommand::Increment => ScrollControlRequest::SmallStepForward,
            SemanticCommand::Decrement => ScrollControlRequest::SmallStepBackward,
            SemanticCommand::SetValue(value) => {
                let percentage = value.get();
                if !(0.0..=100.0).contains(&percentage) {
                    return WidgetEventOutput::none();
                }
                let Some(normalized) = normalized_scroll_percentage(percentage) else {
                    return WidgetEventOutput::none();
                };
                ScrollControlRequest::SetNormalized(normalized)
            }
            _ => return WidgetEventOutput::none(),
        };
        let snapshot = context
            .scroll_control_snapshot()
            .unwrap_or_else(|| unreachable!("bound downstream callback has a live snapshot"));
        self.events.borrow_mut().push(snapshot);
        context.prevent_default();
        context.stop_propagation();
        context.emit_command(SemanticCommand::ScrollControl(request));
        WidgetEventOutput::none()
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        assert!(
            context.scroll_control_snapshot().is_some(),
            "downstream hit contribution observes the public binding projection"
        );
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("control hit bounds are finite")),
        )
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        if let Some(snapshot) = context.scroll_control_snapshot() {
            self.observed.borrow_mut().push(snapshot);
        }
        PaintContribution::empty()
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        assert!(
            context.scroll_control_snapshot().is_some(),
            "downstream semantic contribution observes the same public binding projection"
        );
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ScrollBar)
                .with_action(SemanticAction::RequestFocus),
        )
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the semantic percentage is range-checked to finite [0, 100] before normalization into the accepted f32 scroll protocol"
)]
fn normalized_scroll_percentage(percentage: f64) -> Option<ScrollNormalizedValue> {
    if !(0.0..=100.0).contains(&percentage) {
        return None;
    }
    ScrollNormalizedValue::new((percentage / 100.0) as f32).ok()
}

#[derive(Debug)]
struct State {
    observed: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    events: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    content_height: f32,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let binding = ScrollControlBinding::new(Axis::Vertical, length(4.0))
            .unwrap_or_else(|_| unreachable!("fixture binding is valid"));
        let control = Element::new(ExternalScrollControl {
            observed: Rc::clone(&state.observed),
            events: Rc::clone(&state.events),
            height: length(state.content_height),
        })
        .id("external.control")
        .key("external.control")
        .with_layout(
            LayoutStyle::default()
                .with_width(dimension(20.0))
                .with_height(dimension(state.content_height)),
        )
        .scroll_control(binding);
        container(ExternalViewport, children![control])
            .id("external.viewport")
            .key("external.viewport")
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(dimension(30.0))
                    .with_height(dimension(30.0))
                    .with_overflow(OverflowStyle::new(
                        OverflowPolicy::Clip,
                        OverflowPolicy::Scroll,
                    )),
            )
            .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn assert_downstream_scrollbar_semantics(
    runtime: &mut AppRuntime<App>,
    publication: &runenui_runtime::SurfacePublication,
    control: &MountedNodeId,
) -> (runenui_core::SurfaceId, runenui_core::SemanticNodeId) {
    let semantic_snapshot = publication.semantic_publication().snapshot();
    let scrollbar = semantic_snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ScrollBar)
        .unwrap_or_else(|| unreachable!("bound downstream scrollbar semantics are published"));
    assert_eq!(scrollbar.orientation(), Some(SemanticOrientation::Vertical));
    let range = scrollbar
        .range()
        .unwrap_or_else(|| unreachable!("scrollbar range is runtime-derived"));
    assert_eq!(
        range.minimum().map(runenui_core::SemanticNumber::get),
        Some(0.0)
    );
    assert_eq!(
        range.maximum().map(runenui_core::SemanticNumber::get),
        Some(100.0)
    );
    assert_eq!(
        range.current().map(runenui_core::SemanticNumber::get),
        Some(0.0)
    );
    for action in [
        SemanticAction::RequestFocus,
        SemanticAction::Increment,
        SemanticAction::Decrement,
        SemanticAction::SetValue,
    ] {
        assert!(scrollbar.supported_actions().contains(&action));
    }
    let controls = scrollbar
        .relationships()
        .iter()
        .filter(|relationship| relationship.kind() == SemanticRelationshipKind::Controls)
        .collect::<Vec<_>>();
    assert_eq!(controls.len(), 1);
    assert_eq!(
        semantic_snapshot
            .node(controls[0].target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::Group),
        "Controls resolves to the exact published viewport owner"
    );
    assert!(
        runtime
            .index()
            .node(control)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable),
        "positive accepted maximum offset makes Automatic bound control focusable"
    );
    (
        semantic_snapshot.surface_id().clone(),
        scrollbar.id().clone(),
    )
}

fn assert_initial_scroll_snapshot(initial: ScrollControlSnapshot) {
    assert_eq!(initial.axis(), Axis::Vertical);
    assert_eq!(initial.offset().get(), 0.0);
    assert_eq!(initial.maximum_offset().get(), 30.0);
    assert_eq!(initial.viewport_extent().get(), 30.0);
    assert_eq!(initial.content_extent().get(), 60.0);
}

#[test]
fn downstream_viewport_and_control_use_public_scroll_binding_snapshot_and_request_contracts() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observed: Rc::clone(&observed),
        events: Rc::clone(&events),
        content_height: 60.0,
    });
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(30.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("external scroll fixture publishes"));

    let viewport = node_id(&mut runtime, "external.viewport");
    let control = node_id(&mut runtime, "external.control");
    let blank =
        LogicalPoint::new(25.0, 5.0).unwrap_or_else(|_| unreachable!("fixture point is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(blank),
        Some(&viewport)
    );

    let initial = *observed
        .borrow()
        .last()
        .unwrap_or_else(|| unreachable!("downstream paint observed scroll snapshot"));
    assert_initial_scroll_snapshot(initial);

    let (semantic_surface, scrollbar_semantic) =
        assert_downstream_scrollbar_semantics(&mut runtime, &publication, &control);

    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            semantic_surface,
            scrollbar_semantic,
            SemanticAction::Increment,
        ))
        .unwrap_or_else(|_| unreachable!("published scrollbar Increment is admitted"));
    for _ in 0..2 {
        assert_eq!(
            runtime
                .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX)).expect("pump observation").report().to_owned()
                .processed_envelopes(),
            1
        );
    }
    assert_eq!(scroll_offset_for(&mut runtime, &viewport), (0.0, 4.0));
    assert_eq!(events.borrow().as_slice(), &[initial]);
    events.borrow_mut().clear();

    let _ = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("post-increment scroll fixture republishes"));
    let republished = *observed
        .borrow()
        .last()
        .unwrap_or_else(|| unreachable!("republished paint observes current scroll snapshot"));
    assert_eq!(republished.offset().get(), 4.0);

    runtime
        .submit_command(
            control,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("downstream control trigger is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX)).expect("pump observation").report().to_owned()
            .processed_envelopes(),
        1
    );
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX)).expect("pump observation").report().to_owned()
            .processed_envelopes(),
        1,
        "callback-emitted scroll request remains ordinary queued routed work"
    );
    let event_snapshot = *events
        .borrow()
        .last()
        .unwrap_or_else(|| unreachable!("downstream callback observed scroll snapshot"));
    assert_eq!(event_snapshot, republished);

    assert_eq!(scroll_offset_for(&mut runtime, &viewport), (0.0, 30.0));
}

fn scroll_offset_for(runtime: &mut AppRuntime<App>, viewport: &MountedNodeId) -> (f32, f32) {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.id() == viewport)
        .unwrap_or_else(|| unreachable!("external viewport remains mounted"))
        .interaction()
        .scroll_offset()
}

#[test]
fn non_scrollable_bound_scrollbar_remains_semantic_but_is_not_a_dead_focus_stop() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        observed,
        events,
        content_height: 30.0,
    });
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let environment = StyleEnvironment::default();
    let build = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(30.0, 30.0)
            .unwrap_or_else(|_| unreachable!("fixture surface is finite")),
    );
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("non-scrollable bound fixture publishes"));
    let control = node_id(&mut runtime, "external.control");

    assert!(
        runtime
            .index()
            .node(&control)
            .is_some_and(|node| !node.is_focusable()),
        "zero accepted maximum offset with Automatic focusability is excluded"
    );

    let scrollbar = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ScrollBar)
        .unwrap_or_else(|| unreachable!("Always-style zero-range scrollbar remains semantic"));
    let range = scrollbar
        .range()
        .unwrap_or_else(|| unreachable!("zero-range scrollbar still publishes its range"));
    assert_eq!(
        range.current().map(runenui_core::SemanticNumber::get),
        Some(0.0)
    );
    assert!(
        !scrollbar
            .supported_actions()
            .contains(&SemanticAction::RequestFocus)
    );
    assert!(
        !scrollbar
            .supported_actions()
            .contains(&SemanticAction::Increment)
    );

    runtime
        .submit_command(
            control.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live target accepts routed focus command"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    assert_ne!(runtime.focus().focused_node(), Some(&control));
}

const TRACK_COLOR: Color = Color::rgba(220, 40, 40, 255);
const THUMB_COLOR: Color = Color::rgba(40, 80, 220, 255);

#[derive(Debug)]
struct ChromeContent {
    height: LogicalLength,
}

impl Widget<ChromeAction> for ChromeContent {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(length(80.0), self.height)
    }
}

#[derive(Debug)]
struct ChromeViewport;

impl Widget<ChromeAction> for ChromeViewport {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("chrome viewport hit bounds are finite")),
        )
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Group);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<ChromeAction> for ChromeViewport {}

#[derive(Debug)]
struct ChromeTrack {
    semantic_callbacks: Rc<RefCell<usize>>,
}

impl Widget<ChromeAction> for ChromeTrack {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("track hit bounds are finite")),
        )
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(vec![PaintContributionItem::fill(
            SceneShape::rect(
                LogicalRect::try_new(
                    0.0,
                    0.0,
                    context.local_size().width(),
                    context.local_size().height(),
                )
                .unwrap_or_else(|_| unreachable!("track paint bounds are finite")),
            ),
            Brush::solid(TRACK_COLOR),
        )])
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        *self.semantic_callbacks.borrow_mut() += 1;
        let mut node = SemanticNodeContribution::primary(SemanticRole::ScrollBar)
            .with_action(SemanticAction::RequestFocus);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<ChromeAction> for ChromeTrack {}

#[derive(Debug)]
struct ChromeThumb {
    semantic_callbacks: Rc<RefCell<usize>>,
}

impl Widget<ChromeAction> for ChromeThumb {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        HitContribution::single_rect(
            LogicalRect::try_new(
                0.0,
                0.0,
                context.local_size().width(),
                context.local_size().height(),
            )
            .unwrap_or_else(|_| unreachable!("thumb hit bounds are finite")),
        )
    }

    fn paint(&self, (): &Self::State, context: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(vec![PaintContributionItem::fill(
            SceneShape::rect(
                LogicalRect::try_new(
                    0.0,
                    0.0,
                    context.local_size().width(),
                    context.local_size().height(),
                )
                .unwrap_or_else(|_| unreachable!("thumb paint bounds are finite")),
            ),
            Brush::solid(THUMB_COLOR),
        )])
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        *self.semantic_callbacks.borrow_mut() += 1;
        SemanticContribution::empty()
    }
}

#[derive(Debug)]
struct ChromeState {
    content_height: f32,
    visibility: ScrollBarVisibility,
    placement: ScrollBarPlacement,
    explicit_focus: bool,
    track_semantics: Rc<RefCell<usize>>,
    thumb_semantics: Rc<RefCell<usize>>,
}

#[derive(Clone, Copy, Debug)]
enum ChromeAction {
    SetVisibility(ScrollBarVisibility),
}

struct ChromeApp;

impl UiApp for ChromeApp {
    type State = ChromeState;
    type Action = ChromeAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let binding = ScrollControlBinding::new(Axis::Vertical, length(10.0))
            .unwrap_or_else(|_| unreachable!("chrome binding is valid"));
        let thumb = Element::new(ChromeThumb {
            semantic_callbacks: Rc::clone(&state.thumb_semantics),
        })
        .id("chrome.thumb")
        .key("chrome.thumb")
        .with_focusability(Focusability::NotFocusable)
        .scroll_control(binding)
        .scroll_chrome(ScrollChrome::Thumb(Axis::Vertical));
        let track_layout = ScrollBarLayout::new(Axis::Vertical, length(10.0), length(20.0))
            .with_visibility(state.visibility)
            .with_placement(state.placement);
        let track = container(
            ChromeTrack {
                semantic_callbacks: Rc::clone(&state.track_semantics),
            },
            children![thumb],
        )
        .id("chrome.track")
        .key("chrome.track")
        .into_element()
        .with_focusability(if state.explicit_focus {
            Focusability::Focusable
        } else {
            Focusability::Automatic
        })
        .scroll_control(binding)
        .scroll_chrome(ScrollChrome::Bar(track_layout));
        let content = Element::new(ChromeContent {
            height: length(state.content_height),
        })
        .id("chrome.content")
        .key("chrome.content")
        .with_layout(
            LayoutStyle::default()
                .with_width(dimension(80.0))
                .with_height(dimension(state.content_height)),
        );

        container(ChromeViewport, children![content, track])
            .id("chrome.viewport")
            .key("chrome.viewport")
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(dimension(100.0))
                    .with_height(dimension(100.0))
                    .with_overflow(OverflowStyle::new(
                        OverflowPolicy::Clip,
                        OverflowPolicy::Scroll,
                    )),
            )
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            ChromeAction::SetVisibility(visibility) => state.visibility = visibility,
        }
    }
}

fn chrome_node_id(runtime: &mut AppRuntime<ChromeApp>, authored: &str) -> MountedNodeId {
    let authored =
        runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!("valid chrome id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("chrome fixture node is mounted"))
        .id()
        .clone()
}

fn chrome_publish(runtime: &mut AppRuntime<ChromeApp>) -> runenui_runtime::SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(100.0, 100.0)
                .unwrap_or_else(|_| unreachable!("chrome fixture surface is finite")),
        ))
        .unwrap_or_else(|_| unreachable!("chrome fixture publication is admitted"))
}

fn chrome_state(
    content_height: f32,
    visibility: ScrollBarVisibility,
    placement: ScrollBarPlacement,
    explicit_focus: bool,
) -> ChromeState {
    ChromeState {
        content_height,
        visibility,
        placement,
        explicit_focus,
        track_semantics: Rc::new(RefCell::new(0)),
        thumb_semantics: Rc::new(RefCell::new(0)),
    }
}

struct DuplicateChromeApp;

impl UiApp for DuplicateChromeApp {
    type State = ();
    type Action = ChromeAction;
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        let binding = ScrollControlBinding::new(Axis::Vertical, length(10.0))
            .unwrap_or_else(|_| unreachable!("duplicate chrome binding is valid"));
        let layout = ScrollBarLayout::new(Axis::Vertical, length(10.0), length(20.0))
            .with_visibility(ScrollBarVisibility::Always)
            .with_placement(ScrollBarPlacement::Reserved);
        let track = |id: &'static str| {
            Element::new(ChromeTrack {
                semantic_callbacks: Rc::new(RefCell::new(0)),
            })
            .id(id)
            .key(id)
            .with_focusability(Focusability::Focusable)
            .scroll_control(binding)
            .scroll_chrome(ScrollChrome::Bar(layout))
        };
        let content = Element::new(ChromeContent {
            height: length(200.0),
        })
        .with_layout(
            LayoutStyle::default()
                .with_width(dimension(80.0))
                .with_height(dimension(200.0)),
        );

        container(
            ChromeViewport,
            children![
                content,
                track("chrome.duplicate-a"),
                track("chrome.duplicate-b")
            ],
        )
        .with_layout(
            LayoutStyle::default()
                .with_container(LayoutContainer::Block)
                .with_width(dimension(100.0))
                .with_height(dimension(100.0))
                .with_overflow(OverflowStyle::new(
                    OverflowPolicy::Clip,
                    OverflowPolicy::Scroll,
                )),
        )
        .into_element()
    }

    fn update((): &mut Self::State, _: Self::Action) {}
}

#[test]
fn duplicate_scroll_chrome_is_rejected_and_diagnosed_without_losing_mounted_identity() {
    let mut runtime = AppRuntime::<DuplicateChromeApp>::mount(());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(100.0, 100.0)
                .unwrap_or_else(|_| unreachable!("duplicate chrome surface is finite")),
        ))
        .unwrap_or_else(|_| unreachable!("duplicate chrome surface publishes"));
    let expected = runenui_core::WidgetDiagnostic::new(
        "runenui.scroll-chrome.duplicate-bar",
        "multiple scrollbar bars target the same scroll owner axis; all are withheld",
    );

    for authored in ["chrome.duplicate-a", "chrome.duplicate-b"] {
        let authored_id = runenui_core::ElementId::new(authored)
            .unwrap_or_else(|_| unreachable!("duplicate chrome authored ID is valid"));
        let mounted = runtime
            .index()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored_id))
            .unwrap_or_else(|| unreachable!("duplicate chrome remains mounted"))
            .id()
            .clone();
        let layout = publication
            .layout_report()
            .nodes()
            .iter()
            .find(|node| node.id() == &mounted)
            .unwrap_or_else(|| unreachable!("duplicate chrome layout node is retained"));

        assert!(layout.diagnostics().contains(&expected));
        assert!(
            publication
                .hit_test_scene()
                .contains_mounted_target(&mounted)
        );
        assert!(
            runtime
                .index()
                .node(&mounted)
                .is_some_and(|node| !node.is_focusable())
        );
    }
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|node| node.role() != SemanticRole::ScrollBar)
    );
    assert!(
        publication
            .paint_scene()
            .items()
            .iter()
            .all(|item| { !matches!(item_color(item), Some(color) if color == TRACK_COLOR) })
    );
}

const fn item_color(item: &runenui_runtime::PaintSceneItem) -> Option<Color> {
    match item.primitive() {
        PaintPrimitive::Fill {
            brush: Brush::Solid(color),
            ..
        } => Some(*color),
        _ => None,
    }
}

fn colored_item(
    publication: &runenui_runtime::SurfacePublication,
    color: Color,
) -> &runenui_runtime::PaintSceneItem {
    publication
        .paint_scene()
        .items()
        .iter()
        .find(|item| item_color(item) == Some(color))
        .unwrap_or_else(|| unreachable!("expected chrome paint item is published"))
}

fn translated_origin(item: &runenui_runtime::PaintSceneItem) -> LogicalPoint {
    item.local_to_surface()
        .transform_point(
            LogicalPoint::new(0.0, 0.0)
                .unwrap_or_else(|_| unreachable!("fixture origin is finite")),
        )
        .unwrap_or_else(|| unreachable!("published chrome transform is finite"))
}

fn assert_thumb_moves_in_presentation_only(
    runtime: &mut AppRuntime<ChromeApp>,
    track: MountedNodeId,
    thumb: &MountedNodeId,
    thumb_bounds: LogicalRect,
) {
    runtime
        .submit_command(
            track,
            SemanticCommand::ScrollControl(ScrollControlRequest::ToEnd),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("direct bound scroll request is admitted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let scrolled = chrome_publish(runtime);
    let scrolled_thumb_bounds = scrolled
        .frame()
        .node(thumb)
        .unwrap_or_else(|| unreachable!("thumb remains laid out"))
        .bounds();
    assert_eq!(scrolled_thumb_bounds, thumb_bounds);
    assert_eq!(
        translated_origin(colored_item(&scrolled, THUMB_COLOR)).y(),
        50.0
    );
    assert_eq!(
        scrolled.hit_test_scene().target_at(
            LogicalPoint::new(95.0, 75.0)
                .unwrap_or_else(|_| unreachable!("translated thumb sample is finite"))
        ),
        Some(thumb)
    );
}

#[test]
fn downstream_reserved_scroll_chrome_is_viewport_attached_and_thumb_moves_in_presentation_only() {
    let state = chrome_state(
        200.0,
        ScrollBarVisibility::Automatic,
        ScrollBarPlacement::Reserved,
        false,
    );
    let track_callbacks = Rc::clone(&state.track_semantics);
    let thumb_callbacks = Rc::clone(&state.thumb_semantics);
    let mut runtime = AppRuntime::<ChromeApp>::mount(state);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let initial = chrome_publish(&mut runtime);
    let viewport = chrome_node_id(&mut runtime, "chrome.viewport");
    let track = chrome_node_id(&mut runtime, "chrome.track");
    let thumb = chrome_node_id(&mut runtime, "chrome.thumb");

    let viewport_layout = initial
        .layout_report()
        .nodes()
        .iter()
        .find(|node| node.id() == &viewport)
        .unwrap_or_else(|| unreachable!("viewport layout is published"));
    assert_eq!(viewport_layout.scroll_viewport_extent().width(), 90.0);
    assert_eq!(viewport_layout.scroll_viewport_extent().height(), 100.0);

    let track_bounds = initial
        .frame()
        .node(&track)
        .unwrap_or_else(|| unreachable!("track layout is published"))
        .bounds();
    let thumb_bounds = initial
        .frame()
        .node(&thumb)
        .unwrap_or_else(|| unreachable!("thumb layout is published"))
        .bounds();
    assert_eq!(
        (
            track_bounds.x(),
            track_bounds.y(),
            track_bounds.width(),
            track_bounds.height()
        ),
        (90.0, 0.0, 10.0, 100.0)
    );
    assert_eq!(
        (
            thumb_bounds.x(),
            thumb_bounds.y(),
            thumb_bounds.width(),
            thumb_bounds.height()
        ),
        (90.0, 0.0, 10.0, 50.0)
    );

    assert!(initial.hit_test_scene().contains_mounted_target(&track));
    assert!(initial.hit_test_scene().contains_mounted_target(&thumb));
    assert_eq!(
        initial.hit_test_scene().target_at(
            LogicalPoint::new(95.0, 25.0)
                .unwrap_or_else(|_| unreachable!("thumb sample is finite"))
        ),
        Some(&thumb)
    );
    assert_eq!(
        initial.hit_test_scene().target_at(
            LogicalPoint::new(95.0, 75.0)
                .unwrap_or_else(|_| unreachable!("track sample is finite"))
        ),
        Some(&track)
    );
    assert_eq!(
        translated_origin(colored_item(&initial, THUMB_COLOR)).y(),
        0.0
    );
    assert!(*track_callbacks.borrow() > 0);
    assert!(*thumb_callbacks.borrow() > 0);
    assert_eq!(
        initial
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::ScrollBar)
            .count(),
        1
    );

    assert_thumb_moves_in_presentation_only(&mut runtime, track, &thumb, thumb_bounds);
}

#[test]
fn downstream_scroll_chrome_clamps_thumb_to_authored_minimum_extent() {
    let mut runtime = AppRuntime::<ChromeApp>::mount(chrome_state(
        1_000.0,
        ScrollBarVisibility::Automatic,
        ScrollBarPlacement::Reserved,
        false,
    ));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let publication = chrome_publish(&mut runtime);
    let thumb = chrome_node_id(&mut runtime, "chrome.thumb");
    let thumb_bounds = publication
        .frame()
        .node(&thumb)
        .unwrap_or_else(|| unreachable!("minimum-clamped thumb is laid out"))
        .bounds();

    assert_eq!(thumb_bounds.height(), 20.0);
    assert_eq!(thumb_bounds.width(), 10.0);
}

fn assert_nonparticipating_reserved_chrome(visibility: ScrollBarVisibility, content_height: f32) {
    let state = chrome_state(
        content_height,
        visibility,
        ScrollBarPlacement::Reserved,
        true,
    );
    let track_callbacks = Rc::clone(&state.track_semantics);
    let thumb_callbacks = Rc::clone(&state.thumb_semantics);
    let mut runtime = AppRuntime::<ChromeApp>::mount(state);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let publication = chrome_publish(&mut runtime);
    let track = chrome_node_id(&mut runtime, "chrome.track");
    let thumb = chrome_node_id(&mut runtime, "chrome.thumb");

    assert!(publication.hit_test_scene().contains_mounted_target(&track));
    assert!(publication.hit_test_scene().contains_mounted_target(&thumb));
    for point in [
        LogicalPoint::new(95.0, 25.0)
            .unwrap_or_else(|_| unreachable!("chrome hit sample is finite")),
        LogicalPoint::new(95.0, 75.0)
            .unwrap_or_else(|_| unreachable!("chrome hit sample is finite")),
    ] {
        let target = publication.hit_test_scene().target_at(point);
        assert_ne!(target, Some(&track));
        assert_ne!(target, Some(&thumb));
    }
    assert!(publication.paint_scene().items().iter().all(|item| {
        !matches!(item_color(item), Some(color) if color == TRACK_COLOR || color == THUMB_COLOR)
    }));
    assert_eq!(*track_callbacks.borrow(), 0);
    assert_eq!(*thumb_callbacks.borrow(), 0);
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|node| node.role() != SemanticRole::ScrollBar)
    );
    assert!(
        runtime
            .index()
            .node(&track)
            .is_some_and(|node| !node.is_focusable()),
        "authored hidden/absent chrome cannot be reintroduced by explicit focusability"
    );
}

fn assert_overlay_scrollbar_keeps_full_viewport() {
    let mut runtime = AppRuntime::<ChromeApp>::mount(chrome_state(
        200.0,
        ScrollBarVisibility::Always,
        ScrollBarPlacement::Overlay,
        false,
    ));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let publication = chrome_publish(&mut runtime);
    let viewport = chrome_node_id(&mut runtime, "chrome.viewport");
    let layout = publication
        .layout_report()
        .nodes()
        .iter()
        .find(|node| node.id() == &viewport)
        .unwrap_or_else(|| unreachable!("overlay viewport layout is published"));
    assert_eq!(layout.scroll_viewport_extent().width(), 100.0);
    assert_eq!(layout.scroll_viewport_extent().height(), 100.0);
}

fn assert_present_zero_range_chrome_can_be_explicitly_focusable() {
    let mut runtime = AppRuntime::<ChromeApp>::mount(chrome_state(
        100.0,
        ScrollBarVisibility::Always,
        ScrollBarPlacement::Reserved,
        true,
    ));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let publication = chrome_publish(&mut runtime);
    let track = chrome_node_id(&mut runtime, "chrome.track");
    assert!(publication.hit_test_scene().contains_mounted_target(&track));
    assert!(
        runtime
            .index()
            .node(&track)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable),
        "present explicit Focusable chrome may override zero-range Automatic focus exclusion"
    );
    let scrollbar = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ScrollBar)
        .unwrap_or_else(|| unreachable!("Always zero-range scrollbar remains semantic"));
    assert_eq!(
        scrollbar
            .range()
            .and_then(runenui_core::SemanticRange::current)
            .map(runenui_core::SemanticNumber::get),
        Some(0.0)
    );
}

#[test]
fn downstream_scroll_chrome_visibility_and_overlay_share_one_participation_authority() {
    assert_nonparticipating_reserved_chrome(ScrollBarVisibility::Hidden, 200.0);
    assert_nonparticipating_reserved_chrome(ScrollBarVisibility::Automatic, 100.0);
    assert_overlay_scrollbar_keeps_full_viewport();
    assert_present_zero_range_chrome_can_be_explicitly_focusable();
}

#[test]
fn live_chrome_change_clears_focus_before_surface_republication() {
    let mut runtime = AppRuntime::<ChromeApp>::mount(chrome_state(
        200.0,
        ScrollBarVisibility::Always,
        ScrollBarPlacement::Reserved,
        true,
    ));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    let _publication = chrome_publish(&mut runtime);
    let track = chrome_node_id(&mut runtime, "chrome.track");

    runtime
        .submit_command(
            track.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("visible explicit chrome accepts focus request"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();
    assert_eq!(runtime.focus().focused_node(), Some(&track));

    runtime
        .submit_action(ChromeAction::SetVisibility(ScrollBarVisibility::Hidden))
        .unwrap_or_else(|_| unreachable!("chrome visibility change enters the application FIFO"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    )).expect("pump observation").report().to_owned();

    assert_eq!(
        runtime.focus().focused_node(),
        None,
        "live authored chrome that no longer matches retained participation fails closed before republish"
    );
    assert!(
        runtime
            .index()
            .node(&track)
            .is_some_and(|node| !node.is_focusable()),
        "stale retained positive participation cannot keep explicitly focusable chrome eligible"
    );
}
