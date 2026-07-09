//! Reusable control overlay declarations.
//!
//! `ui_controls` may describe that a reusable control can request an overlay and
//! which generic policy it needs. It does not open overlays, maintain runtime
//! stacks, execute product commands, mutate app/editor/game state, or edit text.

use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

/// Reusable overlay families that a control descriptor may request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayKind {
    Popup,
    Dropdown,
    Menu,
    Tooltip,
    PickerPopup,
    FocusContainingOverlay,
    DiagnosticOverlay,
}

impl ControlOverlayKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Popup => "popup",
            Self::Dropdown => "dropdown",
            Self::Menu => "menu",
            Self::Tooltip => "tooltip",
            Self::PickerPopup => "picker-popup",
            Self::FocusContainingOverlay => "focus-containing-overlay",
            Self::DiagnosticOverlay => "diagnostic-overlay",
        }
    }
}

/// Generic reusable trigger that may request an overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayTrigger {
    PointerPress,
    PointerHover,
    Focus,
    KeyboardActivate,
    SemanticAction,
}

impl ControlOverlayTrigger {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PointerPress => "pointer-press",
            Self::PointerHover => "pointer-hover",
            Self::Focus => "focus",
            Self::KeyboardActivate => "keyboard-activate",
            Self::SemanticAction => "semantic-action",
        }
    }
}

/// Preferred side for anchored overlay placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayPlacementSide {
    Top,
    Right,
    Bottom,
    Left,
    Center,
    Cursor,
}

impl ControlOverlayPlacementSide {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Center => "center",
            Self::Cursor => "cursor",
        }
    }
}

/// Preferred cross-axis alignment for anchored overlay placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayPlacementAlignment {
    Start,
    Center,
    End,
    Stretch,
}

impl ControlOverlayPlacementAlignment {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
            Self::Stretch => "stretch",
        }
    }
}

/// Collision behavior requested by reusable overlay declarations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayCollisionPolicy {
    None,
    Flip,
    Shift,
    Clamp,
    Resize,
    Hide,
}

impl ControlOverlayCollisionPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Flip => "flip",
            Self::Shift => "shift",
            Self::Clamp => "clamp",
            Self::Resize => "resize",
            Self::Hide => "hide",
        }
    }
}

/// Control-owned placement preference. Runtime resolves this into evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlOverlayPlacementPreference {
    pub side: ControlOverlayPlacementSide,
    pub alignment: ControlOverlayPlacementAlignment,
    pub main_axis_offset: f32,
    pub cross_axis_offset: f32,
    #[serde(default)]
    pub fallback_order: Vec<ControlOverlayPlacementSide>,
    pub collision_policy: ControlOverlayCollisionPolicy,
    pub viewport_margin: f32,
}

impl ControlOverlayPlacementPreference {
    pub fn anchored_below() -> Self {
        Self {
            side: ControlOverlayPlacementSide::Bottom,
            alignment: ControlOverlayPlacementAlignment::Start,
            main_axis_offset: 4.0,
            cross_axis_offset: 0.0,
            fallback_order: vec![
                ControlOverlayPlacementSide::Bottom,
                ControlOverlayPlacementSide::Top,
                ControlOverlayPlacementSide::Right,
                ControlOverlayPlacementSide::Left,
            ],
            collision_policy: ControlOverlayCollisionPolicy::Clamp,
            viewport_margin: 8.0,
        }
    }

    pub fn tooltip() -> Self {
        Self {
            side: ControlOverlayPlacementSide::Top,
            alignment: ControlOverlayPlacementAlignment::Center,
            main_axis_offset: 6.0,
            cross_axis_offset: 0.0,
            fallback_order: vec![
                ControlOverlayPlacementSide::Top,
                ControlOverlayPlacementSide::Bottom,
            ],
            collision_policy: ControlOverlayCollisionPolicy::Clamp,
            viewport_margin: 8.0,
        }
    }
}

/// Reusable layer class requested by a control overlay declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayLayerPreference {
    AnchoredPopup,
    Menu,
    Submenu,
    Tooltip,
    FocusContainingOverlay,
    DiagnosticOverlay,
}

