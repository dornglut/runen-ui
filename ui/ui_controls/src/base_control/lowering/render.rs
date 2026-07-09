//! File: domain/ui/ui_controls/src/base_control/lowering/render.rs
//! Crate: ui_controls

use ui_render_data::{UiExpectedPrimitiveCount, UiPrimitiveFamily};

use crate::{
    ControlKindId, ControlRenderDescriptor, ControlRenderEvidenceId, RUNENWERK_CONTROL_PACKAGE_ID,
};

use super::super::{ControlDef, ControlPreset};

pub(crate) fn lower_render(def: &ControlDef, kind_id: ControlKindId) -> ControlRenderDescriptor {
    let descriptor = ControlRenderDescriptor::new(kind_id).with_render_evidence(
        ControlRenderEvidenceId::new(format!(
            "{RUNENWERK_CONTROL_PACKAGE_ID}.{}.evidence.render.contract",
            def.kind_suffix()
        )),
    );

    match def.preset() {
        ControlPreset::Label => add_render_families(
            descriptor,
            &[UiPrimitiveFamily::GlyphRun],
            &[UiExpectedPrimitiveCount::at_least(
                UiPrimitiveFamily::GlyphRun,
                1,
            )],
        ),
        ControlPreset::Button | ControlPreset::InspectorField | ControlPreset::ActionPrompt => {
            add_render_families(
                descriptor,
                &[
                    UiPrimitiveFamily::Rect,
                    UiPrimitiveFamily::Border,
                    UiPrimitiveFamily::GlyphRun,
                ],
                &[
                    UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Rect, 1),
                    UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::GlyphRun, 1),
                ],
            )
        }
        ControlPreset::ColorPicker => add_render_families(
            descriptor,
            &[
                UiPrimitiveFamily::Rect,
                UiPrimitiveFamily::Border,
                UiPrimitiveFamily::Stroke,
            ],
            &[
                UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Rect, 3),
                UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Stroke, 1),
            ],
        ),
        ControlPreset::ListView | ControlPreset::TreeView | ControlPreset::TableView => {
            add_render_families(
                descriptor,
                &[
                    UiPrimitiveFamily::Rect,
                    UiPrimitiveFamily::Clip,
                    UiPrimitiveFamily::GlyphRun,
                ],
                &[
                    UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Rect, 1),
                    UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::GlyphRun, 1),
                ],
            )
        }
        ControlPreset::Surface2D => add_render_families(
            descriptor,
            &[
                UiPrimitiveFamily::Rect,
                UiPrimitiveFamily::Border,
                UiPrimitiveFamily::Stroke,
                UiPrimitiveFamily::Clip,
            ],
            &[
                UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Rect, 1),
                UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Border, 1),
                UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Stroke, 1),
            ],
        ),
    }
}

fn add_render_families(
    mut descriptor: ControlRenderDescriptor,
    families: &[UiPrimitiveFamily],
    counts: &[UiExpectedPrimitiveCount],
) -> ControlRenderDescriptor {
    for family in families {
        descriptor = descriptor.with_required_primitive_family(*family);
    }
    for count in counts {
        descriptor = descriptor.with_expected_primitive_count(count.clone());
    }
    descriptor
}
