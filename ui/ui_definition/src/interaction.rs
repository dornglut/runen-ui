//! Interaction V2 formation contracts.

use crate::UiNodeId;
use serde::{Deserialize, Serialize};
use ui_math::Axis;
use ui_tree::WidgetId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiScrollBoundaryPolicyDefinition {
    #[default]
    ConsumeAtBoundary,
    PropagateAtBoundary,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiScrollOwnershipDefinition {
    #[serde(default)]
    pub boundary: UiScrollBoundaryPolicyDefinition,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMenuDismissPolicyDefinition {
    #[default]
    OutsidePointerDown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMenuFocusReturnDefinition {
    #[default]
    Anchor,
    Widget(UiNodeId),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMenuStackScopeDefinition {
    pub id: String,
    pub anchor: UiNodeId,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub dismiss: UiMenuDismissPolicyDefinition,
    #[serde(default)]
    pub focus_return: UiMenuFocusReturnDefinition,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormedInteractionModel {
    pub chrome_slots: Vec<FormedChromeSlot>,
    pub dock_drop_zones: Vec<FormedDockDropZone>,
    pub menu_scopes: Vec<FormedMenuStackScope>,
    pub menu_sizing: Vec<FormedMenuSizing>,
    pub scroll_owners: Vec<FormedScrollOwner>,
    pub viewport_status_regions: Vec<FormedViewportStatusRegion>,
}

impl FormedInteractionModel {
    pub fn extend(&mut self, other: FormedInteractionModel) {
        self.chrome_slots.extend(other.chrome_slots);
        self.dock_drop_zones.extend(other.dock_drop_zones);
        self.menu_scopes.extend(other.menu_scopes);
        self.menu_sizing.extend(other.menu_sizing);
        self.scroll_owners.extend(other.scroll_owners);
        self.viewport_status_regions
            .extend(other.viewport_status_regions);
    }

    pub fn push_chrome_slot(&mut self, slot: FormedChromeSlot) {
        self.chrome_slots.push(slot);
    }

    pub fn push_dock_drop_zone(&mut self, zone: FormedDockDropZone) {
        self.dock_drop_zones.push(zone);
    }

    pub fn push_menu_scope(&mut self, scope: FormedMenuStackScope) {
        self.menu_scopes.push(scope);
    }

    pub fn push_menu_sizing(&mut self, sizing: FormedMenuSizing) {
        self.menu_sizing.push(sizing);
    }

    pub fn push_scroll_owner(&mut self, owner: FormedScrollOwner) {
        self.scroll_owners.push(owner);
    }

    pub fn push_viewport_status_region(&mut self, region: FormedViewportStatusRegion) {
        self.viewport_status_regions.push(region);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedChromeSlot {
    pub host_widget_id: WidgetId,
    pub slot_widget_id: WidgetId,
    pub kind: UiChromeSlotKindDefinition,
    pub input_policy: UiChromeSlotInputPolicyDefinition,
    pub order: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiChromeSlotKindDefinition {
    CloseAffordance,
    ActiveIndicator,
    Label,
    CommandArea,
    DragRegion,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiChromeSlotInputPolicyDefinition {
    #[default]
    None,
    Activate,
    Command,
    Drag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedDockDropZone {
    pub zone_widget_id: WidgetId,
    pub anchor_widget_id: WidgetId,
    pub kind: UiDockDropZoneKindDefinition,
    pub scope: UiDockDropScopeDefinition,
    pub side: Option<UiDockDropSideDefinition>,
    pub state: UiDockDropZoneStateDefinition,
    pub priority: u16,
    pub preview_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiDockDropZoneKindDefinition {
    TabReorder,
    SplitInsertion,
    FloatingHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiDockDropScopeDefinition {
    Area,
    Group,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiDockDropSideDefinition {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiDockDropZoneStateDefinition {
    Candidate,
    Active,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedMenuStackScope {
    pub scope_id: String,
    pub popup_widget_id: WidgetId,
    pub anchor_widget_id: WidgetId,
    pub parent_scope_id: Option<String>,
    pub dismiss: UiMenuDismissPolicyDefinition,
    pub focus_return: Option<WidgetId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedMenuSizing {
    pub popup_widget_id: WidgetId,
    pub list_widget_id: WidgetId,
    pub item_width: UiMenuItemWidthDefinition,
    pub overflow: UiMenuOverflowDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedScrollOwner {
    pub widget_id: WidgetId,
    pub axes: Vec<Axis>,
    pub boundary: UiScrollBoundaryPolicyDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedViewportStatusRegion {
    pub status_widget_id: WidgetId,
    pub viewport_canvas_widget_id: WidgetId,
    pub viewport_surface_widget_id: WidgetId,
    pub overflow: UiStatusOverflowPolicyDefinition,
    pub input_arbitration: UiViewportInputArbitrationPolicyDefinition,
    pub metrics: Vec<FormedViewportStatusMetric>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedViewportStatusMetric {
    pub widget_id: WidgetId,
    pub kind: UiViewportStatusMetricKindDefinition,
    pub priority: UiViewportStatusMetricPriorityDefinition,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiStatusOverflowPolicyDefinition {
    #[default]
    SingleRowHorizontalScroll,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiViewportInputArbitrationPolicyDefinition {
    #[default]
    UiOwnsStatusBeforeViewportFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiViewportStatusMetricKindDefinition {
    Details,
    FrameRate,
    FrameTime,
    OverlayStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiViewportStatusMetricPriorityDefinition {
    Essential,
    Supplemental,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMenuItemWidthDefinition {
    #[default]
    FillToMenuWidth,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMenuOverflowDefinition {
    #[default]
    ScrollWhenClamped,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMenuSizingDefinition {
    #[serde(default)]
    pub item_width: UiMenuItemWidthDefinition,
    #[serde(default)]
    pub overflow: UiMenuOverflowDefinition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_model_extends_viewport_status_regions() {
        let mut base = FormedInteractionModel::default();
        let mut incoming = FormedInteractionModel::default();
        incoming.push_viewport_status_region(FormedViewportStatusRegion {
            status_widget_id: WidgetId(1),
            viewport_canvas_widget_id: WidgetId(2),
            viewport_surface_widget_id: WidgetId(3),
            overflow: UiStatusOverflowPolicyDefinition::SingleRowHorizontalScroll,
            input_arbitration:
                UiViewportInputArbitrationPolicyDefinition::UiOwnsStatusBeforeViewportFallback,
            metrics: vec![FormedViewportStatusMetric {
                widget_id: WidgetId(4),
                kind: UiViewportStatusMetricKindDefinition::FrameRate,
                priority: UiViewportStatusMetricPriorityDefinition::Essential,
            }],
        });

        base.extend(incoming);

        assert_eq!(base.viewport_status_regions.len(), 1);
        assert_eq!(base.viewport_status_regions[0].metrics.len(), 1);
    }
}
