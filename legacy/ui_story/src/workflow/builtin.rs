use serde::{Deserialize, Serialize};

use crate::identity::{UiStoryWorkflowNodeId, UiStoryWorkflowProfileId};

use super::{
    UiStoryWorkflowEdge, UiStoryWorkflowGraph, UiStoryWorkflowNode, UiStoryWorkflowProfile,
};

pub const WORKFLOW_SOURCE_LOAD_ONLY: &str = "ui_story.workflow.source_load_only";
pub const WORKFLOW_COMPILER_ONLY: &str = "ui_story.workflow.compiler_only";
pub const WORKFLOW_STATIC_PREVIEW: &str = "ui_story.workflow.static_preview";
pub const WORKFLOW_EXECUTABLE_INTERACTION_PROOF: &str =
    "ui_story.workflow.executable_interaction_proof";

pub const NODE_MANIFEST: &str = "manifest";
pub const NODE_SOURCE_LOAD: &str = "source_load";
pub const NODE_SOURCE_PARSE: &str = "source_parse";
pub const NODE_PROGRAM_FORMATION: &str = "program_formation";
pub const NODE_COMPILER: &str = "compiler";
pub const NODE_RUNTIME_VIEW: &str = "runtime_view";
pub const NODE_RENDER_PRIMITIVES: &str = "render_primitives";
pub const NODE_RENDER_DATA: &str = "render_data";
pub const NODE_STATIC_MOUNT: &str = "static_mount";
pub const NODE_PREVIEW_FRAME: &str = "preview_frame";
pub const NODE_INTERACTION_STORY: &str = "interaction_story";
pub const NODE_INTERACTION_REPLAY: &str = "interaction_replay";
pub const NODE_LIVE_INTERACTION_PROOF: &str = "live_interaction_proof";
pub const NODE_REPLAY_LIVE_PARITY: &str = "replay_live_parity";
pub const NODE_INTERACTION_STATIC_MOUNT: &str = "interaction_static_mount";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryBuiltinWorkflowProfile {
    SourceLoadOnly,
    CompilerOnly,
    StaticPreview,
    ExecutableInteractionProof,
}

impl UiStoryBuiltinWorkflowProfile {
    pub const fn all() -> UiStoryBuiltinWorkflowProfiles {
        UiStoryBuiltinWorkflowProfiles { index: 0 }
    }

    pub fn profile_id(self) -> UiStoryWorkflowProfileId {
        match self {
            Self::SourceLoadOnly => UiStoryWorkflowProfileId::new(WORKFLOW_SOURCE_LOAD_ONLY),
            Self::CompilerOnly => UiStoryWorkflowProfileId::new(WORKFLOW_COMPILER_ONLY),
            Self::StaticPreview => UiStoryWorkflowProfileId::new(WORKFLOW_STATIC_PREVIEW),
            Self::ExecutableInteractionProof => {
                UiStoryWorkflowProfileId::new(WORKFLOW_EXECUTABLE_INTERACTION_PROOF)
            }
        }
    }

    pub fn profile(self) -> UiStoryWorkflowProfile {
        let graph = self.graph();
        UiStoryWorkflowProfile::new(graph.profile_id.clone(), graph)
    }

