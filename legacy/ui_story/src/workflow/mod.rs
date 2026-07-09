//! Semantic workflow graph contracts for UI Story V2.
//!
//! This module owns the durable proof graph shape only. It does not load files,
//! run compilers, execute renderers, mount previews, or own app/editor behavior.

mod builtin;
mod edge;
mod graph;
mod node;
mod profile;
mod topo;
mod validate;

pub use builtin::{
    NODE_COMPILER, NODE_INTERACTION_REPLAY, NODE_INTERACTION_STATIC_MOUNT, NODE_INTERACTION_STORY,
    NODE_LIVE_INTERACTION_PROOF, NODE_MANIFEST, NODE_PREVIEW_FRAME, NODE_PROGRAM_FORMATION,
    NODE_RENDER_DATA, NODE_RENDER_PRIMITIVES, NODE_REPLAY_LIVE_PARITY, NODE_RUNTIME_VIEW,
    NODE_SOURCE_LOAD, NODE_SOURCE_PARSE, NODE_STATIC_MOUNT, UiStoryBuiltinWorkflowProfile,
    UiStoryBuiltinWorkflowProfiles, WORKFLOW_COMPILER_ONLY, WORKFLOW_EXECUTABLE_INTERACTION_PROOF,
    WORKFLOW_SOURCE_LOAD_ONLY, WORKFLOW_STATIC_PREVIEW,
};
pub use edge::{UiStoryWorkflowDependency, UiStoryWorkflowEdge};
pub use graph::UiStoryWorkflowGraph;
pub use node::{UiStoryWorkflowNode, UiStoryWorkflowNodePolicy};
pub use profile::UiStoryWorkflowProfile;
pub use topo::UiStoryWorkflowTopologyError;
pub use validate::validate_workflow_graph;
