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
            operations: Vec::new(),
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
        let bindings =
            transaction.plan_owner(runtime, owner, current, ordered_keys, public_slot_limit)?;
        transaction.commit();
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

/// Borrow-scoped, publication-wide semantic identity transaction.
///
/// Planning mutates only [`ArenaPlanner`]. The live [`SemanticStore`] stays
/// untouched until [`Self::commit`], and the mutable borrow held for the whole
/// transaction prevents any intervening semantic-store mutation between plan and
/// commit.
pub(super) struct SemanticStoreTransaction<'a> {
    store: &'a mut SemanticStore,
    planner: ArenaPlanner,
    operations: Vec<SemanticStoreOperation>,
}

impl SemanticStoreTransaction<'_> {
    pub(super) fn plan_owner(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        current: &[SemanticBinding],
        ordered_keys: &[SemanticKey],
        public_slot_limit: u64,
    ) -> Result<Vec<SemanticBinding>, SemanticReconcileError> {
        let mut requested = BTreeSet::new();
        for key in ordered_keys {
            if !requested.insert(key.clone()) {
                return Err(SemanticReconcileError::Integrity(
                    SemanticStoreIntegrityError::DuplicateRequestedKey(key.clone()),
                ));
            }
        }

        let mut existing = self.store.validate_current(runtime, owner, current)?;
        let mut desired = Vec::with_capacity(ordered_keys.len());
        let mut entries = Vec::with_capacity(ordered_keys.len());
        for key in ordered_keys {
            if let Some(binding) = existing.remove(key) {
                desired.push(binding.binding.clone());
                entries.push(OwnerPlanEntry::Existing);
            } else {
                entries.push(OwnerPlanEntry::New(key.clone()));
            }
        }

        for binding in existing.into_values() {
            self.planner.remove(binding.slot, binding.generation)?;
            self.operations.push(SemanticStoreOperation::Remove {
                slot: binding.slot,
                generation: binding.generation,
            });
        }

        let existing_count = desired.len();
        let mut existing_index = 0usize;
        let mut final_bindings = Vec::with_capacity(entries.len());
        for entry in entries {
            match entry {
                OwnerPlanEntry::Existing => {
                    let binding = desired.get(existing_index).cloned().ok_or(
                        SemanticReconcileError::Integrity(
                            SemanticStoreIntegrityError::PlanningStateMismatch,
                        ),
                    )?;
                    existing_index = existing_index.checked_add(1).ok_or(
                        SemanticReconcileError::Integrity(
                            SemanticStoreIntegrityError::PlanningStateMismatch,
                        ),
                    )?;
                    final_bindings.push(binding);
                }
                OwnerPlanEntry::New(key) => {
                    let (slot, generation) = self.planner.allocate(public_slot_limit)?;
                    let public_slot = u32::try_from(slot)
                        .map_err(|_| SemanticReconcileError::IdentityExhausted)?;
                    let binding = SemanticBinding {
                        key: key.clone(),
                        id: runtime.__runtime_semantic_id(public_slot, generation),
                    };
                    self.operations.push(SemanticStoreOperation::Insert {
                        slot,
                        generation,
                        public_slot_limit,
                        record: SemanticRecord {
                            owner: owner.clone(),
                            key,
                        },
                    });
                    final_bindings.push(binding);
                }
            }
        }
        debug_assert_eq!(existing_index, existing_count);
        Ok(final_bindings)
    }

    pub(super) fn commit(self) {
        for operation in self.operations {
            match operation {
                SemanticStoreOperation::Remove { slot, generation } => {
                    let removed = self.store.arena.remove(slot, generation);
                    assert!(
                        removed.is_some(),
                        "planned semantic removal must match the borrow-protected live arena"
                    );
                }
                SemanticStoreOperation::Insert {
                    slot,
                    generation,
                    public_slot_limit,
                    record,
                } => {
                    let actual = self
                        .store
                        .arena
                        .insert_with_public_slot_limit(public_slot_limit, move |_, _| record);
                    assert_eq!(
                        actual,
                        Ok((slot, generation)),
                        "planned semantic allocation must match the borrow-protected live arena"
                    );
                }
            }
        }
    }
}

struct ValidatedBinding {
    binding: SemanticBinding,
    slot: usize,
    generation: u64,
}

enum OwnerPlanEntry {
    Existing,
    New(SemanticKey),
}

enum SemanticStoreOperation {
    Remove {
        slot: usize,
        generation: u64,
    },
    Insert {
        slot: usize,
        generation: u64,
        public_slot_limit: u64,
        record: SemanticRecord,
    },
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
            let first = transaction
                .plan_owner(&runtime, &first_owner, &[], &[SemanticKey::PRIMARY], 2)
                .unwrap_or_else(|_| unreachable!("first owner fits"));
            let second = transaction
                .plan_owner(&runtime, &second_owner, &[], &[SemanticKey::PRIMARY], 2)
                .unwrap_or_else(|_| unreachable!("second owner fits"));
            assert_ne!(first[0].id(), second[0].id());
            transaction.commit();
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
    fn failed_later_plan_drops_the_whole_transaction_without_mutation() {
        let runtime = RuntimeNamespace::__runtime_new();
        let first_owner = runtime.__runtime_mounted_id(1, 1);
        let second_owner = runtime.__runtime_mounted_id(2, 1);
        let mut store = SemanticStore::new();

        {
            let mut transaction = store.transaction();
            let first =
                transaction.plan_owner(&runtime, &first_owner, &[], &[SemanticKey::PRIMARY], 1);
            assert!(first.is_ok());
            let second =
                transaction.plan_owner(&runtime, &second_owner, &[], &[SemanticKey::PRIMARY], 1);
            assert_eq!(second, Err(SemanticReconcileError::IdentityExhausted));
        }

        assert_eq!(store.live_count(), 0);
    }
}
