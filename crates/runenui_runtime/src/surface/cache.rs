use runenui_core::{SemanticContribution, StyleTokens, WidgetDiagnostic, WidgetPaintProof};

use crate::{AxisConstraints, AxisLimit, LogicalRect, LogicalSize};

use super::{
    SurfaceBuildContext, SurfaceLayoutReport, SurfacePublication,
    resolve::{CachedStyleFacts, SurfaceTopologySnapshot},
};

#[cfg(test)]
std::thread_local! {
    static PHASE_FUNCTION_COUNTS: std::cell::Cell<[usize; 7]> = const {
        std::cell::Cell::new([0; 7])
    };
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfacePhase {
    Tree,
    Style,
    Layout,
    HitTesting,
    Paint,
    Semantics,
    Diagnostics,
    FocusValidation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SurfacePhaseReport {
    executed: Vec<SurfacePhase>,
}

impl SurfacePhaseReport {
    #[must_use]
    pub fn executed(&self) -> &[SurfacePhase] {
        &self.executed
    }
    #[must_use]
    pub fn contains(&self, phase: SurfacePhase) -> bool {
        self.executed.contains(&phase)
    }

    pub(crate) fn one(phase: SurfacePhase) -> Self {
        Self {
            executed: vec![phase],
        }
    }

    pub(super) fn record(&mut self, phase: SurfacePhase) {
        if !self.executed.contains(&phase) {
            self.executed.push(phase);
        }
    }
}

#[cfg(test)]
fn note_phase_function_execution(index: usize) {
    PHASE_FUNCTION_COUNTS.with(|counts| {
        let mut next = counts.get();
        next[index] += 1;
        counts.set(next);
    });
}

#[cfg(test)]
pub(super) fn note_tree_phase_execution() {
    note_phase_function_execution(0);
}

#[cfg(test)]
pub(super) fn note_style_phase_execution() {
    note_phase_function_execution(1);
}

#[cfg(test)]
pub(super) fn note_layout_phase_execution() {
    note_phase_function_execution(2);
}

#[cfg(test)]
pub(super) fn note_hit_test_phase_execution() {
    note_phase_function_execution(3);
}

#[cfg(test)]
pub(super) fn note_paint_phase_execution() {
    note_phase_function_execution(4);
}

#[cfg(test)]
pub(super) fn note_semantics_phase_execution() {
    note_phase_function_execution(5);
}

#[cfg(test)]
pub(super) fn note_diagnostics_phase_execution() {
    note_phase_function_execution(6);
}

#[cfg(test)]
pub(super) fn reset_phase_function_counts() {
    PHASE_FUNCTION_COUNTS.with(|counts| counts.set([0; 7]));
}

#[cfg(test)]
pub(super) fn phase_function_counts() -> [usize; 7] {
    PHASE_FUNCTION_COUNTS.with(std::cell::Cell::get)
}

#[derive(Clone, Debug, Eq, PartialEq)]
// Publication context key: normalized root constraints.
pub(super) struct RootConstraintKey([u32; 4]);

#[derive(Clone, Debug, PartialEq)]
// Publication context key: exact token content plus a diagnostic revision hint.
pub(super) struct StyleTokensCacheKey {
    pub(super) snapshot: StyleTokens,
    pub(super) revision: u64,
}

impl StyleTokensCacheKey {
    pub(super) fn content_differs(&self, other: &Self) -> bool {
        if self.revision == other.revision && self.snapshot == other.snapshot {
            return false;
        }
        self.snapshot != other.snapshot
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SurfaceContextKey {
    // Every field is a context key, not a mounted or phase-owned authored fact.
    pub(super) constraints: RootConstraintKey,
    pub(super) style_tokens: StyleTokensCacheKey,
    pub(super) measurement_identity: u64,
    pub(super) measurement_revision: u64,
}

#[derive(Clone, Debug)]
pub(super) struct CachedLayoutFacts {
    // Layout-phase facts: invalid whenever layout executes.
    pub(super) size: LogicalSize,
    pub(super) bounds: Vec<LogicalRect>,
    pub(super) report: SurfaceLayoutReport,
}

#[derive(Clone, Debug)]
pub(super) struct CachedHitTestFacts {
    // Layout-phase fact projected by the explicit hit-test phase.
    pub(super) bounds: Vec<LogicalRect>,
}

pub(super) fn build_hit_test_facts(layout: &CachedLayoutFacts) -> CachedHitTestFacts {
    #[cfg(test)]
    note_hit_test_phase_execution();
    CachedHitTestFacts {
        bounds: layout.bounds.clone(),
    }
}

#[derive(Clone)]
pub(crate) struct SurfaceCache {
    // Context key.
    pub(super) context_key: SurfaceContextKey,
    // Topology facts.
    pub(super) topology: SurfaceTopologySnapshot,
    // Style-phase facts.
    pub(super) styles: CachedStyleFacts,
    // Layout-phase facts.
    pub(super) layout: CachedLayoutFacts,
    // Layout-phase hit-test projection.
    pub(super) hit_test: CachedHitTestFacts,
    // Paint-phase facts.
    pub(super) paint: Vec<WidgetPaintProof>,
    // Temporary M5A semantic contribution facts. M5B moves semantics into an
    // independent renderer-neutral publication product.
    pub(super) semantics: Vec<SemanticContribution>,
    // Diagnostic-phase facts.
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
    // Derived materialization of the aligned phase facts above, never separate
    // authority. No authored StyleIntent or LayoutStyle is retained here.
    pub(super) publication: SurfacePublication,
}

pub(super) fn context_key(context: &SurfaceBuildContext<'_>) -> SurfaceContextKey {
    const fn axis(axis: AxisConstraints) -> [u32; 2] {
        [
            axis.min().get().to_bits(),
            match axis.max() {
                AxisLimit::Finite(value) => value.get().to_bits(),
                AxisLimit::Unbounded => f32::INFINITY.to_bits(),
            },
        ]
    }
    let horizontal = axis(context.root_constraints().horizontal());
    let vertical = axis(context.root_constraints().vertical());
    SurfaceContextKey {
        constraints: RootConstraintKey([horizontal[0], horizontal[1], vertical[0], vertical[1]]),
        style_tokens: StyleTokensCacheKey {
            snapshot: context.style_tokens().clone(),
            revision: context.style_tokens().revision(),
        },
        measurement_identity: context.measurement_provider().cache_identity(),
        measurement_revision: context.measurement_provider().cache_revision(),
    }
}