impl ControlOverlayLayerPreference {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AnchoredPopup => "anchored-popup",
            Self::Menu => "menu",
            Self::Submenu => "submenu",
            Self::Tooltip => "tooltip",
            Self::FocusContainingOverlay => "focus-containing-overlay",
            Self::DiagnosticOverlay => "diagnostic-overlay",
        }
    }
}

/// Reusable overlay dismissal behavior. Product lifecycle remains host-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayDismissPolicy {
    Escape,
    OutsidePointer,
    EscapeOrOutsidePointer,
    SelectionIntent,
    ExplicitClose,
    None,
    HostOwned,
}

impl ControlOverlayDismissPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Escape => "escape",
            Self::OutsidePointer => "outside-pointer",
            Self::EscapeOrOutsidePointer => "escape-or-outside-pointer",
            Self::SelectionIntent => "selection-intent",
            Self::ExplicitClose => "explicit-close",
            Self::None => "none",
            Self::HostOwned => "host-owned",
        }
    }
}

/// Reusable focus policy for overlay proof. It does not create app modals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlOverlayFocusPolicy {
    None,
    FocusOverlay,
    ContainFocus,
    ReturnToAnchor,
    ReturnToPrevious,
}

impl ControlOverlayFocusPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::FocusOverlay => "focus-overlay",
            Self::ContainFocus => "contain-focus",
            Self::ReturnToAnchor => "return-to-anchor",
            Self::ReturnToPrevious => "return-to-previous",
        }
    }
}

/// One overlay declaration that a reusable control may expose.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlOverlayRequirement {
    pub kind: ControlOverlayKind,
    pub trigger: ControlOverlayTrigger,
    pub anchor_role: String,
    pub content_role: String,
    pub placement: ControlOverlayPlacementPreference,
    pub layer: ControlOverlayLayerPreference,
    pub dismiss_policy: ControlOverlayDismissPolicy,
    pub focus_policy: ControlOverlayFocusPolicy,
    #[serde(default = "default_suppresses_when_disabled")]
    pub suppresses_when_disabled: bool,
}

impl ControlOverlayRequirement {
    pub fn new(
        kind: ControlOverlayKind,
        trigger: ControlOverlayTrigger,
        anchor_role: impl Into<String>,
        content_role: impl Into<String>,
    ) -> Self {
        let (placement, layer, dismiss_policy, focus_policy) = default_overlay_policy(kind);
        Self {
            kind,
            trigger,
            anchor_role: anchor_role.into(),
            content_role: content_role.into(),
            placement,
            layer,
            dismiss_policy,
            focus_policy,
            suppresses_when_disabled: true,
        }
    }

    pub fn with_layer(mut self, layer: ControlOverlayLayerPreference) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_dismiss_policy(mut self, dismiss_policy: ControlOverlayDismissPolicy) -> Self {
        self.dismiss_policy = dismiss_policy;
        self
    }

    pub fn with_focus_policy(mut self, focus_policy: ControlOverlayFocusPolicy) -> Self {
        self.focus_policy = focus_policy;
        self
    }

    pub fn with_placement(mut self, placement: ControlOverlayPlacementPreference) -> Self {
        self.placement = placement;
        self
    }
}

/// Package-owned overlay declaration for one reusable control kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlOverlayDescriptor {
    pub control_kind_id: ControlKindId,
    #[serde(default)]
    pub requirements: Vec<ControlOverlayRequirement>,
}

