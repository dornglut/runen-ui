#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, HitContribution, HitContributionContext,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    LogicalPoint, LogicalRect, NoHostProtocol, PhysicalKey, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticAction,
    SemanticActionRequest, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticNumber, SemanticOrientation, SemanticRange, SemanticRole,
    SemanticState, StyleEnvironment, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetMeasure, WidgetMeasureInput, slider,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext};

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("test number is finite"))
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Set(SemanticNumber),
    Enabled(bool),
    External(bool),
}

struct State {
    value: SemanticNumber,
    enabled: bool,
    external: bool,
    proposals: u32,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        if state.external {
            Element::new(ExternalSlider {
                value: state.value,
                enabled: state.enabled,
            })
            .id("controlled.slider")
            .into_element()
        } else {
            slider("Volume", 0.0, 100.0, state.value.get(), 5.0)
                .unwrap_or_else(|_| unreachable!("application range is checked"))
                .value_text("Volume percentage")
                .enabled(state.enabled)
                .on_change(Action::Set)
                .id("controlled.slider")
                .into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Set(value) => {
                state.proposals += 1;
                state.value = value;
            }
            Action::Enabled(enabled) => state.enabled = enabled,
            Action::External(external) => state.external = external,
        }
    }
}

#[derive(Debug)]
struct ExternalSlider {
    value: SemanticNumber,
    enabled: bool,
}

impl Widget<Action> for ExternalSlider {
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
        let proposed = match event.as_semantic_command().map(|command| command.command()) {
            Some(SemanticCommand::Increment) => (self.value.get() + 5.0).min(100.0),
            Some(SemanticCommand::Decrement) => (self.value.get() - 5.0).max(0.0),
            Some(SemanticCommand::SetValue(value)) => value.get(),
            _ => return WidgetEventOutput::none(),
        };
        if (0.0..=100.0).contains(&proposed) && proposed != self.value.get() {
            context.emit(Action::Set(number(proposed)));
            context.prevent_default();
            context.stop_propagation();
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            runenui_core::LogicalLength::from(160_u16),
            runenui_core::LogicalLength::from(24_u16),
        )
    }

    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("finite final size")),
        )
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let range = SemanticRange::new(Some(number(0.0)), Some(number(100.0)), Some(self.value))
            .unwrap_or_else(|_| unreachable!("controlled range valid"))
            .with_small_step(number(5.0))
            .unwrap_or_else(|_| unreachable!("positive step"))
            .with_value_text("Volume percentage")
            .unwrap_or_else(|_| unreachable!("has current"));
        let mut node = SemanticNodeContribution::primary(SemanticRole::Slider)
            .with_name("Volume")
            .with_orientation(SemanticOrientation::Horizontal)
            .with_range(range)
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
        value: number(50.0),
        enabled: true,
        external: false,
        proposals: 0,
    });
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(256, 256, 256, 256));
}

fn publish(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("slider scene publishes"))
}

fn inspect(publication: &runenui_runtime::SurfacePublication, expected: f64, enabled: bool) {
    let nodes = publication.semantic_publication().snapshot();
    let slider = nodes
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Slider)
        .unwrap_or_else(|| unreachable!("slider role published"));
    assert_eq!(slider.name(), Some("Volume"));
    assert_eq!(slider.orientation(), Some(SemanticOrientation::Horizontal));
    let range = slider
        .range()
        .unwrap_or_else(|| unreachable!("range published"));
    assert_eq!(range.current(), Some(number(expected)));
    assert_eq!(range.small_step(), Some(number(5.0)));
    assert_eq!(range.value_text(), Some("Volume percentage"));
    assert_eq!(slider.state().disabled(), !enabled);
    for action in [
        SemanticAction::Increment,
        SemanticAction::Decrement,
        SemanticAction::SetValue,
    ] {
        assert_eq!(slider.supported_actions().contains(&action), enabled);
    }
}

fn semantic_action(
    runtime: &mut AppRuntime<App>,
    publication: &runenui_runtime::SurfacePublication,
    action: SemanticAction,
) -> Result<runenui_runtime::CommandSubmission, runenui_runtime::SubmitSemanticActionError> {
    let snapshot = publication.semantic_publication().snapshot();
    let node = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Slider)
        .unwrap_or_else(|| unreachable!("slider is published"));
    runtime.submit_semantic_action(SemanticActionRequest::new(
        snapshot.surface_id().clone(),
        node.id().clone(),
        action,
    ))
}

