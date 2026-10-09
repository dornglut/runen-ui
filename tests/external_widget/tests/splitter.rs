#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, HitContribution, HitContributionContext,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LogicalDelta, LogicalKey, LogicalPoint, LogicalRect, NoHostProtocol, PhysicalKey,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    SemanticAction, SemanticActionRequest, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticNumber, SemanticOrientation,
    SemanticRange, SemanticRole, SemanticState, SplitterRequest, StyleEnvironment, UiApp,
    UiEvent, View, Widget, WidgetActivation, WidgetEventOutput, WidgetMeasure, WidgetMeasureInput,
    splitter,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext};

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("finite numeric value"))
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Resize(SplitterRequest),
    Enabled(bool),
    External(bool),
    Horizontal(bool),
}

struct State {
    size: f64,
    enabled: bool,
    external: bool,
    horizontal: bool,
    proposals: usize,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        if state.external {
            Element::new(DownstreamSplitter {
                size: state.size,
                enabled: state.enabled,
                horizontal: state.horizontal,
            })
            .id("controlled.splitter")
            .into_element()
        } else {
            splitter("Sidebar", 0.0, 100.0, state.size, 5.0)
                .unwrap_or_else(|_| unreachable!("bounded application layout"))
                .orientation(if state.horizontal {
                    SemanticOrientation::Horizontal
                } else {
                    SemanticOrientation::Vertical
                })
                .value_text("Sidebar extent")
                .page_step(20.0)
                .unwrap_or_else(|_| unreachable!("checked page step"))
                .enabled(state.enabled)
                .on_resize(Action::Resize)
                .id("controlled.splitter")
                .into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Resize(request) => {
                state.proposals += 1;
                let next = match request {
                    SplitterRequest::MoveBy(pixels) => state.size + f64::from(pixels),
                    SplitterRequest::AdjustBy(amount) => state.size + amount.get(),
                    SplitterRequest::SetValue(value) => value.get(),
                };
                state.size = next.clamp(0.0, 100.0);
            }
            Action::Enabled(value) => state.enabled = value,
            Action::External(value) => state.external = value,
            Action::Horizontal(value) => state.horizontal = value,
        }
    }
}

#[derive(Debug)]
struct DownstreamSplitter {
    size: f64,
    enabled: bool,
    horizontal: bool,
}

impl Widget<Action> for DownstreamSplitter {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target || !self.enabled {
            return WidgetEventOutput::none();
        }
        let request = match event.as_semantic_command().map(|event| event.command()) {
            Some(SemanticCommand::Increment) => SplitterRequest::AdjustBy(number(5.0)),
            Some(SemanticCommand::Decrement) => SplitterRequest::AdjustBy(number(-5.0)),
            Some(SemanticCommand::SetValue(value)) => SplitterRequest::SetValue(value),
            _ => return WidgetEventOutput::none(),
        };
        context.emit(Action::Resize(request));
        context.prevent_default();
        context.stop_propagation();
        WidgetEventOutput::none()
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            runenui_core::LogicalLength::from(12_u16),
            runenui_core::LogicalLength::from(160_u16),
        )
    }

    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("checked local geometry")),
        )
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let range = SemanticRange::new(
            Some(number(0.0)),
            Some(number(100.0)),
            Some(number(self.size)),
        )
        .unwrap_or_else(|_| unreachable!("checked range"))
        .with_small_step(number(5.0))
        .unwrap_or_else(|_| unreachable!("checked step"))
        .with_large_step(number(20.0))
        .unwrap_or_else(|_| unreachable!("checked step"))
        .with_value_text("Sidebar extent")
        .unwrap_or_else(|_| unreachable!("range has value"));
        let mut node = SemanticNodeContribution::primary(SemanticRole::Splitter)
            .with_name("Sidebar")
            .with_range(range)
            .with_orientation(if self.horizontal {
                SemanticOrientation::Horizontal
            } else {
                SemanticOrientation::Vertical
            })
            .with_state(SemanticState::ENABLED.with_disabled(!self.enabled));
        if self.enabled {
            node = node
                .with_action(SemanticAction::Increment)
                .with_action(SemanticAction::Decrement)
                .with_action(SemanticAction::SetValue);
        }
        SemanticContribution::single(node)
    }
}

fn fresh() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        size: 50.0,
        enabled: true,
        external: false,
        horizontal: false,
        proposals: 0,
    });
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(256, 256, 256, 256));
}

fn publish(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("splitter surface publishes"))
}

