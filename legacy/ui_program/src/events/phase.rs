//! UiEvent phase contracts.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiEventPhase {
    Preview,
    Commit,
    #[default]
    Activate,
    Cancel,
}
