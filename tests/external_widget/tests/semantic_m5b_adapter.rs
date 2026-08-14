use runenui_core::{
    CommandOrigin, Element, IntoEffects, LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol,
    SemanticAction, SemanticBounds, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticKey, SemanticNodeContribution, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState, SemanticText,
    SemanticValue, StyleTokens, UiApp, View, Widget, WidgetActivation,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SemanticNode, SemanticNodeId, SemanticPublication,
    SemanticRevision, SemanticSnapshot, SemanticUpdate, SemanticUpdateResult, SurfaceBuildContext,
    SurfaceId, SurfacePublication,
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

fn publish_adapter(runtime: &mut AppRuntime<AdapterApp>) -> SurfacePublication {
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
    transition: &'static str,
) -> &'a SemanticUpdate {
    match publication.update_from(surface, revision) {
        SemanticUpdateResult::Delta(update) => update,
        SemanticUpdateResult::Unchanged => {
            unreachable!("{transition} semantic publication unexpectedly remained unchanged")
        }
        SemanticUpdateResult::FullResync(snapshot) => unreachable!(
            "{transition} semantic publication required full resync from revision {} to {}",
            revision.get(),
            snapshot.revision().get()
        ),
    }
}

fn assert_initial_adapter_snapshot(first: &SurfacePublication) -> (SurfaceId, SemanticRevision) {
    let semantics = first.semantic_publication();
    let snapshot = semantics.snapshot();
    let adapted = AdapterSnapshot::read(snapshot);
    assert_eq!(adapted.revision.get(), 1);
    assert_eq!(&adapted.surface, snapshot.surface_id());
    assert_eq!(adapted.roots.as_slice(), snapshot.roots());
    assert_eq!(adapted.nodes.len(), 2);
    assert!(adapted.focused.is_none());
    assert!(semantics.update().is_none());

    let root = adapted
        .nodes
        .first()
        .unwrap_or_else(|| unreachable!("adapter root is mapped"));
    assert_eq!(root.parent, None);
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.role, SemanticRole::Group);
    assert_eq!(root.name.as_deref(), Some("adapter root"));
    assert_eq!(root.description.as_deref(), Some("initial"));
    assert_eq!(root.value, Some(SemanticValue::Integer(1)));
    assert!(!root.disabled);
    assert!(!root.inert);
    assert_eq!(root.supported_actions.len(), 4);
    assert_eq!(root.relationships.len(), 1);
    assert_eq!(
        root.relationships[0].kind,
        SemanticRelationshipKind::LabelledBy
    );
    assert_eq!(
        &root.relationships[0].target,
        root.children
            .first()
            .unwrap_or_else(|| unreachable!("label target is the mapped child"))
    );
    assert!((root.bounds.width() - 10.0).abs() <= f32::EPSILON);
    assert!(root.text.is_none());

    let detail = adapted
        .nodes
        .get(1)
        .unwrap_or_else(|| unreachable!("adapter detail is mapped"));
    assert_eq!(detail.id, root.children[0]);
    assert_eq!(detail.parent.as_ref(), Some(&root.id));
    assert_eq!(detail.name.as_deref(), Some("detail"));
    assert_eq!(
        detail.text.as_ref().and_then(SemanticText::as_plain),
        Some("initial detail")
    );
    (adapted.surface, adapted.revision)
}

fn commit_focus_and_assert_delta(
    runtime: &mut AppRuntime<AdapterApp>,
    first: &SurfacePublication,
    surface: &SurfaceId,
    revision: SemanticRevision,
) -> SemanticRevision {
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
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary RequestFocus command is admitted"));
    assert!(runtime.pump(full_budget()).is_quiescent());
    assert_eq!(runtime.focus().focused_node(), Some(&owner));

    let focused = publish_adapter(runtime);
    let semantics = focused.semantic_publication();
    let update = AdapterUpdate::read(expect_delta(semantics, surface, revision, "focus"));
    assert_eq!(&update.surface, surface);
    assert_eq!(update.previous_revision, revision);
    assert!(update.added.is_empty());
    assert!(update.changed.is_empty());
    assert!(update.removed.is_empty());
    assert!(update.roots.is_none());
    let focus = update
        .focus
        .as_ref()
        .unwrap_or_else(|| unreachable!("runtime focus change is published in the delta"));
    assert!(focus.previous.is_none());
    assert_eq!(focus.current.as_ref(), semantics.snapshot().focused());
    update.revision
}

