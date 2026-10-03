use core::fmt::Write as _;

use runenui_core::{
    __runtime::RuntimeNamespace, CommandOrigin, LogicalDelta, MountedNodeId, SemanticAction,
    SemanticActionTarget, SemanticCommand, SemanticKey, SemanticNodeId, SurfaceId,
    WidgetInvalidation,
};

use crate::TraceTarget;

use super::{json, tokens};

pub(super) fn mounted_id(output: &mut String, runtime: &RuntimeNamespace, id: &MountedNodeId) {
    if let Some((slot, generation)) = runtime.__runtime_mounted_parts(id) {
        output.push_str("{\"scope\":\"local\",\"token\":");
        let mut token = String::new();
        write!(&mut token, "m-{slot:08x}-{generation:016x}")
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        json::string(output, &token);
        output.push('}');
    } else {
        output.push_str("{\"scope\":\"foreign\"}");
    }
}

pub(super) fn optional_mounted_id(
    output: &mut String,
    runtime: &RuntimeNamespace,
    id: Option<&MountedNodeId>,
) {
    if let Some(id) = id {
        mounted_id(output, runtime, id);
    } else {
        output.push_str("null");
    }
}

pub(super) fn semantic_id(output: &mut String, runtime: &RuntimeNamespace, id: &SemanticNodeId) {
    if let Some((slot, generation)) = runtime.__runtime_semantic_parts(id) {
        output.push_str("{\"scope\":\"local\",\"token\":");
        let mut token = String::new();
        write!(&mut token, "n-{slot:08x}-{generation:016x}")
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        json::string(output, &token);
        output.push('}');
    } else {
        output.push_str("{\"scope\":\"foreign\"}");
    }
}

pub(super) fn surface_id(output: &mut String, runtime: &RuntimeNamespace, id: &SurfaceId) {
    if let Some((slot, generation)) = runtime.__runtime_surface_parts(id) {
        output.push_str("{\"scope\":\"local\",\"token\":");
        let mut token = String::new();
        write!(&mut token, "s-{slot:08x}-{generation:016x}")
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        json::string(output, &token);
        output.push('}');
    } else {
        output.push_str("{\"scope\":\"foreign\"}");
    }
}

pub(super) fn semantic_action_target(
    output: &mut String,
    runtime: &RuntimeNamespace,
    target: &SemanticActionTarget,
) {
    output.push('{');
    json::name(output, "surface");
    surface_id(output, runtime, target.surface_id());
    output.push(',');
    json::name(output, "node");
    semantic_id(output, runtime, target.target());
    output.push(',');
    json::name(output, "key");
    semantic_key(output, target.semantic_key());
    output.push(',');
    json::name(output, "action");
    semantic_action(output, target.action());
    output.push('}');
}

fn semantic_key(output: &mut String, key: &SemanticKey) {
    output.push('{');
    json::name(output, "kind");
    if key.is_primary() {
        json::string(output, "primary");
    } else {
        json::string(output, "named");
        output.push(',');
        json::name(output, "value");
        json::string(
            output,
            key.as_str()
                .unwrap_or_else(|| unreachable!("named semantic key carries text")),
        );
    }
    output.push('}');
}

fn semantic_action(output: &mut String, action: &SemanticAction) {
    json::string(
        output,
        match action {
            SemanticAction::Activate => "activate",
            SemanticAction::RequestFocus => "request_focus",
            SemanticAction::OpenMenu => "open_menu",
            SemanticAction::OpenContextMenu => "open_context_menu",
            SemanticAction::Increment => "increment",
            SemanticAction::Decrement => "decrement",
            SemanticAction::SetValue => "set_value",
            SemanticAction::Expand => "expand",
            SemanticAction::Collapse => "collapse",
            _ => "unknown",
        },
    );
}

pub(super) fn target(output: &mut String, runtime: &RuntimeNamespace, target: &TraceTarget) {
    output.push('{');
    json::name(output, "mounted");
    mounted_id(output, runtime, target.mounted_node_id());
    output.push(',');
    json::name(output, "authored_id");
    json::optional_string(
        output,
        target.authored_id().map(runenui_core::ElementId::as_str),
    );
    output.push('}');
}

pub(super) fn optional_target(
    output: &mut String,
    runtime: &RuntimeNamespace,
    target_value: Option<&TraceTarget>,
) {
    if let Some(target_value) = target_value {
        target(output, runtime, target_value);
    } else {
        output.push_str("null");
    }
}

pub(super) fn command_origin(output: &mut String, origin: CommandOrigin) {
    output.push('{');
    json::name(output, "source");
    json::string(output, tokens::event_source(origin.source()));
    output.push(',');
    json::name(output, "derivation");
    json::string(output, tokens::command_derivation(origin.derivation()));
    output.push('}');
}

