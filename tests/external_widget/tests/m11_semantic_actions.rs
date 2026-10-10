#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, NoHostProtocol, SemanticAction,
    SemanticActionData, SemanticActionRequest, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticItem, SemanticKey, SemanticNodeContribution,
    SemanticNumber, SemanticRange, SemanticRole, SemanticState, StyleEnvironment, UiApp, UiEvent,
    View, Widget, WidgetEventOutput,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SubmitSemanticActionError,
    SubmitSemanticActionErrorKind, SurfaceBuildContext,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    command: SemanticCommand,
    semantic_data: Option<SemanticActionData>,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    SetValue(SemanticNumber),
    Expand,
    SetReadOnly(bool),
}

#[derive(Debug)]
struct State {
    value: SemanticNumber,
    expanded: bool,
    read_only: bool,
    observations: Rc<RefCell<Vec<Observation>>>,
}

#[derive(Debug)]
struct Probe {
    value: SemanticNumber,
    expanded: bool,
    read_only: bool,
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<Action> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }
        let Some(command) = event.as_semantic_command() else {
            return WidgetEventOutput::none();
        };
        self.observations.borrow_mut().push(Observation {
            command: command.command(),
            semantic_data: command
                .semantic_action_target()
                .and_then(|target| target.data().cloned()),
        });
        match command.command() {
            SemanticCommand::SetValue(value) => context.emit(Action::SetValue(value)),
            SemanticCommand::Expand => context.emit(Action::Expand),
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let range_key = SemanticKey::from_static("range")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let expand_key = SemanticKey::from_static("expand")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let minimum = SemanticNumber::new(0.0)
            .unwrap_or_else(|_| unreachable!("controlled minimum is finite"));
        let maximum = SemanticNumber::new(10.0)
            .unwrap_or_else(|_| unreachable!("controlled maximum is finite"));
        let range = SemanticNodeContribution::new(range_key, SemanticRole::SpinButton)
            .with_name("Range")
            .with_state(SemanticState::ENABLED.with_read_only(self.read_only))
            .with_range(
                SemanticRange::new(Some(minimum), Some(maximum), Some(self.value))
                    .unwrap_or_else(|_| unreachable!("controlled range is valid")),
            )
            .with_action(SemanticAction::Increment)
            .with_action(SemanticAction::Decrement)
            .with_action(SemanticAction::SetValue);
        let expander = SemanticNodeContribution::new(expand_key, SemanticRole::Button)
            .with_name("Expander")
            .with_state(SemanticState::ENABLED.with_expanded(self.expanded))
            .with_action(SemanticAction::Expand)
            .with_action(SemanticAction::Collapse);
        let progress = SemanticNodeContribution::new(
            SemanticKey::from_static("progress")
                .unwrap_or_else(|_| unreachable!("static semantic key is valid")),
            SemanticRole::Progress,
        )
        .with_name("Progress")
        .with_range(
            SemanticRange::new(Some(minimum), Some(maximum), Some(self.value))
                .unwrap_or_else(|_| unreachable!("controlled range is valid")),
        )
        .with_action(SemanticAction::SetValue);
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group).with_children(vec![
                SemanticItem::node(range),
                SemanticItem::node(expander),
                SemanticItem::node(progress),
            ]),
        )
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            value: state.value,
            expanded: state.expanded,
            read_only: state.read_only,
            observations: Rc::clone(&state.observations),
        })
        .id("probe")
        .key("probe")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::SetValue(value) => state.value = value,
            Action::Expand => state.expanded = true,
            Action::SetReadOnly(read_only) => state.read_only = read_only,
        }
    }
}

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("controlled value is finite"))
}

fn app_runtime(read_only: bool) -> AppRuntime<App> {
    AppRuntime::mount(State {
        value: number(5.0),
        expanded: false,
        read_only,
        observations: Rc::new(RefCell::new(Vec::new())),
    })
}

fn publish(
    runtime: &mut AppRuntime<App>,
) -> (
    runenui_core::SurfaceId,
    runenui_core::SemanticNodeId,
    runenui_core::SemanticNodeId,
) {
    runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0)).expect("pump observation").report().to_owned();
    let style = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("semantic publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();
    let range = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Range"))
        .unwrap_or_else(|| unreachable!("range semantic node is published"));
    let expander = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Expander"))
        .unwrap_or_else(|| unreachable!("expander semantic node is published"));
    (
        snapshot.surface_id().clone(),
        range.id().clone(),
        expander.id().clone(),
    )
}

fn expect_rejection(
    result: Result<runenui_runtime::CommandSubmission, SubmitSemanticActionError>,
) -> SubmitSemanticActionError {
    let Err(error) = result else {
        unreachable!("semantic action was expected to reject")
    };
    error
}

fn pump_one(runtime: &mut AppRuntime<App>) {
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX)).expect("pump observation").report().to_owned()
            .processed_envelopes(),
        1
    );
}

