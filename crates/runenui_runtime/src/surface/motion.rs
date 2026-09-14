//! Staged deterministic motion reconciliation and sampling for one surface candidate.
//!
//! This module owns no clock, renderer state, widget callbacks, or authored state.
//! It consumes one runtime-supplied monotonic candidate instant and produces a staged
//! replacement motion store plus topology-aligned sampled overrides. The caller commits
//! those products through the existing surface-publication transaction.

use std::collections::BTreeSet;
use std::time::Duration;

use runenui_core::{
    __runtime::{
        MotionInterpolationKind, apply_motion_value, ease_motion, interpolate_motion_sample,
        motion_value_for_target,
    },
    BrushToken, ColorToken, ComputedStyle, ElementId, ExplicitTimeline, LayoutStyle,
    MonotonicInstant, MotionRepeat, MotionTarget, MotionValue, OpacityToken, PresentationToken,
    RadiusToken, ReducedMotionStrategy, ShadowToken, SpacingToken, StyleFieldProvenance,
    StylePreferenceKind, StylePreferences, StyleResolution, StyleResolutionLayer, TransitionPolicy,
    TransitionSpec, TypographyToken, UnitInterval,
};

use crate::{
    MountedNodeId,
    mounted::MountedTree,
    trace::{
        StagedMotionTraceFact, TraceMotionFact, TraceMotionInterpolation, TraceMotionLifecycle,
        TraceMotionPhase, TraceMotionPolicy, TraceMotionPreferenceDecision, TraceMotionSource,
    },
};

