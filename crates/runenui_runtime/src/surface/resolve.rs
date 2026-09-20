use std::{collections::HashMap, sync::Arc};

mod explicit_groups;
mod groups;
mod image_mapping;

use crate::MountedNodeId;
use crate::mounted::SurfaceCapabilityPlan;
use crate::scene::{HitTestRegion, HitTestSceneContent, PaintScene, PaintSceneItem, SceneClip};
use crate::style_debug::{SurfaceStyleNode, SurfaceStyleReport};
use runenui_core::{
    __runtime::transform_rect_aabb, Color, ComputedStyle, ContributionClip, ElementId,
    HitContributionContext, LayoutStyle, LogicalPoint, LogicalRect, LogicalTransform,
    OverflowPolicy, OverflowStyle, PaintContribution, PaintContributionContext,
    PaintContributionItem, Radius, SceneShape, StyleEnvironment, StyleInteractionState,
    StyleResolution, WidgetDiagnostic, WidgetTypeId, resolve_style_in_environment,
    style_effects_between,
};
use runenui_text::TextSystem;

use super::{
    SurfaceInteractionProjection, SurfaceScrollProjection,
    cache::{CachedLayoutFacts, CachedPresentationFacts, PresentationNodeFacts},
};

/// Topology and publication-alignment facts for one mounted preorder.
///
/// Mutable authored style and layout remain owned only by mounted nodes.
#[derive(Clone, Debug)]
pub(super) struct SurfaceTopologySnapshot {
    pub(super) nodes: Vec<SurfaceTopologyNode>,
}

#[derive(Clone, Debug)]
pub(super) struct SurfaceTopologyNode {
    pub(super) id: MountedNodeId,
    pub(super) parent: Option<MountedNodeId>,
    pub(super) authored_id: Option<ElementId>,
    pub(super) widget_type_id: WidgetTypeId,
    pub(super) children: Vec<MountedNodeId>,
    pub(super) overflow: OverflowStyle,
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
            }
        })
        .collect();
    SurfaceTopologySnapshot { nodes }
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
) -> Vec<PaintContributionContext> {
    layout
        .bounds
        .iter()
        .zip(&effective.nodes)
        .map(|(bounds, node)| {
            PaintContributionContext::__runtime_new(bounds.size(), node.computed_style().clone())
        })
        .collect()
}

pub(super) fn hit_contexts(layout: &CachedLayoutFacts) -> Vec<HitContributionContext> {
    layout
        .bounds
        .iter()
        .map(|bounds| HitContributionContext::__runtime_new(bounds.size()))
        .collect()
}

/// Recoverable failure to derive a finite node-presentation publication product.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PresentationGeometryError;

pub(super) fn resolve_presentation(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    effective: &CachedEffectiveFacts,
    scroll: &SurfaceScrollProjection,
) -> Result<CachedPresentationFacts, PresentationGeometryError> {
    if layout.bounds.len() != effective.nodes.len() || layout.bounds.len() != topology.nodes.len() {
        return Err(PresentationGeometryError);
    }
    let positions = topology
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.clone(), position))
        .collect::<HashMap<_, _>>();
    let mut nodes = Vec::with_capacity(layout.bounds.len());
    let mut child_offsets = Vec::with_capacity(layout.bounds.len());
    let mut child_clips = Vec::<Vec<SceneClip>>::with_capacity(layout.bounds.len());
    let mut child_clip_bounds = Vec::<Vec<LogicalRect>>::with_capacity(layout.bounds.len());
    for ((bounds, effective), topology_node) in layout
        .bounds
        .iter()
        .zip(&effective.nodes)
        .zip(&topology.nodes)
    {
        let parent_position = topology_node
            .parent
            .as_ref()
            .and_then(|parent| positions.get(parent).copied());
        let (ancestor_x, ancestor_y) =
            parent_position.map_or((0.0, 0.0), |parent| child_offsets[parent]);
        let mut inherited_clips = parent_position
            .map(|parent| child_clips[parent].clone())
            .unwrap_or_default();
        let mut inherited_clip_bounds = parent_position
            .map(|parent| child_clip_bounds[parent].clone())
            .unwrap_or_default();
        let node_presentation = effective
            .computed_style()
            .presentation()
            .map_or(Ok(LogicalTransform::IDENTITY), |presentation| {
                presentation.resolve_in_box(bounds.size())
            })
            .map_err(|_| PresentationGeometryError)?;
        let placement =
            LogicalTransform::translation(bounds.x() - ancestor_x, bounds.y() - ancestor_y)
                .map_err(|_| PresentationGeometryError)?;
        let owner_to_surface = node_presentation
            .then(placement)
            .map_err(|_| PresentationGeometryError)?;
        let local_bounds = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
            .unwrap_or_else(|_| unreachable!("published layout size is valid"));
        let owner_bounds =
            transform_rect_aabb(owner_to_surface, local_bounds).ok_or(PresentationGeometryError)?;
        let visible_bounds = inherited_clip_bounds
            .iter()
            .fold(owner_bounds, |visible, clip| {
                intersect_rects(visible, *clip)
            });
        nodes.push(PresentationNodeFacts::new(
            owner_to_surface,
            owner_bounds,
            visible_bounds,
            Arc::from(inherited_clips.clone()),
        ));

        let local_scroll = scroll.offset(&topology_node.id);
        let scroll_x = if topology_node.overflow.horizontal() == OverflowPolicy::Scroll {
            local_scroll.0
        } else {
            0.0
        };
        let scroll_y = if topology_node.overflow.vertical() == OverflowPolicy::Scroll {
            local_scroll.1
        } else {
            0.0
        };
        let child_x = ancestor_x + scroll_x;
        let child_y = ancestor_y + scroll_y;
        if !child_x.is_finite() || !child_y.is_finite() {
            return Err(PresentationGeometryError);
        }
        child_offsets.push((child_x, child_y));

        if scroll_x != 0.0
            || scroll_y != 0.0
            || topology_node.overflow.horizontal() == OverflowPolicy::Scroll
            || topology_node.overflow.vertical() == OverflowPolicy::Scroll
        {
            let clip_rect = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
                .unwrap_or_else(|_| unreachable!("published viewport extent is valid"));
            let clip_bounds = transform_rect_aabb(owner_to_surface, clip_rect)
                .ok_or(PresentationGeometryError)?;
            inherited_clips.push(SceneClip::new(
                SceneShape::rect(clip_rect),
                owner_to_surface,
            ));
            inherited_clip_bounds.push(clip_bounds);
        }
        child_clips.push(inherited_clips);
        child_clip_bounds.push(inherited_clip_bounds);
    }
    Ok(CachedPresentationFacts { nodes })
}

