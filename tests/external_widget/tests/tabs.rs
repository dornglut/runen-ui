#![allow(refining_impl_trait)]

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, ElementId, EventContext, EventPhase, FocusGroup,
    FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, Focusability, NoHostProtocol,
    SemanticAction, SemanticCommand, SemanticCommandEvent, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticOrientation, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState, UiApp, UiEvent,
    View, Widget, WidgetActivation, WidgetActivationContext, WidgetActivationOutput,
    WidgetEventOutput, WidgetInvalidation, WidgetUpdateContext, column, container, text,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Select(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    selected: u8,
}

#[derive(Debug)]
struct DownstreamTab {
    label: &'static str,
    value: u8,
    selected: bool,
    controls: ElementId,
}

impl Widget<Action> for DownstreamTab {
    type State = bool;

    fn create_state(&self) -> Self::State {
        self.selected
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.selected {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        *state = self.selected;
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
        selected: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Tab)
                .with_name(self.label)
                .with_state(SemanticState::ENABLED.with_selected(*selected))
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::Activate)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: self.controls.clone(),
                        semantic_key: None,
                    },
                )),
        )
    }
}

#[derive(Debug)]
struct DownstreamTabList;

impl Widget<Action> for DownstreamTabList {
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
        let Some(command) = event
            .as_semantic_command()
            .map(SemanticCommandEvent::command)
        else {
            return WidgetEventOutput::none();
        };
        let delegated = match command {
            SemanticCommand::FocusLeft => Some(SemanticCommand::FocusGroupPrevious),
            SemanticCommand::FocusRight => Some(SemanticCommand::FocusGroupNext),
            _ => None,
        };
        if let Some(delegated) = delegated {
            context.prevent_default();
            context.stop_propagation();
            context.emit_command(delegated);
        }
        WidgetEventOutput::none()
    }

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::TabList)
            .with_orientation(SemanticOrientation::Horizontal);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<Action> for DownstreamTabList {}

#[derive(Debug)]
struct DownstreamPanel {
    labelled_by: ElementId,
}

impl Widget<Action> for DownstreamPanel {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn semantics(
        &self,
        (): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::TabPanel).with_relationship(
            SemanticRelationship::new(
                SemanticRelationshipKind::LabelledBy,
                SemanticReference::Authored {
                    element_id: self.labelled_by.clone(),
                    semantic_key: None,
                },
            ),
        );
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<Action> for DownstreamPanel {}

struct DownstreamApp;

impl UiApp for DownstreamApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let values = [
            ("One", 1_u8, "downstream.panel.one"),
            ("Two", 2, "downstream.panel.two"),
        ];
        let tabs = values
            .into_iter()
            .map(|(label, value, panel)| {
                let selected = state.selected == value;
                Element::new(DownstreamTab {
                    label,
                    value,
                    selected,
                    controls: ElementId::new(panel)
                        .unwrap_or_else(|_| unreachable!("fixture panel id is valid")),
                })
                .id(format!("downstream.tab.{value}"))
                .with_focusability(Focusability::Focusable)
                .focus_group_preferred(selected)
            })
            .collect::<Vec<_>>();

        let tab_list = container(DownstreamTabList, tabs)
            .id("downstream.tabs")
            .into_element()
            .focus_group(
                FocusGroup::new()
                    .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                    .with_activation(FocusGroupActivationPolicy::Manual),
            );

        let panel_one = container(
            DownstreamPanel {
                labelled_by: ElementId::new("downstream.tab.1")
                    .unwrap_or_else(|_| unreachable!("fixture tab id is valid")),
            },
            [text("Panel one")],
        )
        .id("downstream.panel.one")
        .into_element();
        let panel_two = container(
            DownstreamPanel {
                labelled_by: ElementId::new("downstream.tab.2")
                    .unwrap_or_else(|_| unreachable!("fixture tab id is valid")),
            },
            [text("Panel two")],
        )
        .id("downstream.panel.two")
        .into_element();

        column([tab_list, panel_one, panel_two]).into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let Action::Select(value) = action;
        state.selected = value;
    }
}

fn id(runtime: &mut AppRuntime<DownstreamApp>, authored: &str) -> MountedNodeId {
    let authored = ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("Tabs fixture node is mounted"))
        .id()
        .clone()
}

fn settle(runtime: &mut AppRuntime<DownstreamApp>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            )).expect("pump observation").report().to_owned()
            .is_quiescent()
    );
}

fn command(
    runtime: &mut AppRuntime<DownstreamApp>,
    target: MountedNodeId,
    command: SemanticCommand,
) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("Tabs command is accepted"));
    settle(runtime);
}

#[test]
fn downstream_tabs_match_public_focus_semantic_relationship_and_application_state_contracts() {
    let mut runtime = AppRuntime::<DownstreamApp>::mount(State { selected: 1 });
    settle(&mut runtime);
    let one = id(&mut runtime, "downstream.tab.1");
    let two = id(&mut runtime, "downstream.tab.2");

    command(&mut runtime, one.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, one, SemanticCommand::FocusRight);
    assert_eq!(runtime.focus().focused_node(), Some(&two));
    assert_eq!(runtime.state().selected, 1);

    command(&mut runtime, two, SemanticCommand::Activate);
    assert_eq!(runtime.state().selected, 2);

    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(640.0, 480.0)
                .unwrap_or_else(|_| unreachable!("test surface is valid")),
        ))
        .unwrap_or_else(|error| unreachable!("downstream Tabs publish: {error:?}"));
    assert!(publication.semantic_diagnostics().is_empty());
    let snapshot = publication.semantic_publication().snapshot();
    let list = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::TabList)
        .unwrap_or_else(|| unreachable!("downstream TabList is semantic"));
    assert_eq!(list.orientation(), Some(SemanticOrientation::Horizontal));
    assert_eq!(list.children().len(), 2);

    let tab = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tab && node.name() == Some("Two"))
        .unwrap_or_else(|| unreachable!("downstream Tab is semantic"));
    assert_eq!(tab.state().selected(), Some(true));
    let controls = tab
        .relationships()
        .iter()
        .find(|relationship| relationship.kind() == SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("downstream Tab controls one panel"));
    assert_eq!(
        snapshot
            .node(controls.target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::TabPanel)
    );
    let panel = snapshot
        .node(controls.target())
        .unwrap_or_else(|| unreachable!("controlled downstream panel exists"));
    let labelled_by = panel
        .relationships()
        .iter()
        .find(|relationship| relationship.kind() == SemanticRelationshipKind::LabelledBy)
        .unwrap_or_else(|| unreachable!("downstream panel is labelled by its Tab"));
    assert_eq!(
        snapshot
            .node(labelled_by.target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::Tab)
    );
}