    pub fn graph(self) -> UiStoryWorkflowGraph {
        match self {
            Self::SourceLoadOnly => source_load_only_graph(),
            Self::CompilerOnly => compiler_only_graph(),
            Self::StaticPreview => static_preview_graph(),
            Self::ExecutableInteractionProof => executable_interaction_proof_graph(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UiStoryBuiltinWorkflowProfiles {
    index: usize,
}

impl Iterator for UiStoryBuiltinWorkflowProfiles {
    type Item = UiStoryBuiltinWorkflowProfile;

    fn next(&mut self) -> Option<Self::Item> {
        let profile = match self.index {
            0 => UiStoryBuiltinWorkflowProfile::SourceLoadOnly,
            1 => UiStoryBuiltinWorkflowProfile::CompilerOnly,
            2 => UiStoryBuiltinWorkflowProfile::StaticPreview,
            3 => UiStoryBuiltinWorkflowProfile::ExecutableInteractionProof,
            _ => return None,
        };
        self.index += 1;
        Some(profile)
    }
}

fn source_load_only_graph() -> UiStoryWorkflowGraph {
    UiStoryWorkflowGraph::new(
        UiStoryWorkflowProfileId::new(WORKFLOW_SOURCE_LOAD_ONLY),
        [
            manifest_node(),
            UiStoryWorkflowNode::required(NODE_SOURCE_LOAD, "Source load"),
        ],
        [UiStoryWorkflowEdge::requires_completed(
            NODE_MANIFEST,
            NODE_SOURCE_LOAD,
        )],
        UiStoryWorkflowNodeId::new(NODE_SOURCE_LOAD),
    )
}

fn compiler_only_graph() -> UiStoryWorkflowGraph {
    UiStoryWorkflowGraph::new(
        UiStoryWorkflowProfileId::new(WORKFLOW_COMPILER_ONLY),
        [
            manifest_node(),
            UiStoryWorkflowNode::required(NODE_SOURCE_LOAD, "Source load"),
            UiStoryWorkflowNode::required(NODE_SOURCE_PARSE, "Source parse"),
            UiStoryWorkflowNode::required(NODE_PROGRAM_FORMATION, "Program formation"),
            UiStoryWorkflowNode::required(NODE_COMPILER, "Compiler"),
        ],
        [
            UiStoryWorkflowEdge::requires_completed(NODE_MANIFEST, NODE_SOURCE_LOAD),
            UiStoryWorkflowEdge::requires_completed(NODE_SOURCE_LOAD, NODE_SOURCE_PARSE),
            UiStoryWorkflowEdge::requires_completed(NODE_SOURCE_PARSE, NODE_PROGRAM_FORMATION),
            UiStoryWorkflowEdge::requires_completed(NODE_PROGRAM_FORMATION, NODE_COMPILER),
        ],
        UiStoryWorkflowNodeId::new(NODE_COMPILER),
    )
}

fn static_preview_graph() -> UiStoryWorkflowGraph {
    UiStoryWorkflowGraph::new(
        UiStoryWorkflowProfileId::new(WORKFLOW_STATIC_PREVIEW),
        [
            manifest_node(),
            UiStoryWorkflowNode::required(NODE_SOURCE_LOAD, "Source load"),
            UiStoryWorkflowNode::required(NODE_SOURCE_PARSE, "Source parse"),
            UiStoryWorkflowNode::required(NODE_PROGRAM_FORMATION, "Program formation"),
            UiStoryWorkflowNode::required(NODE_COMPILER, "Compiler"),
            UiStoryWorkflowNode::required(NODE_RUNTIME_VIEW, "Runtime view"),
            UiStoryWorkflowNode::required(NODE_RENDER_PRIMITIVES, "Render primitives"),
            UiStoryWorkflowNode::required(NODE_RENDER_DATA, "Render data"),
            UiStoryWorkflowNode::required(NODE_STATIC_MOUNT, "Static mount"),
            UiStoryWorkflowNode::required(NODE_PREVIEW_FRAME, "Preview frame"),
        ],
        [
            UiStoryWorkflowEdge::requires_completed(NODE_MANIFEST, NODE_SOURCE_LOAD),
            UiStoryWorkflowEdge::requires_completed(NODE_SOURCE_LOAD, NODE_SOURCE_PARSE),
            UiStoryWorkflowEdge::requires_completed(NODE_SOURCE_PARSE, NODE_PROGRAM_FORMATION),
            UiStoryWorkflowEdge::requires_completed(NODE_PROGRAM_FORMATION, NODE_COMPILER),
            UiStoryWorkflowEdge::requires_completed(NODE_COMPILER, NODE_RUNTIME_VIEW),
            UiStoryWorkflowEdge::requires_completed(NODE_RUNTIME_VIEW, NODE_RENDER_PRIMITIVES),
            UiStoryWorkflowEdge::requires_completed(NODE_RENDER_PRIMITIVES, NODE_RENDER_DATA),
            UiStoryWorkflowEdge::requires_completed(NODE_RENDER_DATA, NODE_STATIC_MOUNT),
            UiStoryWorkflowEdge::requires_completed(NODE_STATIC_MOUNT, NODE_PREVIEW_FRAME),
        ],
        UiStoryWorkflowNodeId::new(NODE_PREVIEW_FRAME),
    )
}

fn executable_interaction_proof_graph() -> UiStoryWorkflowGraph {
    UiStoryWorkflowGraph::new(
        UiStoryWorkflowProfileId::new(WORKFLOW_EXECUTABLE_INTERACTION_PROOF),
        [
            manifest_node(),
            UiStoryWorkflowNode::required(NODE_SOURCE_LOAD, "Source load"),
            UiStoryWorkflowNode::required(NODE_SOURCE_PARSE, "Source parse"),
            UiStoryWorkflowNode::required(NODE_PROGRAM_FORMATION, "Program formation"),
            UiStoryWorkflowNode::required(NODE_COMPILER, "Compiler"),
            UiStoryWorkflowNode::required(NODE_RUNTIME_VIEW, "Runtime view"),
            UiStoryWorkflowNode::required(NODE_INTERACTION_STORY, "Interaction story"),
            UiStoryWorkflowNode::required(NODE_INTERACTION_REPLAY, "Interaction replay"),
            UiStoryWorkflowNode::required(NODE_LIVE_INTERACTION_PROOF, "Live interaction proof"),
            UiStoryWorkflowNode::required(NODE_REPLAY_LIVE_PARITY, "Replay live parity"),
            UiStoryWorkflowNode::required(
                NODE_INTERACTION_STATIC_MOUNT,
                "Interaction static mount",
            ),
            UiStoryWorkflowNode::required(NODE_PREVIEW_FRAME, "Preview frame"),
        ],
        [
            UiStoryWorkflowEdge::requires_completed(NODE_MANIFEST, NODE_SOURCE_LOAD),
            UiStoryWorkflowEdge::requires_completed(NODE_SOURCE_LOAD, NODE_SOURCE_PARSE),
            UiStoryWorkflowEdge::requires_completed(NODE_SOURCE_PARSE, NODE_PROGRAM_FORMATION),
            UiStoryWorkflowEdge::requires_completed(NODE_PROGRAM_FORMATION, NODE_COMPILER),
            UiStoryWorkflowEdge::requires_completed(NODE_COMPILER, NODE_RUNTIME_VIEW),
            UiStoryWorkflowEdge::requires_completed(NODE_RUNTIME_VIEW, NODE_INTERACTION_STORY),
            UiStoryWorkflowEdge::requires_completed(
                NODE_INTERACTION_STORY,
                NODE_INTERACTION_REPLAY,
            ),
            UiStoryWorkflowEdge::requires_completed(
                NODE_INTERACTION_STORY,
                NODE_LIVE_INTERACTION_PROOF,
            ),
            UiStoryWorkflowEdge::requires_completed(
                NODE_INTERACTION_REPLAY,
                NODE_REPLAY_LIVE_PARITY,
            ),
            UiStoryWorkflowEdge::requires_completed(
                NODE_LIVE_INTERACTION_PROOF,
                NODE_REPLAY_LIVE_PARITY,
            ),
            UiStoryWorkflowEdge::requires_completed(
                NODE_REPLAY_LIVE_PARITY,
                NODE_INTERACTION_STATIC_MOUNT,
            ),
            UiStoryWorkflowEdge::requires_completed(
                NODE_INTERACTION_STATIC_MOUNT,
                NODE_PREVIEW_FRAME,
            ),
        ],
        UiStoryWorkflowNodeId::new(NODE_PREVIEW_FRAME),
    )
}

fn manifest_node() -> UiStoryWorkflowNode {
    UiStoryWorkflowNode::derived(NODE_MANIFEST, "Manifest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_profiles_validate() {
        for profile in UiStoryBuiltinWorkflowProfile::all() {
            let graph = profile.graph();
            assert!(
                graph.validate().is_empty(),
                "profile {:?} should validate: {:?}",
                profile,
                graph.validate()
            );
        }
    }

    #[test]
    fn builtin_profile_ids_are_stable() {
        assert_eq!(
            UiStoryBuiltinWorkflowProfile::SourceLoadOnly
                .profile_id()
                .as_str(),
            WORKFLOW_SOURCE_LOAD_ONLY
        );
        assert_eq!(
            UiStoryBuiltinWorkflowProfile::CompilerOnly
                .profile_id()
                .as_str(),
            WORKFLOW_COMPILER_ONLY
        );
        assert_eq!(
            UiStoryBuiltinWorkflowProfile::StaticPreview
                .profile_id()
                .as_str(),
            WORKFLOW_STATIC_PREVIEW
        );
        assert_eq!(
            UiStoryBuiltinWorkflowProfile::ExecutableInteractionProof
                .profile_id()
                .as_str(),
            WORKFLOW_EXECUTABLE_INTERACTION_PROOF
        );
    }

    #[test]
    fn static_preview_order_is_the_story_readiness_spine() {
        let graph = UiStoryBuiltinWorkflowProfile::StaticPreview.graph();
        let order = graph
            .topological_nodes()
            .expect("static preview should be valid");

        assert_eq!(
            order
                .iter()
                .map(|node| node.node_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                NODE_MANIFEST,
                NODE_SOURCE_LOAD,
                NODE_SOURCE_PARSE,
                NODE_PROGRAM_FORMATION,
                NODE_COMPILER,
                NODE_RUNTIME_VIEW,
                NODE_RENDER_PRIMITIVES,
                NODE_RENDER_DATA,
                NODE_STATIC_MOUNT,
                NODE_PREVIEW_FRAME,
            ]
        );
    }

    #[test]
    fn executable_interaction_proof_order_is_stable() {
        let graph = UiStoryBuiltinWorkflowProfile::ExecutableInteractionProof.graph();
        let order = graph
            .topological_nodes()
            .expect("executable interaction proof should be valid");

        assert_eq!(
            order
                .iter()
                .map(|node| node.node_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                NODE_MANIFEST,
                NODE_SOURCE_LOAD,
                NODE_SOURCE_PARSE,
                NODE_PROGRAM_FORMATION,
                NODE_COMPILER,
                NODE_RUNTIME_VIEW,
                NODE_INTERACTION_STORY,
                NODE_INTERACTION_REPLAY,
                NODE_LIVE_INTERACTION_PROOF,
                NODE_REPLAY_LIVE_PARITY,
                NODE_INTERACTION_STATIC_MOUNT,
                NODE_PREVIEW_FRAME,
            ]
        );
    }

    #[test]
    fn builtin_iterator_lists_all_profiles_in_stable_order() {
        let ids = UiStoryBuiltinWorkflowProfile::all()
            .map(UiStoryBuiltinWorkflowProfile::profile_id)
            .map(|profile_id| profile_id.as_str().to_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                WORKFLOW_SOURCE_LOAD_ONLY,
                WORKFLOW_COMPILER_ONLY,
                WORKFLOW_STATIC_PREVIEW,
                WORKFLOW_EXECUTABLE_INTERACTION_PROOF,
            ]
        );
    }
}
