#![allow(refining_impl_trait)]

//! Independent downstream Tree/TreeItem authoring over public Widget contracts.
use core::time::Duration;
use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, FocusGroup, FocusGroupActivationPolicy,
    FocusGroupBoundaryPolicy, FocusGroupTypeAhead, Focusability, NoHostProtocol, SemanticAction,
    SemanticCollectionPosition, SemanticCommand, SemanticContribution, SemanticContributionContext,
    SemanticHierarchyLevel, SemanticNodeContribution, SemanticRole, SemanticSelectionMode,
    SemanticState, UiApp, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetUpdateContext, container,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Select(String),
}

#[derive(Debug)]
struct TreeRoot;
impl Widget<Action> for TreeRoot {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn semantics(
        &self,
        &(): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Tree)
            .with_selection_mode(SemanticSelectionMode::Single);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl ChildBearingWidget<Action> for TreeRoot {}

#[derive(Debug)]
struct Branch;
impl Widget<Action> for Branch {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn semantics(
        &self,
        &(): &Self::State,
        context: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::TreeItem)
            .with_name("Folder")
            .with_state(
                SemanticState::ENABLED
                    .with_expanded(true)
                    .with_selected(false),
            )
            .with_hierarchy_level(
                SemanticHierarchyLevel::new(1).unwrap_or_else(|_| unreachable!("level 1 is valid")),
            )
            .with_collection_position(
                SemanticCollectionPosition::new(0, Some(1))
                    .unwrap_or_else(|_| unreachable!("root position valid")),
            )
            .with_action(SemanticAction::RequestFocus);
        if context.has_mounted_children() {
            node = node.with_mounted_children();
        }
        SemanticContribution::single(node)
    }
}
impl ChildBearingWidget<Action> for Branch {}

#[derive(Debug)]
struct Leaf {
    name: &'static str,
    id: &'static str,
    index: u64,
    selected: bool,
}
impl Widget<Action> for Leaf {
    type State = bool;
    fn create_state(&self) -> Self::State {
        self.selected
    }
    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.selected {
            context.invalidate(runenui_core::WidgetInvalidation::SEMANTICS);
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
        WidgetActivationOutput::action(Action::Select(self.id.to_owned()))
    }
    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::TreeItem)
                .with_name(self.name)
                .with_state(SemanticState::ENABLED.with_selected(*state))
                .with_hierarchy_level(
                    SemanticHierarchyLevel::new(2)
                        .unwrap_or_else(|_| unreachable!("level 2 valid")),
                )
                .with_collection_position(
                    SemanticCollectionPosition::new(self.index, Some(2))
                        .unwrap_or_else(|_| unreachable!("sibling position valid")),
                )
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::Activate),
        )
    }
}
struct CustomTree;
impl UiApp for CustomTree {
    type State = String;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> Element<Self::Action> {
        let leaves = [("A", "domain-alpha"), ("B", "domain-beta")]
            .into_iter()
            .enumerate()
            .map(|(index, (name, id))| {
                Element::new(Leaf {
                    name,
                    id,
                    index: u64::try_from(index).unwrap_or_else(|_| unreachable!()),
                    selected: state == id,
                })
                .id(format!("custom.{id}"))
                .with_focusability(Focusability::Focusable)
                .focus_group_search_text(name)
            })
            .collect::<Vec<_>>();
        let branch = container(Branch, leaves)
            .id("custom.branch")
            .into_element()
            .with_focusability(Focusability::Focusable)
            .focus_group_preferred(true);
        container(TreeRoot, vec![branch])
            .id("custom.root")
            .into_element()
            .focus_group(
                FocusGroup::new()
                    .with_boundary(FocusGroupBoundaryPolicy::Stop)
                    .with_activation(FocusGroupActivationPolicy::Manual)
                    .with_type_ahead(
                        FocusGroupTypeAhead::new(Duration::from_millis(500))
                            .unwrap_or_else(|_| unreachable!("bounded search timeout")),
                    ),
            )
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        let Action::Select(id) = action;
        *state = id;
    }
}

fn settle(runtime: &mut AppRuntime<CustomTree>) {
    let batch = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("TreeView pump observation"));
    assert!(batch.report().is_quiescent());
}

#[test]
fn custom_tree_uses_exact_public_hierarchy_focus_and_domain_selection() {
    let mut runtime = AppRuntime::<CustomTree>::mount("domain-alpha".into());
    settle(&mut runtime);
    let branch_id = runenui_core::ElementId::new("custom.branch")
        .unwrap_or_else(|_| unreachable!("valid authoring ID"));
    let leaf_id = runenui_core::ElementId::new("custom.domain-alpha")
        .unwrap_or_else(|_| unreachable!("valid authoring ID"));
    let branch = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&branch_id))
        .unwrap_or_else(|| unreachable!("custom branch mounted"))
        .id()
        .clone();
    let leaf = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&leaf_id))
        .unwrap_or_else(|| unreachable!("custom leaf mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            branch.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus admitted"));
    settle(&mut runtime);
    runtime
        .submit_command(
            branch,
            SemanticCommand::FocusGroupNext,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("group navigation admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&leaf));
    assert_eq!(
        runtime.state(),
        "domain-alpha",
        "focus does not commit selection"
    );
    runtime
        .submit_command(
            leaf,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("activation admitted"));
    settle(&mut runtime);

    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(640.0, 480.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("public Tree contribution: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let tree = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tree)
        .unwrap_or_else(|| unreachable!("downstream Tree projects"));
    assert_eq!(tree.children().len(), 1);
    let branch = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::TreeItem && node.name() == Some("Folder"))
        .unwrap_or_else(|| unreachable!("downstream branch projects"));
    assert_eq!(branch.children().len(), 2);
    assert_eq!(
        branch.hierarchy_level().map(SemanticHierarchyLevel::get),
        Some(1)
    );
    assert!(
        snapshot
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::TreeItem
                && node.name() == Some("A")
                && node.hierarchy_level().map(SemanticHierarchyLevel::get) == Some(2))
    );
}
