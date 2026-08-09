mod json;
mod kind;
mod tokens;
mod value;

use runenui_core::__runtime::RuntimeNamespace;

use crate::{
    TraceAutomationContext, TraceContext, TraceInputContext, TracePointerCleanup, TraceRecord,
    TraceRouteSnapshot, TraceTargetTransition, TraceWorkIdentity, TraceWorkOwner,
};

pub(crate) fn encode_trace_jsonl<'a>(
    runtime: &RuntimeNamespace,
    dropped_before_sequence: Option<crate::TraceSequence>,
    records: impl ExactSizeIterator<Item = &'a TraceRecord>,
) -> String {
    let retained_records = records.len();
    let mut output = String::new();
    output.push_str("{\"schema\":\"runenui.trace\",\"version\":1,\"dropped_before_sequence\":");
    json::optional_u64(
        &mut output,
        dropped_before_sequence.map(crate::TraceSequence::get),
    );
    output.push_str(",\"retained_records\":");
    json::usize_value(&mut output, retained_records);
    output.push_str("}\n");
    for record in records {
        output.push_str(&encode_record_json(runtime, record));
        output.push('\n');
    }
    output
}

pub(crate) fn encode_record_json(runtime: &RuntimeNamespace, record: &TraceRecord) -> String {
    let mut output = String::new();
    output.push_str("{\"schema\":\"runenui.trace.record\",\"version\":1,\"sequence\":");
    json::u64_value(&mut output, record.sequence().get());
    output.push_str(",\"kind\":");
    kind::encode(&mut output, record.kind());
    output.push_str(",\"work_sequence\":");
    json::optional_u64(
        &mut output,
        record.work_sequence().map(runenui_core::WorkSequence::get),
    );
    output.push_str(",\"causal_parent\":");
    json::optional_u64(
        &mut output,
        record.causal_parent().map(crate::TraceSequence::get),
    );
    output.push_str(",\"reconciliation_before\":");
    json::optional_u64(
        &mut output,
        record
            .reconciliation_before()
            .map(crate::ReconciliationGeneration::get),
    );
    output.push_str(",\"reconciliation_after\":");
    json::optional_u64(
        &mut output,
        record
            .reconciliation_after()
            .map(crate::ReconciliationGeneration::get),
    );
    output.push_str(",\"target\":");
    value::optional_target(&mut output, runtime, record.target());
    output.push_str(",\"work\":");
    encode_work(&mut output, runtime, record.work());
    output.push_str(",\"instant_nanos\":");
    json::optional_u64(
        &mut output,
        record
            .instant()
            .map(runenui_core::MonotonicInstant::as_nanos),
    );
    output.push_str(",\"original_target\":");
    value::optional_mounted_id(&mut output, runtime, record.original_target());
    output.push_str(",\"current_target\":");
    value::optional_mounted_id(&mut output, runtime, record.current_target());
    output.push_str(",\"command_origin\":");
    if let Some(origin) = record.command_origin() {
        value::command_origin(&mut output, origin);
    } else {
        output.push_str("null");
    }
    output.push_str(",\"context\":");
    encode_context(&mut output, runtime, record.context());
    output.push_str(",\"sink_delivery\":");
    json::optional_string(
        &mut output,
        record.sink_delivery().map(tokens::sink_delivery),
    );
    output.push('}');
    output
}

fn encode_work(output: &mut String, runtime: &RuntimeNamespace, work: Option<&TraceWorkIdentity>) {
    let Some(work) = work else {
        output.push_str("null");
        return;
    };
    output.push('{');
    json::name(output, "owner");
    match work.owner() {
        TraceWorkOwner::Application => json::string(output, "application"),
        TraceWorkOwner::Mounted(owner) => {
            output.push_str("{\"mounted\":");
            value::mounted_id(output, runtime, owner);
            output.push('}');
        }
    }
    output.push(',');
    json::name(output, "family");
    json::string(output, tokens::work_family(work.family()));
    output.push(',');
    json::name(output, "generation");
    json::u64_value(output, work.generation());
    output.push(',');
    json::name(output, "key");
    json::optional_string(output, work.key().map(runenui_core::WorkKey::as_str));
    output.push('}');
}

fn encode_context(output: &mut String, runtime: &RuntimeNamespace, context: &TraceContext) {
    output.push('{');
    encode_event_pointer_context(output, runtime, context);
    encode_input_action_context(output, runtime, context);
    encode_route_transition_context(output, runtime, context);
    encode_publication_context(output, runtime, context);
    output.push_str(",\"delivery\":");
    json::optional_string(output, context.delivery().map(tokens::delivery_outcome));
    output.push('}');
}

