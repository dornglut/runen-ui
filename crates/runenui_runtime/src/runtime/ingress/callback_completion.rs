use super::{
    HostProtocol, Runtime, RuntimeTerminalReason, TraceActionCategory, TraceSequence, WorkSequence,
};
use crate::queue::RootQuotaId;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn queue_callback_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
    ) -> Option<WorkSequence> {
        let instant = self.now();
        self.admit_and_commit_action(
            action,
            causal_parent,
            None,
            TraceActionCategory::ApplicationEffect,
            instant,
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_callback_action_from_root(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        root_quota: RootQuotaId,
    ) -> Option<WorkSequence> {
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        let result = self.commit_preflighted_action_from_root(
            action,
            producer_budget,
            root_quota,
            causal_parent,
            None,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        );
        result.map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_timer_callback_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        root_quota: RootQuotaId,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_timer_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_callback_action_for_work(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        match self.work.root_quota(generation) {
            Some(root_quota) => {
                self.queue_callback_action_from_root(action, causal_parent, root_quota)
            }
            None => self.queue_callback_action(action, causal_parent),
        }
    }

    pub(in crate::runtime) fn queue_local_task_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(root_quota) = self.work.root_quota(generation) else {
            return self.queue_callback_action(action, causal_parent);
        };
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_local_task_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_send_task_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(root_quota) = self.work.root_quota(generation) else {
            return self.queue_callback_action(action, causal_parent);
        };
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_send_task_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_host_response_action_for_work(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(root_quota) = self.work.root_quota(generation) else {
            return self.queue_callback_action(action, causal_parent);
        };
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        if let Some(parent) = self.queue.root_profile(root_quota)
            && let Some((component, declared, attempted)) =
                crate::transaction::uncovered_component(parent, producer_budget)
        {
            self.enter_terminal(
                RuntimeTerminalReason::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DescendantProfileNotCovered,
                        phase: runenui_core::ProducerPhase::ApplicationUpdate,
                        component,
                        declared,
                        attempted: Some(attempted),
                    },
                ),
                0,
            );
            return None;
        }
        self.commit_preflighted_host_response_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }
}
