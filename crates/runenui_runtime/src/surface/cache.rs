use std::{collections::HashMap, sync::Arc};

use runenui_core::{
    Axis, LogicalPoint, LogicalTransform, ScrollChrome, ScrollControlBinding,
    ScrollControlSnapshot, StyleEnvironment, SurfacePresentation, TextDocumentSnapshot,
    WidgetDiagnostic,
};
use runenui_text::{
    FontSourceSnapshot, TextCaretMap, TextCaretMapError, TextDisplaySelection, TextLayoutState,
    TextPreeditProjection,
};

use crate::{AxisConstraints, AxisLimit, LogicalRect, LogicalSize, MountedNodeId};
use crate::{
    editing::EditingSemanticProjection,
    scene::{HitTestSceneContent, PaintScene, SceneClip},
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct EditingPaintIdentity {
    snapshot: TextDocumentSnapshot,
    selection: runenui_core::TextSelection,
    sensitivity: runenui_core::TextSensitivity,
}

#[derive(Clone, Copy)]
pub(crate) struct TextEditingPaintInputs<'a> {
    pub(super) focused_owner: Option<&'a MountedNodeId>,
    pub(super) editing: &'a HashMap<MountedNodeId, EditingSemanticProjection>,
    pub(super) preedits: &'a HashMap<MountedNodeId, Arc<TextPreeditProjection>>,
}

impl<'a> TextEditingPaintInputs<'a> {
    pub(crate) const fn new(
        focused_owner: Option<&'a MountedNodeId>,
        editing: &'a HashMap<MountedNodeId, EditingSemanticProjection>,
        preedits: &'a HashMap<MountedNodeId, Arc<TextPreeditProjection>>,
    ) -> Self {
        Self {
            focused_owner,
            editing,
            preedits,
        }
    }
}

/// Cache-compatibility view of runtime-owned editing paint inputs.
///
/// It retains no document source bytes; source identity is revision-scoped, and
/// preedit projections are shared immutable runtime values rather than copies.
#[derive(Clone, Default, PartialEq)]
pub(super) struct TextEditingPaintKey {
    focused_owner: Option<MountedNodeId>,
    editing: HashMap<MountedNodeId, EditingPaintIdentity>,
    preedits: HashMap<MountedNodeId, Arc<TextPreeditProjection>>,
}

impl TextEditingPaintKey {
    /// Security-classification changes must never retain the previous M8
    /// shaped source in a paint-only surface update.
    pub(super) fn secret_layout_changed(&self, next: &Self) -> bool {
        use runenui_core::TextSensitivity;
        self.editing.iter().any(|(owner, old)| {
            let current = next.editing.get(owner).map(|item| item.sensitivity);
            current != Some(old.sensitivity)
                && (old.sensitivity == TextSensitivity::Secret
                    || current == Some(TextSensitivity::Secret))
        }) || next.editing.iter().any(|(owner, current)| {
            current.sensitivity == TextSensitivity::Secret
                && !self.editing.contains_key(owner)
        })
    }

    pub(super) fn new(inputs: TextEditingPaintInputs<'_>) -> Self {
        Self {
            focused_owner: inputs.focused_owner.cloned(),
            editing: inputs
                .editing
                .iter()
                .map(|(owner, projection)| {
                    (
                        owner.clone(),
                        EditingPaintIdentity {
                            snapshot: projection.snapshot,
                            selection: projection.selection,
                            sensitivity: projection.sensitivity,
                        },
                    )
                })
                .collect(),
            preedits: inputs.preedits.clone(),
        }
    }
}

