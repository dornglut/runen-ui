//! File: domain/ui/ui_controls/src/base_control/lowering/theme.rs
//! Crate: ui_controls

use crate::{
    ControlKindId, ControlStyleRequirement, ControlThemeDescriptor, ControlThemeTokenRequirement,
    ControlVisualStateRequirement,
};

use super::super::ControlDef;

pub(crate) fn lower_theme(def: &ControlDef, kind_id: ControlKindId) -> ControlThemeDescriptor {
    let mut descriptor = ControlThemeDescriptor::new(kind_id);

    for group in def.theme_groups() {
        for token in &group.tokens {
            let requirement = ControlThemeTokenRequirement::new(
                theme_token_id(&group.group_id, &token.token_name),
                token.kind,
                token.role,
            );
            descriptor = descriptor.with_token(if token.required {
                requirement
            } else {
                requirement.optional()
            });
        }

        for style in &group.styles {
            let requirement = ControlStyleRequirement::new(
                style.role,
                theme_token_id(&group.group_id, &style.token_name),
            );
            descriptor = descriptor.with_style(if style.required {
                requirement
            } else {
                requirement.optional()
            });
        }

        for state in &group.visual_states {
            let requirement = ControlVisualStateRequirement::new(state.state);
            descriptor = descriptor.with_visual_state(if state.required {
                requirement
            } else {
                requirement.optional()
            });
        }
    }

    descriptor
}

fn theme_token_id(group_id: &str, token: &str) -> String {
    format!("runenwerk.theme.controls.{group_id}.{token}")
}
