use std::{collections::HashMap, sync::Arc};

mod explicit_groups;
mod groups;
mod image_mapping;
mod presentation;

pub(super) use presentation::resolve_presentation;

use crate::MountedNodeId;
use crate::mounted::SurfaceCapabilityPlan;
use crate::scene::{HitTestRegion, HitTestSceneContent, PaintScene, PaintSceneItem, SceneClip};
use crate::style_debug::{SurfaceStyleNode, SurfaceStyleReport};
use runenui_core::{
    __runtime::transform_rect_aabb, Axis, Color, ComputedStyle, ContributionClip, ElementId,
    HitContributionContext, LayoutStyle, LogicalLength, LogicalPoint, LogicalRect,
    LogicalTransform, OverflowPolicy, OverflowStyle, PaintContribution, PaintContributionContext,
    PaintContributionItem, Radius, SceneShape, ScrollBarLayout, ScrollChrome,
    ScrollControlSnapshot, SemanticContributionContext, StyleEnvironment, StyleInteractionState,
    StyleResolution, SurfacePresentation, TextAffinity, WidgetDiagnostic, WidgetTypeId,
    resolve_style_in_environment, style_effects_between,
};
use runenui_text::{ShapedTextLease, TextDisplaySelection, TextPreeditProjection, TextSystem};

use super::{
    DisplayedScrollMetrics, SurfaceInteractionProjection, SurfaceScrollProjection,
    cache::{
        CachedLayoutFacts, CachedPresentationFacts, CachedScrollChromeKind,
        CachedScrollChromeProjection, CachedScrollControlProjection, PresentationNodeFacts,
        PresentationNodeFactsInit, TextEditingPaintInputs,
    },
};

/// Topology and publication-alignment facts for one mounted preorder.
///
/// Mutable authored style and layout remain owned only by mounted nodes.
#[derive(Clone, Debug)]
pub(super) struct SurfaceTopologySnapshot {
    pub(super) nodes: Vec<SurfaceTopologyNode>,
    positions: HashMap<MountedNodeId, usize>,
}

impl SurfaceTopologySnapshot {
    pub(super) fn position(&self, id: &MountedNodeId) -> Option<usize> {
        self.positions.get(id).copied()
    }
}

#[derive(Clone, Debug)]
pub(super) struct SurfaceTopologyNode {
    pub(super) id: MountedNodeId,
    pub(super) parent: Option<MountedNodeId>,
    pub(super) authored_id: Option<ElementId>,
    pub(super) widget_type_id: WidgetTypeId,
    pub(super) children: Vec<MountedNodeId>,
    pub(super) overflow: OverflowStyle,
    pub(super) scroll_chrome: Option<ScrollChrome>,
    pub(super) surface_presentation: Option<SurfacePresentation>,
}

pub(super) fn collect_topology<Action>(
    tree: &crate::mounted::MountedTree<Action>,
) -> SurfaceTopologySnapshot {
    #[cfg(test)]
    super::cache::note_tree_phase_execution();
    let nodes = tree
        .publication_preorder_ids()
        .into_iter()
        .map(|id| {
            let node = tree
                .node(&id)
                .unwrap_or_else(|| unreachable!("publication preorder remains live"));
            SurfaceTopologyNode {
                id: node.id.clone(),
                parent: node.parent.clone(),
                authored_id: node.authored_id.clone(),
                widget_type_id: node.widget.widget_type_id(),
                children: node.children.clone(),
                overflow: node.layout.overflow(),
                scroll_chrome: node.scroll_chrome,
                surface_presentation: node.surface_presentation.clone(),
            }
        })
        .collect::<Vec<_>>();
    let positions = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.clone(), position))
        .collect();
    SurfaceTopologySnapshot { nodes, positions }
}

#[derive(Clone, Debug)]
pub(super) struct CachedStyleFacts {
    // Target style/provenance facts aligned to the topology snapshot. They are
    // refreshed whenever mounted style intent or exact style-environment content changes.
    pub(super) resolutions: Vec<StyleResolution>,
    pub(super) report: SurfaceStyleReport,
}

/// Direct downstream invalidation caused by changing one accepted effective snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EffectiveEffects {
    layout: bool,
    paint: bool,
    presentation: bool,
}

impl EffectiveEffects {
    pub(super) const fn layout(self) -> bool {
        self.layout
    }

    pub(super) const fn paint(self) -> bool {
        self.paint
    }

    pub(super) const fn presentation(self) -> bool {
        self.presentation
    }
}

/// One topology-aligned effective publication input.
///
/// Target style provenance and authored layout remain owned by their existing
/// authorities. This snapshot is the sole downstream value projection that motion
/// may replace during staged publication.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct EffectiveNodeFacts {
    layout: LayoutStyle,
    computed_style: ComputedStyle,
    retain_node_effect_group: bool,
}

impl EffectiveNodeFacts {
    pub(super) const fn new(
        layout: LayoutStyle,
        computed_style: ComputedStyle,
        retain_node_effect_group: bool,
    ) -> Self {
        Self {
            layout,
            computed_style,
            retain_node_effect_group,
        }
    }

    pub(super) const fn layout(&self) -> &LayoutStyle {
        &self.layout
    }

    pub(super) const fn computed_style(&self) -> &ComputedStyle {
        &self.computed_style
    }

    pub(super) const fn retain_node_effect_group(&self) -> bool {
        self.retain_node_effect_group
    }

    pub(super) fn requires_node_effect_group(&self) -> bool {
        self.retain_node_effect_group
            || self.computed_style.opacity() != runenui_core::SceneOpacity::OPAQUE
            || !self.computed_style.shadows().is_empty()
    }
}

/// Accepted effective values aligned exactly with the retained topology.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CachedEffectiveFacts {
    pub(super) nodes: Vec<EffectiveNodeFacts>,
}

impl CachedEffectiveFacts {
    /// Builds the behavior-preserving projection used before a motion source overrides
    /// any target. No authored state is mutated or transferred into runtime authority.
    pub(super) fn identity<Action>(
        tree: &crate::mounted::MountedTree<Action>,
        topology: &SurfaceTopologySnapshot,
        styles: &CachedStyleFacts,
    ) -> Self {
        debug_assert_eq!(topology.nodes.len(), styles.resolutions.len());
        let nodes = topology
            .nodes
            .iter()
            .zip(&styles.resolutions)
            .map(|(topology, resolution)| {
                let mounted = tree
                    .node(&topology.id)
                    .unwrap_or_else(|| unreachable!("effective topology remains live"));
                EffectiveNodeFacts::new(
                    mounted.layout.clone(),
                    resolution.computed_style().clone(),
                    false,
                )
            })
            .collect();
        Self { nodes }
    }