#[test]
fn set_value_routes_exact_payload_before_application_update_and_direct_command_converges() {
    let mut runtime = app_runtime(false);
    let (surface, range, _) = publish(&mut runtime);
    let value = number(7.0);
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(surface, range, value))
        .unwrap_or_else(|_| unreachable!("in-range SetValue is admitted"));

    pump_one(&mut runtime);
    assert_eq!(runtime.state().value, number(5.0));
    assert_eq!(
        runtime.state().observations.borrow().as_slice(),
        &[Observation {
            command: SemanticCommand::SetValue(value),
            semantic_data: Some(SemanticActionData::NumericValue(value)),
        }]
    );

    pump_one(&mut runtime);
    assert_eq!(runtime.state().value, value);

    runtime.state().observations.borrow_mut().clear();
    let owner = runtime.index().nodes()[0].id().clone();
    let direct = number(8.0);
    runtime
        .submit_command(
            owner,
            SemanticCommand::SetValue(direct),
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("direct SetValue command is admitted"));
    pump_one(&mut runtime);
    assert_eq!(
        runtime.state().observations.borrow().as_slice(),
        &[Observation {
            command: SemanticCommand::SetValue(direct),
            semantic_data: None,
        }]
    );
    pump_one(&mut runtime);
    assert_eq!(runtime.state().value, direct);
}

#[test]
fn set_value_rejects_missing_payload_out_of_range_and_read_only_state() {
    let mut runtime = app_runtime(false);
    let (surface, range, _) = publish(&mut runtime);

    let missing = expect_rejection(runtime.submit_semantic_action(SemanticActionRequest::new(
        surface.clone(),
        range.clone(),
        SemanticAction::SetValue,
    )));
    assert_eq!(
        missing.kind(),
        SubmitSemanticActionErrorKind::UnsupportedAction
    );

    let out_of_range = expect_rejection(runtime.submit_semantic_action(
        SemanticActionRequest::set_value(surface, range, number(11.0)),
    ));
    assert_eq!(
        out_of_range.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );

    let mut read_only = app_runtime(true);
    let (surface, range, _) = publish(&mut read_only);
    let rejected = expect_rejection(read_only.submit_semantic_action(
        SemanticActionRequest::set_value(surface, range, number(6.0)),
    ));
    assert_eq!(
        rejected.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );
}

#[test]
fn expand_and_collapse_follow_current_authored_expanded_state() {
    let mut runtime = app_runtime(false);
    let (surface, _, expander) = publish(&mut runtime);

    let collapse = expect_rejection(runtime.submit_semantic_action(SemanticActionRequest::new(
        surface.clone(),
        expander.clone(),
        SemanticAction::Collapse,
    )));
    assert_eq!(
        collapse.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );

    runtime
        .submit_semantic_action(SemanticActionRequest::new(
            surface,
            expander,
            SemanticAction::Expand,
        ))
        .unwrap_or_else(|_| unreachable!("collapsed expander admits Expand"));
    pump_one(&mut runtime);
    assert!(!runtime.state().expanded);
    pump_one(&mut runtime);
    assert!(runtime.state().expanded);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn processing_time_revalidation_uses_republished_current_range_state_before_callback() {
    let mut runtime = app_runtime(false);
    let (surface, range, _) = publish(&mut runtime);
    runtime
        .submit_action(Action::SetReadOnly(true))
        .unwrap_or_else(|_| unreachable!("reconfiguration enters the FIFO"));
    runtime
        .submit_semantic_action(SemanticActionRequest::set_value(
            surface,
            range,
            number(7.0),
        ))
        .unwrap_or_else(|_| unreachable!("request is admitted against current writable semantics"));

    pump_one(&mut runtime);
    assert!(runtime.state().read_only);
    let style = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style,
            runenui_runtime::LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("updated read-only semantics republish"));

    pump_one(&mut runtime);
    assert_eq!(runtime.state().value, number(5.0));
    assert!(runtime.state().observations.borrow().is_empty());
}

#[test]
fn progress_cannot_advertise_or_execute_range_mutation() {
    let mut runtime = app_runtime(false);
    publish(&mut runtime);
    let style = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style,
            runenui_runtime::LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("current semantic surface republishes"));
    let progress = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Progress"))
        .unwrap_or_else(|| unreachable!("progress node is published"));
    assert!(
        !progress
            .supported_actions()
            .contains(&SemanticAction::SetValue)
    );

    let rejected = expect_rejection(
        runtime.submit_semantic_action(SemanticActionRequest::set_value(
            publication
                .semantic_publication()
                .snapshot()
                .surface_id()
                .clone(),
            progress.id().clone(),
            number(7.0),
        )),
    );
    assert_eq!(
        rejected.kind(),
        SubmitSemanticActionErrorKind::UnsupportedAction
    );
}

#[test]
fn unhandled_increment_has_no_runtime_or_application_default_mutation() {
    let mut runtime = app_runtime(false);
    publish(&mut runtime);
    let owner = runtime.index().nodes()[0].id().clone();
    runtime.state().observations.borrow_mut().clear();
    runtime
        .submit_command(
            owner,
            SemanticCommand::Increment,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("direct Increment command is admitted"));
    pump_one(&mut runtime);

    assert_eq!(runtime.state().value, number(5.0));
    assert!(!runtime.state().expanded);
    assert_eq!(
        runtime.state().observations.borrow().as_slice(),
        &[Observation {
            command: SemanticCommand::Increment,
            semantic_data: None,
        }]
    );
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX)).expect("pump observation").report().to_owned()
            .processed_envelopes(),
        0
    );
}