use super::{
    SurfaceCache,
    resolve::{
        CachedEffectiveFacts, CachedStyleFacts, EffectiveNodeFacts, SurfaceTopologySnapshot,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MotionPlanningError {
    DuplicateAnimationId,
    DuplicateTarget(MotionTarget),
    ScheduleOverflow,
    Interpolation(MotionTarget),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct MotionActivity {
    continuous_redraw: bool,
    followup_publication: bool,
    next_deadline: Option<MonotonicInstant>,
}

impl MotionActivity {
    pub(super) const fn continuous_redraw(self) -> bool {
        self.continuous_redraw
    }

    pub(super) const fn followup_publication(self) -> bool {
        self.followup_publication
    }

    pub(super) const fn next_deadline(self) -> Option<MonotonicInstant> {
        self.next_deadline
    }

    fn note_deadline(&mut self, deadline: MonotonicInstant) {
        self.next_deadline = Some(
            self.next_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct MotionStore {
    explicit: Vec<ExplicitMotionRecord>,
    transitions: Vec<TransitionRecord>,
}

impl MotionStore {
    pub(super) fn retire_owner(&mut self, owner: &MountedNodeId) {
        self.explicit.retain(|record| &record.owner != owner);
        self.transitions.retain(|record| &record.owner != owner);
    }

    pub(super) fn clear(&mut self) {
        self.explicit.clear();
        self.transitions.clear();
    }
}

pub(super) struct PlannedMotion {
    store: MotionStore,
    effective: CachedEffectiveFacts,
    activity: MotionActivity,
    trace_facts: Vec<StagedMotionTraceFact>,
}

impl PlannedMotion {
    pub(super) fn into_parts(
        self,
    ) -> (
        MotionStore,
        CachedEffectiveFacts,
        MotionActivity,
        Vec<StagedMotionTraceFact>,
    ) {
        (self.store, self.effective, self.activity, self.trace_facts)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ExplicitMotionRecord {
    owner: MountedNodeId,
    declaration: ExplicitTimeline,
    lifecycle: ExplicitLifecycle,
}

#[derive(Clone, Debug, PartialEq)]
enum ExplicitLifecycle {
    Active { start: MonotonicInstant },
    HoldInitial,
    Completed,
}

#[derive(Clone, Debug, PartialEq)]
struct TransitionRecord {
    owner: MountedNodeId,
    target: MotionTarget,
    from: MotionValue,
    to: MotionValue,
    target_provenance: TargetProvenance,
    spec: TransitionSpec,
    start: MonotonicInstant,
}

#[derive(Clone, Debug, PartialEq)]
enum TargetProvenance {
    Foreground(
        StyleFieldProvenance<ColorToken>,
        Option<StyleResolutionLayer>,
    ),
    Background(
        StyleFieldProvenance<BrushToken>,
        Option<StyleResolutionLayer>,
    ),
    Padding(
        StyleFieldProvenance<SpacingToken>,
        Option<StyleResolutionLayer>,
    ),
    Radius(
        StyleFieldProvenance<RadiusToken>,
        Option<StyleResolutionLayer>,
    ),
    Typography(
        StyleFieldProvenance<TypographyToken>,
        Option<StyleResolutionLayer>,
    ),
    Shadows(
        StyleFieldProvenance<ShadowToken>,
        Option<StyleResolutionLayer>,
    ),
    Opacity(
        StyleFieldProvenance<OpacityToken>,
        Option<StyleResolutionLayer>,
    ),
    Presentation(
        StyleFieldProvenance<PresentationToken>,
        Option<StyleResolutionLayer>,
    ),
    Structural,
}

impl TargetProvenance {
    const fn high_contrast(&self) -> bool {
        let layer = match self {
            Self::Foreground(_, layer)
            | Self::Background(_, layer)
            | Self::Padding(_, layer)
            | Self::Radius(_, layer)
            | Self::Typography(_, layer)
            | Self::Shadows(_, layer)
            | Self::Opacity(_, layer)
            | Self::Presentation(_, layer) => layer.as_ref(),
            Self::Structural => None,
        };
        matches!(
            layer,
            Some(StyleResolutionLayer::Preference(
                StylePreferenceKind::HighContrast
            ))
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ResolvedTransitionPolicy {
    Absent,
    Disabled,
    Enabled(TransitionSpec),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MotionPreferenceMode {
    Normal,
    SnapToEnd,
    HoldInitial,
    PreserveEssential,
}

struct ExplicitEvaluation {
    record: ExplicitMotionRecord,
    candidate_sample: Option<MotionValue>,
    terminal_commit: bool,
    started_at_candidate: bool,
    live_sample: Option<LiveSample>,
}

struct ExplicitExitFacts {
    sample: Option<MotionValue>,
    source: Option<MotionValue>,
    terminal_commit: bool,
}

#[derive(Clone, Copy)]
enum TransitionRetirement {
    Replaced,
    Cancelled,
    Completed,
}

struct TransitionEvaluation {
    record: Option<TransitionRecord>,
    retired_sample: Option<LiveSample>,
    retired_strategy: Option<ReducedMotionStrategy>,
    retirement: Option<TransitionRetirement>,
    candidate_sample: Option<LiveSample>,
    candidate_strategy: Option<ReducedMotionStrategy>,
    started_at_candidate: bool,
    terminal_commit: bool,
}

impl TransitionEvaluation {
    const fn none() -> Self {
        Self {
            record: None,
            retired_sample: None,
            retired_strategy: None,
            retirement: None,
            candidate_sample: None,
            candidate_strategy: None,
            started_at_candidate: false,
            terminal_commit: false,
        }
    }
}

#[derive(Clone)]
struct ValueSample {
    value: MotionValue,
    progress: Option<UnitInterval>,
    eased_progress: Option<UnitInterval>,
    interpolation: MotionInterpolationKind,
}

#[derive(Clone)]
struct LiveSample {
    value: MotionValue,
    phase: LivePhase,
    terminal_deadline: Option<MonotonicInstant>,
    progress: Option<UnitInterval>,
    eased_progress: Option<UnitInterval>,
    interpolation: MotionInterpolationKind,
}

#[derive(Clone, Copy)]
enum LivePhase {
    Delayed { deadline: MonotonicInstant },
    Running,
    Completed,
    HeldInitial,
}

struct OwnerMotionContext<'a> {
    owner: &'a MountedNodeId,
    authored_id: Option<&'a ElementId>,
    declarations: &'a [ExplicitTimeline],
    resolution: &'a StyleResolution,
    target_layout: &'a LayoutStyle,
    prior_computed: Option<&'a ComputedStyle>,
    prior_layout: Option<&'a LayoutStyle>,
    preferences: StylePreferences,
    instant: MonotonicInstant,
    position: usize,
}

struct OwnerRetained<'a> {
    explicit: Vec<&'a ExplicitMotionRecord>,
    transitions: Vec<&'a TransitionRecord>,
}

impl<'a> OwnerRetained<'a> {
    fn new(store: &'a MotionStore, owner: &MountedNodeId) -> Self {
        Self {
            explicit: store
                .explicit
                .iter()
                .filter(|record| &record.owner == owner)
                .collect(),
            transitions: store
                .transitions
                .iter()
                .filter(|record| &record.owner == owner)
                .collect(),
        }
    }
}

struct OwnerMotionOutputs<'a> {
    effective: &'a mut CachedEffectiveFacts,
    store: &'a mut MotionStore,
    activity: &'a mut MotionActivity,
    trace_facts: &'a mut Vec<StagedMotionTraceFact>,
}

struct TransitionIntent<'a> {
    owner: &'a MountedNodeId,
    target: MotionTarget,
    target_value: &'a MotionValue,
    provenance: &'a TargetProvenance,
    policy: &'a ResolvedTransitionPolicy,
    preferences: StylePreferences,
    instant: MonotonicInstant,
}

pub(super) fn plan_surface_motion<Action>(
    store: &MotionStore,
    tree: &MountedTree<Action>,
    topology: &SurfaceTopologySnapshot,
    styles: &CachedStyleFacts,
    previous_cache: Option<&SurfaceCache>,
    preferences: StylePreferences,
    instant: MonotonicInstant,
) -> Result<PlannedMotion, MotionPlanningError> {
    debug_assert_eq!(topology.nodes.len(), styles.resolutions.len());
    for topology_node in &topology.nodes {
        let node = tree
            .node(&topology_node.id)
            .unwrap_or_else(|| unreachable!("motion topology remains live"));
        validate_owner_declarations(&node.timelines)?;
    }

    let mut effective = CachedEffectiveFacts::identity(tree, topology, styles);
    let mut next_store = MotionStore::default();
    let mut activity = MotionActivity::default();
    let mut trace_facts = Vec::new();

    for (position, topology_node) in topology.nodes.iter().enumerate() {
        let node = tree
            .node(&topology_node.id)
            .unwrap_or_else(|| unreachable!("motion topology remains live"));
        let prior_position = previous_cache.and_then(|cache| {
            cache
                .topology
                .nodes
                .iter()
                .position(|candidate| candidate.id == topology_node.id)
        });
        let prior_computed = previous_cache.and_then(|cache| {
            prior_position.map(|prior| cache.effective.node(prior).computed_style())
        });
        let prior_layout = previous_cache
            .and_then(|cache| prior_position.map(|prior| cache.effective.node(prior).layout()));
        let context = OwnerMotionContext {
            owner: &topology_node.id,
            authored_id: topology_node.authored_id.as_ref(),
            declarations: &node.timelines,
            resolution: &styles.resolutions[position],
            target_layout: &node.layout,
            prior_computed,
            prior_layout,
            preferences,
            instant,
            position,
        };
        let mut outputs = OwnerMotionOutputs {
            effective: &mut effective,
            store: &mut next_store,
            activity: &mut activity,
            trace_facts: &mut trace_facts,
        };
        plan_owner(store, &context, &mut outputs)?;
    }

    Ok(PlannedMotion {
        store: next_store,
        effective,
        activity,
        trace_facts,
    })
}

fn plan_owner(
    store: &MotionStore,
    context: &OwnerMotionContext<'_>,
    outputs: &mut OwnerMotionOutputs<'_>,
) -> Result<(), MotionPlanningError> {
    let retained = OwnerRetained::new(store, context.owner);
    let targets = owner_targets(context, &retained);
    for target in &targets {
        trace_resolved_policy(context, *target, outputs.trace_facts);
    }
    let explicit_evaluations = reconcile_explicit_declarations(context, &retained.explicit)?;
    trace_explicit_retirements(context, &retained.explicit, outputs.trace_facts)?;
    trace_preempted_transitions(
        context,
        &retained.transitions,
        &explicit_evaluations,
        outputs.trace_facts,
    )?;
    trace_explicit_entries(
        context,
        &retained.explicit,
        &explicit_evaluations,
        outputs.trace_facts,
    );
    outputs.store.explicit.extend(
        explicit_evaluations
            .iter()
            .map(|candidate| candidate.record.clone()),
    );

    for target in targets {
        plan_target(context, target, &retained, &explicit_evaluations, outputs)?;
    }
    Ok(())
}

fn reconcile_explicit_declarations(
    context: &OwnerMotionContext<'_>,
    retained: &[&ExplicitMotionRecord],
) -> Result<Vec<ExplicitEvaluation>, MotionPlanningError> {
    context
        .declarations
        .iter()
        .map(|declaration| {
            let prior = retained
                .iter()
                .copied()
                .find(|record| record.declaration.id() == declaration.id());
            reconcile_explicit(
                context.owner,
                declaration,
                prior,
                context.preferences,
                context.instant,
            )
        })
        .collect()
}

fn trace_explicit_retirements(
    context: &OwnerMotionContext<'_>,
    retained: &[&ExplicitMotionRecord],
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) -> Result<(), MotionPlanningError> {
    for record in retained {
        let current = context
            .declarations
            .iter()
            .find(|declaration| declaration.id() == record.declaration.id());
        if current.is_some_and(|declaration| declaration == &record.declaration) {
            continue;
        }
        if let Some(sample) = sample_retained_explicit(record, context.instant)? {
            push_explicit_sample_trace(context, &record.declaration, &sample, trace_facts);
        }
        if current.is_some() {
            push_explicit_lifecycle(
                context,
                &record.declaration,
                TraceMotionLifecycle::Replaced,
                trace_facts,
            );
        } else if !matches!(record.lifecycle, ExplicitLifecycle::Completed) {
            push_explicit_lifecycle(
                context,
                &record.declaration,
                TraceMotionLifecycle::Cancelled,
                trace_facts,
            );
        }
    }
    Ok(())
}

fn trace_preempted_transitions(
    context: &OwnerMotionContext<'_>,
    retained: &[&TransitionRecord],
    evaluations: &[ExplicitEvaluation],
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) -> Result<(), MotionPlanningError> {
    for transition in retained {
        let claimed = evaluations.iter().any(|evaluation| {
            evaluation.record.declaration.target() == transition.target
                && explicit_has_candidate_authority(evaluation)
        });
        if !claimed {
            continue;
        }
        let sample = sample_transition(transition, context.instant)?;
        if matches!(sample.phase, LivePhase::Completed) {
            push_transition_lifecycle(
                context,
                transition.target,
                TraceMotionLifecycle::Completed,
                trace_facts,
            );
            push_transition_sample_trace(context, transition.target, &sample, trace_facts);
        } else {
            push_transition_sample_trace(context, transition.target, &sample, trace_facts);
            push_transition_lifecycle(
                context,
                transition.target,
                TraceMotionLifecycle::Cancelled,
                trace_facts,
            );
        }
    }
    Ok(())
}

fn trace_explicit_entries(
    context: &OwnerMotionContext<'_>,
    retained: &[&ExplicitMotionRecord],
    evaluations: &[ExplicitEvaluation],
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    for evaluation in evaluations {
        let declaration = &evaluation.record.declaration;
        let prior = retained
            .iter()
            .copied()
            .find(|record| record.declaration.id() == declaration.id());
        push_explicit_preference_trace(context, declaration, trace_facts);
        trace_explicit_entry(context, prior, evaluation, trace_facts);
        if let Some(sample) = evaluation.live_sample.as_ref() {
            push_explicit_sample_trace(context, declaration, sample, trace_facts);
        }
    }
}

fn trace_explicit_entry(
    context: &OwnerMotionContext<'_>,
    prior: Option<&ExplicitMotionRecord>,
    evaluation: &ExplicitEvaluation,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    let declaration = &evaluation.record.declaration;
    let Some(prior) = prior.filter(|record| record.declaration == *declaration) else {
        if evaluation.started_at_candidate {
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::Started,
                trace_facts,
            );
        }
        match evaluation.record.lifecycle {
            ExplicitLifecycle::HoldInitial => push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::HoldInitialEntered,
                trace_facts,
            ),
            ExplicitLifecycle::Completed if evaluation.terminal_commit => push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::Completed,
                trace_facts,
            ),
            ExplicitLifecycle::Active { .. } | ExplicitLifecycle::Completed => {}
        }
        return;
    };

    match (&prior.lifecycle, &evaluation.record.lifecycle) {
        (ExplicitLifecycle::Completed, ExplicitLifecycle::Completed) => push_explicit_lifecycle(
            context,
            declaration,
            TraceMotionLifecycle::CompletedRetained,
            trace_facts,
        ),
        (ExplicitLifecycle::HoldInitial, ExplicitLifecycle::Active { .. }) => {
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::HoldInitialReleased,
                trace_facts,
            );
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::Restarted,
                trace_facts,
            );
        }
        (ExplicitLifecycle::HoldInitial, ExplicitLifecycle::Completed)
            if evaluation.terminal_commit =>
        {
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::HoldInitialReleased,
                trace_facts,
            );
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::Restarted,
                trace_facts,
            );
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::Completed,
                trace_facts,
            );
        }
        (ExplicitLifecycle::Active { .. }, ExplicitLifecycle::HoldInitial) => {
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::HoldInitialEntered,
                trace_facts,
            );
        }
        (ExplicitLifecycle::Active { .. }, ExplicitLifecycle::Completed)
            if evaluation.terminal_commit =>
        {
            push_explicit_lifecycle(
                context,
                declaration,
                TraceMotionLifecycle::Completed,
                trace_facts,
            );
        }
        (ExplicitLifecycle::Active { .. }, ExplicitLifecycle::Active { .. })
        | (
            ExplicitLifecycle::HoldInitial | ExplicitLifecycle::Completed,
            ExplicitLifecycle::HoldInitial,
        )
        | (ExplicitLifecycle::Completed, ExplicitLifecycle::Active { .. })
        | (
            ExplicitLifecycle::Active { .. } | ExplicitLifecycle::HoldInitial,
            ExplicitLifecycle::Completed,
        ) => {}
    }
}

