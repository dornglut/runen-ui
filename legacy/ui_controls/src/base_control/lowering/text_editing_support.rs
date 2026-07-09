use super::super::{ControlDef, ControlPreset};
use crate::{ControlEditableTextDescriptor, ControlKindId};

pub(crate) fn lower_text_editing_support(
    def: &ControlDef,
    kind_id: ControlKindId,
) -> Option<ControlEditableTextDescriptor> {
    match def.preset() {
        ControlPreset::InspectorField => Some(
            ControlEditableTextDescriptor::inspector_field_input(kind_id),
        ),
        _ => None,
    }
}
