use runenui_core::__runtime::RuntimeNamespace;
use runenui_core::{
    Focusability, SemanticContribution, SemanticContributionContext, SemanticKey, SemanticNodeId,
    WidgetActivation,
};

use super::{
    CachedCapability, CachedSemanticContribution, MountedNodeId, MountedTree,
    node::{MountedNode, state_is_corrupted},
    semantic::{SemanticBinding, SemanticOwnerPlan, SemanticReconcileError, SemanticStorePlan},
};

const PUBLIC_SEMANTIC_SLOT_LIMIT: u64 = 1_u64 << u32::BITS;

#[derive(Clone, Debug)]
pub(crate) struct StagedSemanticOwnerCapabilities {
    owner: MountedNodeId,
    contribution: SemanticContribution,
    ordered_keys: Vec<SemanticKey>,
    current_bindings: Vec<SemanticBinding>,
    semantic_cache: CachedSemanticContribution,
    activation_cache: CachedCapability<WidgetActivation>,
    focusability: Focusability,
    mark_integrity_failed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct SemanticCapabilityPlan {
    owners: Vec<StagedSemanticOwnerCapabilities>,
}

pub(crate) struct FinalizedSemanticPublication<'a> {
    store_plan: SemanticStorePlan<'a>,
    owners: Vec<FinalizedSemanticOwner>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FinalizedSemanticOwnerFacts {
    pub(crate) owner: MountedNodeId,
    pub(crate) contribution: SemanticContribution,
    pub(crate) bindings: Vec<(SemanticKey, SemanticNodeId)>,
    pub(crate) activation: WidgetActivation,
    pub(crate) focusability: Focusability,
}

pub(crate) struct SemanticMountedCommit {
    owners: Vec<FinalizedSemanticOwner>,
}

struct FinalizedSemanticOwner {
    owner: MountedNodeId,
    contribution: SemanticContribution,
    bindings: Vec<SemanticBinding>,
    semantic_cache: CachedSemanticContribution,
    activation_cache: CachedCapability<WidgetActivation>,
    focusability: Focusability,
    mark_integrity_failed: bool,
}

#[derive(Clone, Copy)]
enum ForcedWithdrawal {
    IdentityExhausted,
    IndexIntegrityFailure,
}

struct StagedSemanticCapability {
    contribution: SemanticContribution,
    ordered_keys: Vec<SemanticKey>,
    cache: CachedSemanticContribution,
    integrity_failed: bool,
}

struct StagedActivationCapability {
    cache: CachedCapability<WidgetActivation>,
    integrity_failed: bool,
}

impl FinalizedSemanticPublication<'_> {
    pub(crate) fn owner_facts(
        &self,
    ) -> impl ExactSizeIterator<Item = FinalizedSemanticOwnerFacts> + '_ {
        self.owners.iter().map(|owner| FinalizedSemanticOwnerFacts {
            owner: owner.owner.clone(),
            contribution: owner.contribution.clone(),
            bindings: owner
                .bindings
                .iter()
                .map(|binding| (binding.key().clone(), binding.id().clone()))
                .collect(),
            activation: owner.activation_cache.ready().unwrap_or_default(),
            focusability: owner.focusability,
        })
    }

    pub(crate) fn commit_store(self) -> SemanticMountedCommit {
        let Self { store_plan, owners } = self;
        store_plan.commit();
        SemanticMountedCommit { owners }
    }
}