fn inspect(publication: &runenui_runtime::SurfacePublication, current: f64, enabled: bool, horizontal: bool) {
    let snapshot = publication.semantic_publication().snapshot();
    let node = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Splitter)
        .unwrap_or_else(|| unreachable!("real Splitter role"));
    assert_eq!(node.name(), Some("Sidebar"));
    assert_eq!(
        node.orientation(),
        Some(if horizontal { SemanticOrientation::Horizontal } else { SemanticOrientation::Vertical }),
    );
    let range = node.range().unwrap_or_else(|| unreachable!("bounded range"));
    assert_eq!(range.minimum(), Some(number(0.0)));
    assert_eq!(range.maximum(), Some(number(100.0)));
    assert_eq!(range.current(), Some(number(current)));
    assert_eq!(range.small_step(), Some(number(5.0)));
    assert_eq!(range.large_step(), Some(number(20.0)));
    assert_eq!(range.value_text(), Some("Sidebar extent"));
    assert_eq!(node.state().disabled(), !enabled);
    for action in [SemanticAction::Increment, SemanticAction::Decrement, SemanticAction::SetValue] {
        assert_eq!(node.supported_actions().contains(&action), enabled);
    }
}

fn semantic_action(
    runtime: &mut AppRuntime<App>,
    publication: &runenui_runtime::SurfacePublication,
    action: SemanticAction,
) -> Result<runenui_runtime::CommandSubmission, runenui_runtime::SubmitSemanticActionError> {
    let snapshot = publication.semantic_publication().snapshot();
    let node = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::Splitter)
        .unwrap_or_else(|| unreachable!("published Splitter role"));
    runtime.submit_semantic_action(SemanticActionRequest::new(
        snapshot.surface_id().clone(),
        node.id().clone(),
        action,
    ))
}

#[test]
fn routed_semantic_actions_rebuild_application_owned_range_and_downstream_parity() {
    let mut runtime = fresh();
    let before = publish(&mut runtime);
    inspect(&before, 50.0, true, false);
    semantic_action(&mut runtime, &before, SemanticAction::Increment)
        .unwrap_or_else(|_| unreachable!("admitted increment"));
    assert_eq!(runtime.state().size, 50.0, "admission is not application update");
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 55.0);
    let next = publish(&mut runtime);
    let snapshot = next.semantic_publication().snapshot();
    let node = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::Splitter)
        .unwrap_or_else(|| unreachable!());
    runtime.submit_semantic_action(SemanticActionRequest::set_value(
        snapshot.surface_id().clone(), node.id().clone(), number(90.0)
    )).unwrap_or_else(|_| unreachable!("SetValue admitted"));
    settle(&mut runtime);
    inspect(&publish(&mut runtime), 90.0, true, false);
    runtime.submit_action(Action::External(true)).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    let external = publish(&mut runtime);
    inspect(&external, 90.0, true, false);
    semantic_action(&mut runtime, &external, SemanticAction::Decrement)
        .unwrap_or_else(|_| unreachable!("downstream decrement admitted"));
    settle(&mut runtime);
    inspect(&publish(&mut runtime), 85.0, true, false);
}

