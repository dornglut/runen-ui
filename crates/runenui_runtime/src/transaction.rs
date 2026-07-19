//! Provisional ordered output planning and atomic queue/registry commit inputs.

#![allow(clippy::redundant_pub_crate)]

use std::collections::{HashMap, HashSet};

use runenui_core::{
    __runtime::{Effect, Subscription},
    Effects, HostProtocol, WorkKey,
};

use crate::{
    MountedNodeId,
    queue::WorkQueue,
    work::{WorkFamily, WorkGeneration, WorkOwner, WorkRegistry},
};

pub(crate) struct OwnedTransactionLedger<Action, Protocol: HostProtocol> {
    pub(crate) owner: WorkOwner,
    pub(crate) ledger: TransactionLedger<Action, Protocol>,
}

pub(crate) struct ApplicationTransactionInput<Action, Protocol: HostProtocol> {
    pub(crate) lifecycle_invalidated: Vec<WorkGeneration>,
    pub(crate) mounted_subscription_dirty: Vec<MountedNodeId>,
    pub(crate) application: TransactionLedger<Action, Protocol>,
    pub(crate) application_subscription_invalidated: Vec<WorkGeneration>,
    pub(crate) application_subscription_starts: Vec<Subscription<Action>>,
    pub(crate) mounted: Vec<OwnedTransactionLedger<Action, Protocol>>,
}

pub(crate) struct PlannedApplicationTransaction<Action, Protocol: HostProtocol> {
    pub(crate) invalidated: Vec<WorkGeneration>,
    pub(crate) starts: Vec<PlannedOwnedStart<Action, Protocol>>,
    pub(crate) application_outputs: Vec<PlannedOutput<Action>>,
    pub(crate) application_subscription_starts: Vec<WorkGeneration>,
    pub(crate) mounted_outputs: Vec<PlannedOutput<Action>>,
    pub(crate) mounted_subscription_dirty: Vec<MountedNodeId>,
    pub(crate) next_generation: Option<core::num::NonZeroU64>,
    pub(crate) semantic_events: Vec<PlannedWorkSemanticEvent>,
}

pub(crate) struct PlannedOwnedStart<Action, Protocol: HostProtocol> {
    pub(crate) generation: WorkGeneration,
    pub(crate) owner: WorkOwner,
    pub(crate) family: WorkFamily,
    pub(crate) key: Option<WorkKey>,
    pub(crate) payload: PlannedStartPayload<Action, Protocol>,
}

pub(crate) enum PlannedStartPayload<Action, Protocol: HostProtocol> {
    Effect(Effect<Action, Protocol>),
    Subscription(Subscription<Action>),
}

pub(crate) struct TransactionLedger<Action, Protocol: HostProtocol> {
    outputs: Vec<Effect<Action, Protocol>>,
}

impl<Action, Protocol: HostProtocol> TransactionLedger<Action, Protocol> {
    pub(crate) fn collect(
        effects: Effects<Action, Protocol>,
        limit: usize,
    ) -> Result<Self, TransactionOutputError> {
        let outputs = effects.__runtime_into_items();
        if outputs.len() > limit {
            return Err(TransactionOutputError::Full {
                limit,
                attempted: outputs.len(),
            });
        }
        Ok(Self { outputs })
    }

    pub(crate) fn into_outputs(self) -> Vec<Effect<Action, Protocol>> {
        self.outputs
    }

    pub(crate) fn from_outputs(
        outputs: Vec<Effect<Action, Protocol>>,
        limit: usize,
    ) -> Result<Self, TransactionOutputError> {
        if outputs.len() > limit {
            return Err(TransactionOutputError::Full {
                limit,
                attempted: outputs.len(),
            });
        }
        Ok(Self { outputs })
    }