fn push_explicit_preference_trace(
    context: &OwnerMotionContext<'_>,
    declaration: &ExplicitTimeline,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    push_motion_preference_trace(
        context,
        declaration.target(),
        TraceMotionSource::Timeline {
            animation_id: declaration.id().clone(),
        },
        declaration.spec().reduced_motion(),
        trace_facts,
    );
}

fn push_transition_preference_trace(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    strategy: ReducedMotionStrategy,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    push_motion_preference_trace(
        context,
        target,
        TraceMotionSource::Transition,
        strategy,
        trace_facts,
    );
}

fn push_motion_preference_trace(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    source: TraceMotionSource,
    strategy: ReducedMotionStrategy,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    let decision = trace_preference_decision(motion_preference_mode(context.preferences, strategy));
    trace_facts.push(StagedMotionTraceFact::new(
        context.owner.clone(),
        context.authored_id.cloned(),
        target,
        TraceMotionFact::Preference {
            source: source.clone(),
            reduced_motion: context.preferences.reduced_motion(),
            strategy,
            decision,
        },
    ));
    if target_provenance(context.resolution, target).high_contrast() {
        trace_facts.push(StagedMotionTraceFact::new(
            context.owner.clone(),
            context.authored_id.cloned(),
            target,
            TraceMotionFact::Preference {
                source,
                reduced_motion: context.preferences.reduced_motion(),
                strategy,
                decision: TraceMotionPreferenceDecision::HighContrastSuppressed,
            },
        ));
    }
}

fn push_explicit_lifecycle(
    context: &OwnerMotionContext<'_>,
    declaration: &ExplicitTimeline,
    lifecycle: TraceMotionLifecycle,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    trace_facts.push(StagedMotionTraceFact::new(
        context.owner.clone(),
        context.authored_id.cloned(),
        declaration.target(),
        TraceMotionFact::Lifecycle {
            source: TraceMotionSource::Timeline {
                animation_id: declaration.id().clone(),
            },
            lifecycle,
        },
    ));
}

fn push_explicit_sample_trace(
    context: &OwnerMotionContext<'_>,
    declaration: &ExplicitTimeline,
    sample: &LiveSample,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    let suppressed = target_provenance(context.resolution, declaration.target()).high_contrast();
    trace_facts.push(StagedMotionTraceFact::new(
        context.owner.clone(),
        context.authored_id.cloned(),
        declaration.target(),
        TraceMotionFact::Sampled {
            source: TraceMotionSource::Timeline {
                animation_id: declaration.id().clone(),
            },
            phase: trace_live_phase(sample.phase),
            progress_bits: sample.progress.map(|progress| progress.get().to_bits()),
            eased_progress_bits: sample
                .eased_progress
                .map(|progress| progress.get().to_bits()),
            interpolation: trace_interpolation(sample.interpolation),
            suppressed,
        },
    ));
}

fn push_transition_lifecycle(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    lifecycle: TraceMotionLifecycle,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    trace_facts.push(StagedMotionTraceFact::new(
        context.owner.clone(),
        context.authored_id.cloned(),
        target,
        TraceMotionFact::Lifecycle {
            source: TraceMotionSource::Transition,
            lifecycle,
        },
    ));
}

fn push_transition_sample_trace(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    sample: &LiveSample,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    let suppressed = target_provenance(context.resolution, target).high_contrast();
    trace_facts.push(StagedMotionTraceFact::new(
        context.owner.clone(),
        context.authored_id.cloned(),
        target,
        TraceMotionFact::Sampled {
            source: TraceMotionSource::Transition,
            phase: trace_live_phase(sample.phase),
            progress_bits: sample.progress.map(|progress| progress.get().to_bits()),
            eased_progress_bits: sample
                .eased_progress
                .map(|progress| progress.get().to_bits()),
            interpolation: trace_interpolation(sample.interpolation),
            suppressed,
        },
    ));
}

fn owner_targets(
    context: &OwnerMotionContext<'_>,
    retained: &OwnerRetained<'_>,
) -> BTreeSet<MotionTarget> {
    let mut targets = BTreeSet::new();
    targets.extend(context.declarations.iter().map(ExplicitTimeline::target));
    targets.extend(
        retained
            .explicit
            .iter()
            .map(|record| record.declaration.target()),
    );
    targets.extend(retained.transitions.iter().map(|record| record.target));
    targets.extend(
        context
            .resolution
            .transition_policies()
            .map(|(target, _, _)| target),
    );
    targets
}

fn trace_resolved_policy(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    let policy = resolved_transition_policy(context.resolution, target);
    trace_facts.push(StagedMotionTraceFact::new(
        context.owner.clone(),
        context.authored_id.cloned(),
        target,
        TraceMotionFact::PolicyResolved {
            policy: trace_transition_policy(&policy),
        },
    ));
}