#[test]
fn pointer_deltas_are_independent_of_thin_hit_box_and_fifo_cumulative() {
    let mut runtime = fresh();
    let publication = publish(&mut runtime);
    let input = publication.input_context().clone();
    let pointer_id = PointerId::new(47).unwrap_or_else(|| unreachable!("nonzero pointer"));
    let down = PointerEvent::new(
        pointer_id, PointerDeviceKind::Mouse, PointerPhase::Down,
        LogicalPoint::new(6.0, 40.0).unwrap_or_else(|_| unreachable!()), input.clone()
    ).with_changed_button(PointerButton::Primary)
        .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime.submit_pointer(down).unwrap_or_else(|_| unreachable!("down admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 50.0, "down must not jump to a value derived from 12px width");
    for (position, delta) in [(16.0, 10.0), (25.0, 9.0)] {
        runtime.submit_pointer(
            PointerEvent::new(
                pointer_id, PointerDeviceKind::Mouse, PointerPhase::Move,
                LogicalPoint::new(position, 40.0).unwrap_or_else(|_| unreachable!()), input.clone()
            )
            .with_movement_delta(LogicalDelta::new(delta, 0.0).unwrap_or_else(|_| unreachable!()))
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
        ).unwrap_or_else(|_| unreachable!("captured move admitted"));
    }
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 69.0, "both relative deltas applied to live app state");
    assert_eq!(runtime.state().proposals, 2);
    runtime.submit_pointer(
        PointerEvent::new(
            pointer_id, PointerDeviceKind::Mouse, PointerPhase::Up,
            LogicalPoint::new(25.0, 40.0).unwrap_or_else(|_| unreachable!()), input,
        ).with_changed_button(PointerButton::Primary)
    ).unwrap_or_else(|_| unreachable!("up admitted"));
    settle(&mut runtime);
    inspect(&publish(&mut runtime), 69.0, true, false);
}

#[test]
fn disabled_rejects_input_and_horizontal_orientation_changes_move_axis() {
    let mut runtime = fresh();
    runtime.submit_action(Action::Enabled(false)).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    let disabled = publish(&mut runtime);
    inspect(&disabled, 50.0, false, false);
    assert!(semantic_action(&mut runtime, &disabled, SemanticAction::Increment).is_err());
    runtime.submit_action(Action::Enabled(true)).unwrap_or_else(|_| unreachable!());
    runtime.submit_action(Action::Horizontal(true)).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    let rotated = publish(&mut runtime);
    inspect(&rotated, 50.0, true, true);
    let owner = runtime.index().nodes().iter()
        .find(|node| node.authored_id() == Some(
            &runenui_core::ElementId::new("controlled.splitter").unwrap_or_else(|_| unreachable!())
        ))
        .unwrap_or_else(|| unreachable!("mounted Splitter")).id().clone();
    runtime.submit_command(owner, SemanticCommand::RequestFocus, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("focus"));
    settle(&mut runtime);
    runtime.submit_keyboard(KeyboardEvent::new(
        KeyboardPhase::Down, PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        KeyModifiers::NONE, false, KeyLocation::Standard,
        KeyboardCompositionState::Inactive, None,
    )).unwrap_or_else(|_| unreachable!("keyboard"));
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 55.0);
}

#[test]
fn routed_steps_accumulate_before_rebuild_and_page_step_is_not_duplicated() {
    let mut runtime = fresh();
    let publication = publish(&mut runtime);
    let owner = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id() == Some(
                &runenui_core::ElementId::new("controlled.splitter")
                    .unwrap_or_else(|_| unreachable!()),
            )
        })
        .unwrap_or_else(|| unreachable!("mounted splitter"))
        .id()
        .clone();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::Increment,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("generic controller increment"));
    runtime
        .submit_command(
            owner,
            SemanticCommand::Increment,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("second generic controller increment"));
    assert_eq!(runtime.state().size, 50.0);
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 60.0, "no stale absolute proposals");
    inspect(&publish(&mut runtime), 60.0, true, false);
    assert_eq!(runtime.state().proposals, 2);
    runtime
        .submit_action(Action::Enabled(false))
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert!(semantic_action(&mut runtime, &publication, SemanticAction::Increment).is_err());
}

#[test]
fn disabled_mid_drag_discards_captured_motion_without_mutating_app_size() {
    let mut runtime = fresh();
    let publication = publish(&mut runtime);
    let input = publication.input_context().clone();
    let pointer = PointerId::new(84).unwrap_or_else(|| unreachable!());
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                LogicalPoint::new(6.0, 40.0).unwrap_or_else(|_| unreachable!()),
                input.clone(),
            )
            .with_changed_button(PointerButton::Primary)
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    runtime
        .submit_action(Action::Enabled(false))
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Move,
                LogicalPoint::new(20.0, 40.0).unwrap_or_else(|_| unreachable!()),
                input,
            )
            .with_movement_delta(LogicalDelta::new(14.0, 0.0).unwrap_or_else(|_| unreachable!()))
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 50.0);
    assert_eq!(runtime.state().proposals, 0);
    inspect(&publish(&mut runtime), 50.0, false, false);
}

