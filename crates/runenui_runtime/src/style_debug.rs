//! Debug style reports for surface frames.

use core::fmt::{self, Write as _};

use runenui_core::{
    ComputedStyle, ElementId, StyleProvenance, StyleResolution, UnresolvedStyleToken,
};

use crate::{MountedNodeId, SemanticNodeId};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceStyleReport {
    nodes: Vec<SurfaceStyleNode>,
}

impl SurfaceStyleReport {
    #[must_use]
    pub(crate) const fn new(nodes: Vec<SurfaceStyleNode>) -> Self {
        Self { nodes }
    }

    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceStyleNode] {
        self.nodes.as_slice()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&SurfaceStyleNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceStyleNode {
    id: MountedNodeId,
    semantic_id: SemanticNodeId,
    parent: Option<MountedNodeId>,
    authored_id: Option<ElementId>,
    resolution: StyleResolution,
}

impl SurfaceStyleNode {
    #[must_use]
    pub(crate) const fn new(
        id: MountedNodeId,
        semantic_id: SemanticNodeId,
        parent: Option<MountedNodeId>,
        authored_id: Option<ElementId>,
        resolution: StyleResolution,
    ) -> Self {
        Self {
            id,
            semantic_id,
            parent,
            authored_id,
            resolution,
        }
    }

    #[must_use]
    pub const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    #[must_use]
    pub const fn semantic_id(&self) -> &SemanticNodeId {
        &self.semantic_id
    }

    #[must_use]
    pub const fn parent(&self) -> Option<&MountedNodeId> {
        self.parent.as_ref()
    }

    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    /// Returns the complete core style-resolution product for this node.
    #[must_use]
    pub const fn resolution(&self) -> &StyleResolution {
        &self.resolution
    }

    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.resolution.computed_style()
    }

    /// Returns per-field style-resolution provenance.
    #[must_use]
    pub const fn provenance(&self) -> &StyleProvenance {
        self.resolution.provenance()
    }

    #[must_use]
    pub const fn unresolved_tokens(&self) -> &[UnresolvedStyleToken] {
        self.resolution.unresolved_tokens()
    }

    #[must_use]
    pub const fn is_fully_resolved(&self) -> bool {
        self.resolution.is_fully_resolved()
    }
}

#[must_use]
pub fn render_debug_surface_style_report(report: &SurfaceStyleReport) -> String {
    let mut output = String::new();

    append_line(
        &mut output,
        format_args!("surface styles nodes={}", report.nodes().len()),
    );

    for node in report.nodes() {
        append_line(&mut output, format_args!("{}", DebugSurfaceStyleNode(node)));
    }

    output
}

struct DebugSurfaceStyleNode<'a>(&'a SurfaceStyleNode);

impl fmt::Display for DebugSurfaceStyleNode<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.0;

        write!(
            formatter,
            "style id={:?} authored={} computed={:?} provenance={:?} unresolved={:?}",
            node.id(),
            format_authored_id(node),
            node.computed_style(),
            node.provenance(),
            node.unresolved_tokens()
        )
    }
}

fn append_line(output: &mut String, arguments: fmt::Arguments<'_>) {
    match output.write_fmt(arguments) {
        Ok(()) | Err(_) => {}
    }
    output.push('\n');
}

fn format_authored_id(node: &SurfaceStyleNode) -> &str {
    node.authored_id().map_or("-", ElementId::as_str)
}
