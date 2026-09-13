//! Staged deterministic motion reconciliation and sampling for one surface candidate.
//!
//! This module owns no clock, renderer state, widget callbacks, or authored state.
//! It consumes one runtime-supplied monotonic candidate instant and produces a staged
//! replacement motion store plus topology-aligned sampled overrides. The caller commits
//! those products through the existing surface-publication transaction.

use std::collections::BTreeSet;
use std::time::Duration;

use runenui_core::{
    AnimationId, BrushToken, ColorToken, ComputedStyle, ExplicitTimeline, LayoutStyle,
    MonotonicInstant, MotionRepeat, MotionTarget, MotionValue, OpacityToken, PresentationToken,
    RadiusToken, ReducedMotionStrategy, ShadowToken, SpacingToken, StyleFieldProvenance,
    StylePreferenceKind, StylePreferences, StyleResolution, StyleResolutionLayer, TransitionPolicy,
    TransitionSpec, TypographyToken, UnitInterval,
    __runtime::{apply_motion_value, ease_motion, interpolate_motion_value, motion_value_for_target},
};

use crate::{MountedNodeId, mounted::MountedTree};

use super::{
    SurfaceCache,
    resolve::{CachedEffectiveFacts, CachedStyleFacts, SurfaceTopologySnapshot},
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
}