    pub(crate) const fn len(&self) -> usize {
        self.outputs.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlannedWorkSemanticEvent {
    Requested(WorkGeneration),
    Invalidated(WorkGeneration),
}

pub(crate) enum PlannedOutput<Action> {
    Action(Action),
    Start(WorkGeneration),
    Redraw,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct OwnedProvisionalKey {
    owner: WorkOwner,
    family: WorkFamily,
    key: WorkKey,
}

struct PreparedApplicationTransaction<Action, Protocol: HostProtocol> {
    lifecycle_invalidated: Vec<WorkGeneration>,
    mounted_subscription_dirty: Vec<MountedNodeId>,
    application_effects: Vec<Effect<Action, Protocol>>,
    application_subscription_invalidated: Vec<WorkGeneration>,
    application_subscription_starts: Vec<Subscription<Action>>,
    mounted_effects: Vec<(WorkOwner, Vec<Effect<Action, Protocol>>)>,
}

impl<Action, Protocol: HostProtocol> PreparedApplicationTransaction<Action, Protocol> {
    fn new(input: ApplicationTransactionInput<Action, Protocol>) -> Self {
        Self {
            lifecycle_invalidated: input.lifecycle_invalidated,
            mounted_subscription_dirty: input.mounted_subscription_dirty,
            application_effects: input.application.into_outputs(),
            application_subscription_invalidated: input.application_subscription_invalidated,
            application_subscription_starts: input.application_subscription_starts,
            mounted_effects: input
                .mounted
                .into_iter()
                .map(|batch| (batch.owner, batch.ledger.into_outputs()))
                .collect(),
        }
    }

    fn start_count(&self) -> Result<usize, TransactionPlanError> {
        let application = self
            .application_effects
            .iter()
            .filter(|effect| effect_family(effect).is_some())
            .count()
            .checked_add(self.application_subscription_starts.len())
            .ok_or(TransactionPlanError::WorkGenerationExhausted)?;
        self.mounted_effects
            .iter()
            .try_fold(application, |count, (_, effects)| {
                count.checked_add(
                    effects
                        .iter()
                        .filter(|effect| effect_family(effect).is_some())
                        .count(),
                )
            })
            .ok_or(TransactionPlanError::WorkGenerationExhausted)
    }
}

impl<Action, Protocol: HostProtocol> PlannedApplicationTransaction<Action, Protocol> {
    pub(crate) fn plan(
        input: ApplicationTransactionInput<Action, Protocol>,
        work: &WorkRegistry<Action, Protocol>,
        queue: &WorkQueue<Action>,
    ) -> Result<Self, TransactionPlanError> {
        let prepared = PreparedApplicationTransaction::new(input);
        let start_count = prepared.start_count()?;
        let PreparedApplicationTransaction {
            lifecycle_invalidated,
            mounted_subscription_dirty,
            application_effects,
            application_subscription_invalidated,
            application_subscription_starts,
            mounted_effects,
        } = prepared;
        let (generations, next_generation) = work.preview_generations(start_count)?;
        let mut generations = generations.into_iter();
        let mut provisional = HashMap::<OwnedProvisionalKey, WorkGeneration>::new();
        let mut invalidated_set = HashSet::new();
        let mut invalidated = Vec::new();
        for generation in lifecycle_invalidated {
            insert_invalidation(generation, &mut invalidated_set, &mut invalidated);
        }
        let mut semantic_events = Vec::with_capacity(start_count.saturating_mul(2));

        let mut starts = Vec::with_capacity(start_count);
        let mut application_outputs = Vec::with_capacity(application_effects.len());
        plan_owned_effects(
            &WorkOwner::Application,
            application_effects,
            &mut generations,
            work,
            &mut provisional,
            &mut invalidated_set,
            &mut invalidated,
            &mut starts,
            &mut application_outputs,
            &mut semantic_events,
        );

        for generation in application_subscription_invalidated {
            if insert_invalidation(generation, &mut invalidated_set, &mut invalidated) {
                semantic_events.push(PlannedWorkSemanticEvent::Invalidated(generation));
            }
        }

        let application_subscription_start_generations = plan_application_subscription_starts(
            application_subscription_starts,
            &mut generations,
            &mut provisional,
            &mut starts,
            &mut semantic_events,
        );

        let mounted_output_capacity = mounted_effects
            .iter()
            .map(|(_, effects)| effects.len())
            .sum();
        let mut mounted_outputs = Vec::with_capacity(mounted_output_capacity);
        for (owner, effects) in mounted_effects {
            plan_owned_effects(
                &owner,
                effects,
                &mut generations,
                work,
                &mut provisional,
                &mut invalidated_set,
                &mut invalidated,
                &mut starts,
                &mut mounted_outputs,
                &mut semantic_events,
            );
        }

        let sequenced_outputs = application_outputs
            .iter()
            .chain(&mounted_outputs)
            .filter(|output| !matches!(output, PlannedOutput::Redraw))
            .count()
            .checked_add(application_subscription_start_generations.len())
            .and_then(|count| count.checked_add(invalidated.len()))
            .and_then(|count| count.checked_add(mounted_subscription_dirty.len()))
            .ok_or(TransactionPlanError::QueueFull)?;
        queue.preflight_commit(sequenced_outputs)?;
        work.preflight_planned_families(
            &invalidated_set,
            &starts.iter().map(|start| (start.generation, start.family)),
        )?;
        Ok(Self {
            invalidated,
            starts,
            application_outputs,
            application_subscription_starts: application_subscription_start_generations,
            mounted_outputs,
            mounted_subscription_dirty,
            next_generation,
            semantic_events,
        })
    }
}

fn plan_application_subscription_starts<Action, Protocol: HostProtocol>(
    declarations: Vec<Subscription<Action>>,
    generations: &mut impl Iterator<Item = WorkGeneration>,
    provisional: &mut HashMap<OwnedProvisionalKey, WorkGeneration>,
    starts: &mut Vec<PlannedOwnedStart<Action, Protocol>>,
    semantic_events: &mut Vec<PlannedWorkSemanticEvent>,
) -> Vec<WorkGeneration> {
    let mut planned = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let generation = generations
            .next()
            .unwrap_or_else(|| unreachable!("start count and generations agree"));
        let key = declaration.key.clone();
        provisional.insert(
            OwnedProvisionalKey {
                owner: WorkOwner::Application,
                family: WorkFamily::Subscription,
                key: key.clone(),
            },
            generation,
        );
        semantic_events.push(PlannedWorkSemanticEvent::Requested(generation));
        starts.push(PlannedOwnedStart {
            generation,
            owner: WorkOwner::Application,
            family: WorkFamily::Subscription,
            key: Some(key),
            payload: PlannedStartPayload::Subscription(declaration),
        });
        planned.push(generation);
    }
    planned
}

#[allow(clippy::too_many_arguments)]
fn plan_owned_effects<Action, Protocol: HostProtocol>(
    owner: &WorkOwner,
    effects: Vec<Effect<Action, Protocol>>,
    generations: &mut impl Iterator<Item = WorkGeneration>,
    work: &WorkRegistry<Action, Protocol>,
    provisional: &mut HashMap<OwnedProvisionalKey, WorkGeneration>,
    invalidated_set: &mut HashSet<WorkGeneration>,
    invalidated: &mut Vec<WorkGeneration>,
    starts: &mut Vec<PlannedOwnedStart<Action, Protocol>>,
    outputs: &mut Vec<PlannedOutput<Action>>,
    semantic_events: &mut Vec<PlannedWorkSemanticEvent>,
) {
    for effect in effects {
        if let Effect::Cancel { family, key } = effect {
            let family = family.into();
            let identity = OwnedProvisionalKey {
                owner: owner.clone(),
                family,
                key,
            };
            let target = provisional
                .get(&identity)
                .copied()
                .or_else(|| work.current(&identity.owner, family, &identity.key));
            if let Some(target) = target
                && insert_invalidation(target, invalidated_set, invalidated)
            {
                semantic_events.push(PlannedWorkSemanticEvent::Invalidated(target));
            }
            continue;
        }
        if let Effect::Action(action) = effect {
            outputs.push(PlannedOutput::Action(action));
            continue;
        }
        if matches!(effect, Effect::Redraw) {
            outputs.push(PlannedOutput::Redraw);
            continue;
        }
        let generation = generations
            .next()
            .unwrap_or_else(|| unreachable!("start count and generations agree"));
        let family =
            effect_family(&effect).unwrap_or_else(|| unreachable!("work effect has a family"));
        let key = effect_key(&effect).cloned();
        if let Some(key) = key.as_ref() {
            let identity = OwnedProvisionalKey {
                owner: owner.clone(),
                family,
                key: key.clone(),
            };
            let replaced = provisional
                .get(&identity)
                .copied()
                .or_else(|| work.current(owner, family, key));
            if let Some(replaced) = replaced
                && insert_invalidation(replaced, invalidated_set, invalidated)
            {
                semantic_events.push(PlannedWorkSemanticEvent::Invalidated(replaced));
            }
            provisional.insert(identity, generation);
        }
        semantic_events.push(PlannedWorkSemanticEvent::Requested(generation));
        starts.push(PlannedOwnedStart {
            generation,
            owner: owner.clone(),
            family,
            key,
            payload: PlannedStartPayload::Effect(effect),
        });
        outputs.push(PlannedOutput::Start(generation));
    }
}

fn insert_invalidation(
    generation: WorkGeneration,
    invalidated_set: &mut HashSet<WorkGeneration>,
    invalidated: &mut Vec<WorkGeneration>,
) -> bool {
    if invalidated_set.insert(generation) {
        invalidated.push(generation);
        true
    } else {
        false
    }
}

const fn effect_family<Action, Protocol: HostProtocol>(
    effect: &Effect<Action, Protocol>,
) -> Option<WorkFamily> {
    match effect {
        Effect::LocalTask(_) => Some(WorkFamily::LocalTask),
        Effect::SendTask(_) => Some(WorkFamily::SendTask),
        Effect::Timer(_) => Some(WorkFamily::Timer),
        Effect::HostRequest(_) => Some(WorkFamily::HostRequest),
        Effect::Action(_) | Effect::Cancel { .. } | Effect::Redraw => None,
    }
}

const fn effect_key<Action, Protocol: HostProtocol>(
    effect: &Effect<Action, Protocol>,
) -> Option<&WorkKey> {
    match effect {
        Effect::LocalTask(task) => task.key.as_ref(),
        Effect::SendTask(task) => task.key.as_ref(),
        Effect::Timer(timer) => timer.__runtime_key(),
        Effect::HostRequest(request) => request.key.as_ref(),
        Effect::Action(_) | Effect::Cancel { .. } | Effect::Redraw => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransactionOutputError {
    Full { limit: usize, attempted: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransactionPlanError {
    QueueFull,
    WorkSequenceExhausted,
    RegistryFull,
    WorkGenerationExhausted,
}

impl From<crate::queue::QueueCommitError> for TransactionPlanError {
    fn from(error: crate::queue::QueueCommitError) -> Self {
        match error {
            crate::queue::QueueCommitError::Full => Self::QueueFull,
            crate::queue::QueueCommitError::SequenceExhausted => Self::WorkSequenceExhausted,
        }
    }
}

impl From<crate::work::RegistryInsertError> for TransactionPlanError {
    fn from(error: crate::work::RegistryInsertError) -> Self {
        match error {
            crate::work::RegistryInsertError::Full => Self::RegistryFull,
            crate::work::RegistryInsertError::GenerationExhausted => Self::WorkGenerationExhausted,
        }
    }
}
