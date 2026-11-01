use super::{
    HashMap, HashSet, HostProtocol, LiveSubscription, MandatoryTracePlan, MountedNodeId, Runtime,
    RuntimeTerminalReason, Subscription, SubscriptionDiagnostic, SubscriptionDiff,
    SubscriptionOwnerKind, SubscriptionSet, TargetStatus, TraceRecordKind, TraceSequence,
    TraceTarget, WorkOwner, WorkSequence,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn process_mounted_subscription_reconcile(
        &mut self,
        sequence: WorkSequence,
        owner: &MountedNodeId,
        causal_parent: Option<TraceSequence>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) {
        self.mounted_subscription_reconcile_pending
            .retain(|pending| pending != owner);
        if self.tree.target_status(owner) != TargetStatus::Live {
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                return;
            }
            self.trace.record(
                TraceRecordKind::MountedSubscriptionReconciliationSuppressedStale,
                Some(sequence),
                causal_parent,
                None,
                None,
                Some(TraceTarget::new(owner.clone(), None)),
            );
            self.cancel_owner_work(&WorkOwner::Mounted(owner.clone()));
            return;
        }
        let declarations = if let Some(declarations) = self.tree.take_initial_subscriptions(owner) {
            declarations
        } else {
            let Ok(widget_budget) = self.tree.subscriptions_budget(owner) else {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return;
            };
            let mut subscriptions = match root_quota {
                Some(root) => match self.queue.root_profile(root) {
                    Some(profile)
                        if widget_budget.subscription_declarations()
                            <= profile.mounted_subscriptions_per_owner() =>
                    {
                        SubscriptionSet::__runtime_new_bounded(
                            widget_budget.subscription_declarations(),
                        )
                    }
                    Some(_) | None => {
                        self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                        return;
                    }
                },
                None => SubscriptionSet::__runtime_new_bounded(
                    widget_budget.subscription_declarations(),
                ),
            };
            subscriptions.__runtime_set_widget_budget(widget_budget.work_starts());
            if self
                .tree
                .declare_subscriptions(owner, &mut subscriptions)
                .is_err()
            {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return;
            }
            if let Some(cause) = subscriptions.producer_violation() {
                self.enter_terminal(RuntimeTerminalReason::ProducerContractViolation(cause), 0);
                return;
            }
            if subscriptions.__runtime_overflowed() {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return;
            }
            let declarations = subscriptions.__runtime_into_declarations();
            let mut local_tasks = 0usize;
            let mut send_tasks = 0usize;
            for declaration in &declarations {
                match &declaration.source {
                    runenui_core::__runtime::SubscriptionSource::Local(_) => {
                        let Some(count) = local_tasks.checked_add(1) else {
                            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                            return;
                        };
                        local_tasks = count;
                    }
                    runenui_core::__runtime::SubscriptionSource::Send { .. } => {
                        let Some(count) = send_tasks.checked_add(1) else {
                            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                            return;
                        };
                        send_tasks = count;
                    }
                }
            }
            let starts = widget_budget.work_starts();
            if local_tasks > starts.for_family(runenui_core::WorkFamily::LocalTask)
                || send_tasks > starts.for_family(runenui_core::WorkFamily::SendTask)
            {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return;
            }
            declarations
        };
        self.reconcile_subscriptions(
            &WorkOwner::Mounted(owner.clone()),
            declarations,
            Some(sequence),
            causal_parent,
            root_quota,
        );
    }

    pub(in crate::runtime) fn validate_subscription_residency(
        &self,
        root: Option<crate::queue::RootQuotaId>,
        invalidated: &[crate::work::WorkGeneration],
        starts: usize,
        phase: runenui_core::ProducerPhase,
    ) -> Result<(), RuntimeTerminalReason> {
        let Some(root) = root else {
            return Ok(());
        };
        let invalidated = invalidated.iter().copied().collect();
        let required = self
            .work
            .resident_family_requirement(
                crate::work::WorkFamily::Subscription,
                &invalidated,
                starts,
                Some(root),
            )
            .ok_or(RuntimeTerminalReason::Poisoned)?;
        let declared = self
            .queue
            .root_work_residency(root, crate::work::WorkFamily::Subscription)
            .ok_or(RuntimeTerminalReason::Poisoned)?;
        if required > declared {
            return Err(RuntimeTerminalReason::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase,
                    component: runenui_core::BudgetComponent::SubscriptionResidency,
                    declared,
                    attempted: Some(required),
                },
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(in crate::runtime) fn reconcile_subscriptions(
        &mut self,
        owner: &WorkOwner,
        declarations: Vec<Subscription<Action>>,
        work_sequence: Option<WorkSequence>,
        transaction_parent: Option<TraceSequence>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) {
        let SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        } = self.derive_subscription_diff(owner, declarations);
        if let Err(reason) = self.validate_subscription_residency(
            root_quota,
            &invalidated,
            starts.len(),
            runenui_core::ProducerPhase::Widget(runenui_core::WidgetProducerPhase::Subscriptions),
        ) {
            self.enter_terminal(reason, 0);
            return;
        }
        let invalidated_set: HashSet<_> = invalidated.iter().copied().collect();
        let Ok((generations, next_generation)) = self.work.preview_generations(starts.len()) else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let sequenced = invalidated.len().checked_add(starts.len());
        if sequenced.is_none_or(|count| {
            root_quota.map_or_else(
                || self.queue.preflight_commit(count).is_err(),
                |root| self.queue.preflight_root_commit(root, count).is_err(),
            )
        }) || self
            .work
            .preflight_subscriptions(&invalidated_set, starts.len(), &self.queue, root_quota)
            .is_err()
            || root_quota.is_some_and(|root| {
                self.queue
                    .preflight_root_work_leases(root, starts.len())
                    .is_err()
            })
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        }
        let required_trace_records = invalidated
            .len()
            .saturating_mul(2)
            .saturating_add(starts.len().saturating_mul(3))
            .saturating_add(1);
        if !self
            .trace
            .can_admit(MandatoryTracePlan::planned_scheduler_transaction(
                required_trace_records,
            ))
        {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }

        self.record_subscription_duplicates(owner, &duplicate_keys);
        let cancelled_count = invalidated.len();
        let started_count = starts.len();
        let transaction_parent = self.trace.record(
            TraceRecordKind::SubscriptionDiffCommitted {
                started: started_count,
                cancelled: cancelled_count,
                duplicate_keys: duplicate_keys.len(),
            },
            work_sequence,
            transaction_parent,
            None,
            None,
            None,
        );
        let invalidated_identities: Vec<_> = invalidated
            .iter()
            .filter_map(|generation| self.trace_work_identity(*generation))
            .collect();
        self.work.commit_generation_reservation(next_generation);
        let cancellation_lineage =
            self.record_invalidation_facts(&invalidated_identities, transaction_parent);
        for generation in &invalidated {
            self.invalidate_generation_now(*generation);
        }
        let start_generations = generations.clone();
        for (generation, declaration) in generations.into_iter().zip(starts) {
            let key = declaration.key.clone();
            if let Some(root_quota) = root_quota {
                self.queue
                    .retain_root_work_lease(root_quota)
                    .unwrap_or_else(|_| unreachable!("subscription root leases were preflighted"));
            }
            self.work
                .commit_subscription_record(generation, owner.clone(), key, root_quota);
            self.subscriptions.push(LiveSubscription::new(
                generation,
                owner.clone(),
                declaration,
                self.wake.handle(),
            ));
            let identity = self
                .trace_work_identity(generation)
                .unwrap_or_else(|| unreachable!("committed subscription has trace identity"));
            self.record_work_fact_with_parent(
                TraceRecordKind::WorkRequested,
                transaction_parent,
                identity.clone(),
            );
            self.record_work_fact(TraceRecordKind::SubscriptionDeclared, identity.clone());
            self.record_work_fact(TraceRecordKind::WorkGenerationCommitted, identity);
        }
        for generation in invalidated {
            let (identity, parent) = cancellation_lineage
                .get(&generation.get())
                .cloned()
                .unwrap_or_else(|| unreachable!("cancelled subscription retains trace lineage"));
            if let Some(root_quota) = root_quota {
                self.queue
                    .push_cancellation_rooted(generation, identity, parent, root_quota)
                    .unwrap_or_else(|_| unreachable!("subscription root quota was preflighted"));
            } else {
                self.queue
                    .push_cancellation(generation, identity, parent)
                    .unwrap_or_else(|_| unreachable!("subscription diff was preflighted"));
            }
        }
        for generation in start_generations {
            if let Some(root_quota) = root_quota {
                self.queue
                    .push_effect_start_rooted(generation, root_quota)
                    .unwrap_or_else(|_| unreachable!("subscription root quota was preflighted"));
            } else {
                self.queue
                    .push_effect_start(generation)
                    .unwrap_or_else(|_| unreachable!("subscription diff was preflighted"));
            }
        }
    }

    pub(in crate::runtime) fn derive_subscription_diff(
        &self,
        owner: &WorkOwner,
        declarations: Vec<Subscription<Action>>,
    ) -> SubscriptionDiff<Action> {
        let mut counts = HashMap::new();
        for declaration in &declarations {
            *counts.entry(declaration.key.clone()).or_insert(0usize) += 1;
        }
        let duplicate_keys: HashSet<_> = counts
            .into_iter()
            .filter_map(|(key, count)| (count > 1).then_some(key))
            .collect();
        let desired: HashMap<_, _> = declarations
            .iter()
            .filter(|declaration| !duplicate_keys.contains(&declaration.key))
            .map(|declaration| (declaration.key.clone(), declaration))
            .collect();
        let invalidated: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|subscription| {
                if &subscription.owner != owner {
                    return false;
                }
                desired.get(&subscription.key).is_none_or(|declaration| {
                    subscription.source_type != declaration.source_type
                        || subscription.revision != declaration.revision
                })
            })
            .map(|subscription| subscription.generation)
            .collect();
        let starts: Vec<_> = declarations
            .into_iter()
            .filter(|declaration| !duplicate_keys.contains(&declaration.key))
            .filter(|declaration| {
                !self.subscriptions.iter().any(|subscription| {
                    &subscription.owner == owner
                        && subscription.key == declaration.key
                        && subscription.source_type == declaration.source_type
                        && subscription.revision == declaration.revision
                })
            })
            .collect();
        SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        }
    }

    pub(in crate::runtime) fn record_subscription_duplicates(
        &mut self,
        owner: &WorkOwner,
        duplicate_keys: &HashSet<runenui_core::WorkKey>,
    ) {
        let owner = match owner {
            WorkOwner::Application => SubscriptionOwnerKind::Application,
            WorkOwner::Mounted(_) => SubscriptionOwnerKind::Mounted,
        };
        let mut duplicate_keys: Vec<_> = duplicate_keys.iter().cloned().collect();
        duplicate_keys.sort_unstable();
        let limit = self.limits.subscription_diagnostics();
        if limit == 0 {
            return;
        }
        for key in duplicate_keys {
            if self.subscription_diagnostics.len() == limit {
                self.subscription_diagnostics.remove(0);
            }
            self.subscription_diagnostics
                .push(SubscriptionDiagnostic::DuplicateKey { owner, key });
        }
    }
}