impl PlannedMotion {
    pub(super) fn into_parts(self) -> (MotionStore, CachedEffectiveFacts, MotionActivity) {
        (self.store, self.effective, self.activity)
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
    fn high_contrast(&self) -> bool {
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

struct ExplicitEvaluation {
    record: ExplicitMotionRecord,
    candidate_sample: Option<MotionValue>,
    terminal_commit: bool,
    live_sample: Option<LiveSample>,
}

#[derive(Clone)]
struct LiveSample {
    value: MotionValue,
    phase: LivePhase,
    terminal_deadline: Option<MonotonicInstant>,
}

#[derive(Clone, Copy)]
enum LivePhase {
    Delayed { deadline: MonotonicInstant },
    Running,
    Completed,
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

    for (position, topology_node) in topology.nodes.iter().enumerate() {
        let node = tree
            .node(&topology_node.id)
            .unwrap_or_else(|| unreachable!("motion topology remains live"));
        let resolution = &styles.resolutions[position];
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

        plan_owner(
            store,
            &topology_node.id,
            &node.timelines,
            resolution,
            &node.layout,
            prior_computed,
            prior_layout,
            preferences,
            instant,
            position,
            &mut effective,
            &mut next_store,
            &mut activity,
        )?;
    }

    Ok(PlannedMotion {
        store: next_store,
        effective,
        activity,
    })
}

#[allow(clippy::too_many_arguments)]
fn plan_owner(
    store: &MotionStore,
    owner: &MountedNodeId,
    declarations: &[ExplicitTimeline],
    resolution: &StyleResolution,
    target_layout: &LayoutStyle,
    prior_computed: Option<&ComputedStyle>,
    prior_layout: Option<&LayoutStyle>,
    preferences: StylePreferences,
    instant: MonotonicInstant,
    position: usize,
    effective: &mut CachedEffectiveFacts,
    next_store: &mut MotionStore,
    activity: &mut MotionActivity,
) -> Result<(), MotionPlanningError> {
    let old_explicit = store
        .explicit
        .iter()
        .filter(|record| &record.owner == owner)
        .collect::<Vec<_>>();
    let old_transitions = store
        .transitions
        .iter()
        .filter(|record| &record.owner == owner)
        .collect::<Vec<_>>();

    let mut explicit_evaluations = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let retained = old_explicit
            .iter()
            .copied()
            .find(|record| record.declaration.id() == declaration.id());
        explicit_evaluations.push(reconcile_explicit(
            owner,
            declaration,
            retained,
            preferences,
            instant,
        )?);
    }

    let mut targets = BTreeSet::new();
    targets.extend(declarations.iter().map(ExplicitTimeline::target));
    targets.extend(old_explicit.iter().map(|record| record.declaration.target()));
    targets.extend(old_transitions.iter().map(|record| record.target));
    targets.extend(
        resolution
            .transition_policies()
            .map(|(target, _, _)| target),
    );

    for target in targets {
        let target_value = motion_value_for_target(
            resolution.computed_style(),
            target_layout,
            target,
        );
        let provenance = target_provenance(resolution, target);
        let suppressed = provenance.high_contrast();
        let policy = resolved_transition_policy(resolution, target);

        let new_explicit = explicit_evaluations
            .iter()
            .find(|candidate| candidate.record.declaration.target() == target);
        let old_explicit_for_target = old_explicit
            .iter()
            .copied()
            .find(|record| record.declaration.target() == target);
        let old_transition = old_transitions
            .iter()
            .copied()
            .find(|record| record.target == target);

        let old_explicit_sample = old_explicit_for_target
            .map(|record| sample_retained_explicit(record, instant))
            .transpose()?
            .flatten();
        let old_transition_sample = old_transition
            .map(|record| sample_transition(record, instant))
            .transpose()?;
        let prior_value = match (prior_computed, prior_layout) {
            (Some(computed), Some(layout)) => {
                Some(motion_value_for_target(computed, layout, target))
            }
            _ => None,
        };

        if let Some(candidate) = new_explicit {
            next_store.explicit.push(candidate.record.clone());
        }

        let explicit_sample = new_explicit.and_then(|candidate| candidate.candidate_sample.clone());
        let explicit_terminal_commit = new_explicit.is_some_and(|candidate| candidate.terminal_commit);

        if let Some(candidate) = new_explicit
            && let Some(live) = candidate.live_sample.as_ref()
        {
            note_live_activity(live, suppressed, activity);
        }

        let new_explicit_owns_target = new_explicit.is_some_and(|candidate| {
            matches!(
                candidate.record.lifecycle,
                ExplicitLifecycle::Active { .. } | ExplicitLifecycle::HoldInitial
            )
        });

        if new_explicit_owns_target {
            if let Some(sample) = explicit_sample.as_ref()
                && !suppressed
            {
                effective.apply_motion_value(position, sample);
            }
            if !suppressed
                && new_explicit
                    .is_some_and(|candidate| explicit_requires_group(&candidate.record.declaration))
            {
                effective.retain_motion_group(position);
            }
            continue;
        }

        let removed_or_replaced_explicit = old_explicit_for_target.is_some_and(|old| {
            !declarations
                .iter()
                .any(|declaration| declaration.id() == old.declaration.id()
                    && declaration == &old.declaration)
        });
        let explicit_exit_source = if explicit_terminal_commit {
            explicit_sample.clone()
        } else if removed_or_replaced_explicit {
            old_explicit_sample.clone()
        } else {
            None
        };

        let source = explicit_exit_source
            .clone()
            .or_else(|| old_transition_sample.as_ref().map(|sample| sample.value.clone()))
            .or(prior_value)
            .unwrap_or_else(|| target_value.clone());

        let staged_transition = reconcile_transition(
            owner,
            target,
            &source,
            &target_value,
            &provenance,
            &policy,
            old_transition,
            old_transition_sample.as_ref(),
            explicit_exit_source.is_some(),
            preferences,
            instant,
        )?;

        if let Some(transition) = staged_transition.as_ref() {
            let live = sample_transition(transition, instant)?;
            note_live_activity(&live, suppressed, activity);
            next_store.transitions.push(transition.clone());
            if !suppressed && transition_requires_group(transition) {
                effective.retain_motion_group(position);
            }
            if !explicit_terminal_commit && !suppressed {
                effective.apply_motion_value(position, &live.value);
            }
        } else if explicit_terminal_commit {
            if let Some(sample) = explicit_sample.as_ref()
                && !suppressed
            {
                effective.apply_motion_value(position, sample);
            }
            if explicit_sample.as_ref().is_some_and(|sample| sample != &target_value) {
                activity.followup_publication = true;
            }
        } else if let Some(old) = old_transition_sample.as_ref()
            && matches!(old.phase, LivePhase::Completed)
        {
            // The completed transition no longer owns sampled authority.
        }
    }

    for candidate in explicit_evaluations {
        if !next_store
            .explicit
            .iter()
            .any(|record| record.owner == candidate.record.owner
                && record.declaration.id() == candidate.record.declaration.id())
        {
            next_store.explicit.push(candidate.record);
        }
    }

    Ok(())
}

fn reconcile_explicit(
    owner: &MountedNodeId,
    declaration: &ExplicitTimeline,
    retained: Option<&ExplicitMotionRecord>,
    preferences: StylePreferences,
    instant: MonotonicInstant,
) -> Result<ExplicitEvaluation, MotionPlanningError> {
    let mut terminal_commit = false;
    let lifecycle = match retained {
        Some(record) if record.declaration == *declaration => match (&record.lifecycle, preferences.reduced_motion()) {
            (ExplicitLifecycle::Completed, _) => ExplicitLifecycle::Completed,
            (ExplicitLifecycle::HoldInitial, true) => ExplicitLifecycle::HoldInitial,
            (ExplicitLifecycle::HoldInitial, false) => checked_active(declaration, instant)?,
            (ExplicitLifecycle::Active { start }, false) => ExplicitLifecycle::Active { start: *start },
            (ExplicitLifecycle::Active { start }, true) => match declaration.spec().reduced_motion() {
                ReducedMotionStrategy::SnapToEnd => {
                    terminal_commit = true;
                    ExplicitLifecycle::Completed
                }
                ReducedMotionStrategy::HoldInitial => ExplicitLifecycle::HoldInitial,
                ReducedMotionStrategy::PreserveEssential => ExplicitLifecycle::Active { start: *start },
            },
        },
        Some(_) | None => {
            if preferences.reduced_motion() {
                match declaration.spec().reduced_motion() {
                    ReducedMotionStrategy::SnapToEnd => {
                        terminal_commit = true;
                        ExplicitLifecycle::Completed
                    }
                    ReducedMotionStrategy::HoldInitial => ExplicitLifecycle::HoldInitial,
                    ReducedMotionStrategy::PreserveEssential => checked_active(declaration, instant)?,
                }
            } else {
                checked_active(declaration, instant)?
            }
        }
    };

    let mut live_sample = None;
    let candidate_sample = match lifecycle {
        ExplicitLifecycle::Completed if terminal_commit => {
            Some(terminal_keyframe(declaration).clone())
        }
        ExplicitLifecycle::Completed => None,
        ExplicitLifecycle::HoldInitial => Some(initial_keyframe(declaration).clone()),
        ExplicitLifecycle::Active { start } => {
            let sampled = sample_explicit(declaration, start, instant)?;
            let value = sampled.value.clone();
            if matches!(sampled.phase, LivePhase::Completed) {
                terminal_commit = true;
            } else {
                live_sample = Some(sampled.clone());
            }
            Some(value)
        }
    };

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
        live_sample,
    })
}

