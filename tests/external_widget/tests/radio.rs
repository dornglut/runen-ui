#![allow(refining_impl_trait)]

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, EventContext, EventPhase, FocusGroup,
    FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, NoHostProtocol, SemanticAction,
    SemanticCheckedState, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticRole, SemanticState, UiApp, UiEvent, View, Widget,
    WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput,
    WidgetInvalidation, WidgetUpdateContext, column, container,
};
use runenui_runtime::{AppRuntime, MountedNodeId, PumpBudget};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Select(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    selected: u8,
}

#[derive(Debug)]
struct DownstreamRadio {
    label: &'static str,
    value: u8,
    checked: bool,
}

impl Widget<Action> for DownstreamRadio {
    type State = bool;

    fn create_state(&self) -> Self::State {
        self.checked
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.checked {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        *state = self.checked;
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _: &mut Self::State,
        _: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Select(self.value))
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::RadioButton)
                .with_name(self.label)
                .with_state(SemanticState::ENABLED.with_checked(if *state {
                    SemanticCheckedState::Checked
                } else {
                    SemanticCheckedState::Unchecked
                }))
                .with_action(SemanticAction::Activate),
        )
    }
}

#[derive(Debug)]
struct DownstreamRadioGroup;

impl Widget<Action> for DownstreamRadioGroup {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Bubble || context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        let Some(command) = event.as_semantic_command().map(|event| event.command()) else {
            return WidgetEventOutput::none();
        };
        let group_command = match command {
            SemanticCommand::FocusLeft | SemanticCommand::FocusUp => {
                Some(SemanticCommand::FocusGroupPrevious)
            }
            SemanticCommand::FocusRight | SemanticCommand::FocusDown => {
                Some(SemanticCommand::FocusGroupNext)
            }
            _ => None,
        };
        if let Some(group_command) = group_command {
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(group_command);
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::RadioGroup);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<Action> for DownstreamRadioGroup {}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let radios = [("One", 1_u8), ("Two", 2), ("Three", 3)]
            .into_iter()
            .map(|(label, value)| {
                let checked = state.selected == value;
                Element::new(DownstreamRadio {
                    label,
                    value,
                    checked,
                })
                .id(format!("downstream.radio.{value}"))
                .focus_group_preferred(checked)
            })
            .collect::<Vec<_>>();
        let group = container(DownstreamRadioGroup, radios)
            .id("downstream.group")
            .into_element()
            .focus_group(
                FocusGroup::new()
                    .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                    .with_activation(FocusGroupActivationPolicy::ActivateTarget),
            );
        column(vec![group]).into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let Action::Select(value) = action;
        state.selected = value;
    }
}

fn id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("downstream radio node is mounted"))
        .id()
        .clone()
}

fn command(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("downstream radio command is accepted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[test]
fn downstream_radio_composite_matches_public_focus_semantic_and_selection_contracts() {
    let mut runtime = AppRuntime::<App>::mount(State { selected: 1 });
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    let one = id(&mut runtime, "downstream.radio.1");
    let two = id(&mut runtime, "downstream.radio.2");
    let group = id(&mut runtime, "downstream.group");

    assert_eq!(
        runtime
            .index()
            .node(&group)
            .unwrap_or_else(|| unreachable!("group remains publicly inspectable"))
            .focus_group(),
        Some(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_activation(FocusGroupActivationPolicy::ActivateTarget),
        )
    );

    command(&mut runtime, one.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, one, SemanticCommand::FocusRight);
    assert_eq!(runtime.focus().focused_node(), Some(&two));
    assert_eq!(runtime.state().selected, 2);

    let publication = runtime
        .publish_surface(&runenui_runtime::SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            runenui_core::LogicalSize::try_new(640.0, 480.0)
                .unwrap_or_else(|_| unreachable!("test surface is valid")),
        ))
        .unwrap_or_else(|error| unreachable!("downstream radio surface publishes: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let checked = snapshot
        .nodes()
        .iter()
        .find(|node| {
            node.role() == SemanticRole::RadioButton
                && node.state().checked() == Some(SemanticCheckedState::Checked)
        })
        .unwrap_or_else(|| unreachable!("selected downstream radio is published"));
    assert_eq!(checked.name(), Some("Two"));
    assert!(
        snapshot
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::RadioGroup)
    );
}