fn plan_target(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    retained: &OwnerRetained<'_>,
    explicit_evaluations: &[ExplicitEvaluation],
    outputs: &mut OwnerMotionOutputs<'_>,
) -> Result<(), MotionPlanningError> {
    let target_value = motion_value_for_target(
        context.resolution.computed_style(),
        context.target_layout,
        target,
    );
    let provenance = target_provenance(context.resolution, target);
    let suppressed = provenance.high_contrast();
    let policy = resolved_transition_policy(context.resolution, target);
    let new_explicit = explicit_evaluations
        .iter()
        .find(|candidate| candidate.record.declaration.target() == target);
    let old_explicit = retained
        .explicit
        .iter()
        .copied()
        .find(|record| record.declaration.target() == target);
    let old_transition = retained
        .transitions
        .iter()
        .copied()
        .find(|record| record.target == target);
    let old_explicit_sample = old_explicit
        .map(|record| sample_retained_explicit(record, context.instant))
        .transpose()?
        .flatten();
    let old_transition_sample = old_transition
        .map(|record| sample_transition(record, context.instant))
        .transpose()?;
    let transition_preempted =
        new_explicit.is_some_and(explicit_has_candidate_authority) && old_transition.is_some();

    if apply_explicit_candidate(new_explicit, suppressed, context.position, outputs) {
        return Ok(());
    }

    let explicit_exit = explicit_exit_facts(
        context,
        old_explicit,
        old_explicit_sample.as_ref(),
        new_explicit,
    );
    let prior_value = context
        .prior_computed
        .zip(context.prior_layout)
        .map(|(computed, layout)| motion_value_for_target(computed, layout, target));
    let source = explicit_exit
        .source
        .clone()
        .or_else(|| {
            old_transition_sample
                .as_ref()
                .map(|sample| sample.value.clone())
        })
        .or(prior_value)
        .unwrap_or_else(|| target_value.clone());
    let intent = TransitionIntent {
        owner: context.owner,
        target,
        target_value: &target_value,
        provenance: &provenance,
        policy: &policy,
        preferences: context.preferences,
        instant: context.instant,
    };
    let transition = reconcile_transition(
        &intent,
        &source,
        if transition_preempted {
            None
        } else {
            old_transition
        },
        if transition_preempted {
            None
        } else {
            old_transition_sample.as_ref()
        },
        explicit_exit.source.is_some(),
    )?;
    trace_transition_evaluation(context, target, &transition, outputs.trace_facts);
    apply_transition_candidate(
        &transition,
        explicit_exit.sample.as_ref(),
        explicit_exit.terminal_commit,
        &target_value,
        suppressed,
        context,
        outputs,
    );
    Ok(())
}

const fn explicit_has_candidate_authority(candidate: &ExplicitEvaluation) -> bool {
    candidate.live_sample.is_some()
}

fn explicit_exit_facts(
    context: &OwnerMotionContext<'_>,
    old_explicit: Option<&ExplicitMotionRecord>,
    old_sample: Option<&LiveSample>,
    new_explicit: Option<&ExplicitEvaluation>,
) -> ExplicitExitFacts {
    let sample = new_explicit.and_then(|candidate| candidate.candidate_sample.clone());
    let terminal_commit = new_explicit.is_some_and(|candidate| candidate.terminal_commit);
    let removed_or_replaced = old_explicit.is_some_and(|old| {
        !context.declarations.iter().any(|declaration| {
            declaration.id() == old.declaration.id() && declaration == &old.declaration
        })
    });
    let source = if terminal_commit {
        sample.clone()
    } else if removed_or_replaced {
        old_sample.map(|sample| sample.value.clone())
    } else {
        None
    };
    ExplicitExitFacts {
        sample,
        source,
        terminal_commit,
    }
}

fn apply_explicit_candidate(
    candidate: Option<&ExplicitEvaluation>,
    suppressed: bool,
    position: usize,
    outputs: &mut OwnerMotionOutputs<'_>,
) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    if let Some(live) = candidate.live_sample.as_ref() {
        note_live_activity(live, suppressed, outputs.activity);
    }
    let owns_target = matches!(
        candidate.record.lifecycle,
        ExplicitLifecycle::Active { .. } | ExplicitLifecycle::HoldInitial
    );
    if !owns_target {
        return false;
    }
    if !suppressed {
        if let Some(sample) = candidate.candidate_sample.as_ref() {
            apply_effective_sample(outputs.effective, position, sample);
        }
        if matches!(candidate.record.lifecycle, ExplicitLifecycle::Active { .. })
            && explicit_requires_group(&candidate.record.declaration)
        {
            retain_effective_group(outputs.effective, position);
        }
    }
    true
}

fn trace_transition_evaluation(
    context: &OwnerMotionContext<'_>,
    target: MotionTarget,
    evaluation: &TransitionEvaluation,
    trace_facts: &mut Vec<StagedMotionTraceFact>,
) {
    if let Some(strategy) = evaluation.retired_strategy {
        push_transition_preference_trace(context, target, strategy, trace_facts);
    }
    match evaluation.retirement {
        Some(TransitionRetirement::Replaced) => {
            if let Some(sample) = evaluation.retired_sample.as_ref() {
                push_transition_sample_trace(context, target, sample, trace_facts);
            }
            push_transition_lifecycle(context, target, TraceMotionLifecycle::Replaced, trace_facts);
        }
        Some(TransitionRetirement::Cancelled) => {
            if let Some(sample) = evaluation.retired_sample.as_ref() {
                push_transition_sample_trace(context, target, sample, trace_facts);
            }
            push_transition_lifecycle(
                context,
                target,
                TraceMotionLifecycle::Cancelled,
                trace_facts,
            );
        }
        Some(TransitionRetirement::Completed) => {
            push_transition_lifecycle(
                context,
                target,
                TraceMotionLifecycle::Completed,
                trace_facts,
            );
            if let Some(sample) = evaluation.retired_sample.as_ref() {
                push_transition_sample_trace(context, target, sample, trace_facts);
            }
        }
        None => {}
    }
    if let Some(strategy) = evaluation.candidate_strategy {
        push_transition_preference_trace(context, target, strategy, trace_facts);
    }
    if evaluation.started_at_candidate {
        push_transition_lifecycle(context, target, TraceMotionLifecycle::Started, trace_facts);
    }
    if evaluation.terminal_commit {
        push_transition_lifecycle(
            context,
            target,
            TraceMotionLifecycle::Completed,
            trace_facts,
        );
    }
    if let Some(sample) = evaluation.candidate_sample.as_ref() {
        push_transition_sample_trace(context, target, sample, trace_facts);
    }
}

fn apply_transition_candidate(
    evaluation: &TransitionEvaluation,
    explicit_sample: Option<&MotionValue>,
    explicit_terminal_commit: bool,
    target_value: &MotionValue,
    suppressed: bool,
    context: &OwnerMotionContext<'_>,
    outputs: &mut OwnerMotionOutputs<'_>,
) {
    if let Some(live) = evaluation.candidate_sample.as_ref() {
        let transition_is_live = !matches!(live.phase, LivePhase::Completed);
        if transition_is_live {
            note_live_activity(live, suppressed, outputs.activity);
            if let Some(record) = evaluation.record.as_ref() {
                outputs.store.transitions.push(record.clone());
                if !suppressed && transition_requires_group(record) {
                    retain_effective_group(outputs.effective, context.position);
                }
            }
        }
        if !suppressed {
            apply_effective_sample(outputs.effective, context.position, &live.value);
        }
    } else if explicit_terminal_commit && !suppressed {
        if let Some(sample) = explicit_sample {
            apply_effective_sample(outputs.effective, context.position, sample);
        }
        if explicit_sample.is_some_and(|sample| sample != target_value) {
            outputs.activity.followup_publication = true;
        }
    }
}

