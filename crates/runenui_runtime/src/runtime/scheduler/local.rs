use super::{
    HashSet, HostProtocol, LiveSubscription, MandatoryTracePlan, Runtime, SubscriptionPoll,
    TaskReady, TraceRecordKind, WorkFamily,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    #[allow(clippy::too_many_lines)]
    pub(in crate::runtime) fn poll_local_work(&mut self, limit: usize) -> usize {
        self.local_tasks
            .retain(|task| self.work.is_running(task.generation));
        self.subscriptions.retain(|subscription| {
            self.work
                .is_live_family(subscription.generation, WorkFamily::Subscription)
        });

        let mut visited = HashSet::new();
        let mut polled = 0;
        while polled < limit && !self.queue.is_full() {
            let next_task = self
                .local_tasks
                .iter()
                .filter(|task| task.is_eligible() && !visited.contains(&task.generation))
                .map(|task| task.generation)
                .min();
            let next_subscription = self
                .subscriptions
                .iter()
                .filter(|subscription| {
                    subscription.is_local_eligible() && !visited.contains(&subscription.generation)
                })
                .map(|subscription| subscription.generation)
                .min();
            let next = match (next_task, next_subscription) {
                (Some(task), Some(subscription)) => Some(task.min(subscription)),
                (Some(task), None) => Some(task),
                (None, Some(subscription)) => Some(subscription),
                (None, None) => None,
            };
            let Some(generation) = next else {
                break;
            };
            visited.insert(generation);

            let family = if self
                .local_tasks
                .iter()
                .any(|task| task.generation == generation)
            {
                WorkFamily::LocalTask
            } else {
                WorkFamily::Subscription
            };
            if !self.callback_output_preflight(
                Some((generation, family)),
                MandatoryTracePlan::callback_with_action(),
            ) {
                break;
            }
            let identity = self
                .trace_work_identity(generation)
                .unwrap_or_else(|| unreachable!("live local work has trace identity"));
            self.record_work_fact(TraceRecordKind::LocalWorkPolled, identity.clone());

            if let Some(task_index) = self
                .local_tasks
                .iter()
                .position(|task| task.generation == generation)
            {
                let ready = {
                    let task = &mut self.local_tasks[task_index];
                    task.poll_once().then(|| task.take_ready())
                };
                let Some(ready) = ready else {
                    continue;
                };
                polled += 1;
                match ready {
                    TaskReady::Complete(Some(action)) => {
                        let ready =
                            self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                        self.revoke_generation(generation);
                        self.queue_callback_action(action, ready);
                    }
                    TaskReady::Complete(None) => {
                        self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                        self.revoke_generation(generation);
                    }
                    TaskReady::NotReady => {}
                }
                continue;
            }

            let poll = self
                .subscriptions
                .iter_mut()
                .find(|subscription| subscription.generation == generation)
                .map_or(
                    SubscriptionPoll::NotEligible,
                    LiveSubscription::poll_local_once,
                );
            match poll {
                SubscriptionPoll::Item(action) => {
                    polled += 1;
                    let ready = self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                    self.queue_callback_action(action, ready);
                }
                SubscriptionPoll::Closed => {
                    polled += 1;
                    self.record_work_fact(TraceRecordKind::LocalWorkReady, identity);
                    self.revoke_generation(generation);
                }
                SubscriptionPoll::Pending => polled += 1,
                SubscriptionPoll::NotEligible => {}
            }
        }
        self.local_tasks
            .retain(|task| self.work.is_running(task.generation));
        self.subscriptions.retain(|subscription| {
            self.work
                .is_live_family(subscription.generation, WorkFamily::Subscription)
        });
        polled
    }
}
