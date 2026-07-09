//! File: domain/ui/ui_geometry/src/lib.rs
//! Crate: ui_geometry

use std::fmt;

use serde::{Deserialize, Serialize};
use ui_artifacts::UiRuntimeArtifact;
use ui_program::{ControlNodeId, LayoutConstraintId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometrySpace {
    Logical,
    Screen,
    WorldSpace,
    Headless,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl GeometryRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::try_new(x, y, width, height).expect("geometry rectangles must be finite and positive")
    }

    pub fn try_new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, GeometryContractError> {
        if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
            return Err(GeometryContractError::NonFiniteRect);
        }
        if width < 0.0 || height < 0.0 {
            return Err(GeometryContractError::NegativeExtent);
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn contains(&self, other: &Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }

    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);
        if x2 <= x1 || y2 <= y1 {
            return None;
        }
        Some(Self::new(x1, y1, x2 - x1, y2 - y1))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryViewport {
    pub space: GeometrySpace,
    pub bounds: GeometryRect,
}

impl GeometryViewport {
    pub fn headless(width: f32, height: f32) -> Self {
        Self {
            space: GeometrySpace::Headless,
            bounds: GeometryRect::new(0.0, 0.0, width, height),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryFrame {
    pub slot_id: LayoutConstraintId,
    pub control_id: ControlNodeId,
    pub space: GeometrySpace,
    pub bounds: GeometryRect,
    #[serde(default)]
    pub clip: Option<GeometryRect>,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl GeometryFrame {
    pub fn is_source_mapped(&self) -> bool {
        self.source_map_index.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeometryPlan {
    pub frames: Vec<GeometryFrame>,
    pub diagnostics: Vec<GeometryDiagnostic>,
}

impl GeometryPlan {
    pub fn from_artifact(artifact: &UiRuntimeArtifact, viewport: GeometryViewport) -> Self {
        if artifact.tables.layout.rows.is_empty() {
            return Self {
                frames: Vec::new(),
                diagnostics: vec![GeometryDiagnostic {
                    code: "ui.geometry.layout.empty".to_owned(),
                    message: "runtime artifact has no layout rows to project".to_owned(),
                }],
            };
        }

        let row_height = viewport.bounds.height / artifact.tables.layout.rows.len() as f32;
        let frames = artifact
            .tables
            .layout
            .rows
            .iter()
            .enumerate()
            .map(|(row, layout)| GeometryFrame {
                slot_id: layout.constraint.constraint_id.clone(),
                control_id: layout.constraint.target_control.clone(),
                space: viewport.space,
                bounds: GeometryRect::new(
                    viewport.bounds.x,
                    viewport.bounds.y + row as f32 * row_height,
                    viewport.bounds.width,
                    row_height,
                ),
                clip: Some(viewport.bounds),
                source_map_index: layout.source_map_index,
            })
            .collect::<Vec<_>>();

        let diagnostics = frames
            .iter()
            .filter(|frame| !frame.is_source_mapped())
            .map(|frame| GeometryDiagnostic {
                code: "ui.geometry.source_map_missing".to_owned(),
                message: format!(
                    "geometry frame {} has no source-map row",
                    frame.slot_id.as_str()
                ),
            })
            .collect();

        Self {
            frames,
            diagnostics,
        }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn frame_for_control(&self, control_id: &ControlNodeId) -> Option<&GeometryFrame> {
        self.frames
            .iter()
            .find(|frame| &frame.control_id == control_id)
    }

    pub fn source_mapped_count(&self) -> usize {
        self.frames
            .iter()
            .filter(|frame| frame.is_source_mapped())
            .count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeometryDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryContractError {
    NonFiniteRect,
    NegativeExtent,
}

impl fmt::Display for GeometryContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteRect => {
                write!(formatter, "geometry rectangle contains non-finite values")
            }
            Self::NegativeExtent => {
                write!(formatter, "geometry rectangle extents must be non-negative")
            }
        }
    }
}

impl std::error::Error for GeometryContractError {}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_artifacts::UiRuntimeArtifact;
    use ui_program::{
        ControlGraphNode, ControlKernelRef, ControlKindRef, ControlNodeId, ControlPackageRef,
        LayoutConstraintId, LayoutGraphNode, UiProgram, UiProgramId, UiProgramSourceId,
        UiProgramSourceMapAttachment, UiProgramSourceMapEntry, UiProgramTargetId, UiProgramVersion,
    };

    #[test]
    fn geometry_contract_projects_layout_rows_into_headless_frames() {
        let artifact = UiRuntimeArtifact::from_program(&geometry_program());
        let plan = GeometryPlan::from_artifact(&artifact, GeometryViewport::headless(320.0, 200.0));

        assert!(plan.passed());
        assert_eq!(plan.frames.len(), 2);
        assert_eq!(plan.source_mapped_count(), 2);
        assert_eq!(
            plan.frame_for_control(&ControlNodeId::new("control.fixture.title"))
                .map(|frame| frame.bounds),
            Some(GeometryRect::new(0.0, 0.0, 320.0, 100.0))
        );
        assert_eq!(
            plan.frame_for_control(&ControlNodeId::new("control.fixture.value"))
                .map(|frame| frame.bounds),
            Some(GeometryRect::new(0.0, 100.0, 320.0, 100.0))
        );
        assert!(
            plan.frames[0]
                .clip
                .unwrap()
                .contains(&plan.frames[0].bounds)
        );
    }

    fn geometry_program() -> UiProgram {
        let mut program = UiProgram::new(
            UiProgramId::new("fixture.geometry"),
            UiProgramVersion::new(1),
        );
        for (suffix, row) in [("title", 0), ("value", 1)] {
            let control_id = ControlNodeId::new(format!("control.fixture.{suffix}"));
            let source_map = UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
                UiProgramSourceId::new(format!("definition.fixture.{suffix}")),
                UiProgramTargetId::new(format!("program.fixture.layout.{suffix}")),
            ));
            program.graphs.control.add_node(ControlGraphNode::new(
                control_id.clone(),
                ControlPackageRef::new("runenwerk.ui.controls"),
                ControlKindRef::new("runenwerk.ui.controls.label"),
            ));
            program.graphs.layout.constraints.push(
                LayoutGraphNode::new(
                    LayoutConstraintId::new(format!("layout.fixture.{row}")),
                    control_id,
                )
                .with_layout_kernel(ControlKernelRef::new("runenwerk.ui.controls.label.layout"))
                .with_source_map(source_map),
            );
        }
        program
    }
}