pub(super) fn normalize_scroll_projection(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    scroll: &SurfaceScrollProjection,
) -> Result<SurfaceScrollProjection, PresentationGeometryError> {
    if topology.nodes.len() != layout.bounds.len() {
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
            .node(&node.id)
            .ok_or(PresentationGeometryError)?;
        let viewport = layout.bounds[position].size();
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

fn text_run_item(run: &runenui_text::TextRun, computed: &ComputedStyle) -> PaintContributionItem {
    let padding = computed.padding().unwrap_or_default();
    let origin = LogicalPoint::new(
        padding.left().get() + run.origin_x(),
        padding.top().get() + run.origin_y(),
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

pub(super) fn resolve_paint(
    topology: &SurfaceTopologySnapshot,
    layout: &CachedLayoutFacts,
    presentation: &CachedPresentationFacts,
    effective: &CachedEffectiveFacts,
    capabilities: &SurfaceCapabilityPlan,
    text_system: &mut TextSystem,
) -> ResolvedPaint {
    #[cfg(test)]
    super::cache::note_paint_phase_execution();
    let mut diagnostics = empty_scene_diagnostics(topology);
    let mut ordered = Vec::new();
    let mut explicit_groups = Vec::new();
    let mut shaped_text_leases = Vec::new();
    for (mounted_preorder, node) in topology.nodes.iter().enumerate() {
        let presentation_node = presentation.node(mounted_preorder);
        let owner_to_surface = presentation_node.owner_to_surface();
        let inherited_clips = presentation_node.inherited_clips();
        let computed = effective.node(mounted_preorder).computed_style();
        let decoration_shape = (computed.background().is_some() || computed.outline().is_some())
            .then(|| node_decoration_shape(layout.bounds[mounted_preorder], computed));
        let mut next_local_order = 0;

        if let (Some(shape), Some(background)) = (decoration_shape.as_ref(), computed.background())
        {
            append_runtime_paint_item(
                &PaintContributionItem::fill(shape.clone(), background.clone()),
                mounted_preorder,
                next_local_order,
                owner_to_surface,
                inherited_clips,
                &mut ordered,
            );
            next_local_order += 1;
        }

        if let Some(contribution) = capabilities.paint_at(mounted_preorder, &node.id) {
            next_local_order += append_paint_contribution(
                &contribution,
                mounted_preorder,
                next_local_order,
                owner_to_surface,
                inherited_clips,
                &mut diagnostics[mounted_preorder],
                &mut explicit_groups,
                &mut ordered,
            );
        }

        if let Some(artifact) = layout.text_layouts[mounted_preorder].artifact() {
            for line in artifact.lines() {
                for run in line.runs() {
                    let lease = text_system
                        .lease_shaped_run(run.resource_ref())
                        .unwrap_or_else(|| {
                            unreachable!(
                                "published text artifact retains its exact shaped resource"
                            )
                        });
                    shaped_text_leases.push(lease);
                    let item = text_run_item(run, computed);
                    append_runtime_paint_item(
                        &item,
                        mounted_preorder,
                        next_local_order,
                        owner_to_surface,
                        inherited_clips,
                        &mut ordered,
                    );
                    next_local_order += 1;
                }
            }
        }

        if let (Some(shape), Some(outline)) = (decoration_shape.as_ref(), computed.outline()) {
            append_runtime_paint_item(
                &PaintContributionItem::stroke(
                    shape.clone(),
                    outline.brush().clone(),
                    outline.style(),
                ),
                mounted_preorder,
                next_local_order,
                owner_to_surface,
                inherited_clips,
                &mut ordered,
            );
        }
    }
    ordered.sort_by_key(groups::OrderedPaintItem::ordering_key);
    let (items, composition) = groups::derive_composition_groups(
        topology,
        effective,
        presentation,
        &explicit_groups,
        ordered,
    );
    ResolvedPaint {
        scene: PaintScene::with_composition(items, shaped_text_leases, composition),
        diagnostics,
    }
}

pub(super) struct ResolvedHitTest {
    pub(super) scene: HitTestSceneContent,
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
}

pub(super) fn resolve_hit_test(
    topology: &SurfaceTopologySnapshot,
    presentation: &CachedPresentationFacts,
    capabilities: &SurfaceCapabilityPlan,
) -> ResolvedHitTest {
    #[cfg(test)]
    super::cache::note_hit_test_phase_execution();
    let membership = topology.nodes.iter().map(|node| node.id.clone()).collect();
    let mut diagnostics = empty_scene_diagnostics(topology);
    let mut ordered = Vec::new();
    for (mounted_preorder, node) in topology.nodes.iter().enumerate() {
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
        (*layer, *mounted_preorder, *contribution_local_order)
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
