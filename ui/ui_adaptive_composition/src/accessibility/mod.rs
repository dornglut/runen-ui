//! Headless accessibility metadata for adaptive composition.

use ui_composition::{MountedUnitId, PresentationTargetId, RegionId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdaptiveInspectionRole {
    Target,
    Region,
    Tab,
    Panel,
    DockTarget,
    Preview,
    Drawer,
    UnavailableContent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveAccessibilityNode {
    pub target: PresentationTargetId,
    pub region: RegionId,
    pub mounted_unit: Option<MountedUnitId>,
    pub role: AdaptiveInspectionRole,
    pub label: String,
    pub focus_order: usize,
    pub focus_visible: bool,
    pub high_contrast: bool,
    pub text_scale: f32,
    pub minimum_touch_target: f32,
    pub transition_duration_ms: u16,
}

impl AdaptiveAccessibilityNode {
    pub fn is_complete(&self) -> bool {
        !self.label.trim().is_empty()
            && self.text_scale.is_finite()
            && self.text_scale > 0.0
            && self.minimum_touch_target.is_finite()
            && self.minimum_touch_target >= 24.0
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdaptiveAccessibilityProjection {
    nodes: Vec<AdaptiveAccessibilityNode>,
}

impl AdaptiveAccessibilityProjection {
    pub fn new(mut nodes: Vec<AdaptiveAccessibilityNode>) -> Self {
        nodes.sort_by_key(|node| (node.target, node.focus_order, node.region));
        Self { nodes }
    }

    pub fn nodes(&self) -> &[AdaptiveAccessibilityNode] {
        &self.nodes
    }

    pub fn is_complete(&self) -> bool {
        !self.nodes.is_empty()
            && self
                .nodes
                .iter()
                .all(AdaptiveAccessibilityNode::is_complete)
    }
}
