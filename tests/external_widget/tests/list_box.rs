#![allow(refining_impl_trait)]

use core::time::Duration;

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, EventContext, EventPhase, FocusGroup,
    FocusGroupActivationPolicy, FocusGroupBoundaryPolicy, FocusGroupTypeAhead, Focusability,
    LayoutContainer, LayoutDimension, LayoutStyle, LogicalLength, NoHostProtocol, OverflowPolicy,
    OverflowStyle, SemanticAction, SemanticCollectionPosition, SemanticCommand,
    SemanticCommandEvent, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticOrientation, SemanticRole, SemanticSelectionMode,
    SemanticState, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation, WidgetUpdateContext, container,
    list_box, option_item, scroll_container,
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
struct DownstreamOption {
    label: &'static str,
    value: u8,
    selected: bool,
    position: SemanticCollectionPosition,
}

impl Widget<Action> for DownstreamOption {
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
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Option)
                .with_name(self.label)
                .with_state(SemanticState::ENABLED.with_selected(*state))
                .with_collection_position(self.position)
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::Activate),
        )
    }
}

#[derive(Debug)]
struct DownstreamListBox;

impl Widget<Action> for DownstreamListBox {
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
            SemanticCommand::FocusUp => Some(SemanticCommand::FocusGroupPrevious),
            SemanticCommand::FocusDown => Some(SemanticCommand::FocusGroupNext),
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
        let mut node = SemanticNodeContribution::primary(SemanticRole::ListBox)
            .with_orientation(SemanticOrientation::Vertical)
            .with_selection_mode(SemanticSelectionMode::Single);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}

impl ChildBearingWidget<Action> for DownstreamListBox {}

struct DownstreamApp;

impl UiApp for DownstreamApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let labels = [("Alpha", 1_u8), ("Bravo", 2), ("Charlie", 3)];
        let known_size =
            u64::try_from(labels.len()).unwrap_or_else(|_| unreachable!("fixture length fits u64"));
        let children = labels
            .into_iter()
            .enumerate()
            .map(|(index, (label, value))| {
                let selected = state.selected == value;
                let position = SemanticCollectionPosition::new(
                    u64::try_from(index).unwrap_or_else(|_| unreachable!("fixture index fits u64")),
                    Some(known_size),
                )
                .unwrap_or_else(|_| unreachable!("fixture collection position is valid"));
                Element::new(DownstreamOption {
                    label,
                    value,
                    selected,
                    position,
                })
                .id(format!("downstream.option.{value}"))
                .with_focusability(Focusability::Focusable)
                .focus_group_preferred(selected)
                .focus_group_search_text(label)
            })
            .collect::<Vec<_>>();

        container(DownstreamListBox, children)
            .id("downstream.list")
            .into_element()
            .focus_group(
                FocusGroup::new()
                    .with_boundary(FocusGroupBoundaryPolicy::Stop)
                    .with_activation(FocusGroupActivationPolicy::Manual)
                    .with_type_ahead(
                        FocusGroupTypeAhead::new(Duration::from_millis(500))
                            .unwrap_or_else(|_| unreachable!("fixture timeout is bounded")),
                    ),
            )
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let Action::Select(value) = action;
        state.selected = value;
    }
}

fn id<App: UiApp>(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("collection fixture node is mounted"))
        .id()
        .clone()
}

fn settle<App: UiApp>(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
}

fn command<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    target: MountedNodeId,
    command: SemanticCommand,
) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("collection command is accepted"));
    settle(runtime);
}

#[test]
fn downstream_list_box_matches_public_focus_semantic_and_application_selection_contracts() {
    let mut runtime = AppRuntime::<DownstreamApp>::mount(State { selected: 1 });
    settle(&mut runtime);
    let one = id(&mut runtime, "downstream.option.1");
    let two = id(&mut runtime, "downstream.option.2");

    command(&mut runtime, one.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, one, SemanticCommand::FocusDown);
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
        .unwrap_or_else(|error| unreachable!("downstream ListBox publishes: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let list = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ListBox)
        .unwrap_or_else(|| unreachable!("downstream ListBox is semantic"));
    assert_eq!(list.orientation(), Some(SemanticOrientation::Vertical));
    assert_eq!(list.selection_mode(), Some(SemanticSelectionMode::Single));
    assert_eq!(list.children().len(), 3);
}

#[derive(Clone, Copy, Debug)]
struct StandardState;

struct StandardScrollApp;

impl UiApp for StandardScrollApp {
    type State = StandardState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        let item_layout = || {
            LayoutStyle::default()
                .with_width(LayoutDimension::Length(LogicalLength::from(20_u8)))
                .with_height(LayoutDimension::Length(LogicalLength::from(20_u8)))
        };
        let list = list_box([
            option_item("Alpha", true)
                .id("standard.option.one")
                .with_layout(item_layout())
                .on_activate(|| ()),
            option_item("Bravo", false)
                .id("standard.option.two")
                .with_layout(item_layout())
                .on_activate(|| ()),
            option_item("Charlie", false)
                .id("standard.option.three")
                .with_layout(item_layout())
                .on_activate(|| ()),
        ])
        .id("standard.list");

        let overflow = OverflowStyle::all(OverflowPolicy::Scroll);
        scroll_container(list, overflow)
            .id("standard.viewport")
            .with_layout(
                LayoutStyle::default()
                    .with_container(LayoutContainer::Block)
                    .with_width(LayoutDimension::Length(LogicalLength::from(20_u8)))
                    .with_height(LayoutDimension::Length(LogicalLength::from(20_u8)))
                    .with_overflow(overflow),
            )
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn standard_list_box_navigation_reveals_through_existing_scroll_container_authority() {
    let mut runtime = AppRuntime::<StandardScrollApp>::mount(StandardState);
    settle(&mut runtime);
    let environment = runenui_core::StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(20.0, 20.0)
                .unwrap_or_else(|_| unreachable!("fixture surface is valid")),
        ))
        .unwrap_or_else(|error| unreachable!("standard ListBox publishes: {error:?}"));

    let one = id(&mut runtime, "standard.option.one");
    let two = id(&mut runtime, "standard.option.two");
    let three = id(&mut runtime, "standard.option.three");
    let viewport = id(&mut runtime, "standard.viewport");

    command(&mut runtime, one.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, one, SemanticCommand::FocusDown);
    assert_eq!(runtime.focus().focused_node(), Some(&two));
    command(&mut runtime, two, SemanticCommand::FocusDown);
    assert_eq!(runtime.focus().focused_node(), Some(&three));

    assert_eq!(
        runtime
            .index()
            .node(&viewport)
            .unwrap_or_else(|| unreachable!("standard viewport remains mounted"))
            .interaction()
            .scroll_offset(),
        (0.0, 40.0)
    );
}