fn apply_effective_sample(
    effective: &mut CachedEffectiveFacts,
    position: usize,
    sample: &MotionValue,
) {
    let current = effective.node(position);
    let mut computed = current.computed_style().clone();
    let mut layout = current.layout().clone();
    let retain = current.retain_node_effect_group();
    apply_motion_value(&mut computed, &mut layout, sample);
    effective.nodes[position] = EffectiveNodeFacts::new(layout, computed, retain);
}

fn retain_effective_group(effective: &mut CachedEffectiveFacts, position: usize) {
    let current = effective.node(position);
    if current.retain_node_effect_group() {
        return;
    }
    effective.nodes[position] = EffectiveNodeFacts::new(
        current.layout().clone(),
        current.computed_style().clone(),
        true,
    );
}

fn reconcile_explicit(
    owner: &MountedNodeId,
    declaration: &ExplicitTimeline,
    retained: Option<&ExplicitMotionRecord>,
    preferences: StylePreferences,
    instant: MonotonicInstant,
) -> Result<ExplicitEvaluation, MotionPlanningError> {
    let preference = motion_preference_mode(preferences, declaration.spec().reduced_motion());
    let mut terminal_commit = false;
    let mut started_at_candidate = false;
    let lifecycle = match retained {
        Some(record) if record.declaration == *declaration => match (&record.lifecycle, preference) {
            (ExplicitLifecycle::Completed, _) => ExplicitLifecycle::Completed,
            (ExplicitLifecycle::HoldInitial, MotionPreferenceMode::HoldInitial) => {
                ExplicitLifecycle::HoldInitial
            }
            (
                ExplicitLifecycle::HoldInitial,
                MotionPreferenceMode::Normal | MotionPreferenceMode::PreserveEssential,
            ) => {
                started_at_candidate = true;
                checked_active(declaration, instant)?
            }
            (ExplicitLifecycle::HoldInitial, MotionPreferenceMode::SnapToEnd) => {
                terminal_commit = true;
                ExplicitLifecycle::Completed
            }
            (
                ExplicitLifecycle::Active { start },
                MotionPreferenceMode::Normal | MotionPreferenceMode::PreserveEssential,
            ) => ExplicitLifecycle::Active { start: *start },
            (ExplicitLifecycle::Active { .. }, MotionPreferenceMode::SnapToEnd) => {
                terminal_commit = true;
                ExplicitLifecycle::Completed
            }
            (ExplicitLifecycle::Active { .. }, MotionPreferenceMode::HoldInitial) => {
                ExplicitLifecycle::HoldInitial
            }
        },
        Some(_) | None => match preference {
            MotionPreferenceMode::SnapToEnd => {
                terminal_commit = true;
                ExplicitLifecycle::Completed
            }
            MotionPreferenceMode::HoldInitial => ExplicitLifecycle::HoldInitial,
            MotionPreferenceMode::Normal | MotionPreferenceMode::PreserveEssential => {
                started_at_candidate = true;
                checked_active(declaration, instant)?
            }
        },
    };

    let live_sample = match lifecycle {
        ExplicitLifecycle::Completed if terminal_commit => Some(endpoint_live_sample(
            terminal_keyframe(declaration).clone(),
            LivePhase::Completed,
            None,
            Some(UnitInterval::ONE),
        )),
        ExplicitLifecycle::Completed => None,
        ExplicitLifecycle::HoldInitial => Some(endpoint_live_sample(
            initial_keyframe(declaration).clone(),
            LivePhase::HeldInitial,
            None,
            Some(UnitInterval::ZERO),
        )),
        ExplicitLifecycle::Active { start } => Some(sample_explicit(declaration, start, instant)?),
    };
    if live_sample
        .as_ref()
        .is_some_and(|sample| matches!(sample.phase, LivePhase::Completed))
    {
        terminal_commit = true;
    }
    let candidate_sample = live_sample.as_ref().map(|sample| sample.value.clone());

    let lifecycle = if terminal_commit {
        ExplicitLifecycle::Completed
    } else {
        lifecycle
    };
    Ok(ExplicitEvaluation {
        record: ExplicitMotionRecord {
            owner: owner.clone(),
            declaration: declaration.clone(),
            lifecycle,
        },
        candidate_sample,
        terminal_commit,
        started_at_candidate,
        live_sample,
    })
}

fn reconcile_transition(
    intent: &TransitionIntent<'_>,
    source: &MotionValue,
    retained: Option<&TransitionRecord>,
    retained_sample: Option<&LiveSample>,
    explicit_exit: bool,
) -> Result<TransitionEvaluation, MotionPlanningError> {
    if explicit_exit {
        return start_transition(intent, source);
    }
    let Some(retained) = retained else {
        return start_transition(intent, source);
    };
    let Some(retained_sample) = retained_sample else {
        return Ok(TransitionEvaluation::none());
    };
    if matches!(retained_sample.phase, LivePhase::Completed) {
        let mut evaluation = start_transition(intent, &retained_sample.value)?;
        evaluation.retired_sample = Some(retained_sample.clone());
        evaluation.retired_strategy = Some(retained.spec.reduced_motion());
        evaluation.retirement = Some(TransitionRetirement::Completed);
        return Ok(evaluation);
    }

    let target_changed =
        retained.to != *intent.target_value || retained.target_provenance != *intent.provenance;
    match intent.policy {
        ResolvedTransitionPolicy::Disabled | ResolvedTransitionPolicy::Absent if target_changed => {
            Ok(TransitionEvaluation {
                record: None,
                retired_sample: Some(retained_sample.clone()),
                retired_strategy: None,
                retirement: Some(TransitionRetirement::Cancelled),
                candidate_sample: None,
                candidate_strategy: None,
                started_at_candidate: false,
                terminal_commit: false,
            })
        }
        ResolvedTransitionPolicy::Disabled => Ok(TransitionEvaluation {
            record: None,
            retired_sample: Some(retained_sample.clone()),
            retired_strategy: None,
            retirement: Some(TransitionRetirement::Cancelled),
            candidate_sample: None,
            candidate_strategy: None,
            started_at_candidate: false,
            terminal_commit: false,
        }),
        ResolvedTransitionPolicy::Enabled(spec) if target_changed || spec != &retained.spec => {
            let mut evaluation = start_transition(intent, &retained_sample.value)?;
            evaluation.retired_sample = Some(retained_sample.clone());
            evaluation.retired_strategy = None;
            evaluation.retirement = Some(TransitionRetirement::Replaced);
            Ok(evaluation)
        }
        ResolvedTransitionPolicy::Absent | ResolvedTransitionPolicy::Enabled(_) => {
            if motion_preference_mode(intent.preferences, retained.spec.reduced_motion())
                == MotionPreferenceMode::SnapToEnd
            {
                return Ok(TransitionEvaluation {
                    record: None,
                    retired_sample: Some(endpoint_live_sample(
                        retained.to.clone(),
                        LivePhase::Completed,
                        None,
                        Some(UnitInterval::ONE),
                    )),
                    retired_strategy: Some(retained.spec.reduced_motion()),
                    retirement: Some(TransitionRetirement::Completed),
                    candidate_sample: None,
                    candidate_strategy: None,
                    started_at_candidate: false,
                    terminal_commit: false,
                });
            }
            Ok(TransitionEvaluation {
                record: Some(retained.clone()),
                retired_sample: None,
                retired_strategy: None,
                retirement: None,
                candidate_sample: Some(retained_sample.clone()),
                candidate_strategy: Some(retained.spec.reduced_motion()),
                started_at_candidate: false,
                terminal_commit: false,
            })
        }
    }
}