    pub(super) fn effects_against(&self, other: &Self) -> EffectiveEffects {
        debug_assert_eq!(self.nodes.len(), other.nodes.len());
        self.nodes.iter().zip(&other.nodes).fold(
            EffectiveEffects::default(),
            |mut effects, (old, new)| {
                if old.layout != new.layout {
                    effects.layout = true;
                }
                let style = style_effects_between(old.computed_style(), new.computed_style());
                effects.layout |= style.layout();
                effects.paint |= style.paint();
                effects.presentation |= style.presentation();
                effects.paint |= old.retain_node_effect_group != new.retain_node_effect_group;
                effects
            },
        )
    }

    pub(super) fn node(&self, position: usize) -> &EffectiveNodeFacts {
        self.nodes
            .get(position)
            .unwrap_or_else(|| unreachable!("effective facts remain topology-aligned"))
    }
}

pub(super) fn resolve_styles<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    topology: &SurfaceTopologySnapshot,
    environment: &StyleEnvironment,
    interaction: &SurfaceInteractionProjection,
    capabilities: &SurfaceCapabilityPlan,
) -> CachedStyleFacts {
    #[cfg(test)]
    super::cache::note_style_phase_execution();
    let mut computed_by_id = HashMap::with_capacity(topology.nodes.len());
    let mut resolutions = Vec::with_capacity(topology.nodes.len());
    for (position, node) in topology.nodes.iter().enumerate() {
        let mounted = tree
            .node(&node.id)
            .unwrap_or_else(|| unreachable!("style topology remains live"));
        let parent = node
            .parent
            .as_ref()
            .and_then(|parent| computed_by_id.get(parent));
        let interaction = interaction.facts_for(&node.id).with(
            StyleInteractionState::Disabled,
            !capabilities.activation_at(position, &node.id).enabled(),
        );
        let resolution =
            resolve_style_in_environment(&mounted.style, environment, interaction, parent);
        computed_by_id.insert(node.id.clone(), resolution.computed_style().clone());
        resolutions.push(resolution);
    }
    let report = SurfaceStyleReport::new(
        topology
            .nodes
            .iter()
            .zip(&resolutions)
            .map(|(node, resolution)| {
                SurfaceStyleNode::new(
                    node.id.clone(),
                    node.parent.clone(),
                    node.authored_id.clone(),
                    resolution.clone(),
                )
            })
            .collect(),
    );
    CachedStyleFacts {
        resolutions,
        report,
    }
}

pub(super) struct ResolvedSurfaceTree {
    nodes: Vec<ResolvedSurfaceNode>,
}

impl ResolvedSurfaceTree {
    pub(super) fn for_layout(
        topology: &SurfaceTopologySnapshot,
        effective: &CachedEffectiveFacts,
    ) -> Self {
        debug_assert_eq!(topology.nodes.len(), effective.nodes.len());
        let nodes = topology
            .nodes
            .iter()
            .zip(&effective.nodes)
            .map(|(topology, effective)| ResolvedSurfaceNode {
                topology: topology.clone(),
                effective: effective.clone(),
            })
            .collect();
        Self { nodes }
    }

    pub(super) const fn nodes(&self) -> &[ResolvedSurfaceNode] {
        self.nodes.as_slice()
    }

    pub(super) fn position(&self, id: &MountedNodeId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id() == id)
    }
}

pub(super) struct ResolvedSurfaceNode {
    topology: SurfaceTopologyNode,
    effective: EffectiveNodeFacts,
}

impl ResolvedSurfaceNode {
    pub(super) const fn id(&self) -> &MountedNodeId {
        &self.topology.id
    }
    pub(super) const fn parent(&self) -> Option<&MountedNodeId> {
        self.topology.parent.as_ref()
    }
    pub(super) const fn authored_id(&self) -> Option<&ElementId> {
        self.topology.authored_id.as_ref()
    }
    pub(super) const fn children(&self) -> &[MountedNodeId] {
        self.topology.children.as_slice()
    }
    pub(super) const fn surface_presentation(&self) -> Option<&SurfacePresentation> {
        self.topology.surface_presentation.as_ref()
    }
    pub(super) const fn layout(&self) -> &LayoutStyle {
        self.effective.layout()
    }
    pub(super) const fn computed_style(&self) -> &ComputedStyle {
        self.effective.computed_style()
    }
}

pub(super) fn paint_contexts(
    layout: &CachedLayoutFacts,
    effective: &CachedEffectiveFacts,
    scroll_controls: &[Option<CachedScrollControlProjection>],
) -> Vec<PaintContributionContext> {
    debug_assert_eq!(layout.bounds.len(), scroll_controls.len());
    layout
        .bounds
        .iter()
        .zip(&effective.nodes)
        .zip(scroll_controls)
        .map(|((bounds, node), scroll_control)| {
            PaintContributionContext::__runtime_with_scroll_control(
                bounds.size(),
                node.computed_style().clone(),
                scroll_control
                    .as_ref()
                    .map(|projection| projection.snapshot),
            )
        })
        .collect()
}

pub(super) fn displayed_scroll_metrics(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    owner: &MountedNodeId,
) -> Option<DisplayedScrollMetrics> {
    let position = topology.position(owner)?;
    displayed_scroll_metrics_at(topology, layout, owner, position)
}

fn displayed_scroll_metrics_at(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    owner: &MountedNodeId,
    position: usize,
) -> Option<DisplayedScrollMetrics> {
    let topology_node = topology.nodes.get(position)?;
    if &topology_node.id != owner
        || (topology_node.overflow.horizontal() != OverflowPolicy::Scroll
            && topology_node.overflow.vertical() != OverflowPolicy::Scroll)
    {
        return None;
    }
    let layout_node = layout.report.nodes().get(position)?;
    if layout_node.id() != owner {
        return None;
    }
    Some(DisplayedScrollMetrics {
        overflow: topology_node.overflow,
        viewport: layout_node.scroll_viewport_extent(),
        content: layout_node.scrollable_extent(),
    })
}

