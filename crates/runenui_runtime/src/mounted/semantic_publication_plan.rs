use runenui_core::{
    SemanticContribution, SemanticContributionContext, SemanticKey, WidgetActivation,
};

use super::{
    CachedCapability, CachedSemanticContribution, MountedNodeId, MountedTree,
    node::state_is_corrupted,
};

#[derive(Clone, Debug)]
pub(super) struct StagedSemanticOwnerCapabilities {
    pub(super) owner: MountedNodeId,
    pub(super) contribution: SemanticContribution,
    pub(super) ordered_keys: Vec<SemanticKey>,
    pub(super) activation: WidgetActivation,
    pub(super) semantic_cache: CachedSemanticContribution,
    pub(super) activation_cache: CachedCapability<WidgetActivation>,
    pub(super) mark_integrity_failed: bool,
}

#[derive(Clone, Debug)]
pub(super) struct SemanticCapabilityPlan {
    pub(super) owners: Vec<StagedSemanticOwnerCapabilities>,
}

impl<Action> MountedTree<Action> {
    /// Evaluates semantic-publication capabilities without mutating mounted
    /// caches, semantic bindings, semantic identity storage, or integrity state.
    ///
    /// This is the read-only capability stage used by the future atomic surface
    /// publication transaction. A later identity/finalization stage decides
    /// whether these staged facts can commit.
    pub(super) fn plan_semantic_publication_capabilities(
        &self,
        owners: &[MountedNodeId],
    ) -> SemanticCapabilityPlan {
        SemanticCapabilityPlan {
            owners: owners
                .iter()
                .filter_map(|owner| self.stage_semantic_owner_capabilities(owner))
                .collect(),
        }
    }

    fn stage_semantic_owner_capabilities(
        &self,
        owner: &MountedNodeId,
    ) -> Option<StagedSemanticOwnerCapabilities> {
        let node = self.node(owner)?;
        if state_is_corrupted(node) {
            return Some(StagedSemanticOwnerCapabilities {
                owner: owner.clone(),
                contribution: SemanticContribution::empty(),
                ordered_keys: Vec::new(),
                activation: WidgetActivation::disabled(),
                semantic_cache: CachedSemanticContribution::StatePayloadMismatch,
                activation_cache: CachedCapability::StatePayloadMismatch,
                mark_integrity_failed: true,
            });
        }

        let direct_mounted_children = node.children.len();
        let context = SemanticContributionContext::__runtime_new(direct_mounted_children);
        let (contribution, ordered_keys, semantic_cache, mut mark_integrity_failed) =
            match &node.caches.semantics {
                CachedSemanticContribution::Ready(contribution) => {
                    match contribution.validate(context) {
                        Ok(validation) => (
                            contribution.clone(),
                            validation.ordered_keys().to_vec(),
                            CachedSemanticContribution::Ready(contribution.clone()),
                            false,
                        ),
                        Err(error) => (
                            SemanticContribution::empty(),
                            Vec::new(),
                            CachedSemanticContribution::Invalid(error),
                            false,
                        ),
                    }
                }
                CachedSemanticContribution::Unresolved => {
                    match node.widget.semantics(&node.state, context) {
                        Ok(contribution) => match contribution.validate(context) {
                            Ok(validation) => (
                                contribution.clone(),
                                validation.ordered_keys().to_vec(),
                                CachedSemanticContribution::Ready(contribution),
                                false,
                            ),
                            Err(error) => (
                                SemanticContribution::empty(),
                                Vec::new(),
                                CachedSemanticContribution::Invalid(error),
                                false,
                            ),
                        },
                        Err(_) => (
                            SemanticContribution::empty(),
                            Vec::new(),
                            CachedSemanticContribution::StatePayloadMismatch,
                            true,
                        ),
                    }
                }
                CachedSemanticContribution::Invalid(error) => (
                    SemanticContribution::empty(),
                    Vec::new(),
                    CachedSemanticContribution::Invalid(error.clone()),
                    false,
                ),
                CachedSemanticContribution::IdentityExhausted => (
                    SemanticContribution::empty(),
                    Vec::new(),
                    CachedSemanticContribution::IdentityExhausted,
                    false,
                ),
                CachedSemanticContribution::IndexIntegrityFailure => (
                    SemanticContribution::empty(),
                    Vec::new(),
                    CachedSemanticContribution::IndexIntegrityFailure,
                    true,
                ),
                CachedSemanticContribution::StatePayloadMismatch => (
                    SemanticContribution::empty(),
                    Vec::new(),
                    CachedSemanticContribution::StatePayloadMismatch,
                    true,
                ),
            };

        let (activation, activation_cache) = match node.caches.activation {
            CachedCapability::Ready(value) => (value, CachedCapability::Ready(value)),
            CachedCapability::Unresolved => match node.widget.activation(&node.state) {
                Ok(value) => (value, CachedCapability::Ready(value)),
                Err(_) => {
                    mark_integrity_failed = true;
                    (WidgetActivation::disabled(), CachedCapability::StatePayloadMismatch)
                }
            },
            CachedCapability::StatePayloadMismatch => {
                mark_integrity_failed = true;
                (WidgetActivation::disabled(), CachedCapability::StatePayloadMismatch)
            }
        };

        if mark_integrity_failed {
            return Some(StagedSemanticOwnerCapabilities {
                owner: owner.clone(),
                contribution: SemanticContribution::empty(),
                ordered_keys: Vec::new(),
                activation,
                semantic_cache: CachedSemanticContribution::StatePayloadMismatch,
                activation_cache,
                mark_integrity_failed,
            });
        }

        Some(StagedSemanticOwnerCapabilities {
            owner: owner.clone(),
            contribution,
            ordered_keys,
            activation,
            semantic_cache,
            activation_cache,
            mark_integrity_failed,
        })
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};

