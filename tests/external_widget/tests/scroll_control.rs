#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Axis, ChildBearingWidget, CommandOrigin, Element, EventContext, EventPhase, HitContribution,
    HitContributionContext,
    LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint, LogicalRect,
    NoHostProtocol, OverflowPolicy, OverflowStyle, PaintContribution, PaintContributionContext,
    ScrollControlBinding, ScrollControlRequest, ScrollControlSnapshot, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution, SemanticRole,
    StyleEnvironment, UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetMeasure,
    WidgetMeasureInput, children, container,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext,
};

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
}

impl Widget<()> for ExternalScrollControl {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(length(20.0), length(60.0))
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && event
                .as_semantic_command()
                .is_some_and(|event| event.command() == SemanticCommand::Activate)
        {
            let snapshot = context
                .scroll_control_snapshot()
                .unwrap_or_else(|| unreachable!("bound downstream callback has a live snapshot"));
            self.events.borrow_mut().push(snapshot);
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(SemanticCommand::ScrollControl(
                ScrollControlRequest::PageForward,
            ));
        }
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
        SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Generic))
    }
}

#[derive(Debug)]
struct State {
    observed: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
    events: Rc<RefCell<Vec<ScrollControlSnapshot>>>,
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
        })
        .id("external.control")
        .key("external.control")
        .with_layout(
            LayoutStyle::default()
                .with_width(dimension(20.0))
                .with_height(dimension(60.0)),
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
    assert_eq!(publication.hit_test_scene().target_at(blank), Some(&viewport));

    let initial = *observed
        .borrow()
        .last()
        .unwrap_or_else(|| unreachable!("downstream paint observed scroll snapshot"));
    assert_eq!(initial.axis(), Axis::Vertical);
    assert_eq!(initial.offset().get(), 0.0);
    assert_eq!(initial.maximum_offset().get(), 30.0);
    assert_eq!(initial.viewport_extent().get(), 30.0);
    assert_eq!(initial.content_extent().get(), 60.0);

    runtime
        .submit_command(control, SemanticCommand::Activate, CommandOrigin::programmatic())
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

    let offset = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.id() == &viewport)
        .unwrap_or_else(|| unreachable!("external viewport remains mounted"))
        .interaction()
        .scroll_offset();
    assert_eq!(offset, (0.0, 30.0));
}