impl StagedSemanticCapability {
    fn ready(contribution: SemanticContribution, context: SemanticContributionContext) -> Self {
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

    const fn withdrawn(cache: CachedSemanticContribution, integrity_failed: bool) -> Self {
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
    pub(crate) fn plan_semantic_publication_capabilities(&self) -> SemanticCapabilityPlan {
        SemanticCapabilityPlan {
            owners: self
                .publication_preorder_ids()
                .into_iter()
                .map(|owner| self.stage_semantic_owner_capabilities(&owner))
                .collect(),
        }
    }

    fn stage_semantic_owner_capabilities(
        &self,
        owner: &MountedNodeId,
    ) -> StagedSemanticOwnerCapabilities {
        let node = self
            .node(owner)
            .unwrap_or_else(|| unreachable!("semantic publication owner remains live"));
        if state_is_corrupted(node) {
            return integrity_withdrawal(owner, node.semantic_bindings.clone(), node.focusability);
        }

        let semantic = stage_semantic_capability(node);
        let activation = stage_activation_capability(node);
        let mark_integrity_failed = semantic.integrity_failed || activation.integrity_failed;
        if mark_integrity_failed {
            return StagedSemanticOwnerCapabilities {
                owner: owner.clone(),
                contribution: SemanticContribution::empty(),
                ordered_keys: Vec::new(),
                current_bindings: node.semantic_bindings.clone(),
                semantic_cache: CachedSemanticContribution::StatePayloadMismatch,
                activation_cache: activation.cache,
                focusability: node.focusability,
                mark_integrity_failed,
            };
        }

        StagedSemanticOwnerCapabilities {
            owner: owner.clone(),
            contribution: semantic.contribution,
            ordered_keys: semantic.ordered_keys,
            current_bindings: node.semantic_bindings.clone(),
            semantic_cache: semantic.cache,
            activation_cache: activation.cache,
            focusability: node.focusability,
            mark_integrity_failed,
        }
    }

    /// Finalizes semantic identity transitions publication-wide without mutating
    /// the live semantic store or mounted owner records.
    ///
    /// Fail-closed owner withdrawals are first classified in disposable,
    /// borrow-scoped transaction attempts. A failed attempt is dropped in full
    /// before the exact owner is retried as a staged purge, so no tentative
    /// transaction-local removal survives a newly discovered withdrawal. Once
    /// classification is stable, one fresh borrow-scoped transaction produces
    /// the returned store plan and keeps the live store protected until commit.
    pub(crate) fn finalize_semantic_publication(
        &mut self,
        capability_plan: SemanticCapabilityPlan,
    ) -> Result<FinalizedSemanticPublication<'_>, SemanticReconcileError> {
        let runtime = self.runtime.clone();
        let forced = self.classify_semantic_withdrawals(&runtime, &capability_plan)?;
        let mut transaction = self.semantic_store.transaction();
        let mut owner_plans = Vec::with_capacity(capability_plan.owners.len());

        for (index, staged) in capability_plan.owners.iter().enumerate() {
            let ready = forced[index].is_none()
                && matches!(staged.semantic_cache, CachedSemanticContribution::Ready(_));
            let owner_plan = if ready {
                transaction.stage_owner(
                    &runtime,
                    &staged.owner,
                    &staged.current_bindings,
                    &staged.ordered_keys,
                    PUBLIC_SEMANTIC_SLOT_LIMIT,
                )?
            } else {
                transaction.stage_owner_purge(&staged.owner, PUBLIC_SEMANTIC_SLOT_LIMIT)?
            };
            owner_plans.push(owner_plan);
        }

        let store_plan = transaction.finalize_fail_closed(&runtime)?;
        let owners = capability_plan
            .owners
            .into_iter()
            .zip(owner_plans)
            .enumerate()
            .map(|(index, (staged, owner_plan))| {
                finalize_owner(&store_plan, owner_plan, staged, forced[index])
            })
            .collect();
        Ok(FinalizedSemanticPublication { store_plan, owners })
    }

    fn classify_semantic_withdrawals(
        &mut self,
        runtime: &RuntimeNamespace,
        capability_plan: &SemanticCapabilityPlan,
    ) -> Result<Vec<Option<ForcedWithdrawal>>, SemanticReconcileError> {
        let mut forced = vec![None; capability_plan.owners.len()];

        loop {
            let mut transaction = self.semantic_store.transaction();
            let mut restart = false;

            for (index, staged) in capability_plan.owners.iter().enumerate() {
                let ready = forced[index].is_none()
                    && matches!(staged.semantic_cache, CachedSemanticContribution::Ready(_));
                let result = if ready {
                    transaction.stage_owner(
                        runtime,
                        &staged.owner,
                        &staged.current_bindings,
                        &staged.ordered_keys,
                        PUBLIC_SEMANTIC_SLOT_LIMIT,
                    )
                } else {
                    transaction.stage_owner_purge(&staged.owner, PUBLIC_SEMANTIC_SLOT_LIMIT)
                };

                match result {
                    Ok(_) => {}
                    Err(SemanticReconcileError::Integrity(_)) if ready => {
                        forced[index] = Some(ForcedWithdrawal::IndexIntegrityFailure);
                        restart = true;
                        break;
                    }
                    Err(SemanticReconcileError::IdentityExhausted) if ready => {
                        forced[index] = Some(ForcedWithdrawal::IdentityExhausted);
                        restart = true;
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }

            if !restart {
                return Ok(forced);
            }
        }
    }

    pub(crate) fn commit_semantic_publication(&mut self, commit: SemanticMountedCommit) {
        for finalized in commit.owners {
            let node = self
                .node_mut(&finalized.owner)
                .unwrap_or_else(|| unreachable!("finalized semantic owner remains live"));
            node.semantic_bindings = finalized.bindings;
            node.caches.semantics = finalized.semantic_cache;
            node.caches.activation = finalized.activation_cache;
            node.integrity_failed |= finalized.mark_integrity_failed;
        }
    }
}

fn finalize_owner(
    store_plan: &SemanticStorePlan<'_>,
    owner_plan: SemanticOwnerPlan,
    staged: StagedSemanticOwnerCapabilities,
    forced: Option<ForcedWithdrawal>,
) -> FinalizedSemanticOwner {
    let bindings = store_plan.bindings(owner_plan).to_vec();
    if store_plan.identity_exhausted(owner_plan) {
        return FinalizedSemanticOwner {
            owner: staged.owner,
            contribution: SemanticContribution::empty(),
            bindings,
            semantic_cache: CachedSemanticContribution::IdentityExhausted,
            activation_cache: staged.activation_cache,
            focusability: staged.focusability,
            mark_integrity_failed: staged.mark_integrity_failed,
        };
    }

    match forced {
        Some(ForcedWithdrawal::IdentityExhausted) => FinalizedSemanticOwner {
            owner: staged.owner,
            contribution: SemanticContribution::empty(),
            bindings,
            semantic_cache: CachedSemanticContribution::IdentityExhausted,
            activation_cache: staged.activation_cache,
            focusability: staged.focusability,
            mark_integrity_failed: staged.mark_integrity_failed,
        },
        Some(ForcedWithdrawal::IndexIntegrityFailure) => FinalizedSemanticOwner {
            owner: staged.owner,
            contribution: SemanticContribution::empty(),
            bindings,
            semantic_cache: CachedSemanticContribution::IndexIntegrityFailure,
            activation_cache: staged.activation_cache,
            focusability: staged.focusability,
            mark_integrity_failed: true,
        },
        None => FinalizedSemanticOwner {
            owner: staged.owner,
            contribution: staged.contribution,
            bindings,
            semantic_cache: staged.semantic_cache,
            activation_cache: staged.activation_cache,
            focusability: staged.focusability,
            mark_integrity_failed: staged.mark_integrity_failed,
        },
    }
}

fn integrity_withdrawal(
    owner: &MountedNodeId,
    current_bindings: Vec<SemanticBinding>,
    focusability: Focusability,
) -> StagedSemanticOwnerCapabilities {
    StagedSemanticOwnerCapabilities {
        owner: owner.clone(),
        contribution: SemanticContribution::empty(),
        ordered_keys: Vec::new(),
        current_bindings,
        semantic_cache: CachedSemanticContribution::StatePayloadMismatch,
        activation_cache: CachedCapability::StatePayloadMismatch,
        focusability,
        mark_integrity_failed: true,
    }
}

fn stage_semantic_capability<Action>(node: &MountedNode<Action>) -> StagedSemanticCapability {
    let context = SemanticContributionContext::__runtime_new(node.children.len());
    match &node.caches.semantics {
        CachedSemanticContribution::Ready(contribution) => {
            StagedSemanticCapability::ready(contribution.clone(), context)
        }
        CachedSemanticContribution::Unresolved => {
            node.widget.semantics(&node.state, context).map_or_else(
                |_| {
                    StagedSemanticCapability::withdrawn(
                        CachedSemanticContribution::StatePayloadMismatch,
                        true,
                    )
                },
                |contribution| StagedSemanticCapability::ready(contribution, context),
            )
        }
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
            cache: CachedCapability::Ready(*value),
            integrity_failed: false,
        },
        CachedCapability::Unresolved => node.widget.activation(&node.state).map_or_else(
            |_| StagedActivationCapability {
                cache: CachedCapability::StatePayloadMismatch,
                integrity_failed: true,
            },
            |value| StagedActivationCapability {
                cache: CachedCapability::Ready(value),
                integrity_failed: false,
            },
        ),
        CachedCapability::StatePayloadMismatch => StagedActivationCapability {
            cache: CachedCapability::StatePayloadMismatch,
            integrity_failed: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use runenui_core::{
        Element, SemanticContribution, SemanticContributionContext, SemanticItem,
        SemanticNodeContribution, SemanticRole, Widget, WidgetActivation,
    };

    use super::*;

    #[derive(Debug)]
    struct Probe {
        invalid: bool,
        semantic_callbacks: Arc<AtomicUsize>,
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
            self.semantic_callbacks.fetch_add(1, Ordering::SeqCst);
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

    fn probe(invalid: bool) -> (Probe, Arc<AtomicUsize>) {
        let semantic_callbacks = Arc::new(AtomicUsize::new(0));
        (
            Probe {
                invalid,
                semantic_callbacks: Arc::clone(&semantic_callbacks),
            },
            semantic_callbacks,
        )
    }

    fn root_id(tree: &MountedTree<()>) -> MountedNodeId {
        tree.root
            .clone()
            .unwrap_or_else(|| unreachable!("mounted test tree has a root"))
    }

    #[test]
    fn unresolved_capabilities_are_staged_without_mutating_live_authority() {
        let (probe, semantic_callbacks) = probe(false);
        let (tree, _) = MountedTree::mount(Element::new(probe));
        let root = root_id(&tree);
        let plan = tree.plan_semantic_publication_capabilities();
        let staged = &plan.owners[0];

        assert_eq!(semantic_callbacks.load(Ordering::SeqCst), 1);
        assert_eq!(staged.owner, root);
        assert_eq!(staged.ordered_keys, vec![SemanticKey::PRIMARY]);
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
        let (probe, semantic_callbacks) = probe(false);
        let (mut tree, _) = MountedTree::mount(Element::new(probe));
        let root = root_id(&tree);
        tree.ensure_semantics_capability(&root);
        assert_eq!(semantic_callbacks.load(Ordering::SeqCst), 1);
        let bindings_before = tree
            .node(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"))
            .semantic_bindings
            .clone();
        let live_count_before = tree.semantic_store.live_count();

        let plan = tree.plan_semantic_publication_capabilities();
        assert_eq!(semantic_callbacks.load(Ordering::SeqCst), 1);
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
    fn finalized_semantics_commit_store_before_mounted_owner_facts() {
        let (probe, _) = probe(false);
        let (mut tree, _) = MountedTree::mount(Element::new(probe));
        let root = root_id(&tree);
        let expected_focusability = tree
            .node(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"))
            .focusability;
        let plan = tree.plan_semantic_publication_capabilities();
        let finalized = tree
            .finalize_semantic_publication(plan)
            .unwrap_or_else(|_| unreachable!("valid semantic plan finalizes"));
        let owner_facts = finalized.owner_facts().collect::<Vec<_>>();
        assert_eq!(owner_facts.len(), 1);
        assert_eq!(owner_facts[0].owner, root);
        assert_eq!(owner_facts[0].bindings.len(), 1);
        assert_eq!(owner_facts[0].bindings[0].0, SemanticKey::PRIMARY);
        assert_eq!(
            owner_facts[0].activation,
            WidgetActivation::actionable(true)
        );
        assert_eq!(owner_facts[0].focusability, expected_focusability);
        let mounted_commit = finalized.commit_store();

        assert_eq!(tree.semantic_store.live_count(), 1);
        assert!(
            tree.node(&root)
                .unwrap_or_else(|| unreachable!("root remains mounted"))
                .semantic_bindings
                .is_empty()
        );

        tree.commit_semantic_publication(mounted_commit);
        let live = tree
            .node(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"));
        assert_eq!(live.semantic_bindings.len(), 1);
        assert!(matches!(
            live.caches.semantics,
            CachedSemanticContribution::Ready(_)
        ));
        assert!(matches!(
            live.caches.activation,
            CachedCapability::Ready(value) if value == WidgetActivation::actionable(true)
        ));
    }

    #[test]
    fn invalid_authoring_stages_complete_owner_withdrawal_without_live_revocation() {
        let (probe, _) = probe(true);
        let (tree, _) = MountedTree::mount(Element::new(probe));
        let root = root_id(&tree);
        let plan = tree.plan_semantic_publication_capabilities();
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
        let (probe, _) = probe(false);
        let (mut tree, _) = MountedTree::mount(Element::new(probe));
        let root = root_id(&tree);
        tree.node_mut(&root)
            .unwrap_or_else(|| unreachable!("root remains mounted"))
            .state_corrupted = true;

        let plan = tree.plan_semantic_publication_capabilities();
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
