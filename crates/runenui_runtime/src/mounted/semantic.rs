use std::collections::{BTreeMap, BTreeSet};

use runenui_core::__runtime::RuntimeNamespace;
use runenui_core::{MountedNodeId, SemanticKey, SemanticNodeId};

use super::arena::{ArenaCapacityError, GenerationalArena};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SemanticIdentityExhausted;

impl From<ArenaCapacityError> for SemanticIdentityExhausted {
    fn from(_: ArenaCapacityError) -> Self {
        Self
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

    pub(super) fn record(
        &self,
        runtime: &RuntimeNamespace,
        id: &SemanticNodeId,
    ) -> Option<&SemanticRecord> {
        let (slot, generation) = runtime.__runtime_semantic_parts(id)?;
        self.arena.get(slot as usize, generation)
    }

    pub(super) fn reconcile_owner(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        current: &[SemanticBinding],
        ordered_keys: &[SemanticKey],
        public_slot_limit: u64,
    ) -> Result<Vec<SemanticBinding>, SemanticIdentityExhausted> {
        debug_assert!(
            ordered_keys.iter().collect::<BTreeSet<_>>().len() == ordered_keys.len(),
            "semantic contribution validation guarantees unique owner-local keys"
        );

        let mut existing = current
            .iter()
            .cloned()
            .map(|binding| (binding.key.clone(), binding))
            .collect::<BTreeMap<_, _>>();
        let mut plan = Vec::with_capacity(ordered_keys.len());
        let mut additions = 0usize;
        for key in ordered_keys {
            if let Some(binding) = existing.remove(key) {
                plan.push(PlannedBinding::Existing(binding));
            } else {
                additions = additions.checked_add(1).ok_or(SemanticIdentityExhausted)?;
                plan.push(PlannedBinding::New(key.clone()));
            }
        }
        let removals = existing.into_values().collect::<Vec<_>>();

        let desired_live_count = self
            .arena
            .live_count()
            .checked_sub(removals.len())
            .and_then(|count| count.checked_add(additions))
            .ok_or(SemanticIdentityExhausted)?;
        let additionally_retired = removals
            .iter()
            .filter(|binding| {
                runtime
                    .__runtime_semantic_parts(binding.id())
                    .is_some_and(|(_, generation)| generation == u64::MAX)
            })
            .count();
        self.arena.preflight_live_count_after_retirement(
            desired_live_count,
            additionally_retired,
            public_slot_limit,
        )?;

        for binding in removals {
            self.revoke_binding(runtime, owner, &binding);
        }

        let mut bindings = Vec::with_capacity(plan.len());
        for entry in plan {
            match entry {
                PlannedBinding::Existing(binding) => {
                    debug_assert!(
                        self.record(runtime, binding.id()).is_some_and(|record| {
                            record.owner() == owner && record.key() == binding.key()
                        }),
                        "retained semantic binding must still name its exact owner and key"
                    );
                    bindings.push(binding);
                }
                PlannedBinding::New(key) => {
                    let owner_for_record = owner.clone();
                    let key_for_record = key.clone();
                    let (slot, generation) = self
                        .arena
                        .insert_with_public_slot_limit(public_slot_limit, move |_, _| {
                            SemanticRecord {
                                owner: owner_for_record,
                                key: key_for_record,
                            }
                        })
                        .unwrap_or_else(|_| {
                            unreachable!("semantic identity capacity was preflighted")
                        });
                    let slot = u32::try_from(slot)
                        .unwrap_or_else(|_| unreachable!("semantic arena uses public slots"));
                    bindings.push(SemanticBinding {
                        key,
                        id: runtime.__runtime_semantic_id(slot, generation),
                    });
                }
            }
        }
        Ok(bindings)
    }

    pub(super) fn revoke_owner(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        bindings: Vec<SemanticBinding>,
    ) {
        for binding in bindings {
            self.revoke_binding(runtime, owner, &binding);
        }
    }

    fn revoke_binding(
        &mut self,
        runtime: &RuntimeNamespace,
        owner: &MountedNodeId,
        binding: &SemanticBinding,
    ) {
        let Some((slot, generation)) = runtime.__runtime_semantic_parts(binding.id()) else {
            debug_assert!(
                false,
                "semantic store binding belongs to its runtime namespace"
            );
            return;
        };
        let slot = slot as usize;
        let matches = self
            .arena
            .get(slot, generation)
            .is_some_and(|record| record.owner() == owner && record.key() == binding.key());
        debug_assert!(
            matches,
            "semantic store binding must match its exact record"
        );
        if matches {
            let _ = self.arena.remove(slot, generation);
        }
    }
}

enum PlannedBinding {
    Existing(SemanticBinding),
    New(SemanticKey),
}

#[cfg(test)]
mod tests {
    use runenui_core::__runtime::RuntimeNamespace;
    use runenui_core::SemanticKey;

    use super::{SemanticIdentityExhausted, SemanticStore, SemanticTargetStatus};

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
            .reconcile_owner(
                &runtime,
                &owner,
                &first,
                &[b.clone(), SemanticKey::PRIMARY, a.clone()],
                8,
            )
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
            .reconcile_owner(
                &runtime,
                &owner,
                &[],
                &[SemanticKey::PRIMARY, extra.clone()],
                2,
            )
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
        store.revoke_owner(&runtime, &owner, bindings);
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
            Err(SemanticIdentityExhausted)
        );
        assert_eq!(store.live_count(), 0);
    }
}
