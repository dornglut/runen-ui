use std::collections::HashMap;

use crate::MountedNodeId;
use crate::mounted::SurfaceCapabilityPlan;
use crate::scene::{HitTestRegion, HitTestSceneContent, PaintScene, PaintSceneItem, SceneClip};
use crate::style_debug::{SurfaceStyleNode, SurfaceStyleReport};
use runenui_core::{
    Axis, ChildLayout, ContributionClip, ElementId, HitContributionContext, LayoutStyle,
    LogicalTransform, PaintContributionContext, StyleEffects, StyleEnvironment,
    StyleInteractionFacts, StyleInteractionState, StyleResolution, WidgetDiagnostic, WidgetMeasure,
    WidgetTypeId, resolve_style_in_environment, style_effects_between,
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
            .and_then(|parent| computed_by_id.get(parent).copied());
        let interaction = StyleInteractionFacts::NONE.with(
            StyleInteractionState::Disabled,
            !capabilities.activation_at(position, &node.id).enabled(),
        );
        let resolution =
            resolve_style_in_environment(&mounted.style, environment, interaction, parent);
        computed_by_id.insert(node.id.clone(), resolution.computed_style());
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
        capabilities: &SurfaceCapabilityPlan,
    ) -> Self {
        let nodes = topology
            .nodes
            .iter()
            .zip(&styles.resolutions)
            .enumerate()
            .map(|(position, (topology, resolution))| {
                let mounted = tree
                    .node(&topology.id)
                    .unwrap_or_else(|| unreachable!("layout topology remains live"));
                ResolvedSurfaceNode {
                    position,
                    topology: topology.clone(),
                    layout: mounted.layout,
                    measurement: capabilities
                        .measurement_at(position, &topology.id)
                        .unwrap_or_default(),
                    child_layout: capabilities.child_layout_at_or_else(
                        position,
                        &topology.id,
                        || {
                            (!mounted.children.is_empty()).then_some(ChildLayout::Linear {
                                axis: Axis::Vertical,
                            })
                        },
                    ),
                    resolution: resolution.clone(),
                }
            })
            .collect();
        Self { nodes }
    }

    pub(super) const fn nodes(&self) -> &[ResolvedSurfaceNode] {
        self.nodes.as_slice()
    }

    pub(super) fn node(&self, id: &MountedNodeId) -> &ResolvedSurfaceNode {
        self.nodes
            .iter()
            .find(|node| node.id() == id)
            .unwrap_or_else(|| unreachable!("resolved mounted ID exists"))
    }
}

pub(super) struct ResolvedSurfaceNode {
    pub(super) position: usize,
    topology: SurfaceTopologyNode,
    layout: LayoutStyle,
    measurement: WidgetMeasure,
    child_layout: Option<ChildLayout>,
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
    pub(super) const fn measurement(&self) -> &WidgetMeasure {
        &self.measurement
    }
    pub(super) const fn child_layout(&self) -> Option<ChildLayout> {
        self.child_layout
    }
    pub(super) const fn resolution(&self) -> &StyleResolution {
        &self.resolution
    }
}

pub(super) fn paint_contexts(
    layout: &super::cache::CachedLayoutFacts,
    styles: &CachedStyleFacts,
) -> Vec<PaintContributionContext> {
    layout
        .bounds
        .iter()
        .zip(&styles.resolutions)
        .map(|(bounds, style)| {
            PaintContributionContext::__runtime_new(bounds.size(), style.computed_style())
        })
        .collect()
}

pub(super) fn hit_contexts(
    layout: &super::cache::CachedLayoutFacts,
) -> Vec<HitContributionContext> {
    layout
        .bounds
        .iter()
        .map(|bounds| HitContributionContext::__runtime_new(bounds.size()))
        .collect()
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
        composed.push(SceneClip::new(clip.shape(), clip_to_surface));
    }
    Some(composed)
}

pub(super) struct ResolvedPaint {
    pub(super) scene: PaintScene,
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
}

pub(super) fn resolve_paint(
    topology: &SurfaceTopologySnapshot,
    layout: &super::cache::CachedLayoutFacts,
    capabilities: &SurfaceCapabilityPlan,
) -> ResolvedPaint {
    #[cfg(test)]
    super::cache::note_paint_phase_execution();
    let mut diagnostics = empty_scene_diagnostics(topology);
    let mut ordered = Vec::new();
    for (mounted_preorder, node) in topology.nodes.iter().enumerate() {
        let Some(contribution) = capabilities.paint_at(mounted_preorder, &node.id) else {
            continue;
        };
        let bounds = layout.bounds[mounted_preorder];
        let owner_to_surface = LogicalTransform::translation(bounds.x(), bounds.y())
            .unwrap_or_else(|_| unreachable!("published layout origin is finite"));
        for (contribution_local_order, item) in contribution.items().iter().enumerate() {
            let Ok(local_to_surface) = item.local_transform().then(owner_to_surface) else {
                diagnostics[mounted_preorder].push(scene_transform_diagnostic(
                    SceneContributionFamily::Paint,
                    contribution_local_order,
                    None,
                    true,
                ));
                continue;
            };
            if local_to_surface.inverse().is_none() {
                diagnostics[mounted_preorder].push(scene_transform_diagnostic(
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
                &mut diagnostics[mounted_preorder],
            ) else {
                continue;
            };
            ordered.push((
                item.layer(),
                mounted_preorder,
                contribution_local_order,
                PaintSceneItem::new(
                    item.primitive().clone(),
                    local_to_surface,
                    clips,
                    item.opacity(),
                    item.layer(),
                ),
            ));
        }
    }
    ordered.sort_by_key(|(layer, mounted_preorder, contribution_local_order, _)| {
        (*layer, *mounted_preorder, *contribution_local_order)
    });
    ResolvedPaint {
        scene: PaintScene::new(ordered.into_iter().map(|(_, _, _, item)| item).collect()),
        diagnostics,
    }
}

pub(super) struct ResolvedHitTest {
    pub(super) scene: HitTestSceneContent,
    pub(super) diagnostics: Vec<Vec<WidgetDiagnostic>>,
}

pub(super) fn resolve_hit_test(
    topology: &SurfaceTopologySnapshot,
    layout: &super::cache::CachedLayoutFacts,
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
        let bounds = layout.bounds[mounted_preorder];
        let owner_to_surface = LogicalTransform::translation(bounds.x(), bounds.y())
            .unwrap_or_else(|_| unreachable!("published layout origin is finite"));
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
                    region.shape(),
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
