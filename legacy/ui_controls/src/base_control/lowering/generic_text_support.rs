use super::super::{ControlDef, ControlPreset};
use crate::{
    ControlGenericTextDescriptor, ControlGenericTextRoleDescriptor, ControlGenericTextSemanticRole,
    ControlKindId,
};

pub(crate) fn lower_generic_text_support(
    def: &ControlDef,
    kind_id: ControlKindId,
) -> Option<ControlGenericTextDescriptor> {
    let descriptor = match def.preset() {
        ControlPreset::Label => ControlGenericTextDescriptor::new(kind_id).with_role(
            ControlGenericTextRoleDescriptor::new(
                "label.primary",
                ControlGenericTextSemanticRole::Label,
            )
            .with_inline_spans(),
        ),
        ControlPreset::Button => ControlGenericTextDescriptor::new(kind_id).with_role(
            ControlGenericTextRoleDescriptor::new(
                "label.primary",
                ControlGenericTextSemanticRole::Label,
            ),
        ),
        ControlPreset::InspectorField => ControlGenericTextDescriptor::new(kind_id)
            .with_role(ControlGenericTextRoleDescriptor::new(
                "label.inspector-field",
                ControlGenericTextSemanticRole::InspectorLabel,
            ))
            .with_role(
                ControlGenericTextRoleDescriptor::new(
                    "value.inspector-field",
                    ControlGenericTextSemanticRole::InspectorValue,
                )
                .with_inline_spans(),
            ),
        ControlPreset::ActionPrompt => ControlGenericTextDescriptor::new(kind_id)
            .with_role(
                ControlGenericTextRoleDescriptor::new(
                    "body.prompt",
                    ControlGenericTextSemanticRole::Body,
                )
                .with_inline_spans(),
            )
            .with_role(
                ControlGenericTextRoleDescriptor::new(
                    "helper.prompt",
                    ControlGenericTextSemanticRole::Helper,
                )
                .with_max_lines(2),
            ),
        ControlPreset::ListView | ControlPreset::TreeView | ControlPreset::TableView => {
            ControlGenericTextDescriptor::new(kind_id)
                .with_role(ControlGenericTextRoleDescriptor::new(
                    "row.label",
                    ControlGenericTextSemanticRole::Label,
                ))
                .with_role(
                    ControlGenericTextRoleDescriptor::new(
                        "row.value",
                        ControlGenericTextSemanticRole::Body,
                    )
                    .with_inline_spans(),
                )
        }
        ControlPreset::ColorPicker | ControlPreset::Surface2D => return None,
    };
    Some(descriptor)
}