fn reconcile_transition(
    owner: &MountedNodeId,
    target: MotionTarget,
    source: &MotionValue,
    target_value: &MotionValue,
    provenance: &TargetProvenance,
    policy: &ResolvedTransitionPolicy,
    retained: Option<&TransitionRecord>,
    retained_sample: Option<&LiveSample>,
    explicit_exit: bool,
    preferences: StylePreferences,
    instant: MonotonicInstant,
) -> Result<Option<TransitionRecord>, MotionPlanningError> {
    if explicit_exit {
        return start_transition(
            owner,
            target,
            source,
            target_value,
            provenance,
            policy,
            preferences,
            instant,
        );
    }

    let Some(retained) = retained else {
        return start_transition(
            owner,
            target,
            source,
            target_value,
            provenance,
            policy,
            preferences,
            instant,
        );
    };
    let Some(retained_sample) = retained_sample else {
        return Ok(None);
    };
    if matches!(retained_sample.phase, LivePhase::Completed) {
        return start_transition(
            owner,
            target,
            &retained_sample.value,
            target_value,
            provenance,
            policy,
            preferences,
            instant,
        );
    }

    if preferences.reduced_motion()
        && retained.spec.reduced_motion() == ReducedMotionStrategy::SnapToEnd
    {
        return Ok(None);
    }

    let target_changed = retained.to != *target_value || retained.target_provenance != *provenance;
    match policy {
        ResolvedTransitionPolicy::Disabled => Ok(None),
        ResolvedTransitionPolicy::Absent if !target_changed => Ok(Some(retained.clone())),
        ResolvedTransitionPolicy::Absent => Ok(None),
        ResolvedTransitionPolicy::Enabled(spec) if !target_changed && spec == &retained.spec => {
            Ok(Some(retained.clone()))
        }
        ResolvedTransitionPolicy::Enabled(_) => start_transition(
            owner,
            target,
            &retained_sample.value,
            target_value,
            provenance,
            policy,
            preferences,
            instant,
        ),
    }
}

