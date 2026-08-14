use runenui_core::{
    CommandOrigin, Element, IntoEffects, LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol,
    SemanticAction, SemanticBounds, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticKey, SemanticNodeContribution, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState, SemanticText,
    SemanticValue, StyleTokens, UiApp, View, Widget, WidgetActivation, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SemanticNode, SemanticNodeId, SemanticPublication,
    SemanticRevision, SemanticSnapshot, SemanticUpdate, SemanticUpdateResult, SurfaceBuildContext,
    SurfaceId,
};

const fn full_budget() -> PumpBudget {
    PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX)
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::new(
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite")),
        LogicalSize::try_new(width, height)
            .unwrap_or_else(|_| unreachable!("test semantic size is valid")),
    )
}

fn all_m5_actions(node: SemanticNodeContribution) -> SemanticNodeContribution {
    node.with_action(SemanticAction::Activate)
        .with_action(SemanticAction::RequestFocus)
        .with_action(SemanticAction::OpenMenu)
        .with_action(SemanticAction::OpenContextMenu)
}

#[derive(Debug)]
struct SupportProbe {
    prefix: &'static str,
    activation: WidgetActivation,
    inert: bool,
}

impl Widget<()> for SupportProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        self.activation
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let named = SemanticKey::from_static("named")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let primary = all_m5_actions(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name(format!("{}-primary", self.prefix))
                .with_state(SemanticState::ENABLED.with_inert(self.inert)),
        )
        .with_child(
            all_m5_actions(SemanticNodeContribution::new(named, SemanticRole::Button))
                .with_name(format!("{}-named", self.prefix)),
        );
        SemanticContribution::single(primary)
    }
}

struct SupportApp;

impl UiApp for SupportApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(vec![
            Element::new(SupportProbe {
                prefix: "auto-actionable",
                activation: WidgetActivation::actionable(true),
                inert: false,
            })
            .id("support.auto-actionable")
            .key("auto-actionable"),
            Element::new(SupportProbe {
                prefix: "auto-passive",
                activation: WidgetActivation::NONE,
                inert: false,
            })
            .id("support.auto-passive")
            .key("auto-passive"),
            Element::new(SupportProbe {
                prefix: "explicit-focus",
                activation: WidgetActivation::NONE,
                inert: false,
            })
            .id("support.explicit-focus")
            .key("explicit-focus")
            .focusable(true),
            Element::new(SupportProbe {
                prefix: "disabled-actionable",
                activation: WidgetActivation::actionable(false),
                inert: true,
            })
            .id("support.disabled-actionable")
            .key("disabled-actionable"),
        ])
        .key("support.root")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn named_node<'a>(snapshot: &'a SemanticSnapshot, name: &str) -> &'a SemanticNode {
    snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some(name))
        .unwrap_or_else(|| unreachable!("named semantic conformance node is published"))
}