fn change_adapter_and_assert_delta(
    runtime: &mut AppRuntime<AdapterApp>,
    previous_revision: SemanticRevision,
) -> (SurfaceId, SemanticRevision) {
    set_adapter_phase(runtime, AdapterPhase::Changed);
    let changed = publish_adapter(runtime);
    let semantics = changed.semantic_publication();
    let surface = semantics.snapshot().surface_id().clone();
    let update = AdapterUpdate::read(expect_delta(
        semantics,
        &surface,
        previous_revision,
        "content change",
    ));
    assert_eq!(&update.surface, &surface);
    assert_eq!(update.previous_revision, previous_revision);
    assert_eq!(update.added.len(), 1);
    assert_eq!(update.changed.len(), 2);
    assert!(update.removed.is_empty());
    assert!(update.roots.is_none());
    assert!(update.focus.is_none());
    let root = update
        .changed
        .iter()
        .find(|node| node.name.as_deref() == Some("adapter root changed"))
        .unwrap_or_else(|| unreachable!("changed adapter root is present in delta"));
    assert!(root.inert);
    assert_eq!(root.role, SemanticRole::Button);
    assert_eq!(root.relationships.len(), 1);
    assert_eq!(
        root.relationships[0].kind,
        SemanticRelationshipKind::DescribedBy
    );
    assert!((root.bounds.width() - 12.0).abs() <= f32::EPSILON);
    (surface, update.revision)
}

fn finalize_adapter_and_assert_delta(
    runtime: &mut AppRuntime<AdapterApp>,
    surface: &SurfaceId,
    previous_revision: SemanticRevision,
) -> SemanticPublication {
    set_adapter_phase(runtime, AdapterPhase::Final);
    let final_publication = publish_adapter(runtime);
    let semantics = final_publication.semantic_publication().clone();
    let update = AdapterUpdate::read(expect_delta(
        &semantics,
        surface,
        previous_revision,
        "final removal",
    ));
    assert_eq!(&update.surface, surface);
    assert_eq!(update.previous_revision, previous_revision);
    assert_eq!(update.revision, semantics.snapshot().revision());
    assert_eq!(update.removed.len(), 1);
    assert!(update.added.is_empty());
    assert_eq!(update.changed.len(), 2);
    assert!(update.roots.is_none());
    assert!(update.focus.is_none());
    semantics
}

fn assert_resync_and_noop(
    runtime: &mut AppRuntime<AdapterApp>,
    final_semantics: &SemanticPublication,
    surface: &SurfaceId,
    first_revision: SemanticRevision,
    previous_revision: SemanticRevision,
) {
    match final_semantics.update_from(surface, first_revision) {
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
        previous_revision,
    ) {
        SemanticUpdateResult::FullResync(snapshot) => assert_eq!(snapshot.surface_id(), surface),
        SemanticUpdateResult::Unchanged | SemanticUpdateResult::Delta(_) => {
            unreachable!("wrong surface requires full resynchronization")
        }
    }

    let unchanged = publish_adapter(runtime);
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
fn independent_adapter_shaped_consumer_reads_snapshot_delta_focus_and_resync_contract() {
    let mut runtime = AppRuntime::<AdapterApp>::mount(AdapterPhase::Initial);
    let first = publish_adapter(&mut runtime);
    let (surface, first_revision) = assert_initial_adapter_snapshot(&first);
    let focused_revision =
        commit_focus_and_assert_delta(&mut runtime, &first, &surface, first_revision);
    let (changed_surface, changed_revision) =
        change_adapter_and_assert_delta(&mut runtime, focused_revision);
    let final_semantics =
        finalize_adapter_and_assert_delta(&mut runtime, &changed_surface, changed_revision);
    assert_resync_and_noop(
        &mut runtime,
        &final_semantics,
        &changed_surface,
        first_revision,
        changed_revision,
    );
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