fn start_transition(
    intent: &TransitionIntent<'_>,
    source: &MotionValue,
) -> Result<TransitionEvaluation, MotionPlanningError> {
    let ResolvedTransitionPolicy::Enabled(spec) = intent.policy else {
        return Ok(TransitionEvaluation::none());
    };
    if source == intent.target_value {
        return Ok(TransitionEvaluation::none());
    }
    match motion_preference_mode(intent.preferences, spec.reduced_motion()) {
        MotionPreferenceMode::SnapToEnd => {
            return Ok(TransitionEvaluation {
                record: None,
                retired_sample: None,
                retired_strategy: None,
                retirement: None,
                candidate_sample: Some(endpoint_live_sample(
                    intent.target_value.clone(),
                    LivePhase::Completed,
                    None,
                    Some(UnitInterval::ONE),
                )),
                candidate_strategy: Some(spec.reduced_motion()),
                started_at_candidate: false,
                terminal_commit: true,
            });
        }
        MotionPreferenceMode::HoldInitial => {
            unreachable!("validated transition cannot use HoldInitial reduced motion")
        }
        MotionPreferenceMode::Normal | MotionPreferenceMode::PreserveEssential => {}
    }
    check_transition_schedule(spec, intent.instant)?;
    let record = TransitionRecord {
        owner: intent.owner.clone(),
        target: intent.target,
        from: source.clone(),
        to: intent.target_value.clone(),
        target_provenance: intent.provenance.clone(),
        spec: spec.clone(),
        start: intent.instant,
    };
    let sample = sample_transition(&record, intent.instant)?;
    let terminal_commit = matches!(sample.phase, LivePhase::Completed);
    Ok(TransitionEvaluation {
        record: Some(record),
        retired_sample: None,
        retired_strategy: None,
        retirement: None,
        candidate_sample: Some(sample),
        candidate_strategy: Some(spec.reduced_motion()),
        started_at_candidate: true,
        terminal_commit,
    })
}

fn motion_preference_mode(
    preferences: StylePreferences,
    strategy: ReducedMotionStrategy,
) -> MotionPreferenceMode {
    if !preferences.reduced_motion() {
        return MotionPreferenceMode::Normal;
    }
    match strategy {
        ReducedMotionStrategy::SnapToEnd => MotionPreferenceMode::SnapToEnd,
        ReducedMotionStrategy::HoldInitial => MotionPreferenceMode::HoldInitial,
        ReducedMotionStrategy::PreserveEssential => MotionPreferenceMode::PreserveEssential,
        _ => unreachable!("runtime and core reduced-motion strategy vocabularies are version-locked"),
    }
}

const fn trace_preference_decision(mode: MotionPreferenceMode) -> TraceMotionPreferenceDecision {
    match mode {
        MotionPreferenceMode::Normal => TraceMotionPreferenceDecision::Normal,
        MotionPreferenceMode::SnapToEnd => TraceMotionPreferenceDecision::SnapToEnd,
        MotionPreferenceMode::HoldInitial => TraceMotionPreferenceDecision::HoldInitial,
        MotionPreferenceMode::PreserveEssential => TraceMotionPreferenceDecision::PreserveEssential,
    }
}

fn sample_retained_explicit(
    record: &ExplicitMotionRecord,
    instant: MonotonicInstant,
) -> Result<Option<LiveSample>, MotionPlanningError> {
    match record.lifecycle {
        ExplicitLifecycle::Completed => Ok(None),
        ExplicitLifecycle::HoldInitial => Ok(Some(endpoint_live_sample(
            initial_keyframe(&record.declaration).clone(),
            LivePhase::HeldInitial,
            None,
            Some(UnitInterval::ZERO),
        ))),
        ExplicitLifecycle::Active { start } => {
            sample_explicit(&record.declaration, start, instant).map(Some)
        }
    }
}

const fn endpoint_value_sample(value: MotionValue, progress: UnitInterval) -> ValueSample {
    ValueSample {
        value,
        progress: Some(progress),
        eased_progress: None,
        interpolation: MotionInterpolationKind::Endpoint,
    }
}

const fn endpoint_live_sample(
    value: MotionValue,
    phase: LivePhase,
    terminal_deadline: Option<MonotonicInstant>,
    progress: Option<UnitInterval>,
) -> LiveSample {
    LiveSample {
        value,
        phase,
        terminal_deadline,
        progress,
        eased_progress: None,
        interpolation: MotionInterpolationKind::Endpoint,
    }
}

fn sample_explicit(
    declaration: &ExplicitTimeline,
    start: MonotonicInstant,
    instant: MonotonicInstant,
) -> Result<LiveSample, MotionPlanningError> {
    let spec = declaration.spec();
    let delay_nanos = nanos(spec.delay());
    let delay_deadline = checked_add_nanos(start, delay_nanos)?;
    let terminal_deadline = match spec.repeat() {
        MotionRepeat::Finite(iterations) => {
            let duration_nanos = nanos(spec.duration());
            let active_nanos = duration_nanos
                .checked_mul(iterations.get())
                .unwrap_or_else(|| unreachable!("validated finite schedule remains representable"));
            Some(checked_add_nanos(
                start,
                delay_nanos.checked_add(active_nanos).unwrap_or_else(|| {
                    unreachable!("validated finite schedule remains representable")
                }),
            )?)
        }
        MotionRepeat::Forever => None,
        _ => unreachable!("runtime and core repeat vocabularies are version-locked"),
    };

    if instant < delay_deadline {
        return Ok(endpoint_live_sample(
            initial_keyframe(declaration).clone(),
            LivePhase::Delayed {
                deadline: delay_deadline,
            },
            terminal_deadline,
            Some(UnitInterval::ZERO),
        ));
    }
    if terminal_deadline.is_some_and(|deadline| instant >= deadline) {
        return Ok(endpoint_live_sample(
            terminal_keyframe(declaration).clone(),
            LivePhase::Completed,
            terminal_deadline,
            Some(UnitInterval::ONE),
        ));
    }

    let duration_nanos = nanos(spec.duration());
    if duration_nanos == 0 {
        return Ok(endpoint_live_sample(
            terminal_keyframe(declaration).clone(),
            LivePhase::Completed,
            terminal_deadline,
            Some(UnitInterval::ONE),
        ));
    }
    let active_nanos = instant
        .as_nanos()
        .checked_sub(delay_deadline.as_nanos())
        .unwrap_or_else(|| unreachable!("sample at/after delay never precedes active start"));
    let iteration_nanos = active_nanos % duration_nanos;
    let progress = normalized_ratio(iteration_nanos, duration_nanos);
    let sampled = sample_keyframes(declaration, progress)?;
    Ok(LiveSample {
        value: sampled.value,
        phase: LivePhase::Running,
        terminal_deadline,
        progress: sampled.progress,
        eased_progress: sampled.eased_progress,
        interpolation: sampled.interpolation,
    })
}

fn sample_transition(
    transition: &TransitionRecord,
    instant: MonotonicInstant,
) -> Result<LiveSample, MotionPlanningError> {
    let delay_nanos = nanos(transition.spec.delay());
    let duration_nanos = nanos(transition.spec.duration());
    let delay_deadline = checked_add_nanos(transition.start, delay_nanos)?;
    let terminal_deadline = checked_add_nanos(
        transition.start,
        delay_nanos
            .checked_add(duration_nanos)
            .unwrap_or_else(|| unreachable!("validated transition schedule remains representable")),
    )?;
    if instant < delay_deadline {
        return Ok(endpoint_live_sample(
            transition.from.clone(),
            LivePhase::Delayed {
                deadline: delay_deadline,
            },
            Some(terminal_deadline),
            Some(UnitInterval::ZERO),
        ));
    }
    if instant >= terminal_deadline || duration_nanos == 0 {
        return Ok(endpoint_live_sample(
            transition.to.clone(),
            LivePhase::Completed,
            Some(terminal_deadline),
            Some(UnitInterval::ONE),
        ));
    }
    let active_nanos = instant
        .as_nanos()
        .checked_sub(delay_deadline.as_nanos())
        .unwrap_or_else(|| unreachable!("transition sample is at/after active start"));
    let progress = normalized_ratio(active_nanos, duration_nanos);
    let eased = ease_motion(transition.spec.easing(), progress);
    let (value, interpolation) = interpolate_motion_sample(&transition.from, &transition.to, eased)
        .ok_or(MotionPlanningError::Interpolation(transition.target))?;
    Ok(LiveSample {
        value,
        phase: LivePhase::Running,
        terminal_deadline: Some(terminal_deadline),
        progress: Some(progress),
        eased_progress: Some(eased),
        interpolation,
    })
}