fn encode_event_pointer_context(
    output: &mut String,
    runtime: &RuntimeNamespace,
    context: &TraceContext,
) {
    json::name(output, "event");
    if let Some(event) = context.event() {
        output.push_str("{\"family\":");
        json::string(output, tokens::event_family(event.family()));
        output.push_str(",\"cancelable\":");
        json::bool_value(output, event.is_cancelable());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"surface\":");
    encode_surface(output, runtime, context.surface());
    output.push_str(",\"pointer\":");
    if let Some(pointer) = context.pointer() {
        output.push_str("{\"pointer_id\":");
        json::u64_value(output, pointer.pointer_id().get());
        output.push_str(",\"device_id\":");
        json::optional_u64(
            output,
            pointer.device_id().map(runenui_core::InputDeviceId::get),
        );
        output.push_str(",\"device_kind\":");
        json::string(output, tokens::pointer_device_kind(pointer.device_kind()));
        output.push_str(",\"phase\":");
        json::optional_string(output, pointer.phase().map(tokens::pointer_phase));
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"pointer_role\":");
    json::optional_string(
        output,
        context
            .pointer_record_role()
            .map(tokens::pointer_record_role),
    );
    output.push_str(",\"focus_role\":");
    json::optional_string(
        output,
        context.focus_record_role().map(tokens::focus_record_role),
    );
}

fn encode_input_action_context(
    output: &mut String,
    runtime: &RuntimeNamespace,
    context: &TraceContext,
) {
    output.push_str(",\"input\":");
    encode_input(output, context.input());
    output.push_str(",\"automation\":");
    encode_automation(output, runtime, context.automation());
    output.push_str(",\"action\":");
    if let Some(action) = context.action() {
        output.push_str("{\"type_name\":");
        json::string(output, action.type_name());
        output.push_str(",\"category\":");
        json::string(output, tokens::action_category(action.category()));
        output.push_str(",\"label\":");
        json::optional_string(output, action.label());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"requested_pointer_id\":");
    json::optional_u64(
        output,
        context
            .requested_pointer_id()
            .map(|pointer_id| pointer_id.get()),
    );
}

fn encode_route_transition_context(
    output: &mut String,
    runtime: &RuntimeNamespace,
    context: &TraceContext,
) {
    output.push_str(",\"route\":");
    encode_route(output, runtime, context.route());
    output.push_str(",\"physical_path\":");
    if let Some(path) = context.physical_path() {
        encode_target_list(output, runtime, path.targets());
    } else {
        output.push_str("null");
    }
    output.push_str(",\"target_transition\":");
    encode_transition(output, runtime, context.target_transition());
    output.push_str(",\"pointer_cleanup\":");
    encode_pointer_cleanup(output, runtime, context.pointer_cleanup());
    output.push_str(",\"modality_transition\":");
    if let Some(transition) = context.modality_transition() {
        output.push_str("{\"previous\":");
        json::optional_string(output, transition.previous().map(tokens::input_modality));
        output.push_str(",\"current\":");
        json::string(output, tokens::input_modality(transition.current()));
        output.push('}');
    } else {
        output.push_str("null");
    }
}

fn encode_publication_context(
    output: &mut String,
    runtime: &RuntimeNamespace,
    context: &TraceContext,
) {
    output.push_str(",\"publication\":");
    if let Some(publication) = context.publication() {
        output.push_str("{\"surface\":");
        encode_surface(output, runtime, Some(publication.surface()));
        output.push_str(",\"reconciliation_generation\":");
        json::u64_value(output, publication.reconciliation_generation().get());
        output.push_str(",\"node_count\":");
        json::usize_value(output, publication.node_count());
        output.push_str(",\"executed_phases\":[");
        for (index, phase) in publication.executed_phases().iter().copied().enumerate() {
            if index != 0 {
                output.push(',');
            }
            json::string(output, tokens::surface_phase(phase));
        }
        output.push_str("]}");
    } else {
        output.push_str("null");
    }
}

fn encode_surface(
    output: &mut String,
    runtime: &RuntimeNamespace,
    surface: Option<&crate::TraceSurfaceContext>,
) {
    let Some(surface) = surface else {
        output.push_str("null");
        return;
    };
    output.push_str("{\"surface\":");
    value::surface_id(output, runtime, surface.surface_id());
    output.push_str(",\"coordinate_revision\":");
    json::u64_value(output, surface.coordinate_revision());
    output.push_str(",\"hit_test_generation\":");
    json::u64_value(output, surface.hit_test_generation());
    output.push_str(",\"snapshot\":");
    json::optional_string(output, surface.snapshot().map(tokens::surface_snapshot));
    output.push('}');
}