#[test]
fn standard_slider_semantic_action_converges_only_through_application() {
    let mut runtime = fresh();
    let before = publish(&mut runtime);
    inspect(&before, 50.0, true);
    semantic_action(&mut runtime, &before, SemanticAction::Increment)
        .unwrap_or_else(|_| unreachable!("enabled increment admitted"));
    assert_eq!(runtime.state().value, number(50.0));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, number(55.0));
    assert_eq!(runtime.state().proposals, 1);
    let after = publish(&mut runtime);
    inspect(&after, 55.0, true);
    let snapshot = after.semantic_publication().snapshot();
    let slider_node = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Slider)
        .unwrap_or_else(|| unreachable!());
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            snapshot.surface_id().clone(),
            slider_node.id().clone(),
            number(80.0),
        ))
        .unwrap_or_else(|_| unreachable!("exact SetValue accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, number(80.0));
    inspect(&publish(&mut runtime), 80.0, true);
}

#[test]
fn disabled_action_rejects_and_downstream_custom_widget_preserves_range_contract() {
    let mut runtime = fresh();
    runtime
        .submit_action(Action::Enabled(false))
        .unwrap_or_else(|_| unreachable!("disable accepted"));
    settle(&mut runtime);
    let disabled = publish(&mut runtime);
    inspect(&disabled, 50.0, false);
    assert!(semantic_action(&mut runtime, &disabled, SemanticAction::Increment).is_err());
    assert_eq!(runtime.state().proposals, 0);
    runtime
        .submit_action(Action::Enabled(true))
        .unwrap_or_else(|_| unreachable!("enable accepted"));
    runtime
        .submit_action(Action::External(true))
        .unwrap_or_else(|_| unreachable!("switch implementation accepted"));
    settle(&mut runtime);
    let custom = publish(&mut runtime);
    inspect(&custom, 50.0, true);
    semantic_action(&mut runtime, &custom, SemanticAction::Increment)
        .unwrap_or_else(|_| unreachable!("downstream increment admitted"));
    settle(&mut runtime);
    inspect(&publish(&mut runtime), 55.0, true);
    runtime
        .submit_action(Action::External(false))
        .unwrap_or_else(|_| unreachable!("restoring standard widget accepted"));
    settle(&mut runtime);
    inspect(&publish(&mut runtime), 55.0, true);
}

#[test]
fn keyboard_and_pointer_requests_are_routed_through_same_application_fifo() {
    let mut runtime = fresh();
    let publication = publish(&mut runtime);
    let owner = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                == Some(
                    &runenui_core::ElementId::new("controlled.slider")
                        .unwrap_or_else(|_| unreachable!()),
                )
        })
        .unwrap_or_else(|| unreachable!("mounted slider"))
        .id()
        .clone();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus command accepted"));
    settle(&mut runtime);
    let keyboard = KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::ArrowRight,
        LogicalKey::ArrowRight,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    );
    runtime
        .submit_keyboard(keyboard)
        .unwrap_or_else(|_| unreachable!("key accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, number(55.0));
    let context = publication.input_context().clone();
    let point = LogicalPoint::new(120.0, 12.0).unwrap_or_else(|_| unreachable!("finite position"));
    let pointer = PointerId::new(41).unwrap_or_else(|| unreachable!("nonzero pointer"));
    let down = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        context.clone(),
    )
    .with_changed_button(PointerButton::Primary)
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("pointer down accepted"));
    settle(&mut runtime);
    assert!(runtime.state().value.get() > 55.0);
    let move_event = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        LogicalPoint::new(155.0, 12.0).unwrap_or_else(|_| unreachable!("finite")),
        context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime
        .submit_pointer(move_event)
        .unwrap_or_else(|_| unreachable!("captured drag accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, number(100.0));
    let up = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        LogicalPoint::new(155.0, 12.0).unwrap_or_else(|_| unreachable!("finite")),
        context,
    )
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("terminal pointer accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, number(100.0));
    inspect(&publish(&mut runtime), 100.0, true);
}

#[test]
fn disabled_mid_drag_releases_capture_without_emitting_more_values() {
    let mut runtime = fresh();
    let publication = publish(&mut runtime);
    let context = publication.input_context().clone();
    let pointer = PointerId::new(52).unwrap_or_else(|| unreachable!("nonzero pointer"));
    let down = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        LogicalPoint::new(120.0, 12.0).unwrap_or_else(|_| unreachable!("finite")),
        context.clone(),
    )
    .with_changed_button(PointerButton::Primary)
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("down admitted"));
    settle(&mut runtime);
    let value_after_down = runtime.state().value;
    assert!(value_after_down.get() > 50.0);

    runtime
        .submit_action(Action::Enabled(false))
        .unwrap_or_else(|_| unreachable!("disable accepted"));
    settle(&mut runtime);
    inspect(&publish(&mut runtime), value_after_down.get(), false);
    let previous_proposals = runtime.state().proposals;

    let captured_move = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        LogicalPoint::new(155.0, 12.0).unwrap_or_else(|_| unreachable!("finite")),
        context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    runtime
        .submit_pointer(captured_move)
        .unwrap_or_else(|_| unreachable!("captured move admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, value_after_down);
    assert_eq!(runtime.state().proposals, previous_proposals);

    let terminal = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        LogicalPoint::new(155.0, 12.0).unwrap_or_else(|_| unreachable!("finite")),
        context,
    )
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(terminal)
        .unwrap_or_else(|_| unreachable!("up admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().proposals, previous_proposals);
    runtime
        .submit_action(Action::Enabled(true))
        .unwrap_or_else(|_| unreachable!("enable accepted"));
    settle(&mut runtime);
    let resumed = publish(&mut runtime);
    inspect(&resumed, value_after_down.get(), true);
}

#[test]
fn nonprimary_pointer_down_cannot_modify_application_value() {
    let mut runtime = fresh();
    let publication = publish(&mut runtime);
    let pointer = PointerId::new(53).unwrap_or_else(|| unreachable!("nonzero pointer"));
    let secondary = PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        LogicalPoint::new(150.0, 12.0).unwrap_or_else(|_| unreachable!("finite")),
        publication.input_context().clone(),
    )
    .with_changed_button(PointerButton::Secondary)
    .with_buttons(PointerButtons::new([PointerButton::Secondary]));
    runtime
        .submit_pointer(secondary)
        .unwrap_or_else(|_| unreachable!("secondary pointer admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().value, number(50.0));
    assert_eq!(runtime.state().proposals, 0);
}
