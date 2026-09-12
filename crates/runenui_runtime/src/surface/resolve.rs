use std::collections::HashMap;

mod explicit_groups;
mod groups;
mod image_mapping;

use crate::MountedNodeId;
use crate::mounted::SurfaceCapabilityPlan;
use crate::scene::{HitTestRegion, HitTestSceneContent, PaintScene, PaintSceneItem, SceneClip};
use crate::style_debug::{SurfaceStyleNode, SurfaceStyleReport};
use runenui_core::{
    __runtime::transform_rect_aabb, Color, ContributionClip, ElementId, HitContributionContext,
    LayoutStyle, LogicalPoint, LogicalRect, LogicalTransform, PaintContribution,
    PaintContributionContext, PaintContributionItem, Radius, SceneShape, StyleEffects,
    StyleEnvironment, StyleInteractionState, StyleResolution, WidgetDiagnostic, WidgetTypeId,
    resolve_style_in_environment, style_effects_between,
};
use runenui_text::TextSystem;

use super::{
    SurfaceInteractionProjection,
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
            }
        })
        .collect();
    SurfaceTopologySnapshot { nodes }
}

#[derive(Clone, Debug)]
pub(super) struct CachedStyleFacts {
    // Style-phase facts aligned to the topology snapshot. They are refreshed
    // whenever mounted style intent or exact style-environment content changes.
    pub(super) resolutions: Vec<StyleResolution>,
    pub(super) report: SurfaceStyleReport,
}

impl CachedStyleFacts {
    pub(super) fn effects_against(&self, other: &Self) -> StyleEffects {
        self.resolutions.iter().zip(&other.resolutions).fold(
            StyleEffects::NONE,
            |effects, (old, new)| {
                effects.union(style_effects_between(
                    old.computed_style(),
                    new.computed_style(),
                ))
            },
        )
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
    pub(super) fn for_layout<Action>(
        tree: &crate::mounted::MountedTree<Action>,
        topology: &SurfaceTopologySnapshot,
        styles: &CachedStyleFacts,
    ) -> Self {
        let nodes = topology
            .nodes
            .iter()
            .zip(&styles.resolutions)
            .map(|(topology, resolution)| {
                let mounted = tree
                    .node(&topology.id)
                    .unwrap_or_else(|| unreachable!("layout topology remains live"));
                ResolvedSurfaceNode {
                    topology: topology.clone(),
                    layout: mounted.layout.clone(),
                    resolution: resolution.clone(),
                }
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
    layout: LayoutStyle,
    resolution: StyleResolution,
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
        &self.layout
    }
    pub(super) const fn resolution(&self) -> &StyleResolution {
        &self.resolution
    }
}

pub(super) fn paint_contexts(
    layout: &CachedLayoutFacts,
    styles: &CachedStyleFacts,
) -> Vec<PaintContributionContext> {
    layout
        .bounds
        .iter()
        .zip(&styles.resolutions)
        .map(|(bounds, style)| {
            PaintContributionContext::__runtime_new(bounds.size(), style.computed_style().clone())
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
    layout: &CachedLayoutFacts,
    styles: &CachedStyleFacts,
) -> Result<CachedPresentationFacts, PresentationGeometryError> {
    if layout.bounds.len() != styles.resolutions.len() {
        return Err(PresentationGeometryError);
    }
    let mut nodes = Vec::with_capacity(layout.bounds.len());
    for (bounds, style) in layout.bounds.iter().zip(&styles.resolutions) {
        let node_presentation = style
            .computed_style()
            .presentation()
            .map_or(Ok(LogicalTransform::IDENTITY), |presentation| {
                presentation.resolve_in_box(bounds.size())
            })
            .map_err(|_| PresentationGeometryError)?;
        let placement = LogicalTransform::translation(bounds.x(), bounds.y())
            .map_err(|_| PresentationGeometryError)?;
        let owner_to_surface = node_presentation
            .then(placement)
            .map_err(|_| PresentationGeometryError)?;
        let local_bounds = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
            .unwrap_or_else(|_| unreachable!("published layout size is valid"));
        let owner_bounds =
            transform_rect_aabb(owner_to_surface, local_bounds).ok_or(PresentationGeometryError)?;
        nodes.push(PresentationNodeFacts::new(owner_to_surface, owner_bounds));
    }
    Ok(CachedPresentationFacts { nodes })
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

fn text_run_item(run: &runenui_text::TextRun, style: &StyleResolution) -> PaintContributionItem {
    let computed = style.computed_style();
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

fn node_decoration_shape(bounds: LogicalRect, style: &StyleResolution) -> SceneShape {
    let rect = LogicalRect::try_new(0.0, 0.0, bounds.width(), bounds.height())
        .unwrap_or_else(|_| unreachable!("published layout size is valid"));
    match style.computed_style().radius() {
        Some(radius) if radius != Radius::ZERO => SceneShape::rounded_rect(rect, radius),
        Some(_) | None => SceneShape::rect(rect),
    }
}

fn append_runtime_paint_item(
    item: &PaintContributionItem,
    mounted_preorder: usize,
    contribution_local_order: usize,
    owner_to_surface: LogicalTransform,
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
            Vec::new(),
            item.opacity(),
            item.layer(),
        ),
    ));
}

fn append_paint_contribution(
    contribution: &PaintContribution,
    mounted_preorder: usize,
    local_order_base: usize,
    owner_to_surface: LogicalTransform,
    diagnostics: &mut Vec<WidgetDiagnostic>,
    explicit_groups: &mut Vec<groups::ResolvedExplicitGroup>,
    ordered: &mut Vec<groups::OrderedPaintItem>,
) -> usize {
    let local_groups = explicit_groups::append_resolved_explicit_groups(
        contribution,
        mounted_preorder,
        owner_to_surface,
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
        let Some(clips) = compose_scene_clips(
            item.clips(),
            owner_to_surface,
            SceneContributionFamily::Paint,
            contribution_local_order,
            diagnostics,
        ) else {
            continue;
        };
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
    styles: &CachedStyleFacts,
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
        let owner_to_surface = presentation.node(mounted_preorder).owner_to_surface();
        let style = &styles.resolutions[mounted_preorder];
        let computed = style.computed_style();
        let decoration_shape = (computed.background().is_some() || computed.outline().is_some())
            .then(|| node_decoration_shape(layout.bounds[mounted_preorder], style));
        let mut next_local_order = 0;

        if let (Some(shape), Some(background)) = (decoration_shape.as_ref(), computed.background())
        {
            append_runtime_paint_item(
                &PaintContributionItem::fill(shape.clone(), background.clone()),
                mounted_preorder,
                next_local_order,
                owner_to_surface,
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
                    let item = text_run_item(run, &styles.resolutions[mounted_preorder]);
                    append_runtime_paint_item(
                        &item,
                        mounted_preorder,
                        next_local_order,
                        owner_to_surface,
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
                &mut ordered,
            );
        }
    }
    ordered.sort_by_key(groups::OrderedPaintItem::ordering_key);
    let (items, composition) =
        groups::derive_composition_groups(topology, styles, &explicit_groups, ordered);
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
        let owner_to_surface = presentation.node(mounted_preorder).owner_to_surface();
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
            let Some(clips) = compose_scene_clips(
                region.clips(),
                owner_to_surface,
                SceneContributionFamily::Hit,
                contribution_local_order,
                &mut diagnostics[mounted_preorder],
            ) else {
                continue;
            };
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
