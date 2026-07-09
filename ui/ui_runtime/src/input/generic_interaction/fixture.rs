//! Mounted interaction fixture contracts.

use ui_controls::{
    CompiledControlPackage, ControlInteractionDescriptor, ControlInteractionTrigger,
};
use ui_math::{UiPoint, UiRect};

use crate::WidgetId;

/// Mounted, renderer-neutral interaction fixture used by deterministic replay.
///
/// The fixture binds package-backed control descriptors to bounds and local
/// enabled/focusable/read-only flags. It deliberately does not execute
/// app/editor/game commands, mutate product state, create overlays, or own text
/// editing.
#[derive(Debug, Clone, PartialEq)]
pub struct MountedInteractionFixture {
    /// Stable fixture/story id used by replay reports and proof adapters.
    pub mounted_story_id: String,

    /// Mounted controls with package-backed interaction descriptors.
    pub controls: Vec<MountedInteractionControl>,
}

impl MountedInteractionFixture {
    /// Creates an empty deterministic interaction fixture.
    pub fn new(mounted_story_id: impl Into<String>) -> Self {
        Self {
            mounted_story_id: mounted_story_id.into(),
            controls: Vec::new(),
        }
    }

    /// Adds one mounted control to the fixture.
    pub fn with_control(mut self, control: MountedInteractionControl) -> Self {
        self.controls.push(control);
        self
    }

    /// Builds a fixture from compiled package interaction descriptors.
    ///
    /// The compiled package is the authority for interaction declarations. This
    /// panics when a placement references a control kind without a package-level
    /// interaction descriptor so replay proofs cannot silently use fake data.
    pub fn from_compiled_controls(
        mounted_story_id: impl Into<String>,
        compiled: &CompiledControlPackage,
        placements: impl IntoIterator<Item = MountedInteractionPlacement>,
    ) -> Self {
        let mut fixture = Self::new(mounted_story_id);
        for placement in placements {
            let kind_id = ui_controls::ControlKindId::new(placement.control_kind_id.clone());
            let descriptor = compiled
                .package
                .interaction_descriptor(&kind_id)
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "missing package interaction descriptor for {}",
                        placement.control_kind_id
                    )
                });
            let mut control = MountedInteractionControl::new(
                placement.widget_id,
                placement.label,
                placement.bounds,
                descriptor,
            );
            control.enabled = placement.enabled;
            control.focusable = placement.focusable;
            control.read_only = placement.read_only;
            fixture = fixture.with_control(control);
        }
        fixture
    }

    pub(super) fn target_at(&self, point: UiPoint) -> Option<&MountedInteractionControl> {
        self.controls
            .iter()
            .find(|control| control.bounds.contains(point))
    }

    pub(super) fn focusable(&self) -> impl Iterator<Item = &MountedInteractionControl> {
        self.controls.iter().filter(|control| {
            control.enabled
                && control.focusable
                && control
                    .descriptor
                    .requirements
                    .iter()
                    .any(|requirement| requirement.trigger == ControlInteractionTrigger::Focus)
        })
    }

    pub(super) fn control(&self, widget_id: WidgetId) -> Option<&MountedInteractionControl> {
        self.controls
            .iter()
            .find(|control| control.widget_id == widget_id)
    }
}

/// Placement data that binds a package-backed control descriptor to bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct MountedInteractionPlacement {
    /// Widget id assigned to the mounted proof control.
    pub widget_id: WidgetId,

    /// Control kind id that must resolve through the compiled package.
    pub control_kind_id: String,

    /// Human-readable label rendered by proof adapters.
    pub label: String,

    /// Renderer-neutral hit-test bounds for deterministic replay.
    pub bounds: UiRect,

    /// Whether reusable runtime interaction may target this control.
    pub enabled: bool,

    /// Whether focus traversal and explicit focus may target this control.
    pub focusable: bool,

    /// Whether text intent is observed as read-only probe evidence.
    pub read_only: bool,
}

impl MountedInteractionPlacement {
    /// Creates an enabled, focusable placement for a package-backed control.
    pub fn new(
        widget_id: WidgetId,
        control_kind_id: impl Into<String>,
        label: impl Into<String>,
        bounds: UiRect,
    ) -> Self {
        Self {
            widget_id,
            control_kind_id: control_kind_id.into(),
            label: label.into(),
            bounds,
            enabled: true,
            focusable: true,
            read_only: false,
        }
    }

    /// Marks the placement disabled for suppression proof cases.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Marks the placement inert so focus validation can reject it.
    pub fn inert(mut self) -> Self {
        self.focusable = false;
        self
    }

    /// Marks the placement read-only for text-intent probe evidence.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Mounted control plus the package-owned interaction descriptor copy.
#[derive(Debug, Clone, PartialEq)]
pub struct MountedInteractionControl {
    /// Widget id assigned to the mounted proof control.
    pub widget_id: WidgetId,

    /// Package control kind id copied from the interaction descriptor.
    pub control_kind_id: String,

    /// Human-readable proof label.
    pub label: String,

    /// Renderer-neutral bounds used by deterministic pointer hit testing.
    pub bounds: UiRect,

    /// Package-backed reusable interaction declaration for this control.
    pub descriptor: ControlInteractionDescriptor,

    /// Whether replay may form interaction for this control.
    pub enabled: bool,

    /// Whether focus replay may resolve this control.
    pub focusable: bool,

    /// Whether text intent is observed without edit ownership.
    pub read_only: bool,
}

impl MountedInteractionControl {
    /// Creates a mounted control from a package-backed interaction descriptor.
    pub fn new(
        widget_id: WidgetId,
        label: impl Into<String>,
        bounds: UiRect,
        descriptor: ControlInteractionDescriptor,
    ) -> Self {
        Self {
            widget_id,
            control_kind_id: descriptor.control_kind_id.as_str().to_owned(),
            label: label.into(),
            bounds,
            descriptor,
            enabled: true,
            focusable: true,
            read_only: false,
        }
    }

    /// Marks the mounted control disabled for suppression proof cases.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Marks the mounted control inert so focus validation can reject it.
    pub fn inert(mut self) -> Self {
        self.focusable = false;
        self
    }

    /// Marks the mounted control read-only for text-intent probe evidence.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}