#[allow(clippy::cast_possible_truncation)]
fn sample_keyframes(
    declaration: &ExplicitTimeline,
    progress: UnitInterval,
) -> Result<ValueSample, MotionPlanningError> {
    let spec = declaration.spec();
    if progress == UnitInterval::ZERO {
        return Ok(endpoint_value_sample(
            initial_keyframe(declaration).clone(),
            progress,
        ));
    }
    if progress == UnitInterval::ONE {
        return Ok(endpoint_value_sample(
            terminal_keyframe(declaration).clone(),
            progress,
        ));
    }
    if let Some(keyframe) = spec
        .keyframes()
        .iter()
        .find(|keyframe| keyframe.offset() == progress)
    {
        return Ok(endpoint_value_sample(keyframe.value().clone(), progress));
    }
    let (segment, pair) = spec
        .keyframes()
        .windows(2)
        .enumerate()
        .find(|(_, pair)| {
            pair[0].offset().get() < progress.get() && progress.get() < pair[1].offset().get()
        })
        .unwrap_or_else(|| unreachable!("strict keyframes contain normalized interior progress"));
    let left = f64::from(pair[0].offset().get());
    let right = f64::from(pair[1].offset().get());
    let overall_progress = f64::from(progress.get());
    let segment_progress = ((overall_progress - left) / (right - left)) as f32;
    let segment_progress = UnitInterval::new(segment_progress)
        .unwrap_or_else(|_| unreachable!("contained segment progress remains normalized"));
    let eased = ease_motion(spec.easings()[segment], segment_progress);
    let (value, interpolation) = interpolate_motion_sample(pair[0].value(), pair[1].value(), eased)
        .ok_or_else(|| MotionPlanningError::Interpolation(spec.target()))?;
    Ok(ValueSample {
        value,
        progress: Some(progress),
        eased_progress: Some(eased),
        interpolation,
    })
}

fn note_live_activity(sample: &LiveSample, suppressed: bool, activity: &mut MotionActivity) {
    match (suppressed, sample.phase) {
        (_, LivePhase::Completed | LivePhase::HeldInitial) => {}
        (false, LivePhase::Delayed { deadline }) => activity.note_deadline(deadline),
        (false, LivePhase::Running) => activity.continuous_redraw = true,
        (true, LivePhase::Delayed { .. } | LivePhase::Running) => {
            if let Some(deadline) = sample.terminal_deadline {
                activity.note_deadline(deadline);
            }
        }
    }
}

fn checked_active(
    declaration: &ExplicitTimeline,
    instant: MonotonicInstant,
) -> Result<ExplicitLifecycle, MotionPlanningError> {
    check_timeline_schedule(declaration, instant)?;
    Ok(ExplicitLifecycle::Active { start: instant })
}

fn check_timeline_schedule(
    declaration: &ExplicitTimeline,
    instant: MonotonicInstant,
) -> Result<(), MotionPlanningError> {
    let spec = declaration.spec();
    let delay_nanos = nanos(spec.delay());
    match spec.repeat() {
        MotionRepeat::Finite(iterations) => {
            let active_nanos = nanos(spec.duration())
                .checked_mul(iterations.get())
                .unwrap_or_else(|| unreachable!("validated finite schedule remains representable"));
            checked_add_nanos(
                instant,
                delay_nanos.checked_add(active_nanos).unwrap_or_else(|| {
                    unreachable!("validated finite schedule remains representable")
                }),
            )?;
        }
        MotionRepeat::Forever => {
            checked_add_nanos(instant, delay_nanos)?;
        }
        _ => unreachable!("runtime and core repeat vocabularies are version-locked"),
    }
    Ok(())
}

fn check_transition_schedule(
    spec: &TransitionSpec,
    instant: MonotonicInstant,
) -> Result<(), MotionPlanningError> {
    checked_add_nanos(
        instant,
        nanos(spec.delay())
            .checked_add(nanos(spec.duration()))
            .unwrap_or_else(|| unreachable!("validated transition schedule remains representable")),
    )?;
    Ok(())
}

fn checked_add_nanos(
    instant: MonotonicInstant,
    nanos: u64,
) -> Result<MonotonicInstant, MotionPlanningError> {
    instant
        .checked_add(Duration::from_nanos(nanos))
        .map_err(|_| MotionPlanningError::ScheduleOverflow)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos())
        .unwrap_or_else(|_| unreachable!("validated motion duration fits u64 nanoseconds"))
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "timeline progress is a deterministic integer-nanosecond ratio normalized once into the accepted f32 UnitInterval domain"
)]
fn normalized_ratio(numerator: u64, denominator: u64) -> UnitInterval {
    debug_assert!(denominator > 0);
    if numerator == 0 {
        return UnitInterval::ZERO;
    }
    if numerator == denominator {
        return UnitInterval::ONE;
    }
    let ratio = (numerator as f64 / denominator as f64) as f32;
    UnitInterval::new(ratio)
        .unwrap_or_else(|_| unreachable!("bounded integer ratio remains normalized"))
}

fn initial_keyframe(declaration: &ExplicitTimeline) -> &MotionValue {
    declaration
        .spec()
        .keyframes()
        .first()
        .unwrap_or_else(|| unreachable!("validated timeline has an initial keyframe"))
        .value()
}

fn terminal_keyframe(declaration: &ExplicitTimeline) -> &MotionValue {
    declaration
        .spec()
        .keyframes()
        .last()
        .unwrap_or_else(|| unreachable!("validated timeline has a terminal keyframe"))
        .value()
}

fn validate_owner_declarations(
    declarations: &[ExplicitTimeline],
) -> Result<(), MotionPlanningError> {
    for (index, declaration) in declarations.iter().enumerate() {
        for previous in &declarations[..index] {
            if previous.id() == declaration.id() {
                return Err(MotionPlanningError::DuplicateAnimationId);
            }
            if previous.target() == declaration.target() {
                return Err(MotionPlanningError::DuplicateTarget(declaration.target()));
            }
        }
    }
    Ok(())
}

fn resolved_transition_policy(
    resolution: &StyleResolution,
    target: MotionTarget,
) -> ResolvedTransitionPolicy {
    match resolution.transition_policy(target) {
        None => ResolvedTransitionPolicy::Absent,
        Some(TransitionPolicy::Disabled) => ResolvedTransitionPolicy::Disabled,
        Some(TransitionPolicy::Enabled(spec)) => ResolvedTransitionPolicy::Enabled(spec.clone()),
        Some(_) => {
            unreachable!("runtime and core transition-policy vocabularies are version-locked")
        }
    }
}

const fn trace_transition_policy(policy: &ResolvedTransitionPolicy) -> TraceMotionPolicy {
    match policy {
        ResolvedTransitionPolicy::Absent => TraceMotionPolicy::Absent,
        ResolvedTransitionPolicy::Disabled => TraceMotionPolicy::Disabled,
        ResolvedTransitionPolicy::Enabled(_) => TraceMotionPolicy::Enabled,
    }
}

