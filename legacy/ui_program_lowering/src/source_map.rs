//! File: domain/ui/ui_program_lowering/src/source_map.rs
//! Crate: ui_program_lowering
//!
//! Source-map helpers for UiProgram formation.

use ui_definition::AuthoredUiNodePath;
use ui_program::{
    UiProgramSourceId, UiProgramSourceMapAttachment, UiProgramSourceMapEntry, UiProgramTargetId,
};

pub(crate) fn source_map_for_path(
    path: &AuthoredUiNodePath,
    target_id: impl Into<String>,
) -> UiProgramSourceMapAttachment {
    UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
        UiProgramSourceId::new(format!("definition.{}", path.as_str().replace('/', "."))),
        UiProgramTargetId::new(target_id),
    ))
}