pub(super) fn resolve_scroll_owner(
    topology: &SurfaceTopologySnapshot,
    descendant: &MountedNodeId,
    axis: Axis,
) -> Result<Option<(MountedNodeId, usize)>, PresentationGeometryError> {
    let descendant_position = topology
        .position(descendant)
        .ok_or(PresentationGeometryError)?;
    let mut ancestor = topology.nodes[descendant_position].parent.as_ref();
    while let Some(owner) = ancestor {
        let owner_position = topology.position(owner).ok_or(PresentationGeometryError)?;
        let owner_topology = &topology.nodes[owner_position];
        let scrollable = match axis {
            Axis::Horizontal => owner_topology.overflow.horizontal() == OverflowPolicy::Scroll,
            Axis::Vertical => owner_topology.overflow.vertical() == OverflowPolicy::Scroll,
        };
        if scrollable {
            return Ok(Some((owner.clone(), owner_position)));
        }
        ancestor = owner_topology.parent.as_ref();
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ResolvedScrollBarChrome {
    pub(super) position: usize,
    pub(super) owner_position: usize,
    pub(super) layout: ScrollBarLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ResolvedScrollThumbChrome {
    pub(super) position: usize,
    pub(super) owner_position: usize,
    pub(super) axis: Axis,
    pub(super) bar_position: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ResolvedScrollCornerChrome {
    pub(super) position: usize,
    pub(super) owner_position: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ScrollChromeLayoutPlan {
    pub(super) chrome_positions: Vec<bool>,
    pub(super) bars: Vec<ResolvedScrollBarChrome>,
    pub(super) thumbs: Vec<ResolvedScrollThumbChrome>,
    pub(super) corners: Vec<ResolvedScrollCornerChrome>,
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
}

const fn axis_key(axis: Axis) -> u8 {
    match axis {
        Axis::Horizontal => 0,
        Axis::Vertical => 1,
    }
}

type ScrollChromeOwnerAxis = (usize, u8);
type ScrollChromeBarPositions = HashMap<ScrollChromeOwnerAxis, usize>;

fn resolve_scroll_bar_chrome(
    topology: &SurfaceTopologySnapshot,
    diagnostics: &mut [Vec<WidgetDiagnostic>],
) -> Result<Vec<ResolvedScrollBarChrome>, PresentationGeometryError> {
    let mut candidates = Vec::new();
    let mut counts = HashMap::<ScrollChromeOwnerAxis, usize>::new();
    for (position, node) in topology.nodes.iter().enumerate() {
        let Some(ScrollChrome::Bar(layout)) = node.scroll_chrome else {
            continue;
        };
        let Some((_, owner_position)) = resolve_scroll_owner(topology, &node.id, layout.axis())?
        else {
            diagnostics[position].push(WidgetDiagnostic::new(
                "runenui.scroll-chrome.missing-owner",
                "scrollbar bar has no eligible ancestor scroll owner for its axis",
            ));
            continue;
        };
        *counts
            .entry((owner_position, axis_key(layout.axis())))
            .or_default() += 1;
        candidates.push(ResolvedScrollBarChrome {
            position,
            owner_position,
            layout,
        });
    }
    Ok(candidates
        .into_iter()
        .filter(|bar| {
            let unique = counts
                .get(&(bar.owner_position, axis_key(bar.layout.axis())))
                .copied()
                == Some(1);
            if !unique {
                diagnostics[bar.position].push(WidgetDiagnostic::new(
                    "runenui.scroll-chrome.duplicate-bar",
                    "multiple scrollbar bars target the same scroll owner axis; all are withheld",
                ));
            }
            unique
        })
        .collect())
}

fn scroll_chrome_bar_positions(bars: &[ResolvedScrollBarChrome]) -> ScrollChromeBarPositions {
    bars.iter()
        .map(|bar| {
            (
                (bar.owner_position, axis_key(bar.layout.axis())),
                bar.position,
            )
        })
        .collect()
}

fn resolve_scroll_thumb_chrome(
    topology: &SurfaceTopologySnapshot,
    bar_positions: &ScrollChromeBarPositions,
    diagnostics: &mut [Vec<WidgetDiagnostic>],
) -> Result<Vec<ResolvedScrollThumbChrome>, PresentationGeometryError> {
    let mut candidates = Vec::new();
    let mut counts = HashMap::<ScrollChromeOwnerAxis, usize>::new();
    for (position, node) in topology.nodes.iter().enumerate() {
        let Some(ScrollChrome::Thumb(axis)) = node.scroll_chrome else {
            continue;
        };
        let Some((_, owner_position)) = resolve_scroll_owner(topology, &node.id, axis)? else {
            diagnostics[position].push(WidgetDiagnostic::new(
                "runenui.scroll-chrome.missing-owner",
                "scrollbar thumb has no eligible ancestor scroll owner for its axis",
            ));
            continue;
        };
        *counts.entry((owner_position, axis_key(axis))).or_default() += 1;
        candidates.push((position, owner_position, axis));
    }

    Ok(candidates
        .into_iter()
        .filter_map(|(position, owner_position, axis)| {
            let key = (owner_position, axis_key(axis));
            if counts.get(&key).copied() != Some(1) {
                diagnostics[position].push(WidgetDiagnostic::new(
                    "runenui.scroll-chrome.duplicate-thumb",
                    "multiple scrollbar thumbs target the same scroll owner axis; all are withheld",
                ));
                return None;
            }
            let Some(bar_position) = bar_positions.get(&key).copied() else {
                diagnostics[position].push(WidgetDiagnostic::new(
                    "runenui.scroll-chrome.missing-bar",
                    "scrollbar thumb has no unique scrollbar bar for its scroll owner axis",
                ));
                return None;
            };
            Some(ResolvedScrollThumbChrome {
                position,
                owner_position,
                axis,
                bar_position,
            })
        })
        .collect())
}

fn resolve_scroll_corner_chrome(
    topology: &SurfaceTopologySnapshot,
    bar_positions: &ScrollChromeBarPositions,
    diagnostics: &mut [Vec<WidgetDiagnostic>],
) -> Result<Vec<ResolvedScrollCornerChrome>, PresentationGeometryError> {
    let mut candidates = Vec::new();
    let mut counts = HashMap::<usize, usize>::new();
    for (position, node) in topology.nodes.iter().enumerate() {
        if !matches!(node.scroll_chrome, Some(ScrollChrome::Corner)) {
            continue;
        }
        let horizontal = resolve_scroll_owner(topology, &node.id, Axis::Horizontal)?;
        let vertical = resolve_scroll_owner(topology, &node.id, Axis::Vertical)?;
        if let (Some((horizontal_owner, owner_position)), Some((vertical_owner, _))) =
            (horizontal, vertical)
            && horizontal_owner == vertical_owner
        {
            *counts.entry(owner_position).or_default() += 1;
            candidates.push((position, owner_position));
        } else {
            diagnostics[position].push(WidgetDiagnostic::new(
                "runenui.scroll-chrome.invalid-corner-owner",
                "scrollbar corner must resolve both axes to the same scroll owner",
            ));
        }
    }

    Ok(candidates
        .into_iter()
        .filter_map(|(position, owner_position)| {
            if counts.get(&owner_position).copied() != Some(1) {
                diagnostics[position].push(WidgetDiagnostic::new(
                    "runenui.scroll-chrome.duplicate-corner",
                    "multiple scrollbar corners target the same scroll owner; all are withheld",
                ));
                return None;
            }
            if !bar_positions.contains_key(&(owner_position, axis_key(Axis::Horizontal)))
                || !bar_positions.contains_key(&(owner_position, axis_key(Axis::Vertical)))
            {
                diagnostics[position].push(WidgetDiagnostic::new(
                    "runenui.scroll-chrome.missing-corner-bars",
                    "scrollbar corner requires unique horizontal and vertical bars for its owner",
                ));
                return None;
            }
            Some(ResolvedScrollCornerChrome {
                position,
                owner_position,
            })
        })
        .collect())
}

pub(super) fn resolve_scroll_chrome_layout_plan(
    topology: &SurfaceTopologySnapshot,
) -> Result<ScrollChromeLayoutPlan, PresentationGeometryError> {
    let chrome_positions = topology
        .nodes
        .iter()
        .map(|node| node.scroll_chrome.is_some())
        .collect::<Vec<_>>();
    let mut diagnostics = vec![Vec::new(); topology.nodes.len()];
    let bars = resolve_scroll_bar_chrome(topology, &mut diagnostics)?;
    let bar_positions = scroll_chrome_bar_positions(&bars);
    let thumbs = resolve_scroll_thumb_chrome(topology, &bar_positions, &mut diagnostics)?;
    let corners = resolve_scroll_corner_chrome(topology, &bar_positions, &mut diagnostics)?;

    Ok(ScrollChromeLayoutPlan {
        chrome_positions,
        bars,
        thumbs,
        corners,
        diagnostics,
    })
}

pub(super) fn scroll_control_projections<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    scroll: &SurfaceScrollProjection,
) -> Result<Vec<Option<CachedScrollControlProjection>>, PresentationGeometryError> {
    if topology.nodes.len() != layout.bounds.len()
        || topology.nodes.len() != layout.report.nodes().len()
    {
        return Err(PresentationGeometryError);
    }

    topology
        .nodes
        .iter()
        .map(|node| {
            let Some(binding) = tree
                .node(&node.id)
                .and_then(|mounted| mounted.scroll_control_binding)
            else {
                return Ok(None);
            };
            let Some((owner, owner_position)) =
                resolve_scroll_owner(topology, &node.id, binding.axis())?
            else {
                return Ok(None);
            };
            let metrics = displayed_scroll_metrics_at(topology, layout, &owner, owner_position)
                .ok_or(PresentationGeometryError)?;
            let offset = scroll.offset(&owner);
            let (offset, viewport, content) = match binding.axis() {
                Axis::Horizontal => (offset.0, metrics.viewport.width(), metrics.content.width()),
                Axis::Vertical => (
                    offset.1,
                    metrics.viewport.height(),
                    metrics.content.height(),
                ),
            };
            let snapshot = ScrollControlSnapshot::__runtime_from_metrics(
                binding.axis(),
                offset,
                viewport,
                content,
            )
            .ok_or(PresentationGeometryError)?;
            Ok(Some(CachedScrollControlProjection {
                owner,
                binding,
                snapshot,
            }))
        })
        .collect()
}

pub(super) fn semantic_contexts(
    topology: &SurfaceTopologySnapshot,
    scroll_controls: &[Option<CachedScrollControlProjection>],
) -> Vec<SemanticContributionContext> {
    debug_assert_eq!(topology.nodes.len(), scroll_controls.len());
    topology
        .nodes
        .iter()
        .zip(scroll_controls)
        .map(|(node, scroll_control)| {
            SemanticContributionContext::__runtime_with_scroll_control(
                node.children.len(),
                scroll_control
                    .as_ref()
                    .map(|projection| projection.snapshot),
            )
        })
        .collect()
}

pub(super) fn hit_contexts(
    layout: &CachedLayoutFacts,
    scroll_controls: &[Option<CachedScrollControlProjection>],
) -> Vec<HitContributionContext> {
    debug_assert_eq!(layout.bounds.len(), scroll_controls.len());
    layout
        .bounds
        .iter()
        .zip(scroll_controls)
        .map(|(bounds, scroll_control)| {
            HitContributionContext::__runtime_with_scroll_control(
                bounds.size(),
                scroll_control
                    .as_ref()
                    .map(|projection| projection.snapshot),
            )
        })
        .collect()
}

#[must_use]
pub(super) fn scroll_chrome_participates(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    position: usize,
) -> bool {
    match topology
        .nodes
        .get(position)
        .and_then(|node| node.scroll_chrome)
    {
        None => true,
        Some(_) => layout
            .scroll_chrome
            .get(position)
            .copied()
            .flatten()
            .is_some_and(CachedScrollChromeProjection::present),
    }
}

fn scroll_thumb_presentation_offset(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    scroll: &SurfaceScrollProjection,
    projection: CachedScrollChromeProjection,
) -> Result<(f32, f32), PresentationGeometryError> {
    let CachedScrollChromeKind::Thumb {
        owner_position,
        axis,
        track_position,
    } = projection.kind()
    else {
        return Ok((0.0, 0.0));
    };
    let owner = topology
        .nodes
        .get(owner_position)
        .ok_or(PresentationGeometryError)?;
    let owner_layout = layout
        .report
        .nodes()
        .get(owner_position)
        .filter(|layout_node| layout_node.id() == &owner.id)
        .ok_or(PresentationGeometryError)?;
    let track = layout
        .bounds
        .get(track_position)
        .ok_or(PresentationGeometryError)?;
    let bar_layout = topology
        .nodes
        .get(track_position)
        .and_then(|node| node.scroll_chrome)
        .and_then(ScrollChrome::bar_layout)
        .filter(|bar_layout| bar_layout.axis() == axis)
        .ok_or(PresentationGeometryError)?;
    let viewport = owner_layout.scroll_viewport_extent();
    let content = owner_layout.scrollable_extent();
    let offset = scroll.offset(&owner.id);
    let (offset, viewport_extent, content_extent, track_extent) = match axis {
        Axis::Horizontal => (offset.0, viewport.width(), content.width(), track.width()),
        Axis::Vertical => (
            offset.1,
            viewport.height(),
            content.height(),
            track.height(),
        ),
    };
    let snapshot = ScrollControlSnapshot::__runtime_from_metrics(
        axis,
        offset,
        viewport_extent,
        content_extent,
    )
    .ok_or(PresentationGeometryError)?;
    let geometry = bar_layout
        .thumb_geometry(
            snapshot,
            LogicalLength::new(track_extent).map_err(|_| PresentationGeometryError)?,
        )
        .ok_or(PresentationGeometryError)?;
    let shift = geometry.thumb_origin().get();
    Ok(match axis {
        Axis::Horizontal => (shift, 0.0),
        Axis::Vertical => (0.0, shift),
    })
}

struct ScrollChromePresentationContext<'a> {
    topology: &'a SurfaceTopologySnapshot,
    layout: &'a CachedLayoutFacts,
    scroll: &'a SurfaceScrollProjection,
}

#[derive(Clone, Copy)]
struct ScrollChromePresentationInput {
    node_presentation: LogicalTransform,
    position: usize,
    bounds: LogicalRect,
    projection: CachedScrollChromeProjection,
}

struct PresentedScrollChrome {
    presentation: PresentationNodeFacts,
    child_offset: (f32, f32),
    child_clips: Vec<SceneClip>,
    child_clip_bounds: Vec<LogicalRect>,
}

fn resolve_present_scroll_chrome(
    context: &ScrollChromePresentationContext<'_>,
    input: ScrollChromePresentationInput,
    nodes: &[PresentationNodeFacts],
    inherited_scroll_offsets: &[(f32, f32)],
) -> Result<PresentedScrollChrome, PresentationGeometryError> {
    let owner_position = input.projection.owner_position();
    if owner_position >= input.position {
        return Err(PresentationGeometryError);
    }
    let owner_bounds = *context
        .layout
        .bounds
        .get(owner_position)
        .ok_or(PresentationGeometryError)?;
    let owner_presentation = nodes.get(owner_position).ok_or(PresentationGeometryError)?;
    let child_offset = inherited_scroll_offsets
        .get(owner_position)
        .copied()
        .ok_or(PresentationGeometryError)?;
    let (thumb_x, thumb_y) = scroll_thumb_presentation_offset(
        context.topology,
        context.layout,
        context.scroll,
        input.projection,
    )?;
    let placement = LogicalTransform::translation(
        input.bounds.x() - owner_bounds.x() + thumb_x,
        input.bounds.y() - owner_bounds.y() + thumb_y,
    )
    .map_err(|_| PresentationGeometryError)?;
    let owner_to_surface = input
        .node_presentation
        .then(placement)
        .and_then(|transform| transform.then(owner_presentation.owner_to_surface()))
        .map_err(|_| PresentationGeometryError)?;
    let local_bounds = LogicalRect::try_new(0.0, 0.0, input.bounds.width(), input.bounds.height())
        .unwrap_or_else(|_| unreachable!("published chrome layout size is valid"));
    let presented_bounds =
        transform_rect_aabb(owner_to_surface, local_bounds).ok_or(PresentationGeometryError)?;
    let visible_bounds = intersect_rects(presented_bounds, owner_presentation.visible_bounds());

    let owner_clip_rect =
        LogicalRect::try_new(0.0, 0.0, owner_bounds.width(), owner_bounds.height())
            .unwrap_or_else(|_| unreachable!("published owner layout size is valid"));
    let mut child_clips = owner_presentation.inherited_clips().to_vec();
    child_clips.push(SceneClip::new(
        SceneShape::rect(owner_clip_rect),
        owner_presentation.owner_to_surface(),
    ));
    let content_clips = child_clips.clone();
    let child_clip_bounds = vec![owner_presentation.visible_bounds()];
    Ok(PresentedScrollChrome {
        presentation: PresentationNodeFacts::new(PresentationNodeFactsInit {
            owner_to_surface,
            content_to_surface: owner_to_surface,
            owner_bounds: presented_bounds,
            visible_bounds,
            inherited_clips: Arc::from(child_clips.clone()),
            content_clips: Arc::from(content_clips),
            published: owner_presentation.published(),
            stack_root: owner_presentation.stack_root(),
            root_snapshot: None,
            diagnostics: Arc::from(Vec::<WidgetDiagnostic>::new()),
        }),
        child_offset,
        child_clips,
        child_clip_bounds,
    })
}

/// Recoverable failure to derive a finite node-presentation publication product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PresentationGeometryError;

pub(super) fn normalize_scroll_projection(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    scroll: &SurfaceScrollProjection,
) -> Result<SurfaceScrollProjection, PresentationGeometryError> {
    if topology.nodes.len() != layout.bounds.len()
        || topology.nodes.len() != layout.report.nodes().len()
    {
        return Err(PresentationGeometryError);
    }
    let mut offsets = Vec::with_capacity(topology.nodes.len());
    for (position, node) in topology.nodes.iter().enumerate() {
        let (requested_x, requested_y) = scroll.offset(&node.id);
        if !requested_x.is_finite() || !requested_y.is_finite() {
            return Err(PresentationGeometryError);
        }
        let layout_node = layout
            .report
            .nodes()
            .get(position)
            .filter(|layout_node| layout_node.id() == &node.id)
            .ok_or(PresentationGeometryError)?;
        let viewport = layout_node.scroll_viewport_extent();
        let extent = layout_node.scrollable_extent();
        let max_x = (extent.width() - viewport.width()).max(0.0);
        let max_y = (extent.height() - viewport.height()).max(0.0);
        let x = if node.overflow.horizontal() == OverflowPolicy::Scroll {
            requested_x.clamp(0.0, max_x)
        } else {
            0.0
        };
        let y = if node.overflow.vertical() == OverflowPolicy::Scroll {
            requested_y.clamp(0.0, max_y)
        } else {
            0.0
        };
        offsets.push((node.id.clone(), (canonical_zero(x), canonical_zero(y))));
    }
    Ok(SurfaceScrollProjection::new(offsets))
}

fn intersect_rects(left: LogicalRect, right: LogicalRect) -> LogicalRect {
    let x = left.x().max(right.x());
    let y = left.y().max(right.y());
    let max_x = left.max_x().min(right.max_x());
    let max_y = left.max_y().min(right.max_y());
    LogicalRect::try_new(x, y, (max_x - x).max(0.0), (max_y - y).max(0.0))
        .unwrap_or_else(|_| unreachable!("intersecting finite published bounds remains finite"))
}

const fn canonical_zero(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}

#[derive(Clone, Copy)]
enum SceneContributionFamily {
    Paint,
    Hit,
}

fn scene_transform_diagnostic(
    family: SceneContributionFamily,
    contribution_local_order: usize,
    clip_order: Option<usize>,
    non_finite: bool,
) -> WidgetDiagnostic {
    let (code, subject) = match (family, clip_order, non_finite) {
        (SceneContributionFamily::Paint, None, true) => (
            "runenui.scene.paint-transform-non-finite",
            format!("paint item {contribution_local_order} final transform"),
        ),
        (SceneContributionFamily::Paint, None, false) => (
            "runenui.scene.paint-transform-non-invertible",
            format!("paint item {contribution_local_order} final transform"),
        ),
        (SceneContributionFamily::Paint, Some(clip_order), true) => (
            "runenui.scene.paint-clip-transform-non-finite",
            format!("paint item {contribution_local_order} clip {clip_order} final transform"),
        ),
        (SceneContributionFamily::Paint, Some(clip_order), false) => (
            "runenui.scene.paint-clip-transform-non-invertible",
            format!("paint item {contribution_local_order} clip {clip_order} final transform"),
        ),
        (SceneContributionFamily::Hit, None, true) => (
            "runenui.scene.hit-transform-non-finite",
            format!("hit region {contribution_local_order} final transform"),
        ),
        (SceneContributionFamily::Hit, None, false) => (
            "runenui.scene.hit-transform-non-invertible",
            format!("hit region {contribution_local_order} final transform"),
        ),
        (SceneContributionFamily::Hit, Some(clip_order), true) => (
            "runenui.scene.hit-clip-transform-non-finite",
            format!("hit region {contribution_local_order} clip {clip_order} final transform"),
        ),
        (SceneContributionFamily::Hit, Some(clip_order), false) => (
            "runenui.scene.hit-clip-transform-non-invertible",
            format!("hit region {contribution_local_order} clip {clip_order} final transform"),
        ),
    };
    let message = if non_finite {
        format!("{subject} cannot be represented finitely; the contribution is excluded")
    } else {
        format!("{subject} is non-invertible; logical coverage is empty")
    };
    WidgetDiagnostic::new(code, message)
}

fn empty_scene_diagnostics(topology: &SurfaceTopologySnapshot) -> Vec<Vec<WidgetDiagnostic>> {
    vec![Vec::new(); topology.nodes.len()]
}

fn compose_scene_clips(
    clips: &[ContributionClip],
    owner_to_surface: LogicalTransform,
    family: SceneContributionFamily,
    contribution_local_order: usize,
    diagnostics: &mut Vec<WidgetDiagnostic>,
) -> Option<Vec<SceneClip>> {
    let mut composed = Vec::with_capacity(clips.len());
    for (clip_order, clip) in clips.iter().enumerate() {
        let Ok(clip_to_surface) = clip.local_to_owner().then(owner_to_surface) else {
            diagnostics.push(scene_transform_diagnostic(
                family,
                contribution_local_order,
                Some(clip_order),
                true,
            ));
            return None;
        };
        if clip_to_surface.inverse().is_none() {
            diagnostics.push(scene_transform_diagnostic(
                family,
                contribution_local_order,
                Some(clip_order),
                false,
            ));
        }
        composed.push(SceneClip::new(clip.shape().clone(), clip_to_surface));
    }
    Some(composed)
}

pub(super) struct ResolvedPaint {
    pub(super) scene: PaintScene,
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
}

struct TextEditingPaintGeometry {
    selection: Vec<LogicalRect>,
    preedit_underline: Vec<LogicalRect>,
    caret: LogicalRect,
}

fn text_editing_paint_geometry(
    layout: &CachedLayoutFacts,
    position: usize,
    editing: Option<&crate::editing::EditingSemanticProjection>,
    preedit: Option<&Arc<TextPreeditProjection>>,
) -> Option<TextEditingPaintGeometry> {
    let text_layout = layout.text_layouts.get(position)?;
    if let Some(preedit) = preedit {
        if editing.is_some_and(|editing| {
            preedit.snapshot() != editing.snapshot
                || preedit.document_text() != editing.source.as_ref()
        }) {
            return None;
        }
        let map = text_layout.preedit_caret_map(Arc::clone(preedit)).ok()?;
        let start = preedit
            .position_from_display_offset(preedit.display_preedit_start(), TextAffinity::Downstream)
            .ok()?;
        let end = preedit
            .position_from_display_offset(preedit.display_preedit_end(), TextAffinity::Upstream)
            .ok()?;
        let preedit_underline = map
            .selection_rects(&TextDisplaySelection::new(start, end.clone()))
            .ok()?
            .into_iter()
            .map(runenui_text::TextSelectionRect::rect)
            .collect();
        let selection = map.preedit_selection().ok()?;
        let caret_position = selection
            .as_ref()
            .map_or(&end, |selection| selection.active());
        let caret = map
            .caret_rect(caret_position, LogicalLength::from(1_u8))
            .ok()?;
        let selection = selection
            .map(|selection| map.selection_rects(&selection))
            .transpose()
            .ok()?
            .unwrap_or_default()
            .into_iter()
            .map(runenui_text::TextSelectionRect::rect)
            .collect();
        return Some(TextEditingPaintGeometry {
            selection,
            preedit_underline,
            caret,
        });
    }

    let editing = editing?;
    let map = text_layout
        .caret_map_for_source(editing.snapshot, &editing.source)
        .ok()?;
    let selection = TextDisplaySelection::from_document(editing.selection);
    let caret = map
        .caret_rect(selection.active(), LogicalLength::from(1_u8))
        .ok()?;
    let selection = map
        .selection_rects(&selection)
        .ok()?
        .into_iter()
        .map(runenui_text::TextSelectionRect::rect)
        .collect();
    Some(TextEditingPaintGeometry {
        selection,
        preedit_underline: Vec::new(),
        caret,
    })
}

fn text_editing_paint_geometry_for_owner(
    layout: &CachedLayoutFacts,
    position: usize,
    owner: &MountedNodeId,
    editing: &HashMap<MountedNodeId, crate::editing::EditingSemanticProjection>,
    preedits: &HashMap<MountedNodeId, Arc<TextPreeditProjection>>,
) -> Option<TextEditingPaintGeometry> {
    let editing = editing.get(owner);
    let preedit = preedits.get(owner);
    (editing.is_some() || preedit.is_some())
        .then(|| text_editing_paint_geometry(layout, position, editing, preedit))
        .flatten()
}

fn text_rect_with_origin(rect: LogicalRect, origin: LogicalPoint) -> Option<LogicalRect> {
    LogicalRect::try_new(
        rect.x() + origin.x(),
        rect.y() + origin.y(),
        rect.width(),
        rect.height(),
    )
    .ok()
}

#[derive(Clone, Copy)]
struct OwnerPaintContext<'a> {
    mounted_preorder: usize,
    owner_to_surface: LogicalTransform,
    content_to_surface: LogicalTransform,
    inherited_clips: &'a [SceneClip],
    content_clips: &'a [SceneClip],
}

fn owner_paint_context(
    presentation: &CachedPresentationFacts,
    mounted_preorder: usize,
) -> OwnerPaintContext<'_> {
    let presentation_node = presentation.node(mounted_preorder);
    OwnerPaintContext {
        mounted_preorder,
        owner_to_surface: presentation_node.owner_to_surface(),
        content_to_surface: presentation_node.content_to_surface(),
        inherited_clips: presentation_node.inherited_clips(),
        content_clips: presentation_node.content_clips(),
    }
}

fn append_text_overlay_rect(
    rect: LogicalRect,
    color: Color,
    owner: OwnerPaintContext<'_>,
    next_local_order: &mut usize,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    let item = PaintContributionItem::fill(SceneShape::rect(rect), color.into());
    append_runtime_paint_item(
        &item,
        owner.mounted_preorder,
        *next_local_order,
        owner.content_to_surface,
        owner.content_clips,
        ordered,
    );
    *next_local_order += 1;
}

fn append_text_selection_overlay(
    geometry: &TextEditingPaintGeometry,
    computed: &ComputedStyle,
    origin: LogicalPoint,
    owner: OwnerPaintContext<'_>,
    next_local_order: &mut usize,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    let foreground = computed.foreground().unwrap_or(Color::BLACK);
    let selection_color = Color::rgba(foreground.red(), foreground.green(), foreground.blue(), 96);
    for rect in &geometry.selection {
        if let Some(rect) = text_rect_with_origin(*rect, origin) {
            append_text_overlay_rect(rect, selection_color, owner, next_local_order, ordered);
        }
    }
}

fn append_text_preedit_and_caret(
    geometry: &TextEditingPaintGeometry,
    computed: &ComputedStyle,
    origin: LogicalPoint,
    focused: bool,
    owner: OwnerPaintContext<'_>,
    next_local_order: &mut usize,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    let foreground = computed.foreground().unwrap_or(Color::BLACK);
    for rect in &geometry.preedit_underline {
        let Some(rect) = text_rect_with_origin(*rect, origin).and_then(|rect| {
            LogicalRect::try_new(rect.x(), rect.y() + rect.height() - 1.0, rect.width(), 1.0).ok()
        }) else {
            continue;
        };
        append_text_overlay_rect(rect, foreground, owner, next_local_order, ordered);
    }

    if focused && let Some(rect) = text_rect_with_origin(geometry.caret, origin) {
        append_text_overlay_rect(rect, foreground, owner, next_local_order, ordered);
    }
}

fn append_shaped_text(
    layout: &CachedLayoutFacts,
    computed: &ComputedStyle,
    owner: OwnerPaintContext<'_>,
    next_local_order: &mut usize,
    text_system: &mut TextSystem,
    shaped_text_leases: &mut Vec<ShapedTextLease>,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    #[cfg(feature = "internal-test-seams")]
    let mut profiled_run_count = 0usize;
    let mounted_preorder = owner.mounted_preorder;
    if let Some(artifact) = layout.text_layouts[mounted_preorder].artifact() {
        for line in artifact.lines() {
            for run in line.runs() {
                #[cfg(feature = "internal-test-seams")]
                {
                    profiled_run_count = profiled_run_count.saturating_add(1);
                }
                let lease = text_system
                    .lease_shaped_run(run.resource_ref())
                    .unwrap_or_else(|| {
                        unreachable!("published text artifact retains its exact shaped resource")
                    });
                shaped_text_leases.push(lease);
                let item = text_run_item(run, computed, layout.text_origins[mounted_preorder]);
                append_runtime_paint_item(
                    &item,
                    owner.mounted_preorder,
                    *next_local_order,
                    owner.content_to_surface,
                    owner.content_clips,
                    ordered,
                );
                *next_local_order += 1;
            }
        }
    }
    #[cfg(feature = "internal-test-seams")]
    super::profile::record_paint_text_run_items(profiled_run_count);
}

fn text_run_item(
    run: &runenui_text::TextRun,
    computed: &ComputedStyle,
    text_origin: LogicalPoint,
) -> PaintContributionItem {
    let origin = LogicalPoint::new(
        text_origin.x() + run.origin_x(),
        text_origin.y() + run.origin_y(),
    )
    .unwrap_or_else(|_| unreachable!("text artifact and resolved padding remain finite"));
    PaintContributionItem::shaped_text_run(
        run.resource_ref().clone(),
        origin,
        computed.foreground().unwrap_or(Color::BLACK),
    )
    .unwrap_or_else(|_| unreachable!("logical text artifacts issue shaped-text resource refs"))
}

fn node_decoration_shape(bounds: LogicalRect, computed: &ComputedStyle) -> SceneShape {
    let rect = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
        .unwrap_or_else(|_| unreachable!("published layout size is valid"));
    match computed.radius() {
        Some(radius) if radius != Radius::ZERO => SceneShape::rounded_rect(rect, radius),
        Some(_) | None => SceneShape::rect(rect),
    }
}

fn append_runtime_paint_item(
    item: &PaintContributionItem,
    mounted_preorder: usize,
    contribution_local_order: usize,
    owner_to_surface: LogicalTransform,
    inherited_clips: &[SceneClip],
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    ordered.push(groups::OrderedPaintItem::new(
        item.layer(),
        mounted_preorder,
        contribution_local_order,
        None,
        PaintSceneItem::new(
            item.primitive().clone(),
            owner_to_surface,
            inherited_clips.to_vec(),
            item.opacity(),
            item.layer(),
        ),
    ));
}

fn append_node_background(
    shape: Option<&SceneShape>,
    computed: &ComputedStyle,
    owner: OwnerPaintContext<'_>,
    next_local_order: &mut usize,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    if let (Some(shape), Some(background)) = (shape, computed.background()) {
        append_runtime_paint_item(
            &PaintContributionItem::fill(shape.clone(), background.clone()),
            owner.mounted_preorder,
            *next_local_order,
            owner.owner_to_surface,
            owner.inherited_clips,
            ordered,
        );
        *next_local_order += 1;
    }
}

fn append_node_outline(
    shape: Option<&SceneShape>,
    computed: &ComputedStyle,
    owner: OwnerPaintContext<'_>,
    next_local_order: &mut usize,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) {
    if let (Some(shape), Some(outline)) = (shape, computed.outline()) {
        append_runtime_paint_item(
            &PaintContributionItem::stroke(shape.clone(), outline.brush().clone(), outline.style()),
            owner.mounted_preorder,
            *next_local_order,
            owner.owner_to_surface,
            owner.inherited_clips,
            ordered,
        );
        *next_local_order += 1;
    }
}

#[allow(clippy::too_many_arguments)] // Parallel paint outputs share one topology-aligned contribution pass.
fn append_paint_contribution(
    contribution: &PaintContribution,
    mounted_preorder: usize,
    local_order_base: usize,
    owner_to_surface: LogicalTransform,
    inherited_clips: &[SceneClip],
    diagnostics: &mut Vec<WidgetDiagnostic>,
    explicit_groups: &mut Vec<groups::ResolvedExplicitGroup>,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) -> usize {
    let local_groups = explicit_groups::append_resolved_explicit_groups(
        contribution,
        mounted_preorder,
        owner_to_surface,
        inherited_clips,
        diagnostics,
        explicit_groups,
    );
    for (contribution_local_order, item) in contribution.items().iter().enumerate() {
        let explicit_group = match contribution.__runtime_item_group(contribution_local_order) {
            Some(local_group) => {
                let Some(group) = local_groups.get(local_group).copied().flatten() else {
                    continue;
                };
                Some(group)
            }
            None => None,
        };
        let Ok(local_to_surface) = item.local_transform().then(owner_to_surface) else {
            diagnostics.push(scene_transform_diagnostic(
                SceneContributionFamily::Paint,
                contribution_local_order,
                None,
                true,
            ));
            continue;
        };
        if local_to_surface.inverse().is_none() {
            diagnostics.push(scene_transform_diagnostic(
                SceneContributionFamily::Paint,
                contribution_local_order,
                None,
                false,
            ));
        }
        let Some(authored_clips) = compose_scene_clips(
            item.clips(),
            owner_to_surface,
            SceneContributionFamily::Paint,
            contribution_local_order,
            diagnostics,
        ) else {
            continue;
        };
        let mut clips = inherited_clips.to_vec();
        clips.extend(authored_clips);
        ordered.push(groups::OrderedPaintItem::new(
            item.layer(),
            mounted_preorder,
            local_order_base + contribution_local_order,
            explicit_group,
            PaintSceneItem::new(
                image_mapping::publication_primitive(item),
                local_to_surface,
                clips,
                item.opacity(),
                item.layer(),
            ),
        ));
    }
    contribution.items().len()
}

pub(super) struct PaintResolutionInput<'a> {
    pub(super) topology: &'a SurfaceTopologySnapshot,
    pub(super) layout: &'a CachedLayoutFacts,
    pub(super) presentation: &'a CachedPresentationFacts,
    pub(super) effective: &'a CachedEffectiveFacts,
    pub(super) capabilities: &'a SurfaceCapabilityPlan,
    pub(super) text_system: &'a mut TextSystem,
    pub(super) text_editing: TextEditingPaintInputs<'a>,
}

