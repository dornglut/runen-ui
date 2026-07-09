//! UiProgram graph-family contracts.

pub mod accessibility;
pub mod binding;
pub mod control;
pub mod ids;
pub mod inspection;
pub mod interaction;
pub mod layout;
pub mod properties;
pub mod state;
pub mod style;
pub mod visual;

use serde::{Deserialize, Serialize};

pub use accessibility::*;
pub use binding::*;
pub use control::*;
pub use ids::*;
pub use inspection::*;
pub use interaction::*;
pub use layout::*;
pub use properties::*;
pub use state::*;
pub use style::*;
pub use visual::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiProgramGraphs {
    pub control: ControlGraph,
    pub properties: ControlPropertyGraph,
    pub layout: LayoutGraph,
    pub state: StateGraph,
    pub style: StyleGraph,
    pub interaction: InteractionGraph,
    pub binding: BindingGraph,
    pub visual: VisualGraph,
    pub accessibility: AccessibilityGraph,
    pub inspection: InspectionGraph,
}
