//! Visual layout operation definitions.

use crate::{
    AuthoredId, AuthoredUiNodePath, UiAxisDefinition, UiNodeDefinition, UiNodeId, UiSourceLocation,
    UiTemplateId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub type UiVisualLayoutOperationId = AuthoredId;
pub type UiTargetProfileId = AuthoredId;
pub type UiVisualLayoutHostId = AuthoredId;
pub type UiVisualLayoutSuiteId = AuthoredId;
pub type UiVisualLayoutSurfaceId = AuthoredId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiVisualLayoutActivationMode {
    Preview,
    Activate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiVisualLayoutOperation {
    pub id: UiVisualLayoutOperationId,
    pub source_document: UiTemplateId,
    pub target_path: AuthoredUiNodePath,
    pub expected_node_id: UiNodeId,
    pub target_profile: UiTargetProfileId,
    pub kind: UiVisualLayoutEditKind,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub preview_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiVisualLayoutEditKind {
    InsertNode {
        index: usize,
        node: UiNodeDefinition,
    },
    RemoveNode,
    MoveNode {
        new_parent_path: AuthoredUiNodePath,
        new_index: usize,
    },
    ReorderSibling {
        from_index: usize,
        to_index: usize,
        expected_child_id: UiNodeId,
    },
    ChangeStackAxis {
        axis: UiAxisDefinition,
    },
    ChangeSplitRatio {
        ratio: f32,
    },
    WrapSelectionInContainer {
        first_index: usize,
        count: usize,
        container: UiNodeDefinition,
    },
    UnwrapContainer,
    ReplaceTemplateReference {
        template: UiTemplateId,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiVisualLayoutEditContext {
    pub supported_target_profiles: BTreeSet<UiTargetProfileId>,
    #[serde(default)]
    pub host: Option<UiVisualLayoutHostId>,
    #[serde(default)]
    pub suite: Option<UiVisualLayoutSuiteId>,
    #[serde(default)]
    pub surface: Option<UiVisualLayoutSurfaceId>,
}

impl UiVisualLayoutEditContext {
    pub fn unrestricted() -> Self {
        Self::default()
    }

    pub fn with_supported_target_profiles(
        profiles: impl IntoIterator<Item = UiTargetProfileId>,
    ) -> Self {
        Self {
            supported_target_profiles: profiles.into_iter().collect(),
            host: None,
            suite: None,
            surface: None,
        }
    }

    pub fn with_target_surface(
        mut self,
        host: Option<UiVisualLayoutHostId>,
        suite: Option<UiVisualLayoutSuiteId>,
        surface: Option<UiVisualLayoutSurfaceId>,
    ) -> Self {
        self.host = host;
        self.suite = suite;
        self.surface = surface;
        self
    }

    pub fn supports_target_profile(&self, target_profile: &UiTargetProfileId) -> bool {
        self.supported_target_profiles.is_empty()
            || self.supported_target_profiles.contains(target_profile)
    }
}
