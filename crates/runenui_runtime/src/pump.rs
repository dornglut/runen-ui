//! Explicit bounded pump and its single readiness-checkpoint authority.

#![allow(clippy::redundant_pub_crate)]

use crate::{
    RuntimeStatus, RuntimeTerminalReason, TraceRecordKind,
    app::UiApp,
    runtime::{ProcessApplicationActionOutcome, Runtime, process_application_action},
};

/// Processed-envelope limit for one explicit pump call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PumpBudget {
    max_processed_envelopes: usize,
}

impl PumpBudget {
    #[must_use]
    pub const fn new(max_processed_envelopes: usize) -> Self {
        Self {
            max_processed_envelopes,
        }
    }

    #[must_use]
    pub const fn max_processed_envelopes(self) -> usize {
        self.max_processed_envelopes
    }
}

/// Completion state of one bounded pump call.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PumpOutcome {
    Quiescent,
    BudgetExhausted,
    Closed,
    Terminal(RuntimeTerminalReason),
}

/// Exact observations from one bounded pump call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PumpReport {
    processed_envelopes: usize,
    remaining_queued_envelopes: usize,
    cancelled_by_terminal_transition: usize,
    outcome: PumpOutcome,
}

impl PumpReport {
    #[must_use]
    pub const fn processed_envelopes(self) -> usize {
        self.processed_envelopes
    }
    #[must_use]
    pub const fn remaining_queued_envelopes(self) -> usize {
        self.remaining_queued_envelopes
    }
    #[must_use]
    pub const fn cancelled_by_terminal_transition(self) -> usize {
        self.cancelled_by_terminal_transition
    }
    #[must_use]
    pub const fn outcome(self) -> PumpOutcome {
        self.outcome
    }
    #[must_use]
    pub const fn is_quiescent(self) -> bool {
        matches!(self.outcome, PumpOutcome::Quiescent)
    }
}

pub(crate) fn pump<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action>,
    budget: PumpBudget,
) -> PumpReport {
    let mut processed = 0usize;
    let mut cancelled = 0usize;

    readiness_checkpoint(runtime);
    while processed < budget.max_processed_envelopes() {
        if runtime.queue_is_empty() {
            readiness_checkpoint(runtime);
            return finish_report(runtime, processed, cancelled);
        }
        let Some(envelope) = runtime.pop_application_action() else {
            readiness_checkpoint(runtime);
            return finish_report(runtime, processed, cancelled);
        };
        let result = process_application_action::<App>(runtime, envelope);
        processed += 1;
        if let ProcessApplicationActionOutcome::Terminal {
            reason: _reason,
            cancelled: terminal_cancelled,
        } = result
        {
            cancelled = terminal_cancelled;
            readiness_checkpoint(runtime);
            return finish_report(runtime, processed, cancelled);
        }
        if processed < budget.max_processed_envelopes() {
            readiness_checkpoint(runtime);
        }
    }
    readiness_checkpoint(runtime);
    finish_report(runtime, processed, cancelled)
}

#[cfg(test)]
const fn readiness_checkpoint<State, Action>(runtime: &mut Runtime<State, Action>) {
    runtime.note_readiness_checkpoint();
}

#[cfg(not(test))]
const fn readiness_checkpoint<State, Action>(runtime: &Runtime<State, Action>) {
    let _ = runtime;
}

fn finish_report<State, Action>(
    runtime: &mut Runtime<State, Action>,
    processed: usize,
    cancelled: usize,
) -> PumpReport {
    let remaining = runtime.queued_len();
    let outcome = match runtime.status() {
        RuntimeStatus::Closed => PumpOutcome::Closed,
        RuntimeStatus::Terminal(reason) => PumpOutcome::Terminal(reason),
        RuntimeStatus::Running if remaining > 0 => {
            runtime.record_optional(TraceRecordKind::PumpBudgetExhausted, None, None, None);
            PumpOutcome::BudgetExhausted
        }
        RuntimeStatus::Running => PumpOutcome::Quiescent,
    };
    PumpReport {
        processed_envelopes: processed,
        remaining_queued_envelopes: remaining,
        cancelled_by_terminal_transition: cancelled,
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{Element, View, text};

    use crate::{RuntimeConfig, UiApp, queue::ApplicationActionOrigin, runtime::Runtime};

    use super::{PumpBudget, pump};

    struct App;

    impl UiApp for App {
        type State = usize;
        type Action = ();

        fn root(_: &Self::State) -> Element<Self::Action> {
            text("root").into_element()
        }

        fn update(state: &mut Self::State, (): Self::Action) {
            *state += 1;
        }
    }

    #[test]
    fn one_checkpoint_authority_runs_before_and_at_the_final_boundary() {
        let mut runtime = Runtime::mount(0, App::root, RuntimeConfig::default());
        runtime
            .submit_action((), ApplicationActionOrigin::DirectSubmission, None, None)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 0);
        let report = pump::<App>(&mut runtime, PumpBudget::new(0));
        assert_eq!(report.processed_envelopes(), 0);
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 2);
        let report = pump::<App>(&mut runtime, PumpBudget::new(1));
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(runtime.readiness_checkpoint_count_for_test(), 4);
    }
}
