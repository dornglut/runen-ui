//! Generic embed slot definitions and formed products.

use crate::identity::UiEmbedSlotId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiEmbedSlotRef {
    pub id: UiEmbedSlotId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormedUiEmbed {
    pub slot: UiEmbedSlotId,
}