#[test]
fn public_support_matrix_separates_support_from_current_availability() {
    let mut runtime = AppRuntime::<SupportApp>::mount(());
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("support conformance publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();

    assert_eq!(
        named_node(snapshot, "auto-actionable-primary").supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::RequestFocus,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    assert_eq!(
        named_node(snapshot, "auto-actionable-named").supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    assert_eq!(
        named_node(snapshot, "auto-passive-primary").supported_actions(),
        &[SemanticAction::OpenMenu, SemanticAction::OpenContextMenu]
    );
    assert_eq!(
        named_node(snapshot, "auto-passive-named").supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    assert_eq!(
        named_node(snapshot, "explicit-focus-primary").supported_actions(),
        &[
            SemanticAction::RequestFocus,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );

    let disabled = named_node(snapshot, "disabled-actionable-primary");
    assert!(disabled.state().disabled());
    assert!(disabled.state().inert());
    assert_eq!(
        disabled.supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::RequestFocus,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    let disabled_named = named_node(snapshot, "disabled-actionable-named");
    assert!(disabled_named.state().disabled());
    assert_eq!(
        disabled_named.supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
}

#[derive(Clone, Copy, Debug)]
enum AdapterPhase {
    Initial,
    Changed,
    Final,
}

#[derive(Clone, Copy, Debug)]
struct SetAdapterPhase(AdapterPhase);

#[derive(Debug)]
struct AdapterProbe(AdapterPhase);

impl Widget<SetAdapterPhase> for AdapterProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let detail = SemanticKey::from_static("detail")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let extra = SemanticKey::from_static("extra")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let primary = match self.0 {
            AdapterPhase::Initial => all_m5_actions(
                SemanticNodeContribution::primary(SemanticRole::Group)
                    .with_name("adapter root")
                    .with_description("initial")
                    .with_value(SemanticValue::Integer(1))
                    .with_bounds(SemanticBounds::OwnerLocal(rect(1.0, 2.0, 10.0, 5.0)))
                    .with_relationship(SemanticRelationship::new(
                        SemanticRelationshipKind::LabelledBy,
                        SemanticReference::Local(detail.clone()),
                    )),
            )
            .with_child(
                SemanticNodeContribution::new(detail, SemanticRole::Text)
                    .with_name("detail")
                    .with_text(SemanticText::plain("initial detail")),
            ),
            AdapterPhase::Changed => all_m5_actions(
                SemanticNodeContribution::primary(SemanticRole::Button)
                    .with_name("adapter root changed")
                    .with_description("changed")
                    .with_value(SemanticValue::Integer(2))
                    .with_state(SemanticState::ENABLED.with_inert(true))
                    .with_bounds(SemanticBounds::OwnerLocal(rect(2.0, 3.0, 12.0, 6.0)))
                    .with_relationship(SemanticRelationship::new(
                        SemanticRelationshipKind::DescribedBy,
                        SemanticReference::Local(extra.clone()),
                    )),
            )
            .with_child(
                SemanticNodeContribution::new(detail, SemanticRole::Text)
                    .with_name("detail changed")
                    .with_text(SemanticText::plain("changed detail")),
            )
            .with_child(
                SemanticNodeContribution::new(extra, SemanticRole::Text)
                    .with_name("extra")
                    .with_value(SemanticValue::Boolean(true)),
            ),
            AdapterPhase::Final => SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name("adapter root final")
                .with_description("final")
                .with_value(SemanticValue::Integer(3))
                .with_action(SemanticAction::OpenMenu)
                .with_action(SemanticAction::OpenContextMenu)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Local(extra.clone()),
                ))
                .with_child(
                    SemanticNodeContribution::new(extra, SemanticRole::Text)
                        .with_name("extra")
                        .with_value(SemanticValue::Boolean(false)),
                ),
        };
        SemanticContribution::single(primary)
    }
}

struct AdapterApp;

impl UiApp for AdapterApp {
    type State = AdapterPhase;
    type Action = SetAdapterPhase;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(AdapterProbe(*state))
            .id("adapter.owner")
            .key("adapter.owner")
            .focusable(true)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state = action.0;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct AdapterRelationship {
    kind: SemanticRelationshipKind,
    target: SemanticNodeId,
}

#[derive(Clone, Debug, PartialEq)]
struct AdapterNode {
    id: SemanticNodeId,
    parent: Option<SemanticNodeId>,
    children: Vec<SemanticNodeId>,
    role: SemanticRole,
    name: Option<String>,
    description: Option<String>,
    value: Option<SemanticValue>,
    disabled: bool,
    inert: bool,
    supported_actions: Vec<SemanticAction>,
    relationships: Vec<AdapterRelationship>,
    bounds: LogicalRect,
    text: Option<SemanticText>,
}

impl AdapterNode {
    fn read(node: &SemanticNode) -> Self {
        Self {
            id: node.id().clone(),
            parent: node.parent().cloned(),
            children: node.children().to_vec(),
            role: node.role(),
            name: node.name().map(str::to_owned),
            description: node.description().map(str::to_owned),
            value: node.value().cloned(),
            disabled: node.state().disabled(),
            inert: node.state().inert(),
            supported_actions: node.supported_actions().to_vec(),
            relationships: node
                .relationships()
                .iter()
                .map(|relationship| AdapterRelationship {
                    kind: relationship.kind(),
                    target: relationship.target().clone(),
                })
                .collect(),
            bounds: node.bounds(),
            text: node.text().cloned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct AdapterSnapshot {
    surface: SurfaceId,
    revision: SemanticRevision,
    roots: Vec<SemanticNodeId>,
    focused: Option<SemanticNodeId>,
    nodes: Vec<AdapterNode>,
}

impl AdapterSnapshot {
    fn read(snapshot: &SemanticSnapshot) -> Self {
        Self {
            surface: snapshot.surface_id().clone(),
            revision: snapshot.revision(),
            roots: snapshot.roots().to_vec(),
            focused: snapshot.focused().cloned(),
            nodes: snapshot.nodes().iter().map(AdapterNode::read).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct AdapterFocusChange {
    previous: Option<SemanticNodeId>,
    current: Option<SemanticNodeId>,
}

#[derive(Clone, Debug, PartialEq)]
struct AdapterUpdate {
    surface: SurfaceId,
    previous_revision: SemanticRevision,
    revision: SemanticRevision,
    removed: Vec<SemanticNodeId>,
    added: Vec<AdapterNode>,
    changed: Vec<AdapterNode>,
    roots: Option<Vec<SemanticNodeId>>,
    focus: Option<AdapterFocusChange>,
}

impl AdapterUpdate {
    fn read(update: &SemanticUpdate) -> Self {
        Self {
            surface: update.surface_id().clone(),
            previous_revision: update.previous_revision(),
            revision: update.revision(),
            removed: update.removed().to_vec(),
            added: update.added().iter().map(AdapterNode::read).collect(),
            changed: update.changed().iter().map(AdapterNode::read).collect(),
            roots: update.roots().map(<[SemanticNodeId]>::to_vec),
            focus: update.focus().map(|focus| AdapterFocusChange {
                previous: focus.previous().cloned(),
                current: focus.current().cloned(),
            }),
        }
    }
}

fn publish_adapter(runtime: &mut AppRuntime<AdapterApp>) -> runenui_runtime::SurfacePublication {
    let tokens = StyleTokens::new();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("adapter conformance publication is admitted"))
}

fn set_adapter_phase(runtime: &mut AppRuntime<AdapterApp>, phase: AdapterPhase) {
    runtime
        .submit_action(SetAdapterPhase(phase))
        .unwrap_or_else(|_| unreachable!("adapter phase change is admitted"));
    assert!(runtime.pump(full_budget()).is_quiescent());
}

fn expect_delta<'a>(
    publication: &'a SemanticPublication,
    surface: &SurfaceId,
    revision: SemanticRevision,
) -> &'a SemanticUpdate {
    match publication.update_from(surface, revision) {
        SemanticUpdateResult::Delta(update) => update,
        SemanticUpdateResult::Unchanged | SemanticUpdateResult::FullResync(_) => {
            unreachable!("declared consecutive adapter base yields one delta")
        }
    }
}

#[test]
fn independent_adapter_shaped_consumer_reads_snapshot_delta_focus_and_resync_contract() {
    let mut runtime = AppRuntime::<AdapterApp>::mount(AdapterPhase::Initial);
    let first = publish_adapter(&mut runtime);
    let first_semantics = first.semantic_publication().clone();
    let first_snapshot = first_semantics.snapshot();
    let adapter_first = AdapterSnapshot::read(first_snapshot);
    assert_eq!(adapter_first.revision.get(), 1);
    assert_eq!(&adapter_first.surface, first_snapshot.surface_id());
    assert_eq!(adapter_first.roots.as_slice(), first_snapshot.roots());
    assert_eq!(adapter_first.nodes.len(), 2);
    assert!(adapter_first.focused.is_none());
    assert!(first_semantics.update().is_none());

    let first_root = adapter_first
        .nodes
        .first()
        .unwrap_or_else(|| unreachable!("adapter root is mapped"));
    assert_eq!(first_root.parent, None);
    assert_eq!(first_root.children.len(), 1);
    assert_eq!(first_root.role, SemanticRole::Group);
    assert_eq!(first_root.name.as_deref(), Some("adapter root"));
    assert_eq!(first_root.description.as_deref(), Some("initial"));
    assert_eq!(first_root.value, Some(SemanticValue::Integer(1)));
    assert!(!first_root.disabled);
    assert!(!first_root.inert);
    assert_eq!(first_root.supported_actions.len(), 4);
    assert_eq!(first_root.relationships.len(), 1);
    assert_eq!(
        first_root.relationships[0].kind,
        SemanticRelationshipKind::LabelledBy
    );
    assert_eq!(
        &first_root.relationships[0].target,
        first_root
            .children
            .first()
            .unwrap_or_else(|| unreachable!("label target is the mapped child"))
    );
    assert!((first_root.bounds.width() - 10.0).abs() <= f32::EPSILON);
    assert!(first_root.text.is_none());
    let first_detail = adapter_first
        .nodes
        .get(1)
        .unwrap_or_else(|| unreachable!("adapter detail is mapped"));
    assert_eq!(first_detail.id, first_root.children[0]);
    assert_eq!(first_detail.parent.as_ref(), Some(&first_root.id));
    assert_eq!(first_detail.name.as_deref(), Some("detail"));
    assert_eq!(
        first_detail.text.as_ref().and_then(SemanticText::as_plain),
        Some("initial detail")
    );

    let owner = first
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "adapter.owner")
        })
        .unwrap_or_else(|| unreachable!("adapter owner renderer node is published"))
        .id()
        .clone();
    runtime
        .submit_resolved_surface_command(
            first.input_context().clone(),
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary RequestFocus command is admitted"));
    assert!(runtime.pump(full_budget()).is_quiescent());
    let focused = publish_adapter(&mut runtime);
    let focused_semantics = focused.semantic_publication();
    let focus_delta = AdapterUpdate::read(expect_delta(
        focused_semantics,
        first_snapshot.surface_id(),
        first_snapshot.revision(),
    ));
    assert_eq!(&focus_delta.surface, first_snapshot.surface_id());
    assert_eq!(focus_delta.previous_revision.get(), 1);
    assert_eq!(focus_delta.revision.get(), 2);
    assert!(focus_delta.added.is_empty());
    assert!(focus_delta.changed.is_empty());
    assert!(focus_delta.removed.is_empty());
    assert!(focus_delta.roots.is_none());
    let focus_change = focus_delta
        .focus
        .as_ref()
        .unwrap_or_else(|| unreachable!("runtime focus change is published in the delta"));
    assert!(focus_change.previous.is_none());
    assert_eq!(
        focus_change.current.as_ref(),
        focused_semantics.snapshot().focused()
    );

    let focused_revision = focused_semantics.snapshot().revision();
    set_adapter_phase(&mut runtime, AdapterPhase::Changed);
    let changed = publish_adapter(&mut runtime);
    let changed_semantics = changed.semantic_publication();
    let changed_delta = AdapterUpdate::read(expect_delta(
        changed_semantics,
        changed_semantics.snapshot().surface_id(),
        focused_revision,
    ));
    assert_eq!(
        &changed_delta.surface,
        changed_semantics.snapshot().surface_id()
    );
    assert_eq!(changed_delta.previous_revision, focused_revision);
    assert_eq!(changed_delta.added.len(), 1);
    assert_eq!(changed_delta.changed.len(), 2);
    assert!(changed_delta.removed.is_empty());
    assert!(changed_delta.roots.is_none());
    assert!(changed_delta.focus.is_none());
    let changed_root = changed_delta
        .changed
        .iter()
        .find(|node| node.name.as_deref() == Some("adapter root changed"))
        .unwrap_or_else(|| unreachable!("changed adapter root is present in delta"));
    assert!(changed_root.inert);
    assert_eq!(changed_root.role, SemanticRole::Button);
    assert_eq!(changed_root.relationships.len(), 1);
    assert_eq!(
        changed_root.relationships[0].kind,
        SemanticRelationshipKind::DescribedBy
    );
    assert!((changed_root.bounds.width() - 12.0).abs() <= f32::EPSILON);

    let changed_revision = changed_semantics.snapshot().revision();
    let changed_surface = changed_semantics.snapshot().surface_id().clone();
    set_adapter_phase(&mut runtime, AdapterPhase::Final);
    let final_publication = publish_adapter(&mut runtime);
    let final_semantics = final_publication.semantic_publication();
    let final_delta = AdapterUpdate::read(expect_delta(
        final_semantics,
        &changed_surface,
        changed_revision,
    ));
    assert_eq!(&final_delta.surface, &changed_surface);
    assert_eq!(final_delta.previous_revision, changed_revision);
    assert_eq!(final_delta.revision, final_semantics.snapshot().revision());
    assert_eq!(final_delta.removed.len(), 1);
    assert!(final_delta.added.is_empty());
    assert_eq!(final_delta.changed.len(), 2);
    assert!(final_delta.roots.is_none());
    assert!(final_delta.focus.is_none());

    match final_semantics.update_from(&changed_surface, first_snapshot.revision()) {
        SemanticUpdateResult::FullResync(snapshot) => {
            assert_eq!(snapshot.revision(), final_semantics.snapshot().revision());
        }
        SemanticUpdateResult::Unchanged | SemanticUpdateResult::Delta(_) => {
            unreachable!("skipped adapter revision requires full resynchronization")
        }
    }

    let mut foreign_runtime = AppRuntime::<AdapterApp>::mount(AdapterPhase::Initial);
    let foreign = publish_adapter(&mut foreign_runtime);
    match final_semantics.update_from(
        foreign.semantic_publication().snapshot().surface_id(),
        changed_revision,
    ) {
        SemanticUpdateResult::FullResync(snapshot) => {
            assert_eq!(snapshot.surface_id(), &changed_surface);
        }
        SemanticUpdateResult::Unchanged | SemanticUpdateResult::Delta(_) => {
            unreachable!("wrong surface requires full resynchronization")
        }
    }

    let unchanged = publish_adapter(&mut runtime);
    assert_eq!(
        unchanged.semantic_publication().snapshot().revision(),
        final_semantics.snapshot().revision()
    );
    assert!(matches!(
        unchanged.semantic_publication().update_from(
            unchanged.semantic_publication().snapshot().surface_id(),
            unchanged.semantic_publication().snapshot().revision(),
        ),
        SemanticUpdateResult::Unchanged
    ));
}

#[test]
fn public_semantic_product_and_complete_aggregate_are_explicitly_separate_from_renderer_products() {
    fn consume(snapshot: &SemanticSnapshot) -> (SurfaceId, SemanticRevision, usize) {
        (
            snapshot.surface_id().clone(),
            snapshot.revision(),
            snapshot.nodes().len(),
        )
    }

    let mut runtime = AppRuntime::<AdapterApp>::mount(AdapterPhase::Initial);
    let publication = publish_adapter(&mut runtime);
    let (surface, revision, nodes) = consume(publication.semantic_publication().snapshot());
    assert_eq!(
        &surface,
        publication.semantic_publication().snapshot().surface_id()
    );
    assert_eq!(revision.get(), 1);
    assert_eq!(nodes, 2);

    let renderer_only = publication.clone().into_renderer_products();
    assert_eq!(renderer_only.0.nodes().len(), 1);
    let complete = publication.into_complete_products();
    assert_eq!(complete.4.snapshot().surface_id(), &surface);
    assert_eq!(complete.5.surface_id(), &surface);
}
