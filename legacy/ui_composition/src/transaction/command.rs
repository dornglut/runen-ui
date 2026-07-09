use serde::{Deserialize, Serialize};

use crate::{
    CompositionDefinitionV1, CompositionRootDefinition, CompositionRootId,
    CompositionTransactionId, MountedUnitDefinition, MountedUnitId, PresentationTargetDefinition,
    PresentationTargetId, RegionDefinition, RegionId, SplitAxis, SplitFraction, StateRevision,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionCommand {
    kind: CompositionCommandKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CompositionCommandKind {
    MountUnit {
        unit: MountedUnitDefinition,
        stack: RegionId,
        ordinal: usize,
    },
    UnmountUnit {
        unit: MountedUnitId,
    },
    ActivateUnit {
        stack: RegionId,
        unit: MountedUnitId,
    },
    MoveUnit {
        unit: MountedUnitId,
        stack: RegionId,
        ordinal: usize,
    },
    ReorderStack {
        stack: RegionId,
        ordered_units: Vec<MountedUnitId>,
        active_unit: MountedUnitId,
    },
    SplitRegion {
        region: RegionId,
        preserved_child: RegionId,
        new_region: RegionDefinition,
        axis: SplitAxis,
        fraction: SplitFraction,
        new_region_first: bool,
    },
    SplitRegionWithMovedUnit {
        region: RegionId,
        preserved_child: RegionId,
        moved_region: RegionDefinition,
        unit: MountedUnitId,
        axis: SplitAxis,
        fraction: SplitFraction,
        moved_region_first: bool,
    },
    ResizeSplit {
        region: RegionId,
        fraction: SplitFraction,
    },
    MergeSplit {
        region: RegionId,
        retained_child: RegionId,
    },
    CreateRoot {
        root: CompositionRootDefinition,
        regions: Vec<RegionDefinition>,
    },
    CreateRootWithMovedUnit {
        root: CompositionRootDefinition,
        region: RegionDefinition,
        unit: MountedUnitId,
    },
    MoveRoot {
        root: CompositionRootId,
        target: PresentationTargetId,
        primary: bool,
    },
    CloseRoot {
        root: CompositionRootId,
    },
    AttachTarget {
        target: PresentationTargetDefinition,
    },
    DetachTarget {
        target: PresentationTargetId,
    },
    RatifyExtensionState,
    RestoreDefinition {
        definition: CompositionDefinitionV1,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionCommandView<'a> {
    MountUnit {
        unit: &'a MountedUnitDefinition,
        stack: RegionId,
        ordinal: usize,
    },
    UnmountUnit {
        unit: MountedUnitId,
    },
    ActivateUnit {
        stack: RegionId,
        unit: MountedUnitId,
    },
    MoveUnit {
        unit: MountedUnitId,
        stack: RegionId,
        ordinal: usize,
    },
    ReorderStack {
        stack: RegionId,
        ordered_units: &'a [MountedUnitId],
        active_unit: MountedUnitId,
    },
    SplitRegion {
        region: RegionId,
        preserved_child: RegionId,
        new_region: &'a RegionDefinition,
        axis: SplitAxis,
        fraction: SplitFraction,
        new_region_first: bool,
    },
    SplitRegionWithMovedUnit {
        region: RegionId,
        preserved_child: RegionId,
        moved_region: &'a RegionDefinition,
        unit: MountedUnitId,
        axis: SplitAxis,
        fraction: SplitFraction,
        moved_region_first: bool,
    },
    ResizeSplit {
        region: RegionId,
        fraction: SplitFraction,
    },
    MergeSplit {
        region: RegionId,
        retained_child: RegionId,
    },
    CreateRoot {
        root: &'a CompositionRootDefinition,
        regions: &'a [RegionDefinition],
    },
    CreateRootWithMovedUnit {
        root: &'a CompositionRootDefinition,
        region: &'a RegionDefinition,
        unit: MountedUnitId,
    },
    MoveRoot {
        root: CompositionRootId,
        target: PresentationTargetId,
        primary: bool,
    },
    CloseRoot {
        root: CompositionRootId,
    },
    AttachTarget {
        target: &'a PresentationTargetDefinition,
    },
    DetachTarget {
        target: PresentationTargetId,
    },
    RatifyExtensionState,
    HistoryRestore {
        definition: &'a CompositionDefinitionV1,
    },
}

impl CompositionCommand {
    pub fn mount_unit(unit: MountedUnitDefinition, stack: RegionId, ordinal: usize) -> Self {
        Self::new(CompositionCommandKind::MountUnit {
            unit,
            stack,
            ordinal,
        })
    }

    pub fn unmount_unit(unit: MountedUnitId) -> Self {
        Self::new(CompositionCommandKind::UnmountUnit { unit })
    }

    pub fn activate_unit(stack: RegionId, unit: MountedUnitId) -> Self {
        Self::new(CompositionCommandKind::ActivateUnit { stack, unit })
    }

    pub fn move_unit(unit: MountedUnitId, stack: RegionId, ordinal: usize) -> Self {
        Self::new(CompositionCommandKind::MoveUnit {
            unit,
            stack,
            ordinal,
        })
    }

    pub fn reorder_stack(
        stack: RegionId,
        ordered_units: Vec<MountedUnitId>,
        active_unit: MountedUnitId,
    ) -> Self {
        Self::new(CompositionCommandKind::ReorderStack {
            stack,
            ordered_units,
            active_unit,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn split_region(
        region: RegionId,
        preserved_child: RegionId,
        new_region: RegionDefinition,
        axis: SplitAxis,
        fraction: SplitFraction,
        new_region_first: bool,
    ) -> Self {
        Self::new(CompositionCommandKind::SplitRegion {
            region,
            preserved_child,
            new_region,
            axis,
            fraction,
            new_region_first,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn split_region_with_moved_unit(
        region: RegionId,
        preserved_child: RegionId,
        moved_region: RegionDefinition,
        unit: MountedUnitId,
        axis: SplitAxis,
        fraction: SplitFraction,
        moved_region_first: bool,
    ) -> Self {
        Self::new(CompositionCommandKind::SplitRegionWithMovedUnit {
            region,
            preserved_child,
            moved_region,
            unit,
            axis,
            fraction,
            moved_region_first,
        })
    }

    pub fn resize_split(region: RegionId, fraction: SplitFraction) -> Self {
        Self::new(CompositionCommandKind::ResizeSplit { region, fraction })
    }

    pub fn merge_split(region: RegionId, retained_child: RegionId) -> Self {
        Self::new(CompositionCommandKind::MergeSplit {
            region,
            retained_child,
        })
    }

    pub fn create_root(root: CompositionRootDefinition, regions: Vec<RegionDefinition>) -> Self {
        Self::new(CompositionCommandKind::CreateRoot { root, regions })
    }

    pub fn create_root_with_moved_unit(
        root: CompositionRootDefinition,
        region: RegionDefinition,
        unit: MountedUnitId,
    ) -> Self {
        Self::new(CompositionCommandKind::CreateRootWithMovedUnit { root, region, unit })
    }

    pub fn move_root(root: CompositionRootId, target: PresentationTargetId, primary: bool) -> Self {
        Self::new(CompositionCommandKind::MoveRoot {
            root,
            target,
            primary,
        })
    }

    pub fn close_root(root: CompositionRootId) -> Self {
        Self::new(CompositionCommandKind::CloseRoot { root })
    }

    pub fn attach_target(target: PresentationTargetDefinition) -> Self {
        Self::new(CompositionCommandKind::AttachTarget { target })
    }

    pub fn detach_target(target: PresentationTargetId) -> Self {
        Self::new(CompositionCommandKind::DetachTarget { target })
    }

    /// Advances the linked definition revision for an app-owned extension-only mutation.
    pub fn ratify_extension_state() -> Self {
        Self::new(CompositionCommandKind::RatifyExtensionState)
    }

    pub fn view(&self) -> CompositionCommandView<'_> {
        match &self.kind {
            CompositionCommandKind::MountUnit {
                unit,
                stack,
                ordinal,
            } => CompositionCommandView::MountUnit {
                unit,
                stack: *stack,
                ordinal: *ordinal,
            },
            CompositionCommandKind::UnmountUnit { unit } => {
                CompositionCommandView::UnmountUnit { unit: *unit }
            }
            CompositionCommandKind::ActivateUnit { stack, unit } => {
                CompositionCommandView::ActivateUnit {
                    stack: *stack,
                    unit: *unit,
                }
            }
            CompositionCommandKind::MoveUnit {
                unit,
                stack,
                ordinal,
            } => CompositionCommandView::MoveUnit {
                unit: *unit,
                stack: *stack,
                ordinal: *ordinal,
            },
            CompositionCommandKind::ReorderStack {
                stack,
                ordered_units,
                active_unit,
            } => CompositionCommandView::ReorderStack {
                stack: *stack,
                ordered_units,
                active_unit: *active_unit,
            },
            CompositionCommandKind::SplitRegion {
                region,
                preserved_child,
                new_region,
                axis,
                fraction,
                new_region_first,
            } => CompositionCommandView::SplitRegion {
                region: *region,
                preserved_child: *preserved_child,
                new_region,
                axis: *axis,
                fraction: *fraction,
                new_region_first: *new_region_first,
            },
            CompositionCommandKind::SplitRegionWithMovedUnit {
                region,
                preserved_child,
                moved_region,
                unit,
                axis,
                fraction,
                moved_region_first,
            } => CompositionCommandView::SplitRegionWithMovedUnit {
                region: *region,
                preserved_child: *preserved_child,
                moved_region,
                unit: *unit,
                axis: *axis,
                fraction: *fraction,
                moved_region_first: *moved_region_first,
            },
            CompositionCommandKind::ResizeSplit { region, fraction } => {
                CompositionCommandView::ResizeSplit {
                    region: *region,
                    fraction: *fraction,
                }
            }
            CompositionCommandKind::MergeSplit {
                region,
                retained_child,
            } => CompositionCommandView::MergeSplit {
                region: *region,
                retained_child: *retained_child,
            },
            CompositionCommandKind::CreateRoot { root, regions } => {
                CompositionCommandView::CreateRoot { root, regions }
            }
            CompositionCommandKind::CreateRootWithMovedUnit { root, region, unit } => {
                CompositionCommandView::CreateRootWithMovedUnit {
                    root,
                    region,
                    unit: *unit,
                }
            }
            CompositionCommandKind::MoveRoot {
                root,
                target,
                primary,
            } => CompositionCommandView::MoveRoot {
                root: *root,
                target: *target,
                primary: *primary,
            },
            CompositionCommandKind::CloseRoot { root } => {
                CompositionCommandView::CloseRoot { root: *root }
            }
            CompositionCommandKind::AttachTarget { target } => {
                CompositionCommandView::AttachTarget { target }
            }
            CompositionCommandKind::DetachTarget { target } => {
                CompositionCommandView::DetachTarget { target: *target }
            }
            CompositionCommandKind::RatifyExtensionState => {
                CompositionCommandView::RatifyExtensionState
            }
            CompositionCommandKind::RestoreDefinition { definition } => {
                CompositionCommandView::HistoryRestore { definition }
            }
        }
    }

    pub(crate) fn restore_definition(definition: CompositionDefinitionV1) -> Self {
        Self::new(CompositionCommandKind::RestoreDefinition { definition })
    }

    pub(crate) fn kind(&self) -> &CompositionCommandKind {
        &self.kind
    }

    pub(crate) fn is_history_restore(&self) -> bool {
        matches!(self.kind, CompositionCommandKind::RestoreDefinition { .. })
    }

    const fn new(kind: CompositionCommandKind) -> Self {
        Self { kind }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionTransaction {
    id: CompositionTransactionId,
    expected_revision: StateRevision,
    commands: Vec<CompositionCommand>,
}

impl CompositionTransaction {
    pub fn new(
        id: CompositionTransactionId,
        expected_revision: StateRevision,
        commands: Vec<CompositionCommand>,
    ) -> Self {
        Self {
            id,
            expected_revision,
            commands,
        }
    }
    pub const fn id(&self) -> CompositionTransactionId {
        self.id
    }
    pub const fn expected_revision(&self) -> StateRevision {
        self.expected_revision
    }
    pub fn commands(&self) -> &[CompositionCommand] {
        &self.commands
    }

    pub(crate) fn commands_cloned(&self) -> Vec<CompositionCommand> {
        self.commands.clone()
    }
}
