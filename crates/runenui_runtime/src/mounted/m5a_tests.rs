use runenui_core::{
    Element, SemanticContribution, SemanticContributionContext, SemanticContributionError,
    SemanticItem, SemanticKey, SemanticNodeContribution, SemanticRole, View, Widget,
    WidgetInvalidation, WidgetUpdateContext, column, text,
};

use super::{
    CachedSemanticContribution, MountedNodeId, MountedTree, SemanticNodeId,
    semantic::{SemanticBinding, SemanticTargetStatus},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContributionMode {
    PrimaryOnly,
    Ordered,
    Reversed,
    Invalid,
}

#[derive(Debug)]
struct SemanticProbe {
    mode: ContributionMode,
}

impl Widget<()> for SemanticProbe {
    type State = ContributionMode;

    fn create_state(&self) -> Self::State {
        self.mode
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<()>) {
        if *state != self.mode {
            *state = self.mode;
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let primary = SemanticNodeContribution::primary(SemanticRole::Group);
        let extra = SemanticNodeContribution::new(extra_key(), SemanticRole::Text);
        match state {
            ContributionMode::PrimaryOnly => SemanticContribution::single(primary),
            ContributionMode::Ordered => SemanticContribution::new(vec![
                SemanticItem::node(primary),
                SemanticItem::node(extra),
            ]),
            ContributionMode::Reversed => SemanticContribution::new(vec![
                SemanticItem::node(extra),
                SemanticItem::node(primary),
            ]),
            ContributionMode::Invalid => SemanticContribution::new(vec![
                SemanticItem::node(primary),
                SemanticItem::node(SemanticNodeContribution::primary(SemanticRole::Text)),
            ]),
        }
    }
}

fn extra_key() -> SemanticKey {
    SemanticKey::from_static("extra").unwrap_or_else(|_| unreachable!("test key is valid"))
}

fn probe(mode: ContributionMode, key: &'static str) -> Element<()> {
    Element::new(SemanticProbe { mode }).key(key)
}

fn root_id(tree: &MountedTree<()>) -> MountedNodeId {
    tree.root
        .clone()
        .unwrap_or_else(|| unreachable!("mounted test tree has a root"))
}

fn bindings(tree: &MountedTree<()>, owner: &MountedNodeId) -> Vec<SemanticBinding> {
    tree.node(owner)
        .unwrap_or_else(|| unreachable!("semantic owner remains mounted"))
        .semantic_bindings
        .clone()
}

fn binding_id(bindings: &[SemanticBinding], key: &SemanticKey) -> SemanticNodeId {
    bindings
        .iter()
        .find(|binding| binding.key() == key)
        .unwrap_or_else(|| unreachable!("expected semantic key is bound"))
        .id()
        .clone()
}

#[test]
fn compatible_update_reorders_keys_without_reissuing_semantic_ids() {
    let (mut tree, _) = MountedTree::mount(probe(ContributionMode::Ordered, "probe"));
    let owner = root_id(&tree);
    tree.ensure_semantics_capability(&owner);
    let first = bindings(&tree, &owner);
    let primary_id = binding_id(&first, &SemanticKey::PRIMARY);
    let extra = extra_key();
    let extra_id = binding_id(&first, &extra);

    tree.reconcile(probe(ContributionMode::Reversed, "probe"));
    let retained_owner = root_id(&tree);
    assert_eq!(retained_owner, owner);
    tree.ensure_semantics_capability(&owner);
    let reordered = bindings(&tree, &owner);

    assert_eq!(reordered[0].key(), &extra);
    assert_eq!(reordered[1].key(), &SemanticKey::PRIMARY);
    assert_eq!(binding_id(&reordered, &SemanticKey::PRIMARY), primary_id);
    assert_eq!(binding_id(&reordered, &extra), extra_id);
    assert_eq!(
        tree.semantic_store
            .target_status(&tree.runtime, &primary_id),
        SemanticTargetStatus::Live
    );
    assert_eq!(
        tree.semantic_store.target_status(&tree.runtime, &extra_id),
        SemanticTargetStatus::Live
    );
}

#[test]
fn invalid_contribution_revokes_semantics_without_replacing_owner_and_can_recover() {
    let (mut tree, _) = MountedTree::mount(probe(ContributionMode::Ordered, "probe"));
    let owner = root_id(&tree);
    tree.ensure_semantics_capability(&owner);
    let old_ids = bindings(&tree, &owner)
        .into_iter()
        .map(|binding| binding.id().clone())
        .collect::<Vec<_>>();

    tree.reconcile(probe(ContributionMode::Invalid, "probe"));
    assert_eq!(root_id(&tree), owner);
    tree.ensure_semantics_capability(&owner);
    let invalid = tree
        .node(&owner)
        .unwrap_or_else(|| unreachable!("invalid semantic owner remains mounted"));
    assert!(!invalid.integrity_failed);
    assert!(invalid.semantic_bindings.is_empty());
    assert!(matches!(
        &invalid.caches.semantics,
        CachedSemanticContribution::Invalid(SemanticContributionError::DuplicateKey { .. })
    ));
    for id in &old_ids {
        assert_eq!(
            tree.semantic_store.target_status(&tree.runtime, id),
            SemanticTargetStatus::Stale
        );
    }

    tree.reconcile(probe(ContributionMode::Ordered, "probe"));
    assert_eq!(root_id(&tree), owner);
    tree.ensure_semantics_capability(&owner);
    let recovered = tree
        .node(&owner)
        .unwrap_or_else(|| unreachable!("recovered semantic owner remains mounted"));
    assert!(!recovered.integrity_failed);
    assert_eq!(recovered.semantic_bindings.len(), 2);
    assert!(matches!(
        recovered.caches.semantics,
        CachedSemanticContribution::Ready(_)
    ));
    for binding in &recovered.semantic_bindings {
        assert_eq!(
            tree.semantic_store
                .target_status(&tree.runtime, binding.id()),
            SemanticTargetStatus::Live
        );
        assert!(!old_ids.iter().any(|old| old == binding.id()));
    }
}

#[test]
fn capacity_failure_withdraws_complete_owner_semantics_and_recovers_cleanly() {
    let (mut tree, _) = MountedTree::mount(probe(ContributionMode::PrimaryOnly, "probe"));
    let owner = root_id(&tree);
    tree.ensure_semantics_capability_with_public_slot_limit_for_test(&owner, 1);
    let old = binding_id(&bindings(&tree, &owner), &SemanticKey::PRIMARY);
    assert_eq!(tree.semantic_store.live_count(), 1);

    tree.reconcile(probe(ContributionMode::Ordered, "probe"));
    tree.ensure_semantics_capability_with_public_slot_limit_for_test(&owner, 1);
    let exhausted = tree
        .node(&owner)
        .unwrap_or_else(|| unreachable!("capacity-rejected semantic owner remains mounted"));
    assert!(!exhausted.integrity_failed);
    assert!(exhausted.semantic_bindings.is_empty());
    assert!(matches!(
        exhausted.caches.semantics,
        CachedSemanticContribution::IdentityExhausted
    ));
    assert_eq!(tree.semantic_store.live_count(), 0);
    assert_eq!(
        tree.semantic_store.target_status(&tree.runtime, &old),
        SemanticTargetStatus::Stale
    );

    tree.reconcile(probe(ContributionMode::PrimaryOnly, "probe"));
    tree.ensure_semantics_capability_with_public_slot_limit_for_test(&owner, 1);
    let recovered = tree
        .node(&owner)
        .unwrap_or_else(|| unreachable!("capacity-recovered semantic owner remains mounted"));
    assert!(!recovered.integrity_failed);
    assert_eq!(recovered.semantic_bindings.len(), 1);
    assert!(matches!(
        recovered.caches.semantics,
        CachedSemanticContribution::Ready(_)
    ));
    let replacement = recovered.semantic_bindings[0].id().clone();
    assert_ne!(replacement, old);
    assert_eq!(
        tree.semantic_store
            .target_status(&tree.runtime, &replacement),
        SemanticTargetStatus::Live
    );
}

#[test]
fn semantic_index_corruption_withdraws_owner_and_marks_integrity_failure() {
    let (mut tree, _) = MountedTree::mount(probe(ContributionMode::PrimaryOnly, "probe"));
    let owner = root_id(&tree);
    tree.ensure_semantics_capability(&owner);
    let old = binding_id(&bindings(&tree, &owner), &SemanticKey::PRIMARY);

    {
        let node = tree
            .node_mut(&owner)
            .unwrap_or_else(|| unreachable!("semantic owner remains mounted"));
        let duplicate = node.semantic_bindings[0].clone();
        node.semantic_bindings.push(duplicate);
        node.caches.semantics = CachedSemanticContribution::Unresolved;
    }

    tree.ensure_semantics_capability(&owner);
    let failed = tree
        .node(&owner)
        .unwrap_or_else(|| unreachable!("integrity-failed semantic owner remains mounted"));
    assert!(failed.integrity_failed);
    assert!(failed.semantic_bindings.is_empty());
    assert!(matches!(
        failed.caches.semantics,
        CachedSemanticContribution::IndexIntegrityFailure
    ));
    assert_eq!(tree.semantic_store.live_count(), 0);
    assert_eq!(
        tree.semantic_store.target_status(&tree.runtime, &old),
        SemanticTargetStatus::Stale
    );
}

fn column_tree(with_child: bool) -> Element<()> {
    let children = if with_child {
        vec![text("child").key("child").into_element()]
    } else {
        Vec::new()
    };
    column(children).key("root").into_element()
}

#[test]
fn mounted_child_count_change_recomputes_contribution_and_retains_surviving_key_identity() {
    let (mut tree, _) = MountedTree::mount(column_tree(true));
    let owner = root_id(&tree);
    tree.ensure_semantics_capability(&owner);
    let primary_id = binding_id(&bindings(&tree, &owner), &SemanticKey::PRIMARY);

    tree.reconcile(column_tree(false));
    assert_eq!(root_id(&tree), owner);
    assert!(matches!(
        tree.node(&owner)
            .unwrap_or_else(|| unreachable!("retained container remains mounted"))
            .caches
            .semantics,
        CachedSemanticContribution::Unresolved
    ));

    tree.ensure_semantics_capability(&owner);
    let refreshed = tree
        .node(&owner)
        .unwrap_or_else(|| unreachable!("retained container remains mounted"));
    assert!(matches!(
        refreshed.caches.semantics,
        CachedSemanticContribution::Ready(_)
    ));
    assert_eq!(refreshed.semantic_bindings.len(), 1);
    assert_eq!(refreshed.semantic_bindings[0].key(), &SemanticKey::PRIMARY);
    assert_eq!(refreshed.semantic_bindings[0].id(), &primary_id);
}

#[test]
fn mounted_owner_replacement_revokes_every_old_semantic_lifetime() {
    let (mut tree, _) = MountedTree::mount(probe(ContributionMode::Ordered, "first"));
    let old_owner = root_id(&tree);
    tree.ensure_semantics_capability(&old_owner);
    let old_ids = bindings(&tree, &old_owner)
        .into_iter()
        .map(|binding| binding.id().clone())
        .collect::<Vec<_>>();

    tree.reconcile(probe(ContributionMode::Ordered, "replacement"));
    let replacement = root_id(&tree);
    assert_ne!(replacement, old_owner);
    for id in &old_ids {
        assert_eq!(
            tree.semantic_store.target_status(&tree.runtime, id),
            SemanticTargetStatus::Stale
        );
    }

    tree.ensure_semantics_capability(&replacement);
    let replacement_bindings = bindings(&tree, &replacement);
    assert_eq!(replacement_bindings.len(), 2);
    for binding in &replacement_bindings {
        assert_eq!(
            tree.semantic_store
                .target_status(&tree.runtime, binding.id()),
            SemanticTargetStatus::Live
        );
        assert!(!old_ids.iter().any(|old| old == binding.id()));
    }
}
