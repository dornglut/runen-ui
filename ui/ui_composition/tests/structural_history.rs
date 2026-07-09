use ui_composition::*;

struct Allow;
impl CompositionLifecyclePolicy for Allow {
    fn evaluate(
        &self,
        _: CompositionSnapshot<'_>,
        _: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}
impl CompositionCapabilityPolicy for Allow {
    fn evaluate(
        &self,
        _: CompositionSnapshot<'_>,
        _: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}
impl CompositionTargetPolicy for Allow {
    fn evaluate(
        &self,
        _: CompositionSnapshot<'_>,
        _: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}
fn policies(allow: &Allow) -> CompositionPolicies<'_> {
    CompositionPolicies {
        lifecycle: allow,
        capability: allow,
        target: allow,
    }
}

fn state() -> CompositionState {
    let content = |id| {
        MountedContentRef::new(
            ContentOwnerId::new("fixture.owner").unwrap(),
            ContentProfileId::new("fixture.content").unwrap(),
            ContentInstanceRef::new(format!("fixture.{id}")).unwrap(),
        )
    };
    CompositionState::form(CompositionDefinitionV1::new(
        CompositionDefinitionId::new(1),
        DefinitionRevision::new(1),
        vec![PresentationTargetDefinition::new(
            PresentationTargetId::new(1),
            TargetProfileId::new("fixture.desktop").unwrap(),
        )],
        vec![CompositionRootDefinition::new(
            CompositionRootId::new(1),
            PresentationTargetId::new(1),
            RegionId::new(1),
            true,
        )],
        vec![RegionDefinition::new(
            RegionId::new(1),
            None,
            RegionKind::Stack {
                ordered_units: vec![MountedUnitId::new(1), MountedUnitId::new(2)],
                active_unit: MountedUnitId::new(1),
            },
        )],
        vec![
            MountedUnitDefinition::new(
                MountedUnitId::new(1),
                content(1),
                [],
                UnavailableContentPolicy::ShowFallback,
            ),
            MountedUnitDefinition::new(
                MountedUnitId::new(2),
                content(2),
                [],
                UnavailableContentPolicy::ShowFallback,
            ),
        ],
    ))
    .unwrap()
}

fn active(state: &CompositionState) -> MountedUnitId {
    let RegionKind::Stack { active_unit, .. } =
        &state.snapshot().region(RegionId::new(1)).unwrap().kind
    else {
        panic!()
    };
    *active_unit
}

#[test]
fn undo_and_redo_revalidate_as_new_transactions() {
    let mut state = state();
    let allow = Allow;
    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(1),
                state.revision(),
                vec![CompositionCommand::activate_unit(
                    RegionId::new(1),
                    MountedUnitId::new(2),
                )],
            ),
            policies(&allow),
        )
        .unwrap();
    assert_eq!(active(&state), MountedUnitId::new(2));
    state
        .undo(CompositionTransactionId::new(2), policies(&allow))
        .unwrap();
    assert_eq!(active(&state), MountedUnitId::new(1));
    assert!(state.history().can_redo());
    state
        .redo(CompositionTransactionId::new(3), policies(&allow))
        .unwrap();
    assert_eq!(active(&state), MountedUnitId::new(2));
    assert_eq!(state.revision(), StateRevision::new(4));
}

#[test]
fn rejected_undo_does_not_change_state_or_history() {
    struct Reject;
    impl CompositionLifecyclePolicy for Reject {
        fn evaluate(
            &self,
            _: CompositionSnapshot<'_>,
            tx: &CompositionTransaction,
        ) -> CompositionPolicyDecision {
            CompositionPolicyDecision::Rejected(vec![CompositionDiagnosticRecord::error(
                CompositionDiagnosticCode::PolicyRejected,
                CompositionDiagnosticStage::Policy,
                CompositionDiagnosticSubject::Transaction(tx.id()),
                "Undo denied.",
            )])
        }
    }
    let mut state = state();
    let allow = Allow;
    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(1),
                state.revision(),
                vec![CompositionCommand::activate_unit(
                    RegionId::new(1),
                    MountedUnitId::new(2),
                )],
            ),
            policies(&allow),
        )
        .unwrap();
    let before = state.clone();
    let reject = Reject;
    assert!(
        state
            .undo(
                CompositionTransactionId::new(2),
                CompositionPolicies {
                    lifecycle: &reject,
                    capability: &allow,
                    target: &allow
                }
            )
            .is_err()
    );
    assert_eq!(state, before);
}
