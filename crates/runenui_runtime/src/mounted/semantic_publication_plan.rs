use runenui_core::{
    SemanticContribution, SemanticContributionContext, SemanticKey, WidgetActivation,
};

use super::{
    CachedCapability, CachedSemanticContribution, MountedNodeId, MountedTree,
    node::{MountedNode, state_is_corrupted},
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

struct StagedSemanticCapability {
    contribution: SemanticContribution,
    ordered_keys: Vec<SemanticKey>,
    cache: CachedSemanticContribution,
    integrity_failed: bool,
}

struct StagedActivationCapability {
    activation: WidgetActivation,
    cache: CachedCapability<WidgetActivation>,
    integrity_failed: bool,
}

impl StagedSemanticCapability {
    fn ready(
        contribution: SemanticContribution,
        context: SemanticContributionContext,
    ) -> Self {
        contribution.validate(context).map_or_else(
            |error| Self {
                contribution: SemanticContribution::empty(),
                ordered_keys: Vec::new(),
                cache: CachedSemanticContribution::Invalid(error),
                integrity_failed: false,
            },
            |validation| Self {
                ordered_keys: validation.ordered_keys().to_vec(),
                cache: CachedSemanticContribution::Ready(contribution.clone()),
                contribution,
                integrity_failed: false,
            },
        )
    }

    fn withdrawn(cache: CachedSemanticContribution, integrity_failed: bool) -> Self {
        Self {
            contribution: SemanticContribution::empty(),
            ordered_keys: Vec::new(),
            cache,
            integrity_failed,
        }
    }
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
            return Some(integrity_withdrawal(owner));
        }

        let semantic = stage_semantic_capability(node);
        let activation = stage_activation_capability(node);
        let mark_integrity_failed = semantic.integrity_failed || activation.integrity_failed;
        if mark_integrity_failed {
            return Some(StagedSemanticOwnerCapabilities {
                owner: owner.clone(),
                contribution: SemanticContribution::empty(),
                ordered_keys: Vec::new(),
                activation: activation.activation,
                semantic_cache: CachedSemanticContribution::StatePayloadMismatch,
                activation_cache: activation.cache,
                mark_integrity_failed,
            });
        }

        Some(StagedSemanticOwnerCapabilities {
            owner: owner.clone(),
            contribution: semantic.contribution,
            ordered_keys: semantic.ordered_keys,
            activation: activation.activation,
            semantic_cache: semantic.cache,
            activation_cache: activation.cache,
            mark_integrity_failed,
        })
    }
}

fn integrity_withdrawal(owner: &MountedNodeId) -> StagedSemanticOwnerCapabilities {
    StagedSemanticOwnerCapabilities {
        owner: owner.clone(),
        contribution: SemanticContribution::empty(),
        ordered_keys: Vec::new(),
        activation: WidgetActivation::disabled(),
        semantic_cache: CachedSemanticContribution::StatePayloadMismatch,
        activation_cache: CachedCapability::StatePayloadMismatch,
        mark_integrity_failed: true,
    }
}

fn stage_semantic_capability<Action>(node: &MountedNode<Action>) -> StagedSemanticCapability {
    let context = SemanticContributionContext::__runtime_new(node.children.len());
    match &node.caches.semantics {
        CachedSemanticContribution::Ready(contribution) => {
            StagedSemanticCapability::ready(contribution.clone(), context)
        }
        CachedSemanticContribution::Unresolved => node
            .widget
            .semantics(&node.state, context)
            .map_or_else(
                |_| {
                    StagedSemanticCapability::withdrawn(
                        CachedSemanticContribution::StatePayloadMismatch,
                        true,
                    )
                },
                |contribution| StagedSemanticCapability::ready(contribution, context),
            ),
        CachedSemanticContribution::Invalid(error) => StagedSemanticCapability::withdrawn(
            CachedSemanticContribution::Invalid(error.clone()),
            false,
        ),
        CachedSemanticContribution::IdentityExhausted => StagedSemanticCapability::withdrawn(
            CachedSemanticContribution::IdentityExhausted,
            false,
        ),
        CachedSemanticContribution::IndexIntegrityFailure => StagedSemanticCapability::withdrawn(
            CachedSemanticContribution::IndexIntegrityFailure,
            true,
        ),
        CachedSemanticContribution::StatePayloadMismatch => StagedSemanticCapability::withdrawn(
            CachedSemanticContribution::StatePayloadMismatch,
            true,
        ),
    }
}

fn stage_activation_capability<Action>(node: &MountedNode<Action>) -> StagedActivationCapability {
    match &node.caches.activation {
        CachedCapability::Ready(value) => StagedActivationCapability {
            activation: *value,
            cache: CachedCapability::Ready(*value),
            integrity_failed: false,
        },
        CachedCapability::Unresolved => node.widget.activation(&node.state).map_or_else(
            |_| StagedActivationCapability {
                activation: WidgetActivation::disabled(),
                cache: CachedCapability::StatePayloadMismatch,
                integrity_failed: true,
            },
            |value| StagedActivationCapability {
                activation: value,
                cache: CachedCapability::Ready(value),
                integrity_failed: false,
            },
        ),
        CachedCapability::StatePayloadMismatch => StagedActivationCapability {
            activation: WidgetActivation::disabled(),
            cache: CachedCapability::StatePayloadMismatch,
            integrity_failed: true,
        },
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
            WidgetActivation::actionable(true)
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
        assert_eq!(staged.activation, WidgetActivation::actionable(true));
        assert!(matches!(
            staged.semantic_cache,
            CachedSemanticContribution::Ready(_)
        ));
        assert!(matches!(
            staged.activation_cache,
            CachedCapability::Ready(value) if value == WidgetActivation::actionable(true)
        ));
        assert!(!staged.mark_integrity_failed);

        let live = tree
            .node(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"));
        assert!(matches!(
            live.caches.semantics,
            CachedSemanticContribution::Unresolved
        ));
        assert!(matches!(
            live.caches.activation,
            CachedCapability::Unresolved
        ));
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

        assert!(staged.contribution.roots().is_empty());
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
        assert!(staged.contribution.roots().is_empty());
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
