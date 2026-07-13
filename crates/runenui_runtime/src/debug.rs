//! Debug rendering helpers for surface frames.
//!
//! This module renders a [`SurfaceFrame`] into deterministic text for tests,
//! diagnostics, and early host integration. It is not a pixel renderer and does
//! not define a backend abstraction.

use core::fmt::{self, Write as _};

use crate::{LogicalRect, LogicalSize, MountedNodeId, SurfaceFrame, SurfaceNode};

/// Deterministic text renderer for renderer-facing surface frames.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DebugSurfaceRenderer;

impl DebugSurfaceRenderer {
    /// Creates a debug surface renderer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Renders a surface frame into deterministic diagnostic text.
    #[must_use]
    pub fn render(&self, frame: &SurfaceFrame) -> String {
        render_debug_surface_frame(frame)
    }
}

/// Renders a surface frame into deterministic diagnostic text.
#[must_use]
pub fn render_debug_surface_frame(frame: &SurfaceFrame) -> String {
    let mut output = String::new();

    append_line(
        &mut output,
        format_args!(
            "surface size={} nodes={}",
            format_size(frame.size()),
            frame.nodes().len()
        ),
    );

    for node in frame.nodes() {
        append_line(&mut output, format_args!("{}", DebugSurfaceNode(node)));
    }

    output
}

struct DebugSurfaceNode<'a>(&'a SurfaceNode);

impl fmt::Display for DebugSurfaceNode<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.0;

        write!(
            formatter,
            "node id={} parent={} authored={} bounds={} paint={} {:?} semantic={} {:?} enabled={} actionable={} diagnostics={:?}",
            format_node_id(node.id()),
            format_parent(node.parent()),
            format_authored_id(node),
            format_rect(node.bounds()),
            node.paint().category(),
            node.paint().description(),
            node.semantics().role(),
            node.semantics().name(),
            node.semantics().enabled(),
            node.semantics().actionable(),
            node.diagnostics(),
        )
    }
}

fn append_line(output: &mut String, arguments: fmt::Arguments<'_>) {
    match output.write_fmt(arguments) {
        Ok(()) | Err(_) => {}
    }
    output.push('\n');
}

fn format_node_id(id: &MountedNodeId) -> String {
    format!("{id:?}")
}

fn format_parent(parent: Option<&MountedNodeId>) -> String {
    parent.map_or_else(|| "-".to_owned(), format_node_id)
}

fn format_authored_id(node: &SurfaceNode) -> &str {
    node.authored_id()
        .map_or("-", runenui_core::ElementId::as_str)
}

fn format_size(size: LogicalSize) -> String {
    format!("({:.1},{:.1})", size.width(), size.height())
}

fn format_rect(rect: LogicalRect) -> String {
    format!(
        "({:.1},{:.1},{:.1},{:.1})",
        rect.x(),
        rect.y(),
        rect.width(),
        rect.height()
    )
}
