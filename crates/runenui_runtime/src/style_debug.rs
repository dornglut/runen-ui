//! Debug style reports for surface frames.

use core::fmt::{self, Write as _};

use runenui_core::{
    ComputedStyle, Element, ElementId, ElementKind, StyleTokens, UnresolvedStyleToken,
    resolve_style,
};

use crate::{RuntimeNodeId, SurfaceFrame};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceStyleReport {
    nodes: Vec<SurfaceStyleNode>,
}

impl SurfaceStyleReport {
    #[must_use]
    pub const fn new(nodes: Vec<SurfaceStyleNode>) -> Self {
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
    pub fn node(&self, id: RuntimeNodeId) -> Option<&SurfaceStyleNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceStyleNode {
    id: RuntimeNodeId,
    authored_id: Option<ElementId>,
    computed_style: ComputedStyle,
    unresolved_tokens: Vec<UnresolvedStyleToken>,
}

impl SurfaceStyleNode {
    #[must_use]
    pub const fn new(
        id: RuntimeNodeId,
        authored_id: Option<ElementId>,
        computed_style: ComputedStyle,
        unresolved_tokens: Vec<UnresolvedStyleToken>,
    ) -> Self {
        Self {
            id,
            authored_id,
            computed_style,
            unresolved_tokens,
        }
    }

    #[must_use]
    pub const fn id(&self) -> RuntimeNodeId {
        self.id
    }

    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
    }

    #[must_use]
    pub const fn unresolved_tokens(&self) -> &[UnresolvedStyleToken] {
        self.unresolved_tokens.as_slice()
    }

    #[must_use]
    pub const fn is_fully_resolved(&self) -> bool {
        self.unresolved_tokens.is_empty()
    }
}

#[must_use]
pub fn resolve_surface_style_report<Action>(
    root: &Element<Action>,
    frame: &SurfaceFrame,
    tokens: &StyleTokens,
) -> SurfaceStyleReport {
    let mut resolutions = Vec::new();
    collect_style_resolutions(root, tokens, &mut resolutions);

    let nodes = frame
        .nodes()
        .iter()
        .zip(resolutions)
        .map(|(surface_node, resolution)| {
            SurfaceStyleNode::new(
                surface_node.id(),
                surface_node.authored_id().cloned(),
                resolution.computed_style(),
                resolution.unresolved_tokens().to_vec(),
            )
        })
        .collect();

    SurfaceStyleReport::new(nodes)
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

fn collect_style_resolutions<Action>(
    element: &Element<Action>,
    tokens: &StyleTokens,
    resolutions: &mut Vec<runenui_core::StyleResolution>,
) {
    resolutions.push(resolve_style(element.visual_style(), tokens));

    if let ElementKind::Container(container) = element.kind() {
        for child in container.children() {
            collect_style_resolutions(child, tokens, resolutions);
        }
    }
}

struct DebugSurfaceStyleNode<'a>(&'a SurfaceStyleNode);

impl fmt::Display for DebugSurfaceStyleNode<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.0;

        write!(
            formatter,
            "style id={} authored={} computed={:?} unresolved={:?}",
            node.id().as_usize(),
            format_authored_id(node),
            node.computed_style(),
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