pub(super) fn semantic_command(output: &mut String, command: SemanticCommand) {
    output.push('{');
    json::name(output, "kind");
    match command {
        SemanticCommand::Activate => json::string(output, "activate"),
        SemanticCommand::CancelOrBack => json::string(output, "cancel_or_back"),
        SemanticCommand::OpenMenu => json::string(output, "open_menu"),
        SemanticCommand::OpenContextMenu => json::string(output, "open_context_menu"),
        SemanticCommand::LogicalScroll(scroll) => {
            json::string(output, "logical_scroll");
            output.push(',');
            json::name(output, "pointer_id");
            json::u64_value(output, scroll.pointer_id().get());
            output.push(',');
            json::name(output, "delta");
            logical_delta(output, scroll.delta());
        }
        SemanticCommand::FocusNext => json::string(output, "focus_next"),
        SemanticCommand::FocusPrevious => json::string(output, "focus_previous"),
        SemanticCommand::FocusGroupNext => json::string(output, "focus_group_next"),
        SemanticCommand::FocusGroupPrevious => json::string(output, "focus_group_previous"),
        SemanticCommand::FocusGroupFirst => json::string(output, "focus_group_first"),
        SemanticCommand::FocusGroupLast => json::string(output, "focus_group_last"),
        SemanticCommand::FocusLeft => json::string(output, "focus_left"),
        SemanticCommand::FocusRight => json::string(output, "focus_right"),
        SemanticCommand::FocusUp => json::string(output, "focus_up"),
        SemanticCommand::FocusDown => json::string(output, "focus_down"),
        SemanticCommand::RequestFocus => json::string(output, "request_focus"),
        SemanticCommand::RestoreFocus => json::string(output, "restore_focus"),
        SemanticCommand::LogicalFocusScroll(direction) => {
            json::string(output, "logical_focus_scroll");
            output.push(',');
            json::name(output, "direction");
            json::string(output, tokens::focus_direction(direction));
        }
        SemanticCommand::ScrollIntoView => json::string(output, "scroll_into_view"),
        SemanticCommand::ScrollControl(request) => {
            json::string(output, "scroll_control");
            output.push(',');
            json::name(output, "operation");
            let (operation, normalized) = match request {
                runenui_core::ScrollControlRequest::SmallStepBackward => {
                    ("small_step_backward", None)
                }
                runenui_core::ScrollControlRequest::SmallStepForward => {
                    ("small_step_forward", None)
                }
                runenui_core::ScrollControlRequest::PageBackward => ("page_backward", None),
                runenui_core::ScrollControlRequest::PageForward => ("page_forward", None),
                runenui_core::ScrollControlRequest::ToStart => ("to_start", None),
                runenui_core::ScrollControlRequest::ToEnd => ("to_end", None),
                runenui_core::ScrollControlRequest::SetNormalized(value) => {
                    ("set_normalized", Some(value.get()))
                }
                _ => ("unknown", None),
            };
            json::string(output, operation);
            output.push(',');
            json::name(output, "normalized");
            if let Some(normalized) = normalized {
                json::f32_value(output, normalized);
            } else {
                output.push_str("null");
            }
        }
        SemanticCommand::Increment => json::string(output, "increment"),
        SemanticCommand::Decrement => json::string(output, "decrement"),
        SemanticCommand::SetValue(value) => {
            json::string(output, "set_value");
            output.push(',');
            json::name(output, "value");
            json::f64_value(output, value.get());
        }
        SemanticCommand::Expand => json::string(output, "expand"),
        SemanticCommand::Collapse => json::string(output, "collapse"),
        SemanticCommand::MoveUp => json::string(output, "move_up"),
        SemanticCommand::MoveDown => json::string(output, "move_down"),
        SemanticCommand::ExtendUp => json::string(output, "extend_up"),
        SemanticCommand::ExtendDown => json::string(output, "extend_down"),
        _ => json::string(output, "unknown"),
    }
    output.push('}');
}

pub(super) fn logical_delta(output: &mut String, delta: LogicalDelta) {
    output.push('{');
    json::name(output, "x");
    json::f32_value(output, delta.x());
    output.push(',');
    json::name(output, "y");
    json::f32_value(output, delta.y());
    output.push('}');
}

pub(super) fn invalidation(output: &mut String, invalidation: WidgetInvalidation) {
    output.push('[');
    let mut first = true;
    for (flag, name) in [
        (WidgetInvalidation::INTERACTION, "interaction"),
        (WidgetInvalidation::LAYOUT, "layout"),
        (WidgetInvalidation::PAINT, "paint"),
        (WidgetInvalidation::SEMANTICS, "semantics"),
        (WidgetInvalidation::DIAGNOSTICS, "diagnostics"),
    ] {
        if invalidation.contains(flag) {
            if !first {
                output.push(',');
            }
            first = false;
            json::string(output, name);
        }
    }
    output.push(']');
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        ScrollControlRequest, ScrollNormalizedValue, SemanticAction, SemanticCommand,
        SemanticNumber,
    };

    use super::{semantic_action, semantic_command};

    #[test]
    fn scroll_control_command_trace_token_is_structured_and_keeps_normalized_payload() {
        let mut encoded = String::new();
        semantic_command(
            &mut encoded,
            SemanticCommand::ScrollControl(ScrollControlRequest::SetNormalized(
                ScrollNormalizedValue::new(0.5)
                    .unwrap_or_else(|_| unreachable!("fixture value is normalized")),
            )),
        );
        assert_eq!(
            encoded,
            r#"{"kind":"scroll_control","operation":"set_normalized","normalized":0.5}"#
        );
    }

    #[test]
    fn range_and_expansion_trace_tokens_are_stable_and_set_value_keeps_numeric_value() {
        let mut command = String::new();
        semantic_command(
            &mut command,
            SemanticCommand::SetValue(
                SemanticNumber::new(7.5)
                    .unwrap_or_else(|_| unreachable!("controlled value is finite")),
            ),
        );
        assert_eq!(command, r#"{"kind":"set_value","value":7.5}"#);

        for (action, expected) in [
            (SemanticAction::Increment, r#""increment""#),
            (SemanticAction::Decrement, r#""decrement""#),
            (SemanticAction::SetValue, r#""set_value""#),
            (SemanticAction::Expand, r#""expand""#),
            (SemanticAction::Collapse, r#""collapse""#),
        ] {
            let mut encoded = String::new();
            semantic_action(&mut encoded, &action);
            assert_eq!(encoded, expected);
        }
    }
}