fn start_transition(
    owner: &MountedNodeId,
    target: MotionTarget,
    source: &MotionValue,
    target_value: &MotionValue,
    provenance: &TargetProvenance,
    policy: &ResolvedTransitionPolicy,
    preferences: StylePreferences,
    instant: MonotonicInstant,
) -> Result<Option<TransitionRecord>, MotionPlanningError> {
    let ResolvedTransitionPolicy::Enabled(spec) = policy else {
        return Ok(None);
    };
    if source == target_value {
        return Ok(None);
    }
    if preferences.reduced_motion()
        && spec.reduced_motion() == ReducedMotionStrategy::SnapToEnd
    {
        return Ok(None);
    }
    check_transition_schedule(spec, instant)?;
    Ok(Some(TransitionRecord {
        owner: owner.clone(),
        target,
        from: source.clone(),
        to: target_value.clone(),
        target_provenance: provenance.clone(),
        spec: spec.clone(),
        start: instant,
    }))
}

fn sample_retained_explicit(
    record: &ExplicitMotionRecord,
    instant: MonotonicInstant,
) -> Result<Option<MotionValue>, MotionPlanningError> {
    match record.lifecycle {
        ExplicitLifecycle::Completed => Ok(None),
        ExplicitLifecycle::HoldInitial => Ok(Some(initial_keyframe(&record.declaration).clone())),
        ExplicitLifecycle::Active { start } => {
            sample_explicit(&record.declaration, start, instant).map(|sample| Some(sample.value))
        }
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
                delay_nanos
                    .checked_add(active_nanos)
                    .unwrap_or_else(|| unreachable!("validated finite schedule remains representable")),
            )?)
        }
        MotionRepeat::Forever => None,
    };

    if instant < delay_deadline {
        return Ok(LiveSample {
            value: initial_keyframe(declaration).clone(),
            phase: LivePhase::Delayed {
                deadline: delay_deadline,
            },
            terminal_deadline,
        });
    }
    if terminal_deadline.is_some_and(|deadline| instant >= deadline) {
        return Ok(LiveSample {
            value: terminal_keyframe(declaration).clone(),
            phase: LivePhase::Completed,
            terminal_deadline,
        });
    }

    let duration_nanos = nanos(spec.duration());
    if duration_nanos == 0 {
        return Ok(LiveSample {
            value: terminal_keyframe(declaration).clone(),
            phase: LivePhase::Completed,
            terminal_deadline,
        });
    }
    let active_nanos = instant
        .as_nanos()
        .checked_sub(delay_deadline.as_nanos())
        .unwrap_or_else(|| unreachable!("sample at/after delay never precedes active start"));
    let iteration_nanos = active_nanos % duration_nanos;
    let progress = normalized_ratio(iteration_nanos, duration_nanos);
    let value = sample_keyframes(declaration, progress)?;
    Ok(LiveSample {
        value,
        phase: LivePhase::Running,
        terminal_deadline,
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
        return Ok(LiveSample {
            value: transition.from.clone(),
            phase: LivePhase::Delayed {
                deadline: delay_deadline,
            },
            terminal_deadline: Some(terminal_deadline),
        });
    }
    if instant >= terminal_deadline || duration_nanos == 0 {
        return Ok(LiveSample {
            value: transition.to.clone(),
            phase: LivePhase::Completed,
            terminal_deadline: Some(terminal_deadline),
        });
    }
    let active_nanos = instant
        .as_nanos()
        .checked_sub(delay_deadline.as_nanos())
        .unwrap_or_else(|| unreachable!("transition sample is at/after active start"));
    let progress = normalized_ratio(active_nanos, duration_nanos);
    let eased = ease_motion(transition.spec.easing(), progress);
    let value = interpolate_motion_value(&transition.from, &transition.to, eased)
        .ok_or(MotionPlanningError::Interpolation(transition.target))?;
    Ok(LiveSample {
        value,
        phase: LivePhase::Running,
        terminal_deadline: Some(terminal_deadline),
    })
}