#[allow(
    clippy::let_and_return,
    clippy::too_many_lines,
    reason = "private profiling observes the existing paint transaction without changing its production decomposition"
)]
pub(super) fn resolve_paint(input: PaintResolutionInput<'_>) -> ResolvedPaint {
    #[cfg(feature = "internal-test-seams")]
    let profile_started = std::time::Instant::now();
    let PaintResolutionInput {
        topology,
        layout,
        presentation,
        effective,
        capabilities,
        text_system,
        text_editing,
    } = input;
    let focused_owner = text_editing.focused_owner;
    let editing = text_editing.editing;
    let preedits = text_editing.preedits;
    #[cfg(test)]
    super::cache::note_paint_phase_execution();
    let mut diagnostics = empty_scene_diagnostics(topology);
    let mut ordered = Vec::new();
    let mut explicit_groups = Vec::new();
    let mut shaped_text_leases = Vec::new();
    for (mounted_preorder, node) in topology.nodes.iter().enumerate() {
        if !scroll_chrome_participates(topology, layout, mounted_preorder)
            || !presentation.published(mounted_preorder)
        {
            continue;
        }
        let owner = owner_paint_context(presentation, mounted_preorder);
        let computed = effective.node(mounted_preorder).computed_style();
        let decoration_shape = (computed.background().is_some() || computed.outline().is_some())
            .then(|| node_decoration_shape(layout.bounds[mounted_preorder], computed));
        let mut next_local_order = 0;
        append_node_background(
            decoration_shape.as_ref(),
            computed,
            owner,
            &mut next_local_order,
            &mut ordered,
        );

        if let Some(contribution) = capabilities.paint_at(mounted_preorder, &node.id) {
            next_local_order += append_paint_contribution(
                &contribution,
                mounted_preorder,
                next_local_order,
                owner.content_to_surface,
                owner.content_clips,
                &mut diagnostics[mounted_preorder],
                &mut explicit_groups,
                &mut ordered,
            );
        }

        let text_overlay = text_editing_paint_geometry_for_owner(
            layout,
            mounted_preorder,
            &node.id,
            editing,
            preedits,
        );
        if let Some(geometry) = text_overlay.as_ref() {
            append_text_selection_overlay(
                geometry,
                computed,
                layout.text_origins[mounted_preorder],
                owner,
                &mut next_local_order,
                &mut ordered,
            );
        }

        append_shaped_text(
            layout,
            computed,
            owner,
            &mut next_local_order,
            text_system,
            &mut shaped_text_leases,
            &mut ordered,
        );

        if let Some(geometry) = text_overlay.as_ref() {
            append_text_preedit_and_caret(
                geometry,
                computed,
                layout.text_origins[mounted_preorder],
                focused_owner == Some(&node.id),
                owner,
                &mut next_local_order,
                &mut ordered,
            );
        }

        append_node_outline(
            decoration_shape.as_ref(),
            computed,
            owner,
            &mut next_local_order,
            &mut ordered,
        );
    }
    ordered.sort_by_key(|item| item.ordering_key(presentation));
    let (items, composition) = groups::derive_composition_groups(
        topology,
        effective,
        presentation,
        &explicit_groups,
        ordered,
    );
    let resolved = ResolvedPaint {
        scene: PaintScene::with_composition(items, shaped_text_leases, composition),
        diagnostics,
    };
    #[cfg(feature = "internal-test-seams")]
    super::profile::record_paint(profile_started.elapsed());
    resolved
}