    use runenui_core::{
        Element, SemanticContribution, SemanticContributionContext, SemanticItem,
        SemanticNodeContribution, SemanticRole, Widget, WidgetActivation,
    };

    use super::*;

    static SEMANTIC_CALLBACKS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug)]
    struct Probe {
        invalid: bool,
    }

    impl Widget<()> for Probe {
        type State = bool;

        fn create_state(&self) -> Self::State {
            self.invalid
        }

        fn activation(&self, _: &Self::State) -> WidgetActivation {
            WidgetActivation::actionable()
        }

        fn semantics(
            &self,
            state: &Self::State,
            _: SemanticContributionContext,
        ) -> SemanticContribution {
            SEMANTIC_CALLBACKS.fetch_add(1, Ordering::SeqCst);
            let primary = SemanticNodeContribution::primary(SemanticRole::Group);
            if *state {
                SemanticContribution::new(vec![
                    SemanticItem::node(primary),
                    SemanticItem::node(SemanticNodeContribution::primary(SemanticRole::Text)),
                ])
            } else {
                SemanticContribution::single(primary)
            }
        }
    }

    fn root_id(tree: &MountedTree<()>) -> MountedNodeId {
        tree.root
            .clone()
            .unwrap_or_else(|| unreachable!("mounted test tree has a root"))
    }

    #[test]
    fn unresolved_capabilities_are_staged_without_mutating_live_authority() {
        SEMANTIC_CALLBACKS.store(0, Ordering::SeqCst);
        let (tree, _) = MountedTree::mount(Element::new(Probe { invalid: false }));
        let root = root_id(&tree);
        let plan = tree.plan_semantic_publication_capabilities(core::slice::from_ref(&root));
        let staged = &plan.owners[0];

        assert_eq!(SEMANTIC_CALLBACKS.load(Ordering::SeqCst), 1);
        assert_eq!(staged.owner, root);
        assert_eq!(staged.ordered_keys, vec![SemanticKey::PRIMARY]);
        assert_eq!(staged.activation, WidgetActivation::actionable());
        assert!(matches!(
            staged.semantic_cache,
            CachedSemanticContribution::Ready(_)
        ));
        assert!(matches!(
            staged.activation_cache,
            CachedCapability::Ready(value) if value == WidgetActivation::actionable()
        ));
        assert!(!staged.mark_integrity_failed);

        let live = tree
            .node(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"));
        assert!(matches!(
            live.caches.semantics,
            CachedSemanticContribution::Unresolved
        ));
        assert!(matches!(live.caches.activation, CachedCapability::Unresolved));
        assert!(live.semantic_bindings.is_empty());
        assert_eq!(tree.semantic_store.live_count(), 0);
    }

    #[test]
    fn valid_cached_semantics_are_reused_without_callback_reentry() {
        SEMANTIC_CALLBACKS.store(0, Ordering::SeqCst);
        let (mut tree, _) = MountedTree::mount(Element::new(Probe { invalid: false }));
        let root = root_id(&tree);
        tree.ensure_semantics_capability(&root);
        assert_eq!(SEMANTIC_CALLBACKS.load(Ordering::SeqCst), 1);
        let bindings_before = tree
            .node(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"))
            .semantic_bindings
            .clone();
        let live_count_before = tree.semantic_store.live_count();

        let plan = tree.plan_semantic_publication_capabilities(core::slice::from_ref(&root));
        assert_eq!(SEMANTIC_CALLBACKS.load(Ordering::SeqCst), 1);
        assert_eq!(plan.owners[0].ordered_keys, vec![SemanticKey::PRIMARY]);
        assert_eq!(
            tree.node(&root)
                .unwrap_or_else(|| unreachable!("root remains mounted"))
                .semantic_bindings,
            bindings_before
        );
        assert_eq!(tree.semantic_store.live_count(), live_count_before);
    }

    #[test]
    fn invalid_authoring_stages_complete_owner_withdrawal_without_live_revocation() {
        SEMANTIC_CALLBACKS.store(0, Ordering::SeqCst);
        let (tree, _) = MountedTree::mount(Element::new(Probe { invalid: true }));
        let root = root_id(&tree);
        let plan = tree.plan_semantic_publication_capabilities(core::slice::from_ref(&root));
        let staged = &plan.owners[0];

        assert!(staged.contribution.is_empty());
        assert!(staged.ordered_keys.is_empty());
        assert!(matches!(
            staged.semantic_cache,
            CachedSemanticContribution::Invalid(_)
        ));
        assert!(!staged.mark_integrity_failed);
        assert!(
            tree.node(&root)
                .unwrap_or_else(|| unreachable!("root remains mounted"))
                .semantic_bindings
                .is_empty()
        );
        assert_eq!(tree.semantic_store.live_count(), 0);
    }

    #[test]
    fn corrupted_state_stages_fail_closed_withdrawal_without_marking_live_node() {
        let (mut tree, _) = MountedTree::mount(Element::new(Probe { invalid: false }));
        let root = root_id(&tree);
        tree.node_mut(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"))
            .state_corrupted = true;

        let plan = tree.plan_semantic_publication_capabilities(core::slice::from_ref(&root));
        let staged = &plan.owners[0];
        assert!(staged.contribution.is_empty());
        assert!(staged.ordered_keys.is_empty());
        assert!(matches!(
            staged.semantic_cache,
            CachedSemanticContribution::StatePayloadMismatch
        ));
        assert!(matches!(
            staged.activation_cache,
            CachedCapability::StatePayloadMismatch
        ));
        assert!(staged.mark_integrity_failed);
        assert!(
            !tree
                .node(&root)
                .unwrap_or_else(|| unreachable!("root remains mounted"))
                .integrity_failed
        );
        assert_eq!(tree.semantic_store.live_count(), 0);
    }
}