fn sample_keyframes(
    declaration: &ExplicitTimeline,
    progress: UnitInterval,
) -> Result<MotionValue, MotionPlanningError> {
    let spec = declaration.spec();
    if progress == UnitInterval::ZERO {
        return Ok(initial_keyframe(declaration).clone());
    }
    if progress == UnitInterval::ONE {
        return Ok(terminal_keyframe(declaration).clone());
    }
    if let Some(keyframe) = spec
        .keyframes()
        .iter()
        .find(|keyframe| keyframe.offset() == progress)
    {
        return Ok(keyframe.value().clone());
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
    let progress = f64::from(progress.get());
    let segment_progress = ((progress - left) / (right - left)) as f32;
    let segment_progress = UnitInterval::new(segment_progress)
        .unwrap_or_else(|_| unreachable!("contained segment progress remains normalized"));
    let eased = ease_motion(spec.easings()[segment], segment_progress);
    interpolate_motion_value(pair[0].value(), pair[1].value(), eased)
        .ok_or(MotionPlanningError::Interpolation(spec.target()))
}

fn note_live_activity(
    sample: &LiveSample,
    suppressed: bool,
    activity: &mut MotionActivity,
) {
    if suppressed {
        if let Some(deadline) = sample.terminal_deadline {
            activity.note_deadline(deadline);
        }
        return;
    }
    match sample.phase {
        LivePhase::Delayed { deadline } => activity.note_deadline(deadline),
        LivePhase::Running => activity.continuous_redraw = true,
        LivePhase::Completed => {}
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
                delay_nanos
                    .checked_add(active_nanos)
                    .unwrap_or_else(|| unreachable!("validated finite schedule remains representable")),
            )?;
        }
        MotionRepeat::Forever => {
            checked_add_nanos(instant, delay_nanos)?;
        }
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
        Some(_) => ResolvedTransitionPolicy::Absent,
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
        _ => TargetProvenance::Structural,
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
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use runenui_core::{
        AnimationId, MotionEasing, MotionKeyframe, MotionRepeat, MotionTarget, MotionValue,
        ReducedMotionStrategy, SceneOpacity, TimelineSpec, UnitInterval,
    };

    use super::{
        ExplicitLifecycle, ExplicitMotionRecord, MotionPlanningError, sample_explicit,
        validate_owner_declarations,
    };
    use runenui_core::__runtime::RuntimeNamespace;

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