use super::{
    SurfaceBuildContext, SurfaceInteractionProjection, SurfaceLayoutReport,
    SurfacePresentationSnapshot, SurfacePublication,
    resolve::{
        CachedEffectiveFacts, CachedStyleFacts, SurfaceTopologySnapshot, displayed_scroll_metrics,
    },
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
// Publication context key: complete exact style-environment content.
pub(super) struct StyleEnvironmentCacheKey {
    pub(super) snapshot: StyleEnvironment,
}

impl StyleEnvironmentCacheKey {
    pub(super) fn content_differs(&self, other: &Self) -> bool {
        self.snapshot != other.snapshot
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SurfaceContextKey {
    // Every field is a context key, not a mounted or phase-owned authored fact.
    pub(super) constraints: RootConstraintKey,
    pub(super) style_environment: StyleEnvironmentCacheKey,
    pub(super) font_source: FontSourceSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CachedScrollChromeKind {
    Bar {
        owner_position: usize,
        axis: Axis,
    },
    Thumb {
        owner_position: usize,
        axis: Axis,
        track_position: usize,
    },
    Corner {
        owner_position: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CachedScrollChromeProjection {
    kind: CachedScrollChromeKind,
    present: bool,
}

impl CachedScrollChromeProjection {
    #[must_use]
    pub(super) const fn bar(owner_position: usize, axis: Axis, present: bool) -> Self {
        Self {
            kind: CachedScrollChromeKind::Bar {
                owner_position,
                axis,
            },
            present,
        }
    }

    #[must_use]
    pub(super) const fn thumb(
        owner_position: usize,
        axis: Axis,
        track_position: usize,
        present: bool,
    ) -> Self {
        Self {
            kind: CachedScrollChromeKind::Thumb {
                owner_position,
                axis,
                track_position,
            },
            present,
        }
    }

    #[must_use]
    pub(super) const fn corner(owner_position: usize, present: bool) -> Self {
        Self {
            kind: CachedScrollChromeKind::Corner { owner_position },
            present,
        }
    }

    #[must_use]
    pub(super) const fn kind(self) -> CachedScrollChromeKind {
        self.kind
    }

    #[must_use]
    pub(super) const fn owner_position(self) -> usize {
        match self.kind {
            CachedScrollChromeKind::Bar { owner_position, .. }
            | CachedScrollChromeKind::Thumb { owner_position, .. }
            | CachedScrollChromeKind::Corner { owner_position } => owner_position,
        }
    }

    #[must_use]
    pub(super) const fn present(self) -> bool {
        self.present
    }
}

#[derive(Clone, Debug)]
pub(super) struct CachedLayoutFacts {
    // Layout-phase facts: invalid whenever layout executes.
    pub(super) size: LogicalSize,
    pub(super) bounds: Vec<LogicalRect>,
    pub(super) report: SurfaceLayoutReport,
    // Exact layout-owned scroll-chrome validity/presence aligned with topology.
    // Authored chrome with no entry failed structural validation and is
    // non-participating in every downstream phase.
    pub(super) scroll_chrome: Vec<Option<CachedScrollChromeProjection>>,
    // Runtime-owned reusable logical text state aligned exactly with topology.
    // Each state is cheap COW sharing so a staged reflow cannot mutate accepted
    // shaping/layout state before publication commit.
    pub(super) text_layouts: Vec<TextLayoutState>,
    // One final-layout origin for every retained text artifact, topology-aligned.
    pub(super) text_origins: Vec<LogicalPoint>,
}

/// One runtime-owned node presentation fact in mounted-preorder alignment.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PresentationNodeFacts {
    owner_to_surface: LogicalTransform,
    content_to_surface: LogicalTransform,
    owner_bounds: LogicalRect,
    visible_bounds: LogicalRect,
    inherited_clips: Arc<[SceneClip]>,
    content_clips: Arc<[SceneClip]>,
    published: bool,
    stack_root: Option<usize>,
    root_snapshot: Option<SurfacePresentationSnapshot>,
    diagnostics: Arc<[WidgetDiagnostic]>,
}

pub(super) struct PresentationNodeFactsInit {
    pub(super) owner_to_surface: LogicalTransform,
    pub(super) content_to_surface: LogicalTransform,
    pub(super) owner_bounds: LogicalRect,
    pub(super) visible_bounds: LogicalRect,
    pub(super) inherited_clips: Arc<[SceneClip]>,
    pub(super) content_clips: Arc<[SceneClip]>,
    pub(super) published: bool,
    pub(super) stack_root: Option<usize>,
    pub(super) root_snapshot: Option<SurfacePresentationSnapshot>,
    pub(super) diagnostics: Arc<[WidgetDiagnostic]>,
}

impl PresentationNodeFacts {
    #[must_use]
    pub(super) fn new(init: PresentationNodeFactsInit) -> Self {
        Self {
            owner_to_surface: init.owner_to_surface,
            content_to_surface: init.content_to_surface,
            owner_bounds: init.owner_bounds,
            visible_bounds: init.visible_bounds,
            inherited_clips: init.inherited_clips,
            content_clips: init.content_clips,
            published: init.published,
            stack_root: init.stack_root,
            root_snapshot: init.root_snapshot,
            diagnostics: init.diagnostics,
        }
    }

    #[must_use]
    pub(super) const fn owner_to_surface(&self) -> LogicalTransform {
        self.owner_to_surface
    }

    #[must_use]
    pub(super) const fn content_to_surface(&self) -> LogicalTransform {
        self.content_to_surface
    }

    #[must_use]
    pub(super) const fn owner_bounds(&self) -> LogicalRect {
        self.owner_bounds
    }

    #[must_use]
    pub(super) const fn visible_bounds(&self) -> LogicalRect {
        self.visible_bounds
    }

    #[must_use]
    pub(super) const fn published(&self) -> bool {
        self.published
    }

    #[must_use]
    pub(super) const fn stack_root(&self) -> Option<usize> {
        self.stack_root
    }

    #[must_use]
    pub(super) const fn root_snapshot(&self) -> Option<SurfacePresentationSnapshot> {
        self.root_snapshot
    }

    #[must_use]
    pub(super) fn diagnostics(&self) -> &[WidgetDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub(super) fn inherited_clips(&self) -> &[SceneClip] {
        &self.inherited_clips
    }

    #[must_use]
    pub(super) fn content_clips(&self) -> &[SceneClip] {
        &self.content_clips
    }
}

/// Correlated runtime presentation geometry aligned exactly with topology.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CachedPresentationFacts {
    pub(super) nodes: Vec<PresentationNodeFacts>,
}

impl CachedPresentationFacts {
    #[must_use]
    pub(super) fn node(&self, position: usize) -> &PresentationNodeFacts {
        self.nodes
            .get(position)
            .unwrap_or_else(|| unreachable!("presentation facts remain topology-aligned"))
    }

    #[must_use]
    pub(super) fn published(&self, position: usize) -> bool {
        self.node(position).published()
    }

    #[must_use]
    pub(super) fn stack_root(&self, position: usize) -> Option<usize> {
        self.node(position).stack_root()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PresentationInteractionRoot {
    pub(crate) root: MountedNodeId,
    pub(crate) owner: Option<MountedNodeId>,
    pub(crate) presentation: SurfacePresentation,
}

/// Exact derived binding for one scroll-control owner in the accepted surface projection.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CachedScrollControlProjection {
    pub(super) owner: MountedNodeId,
    pub(super) binding: ScrollControlBinding,
    pub(super) snapshot: ScrollControlSnapshot,
}

/// Sole retained renderer/input-side publication substrate.
///
/// Every phase product is immutable once retained. Non-structural planning
/// stages by cloning these handles and replaces only products owned by phases
/// that actually execute. Canonical paint and hit scene content are retained
/// directly; there is no proof-era paint vector or layout-derived hit snapshot.
pub(crate) struct SurfaceCache {
    // Context key.
    pub(super) context_key: Arc<SurfaceContextKey>,
    // Topology facts.
    pub(super) topology: Arc<SurfaceTopologySnapshot>,
    // Last runtime-derived interaction projection consumed by the style phase.
    // This is cache compatibility only, never pointer/focus authority.
    pub(super) interaction: Arc<SurfaceInteractionProjection>,
    // Exact derived editing inputs used to decide whether transient caret,
    // selection, or preedit paint must be recomputed.
    pub(super) text_editing: Arc<TextEditingPaintKey>,
    // Mounted logical scroll offsets consumed by the correlated presentation,
    // clip, physical-hit and semantic geometry products.
    pub(super) scroll: Arc<super::SurfaceScrollProjection>,
    // Exact topology-aligned derived control -> scroll-owner bindings for this
    // accepted surface projection. This is publication cache, never scroll state.
    pub(super) scroll_controls: Arc<Vec<Option<CachedScrollControlProjection>>>,
    // Target style/provenance facts. Motion never rewrites these.
    pub(super) styles: Arc<CachedStyleFacts>,
    // Accepted effective style/layout values consumed by downstream phases.
    // This is a derived publication snapshot, never authored-state authority.
    pub(super) effective: Arc<CachedEffectiveFacts>,
    // Layout-phase facts: logical layout authority only.
    pub(super) layout: Arc<CachedLayoutFacts>,
    // Runtime-owned presentation geometry derived from final layout + effective style.
    pub(super) presentation: Arc<CachedPresentationFacts>,
    // Canonical physical-hit content; displayed context is added only by the
    // runtime-owned publication state when a generation is committed.
    pub(super) hit_test: HitTestSceneContent,
    // Canonical renderer-neutral paint scene content.
    pub(super) paint: PaintScene,
    // Widget diagnostic-phase facts.
    pub(super) diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Hit-composition diagnostics are owned and replaced with the hit phase.
    pub(super) hit_diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Paint-composition diagnostics are owned and replaced with the paint phase.
    pub(super) paint_diagnostics: Arc<Vec<Vec<WidgetDiagnostic>>>,
    // Derived layout/debug materialization of aligned phase facts above, never
    // renderer, pointer, authored-style, or authored-layout authority. Its clone
    // is cheap immutable sharing.
    pub(super) publication: SurfacePublication,
}

impl SurfaceCache {
    /// Returns published presentation roots from visual topmost to bottommost.
    ///
    /// Ordering is derived from the accepted #341 mounted-preorder presentation band;
    /// this retains no separate "latest popup" state.
    pub(crate) fn presentation_interaction_roots(&self) -> Vec<PresentationInteractionRoot> {
        self.topology
            .nodes
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(position, node)| {
                let presentation = node.surface_presentation.as_ref()?;
                (self.presentation.published(position)
                    && self.presentation.stack_root(position) == Some(position))
                .then(|| PresentationInteractionRoot {
                    root: node.id.clone(),
                    owner: node.parent.clone(),
                    presentation: presentation.clone(),
                })
            })
            .collect()
    }

    pub(crate) fn scroll_target_geometry(
        &self,
        target: &MountedNodeId,
        owner: &MountedNodeId,
    ) -> Option<(LogicalRect, LogicalSize)> {
        if target == owner {
            return None;
        }
        let target_position = self.topology.position(target)?;
        let owner_position = self.topology.position(owner)?;
        let target_presentation = self.presentation.node(target_position);
        let owner_presentation = self.presentation.node(owner_position);
        let surface_to_owner = owner_presentation.owner_to_surface().inverse()?;
        let target_layout_bounds = self.layout.bounds.get(target_position)?;
        let target_local_bounds = LogicalRect::try_new(
            0.0,
            0.0,
            target_layout_bounds.width(),
            target_layout_bounds.height(),
        )
        .ok()?;
        let target_to_owner = target_presentation
            .owner_to_surface()
            .then(surface_to_owner)
            .ok()?;
        let target_bounds =
            runenui_core::__runtime::transform_rect_aabb(target_to_owner, target_local_bounds)?;
        let offset = self.scroll.offset(owner);
        let content_bounds = LogicalRect::try_new(
            target_bounds.x() + offset.0,
            target_bounds.y() + offset.1,
            target_bounds.width(),
            target_bounds.height(),
        )
        .ok()?;
        let owner_layout = self
            .layout
            .report
            .nodes()
            .get(owner_position)
            .filter(|layout_node| layout_node.id() == owner)?;
        Some((content_bounds, owner_layout.scroll_viewport_extent()))
    }

    pub(crate) fn current_scroll_chrome_participation(
        &self,
        target: &MountedNodeId,
        authored: ScrollChrome,
    ) -> Option<bool> {
        let position = self.topology.position(target)?;
        if self.topology.nodes.get(position)?.scroll_chrome != Some(authored) {
            return None;
        }
        Some(
            self.layout
                .scroll_chrome
                .get(position)
                .copied()
                .flatten()
                .is_some_and(CachedScrollChromeProjection::present),
        )
    }

    pub(crate) fn current_scroll_control_projection(
        &self,
        target: &MountedNodeId,
    ) -> super::ScrollControlProjectionLookup {
        let Some(position) = self.topology.position(target) else {
            return super::ScrollControlProjectionLookup::Unavailable;
        };
        let Some(projection) = self.scroll_controls.get(position) else {
            return super::ScrollControlProjectionLookup::Unavailable;
        };
        projection.as_ref().map_or(
            super::ScrollControlProjectionLookup::Unbound,
            |projection| super::ScrollControlProjectionLookup::Bound {
                owner: projection.owner.clone(),
                binding: projection.binding,
                snapshot: projection.snapshot,
            },
        )
    }

    pub(crate) fn current_scroll_metrics(
        &self,
        owner: &MountedNodeId,
    ) -> Option<super::DisplayedScrollMetrics> {
        displayed_scroll_metrics(&self.topology, &self.layout, owner)
    }

    pub(crate) fn text_caret_map(
        &self,
        owner: &MountedNodeId,
        snapshot: TextDocumentSnapshot,
        source: &str,
    ) -> Result<TextCaretMap, TextCaretMapError> {
        let position = self
            .topology
            .position(owner)
            .ok_or(TextCaretMapError::MissingLayout)?;
        self.layout
            .text_layouts
            .get(position)
            .ok_or(TextCaretMapError::MissingLayout)?
            .caret_map_for_source(snapshot, source)
    }

    pub(crate) fn text_candidate_area(
        &self,
        owner: &MountedNodeId,
        snapshot: TextDocumentSnapshot,
        source: &str,
        selection: runenui_core::TextSelection,
        preedit: Option<Arc<TextPreeditProjection>>,
    ) -> Result<LogicalRect, TextCaretMapError> {
        let position = self
            .topology
            .position(owner)
            .ok_or(TextCaretMapError::MissingLayout)?;
        let layout = self
            .layout
            .text_layouts
            .get(position)
            .ok_or(TextCaretMapError::MissingLayout)?;
        let (map, active) = if let Some(preedit) = preedit {
            let map = layout.preedit_caret_map(preedit)?;
            let active = map
                .preedit_selection()?
                .map(|display| display.active().clone())
                .or_else(|| {
                    let projection = map.preedit_projection()?;
                    projection
                        .position_from_display_offset(
                            projection.display_preedit_end(),
                            runenui_core::TextAffinity::Upstream,
                        )
                        .ok()
                })
                .ok_or(TextCaretMapError::DisplayTextMismatch)?;
            (map, active)
        } else {
            let map = layout.caret_map_for_source(snapshot, source)?;
            let active = TextDisplaySelection::from_document(selection)
                .active()
                .clone();
            (map, active)
        };
        let local = map.candidate_rect(&active)?;
        let presentation = self.presentation.node(position);
        if !presentation.published() {
            return Err(TextCaretMapError::InvalidGeometry);
        }
        let origin = self
            .layout
            .text_origins
            .get(position)
            .ok_or(TextCaretMapError::MissingLayout)?;
        let text_origin = LogicalTransform::translation(origin.x(), origin.y())
            .map_err(|_| TextCaretMapError::InvalidGeometry)?;
        let text_to_surface = text_origin
            .then(presentation.content_to_surface())
            .map_err(|_| TextCaretMapError::InvalidGeometry)?;
        runenui_core::__runtime::transform_rect_aabb(text_to_surface, local)
            .ok_or(TextCaretMapError::InvalidGeometry)
    }

    /// Creates a staged non-structural candidate by sharing every retained
    /// product. Dirty phase execution must replace the corresponding product
    /// explicitly before this candidate can commit.
    pub(super) fn staged(&self) -> Self {
        Self {
            context_key: Arc::clone(&self.context_key),
            topology: Arc::clone(&self.topology),
            interaction: Arc::clone(&self.interaction),
            text_editing: Arc::clone(&self.text_editing),
            scroll: Arc::clone(&self.scroll),
            scroll_controls: Arc::clone(&self.scroll_controls),
            styles: Arc::clone(&self.styles),
            effective: Arc::clone(&self.effective),
            layout: Arc::clone(&self.layout),
            presentation: Arc::clone(&self.presentation),
            hit_test: self.hit_test.clone(),
            paint: self.paint.clone(),
            diagnostics: Arc::clone(&self.diagnostics),
            hit_diagnostics: Arc::clone(&self.hit_diagnostics),
            paint_diagnostics: Arc::clone(&self.paint_diagnostics),
            publication: self.publication.clone(),
        }
    }

    /// Projects current directional-focus geometry from the correlated retained
    /// presentation product, independently of physical hit participation.
    pub(crate) fn current_focus_geometry(&self) -> Vec<(MountedNodeId, LogicalRect)> {
        self.topology
            .nodes
            .iter()
            .zip(&self.presentation.nodes)
            .filter(|(_, presentation)| presentation.published())
            .map(|(node, presentation)| (node.id.clone(), presentation.visible_bounds()))
            .collect()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn replace_focus_geometry_for_test(
        &mut self,
        geometry: &[(MountedNodeId, LogicalRect)],
    ) {
        let presentation = Arc::make_mut(&mut self.presentation);
        for (id, bounds) in geometry {
            let position = self
                .topology
                .position(id)
                .unwrap_or_else(|| unreachable!("test geometry names a published node"));
            let current = presentation.nodes[position].clone();
            presentation.nodes[position] = PresentationNodeFacts::new(PresentationNodeFactsInit {
                owner_to_surface: current.owner_to_surface(),
                content_to_surface: current.content_to_surface(),
                owner_bounds: current.owner_bounds,
                visible_bounds: *bounds,
                inherited_clips: Arc::clone(&current.inherited_clips),
                content_clips: Arc::clone(&current.content_clips),
                published: current.published,
                stack_root: current.stack_root,
                root_snapshot: current.root_snapshot,
                diagnostics: Arc::clone(&current.diagnostics),
            });
        }
    }

    #[cfg(test)]
    pub(super) fn retained_product_reuse(&self, other: &Self) -> [bool; 7] {
        [
            Arc::ptr_eq(&self.topology, &other.topology),
            Arc::ptr_eq(&self.styles, &other.styles),
            Arc::ptr_eq(&self.layout, &other.layout)
                && Arc::ptr_eq(&self.presentation, &other.presentation),
            self.hit_test.shares_storage_with(&other.hit_test),
            self.paint.shares_storage_with(&other.paint),
            Arc::ptr_eq(&self.diagnostics, &other.diagnostics)
                && Arc::ptr_eq(&self.hit_diagnostics, &other.hit_diagnostics)
                && Arc::ptr_eq(&self.paint_diagnostics, &other.paint_diagnostics),
            self.publication.shares_storage_with(&other.publication),
        ]
    }
}

pub(super) fn context_key(
    context: &SurfaceBuildContext<'_>,
    font_source: FontSourceSnapshot,
) -> SurfaceContextKey {
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
        style_environment: StyleEnvironmentCacheKey {
            snapshot: context.style_environment().clone(),
        },
        font_source,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use runenui_core::{StyleEnvironment, View, WidgetInvalidation, text};

    use super::{SurfaceCache, SurfacePhase};
    use crate::{
        LayoutConstraints,
        mounted::{DirtyPhases, MountedTree, apply_invalidation},
        surface::{
            SurfaceBuildContext, SurfaceInteractionProjection, SurfaceMotionStore,
            planning::plan_mounted_surface_cached_with_test_text,
        },
    };

    fn publish(
        tree: &mut MountedTree<()>,
        context: &SurfaceBuildContext<'_>,
        cache: &mut Option<SurfaceCache>,
    ) -> super::SurfacePhaseReport {
        let interaction = SurfaceInteractionProjection::default();
        let planned =
            plan_mounted_surface_cached_with_test_text(tree, context, &interaction, cache.as_ref())
                .unwrap_or_else(|_| unreachable!("reuse proof has valid semantic planning"));
        let commit = planned.commit_store();
        let mut motion_store = SurfaceMotionStore::default();
        let (_, report, _activity) = commit.commit(tree, cache, &mut motion_store);
        report
    }

    fn retained(cache: Option<&SurfaceCache>) -> SurfaceCache {
        cache
            .unwrap_or_else(|| unreachable!("initial publication retains a cache"))
            .staged()
    }

    #[test]
    fn effective_facts_start_as_exact_target_projection() {
        let (mut tree, _) = MountedTree::<()>::mount(text("effective").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let cache = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("initial publication retains a cache"));

        assert_eq!(cache.effective.nodes.len(), cache.topology.nodes.len());
        for (index, topology) in cache.topology.nodes.iter().enumerate() {
            let mounted = tree
                .node(&topology.id)
                .unwrap_or_else(|| unreachable!("published topology remains mounted"));
            let effective = cache.effective.node(index);
            assert_eq!(effective.layout(), &mounted.layout);
            assert_eq!(
                effective.computed_style(),
                cache.styles.resolutions[index].computed_style()
            );
            assert!(!effective.retain_node_effect_group());
        }
    }

    #[test]
    fn focus_only_publication_reuses_all_renderer_products() {
        let (mut tree, _) = MountedTree::<()>::mount(text("focus").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let before = retained(cache.as_ref());
        let effective_before = Arc::clone(&before.effective);

        tree.mark_runtime_semantic_product_dirty();
        let report = publish(&mut tree, &context, &mut cache);
        let after = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("focus publication retains a cache"));

        assert_eq!(report.executed(), &[SurfacePhase::Semantics]);
        assert_eq!(before.retained_product_reuse(after), [true; 7]);
        assert!(Arc::ptr_eq(&effective_before, &after.effective));
    }

    #[test]
    fn dropped_dirty_non_structural_plan_leaves_live_cache_and_dirty_work_unchanged() {
        let (mut tree, _) = MountedTree::<()>::mount(text("rollback").key("root").into_element());
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let mut cache = None;
        let _ = publish(&mut tree, &context, &mut cache);
        let root = tree.publication_preorder_ids()[0].clone();
        let before = retained(cache.as_ref());

        let node = tree
            .node_mut(&root)
            .unwrap_or_else(|| unreachable!("rollback proof root remains live"));
        apply_invalidation(node, WidgetInvalidation::PAINT);
        let dirty_before = tree.pending_phases();
        assert!(dirty_before.contains(DirtyPhases::PAINT));

        let interaction = SurfaceInteractionProjection::default();
        let planned = plan_mounted_surface_cached_with_test_text(
            &mut tree,
            &context,
            &interaction,
            cache.as_ref(),
        )
        .unwrap_or_else(|_| unreachable!("dirty staged plan remains valid"));
        drop(planned);

        let still_live = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("dropped plan leaves live cache retained"));
        assert_eq!(before.retained_product_reuse(still_live), [true; 7]);
        assert_eq!(tree.pending_phases(), dirty_before);

        let report = publish(&mut tree, &context, &mut cache);
        assert_eq!(report.executed(), &[SurfacePhase::Paint]);
        let after = cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("successful retry retains a cache"));
        assert_eq!(
            before.retained_product_reuse(after),
            [true, true, true, true, false, true, true]
        );
    }
}
