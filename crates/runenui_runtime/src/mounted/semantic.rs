use std::collections::{BTreeMap, BTreeSet};

use runenui_core::__runtime::RuntimeNamespace;
use runenui_core::{MountedNodeId, SemanticKey, SemanticNodeId};

use super::arena::{ArenaCapacityError, ArenaPlanStateError, ArenaPlanner, GenerationalArena};

#[derive(Clone, Debug)]
pub(super) struct SemanticRecord {
    owner: MountedNodeId,
    key: SemanticKey,
}

impl SemanticRecord {
    pub(super) const fn owner(&self) -> &MountedNodeId {
        &self.owner
    }

    pub(super) const fn key(&self) -> &SemanticKey {
        &self.key
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SemanticBinding {
    key: SemanticKey,
    id: SemanticNodeId,
}

impl SemanticBinding {
    pub(super) const fn key(&self) -> &SemanticKey {
        &self.key
    }

    pub(super) const fn id(&self) -> &SemanticNodeId {
        &self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SemanticStoreIntegrityError {
    DuplicateCurrentKey(SemanticKey),
    DuplicateRequestedKey(SemanticKey),
    ForeignBinding,
    MissingBindingRecord,
    BindingRecordMismatch,
    PlanningStateMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SemanticReconcileError {
    IdentityExhausted,
    Integrity(SemanticStoreIntegrityError),
}

impl From<ArenaCapacityError> for SemanticReconcileError {
    fn from(_: ArenaCapacityError) -> Self {
        Self::IdentityExhausted
    }
}

impl From<ArenaPlanStateError> for SemanticReconcileError {
    fn from(_: ArenaPlanStateError) -> Self {
        Self::Integrity(SemanticStoreIntegrityError::PlanningStateMismatch)
    }
}

impl From<SemanticStoreIntegrityError> for SemanticReconcileError {
    fn from(error: SemanticStoreIntegrityError) -> Self {
        Self::Integrity(error)
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SemanticTargetStatus {
    Live,
    Stale,
    Missing,
    Foreign,
}

pub(super) struct SemanticStore {
    arena: GenerationalArena<SemanticRecord>,
}

impl SemanticStore {
    pub(super) const fn new() -> Self {
        Self {
            arena: GenerationalArena::new(),
        }
    }

    #[cfg(test)]
    pub(super) const fn live_count(&self) -> usize {
        self.arena.live_count()
    }

    #[cfg(test)]
    pub(super) fn target_status(
        &self,
        runtime: &RuntimeNamespace,
        id: &SemanticNodeId,
    ) -> SemanticTargetStatus {
        let Some((slot, generation)) = runtime.__runtime_semantic_parts(id) else {
            return SemanticTargetStatus::Foreign;
        };
        let slot = slot as usize;
        if self.arena.get(slot, generation).is_some() {
            SemanticTargetStatus::Live
        } else if self.arena.contains_slot(slot) {
            SemanticTargetStatus::Stale
        } else {
            SemanticTargetStatus::Missing
        }
    }

    pub(super) fn transaction(&mut self) -> SemanticStoreTransaction<'_> {
        let planner = self.arena.planner();
        SemanticStoreTransaction {
            store: self,
            planner,
            removals: Vec::new(),
            owners: Vec::new(),
        }
    }

    /// Reconciles one owner through the same non-mutating planning path used by
    /// publication-wide semantic transactions.
    ///
    /// This compatibility-internal helper preserves M5A behavior while M5B moves
    /// capability evaluation and commit ownership to the surface publication
    /// transaction. Planning itself never revokes or allocates a live semantic ID.
    pub(super) fn reconcile_owner(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        current: &[SemanticBinding],
        ordered_keys: &[SemanticKey],
        public_slot_limit: u64,
    ) -> Result<Vec<SemanticBinding>, SemanticReconcileError> {
        let mut transaction = self.transaction();
        let owner_plan =
            transaction.stage_owner(runtime, owner, current, ordered_keys, public_slot_limit)?;
        let plan = transaction.finalize(runtime)?;
        let bindings = plan.bindings(owner_plan).to_vec();
        plan.commit();
        Ok(bindings)
    }

    pub(super) fn revoke_owner(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        bindings: &[SemanticBinding],
    ) -> Result<(), SemanticStoreIntegrityError> {
        match self.validate_current(runtime, owner, bindings) {
            Ok(validated) => {
                for binding in validated.values() {
                    self.remove_validated(binding);
                }
                Ok(())
            }
            Err(error) => {
                self.purge_owner_records(owner);
                Err(error)
            }
        }
    }

    fn validate_current(
        &self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        current: &[SemanticBinding],
    ) -> Result<BTreeMap<SemanticKey, ValidatedBinding>, SemanticStoreIntegrityError> {
        let mut existing = BTreeMap::new();
        for binding in current {
            if existing.contains_key(binding.key()) {
                return Err(SemanticStoreIntegrityError::DuplicateCurrentKey(
                    binding.key().clone(),
                ));
            }
            let Some((slot, generation)) = runtime.__runtime_semantic_parts(binding.id()) else {
                return Err(SemanticStoreIntegrityError::ForeignBinding);
            };
            let slot = slot as usize;
            let Some(record) = self.arena.get(slot, generation) else {
                return Err(SemanticStoreIntegrityError::MissingBindingRecord);
            };
            if record.owner() != owner || record.key() != binding.key() {
                return Err(SemanticStoreIntegrityError::BindingRecordMismatch);
            }
            existing.insert(
                binding.key().clone(),
                ValidatedBinding {
                    binding: binding.clone(),
                    slot,
                    generation,
                },
            );
        }
        Ok(existing)
    }

    fn remove_validated(&mut self, binding: &ValidatedBinding) {
        let removed = self.arena.remove(binding.slot, binding.generation);
        debug_assert!(
            removed.is_some(),
            "validated semantic binding remains live until commit"
        );
    }

    fn purge_owner_records(&mut self, owner: &MountedNodeId) {
        let records = self
            .arena
            .live_indices_where(|record| record.owner() == owner);
        for (slot, generation) in records {
            let _ = self.arena.remove(slot, generation);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SemanticOwnerPlan(usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SemanticFinalizeFailure {
    owner: SemanticOwnerPlan,
    error: SemanticReconcileError,
}

impl SemanticFinalizeFailure {
    #[cfg(test)]
    pub(super) const fn owner(&self) -> SemanticOwnerPlan {
        self.owner
    }

    #[cfg(test)]
    pub(super) const fn error(&self) -> &SemanticReconcileError {
        &self.error
    }

    fn into_error(self) -> SemanticReconcileError {
        self.error
    }
}

/// Borrow-scoped semantic identity planning transaction for one publication.
///
/// Owner staging validates live bindings and records every removal in the virtual
/// arena, but performs no allocation. [`Self::finalize`] therefore sees all
/// publication-wide vacancies before assigning any new semantic IDs. The live
/// [`SemanticStore`] stays untouched until the resulting [`SemanticStorePlan`] is
/// committed.
pub(super) struct SemanticStoreTransaction<'a> {
    store: &'a mut SemanticStore,
    planner: ArenaPlanner,
    removals: Vec<SemanticRemoval>,
    owners: Vec<StagedSemanticOwner>,
}

impl<'a> SemanticStoreTransaction<'a> {
    pub(super) fn stage_owner(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        current: &[SemanticBinding],
        ordered_keys: &[SemanticKey],
        public_slot_limit: u64,
    ) -> Result<SemanticOwnerPlan, SemanticReconcileError> {
        let mut requested = BTreeSet::new();
        for key in ordered_keys {
            if !requested.insert(key.clone()) {
                return Err(SemanticReconcileError::Integrity(
                    SemanticStoreIntegrityError::DuplicateRequestedKey(key.clone()),
                ));
            }
        }

        let mut existing = self.store.validate_current(runtime, owner, current)?;
        let mut entries = Vec::with_capacity(ordered_keys.len());
        for key in ordered_keys {
            if let Some(binding) = existing.remove(key) {
                entries.push(StagedSemanticEntry::Existing(binding.binding));
            } else {
                entries.push(StagedSemanticEntry::New(key.clone()));
            }
        }

        for binding in existing.into_values() {
            self.planner.remove(binding.slot, binding.generation)?;
            self.removals.push(SemanticRemoval {
                slot: binding.slot,
                generation: binding.generation,
            });
        }

        let index = self.owners.len();
        self.owners.push(StagedSemanticOwner {
            owner: owner.clone(),
            entries,
            public_slot_limit,
        });
        Ok(SemanticOwnerPlan(index))
    }

    pub(super) fn finalize(
        self,
        runtime: &RuntimeNamespace,
    ) -> Result<SemanticStorePlan<'a>, SemanticReconcileError> {
        self.finalize_attributed(runtime)
            .map_err(SemanticFinalizeFailure::into_error)
    }

    pub(super) fn finalize_attributed(
        mut self,
        runtime: &RuntimeNamespace,
    ) -> Result<SemanticStorePlan<'a>, SemanticFinalizeFailure> {
        let mut inserts = Vec::new();
        let mut owner_bindings = Vec::with_capacity(self.owners.len());
        for (owner_index, staged) in self.owners.into_iter().enumerate() {
            let owner_plan = SemanticOwnerPlan(owner_index);
            let mut bindings = Vec::with_capacity(staged.entries.len());
            for entry in staged.entries {
                match entry {
                    StagedSemanticEntry::Existing(binding) => bindings.push(binding),
                    StagedSemanticEntry::New(key) => {
                        let (slot, generation) = self
                            .planner
                            .allocate(staged.public_slot_limit)
                            .map_err(|error| SemanticFinalizeFailure {
                                owner: owner_plan,
                                error: error.into(),
                            })?;
                        let public_slot =
                            u32::try_from(slot).map_err(|_| SemanticFinalizeFailure {
                                owner: owner_plan,
                                error: SemanticReconcileError::IdentityExhausted,
                            })?;
                        let binding = SemanticBinding {
                            key: key.clone(),
                            id: runtime.__runtime_semantic_id(public_slot, generation),
                        };
                        inserts.push(SemanticInsert {
                            slot,
                            generation,
                            public_slot_limit: staged.public_slot_limit,
                            record: SemanticRecord {
                                owner: staged.owner.clone(),
                                key,
                            },
                        });
                        bindings.push(binding);
                    }
                }
            }
            owner_bindings.push(bindings);
        }
        Ok(SemanticStorePlan {
            store: self.store,
            removals: self.removals,
            inserts,
            owner_bindings,
        })
    }
}

/// Fully preflighted semantic identity transition. Dropping this value performs
/// no live mutation; [`Self::commit`] is the sole mutation boundary.
pub(super) struct SemanticStorePlan<'a> {
    store: &'a mut SemanticStore,
    removals: Vec<SemanticRemoval>,
    inserts: Vec<SemanticInsert>,
    owner_bindings: Vec<Vec<SemanticBinding>>,
}

impl SemanticStorePlan<'_> {
    pub(super) fn bindings(&self, owner: SemanticOwnerPlan) -> &[SemanticBinding] {
        self.owner_bindings.get(owner.0).map_or_else(
            || unreachable!("semantic owner plan belongs to this transaction"),
            Vec::as_slice,
        )
    }

    pub(super) fn commit(self) {
        for removal in self.removals {
            let removed = self.store.arena.remove(removal.slot, removal.generation);
            assert!(
                removed.is_some(),
                "planned semantic removal must match the borrow-protected live arena"
            );
        }
        for insert in self.inserts {
            let actual = self
                .store
                .arena
                .insert_with_public_slot_limit(insert.public_slot_limit, move |_, _| insert.record);
            assert_eq!(
                actual,
                Ok((insert.slot, insert.generation)),
                "planned semantic allocation must match the borrow-protected live arena"
            );
        }
    }
}

struct ValidatedBinding {
    binding: SemanticBinding,
    slot: usize,
    generation: u64,
}

struct StagedSemanticOwner {
    owner: MountedNodeId,
    entries: Vec<StagedSemanticEntry>,
    public_slot_limit: u64,
}

enum StagedSemanticEntry {
    Existing(SemanticBinding),
    New(SemanticKey),
}

struct SemanticRemoval {
    slot: usize,
    generation: u64,
}

struct SemanticInsert {
    slot: usize,
    generation: u64,
    public_slot_limit: u64,
    record: SemanticRecord,
}

#[cfg(test)]
mod tests {
    use runenui_core::__runtime::RuntimeNamespace;
    use runenui_core::SemanticKey;

    use super::{
        SemanticReconcileError, SemanticStore, SemanticStoreIntegrityError, SemanticTargetStatus,
    };

    fn key(value: &'static str) -> SemanticKey {
        SemanticKey::from_static(value).unwrap_or_else(|_| unreachable!("test key is valid"))
    }

    #[test]
    fn owner_local_keys_receive_independent_stable_ids_across_reorder() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(7, 1);
        let mut store = SemanticStore::new();
        let a = key("a");
        let b = key("b");
        let first = store
            .reconcile_owner(
                &runtime,
                &owner,
                &[],
                &[SemanticKey::PRIMARY, a.clone(), b.clone()],
                8,
            )
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(first.len(), 3);
        assert_ne!(first[0].id(), first[1].id());
        assert_ne!(first[1].id(), first[2].id());
        let primary = first[0].id().clone();
        let a_id = first[1].id().clone();
        let b_id = first[2].id().clone();

        let reordered = store
            .reconcile_owner(&runtime, &owner, &first, &[b, SemanticKey::PRIMARY, a], 8)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(reordered[0].id(), &b_id);
        assert_eq!(reordered[1].id(), &primary);
        assert_eq!(reordered[2].id(), &a_id);
        assert_eq!(store.live_count(), 3);
    }

    #[test]
    fn removed_key_becomes_stale_and_reused_slot_gets_later_generation() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(1, 1);
        let mut store = SemanticStore::new();
        let extra = key("extra");
        let first = store
            .reconcile_owner(&runtime, &owner, &[], &[SemanticKey::PRIMARY, extra], 2)
            .unwrap_or_else(|_| unreachable!());
        let removed = first[1].id().clone();
        let retained = store
            .reconcile_owner(&runtime, &owner, &first, &[SemanticKey::PRIMARY], 2)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            store.target_status(&runtime, &removed),
            SemanticTargetStatus::Stale
        );

        let replacement_key = key("replacement");
        let replacement = store
            .reconcile_owner(
                &runtime,
                &owner,
                &retained,
                &[SemanticKey::PRIMARY, replacement_key],
                2,
            )
            .unwrap_or_else(|_| unreachable!());
        let replacement_id = replacement[1].id();
        let removed_parts = runtime
            .__runtime_semantic_parts(&removed)
            .unwrap_or_else(|| unreachable!());
        let replacement_parts = runtime
            .__runtime_semantic_parts(replacement_id)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(removed_parts.0, replacement_parts.0);
        assert!(replacement_parts.1 > removed_parts.1);
        assert_eq!(
            store.target_status(&runtime, &removed),
            SemanticTargetStatus::Stale
        );
        assert_eq!(
            store.target_status(&runtime, replacement_id),
            SemanticTargetStatus::Live
        );
    }

    #[test]
    fn owner_revocation_stales_every_owned_semantic_lifetime() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(1, 1);
        let mut store = SemanticStore::new();
        let bindings = store
            .reconcile_owner(
                &runtime,
                &owner,
                &[],
                &[SemanticKey::PRIMARY, key("virtual")],
                4,
            )
            .unwrap_or_else(|_| unreachable!());
        let ids = bindings
            .iter()
            .map(|binding| binding.id().clone())
            .collect::<Vec<_>>();
        store
            .revoke_owner(&runtime, &owner, &bindings)
            .unwrap_or_else(|_| unreachable!("valid owner bindings revoke exactly"));
        assert_eq!(store.live_count(), 0);
        for id in ids {
            assert_eq!(
                store.target_status(&runtime, &id),
                SemanticTargetStatus::Stale
            );
        }
    }

    #[test]
    fn foreign_and_missing_ids_are_distinguished_without_retargeting() {
        let runtime = RuntimeNamespace::__runtime_new();
        let foreign_runtime = RuntimeNamespace::__runtime_new();
        let store = SemanticStore::new();
        let missing = runtime.__runtime_semantic_id(0, 1);
        let foreign = foreign_runtime.__runtime_semantic_id(0, 1);
        assert_eq!(
            store.target_status(&runtime, &missing),
            SemanticTargetStatus::Missing
        );
        assert_eq!(
            store.target_status(&runtime, &foreign),
            SemanticTargetStatus::Foreign
        );
    }

    #[test]
    fn semantic_capacity_failure_is_preflighted_without_partial_mutation() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(1, 1);
        let mut store = SemanticStore::new();
        assert_eq!(
            store.reconcile_owner(
                &runtime,
                &owner,
                &[],
                &[SemanticKey::PRIMARY, key("extra")],
                1,
            ),
            Err(SemanticReconcileError::IdentityExhausted)
        );
        assert_eq!(store.live_count(), 0);
    }

