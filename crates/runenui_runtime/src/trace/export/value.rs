use core::fmt::Write as _;

use runenui_core::{
    __runtime::RuntimeNamespace, CommandOrigin, LogicalDelta, MountedNodeId, SemanticCommand,
    SurfaceId, WidgetInvalidation,
};

use crate::TraceTarget;

use super::{json, tokens};

pub(super) fn mounted_id(
    output: &mut String,
    runtime: &RuntimeNamespace,
    id: &MountedNodeId,
) {
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

pub(super) fn target(output: &mut String, runtime: &RuntimeNamespace, target: &TraceTarget) {
    output.push('{');
    json::name(output, "mounted");
    mounted_id(output, runtime, target.mounted_node_id());
    output.push(',');
    json::name(output, "authored_id");
    json::optional_string(output, target.authored_id().map(runenui_core::ElementId::as_str));
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
        _ => json::string(output, "unknown"),
    }
    output.push('}');
}

fn logical_delta(output: &mut String, delta: LogicalDelta) {
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
