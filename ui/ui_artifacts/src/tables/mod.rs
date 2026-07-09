use super::*;

mod accessibility;
mod binding_snapshot;
mod control;
mod inspection;
mod interaction;
mod layout;
mod properties;
mod state;
mod style;
mod text_layout;
mod visual;

pub use accessibility::*;
pub use binding_snapshot::*;
pub use control::*;
pub use inspection::*;
pub use interaction::*;
pub use layout::*;
pub use properties::*;
pub use state::*;
pub use style::*;
pub use text_layout::*;
pub use visual::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeArtifactTables {
    pub controls: ControlTable,
    pub properties: ControlPropertyTable,
    pub layout: LayoutPlanTable,
    pub style: StyleResolutionTable,
    pub state: StateTable,
    pub interaction: InteractionDispatchTable,
    pub binding_snapshots: BindingSnapshotTable,
    pub collection_diffs: CollectionDiffPlan,
    pub visual: VisualOperatorTable,
    pub text_layout_requests: TextLayoutRequestTable,
    pub accessibility: AccessibilityTable,
    pub inspection: InspectionTable,
}

impl UiRuntimeArtifactTables {
    pub fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            controls: ControlTable::from_program(program, source_map),
            properties: ControlPropertyTable::from_program(program, source_map),
            layout: LayoutPlanTable::from_program(program, source_map),
            style: StyleResolutionTable::from_program(program, source_map),
            state: StateTable::from_program(program, source_map),
            interaction: InteractionDispatchTable::from_program(program, source_map),
            binding_snapshots: BindingSnapshotTable::from_program(program, source_map),
            collection_diffs: CollectionDiffPlan::from_program(program),
            visual: VisualOperatorTable::from_program(program, source_map),
            text_layout_requests: TextLayoutRequestTable::from_program(program, source_map),
            accessibility: AccessibilityTable::from_program(program, source_map),
            inspection: InspectionTable::from_program(program, source_map),
        }
    }
}
