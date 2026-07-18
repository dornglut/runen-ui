//! Private live application and mounted work identity and registry.

#![allow(clippy::redundant_pub_crate)]

use core::num::NonZeroU64;
use std::collections::HashMap;
use std::collections::HashSet;

use runenui_core::{__runtime::Effect, HostProtocol, WorkFamily as AuthoredWorkFamily, WorkKey};

use crate::{MountedNodeId, RuntimeLimits, TraceSequence};

pub(crate) mod host_request;
pub(crate) mod subscription;
pub(crate) mod task;
pub(crate) mod timer;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum WorkOwner {
    Application,
    Mounted(MountedNodeId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum WorkFamily {
    LocalTask,
    SendTask,
    Timer,
    Subscription,
    HostRequest,
}

impl From<AuthoredWorkFamily> for WorkFamily {
    fn from(value: AuthoredWorkFamily) -> Self {
        match value {
            AuthoredWorkFamily::LocalTask => Self::LocalTask,
            AuthoredWorkFamily::SendTask => Self::SendTask,
            AuthoredWorkFamily::Timer => Self::Timer,
            AuthoredWorkFamily::HostRequest => Self::HostRequest,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct WorkGeneration(NonZeroU64);

impl WorkGeneration {
    pub(crate) const fn get(self) -> u64 {
        self.0.get()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkTraceIdentity {
    pub(crate) owner: WorkOwner,
    pub(crate) family: WorkFamily,
    pub(crate) generation: WorkGeneration,
    pub(crate) key: Option<WorkKey>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkState {
    PendingStart,
    Running,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct KeyIdentity {
    owner: WorkOwner,
    family: WorkFamily,
    key: WorkKey,
}

pub(crate) struct WorkRecord<Action, Protocol: HostProtocol> {
    pub(crate) owner: WorkOwner,
    pub(crate) family: WorkFamily,
    pub(crate) key: Option<WorkKey>,
    pub(crate) state: WorkState,
    pub(crate) effect: Option<Effect<Action, Protocol>>,
    pub(crate) request_trace: Option<TraceSequence>,
    pub(crate) latest_trace: Option<TraceSequence>,
}

pub(crate) struct WorkRegistry<Action, Protocol: HostProtocol> {
    records: HashMap<WorkGeneration, WorkRecord<Action, Protocol>>,
    keyed: HashMap<KeyIdentity, WorkGeneration>,
    next_generation: Option<NonZeroU64>,
    limits: RuntimeLimits,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct WorkCancellationCounts {
    pub(crate) local_tasks: usize,
    pub(crate) send_tasks: usize,
    pub(crate) timers: usize,
    pub(crate) subscriptions: usize,
    pub(crate) host_requests: usize,
}

impl WorkCancellationCounts {
    pub(crate) const fn total(self) -> usize {
        self.local_tasks
            .saturating_add(self.send_tasks)
            .saturating_add(self.timers)
            .saturating_add(self.subscriptions)
            .saturating_add(self.host_requests)
    }
}

impl<Action, Protocol: HostProtocol> WorkRegistry<Action, Protocol> {
    pub(crate) fn new(limits: RuntimeLimits) -> Self {
        Self {
            records: HashMap::new(),
            keyed: HashMap::new(),
            next_generation: NonZeroU64::new(1),
            limits,
        }
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn live_record_count_for_test(&self) -> usize {
        self.records.len()
    }

    pub(crate) fn current(
        &self,
        owner: &WorkOwner,
        family: WorkFamily,
        key: &WorkKey,
    ) -> Option<WorkGeneration> {
        self.keyed
            .get(&KeyIdentity {
                owner: owner.clone(),
                family,
                key: key.clone(),
            })
            .copied()
    }

    pub(crate) fn preview_generations(
        &self,
        count: usize,
    ) -> Result<(Vec<WorkGeneration>, Option<NonZeroU64>), RegistryInsertError> {
        let mut next = self.next_generation;
        let mut generations = Vec::with_capacity(count);
        for _ in 0..count {
            let current = next.ok_or(RegistryInsertError::GenerationExhausted)?;
            generations.push(WorkGeneration(current));
            next = current.get().checked_add(1).and_then(NonZeroU64::new);
        }
        Ok((generations, next))
    }

    pub(crate) fn preflight_mounted_callback(
        &self,
        max_outputs: usize,
    ) -> Result<(), MountedCallbackPreflightError> {
        self.preview_generations(max_outputs)
            .map_err(|_| MountedCallbackPreflightError::GenerationExhausted)?;
        for family in [
            WorkFamily::LocalTask,
            WorkFamily::SendTask,
            WorkFamily::Timer,
        ] {
            if self
                .live_family_count(family)
                .checked_add(max_outputs)
                .is_none_or(|required| required > self.family_limit(family))
            {
                return Err(MountedCallbackPreflightError::FamilyFull(family));
            }
        }
        Ok(())
    }

    pub(crate) const fn commit_generation_reservation(&mut self, next: Option<NonZeroU64>) {
        self.next_generation = next;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_generation_for_test(&mut self, next: u64) {
        self.next_generation = NonZeroU64::new(next);
    }

    pub(crate) fn preflight_records(
        &self,
        invalidated: &HashSet<WorkGeneration>,
        starts: &[crate::transaction::PlannedStart<Action, Protocol>],
    ) -> Result<(), RegistryInsertError> {
        self.preflight_planned_families(
            invalidated,
            &starts.iter().map(|start| (start.generation, start.family)),
        )
    }

    pub(crate) fn preflight_planned_families(
        &self,
        invalidated: &HashSet<WorkGeneration>,
        starts: &(impl Iterator<Item = (WorkGeneration, WorkFamily)> + Clone),
    ) -> Result<(), RegistryInsertError> {
        for family in [
            WorkFamily::LocalTask,
            WorkFamily::SendTask,
            WorkFamily::Timer,
            WorkFamily::Subscription,
            WorkFamily::HostRequest,
        ] {
            let retained = self
                .records
                .iter()
                .filter(|(generation, record)| {
                    record.family == family && !invalidated.contains(generation)
                })
                .count();
            let added = starts
                .clone()
                .filter(|(generation, start_family)| {
                    *start_family == family && !invalidated.contains(generation)
                })
                .count();
            if retained
                .checked_add(added)
                .is_none_or(|count| count > self.family_limit(family))
            {
                return Err(RegistryInsertError::Full);
            }
        }
        Ok(())
    }

    pub(crate) fn generations_for_owner(&self, owner: &WorkOwner) -> Vec<WorkGeneration> {
        let mut generations: Vec<_> = self
            .records
            .iter()
            .filter_map(|(generation, record)| (&record.owner == owner).then_some(*generation))
            .collect();
        generations.sort_unstable();
        generations
    }

    pub(crate) fn commit_record(
        &mut self,
        generation: WorkGeneration,
        owner: WorkOwner,
        family: WorkFamily,
        key: Option<WorkKey>,
        effect: Effect<Action, Protocol>,
    ) {
        if let Some(key) = key.as_ref() {
            self.keyed.insert(
                KeyIdentity {
                    owner: owner.clone(),
                    family,
                    key: key.clone(),
                },
                generation,
            );
        }
        self.records.insert(
            generation,
            WorkRecord {
                owner,
                family,
                key,
                state: WorkState::PendingStart,
                effect: Some(effect),
                request_trace: None,
                latest_trace: None,
            },
        );
    }

    pub(crate) fn invalidate(&mut self, generation: WorkGeneration) -> Option<WorkFamily> {
        let record = self.records.remove(&generation)?;
        if let Some(key) = record.key.as_ref() {
            let identity = KeyIdentity {
                owner: record.owner,
                family: record.family,
                key: key.clone(),
            };
            if self.keyed.get(&identity) == Some(&generation) {
                self.keyed.remove(&identity);
            }
        }
        Some(record.family)
    }

    #[cfg(test)]
    pub(crate) fn insert(
        &mut self,
        owner: WorkOwner,
        family: WorkFamily,
        key: Option<WorkKey>,
        effect: Effect<Action, Protocol>,
    ) -> Result<(WorkGeneration, Option<WorkGeneration>), RegistryInsertError> {
        let replacing = key
            .as_ref()
            .is_some_and(|key| self.current(&owner, family, key).is_some());
        if !replacing && self.live_family_count(family) >= self.family_limit(family) {
            return Err(RegistryInsertError::Full);
        }
        let next = self
            .next_generation
            .ok_or(RegistryInsertError::GenerationExhausted)?;
        let generation = WorkGeneration(next);
        self.next_generation = next.get().checked_add(1).and_then(NonZeroU64::new);
        let replaced = key.as_ref().and_then(|key| {
            self.keyed.insert(
                KeyIdentity {
                    owner: owner.clone(),
                    family,
                    key: key.clone(),
                },
                generation,
            )
        });
        if let Some(replaced) = replaced {
            self.cancel_if_current_record(replaced);
        }
        self.records.insert(
            generation,
            WorkRecord {
                owner,
                family,
                key,
                state: WorkState::PendingStart,
                effect: Some(effect),
                request_trace: None,
                latest_trace: None,
            },
        );
        Ok((generation, replaced))
    }

    pub(crate) fn preflight_subscriptions(
        &self,
        invalidated: &HashSet<WorkGeneration>,
        added: usize,
    ) -> Result<(), RegistryInsertError> {
        let retained = self
            .records
            .iter()
            .filter(|(generation, record)| {
                record.family == WorkFamily::Subscription && !invalidated.contains(generation)
            })
            .count();
        if retained
            .checked_add(added)
            .is_none_or(|count| count > self.family_limit(WorkFamily::Subscription))
        {
            return Err(RegistryInsertError::Full);
        }
        Ok(())
    }

    pub(crate) fn commit_subscription_record(
        &mut self,
        generation: WorkGeneration,
        owner: WorkOwner,
        key: WorkKey,
    ) {
        let identity = KeyIdentity {
            owner: owner.clone(),
            family: WorkFamily::Subscription,
            key: key.clone(),
        };
        self.keyed.insert(identity, generation);
        self.records.insert(
            generation,
            WorkRecord {
                owner,
                family: WorkFamily::Subscription,
                key: Some(key),
                state: WorkState::PendingStart,
                effect: None,
                request_trace: None,
                latest_trace: None,
            },
        );
    }

    pub(crate) fn generation_with_value(&self, value: u64) -> Option<WorkGeneration> {
        self.records
            .keys()
            .find(|generation| generation.get() == value)
            .copied()
    }

    pub(crate) fn trace_parent(&self, generation: WorkGeneration) -> Option<TraceSequence> {
        self.records
            .get(&generation)
            .and_then(|record| record.latest_trace.or(record.request_trace))
    }

    pub(crate) fn set_trace(&mut self, generation: WorkGeneration, trace: TraceSequence) {
        if let Some(record) = self.records.get_mut(&generation) {
            if record.request_trace.is_none() {
                record.request_trace = Some(trace);
            }
            record.latest_trace = Some(trace);
        }
    }

    pub(crate) fn pending_family(&self, generation: WorkGeneration) -> Option<WorkFamily> {
        self.records
            .get(&generation)
            .filter(|record| record.state == WorkState::PendingStart)
            .map(|record| record.family)
    }

    pub(crate) fn trace_identity(&self, generation: WorkGeneration) -> Option<WorkTraceIdentity> {
        self.records
            .get(&generation)
            .map(|record| WorkTraceIdentity {
                owner: record.owner.clone(),
                family: record.family,
                generation,
                key: record.key.clone(),
            })
    }

    pub(crate) fn take_pending_effect(
        &mut self,
        generation: WorkGeneration,
    ) -> Option<Effect<Action, Protocol>> {
        self.records
            .get_mut(&generation)
            .filter(|record| record.state == WorkState::PendingStart)
            .and_then(|record| record.effect.take())
    }

    pub(crate) fn mark_running(&mut self, generation: WorkGeneration) -> Option<WorkFamily> {
        let record = self.records.get_mut(&generation)?;
        if record.state != WorkState::PendingStart {
            return None;
        }
        record.state = WorkState::Running;
        Some(record.family)
    }

    pub(crate) fn is_running(&self, generation: WorkGeneration) -> bool {
        self.records.get(&generation).is_some_and(|record| {
            record.state == WorkState::Running && self.binding_is_current(generation, record)
        })
    }

    pub(crate) fn is_running_family(&self, generation: WorkGeneration, family: WorkFamily) -> bool {
        self.records.get(&generation).is_some_and(|record| {
            record.state == WorkState::Running
                && record.family == family
                && self.binding_is_current(generation, record)
        })
    }

    pub(crate) fn is_live_family(&self, generation: WorkGeneration, family: WorkFamily) -> bool {
        self.records.get(&generation).is_some_and(|record| {
            record.family == family && self.binding_is_current(generation, record)
        })
    }

    fn binding_is_current(
        &self,
        generation: WorkGeneration,
        record: &WorkRecord<Action, Protocol>,
    ) -> bool {
        record.key.as_ref().is_none_or(|key| {
            self.keyed.get(&KeyIdentity {
                owner: record.owner.clone(),
                family: record.family,
                key: key.clone(),
            }) == Some(&generation)
        })
    }

    pub(crate) fn cancel_all_counts(&mut self) -> WorkCancellationCounts {
        let mut cancelled = WorkCancellationCounts::default();
        for record in self.records.drain().map(|(_, record)| record) {
            increment_cancellation_count(&mut cancelled, record.family);
        }
        self.keyed.clear();
        cancelled
    }

    #[cfg(test)]
    fn cancel_if_current_record(&mut self, generation: WorkGeneration) {
        let _ = self.invalidate(generation);
    }

    fn live_family_count(&self, family: WorkFamily) -> usize {
        self.records
            .values()
            .filter(|record| record.family == family)
            .count()
    }

    const fn family_limit(&self, family: WorkFamily) -> usize {
        match family {
            WorkFamily::LocalTask => self.limits.local_tasks(),
            WorkFamily::SendTask => self.limits.send_tasks(),
            WorkFamily::Timer => self.limits.timers(),
            WorkFamily::Subscription => self.limits.subscriptions(),
            WorkFamily::HostRequest => self.limits.host_requests(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MountedCallbackPreflightError {
    FamilyFull(WorkFamily),
    GenerationExhausted,
}

const fn increment_cancellation_count(counts: &mut WorkCancellationCounts, family: WorkFamily) {
    match family {
        WorkFamily::LocalTask => counts.local_tasks += 1,
        WorkFamily::SendTask => counts.send_tasks += 1,
        WorkFamily::Timer => counts.timers += 1,
        WorkFamily::Subscription => counts.subscriptions += 1,
        WorkFamily::HostRequest => counts.host_requests += 1,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RegistryInsertError {
    Full,
    GenerationExhausted,
}

#[cfg(test)]
mod tests {
    use runenui_core::{__runtime::Effect, Effects, NoHostProtocol, WorkKey};

    use super::{WorkFamily, WorkOwner, WorkRegistry, WorkState};
    use crate::RuntimeLimits;

    fn task(key: WorkKey) -> Effect<(), NoHostProtocol> {
        let effects = Effects::keyed_local_task(key, async { None });
        effects
            .__runtime_into_items()
            .pop()
            .unwrap_or_else(|| unreachable!())
    }

    #[test]
    fn stale_key_generation_never_cancels_replacement() {
        let key = WorkKey::new("refresh").unwrap_or_else(|_| unreachable!());
        let mut registry: WorkRegistry<(), NoHostProtocol> =
            WorkRegistry::new(RuntimeLimits::default());
        let (first, replaced) = registry
            .insert(
                WorkOwner::Application,
                WorkFamily::LocalTask,
                Some(key.clone()),
                task(key.clone()),
            )
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(replaced, None);
        let (second, replaced) = registry
            .insert(
                WorkOwner::Application,
                WorkFamily::LocalTask,
                Some(key.clone()),
                task(key.clone()),
            )
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(replaced, Some(first));
        assert!(!registry.records.contains_key(&first));

        assert!(registry.invalidate(first).is_none());
        assert_eq!(
            registry.current(&WorkOwner::Application, WorkFamily::LocalTask, &key),
            Some(second)
        );
        assert_eq!(registry.records[&second].state, WorkState::PendingStart);
    }

    #[test]
    fn completed_anonymous_work_leaves_no_registry_tombstones() {
        let mut registry: WorkRegistry<(), NoHostProtocol> =
            WorkRegistry::new(RuntimeLimits::default());
        for _ in 0..10_000 {
            let effect = Effects::local_task(async { None })
                .__runtime_into_items()
                .pop()
                .unwrap_or_else(|| unreachable!());
            let (generation, replaced) = registry
                .insert(WorkOwner::Application, WorkFamily::LocalTask, None, effect)
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(replaced, None);
            assert_eq!(
                registry.mark_running(generation),
                Some(WorkFamily::LocalTask)
            );
            assert!(registry.invalidate(generation).is_some());
        }
        assert!(registry.records.is_empty());
        assert!(registry.keyed.is_empty());
    }
}