    #[test]
    fn duplicate_current_bindings_fail_closed_without_first_or_last_match() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(3, 1);
        let mut store = SemanticStore::new();
        let first = store
            .reconcile_owner(&runtime, &owner, &[], &[SemanticKey::PRIMARY], 2)
            .unwrap_or_else(|_| unreachable!());
        let duplicate = vec![first[0].clone(), first[0].clone()];
        assert_eq!(
            store.reconcile_owner(&runtime, &owner, &duplicate, &[SemanticKey::PRIMARY], 2,),
            Err(SemanticReconcileError::Integrity(
                SemanticStoreIntegrityError::DuplicateCurrentKey(SemanticKey::PRIMARY)
            ))
        );
        assert_eq!(store.live_count(), 1);
    }

    #[test]
    fn binding_for_a_different_owner_is_an_integrity_failure() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(1, 1);
        let other = runtime.__runtime_mounted_id(2, 1);
        let mut store = SemanticStore::new();
        let first = store
            .reconcile_owner(&runtime, &owner, &[], &[SemanticKey::PRIMARY], 2)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            store.reconcile_owner(&runtime, &other, &first, &[SemanticKey::PRIMARY], 2,),
            Err(SemanticReconcileError::Integrity(
                SemanticStoreIntegrityError::BindingRecordMismatch
            ))
        );
        assert_eq!(store.live_count(), 1);
    }

    #[test]
    fn publication_wide_transaction_reserves_distinct_ids_before_commit() {
        let runtime = RuntimeNamespace::__runtime_new();
        let first_owner = runtime.__runtime_mounted_id(1, 1);
        let second_owner = runtime.__runtime_mounted_id(2, 1);
        let mut store = SemanticStore::new();

        let (first, second) = {
            let mut transaction = store.transaction();
            let first_plan = transaction
                .stage_owner(&runtime, &first_owner, &[], &[SemanticKey::PRIMARY], 2)
                .unwrap_or_else(|_| unreachable!("first owner stages"));
            let second_plan = transaction
                .stage_owner(&runtime, &second_owner, &[], &[SemanticKey::PRIMARY], 2)
                .unwrap_or_else(|_| unreachable!("second owner stages"));
            let plan = transaction
                .finalize(&runtime)
                .unwrap_or_else(|_| unreachable!("both owners fit"));
            let first = plan.bindings(first_plan).to_vec();
            let second = plan.bindings(second_plan).to_vec();
            assert_ne!(first[0].id(), second[0].id());
            plan.commit();
            (first, second)
        };

        assert_eq!(store.live_count(), 2);
        assert_eq!(
            store.target_status(&runtime, first[0].id()),
            SemanticTargetStatus::Live
        );
        assert_eq!(
            store.target_status(&runtime, second[0].id()),
            SemanticTargetStatus::Live
        );
    }

    #[test]
    fn later_owner_release_is_visible_before_any_new_id_allocation() {
        let runtime = RuntimeNamespace::__runtime_new();
        let first_owner = runtime.__runtime_mounted_id(1, 1);
        let second_owner = runtime.__runtime_mounted_id(2, 1);
        let mut store = SemanticStore::new();
        let second_current = store
            .reconcile_owner(&runtime, &second_owner, &[], &[SemanticKey::PRIMARY], 1)
            .unwrap_or_else(|_| unreachable!("initial owner fits"));
        let old_id = second_current[0].id().clone();

        let first = {
            let mut transaction = store.transaction();
            let first_plan = transaction
                .stage_owner(&runtime, &first_owner, &[], &[SemanticKey::PRIMARY], 1)
                .unwrap_or_else(|_| unreachable!("new owner stages before later removal"));
            transaction
                .stage_owner(&runtime, &second_owner, &second_current, &[], 1)
                .unwrap_or_else(|_| unreachable!("old owner removal stages"));
            let plan = transaction
                .finalize(&runtime)
                .unwrap_or_else(|_| unreachable!("later vacancy satisfies earlier allocation"));
            let first = plan.bindings(first_plan).to_vec();
            plan.commit();
            first
        };

        let old_parts = runtime
            .__runtime_semantic_parts(&old_id)
            .unwrap_or_else(|| unreachable!("old id is local"));
        let new_parts = runtime
            .__runtime_semantic_parts(first[0].id())
            .unwrap_or_else(|| unreachable!("new id is local"));
        assert_eq!(old_parts.0, new_parts.0);
        assert!(new_parts.1 > old_parts.1);
        assert_eq!(store.live_count(), 1);
    }

    #[test]
    fn attributed_finalize_reports_exact_exhausted_owner_without_mutation() {
        let runtime = RuntimeNamespace::__runtime_new();
        let first_owner = runtime.__runtime_mounted_id(1, 1);
        let second_owner = runtime.__runtime_mounted_id(2, 1);
        let mut store = SemanticStore::new();

        let (first_plan, second_plan, failure) = {
            let mut transaction = store.transaction();
            let first_plan = transaction
                .stage_owner(&runtime, &first_owner, &[], &[SemanticKey::PRIMARY], 1)
                .unwrap_or_else(|_| unreachable!("first owner stages"));
            let second_plan = transaction
                .stage_owner(&runtime, &second_owner, &[], &[SemanticKey::PRIMARY], 1)
                .unwrap_or_else(|_| unreachable!("second owner stages"));
            let Err(failure) = transaction.finalize_attributed(&runtime) else {
                unreachable!("second owner must exceed the one-slot limit");
            };
            (first_plan, second_plan, failure)
        };

        assert_ne!(first_plan, second_plan);
        assert_eq!(failure.owner(), second_plan);
        assert_eq!(failure.error(), &SemanticReconcileError::IdentityExhausted);
        assert_eq!(store.live_count(), 0);
    }

    #[test]
    fn failed_finalize_drops_the_whole_transaction_without_mutation() {
        let runtime = RuntimeNamespace::__runtime_new();
        let first_owner = runtime.__runtime_mounted_id(1, 1);
        let second_owner = runtime.__runtime_mounted_id(2, 1);
        let mut store = SemanticStore::new();

        let failed = {
            let mut transaction = store.transaction();
            transaction
                .stage_owner(&runtime, &first_owner, &[], &[SemanticKey::PRIMARY], 1)
                .unwrap_or_else(|_| unreachable!("first owner stages"));
            transaction
                .stage_owner(&runtime, &second_owner, &[], &[SemanticKey::PRIMARY], 1)
                .unwrap_or_else(|_| unreachable!("second owner stages"));
            matches!(
                transaction.finalize(&runtime),
                Err(SemanticReconcileError::IdentityExhausted)
            )
        };
        assert!(failed);
        assert_eq!(store.live_count(), 0);
    }
}
