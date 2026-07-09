use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionDiagnosticCode {
    EmptyDefinition,
    UnsupportedSchemaVersion,
    DuplicateTargetId,
    DuplicateRootId,
    DuplicateRegionId,
    DuplicateMountedUnitId,
    UnknownTarget,
    UnknownRegion,
    UnknownMountedUnit,
    InvalidPrimaryRootCount,
    InvalidRegionParentCount,
    RegionCycle,
    UnreachableRegion,
    EmptyStack,
    DuplicateStackUnit,
    InvalidActiveUnit,
    DuplicateOverlayRegion,
    OverlayContainsBase,
    IdenticalSplitChildren,
    InvalidMountedUnitLocation,
    EmptyTransaction,
    StaleRevision,
    RevisionOverflow,
    DuplicateTransactionId,
    PolicyRejected,
    InvalidCommand,
    HistoryUnavailable,
    HistoryConflict,
    PromotionRejected,
    FixtureExpectationFailed,
}

impl CompositionDiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyDefinition => "ui_composition.formation.empty_definition",
            Self::UnsupportedSchemaVersion => "ui_composition.formation.unsupported_schema_version",
            Self::DuplicateTargetId => "ui_composition.formation.duplicate_target_id",
            Self::DuplicateRootId => "ui_composition.formation.duplicate_root_id",
            Self::DuplicateRegionId => "ui_composition.formation.duplicate_region_id",
            Self::DuplicateMountedUnitId => "ui_composition.formation.duplicate_mounted_unit_id",
            Self::UnknownTarget => "ui_composition.formation.unknown_target",
            Self::UnknownRegion => "ui_composition.formation.unknown_region",
            Self::UnknownMountedUnit => "ui_composition.formation.unknown_mounted_unit",
            Self::InvalidPrimaryRootCount => "ui_composition.formation.invalid_primary_root_count",
            Self::InvalidRegionParentCount => {
                "ui_composition.formation.invalid_region_parent_count"
            }
            Self::RegionCycle => "ui_composition.formation.region_cycle",
            Self::UnreachableRegion => "ui_composition.formation.unreachable_region",
            Self::EmptyStack => "ui_composition.formation.empty_stack",
            Self::DuplicateStackUnit => "ui_composition.formation.duplicate_stack_unit",
            Self::InvalidActiveUnit => "ui_composition.formation.invalid_active_unit",
            Self::DuplicateOverlayRegion => "ui_composition.formation.duplicate_overlay_region",
            Self::OverlayContainsBase => "ui_composition.formation.overlay_contains_base",
            Self::IdenticalSplitChildren => "ui_composition.formation.identical_split_children",
            Self::InvalidMountedUnitLocation => {
                "ui_composition.formation.invalid_mounted_unit_location"
            }
            Self::EmptyTransaction => "ui_composition.transaction.empty",
            Self::StaleRevision => "ui_composition.transaction.stale_revision",
            Self::RevisionOverflow => "ui_composition.transaction.revision_overflow",
            Self::DuplicateTransactionId => "ui_composition.transaction.duplicate_id",
            Self::PolicyRejected => "ui_composition.policy.rejected",
            Self::InvalidCommand => "ui_composition.transaction.invalid_command",
            Self::HistoryUnavailable => "ui_composition.history.unavailable",
            Self::HistoryConflict => "ui_composition.history.conflict",
            Self::PromotionRejected => "ui_composition.promotion.rejected",
            Self::FixtureExpectationFailed => "ui_composition.fixture.expectation_failed",
        }
    }
}