pub(super) struct ResolvedHitTest {
    pub(super) scene: HitTestSceneContent,
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
}

pub(super) fn resolve_hit_test(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    presentation: &CachedPresentationFacts,
    capabilities: &SurfaceCapabilityPlan,
) -> ResolvedHitTest {
    #[cfg(test)]
    super::cache::note_hit_test_phase_execution();
    let membership = topology.nodes.iter().map(|node| node.id.clone()).collect();
    let mut diagnostics = empty_scene_diagnostics(topology);
    let mut ordered = Vec::new();
    for (mounted_preorder, node) in topology.nodes.iter().enumerate() {
        if !scroll_chrome_participates(topology, layout, mounted_preorder)
            || !presentation.published(mounted_preorder)
        {
            continue;
        }
        let Some(contribution) = capabilities.hit_test_at(mounted_preorder, &node.id) else {
            continue;
        };
        let presentation_node = presentation.node(mounted_preorder);
        let owner_to_surface = presentation_node.owner_to_surface();
        for (contribution_local_order, region) in contribution.regions().iter().enumerate() {
            let Ok(local_to_surface) = region.local_transform().then(owner_to_surface) else {
                diagnostics[mounted_preorder].push(scene_transform_diagnostic(
                    SceneContributionFamily::Hit,
                    contribution_local_order,
                    None,
                    true,
                ));
                continue;
            };
            if local_to_surface.inverse().is_none() {
                diagnostics[mounted_preorder].push(scene_transform_diagnostic(
                    SceneContributionFamily::Hit,
                    contribution_local_order,
                    None,
                    false,
                ));
            }
            let Some(authored_clips) = compose_scene_clips(
                region.clips(),
                owner_to_surface,
                SceneContributionFamily::Hit,
                contribution_local_order,
                &mut diagnostics[mounted_preorder],
            ) else {
                continue;
            };
            let mut clips = presentation_node.inherited_clips().to_vec();
            clips.extend(authored_clips);
            ordered.push((
                region.layer(),
                mounted_preorder,
                contribution_local_order,
                HitTestRegion::new(
                    node.id.clone(),
                    region.shape().clone(),
                    local_to_surface,
                    clips,
                    region.layer(),
                    region.pointer_policy(),
                ),
            ));
        }
    }
    ordered.sort_by_key(|(layer, mounted_preorder, contribution_local_order, _)| {
        let (band, root) = presentation
            .stack_root(*mounted_preorder)
            .map_or((0, 0), |root| (1, root));
        (
            band,
            root,
            *layer,
            *mounted_preorder,
            *contribution_local_order,
        )
    });
    ResolvedHitTest {
        scene: HitTestSceneContent::new(
            ordered
                .into_iter()
                .map(|(_, _, _, region)| region)
                .collect(),
            membership,
        ),
        diagnostics,
    }
}

pub(super) fn resolve_diagnostics(
    topology: &SurfaceTopologySnapshot,
    capabilities: &SurfaceCapabilityPlan,
) -> Vec<Vec<WidgetDiagnostic>> {
    #[cfg(test)]
    super::cache::note_diagnostics_phase_execution();
    topology
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| {
            capabilities
                .diagnostics_at(position, &node.id)
                .unwrap_or_else(|| {
                    vec![WidgetDiagnostic::new(
                        "runenui.runtime.state-payload-mismatch",
                        "mounted widget state payload does not match its description",
                    )]
                })
        })
        .collect()
}