#[test]
fn horizontal_divider_uses_vertical_pointer_motion_not_horizontal_motion() {
    let mut runtime = fresh();
    runtime.submit_action(Action::Horizontal(true)).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    let publication = publish(&mut runtime);
    let input = publication.input_context().clone();
    let pointer = PointerId::new(85).unwrap_or_else(|| unreachable!());
    runtime.submit_pointer(
        PointerEvent::new(
            pointer, PointerDeviceKind::Touch, PointerPhase::Down,
            LogicalPoint::new(40.0, 6.0).unwrap_or_else(|_| unreachable!()),
            input.clone(),
        ),
    ).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    runtime.submit_pointer(
        PointerEvent::new(
            pointer, PointerDeviceKind::Touch, PointerPhase::Move,
            LogicalPoint::new(60.0, 6.0).unwrap_or_else(|_| unreachable!()),
            input.clone(),
        ).with_movement_delta(LogicalDelta::new(20.0, 0.0).unwrap_or_else(|_| unreachable!())),
    ).unwrap_or_else(|_| unreachable!());
    runtime.submit_pointer(
        PointerEvent::new(
            pointer, PointerDeviceKind::Touch, PointerPhase::Move,
            LogicalPoint::new(60.0, 21.0).unwrap_or_else(|_| unreachable!()),
            input,
        ).with_movement_delta(LogicalDelta::new(0.0, 15.0).unwrap_or_else(|_| unreachable!())),
    ).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(runtime.state().size, 65.0);
    assert_eq!(runtime.state().proposals, 1);
}

struct TwoPaneApp;

#[derive(Clone, Copy)]
struct TwoPaneState {
    left: f64,
}

impl UiApp for TwoPaneApp {
    type State = TwoPaneState;
    type Action = SplitterRequest;
    type HostProtocol = NoHostProtocol;

    #[allow(clippy::cast_possible_truncation)]
    fn root(state: &Self::State) -> impl View<Self::Action> {
        let dimension = |width: f32| {
            runenui_core::LayoutStyle::default()
                .with_width(runenui_core::LayoutDimension::length(
                    runenui_core::LogicalLength::new(width)
                        .unwrap_or_else(|_| unreachable!("app-owned pane size is valid")),
                ))
                .with_height(runenui_core::LayoutDimension::length(
                    runenui_core::LogicalLength::from(160_u16),
                ))
        };
        let left = runenui_core::column(Vec::<Element<SplitterRequest>>::new())
            .id("pane.left")
            .with_layout(dimension(state.left as f32))
            .into_element();
        let divider = splitter("Two panes", 0.0, 200.0, state.left, 5.0)
            .unwrap_or_else(|_| unreachable!("app-clamped range"))
            .on_resize(|request| request)
            .id("pane.splitter")
            .into_element();
        let right = runenui_core::column(Vec::<Element<SplitterRequest>>::new())
            .id("pane.right")
            .with_layout(dimension((200.0 - state.left) as f32))
            .into_element();
        runenui_core::row([left, divider, right])
    }

    fn update(state: &mut Self::State, request: Self::Action) {
        let next = match request {
            SplitterRequest::MoveBy(pixels) => state.left + f64::from(pixels),
            SplitterRequest::AdjustBy(amount) => state.left + amount.get(),
            SplitterRequest::SetValue(value) => value.get(),
        };
        state.left = next.clamp(0.0, 200.0);
    }
}

fn pane_width(
    publication: &runenui_runtime::SurfacePublication,
    id: &str,
) -> f32 {
    publication
        .layout_report()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|candidate| candidate.as_str() == id))
        .unwrap_or_else(|| unreachable!("app-authored pane has layout"))
        .constrained_outer_size()
        .width()
}

#[test]
fn splitter_requests_rebuild_both_application_owned_pane_geometries() {
    let mut runtime = AppRuntime::<TwoPaneApp>::mount(TwoPaneState { left: 80.0 });
    settle_two_pane(&mut runtime);
    let first = publish_two_pane(&mut runtime);
    assert_eq!(pane_width(&first, "pane.left"), 80.0);
    assert_eq!(pane_width(&first, "pane.right"), 120.0);
    let snapshot = first.semantic_publication().snapshot();
    let splitter_node = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Splitter)
        .unwrap_or_else(|| unreachable!("semantic divider"));
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            snapshot.surface_id().clone(),
            splitter_node.id().clone(),
            number(115.0),
        ))
        .unwrap_or_else(|_| unreachable!("semantic resize admitted"));
    assert_eq!(runtime.state().left, 80.0, "admission alone does not mutate panes");
    settle_two_pane(&mut runtime);
    assert_eq!(runtime.state().left, 115.0);
    let second = publish_two_pane(&mut runtime);
    assert_eq!(pane_width(&second, "pane.left"), 115.0);
    assert_eq!(pane_width(&second, "pane.right"), 85.0);
}

fn settle_two_pane(runtime: &mut AppRuntime<TwoPaneApp>) {
    runtime.pump(PumpBudget::new(256, 256, 256, 256));
}

fn publish_two_pane(
    runtime: &mut AppRuntime<TwoPaneApp>,
) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &StyleEnvironment::default(),
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("two-pane application scene publishes"))
}
