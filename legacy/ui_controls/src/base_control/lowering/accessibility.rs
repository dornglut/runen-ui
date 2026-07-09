//! File: domain/ui/ui_controls/src/base_control/lowering/accessibility.rs
//! Crate: ui_controls

use crate::{
    ControlAccessibilityDescriptionRequirement, ControlAccessibilityDescriptor,
    ControlAccessibilityLabelRequirement, ControlAccessibilityRole, ControlFocusRequirement,
    ControlKeyboardActivation, ControlKindId, ControlSemanticHint, ControlSemanticState,
    ControlValueRangeMetadata,
};

use super::super::{ControlDef, ControlPreset};

pub(crate) fn lower_accessibility(
    def: &ControlDef,
    kind_id: ControlKindId,
) -> ControlAccessibilityDescriptor {
    let descriptor = ControlAccessibilityDescriptor::new(kind_id)
        .with_label(ControlAccessibilityLabelRequirement::new(format!(
            "{}.accessible-label",
            def.kind_suffix()
        )))
        .with_description(
            ControlAccessibilityDescriptionRequirement::new(format!(
                "{}.accessible-description",
                def.kind_suffix()
            ))
            .optional(),
        )
        .with_hint(ControlSemanticHint::new(format!(
            "{}.package-summary",
            def.kind_suffix()
        )))
        .with_semantic_state(ControlSemanticState::Enabled)
        .with_semantic_state(ControlSemanticState::Disabled);

    match def.preset() {
        ControlPreset::Label => add_roles(
            descriptor,
            &[
                ControlAccessibilityRole::Label,
                ControlAccessibilityRole::Text,
            ],
        ),
        ControlPreset::Button => add_keyboard(
            focusable(add_roles(descriptor, &[ControlAccessibilityRole::Button]))
                .with_semantic_state(ControlSemanticState::Pressed),
            &[ControlKeyboardActivation::Activate],
        ),
        ControlPreset::InspectorField => add_keyboard(
            focusable(add_roles(
                descriptor
                    .with_semantic_state(ControlSemanticState::Readonly)
                    .with_semantic_state(ControlSemanticState::Invalid),
                &[
                    ControlAccessibilityRole::Panel,
                    ControlAccessibilityRole::Text,
                ],
            )),
            &[
                ControlKeyboardActivation::Commit,
                ControlKeyboardActivation::Cancel,
            ],
        ),
        ControlPreset::ColorPicker => add_keyboard(
            focusable(add_roles(descriptor, &[ControlAccessibilityRole::Custom])).with_value_range(
                ControlValueRangeMetadata::new("color-picker.channel-value")
                    .with_minimum()
                    .with_maximum()
                    .with_step(),
            ),
            &[
                ControlKeyboardActivation::Commit,
                ControlKeyboardActivation::Cancel,
            ],
        ),
        ControlPreset::ActionPrompt => add_keyboard(
            add_roles(
                descriptor,
                &[
                    ControlAccessibilityRole::Dialog,
                    ControlAccessibilityRole::Panel,
                ],
            )
            .with_focus(ControlFocusRequirement::focusable().with_focus_return()),
            &[
                ControlKeyboardActivation::Activate,
                ControlKeyboardActivation::Cancel,
            ],
        ),
        ControlPreset::ListView => collection_accessibility(
            descriptor,
            &[
                ControlAccessibilityRole::List,
                ControlAccessibilityRole::ListItem,
            ],
        ),
        ControlPreset::TreeView => add_keyboard(
            collection_accessibility(
                descriptor
                    .with_semantic_state(ControlSemanticState::Expanded)
                    .with_semantic_state(ControlSemanticState::Collapsed),
                &[
                    ControlAccessibilityRole::Tree,
                    ControlAccessibilityRole::TreeItem,
                ],
            ),
            &[
                ControlKeyboardActivation::Expand,
                ControlKeyboardActivation::Collapse,
            ],
        ),
        ControlPreset::TableView => collection_accessibility(
            descriptor,
            &[
                ControlAccessibilityRole::Table,
                ControlAccessibilityRole::Row,
                ControlAccessibilityRole::Cell,
            ],
        ),
        ControlPreset::Surface2D => add_keyboard(
            focusable(add_roles(
                descriptor,
                &[
                    ControlAccessibilityRole::Canvas,
                    ControlAccessibilityRole::Panel,
                ],
            )),
            &[
                ControlKeyboardActivation::NavigateNext,
                ControlKeyboardActivation::NavigatePrevious,
                ControlKeyboardActivation::Increment,
                ControlKeyboardActivation::Decrement,
                ControlKeyboardActivation::Activate,
                ControlKeyboardActivation::Cancel,
            ],
        ),
    }
}

fn collection_accessibility(
    descriptor: ControlAccessibilityDescriptor,
    roles: &[ControlAccessibilityRole],
) -> ControlAccessibilityDescriptor {
    add_keyboard(
        focusable(add_roles(
            descriptor.with_semantic_state(ControlSemanticState::Selected),
            roles,
        )),
        &[
            ControlKeyboardActivation::NavigateNext,
            ControlKeyboardActivation::NavigatePrevious,
        ],
    )
}

fn add_roles(
    mut descriptor: ControlAccessibilityDescriptor,
    values: &[ControlAccessibilityRole],
) -> ControlAccessibilityDescriptor {
    for value in values {
        descriptor = descriptor.with_role(*value);
    }
    descriptor
}

fn add_keyboard(
    mut descriptor: ControlAccessibilityDescriptor,
    values: &[ControlKeyboardActivation],
) -> ControlAccessibilityDescriptor {
    for value in values {
        descriptor = descriptor.with_keyboard_activation(*value);
    }
    descriptor
}

fn focusable(descriptor: ControlAccessibilityDescriptor) -> ControlAccessibilityDescriptor {
    descriptor.with_focus(ControlFocusRequirement::focusable())
}
