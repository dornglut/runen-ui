use serde::{Deserialize, Serialize};

use crate::diagnostic::{
    UI_STORY_MANIFEST_FIELD_MISSING, UiStoryDiagnostic, UiStoryDiagnosticOrigin,
    UiStoryDiagnosticSubject,
};
use crate::identity::{UiStoryId, UiStoryViewportProfileId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiStoryViewportProfileV2 {
    pub viewport_id: UiStoryViewportProfileId,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

impl UiStoryViewportProfileV2 {
    pub fn new(viewport_id: impl Into<String>, width: u32, height: u32, scale: f32) -> Self {
        Self {
            viewport_id: UiStoryViewportProfileId::new(viewport_id),
            width,
            height,
            scale,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.viewport_id.is_valid()
            && self.width > 0
            && self.height > 0
            && self.scale.is_finite()
            && self.scale > 0.0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UiStoryViewportMatrix {
    profiles: Vec<UiStoryViewportProfileV2>,
}

impl UiStoryViewportMatrix {
    pub fn new(profiles: impl IntoIterator<Item = UiStoryViewportProfileV2>) -> Self {
        let mut profiles = profiles.into_iter().collect::<Vec<_>>();
        profiles.sort_by(|left, right| left.viewport_id.cmp(&right.viewport_id));
        Self { profiles }
    }

    pub fn single(viewport_id: impl Into<String>, width: u32, height: u32, scale: f32) -> Self {
        Self::new([UiStoryViewportProfileV2::new(
            viewport_id,
            width,
            height,
            scale,
        )])
    }

    pub fn profiles(&self) -> &[UiStoryViewportProfileV2] {
        &self.profiles
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    pub fn validate(&self, story_id: &UiStoryId) -> Vec<UiStoryDiagnostic> {
        let mut diagnostics = Vec::new();
        if self.profiles.is_empty() {
            diagnostics.push(UiStoryDiagnostic::error(
                UI_STORY_MANIFEST_FIELD_MISSING,
                UiStoryDiagnosticOrigin::Manifest,
                UiStoryDiagnosticSubject::Story(story_id.clone()),
                "viewport_matrix must contain at least one viewport",
            ));
            return diagnostics;
        }

        for viewport in &self.profiles {
            if !viewport.is_valid() {
                diagnostics.push(
                    UiStoryDiagnostic::error(
                        UI_STORY_MANIFEST_FIELD_MISSING,
                        UiStoryDiagnosticOrigin::Manifest,
                        UiStoryDiagnosticSubject::Story(story_id.clone()),
                        "viewport must have a valid id, positive width, positive height, and positive finite scale",
                    )
                    .with_context("viewport_id", viewport.viewport_id.as_str())
                    .with_context("width", viewport.width.to_string())
                    .with_context("height", viewport.height.to_string())
                    .with_context("scale", viewport.scale.to_string()),
                );
            }
        }

        diagnostics
    }
}
