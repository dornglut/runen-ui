use serde::{Deserialize, Serialize};

use crate::{MountedUnitId, RegionId, SplitFraction};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionKind {
    Split {
        axis: SplitAxis,
        fraction: SplitFraction,
        first: RegionId,
        second: RegionId,
    },
    Stack {
        ordered_units: Vec<MountedUnitId>,
        active_unit: MountedUnitId,
    },
    Overlay {
        base: RegionId,
        ordered_overlays: Vec<RegionId>,
    },
    MountPoint {
        mounted_unit: MountedUnitId,
    },
}

impl RegionKind {
    pub fn child_regions(&self) -> Vec<RegionId> {
        match self {
            Self::Split { first, second, .. } => vec![*first, *second],
            Self::Overlay {
                base,
                ordered_overlays,
            } => {
                let mut result = Vec::with_capacity(1 + ordered_overlays.len());
                result.push(*base);
                result.extend(ordered_overlays.iter().copied());
                result
            }
            Self::Stack { .. } | Self::MountPoint { .. } => Vec::new(),
        }
    }

    pub fn mounted_units(&self) -> &[MountedUnitId] {
        match self {
            Self::Stack { ordered_units, .. } => ordered_units,
            Self::MountPoint { mounted_unit } => core::slice::from_ref(mounted_unit),
            Self::Split { .. } | Self::Overlay { .. } => &[],
        }
    }
}