fn encode_input(output: &mut String, input: Option<&TraceInputContext>) {
    let Some(input) = input else {
        output.push_str("null");
        return;
    };
    output.push_str("{\"role\":");
    json::string(output, tokens::input_record_role(input.role()));
    output.push_str(",\"device_id\":");
    json::optional_u64(
        output,
        input.device_id().map(runenui_core::InputDeviceId::get),
    );
    output.push_str(",\"composition\":");
    if let Some(composition) = input.composition() {
        output.push_str("{\"generation\":");
        json::u64_value(output, composition.generation().get());
        output.push_str(",\"device_id\":");
        json::optional_u64(
            output,
            composition
                .device_id()
                .map(runenui_core::InputDeviceId::get),
        );
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"text_metrics\":");
    if let Some(metrics) = input.text_metrics() {
        output.push_str("{\"bytes\":");
        json::usize_value(output, metrics.bytes());
        output.push_str(",\"scalars\":");
        json::usize_value(output, metrics.scalars());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push_str(",\"captured_text\":");
    json::optional_string(output, input.captured_text());
    output.push_str(",\"composition_range\":");
    if let Some(range) = input.composition_range() {
        output.push_str("{\"byte_start\":");
        json::usize_value(output, range.byte_start());
        output.push_str(",\"byte_end\":");
        json::usize_value(output, range.byte_end());
        output.push_str(",\"scalar_start\":");
        json::usize_value(output, range.scalar_start());
        output.push_str(",\"scalar_end\":");
        json::usize_value(output, range.scalar_end());
        output.push('}');
    } else {
        output.push_str("null");
    }
    output.push('}');
}

fn encode_automation(
    output: &mut String,
    runtime: &RuntimeNamespace,
    automation: Option<&TraceAutomationContext>,
) {
    let Some(automation) = automation else {
        output.push_str("null");
        return;
    };
    output.push_str("{\"role\":");
    json::string(output, tokens::automation_record_role(automation.role()));
    output.push_str(",\"authored_id\":");
    json::string(output, automation.authored_id().as_str());
    output.push_str(",\"command\":");
    value::semantic_command(output, automation.command());
    output.push_str(",\"candidates\":");
    if let Some(candidates) = automation.candidates() {
        output.push('[');
        for (index, candidate) in candidates.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            output.push_str("{\"logical_preorder\":");
            json::usize_value(output, candidate.logical_preorder());
            output.push_str(",\"mounted\":");
            value::mounted_id(output, runtime, candidate.mounted_node_id());
            output.push('}');
        }
        output.push(']');
    } else {
        output.push_str("null");
    }
    output.push('}');
}

fn encode_route(
    output: &mut String,
    runtime: &RuntimeNamespace,
    route: Option<&TraceRouteSnapshot>,
) {
    let Some(route) = route else {
        output.push_str("null");
        return;
    };
    output.push_str("{\"targets\":");
    encode_target_list(output, runtime, route.targets());
    output.push_str(",\"related_target\":");
    value::optional_target(output, runtime, route.related_target());
    output.push('}');
}

fn encode_target_list(
    output: &mut String,
    runtime: &RuntimeNamespace,
    targets: &[crate::TraceTarget],
) {
    output.push('[');
    for (index, target) in targets.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        value::target(output, runtime, target);
    }
    output.push(']');
}

fn encode_transition(
    output: &mut String,
    runtime: &RuntimeNamespace,
    transition: Option<&TraceTargetTransition>,
) {
    let Some(transition) = transition else {
        output.push_str("null");
        return;
    };
    output.push_str("{\"previous\":");
    value::optional_target(output, runtime, transition.previous());
    output.push_str(",\"current\":");
    value::optional_target(output, runtime, transition.current());
    output.push('}');
}

fn encode_pointer_cleanup(
    output: &mut String,
    runtime: &RuntimeNamespace,
    cleanup: Option<&TracePointerCleanup>,
) {
    let Some(cleanup) = cleanup else {
        output.push_str("null");
        return;
    };
    output.push_str("{\"pressed_owner\":");
    encode_transition(output, runtime, cleanup.pressed_owner());
    output.push_str(",\"capture_owner\":");
    encode_transition(output, runtime, cleanup.capture_owner());
    output.push_str(",\"physical_path_cleared\":");
    json::bool_value(output, cleanup.physical_path_cleared());
    output.push('}');
}
