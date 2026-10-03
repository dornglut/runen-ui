#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Axis, ChildBearingWidget, CommandOrigin, Element, EventContext, EventPhase, HitContribution,
    HitContributionContext, LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, OverflowPolicy, OverflowStyle, PaintContribution,
    PaintContributionContext, ScrollControlBinding, ScrollControlRequest, ScrollControlSnapshot,
    ScrollNormalizedValue, SemanticAction, SemanticActionRequest, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticOrientation, SemanticRelationshipKind, SemanticRole, StyleEnvironment, UiApp, UiEvent,
    View, Widget, WidgetActivation, WidgetEventOutput, WidgetMeasure, WidgetMeasureInput, children,
    container,
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
        let Some(command) = event.as_semantic_command().map(|event| event.command()) else {
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
                let normalized = ScrollNormalizedValue::new((percentage / 100.0) as f32)
                    .unwrap_or_else(|_| unreachable!("checked percentage normalizes into [0, 1]"));
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
    ));
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
    assert_eq!(initial.axis(), Axis::Vertical);
    assert_eq!(initial.offset().get(), 0.0);
    assert_eq!(initial.maximum_offset().get(), 30.0);
    assert_eq!(initial.viewport_extent().get(), 30.0);
    assert_eq!(initial.content_extent().get(), 60.0);

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
    assert_eq!(range.minimum().map(|value| value.get()), Some(0.0));
    assert_eq!(range.maximum().map(|value| value.get()), Some(100.0));
    assert_eq!(range.current().map(|value| value.get()), Some(0.0));
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
            .map(|node| node.role()),
        Some(SemanticRole::Group),
        "Controls resolves to the exact published viewport owner"
    );
    let semantic_surface = semantic_snapshot.surface_id().clone();
    let scrollbar_semantic = scrollbar.id().clone();
    assert!(
        runtime
            .index()
            .node(&control)
            .is_some_and(|node| node.is_focusable()),
        "positive accepted maximum offset makes Automatic bound control focusable"
    );

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
                .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
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

    runtime
        .submit_command(
            control,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("downstream control trigger is admitted"));
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
        1,
        "callback-emitted scroll request remains ordinary queued routed work"
    );
    let event_snapshot = *events
        .borrow()
        .last()
        .unwrap_or_else(|| unreachable!("downstream callback observed scroll snapshot"));
    assert_eq!(event_snapshot, initial);

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
    ));
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
    assert_eq!(range.current().map(|value| value.get()), Some(0.0));
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
    ));
    assert_ne!(runtime.focus().focused_node(), Some(&control));
}