impl ControlOverlayDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            requirements: Vec::new(),
        }
    }

    pub fn with_requirement(mut self, requirement: ControlOverlayRequirement) -> Self {
        self.requirements.push(requirement);
        self.requirements.sort_by_key(|requirement| {
            (
                requirement.kind,
                requirement.trigger,
                requirement.layer,
                requirement.anchor_role.clone(),
            )
        });
        self.requirements.dedup_by(|a, b| {
            a.kind == b.kind
                && a.trigger == b.trigger
                && a.layer == b.layer
                && a.anchor_role == b.anchor_role
        });
        self
    }

    pub fn popup_on_press(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        content_role: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::Popup,
            ControlOverlayTrigger::PointerPress,
            anchor_role,
            content_role,
        ))
    }

    pub fn menu_on_press(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        menu_scope: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::Menu,
            ControlOverlayTrigger::PointerPress,
            anchor_role,
            menu_scope,
        ))
    }

    pub fn dropdown_on_press(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        option_scope: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::Dropdown,
            ControlOverlayTrigger::PointerPress,
            anchor_role,
            option_scope,
        ))
    }

    pub fn tooltip_on_hover(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        tooltip_role: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::Tooltip,
            ControlOverlayTrigger::PointerHover,
            anchor_role,
            tooltip_role,
        ))
    }

    pub fn tooltip_on_focus(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        tooltip_role: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::Tooltip,
            ControlOverlayTrigger::Focus,
            anchor_role,
            tooltip_role,
        ))
    }

    pub fn picker_popup_on_press(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        picker_role: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::PickerPopup,
            ControlOverlayTrigger::PointerPress,
            anchor_role,
            picker_role,
        ))
    }

    pub fn focus_containing_overlay_on_press(
        control_kind_id: ControlKindId,
        anchor_role: impl Into<String>,
        focus_scope: impl Into<String>,
    ) -> Self {
        Self::new(control_kind_id).with_requirement(ControlOverlayRequirement::new(
            ControlOverlayKind::FocusContainingOverlay,
            ControlOverlayTrigger::PointerPress,
            anchor_role,
            focus_scope,
        ))
    }

    pub fn summary(&self) -> ControlOverlaySupportSummary {
        ControlOverlaySupportSummary::from_descriptor(self)
    }
}

/// Read-only catalog/inspection summary for overlay support.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlOverlaySupportSummary {
    pub control_kind_id: ControlKindId,
    pub kinds: Vec<String>,
    pub triggers: Vec<String>,
    pub layers: Vec<String>,
    pub dismiss_policies: Vec<String>,
    pub focus_policies: Vec<String>,
    pub overlay_supported: bool,
    pub control_owned_runtime_behavior: bool,
    pub executes_host_commands: bool,
    pub mutates_product_state: bool,
}

