//! File: domain/ui/ui_artifacts/src/lib.rs
//! Crate: ui_artifacts

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use ui_program::{
    AccessibilityNode, BindingEdge, BindingEndpoint, BindingEndpointId, ControlGraphNode,
    ControlNodeId, ControlPropertySnapshot, InspectionEntry, InteractionHandler, LayoutGraphNode,
    StateRequirement, StyleRule, UiProgram, UiProgramDiagnosticSeverity,
    UiProgramSourceMapAttachment, UiProgramSourceMapEntry, UiProgramSourceSpan, UiSchemaRef,
    VisualOperator,
};
use ui_schema::UiSchemaValue;

pub mod artifact;
pub mod cache;
pub(crate) mod cache_key;
pub mod control_packages;
pub mod diagnostics;
pub mod manifest;
pub(crate) mod runtime_capabilities;
pub(crate) mod runtime_collections;
pub(crate) mod runtime_packages;
pub mod source_map;
pub(crate) mod source_map_attachment;
pub mod tables;

pub use artifact::*;
pub use cache::*;
pub(crate) use cache_key::*;
pub use control_packages::*;
pub use diagnostics::*;
pub use manifest::*;
pub(crate) use runtime_capabilities::*;
pub(crate) use runtime_collections::*;
pub(crate) use runtime_packages::*;
pub use source_map::*;
pub(crate) use source_map_attachment::*;
pub use tables::*;

#[cfg(test)]
mod tests;
