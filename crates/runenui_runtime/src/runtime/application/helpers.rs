use super::{HostProtocol, PlannedApplicationTransaction, PlannedOutput, WorkFamily};

pub(in crate::runtime) fn required_application_transaction_trace_records<
    Action,
    Protocol: HostProtocol,
>(
    plan: &PlannedApplicationTransaction<Action, Protocol>,
) -> usize {
    required_application_transaction_trace_records_from_parts(
        &plan.invalidated,
        &plan.starts,
        &plan.application_outputs,
        &plan.mounted_outputs,
    )
    .unwrap_or(usize::MAX)
}

pub(in crate::runtime) fn required_application_transaction_trace_records_from_parts<
    Action,
    Protocol: HostProtocol,
>(
    invalidated: &[crate::work::WorkGeneration],
    starts: &[crate::transaction::PlannedOwnedStart<Action, Protocol>],
    application_outputs: &[PlannedOutput<Action>],
    mounted_outputs: &[PlannedOutput<Action>],
) -> Option<usize> {
    let outputs = application_outputs.iter().chain(mounted_outputs);
    let action_count = outputs
        .clone()
        .filter(|output| matches!(output, PlannedOutput::Action(_)))
        .count();
    let redraw_count = outputs
        .filter(|output| matches!(output, PlannedOutput::Redraw))
        .count();
    let subscription_start_count = starts
        .iter()
        .filter(|start| start.family == WorkFamily::Subscription)
        .count();
    invalidated
        .len()
        .checked_mul(2)?
        .checked_add(starts.len().checked_mul(2)?)?
        .checked_add(subscription_start_count)?
        .checked_add(action_count)?
        .checked_add(redraw_count)?
        .checked_add(1)
}
