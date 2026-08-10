use std::collections::{BTreeMap, BTreeSet};

use runenui_core::{MountedNodeId, SemanticKey, SemanticNodeId};
use runenui_core::__runtime::RuntimeNamespace;

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

    pub(super) const fn live_count(&self) -> usize {
        self.arena.live_count()
    }

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
        current: Vec<SemanticBinding>,
        ordered_keys: &[SemanticKey],
        public_slot_limit: u64,
    ) -> Result<Vec<SemanticBinding>, SemanticIdentityExhausted> {
        debug_assert!(
            ordered_keys.iter().collect::<BTreeSet<_>>().len() == ordered_keys.len(),
            "semantic contribution validation guarantees unique owner-local keys"
        );

        let mut existing = current
            .into_iter()
            .map(|binding| (binding.key.clone(), binding))
            .collect::<BTreeMap<_, _>>();
        let mut plan = Vec::with_capacity(ordered_keys.len());
        let mut additions = 0usize;
        for key in ordered_keys {
            if let Some(binding) = existing.remove(key) {
                plan.push(PlannedBinding::Existing(binding));
            } else {
                additions = additions
                    .checked_add(1)
                    .ok_or(SemanticIdentityExhausted)?;
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
                    let runtime_for_id = runtime.clone();
                    let owner_for_record = owner.clone();
                    let key_for_record = key.clone();
                    let (slot, generation) = self.arena.insert_with_public_slot_limit(
                        public_slot_limit,
                        move |slot, generation| SemanticRecord {
                            owner: owner_for_record,
                            key: key_for_record,
                        },
                    )?;
                    let slot = u32::try_from(slot)
                        .map_err(|_| SemanticIdentityExhausted)?;
                    bindings.push(SemanticBinding {
                        key,
                        id: runtime_for_id.__runtime_semantic_id(slot, generation),
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
            debug_assert!(false, "semantic store binding belongs to its runtime namespace");
            return;
        };
        let slot = slot as usize;
        let matches = self.arena.get(slot, generation).is_some_and(|record| {
            record.owner() == owner && record.key() == binding.key()
        });
        debug_assert!(matches, "semantic store binding must match its exact record");
        if matches {
            let _ = self.arena.remove(slot, generation);
        }
    }
}

enum PlannedBinding {
    Existing(SemanticBinding),
    New(SemanticKey),
}
