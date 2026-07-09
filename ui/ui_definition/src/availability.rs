//! Generic UI availability state.

use crate::identity::UiAvailabilityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAvailability {
    #[default]
    Available,
    Disabled {
        reason: String,
    },
    Unavailable {
        reason: String,
    },
}

impl UiAvailability {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAvailabilityBinding {
    Static(UiAvailability),
    Ref(UiAvailabilityId),
}