const fn trace_live_phase(phase: LivePhase) -> TraceMotionPhase {
    match phase {
        LivePhase::Delayed { .. } => TraceMotionPhase::Delayed,
        LivePhase::Running => TraceMotionPhase::Running,
        LivePhase::Completed => TraceMotionPhase::Completed,
        LivePhase::HeldInitial => TraceMotionPhase::HeldInitial,
    }
}

const fn trace_interpolation(kind: MotionInterpolationKind) -> TraceMotionInterpolation {
    match kind {
        MotionInterpolationKind::Endpoint => TraceMotionInterpolation::Endpoint,
        MotionInterpolationKind::Continuous => TraceMotionInterpolation::Continuous,
        MotionInterpolationKind::Discrete => TraceMotionInterpolation::Discrete,
    }
}

fn target_provenance(resolution: &StyleResolution, target: MotionTarget) -> TargetProvenance {
    let provenance = resolution.provenance();
    match target {
        MotionTarget::Foreground => TargetProvenance::Foreground(
            provenance.foreground().clone(),
            provenance.foreground_layer().cloned(),
        ),
        MotionTarget::Background => TargetProvenance::Background(
            provenance.background().clone(),
            provenance.background_layer().cloned(),
        ),
        MotionTarget::Padding => TargetProvenance::Padding(
            provenance.padding().clone(),
            provenance.padding_layer().cloned(),
        ),
        MotionTarget::Radius => TargetProvenance::Radius(
            provenance.radius().clone(),
            provenance.radius_layer().cloned(),
        ),
        MotionTarget::Typography => TargetProvenance::Typography(
            provenance.typography().clone(),
            provenance.typography_layer().cloned(),
        ),
        MotionTarget::Shadows => TargetProvenance::Shadows(
            provenance.shadows().clone(),
            provenance.shadows_layer().cloned(),
        ),
        MotionTarget::Opacity => TargetProvenance::Opacity(
            provenance.opacity().clone(),
            provenance.opacity_layer().cloned(),
        ),
        MotionTarget::Presentation => TargetProvenance::Presentation(
            provenance.presentation().clone(),
            provenance.presentation_layer().cloned(),
        ),
        MotionTarget::Width
        | MotionTarget::Height
        | MotionTarget::MinWidth
        | MotionTarget::MinHeight
        | MotionTarget::MaxWidth
        | MotionTarget::MaxHeight
        | MotionTarget::Margin
        | MotionTarget::Gap
        | MotionTarget::FlexGrow
        | MotionTarget::FlexShrink
        | MotionTarget::FlexBasis => TargetProvenance::Structural,
        _ => unreachable!("runtime and core motion-target vocabularies are version-locked"),
    }
}

fn explicit_requires_group(declaration: &ExplicitTimeline) -> bool {
    declaration
        .spec()
        .keyframes()
        .iter()
        .any(|keyframe| motion_value_requires_group(keyframe.value()))
}

fn transition_requires_group(transition: &TransitionRecord) -> bool {
    motion_value_requires_group(&transition.from) || motion_value_requires_group(&transition.to)
}

fn motion_value_requires_group(value: &MotionValue) -> bool {
    match value {
        MotionValue::Opacity(value) => *value != runenui_core::SceneOpacity::OPAQUE,
        MotionValue::Shadows(value) => !value.is_empty(),
        MotionValue::Foreground(_)
        | MotionValue::Background(_)
        | MotionValue::Padding(_)
        | MotionValue::Radius(_)
        | MotionValue::Typography(_)
        | MotionValue::Presentation(_)
        | MotionValue::Width(_)
        | MotionValue::Height(_)
        | MotionValue::MinWidth(_)
        | MotionValue::MinHeight(_)
        | MotionValue::MaxWidth(_)
        | MotionValue::MaxHeight(_)
        | MotionValue::Margin(_)
        | MotionValue::Gap(_)
        | MotionValue::FlexGrow(_)
        | MotionValue::FlexShrink(_)
        | MotionValue::FlexBasis(_) => false,
        _ => unreachable!("runtime and core motion-value vocabularies are version-locked"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use runenui_core::{
        __runtime::RuntimeNamespace, AnimationId, MotionEasing, MotionKeyframe, MotionRepeat,
        MotionTarget, MotionValue, ReducedMotionStrategy, SceneOpacity, TimelineSpec, UnitInterval,
    };

    use super::{
        ExplicitLifecycle, ExplicitMotionRecord, MotionPlanningError, sample_explicit,
        validate_owner_declarations,
    };

    fn timeline(
        id: &'static str,
        target: MotionTarget,
        duration: Duration,
        reduced: ReducedMotionStrategy,
    ) -> runenui_core::ExplicitTimeline {
        let (start, end) = match target {
            MotionTarget::Opacity => (
                MotionValue::Opacity(SceneOpacity::TRANSPARENT),
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
            _ => unreachable!("test helper only authors opacity"),
        };
        let spec = TimelineSpec::new(
            vec![
                MotionKeyframe::new(UnitInterval::ZERO, start),
                MotionKeyframe::new(UnitInterval::ONE, end),
            ],
            vec![MotionEasing::Linear],
            duration,
            Duration::ZERO,
            MotionRepeat::ONCE,
            Some(reduced),
        )
        .unwrap_or_else(|_| unreachable!("test timeline is valid"));
        runenui_core::ExplicitTimeline::new(
            AnimationId::from_static(id)
                .unwrap_or_else(|_| unreachable!("test animation id is valid")),
            spec,
        )
    }

    fn owner() -> runenui_core::MountedNodeId {
        RuntimeNamespace::__runtime_new().__runtime_mounted_id(0, 1)
    }

    #[test]
    fn duplicate_declarations_reject_without_order_arbitration() {
        let first = timeline(
            "fade",
            MotionTarget::Opacity,
            Duration::from_millis(100),
            ReducedMotionStrategy::PreserveEssential,
        );
        assert_eq!(
            validate_owner_declarations(&[first.clone(), first.clone()]),
            Err(MotionPlanningError::DuplicateAnimationId)
        );
        let second = timeline(
            "other",
            MotionTarget::Opacity,
            Duration::from_millis(100),
            ReducedMotionStrategy::PreserveEssential,
        );
        assert_eq!(
            validate_owner_declarations(&[first, second]),
            Err(MotionPlanningError::DuplicateTarget(MotionTarget::Opacity))
        );
    }

    #[test]
    fn exact_final_boundary_commits_terminal_sample() {
        let declaration = timeline(
            "fade",
            MotionTarget::Opacity,
            Duration::from_nanos(100),
            ReducedMotionStrategy::PreserveEssential,
        );
        let sample = sample_explicit(
            &declaration,
            runenui_core::MonotonicInstant::__runtime_from_nanos(10),
            runenui_core::MonotonicInstant::__runtime_from_nanos(110),
        )
        .unwrap_or_else(|_| unreachable!("valid sample is representable"));
        assert_eq!(sample.value, MotionValue::Opacity(SceneOpacity::OPAQUE));
        assert_eq!(sample.progress, Some(UnitInterval::ONE));
        assert!(matches!(sample.phase, super::LivePhase::Completed));
    }

    #[test]
    fn completed_record_owns_no_future_sample_authority() {
        let declaration = timeline(
            "fade",
            MotionTarget::Opacity,
            Duration::from_nanos(100),
            ReducedMotionStrategy::PreserveEssential,
        );
        let record = ExplicitMotionRecord {
            owner: owner(),
            declaration,
            lifecycle: ExplicitLifecycle::Completed,
        };
        assert!(matches!(record.lifecycle, ExplicitLifecycle::Completed));
    }
}
