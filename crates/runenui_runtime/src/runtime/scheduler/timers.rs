use super::{
    HostProtocol, MandatoryTracePlan, Runtime, RuntimeTerminalReason, SchedulerObservation,
    TimerFireOutcome, TimerFiringOutcome, TraceRecordKind, TraceTimerTerminalOutcome, WorkFamily,
    WorkSequence,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn promote_due_timers(&mut self, limit: usize) -> usize {
        let now = self.now();
        let mut due: Vec<_> = self
            .timers
            .iter()
            .filter(|timer| self.work.is_running(timer.generation) && timer.is_due(now))
            .map(|timer| (timer.deadline, timer.generation))
            .collect();
        due.sort_unstable();
        let mut promoted = 0;
        for (_deadline, generation) in due {
            if promoted >= limit || self.queue.is_full() {
                break;
            }
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                return promoted;
            }
            let Some(identity) = self.trace_work_identity(generation) else {
                continue;
            };
            if self.queue.push_timer_firing(generation).is_err() {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return promoted;
            }
            if let Some(timer) = self
                .timers
                .iter_mut()
                .find(|timer| timer.generation == generation)
            {
                timer.mark_promoted();
                promoted += 1;
            }
            self.record_work_fact(TraceRecordKind::TimerPromoted, identity);
        }
        self.timers
            .retain(|timer| self.work.is_running(timer.generation));
        promoted
    }

    pub(crate) fn scheduler_observation(&self) -> SchedulerObservation {
        let now = self.now();
        SchedulerObservation {
            completion_imports_pending: self.completion_ingress.len() > 0,
            due_timers_pending: self
                .timers
                .iter()
                .any(|timer| self.work.is_running(timer.generation) && timer.is_due(now)),
            local_polls_pending: self
                .local_tasks
                .iter()
                .any(|task| self.work.is_running(task.generation) && task.is_eligible())
                || self.subscriptions.iter().any(|subscription| {
                    self.work
                        .is_running_family(subscription.generation, WorkFamily::Subscription)
                        && subscription.is_local_eligible()
                }),
            mandatory_derived_work_pending: !self.mounted_subscription_reconcile_pending.is_empty(),
            next_deadline: self
                .timers
                .iter()
                .filter(|timer| self.work.is_running(timer.generation))
                .map(|timer| timer.deadline)
                .min(),
            publication_dirty: self.surface_publication.is_dirty(),
        }
    }

    pub(crate) fn process_timer_firing(
        &mut self,
        sequence: WorkSequence,
        generation: crate::work::WorkGeneration,
    ) {
        if !self.callback_output_preflight(
            Some((generation, WorkFamily::Timer)),
            MandatoryTracePlan::callback_with_action(),
        ) {
            return;
        }
        let Some(identity) = self.trace_work_identity(generation) else {
            return;
        };
        let firing_parent = self.record_work_fact_from_envelope(
            TraceRecordKind::TimerFired,
            sequence,
            identity.clone(),
        );
        let now = self.now();
        let Some(index) = self
            .timers
            .iter()
            .position(|timer| timer.generation == generation)
        else {
            return;
        };
        let (action, outcome) = self.timers[index].fire(now);
        self.last_timer_firing_outcome = Some(match outcome {
            TimerFireOutcome::Completed => TimerFiringOutcome::Completed,
            TimerFireOutcome::Rescheduled => TimerFiringOutcome::Rescheduled,
            TimerFireOutcome::RepeatDeadlineOverflow => TimerFiringOutcome::RepeatDeadlineOverflow,
        });
        let mut action_parent = firing_parent;
        if outcome != TimerFireOutcome::Rescheduled {
            let outcome = match outcome {
                TimerFireOutcome::Completed => TraceTimerTerminalOutcome::Completed,
                TimerFireOutcome::RepeatDeadlineOverflow => {
                    TraceTimerTerminalOutcome::RepeatDeadlineOverflow
                }
                TimerFireOutcome::Rescheduled => unreachable!(),
            };
            action_parent = self.record_work_fact_from_envelope(
                TraceRecordKind::TimerTerminated { outcome },
                sequence,
                identity,
            );
            self.timers.remove(index);
            self.revoke_generation(generation);
        }
        if self.queue.is_full() {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 1);
            return;
        }
        self.queue_callback_action(action, action_parent);
    }
}
