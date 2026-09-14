use runenui_core::{AnimationId, MotionTarget, ReducedMotionStrategy};

/// Authored/runtime motion source represented by one canonical trace fact.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceMotionSource {
    Transition,
    Timeline { animation_id: AnimationId },
}

impl TraceMotionSource {
    /// Returns the owner-local authored animation identity for explicit timelines.
    #[must_use]
    pub const fn animation_id(&self) -> Option<&AnimationId> {
        match self {
            Self::Transition => None,
            Self::Timeline { animation_id } => Some(animation_id),
        }
    }
}

/// Resolved target-keyed transition policy observed by motion reconciliation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionPolicy {
    Absent,
    Disabled,
    Enabled,
}

/// Invalid authored explicit-timeline collision rejected before mutation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionCollision {
    DuplicateAnimationId,
    DuplicateTarget,
}

/// Runtime lifecycle decision for one exact motion source/target lifetime.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionLifecycle {
    Started,
    Replaced,
    Cancelled,
    Restarted,
    Completed,
    CompletedRetained,
    HoldInitialEntered,
    HoldInitialReleased,
}

/// Sampling phase observed at the single staged publication instant.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionPhase {
    Delayed,
    Running,
    Completed,
    HeldInitial,
}

/// Framework-owned endpoint compatibility decision used for one sample.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionInterpolation {
    Endpoint,
    Continuous,
    Discrete,
}

/// Mandatory preference decision applied above a sampled motion source.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionPreferenceDecision {
    Normal,
    SnapToEnd,
    HoldInitial,
    PreserveEssential,
    HighContrastSuppressed,
}

/// Checked staged motion-planning rejection before live motion/publication mutation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceMotionPlanningRejection {
    ScheduleOverflow,
    Interpolation,
}

/// Direct invalidation/cache decision derived from one staged sampled target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceMotionEffectDecision {
    layout: bool,
    presentation: bool,
    paint: bool,
    retain_node_effect_group: bool,
    effective_changed: bool,
}

impl TraceMotionEffectDecision {
    pub(crate) const fn new(
        layout: bool,
        presentation: bool,
        paint: bool,
        retain_node_effect_group: bool,
        effective_changed: bool,
    ) -> Self {
        Self {
            layout,
            presentation,
            paint,
            retain_node_effect_group,
            effective_changed,
        }
    }

    #[must_use]
    pub const fn layout(self) -> bool {
        self.layout
    }

    #[must_use]
    pub const fn presentation(self) -> bool {
        self.presentation
    }

    #[must_use]
    pub const fn paint(self) -> bool {
        self.paint
    }

    #[must_use]
    pub const fn retain_node_effect_group(self) -> bool {
        self.retain_node_effect_group
    }

    #[must_use]
    pub const fn effective_changed(self) -> bool {
        self.effective_changed
    }
}

/// One typed immutable motion observation carried by the canonical runtime trace.
///
/// This is diagnostic projection only. It cannot start, advance, cancel, or identify
/// live motion independently from runtime-owned mounted/source state.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceMotionFact {
    PolicyResolved {
        policy: TraceMotionPolicy,
    },
    CollisionRejected {
        source: TraceMotionSource,
        collision: TraceMotionCollision,
    },
    Lifecycle {
        source: TraceMotionSource,
        lifecycle: TraceMotionLifecycle,
    },
    Sampled {
        source: TraceMotionSource,
        phase: TraceMotionPhase,
        progress_bits: Option<u32>,
        eased_progress_bits: Option<u32>,
        interpolation: TraceMotionInterpolation,
        suppressed: bool,
    },
    Preference {
        source: TraceMotionSource,
        reduced_motion: bool,
        strategy: ReducedMotionStrategy,
        decision: TraceMotionPreferenceDecision,
    },
    Effect {
        decision: TraceMotionEffectDecision,
    },
    PlanningRejected {
        source: Option<TraceMotionSource>,
        rejection: TraceMotionPlanningRejection,
    },
}

impl TraceMotionFact {
    /// Returns the exact motion source when this fact is source-specific.
    #[must_use]
    pub const fn source(&self) -> Option<&TraceMotionSource> {
        match self {
            Self::CollisionRejected { source, .. }
            | Self::Lifecycle { source, .. }
            | Self::Sampled { source, .. }
            | Self::Preference { source, .. } => Some(source),
            Self::PlanningRejected { source, .. } => source.as_ref(),
            Self::PolicyResolved { .. } | Self::Effect { .. } => None,
        }
    }
}

/// One target-scoped canonical motion fact before trace sequence assignment.
///
/// Runtime surface planning produces these ephemerally. They are not retained motion
/// state and become observable only through the existing canonical trace transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StagedMotionTraceFact {
    pub(crate) owner: crate::MountedNodeId,
    pub(crate) authored_id: Option<runenui_core::ElementId>,
    pub(crate) target: MotionTarget,
    pub(crate) fact: TraceMotionFact,
}

impl StagedMotionTraceFact {
    pub(crate) const fn new(
        owner: crate::MountedNodeId,
        authored_id: Option<runenui_core::ElementId>,
        target: MotionTarget,
        fact: TraceMotionFact,
    ) -> Self {
        Self {
            owner,
            authored_id,
            target,
            fact,
        }
    }
}