impl ControlOverlaySupportSummary {
    pub fn from_descriptor(descriptor: &ControlOverlayDescriptor) -> Self {
        let mut kinds = descriptor
            .requirements
            .iter()
            .map(|requirement| requirement.kind.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut triggers = descriptor
            .requirements
            .iter()
            .map(|requirement| requirement.trigger.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut layers = descriptor
            .requirements
            .iter()
            .map(|requirement| requirement.layer.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut dismiss_policies = descriptor
            .requirements
            .iter()
            .map(|requirement| requirement.dismiss_policy.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut focus_policies = descriptor
            .requirements
            .iter()
            .map(|requirement| requirement.focus_policy.as_str().to_owned())
            .collect::<Vec<_>>();
        sort_dedup(&mut kinds);
        sort_dedup(&mut triggers);
        sort_dedup(&mut layers);
        sort_dedup(&mut dismiss_policies);
        sort_dedup(&mut focus_policies);
        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            kinds,
            triggers,
            layers,
            dismiss_policies,
            focus_policies,
            overlay_supported: !descriptor.requirements.is_empty(),
            control_owned_runtime_behavior: false,
            executes_host_commands: false,
            mutates_product_state: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlOverlayInspectionFact> {
        vec![
            ControlOverlayInspectionFact::new("overlay.kinds", self.kinds.join(",")),
            ControlOverlayInspectionFact::new("overlay.triggers", self.triggers.join(",")),
            ControlOverlayInspectionFact::new("overlay.layers", self.layers.join(",")),
            ControlOverlayInspectionFact::new(
                "overlay.dismiss_policies",
                self.dismiss_policies.join(","),
            ),
            ControlOverlayInspectionFact::new(
                "overlay.focus_policies",
                self.focus_policies.join(","),
            ),
            ControlOverlayInspectionFact::new(
                "overlay.supported",
                bool_string(self.overlay_supported),
            ),
            ControlOverlayInspectionFact::new(
                "overlay.control_owned_runtime_behavior",
                bool_string(self.control_owned_runtime_behavior),
            ),
            ControlOverlayInspectionFact::new(
                "overlay.executes_host_commands",
                bool_string(self.executes_host_commands),
            ),
            ControlOverlayInspectionFact::new(
                "overlay.mutates_product_state",
                bool_string(self.mutates_product_state),
            ),
        ]
    }
}

/// One read-only overlay inspection fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlOverlayInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlOverlayInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn default_overlay_policy(
    kind: ControlOverlayKind,
) -> (
    ControlOverlayPlacementPreference,
    ControlOverlayLayerPreference,
    ControlOverlayDismissPolicy,
    ControlOverlayFocusPolicy,
) {
    match kind {
        ControlOverlayKind::Menu => (
            ControlOverlayPlacementPreference::anchored_below(),
            ControlOverlayLayerPreference::Menu,
            ControlOverlayDismissPolicy::EscapeOrOutsidePointer,
            ControlOverlayFocusPolicy::ReturnToAnchor,
        ),
        ControlOverlayKind::Dropdown => (
            ControlOverlayPlacementPreference::anchored_below(),
            ControlOverlayLayerPreference::AnchoredPopup,
            ControlOverlayDismissPolicy::EscapeOrOutsidePointer,
            ControlOverlayFocusPolicy::ReturnToAnchor,
        ),
        ControlOverlayKind::Tooltip => (
            ControlOverlayPlacementPreference::tooltip(),
            ControlOverlayLayerPreference::Tooltip,
            ControlOverlayDismissPolicy::None,
            ControlOverlayFocusPolicy::None,
        ),
        ControlOverlayKind::PickerPopup => (
            ControlOverlayPlacementPreference::anchored_below(),
            ControlOverlayLayerPreference::AnchoredPopup,
            ControlOverlayDismissPolicy::EscapeOrOutsidePointer,
            ControlOverlayFocusPolicy::ReturnToAnchor,
        ),
        ControlOverlayKind::FocusContainingOverlay => (
            ControlOverlayPlacementPreference::anchored_below(),
            ControlOverlayLayerPreference::FocusContainingOverlay,
            ControlOverlayDismissPolicy::EscapeOrOutsidePointer,
            ControlOverlayFocusPolicy::ContainFocus,
        ),
        ControlOverlayKind::DiagnosticOverlay => (
            ControlOverlayPlacementPreference::anchored_below(),
            ControlOverlayLayerPreference::DiagnosticOverlay,
            ControlOverlayDismissPolicy::None,
            ControlOverlayFocusPolicy::None,
        ),
        ControlOverlayKind::Popup => (
            ControlOverlayPlacementPreference::anchored_below(),
            ControlOverlayLayerPreference::AnchoredPopup,
            ControlOverlayDismissPolicy::EscapeOrOutsidePointer,
            ControlOverlayFocusPolicy::ReturnToAnchor,
        ),
    }
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn default_suppresses_when_disabled() -> bool {
    true
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_builders_expand_to_reportable_defaults_without_product_behavior() {
        let descriptor = ControlOverlayDescriptor::menu_on_press(
            ControlKindId::new("runenwerk.ui.action_prompt"),
            "anchor.action-prompt.menu",
            "scope.action-prompt.menu",
        );
        let summary = descriptor.summary();
        assert_eq!(summary.kinds, vec!["menu"]);
        assert_eq!(summary.triggers, vec!["pointer-press"]);
        assert_eq!(summary.layers, vec!["menu"]);
        assert!(!summary.control_owned_runtime_behavior);
        assert!(!summary.executes_host_commands);
        assert!(!summary.mutates_product_state);
        assert_eq!(
            descriptor.requirements[0].dismiss_policy,
            ControlOverlayDismissPolicy::EscapeOrOutsidePointer
        );
    }

    #[test]
    fn focus_containing_overlay_uses_focus_vocabulary_not_app_modal_lifecycle() {
        let descriptor = ControlOverlayDescriptor::focus_containing_overlay_on_press(
            ControlKindId::new("runenwerk.ui.button"),
            "anchor.button.focus-containing",
            "scope.focus-containing",
        );
        let requirement = &descriptor.requirements[0];
        assert_eq!(requirement.kind, ControlOverlayKind::FocusContainingOverlay);
        assert_eq!(
            requirement.focus_policy,
            ControlOverlayFocusPolicy::ContainFocus
        );
        assert_eq!(requirement.kind.as_str(), "focus-containing-overlay");
    }
}
