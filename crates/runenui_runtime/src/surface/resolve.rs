use crate::MountedNodeId;
use crate::mounted::SurfaceCapabilityPlan;
use crate::style_debug::{SurfaceStyleNode, SurfaceStyleReport};
use runenui_core::{
    Axis, ChildLayout, ElementId, LayoutStyle, SemanticContribution, StyleResolution, StyleTokens,
    WidgetDiagnostic, WidgetMeasure, WidgetPaintProof, WidgetTypeId, resolve_style,
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
    // whenever mounted style intent or exact token content changes.
    pub(super) resolutions: Vec<StyleResolution>,
    pub(super) report: SurfaceStyleReport,
}

impl CachedStyleFacts {
    pub(super) fn padding_changed(&self, other: &Self) -> bool {
        self.resolutions
            .iter()
            .zip(&other.resolutions)
            .any(|(old, new)| old.computed_style().padding() != new.computed_style().padding())
    }

    pub(super) fn paint_changed(&self, other: &Self) -> bool {
        self.resolutions
            .iter()
            .zip(&other.resolutions)
            .any(|(old, new)| {
                let old = old.computed_style();
                let new = new.computed_style();
                old.foreground() != new.foreground()
                    || old.background() != new.background()
                    || old.radius() != new.radius()
            })
    }
}

pub(super) fn resolve_styles<Action>(
    tree: &crate::mounted::MountedTree<Action>,
    topology: &SurfaceTopologySnapshot,
    tokens: &StyleTokens,
) -> CachedStyleFacts {
    #[cfg(test)]
    super::cache::note_style_phase_execution();
    let resolutions: Vec<_> = topology
        .nodes
        .iter()
        .map(|node| {
            let mounted = tree
                .node(&node.id)
                .unwrap_or_else(|| unreachable!("style topology remains live"));
            resolve_style(&mounted.style, tokens)
        })
        .collect();
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
    // Topology-aligned layout-phase position.
    pub(super) position: usize,
    // Topology facts.
    topology: SurfaceTopologyNode,
    // Publication-local layout-phase input copied from the current mounted node.
    layout: LayoutStyle,
    // Layout-phase capability fact.
    measurement: WidgetMeasure,
    // Layout-phase capability fact.
    child_layout: Option<ChildLayout>,
    // Current style-phase fact.
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

pub(super) fn resolve_paint(
    topology: &SurfaceTopologySnapshot,
    capabilities: &SurfaceCapabilityPlan,
) -> Vec<WidgetPaintProof> {
    #[cfg(test)]
    super::cache::note_paint_phase_execution();
    topology
        .nodes
        .iter()
        .enumerate()
        .map(|(position, node)| {
            capabilities
                .paint_at(position, &node.id)
                .unwrap_or_default()
        })
        .collect()
}

pub(super) fn resolve_semantics<Action>(
    tree: &mut crate::mounted::MountedTree<Action>,
    topology: &SurfaceTopologySnapshot,
) -> Vec<SemanticContribution> {
    #[cfg(test)]
    super::cache::note_semantics_phase_execution();
    topology
        .nodes
        .iter()
        .map(|node| {
            tree.ensure_semantics_capability(&node.id);
            tree.node(&node.id)
                .and_then(|mounted| mounted.caches.semantics.ready())
                .unwrap_or_default()
        })
        .collect()
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
