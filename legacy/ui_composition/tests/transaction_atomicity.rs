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
            "Resolve lifecycle policy before committing.",
        )])
    }
}

fn policies<'a>(
    lifecycle: &'a dyn CompositionLifecyclePolicy,
    allow: &'a Allow,
) -> CompositionPolicies<'a> {
    CompositionPolicies {
        lifecycle,
        capability: allow,
        target: allow,
    }
}

fn unit(id: u64) -> MountedUnitDefinition {
    MountedUnitDefinition::new(
        MountedUnitId::new(id),
        MountedContentRef::new(
            ContentOwnerId::new("fixture.owner").unwrap(),
            ContentProfileId::new("fixture.content").unwrap(),
            ContentInstanceRef::new(format!("fixture.unit-{id}")).unwrap(),
        ),
        [],
        UnavailableContentPolicy::ShowFallback,
    )
}

fn state() -> CompositionState {
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
        vec![unit(1), unit(2)],
    ))
    .unwrap()
}

fn split_state() -> CompositionState {
    CompositionState::form(CompositionDefinitionV1::new(
        CompositionDefinitionId::new(2),
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
        vec![
            RegionDefinition::new(
                RegionId::new(1),
                None,
                RegionKind::Split {
                    axis: SplitAxis::Horizontal,
                    fraction: SplitFraction::try_new(5_000).unwrap(),
                    first: RegionId::new(2),
                    second: RegionId::new(3),
                },
            ),
            RegionDefinition::new(
                RegionId::new(2),
                None,
                RegionKind::Stack {
                    ordered_units: vec![MountedUnitId::new(1)],
                    active_unit: MountedUnitId::new(1),
                },
            ),
            RegionDefinition::new(
                RegionId::new(3),
                None,
                RegionKind::Stack {
                    ordered_units: vec![MountedUnitId::new(2)],
                    active_unit: MountedUnitId::new(2),
                },
            ),
        ],
        vec![unit(1), unit(2)],
    ))
    .unwrap()
}

#[test]
fn policy_veto_and_stale_revision_leave_all_state_unchanged() {
    let mut state = state();
    let before = state.clone();
    let allow = Allow;
    let tx = CompositionTransaction::new(
        CompositionTransactionId::new(1),
        state.revision(),
        vec![CompositionCommand::activate_unit(
            RegionId::new(1),
            MountedUnitId::new(2),
        )],
    );
    assert!(state.transact(tx, policies(&Reject, &allow)).is_err());
    assert_eq!(state, before);

    let stale = CompositionTransaction::new(
        CompositionTransactionId::new(2),
        StateRevision::new(99),
        vec![CompositionCommand::activate_unit(
            RegionId::new(1),
            MountedUnitId::new(2),
        )],
    );
    assert!(state.transact(stale, policies(&allow, &allow)).is_err());
    assert_eq!(state, before);
}

#[test]
fn multi_command_failure_rolls_back_and_success_commits_once() {
    let mut state = state();
    let before = state.clone();
    let allow = Allow;
    let invalid = CompositionTransaction::new(
        CompositionTransactionId::new(1),
        state.revision(),
        vec![
            CompositionCommand::activate_unit(RegionId::new(1), MountedUnitId::new(2)),
            CompositionCommand::activate_unit(RegionId::new(9), MountedUnitId::new(1)),
        ],
    );
    assert!(state.transact(invalid, policies(&allow, &allow)).is_err());
    assert_eq!(state, before);

    let valid = CompositionTransaction::new(
        CompositionTransactionId::new(2),
        state.revision(),
        vec![CompositionCommand::activate_unit(
            RegionId::new(1),
            MountedUnitId::new(2),
        )],
    );
    let commit = state.transact(valid, policies(&allow, &allow)).unwrap();
    assert_eq!(commit.revision, StateRevision::new(2));
    let RegionKind::Stack { active_unit, .. } =
        &state.snapshot().region(RegionId::new(1)).unwrap().kind
    else {
        panic!()
    };
    assert_eq!(*active_unit, MountedUnitId::new(2));
    assert_eq!(state.history().journal().len(), 1);
}

#[test]
fn resize_only_batch_preserves_global_invariants_and_rejects_atomically() {
    let mut state = split_state();
    let before = state.clone();
    let allow = Allow;
    let invalid = CompositionTransaction::new(
        CompositionTransactionId::new(20),
        state.revision(),
        vec![
            CompositionCommand::resize_split(
                RegionId::new(1),
                SplitFraction::try_new(4_900).unwrap(),
            ),
            CompositionCommand::resize_split(
                RegionId::new(2),
                SplitFraction::try_new(5_100).unwrap(),
            ),
        ],
    );
    assert!(state.transact(invalid, policies(&allow, &allow)).is_err());
    assert_eq!(state, before);

    let valid = CompositionTransaction::new(
        CompositionTransactionId::new(21),
        state.revision(),
        (0..64)
            .map(|index| {
                CompositionCommand::resize_split(
                    RegionId::new(1),
                    SplitFraction::try_new(4_900 + index).unwrap(),
                )
            })
            .collect(),
    );
    state.transact(valid, policies(&allow, &allow)).unwrap();

    let RegionKind::Split { fraction, .. } =
        &state.snapshot().region(RegionId::new(1)).unwrap().kind
    else {
        panic!("fixture root must remain a split")
    };
    assert_eq!(fraction.basis_points(), 4_963);
    assert_eq!(state.history().journal().len(), 1);
}

#[test]
fn duplicate_transaction_id_rejects_after_commit() {
    let mut state = state();
    let allow = Allow;
    let tx = CompositionTransaction::new(
        CompositionTransactionId::new(1),
        state.revision(),
        vec![CompositionCommand::activate_unit(
            RegionId::new(1),
            MountedUnitId::new(2),
        )],
    );
    state.transact(tx, policies(&allow, &allow)).unwrap();
    let duplicate = CompositionTransaction::new(
        CompositionTransactionId::new(1),
        state.revision(),
        vec![CompositionCommand::activate_unit(
            RegionId::new(1),
            MountedUnitId::new(1),
        )],
    );
    let rejection = state
        .transact(duplicate, policies(&allow, &allow))
        .unwrap_err();
    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|value| value.code() == CompositionDiagnosticCode::DuplicateTransactionId)
    );
}

#[test]
fn extension_ratification_advances_revision_without_changing_structure() {
    let mut state = state();
    let definition_before = state.definition().clone();
    let allow = Allow;
    let commit = state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(77),
                state.revision(),
                vec![CompositionCommand::ratify_extension_state()],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    assert_eq!(commit.revision, StateRevision::new(2));
    assert_eq!(state.definition().targets(), definition_before.targets());
    assert_eq!(state.definition().roots(), definition_before.roots());
    assert_eq!(state.definition().regions(), definition_before.regions());
    assert_eq!(
        state.definition().mounted_units(),
        definition_before.mounted_units()
    );
}

#[test]
fn revision_overflow_rejects_without_mutating_state() {
    let initial = state();
    let definition = initial.definition();
    let mut state = CompositionState::form(CompositionDefinitionV1::new(
        definition.id(),
        DefinitionRevision::new(u64::MAX),
        definition.targets().to_vec(),
        definition.roots().to_vec(),
        definition.regions().to_vec(),
        definition.mounted_units().to_vec(),
    ))
    .unwrap();
    let before = state.clone();
    let allow = Allow;
    let transaction = CompositionTransaction::new(
        CompositionTransactionId::new(99),
        state.revision(),
        vec![CompositionCommand::activate_unit(
            RegionId::new(1),
            MountedUnitId::new(2),
        )],
    );

    let rejection = state
        .transact(transaction, policies(&allow, &allow))
        .unwrap_err();

    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|record| record.code() == CompositionDiagnosticCode::RevisionOverflow)
    );
    assert_eq!(state, before);
}

#[test]
fn serialized_history_restore_cannot_bypass_history_authorization() {
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
            policies(&allow, &allow),
        )
        .unwrap();
    let encoded =
        serde_json::to_string(&state.history().journal()[0].inverse_commands()[0]).unwrap();
    let injected = serde_json::from_str::<CompositionCommand>(&encoded).unwrap();
    let before = state.clone();
    let rejection = state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(2),
                state.revision(),
                vec![injected],
            ),
            policies(&allow, &allow),
        )
        .unwrap_err();

    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|record| record.code() == CompositionDiagnosticCode::InvalidCommand)
    );
    assert_eq!(state, before);
}

#[test]
fn moved_unit_topology_commands_are_atomic_and_never_require_invalid_placeholder_stacks() {
    let allow = Allow;
    let mut split = state();
    split
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(30),
                split.revision(),
                vec![CompositionCommand::split_region_with_moved_unit(
                    RegionId::new(1),
                    RegionId::new(2),
                    RegionDefinition::new(
                        RegionId::new(3),
                        None,
                        RegionKind::Stack {
                            ordered_units: vec![MountedUnitId::new(2)],
                            active_unit: MountedUnitId::new(2),
                        },
                    ),
                    MountedUnitId::new(2),
                    SplitAxis::Horizontal,
                    SplitFraction::try_new(5_000).unwrap(),
                    false,
                )],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    let RegionKind::Stack { ordered_units, .. } =
        &split.snapshot().region(RegionId::new(2)).unwrap().kind
    else {
        panic!("preserved child must remain a stack")
    };
    assert_eq!(ordered_units, &[MountedUnitId::new(1)]);
    let RegionKind::Stack { ordered_units, .. } =
        &split.snapshot().region(RegionId::new(3)).unwrap().kind
    else {
        panic!("moved child must be a stack")
    };
    assert_eq!(ordered_units, &[MountedUnitId::new(2)]);

    let mut detached = state();
    detached
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(31),
                detached.revision(),
                vec![
                    CompositionCommand::attach_target(PresentationTargetDefinition::new(
                        PresentationTargetId::new(2),
                        TargetProfileId::new("fixture.secondary").unwrap(),
                    )),
                    CompositionCommand::create_root_with_moved_unit(
                        CompositionRootDefinition::new(
                            CompositionRootId::new(2),
                            PresentationTargetId::new(2),
                            RegionId::new(2),
                            true,
                        ),
                        RegionDefinition::new(
                            RegionId::new(2),
                            None,
                            RegionKind::Stack {
                                ordered_units: vec![MountedUnitId::new(2)],
                                active_unit: MountedUnitId::new(2),
                            },
                        ),
                        MountedUnitId::new(2),
                    ),
                ],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    assert_eq!(detached.snapshot().targets().len(), 2);
    assert_eq!(detached.snapshot().roots().len(), 2);
    let RegionKind::Stack { ordered_units, .. } =
        &detached.snapshot().region(RegionId::new(1)).unwrap().kind
    else {
        panic!("source must remain a stack")
    };
    assert_eq!(ordered_units, &[MountedUnitId::new(1)]);
}

#[test]
fn malformed_moved_unit_topology_payload_rejects_without_mutation() {
    let allow = Allow;
    let mut state = state();
    let before = state.clone();
    let rejection = state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(32),
                state.revision(),
                vec![CompositionCommand::create_root_with_moved_unit(
                    CompositionRootDefinition::new(
                        CompositionRootId::new(2),
                        PresentationTargetId::new(1),
                        RegionId::new(2),
                        false,
                    ),
                    RegionDefinition::new(
                        RegionId::new(2),
                        None,
                        RegionKind::Stack {
                            ordered_units: vec![MountedUnitId::new(1)],
                            active_unit: MountedUnitId::new(1),
                        },
                    ),
                    MountedUnitId::new(2),
                )],
            ),
            policies(&allow, &allow),
        )
        .unwrap_err();

    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|record| record.code() == CompositionDiagnosticCode::InvalidCommand)
    );
    assert_eq!(state, before);
}

#[test]
fn every_structural_command_family_has_a_valid_atomic_path() {
    let mut state = state();
    let allow = Allow;
    let fraction = SplitFraction::try_new(5000).unwrap();

    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(10),
                state.revision(),
                vec![
                    CompositionCommand::split_region(
                        RegionId::new(1),
                        RegionId::new(2),
                        RegionDefinition::new(
                            RegionId::new(3),
                            None,
                            RegionKind::Stack {
                                ordered_units: vec![],
                                active_unit: MountedUnitId::new(2),
                            },
                        ),
                        SplitAxis::Horizontal,
                        fraction,
                        false,
                    ),
                    CompositionCommand::move_unit(MountedUnitId::new(2), RegionId::new(3), 0),
                    CompositionCommand::resize_split(
                        RegionId::new(1),
                        SplitFraction::try_new(6000).unwrap(),
                    ),
                ],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(11),
                state.revision(),
                vec![
                    CompositionCommand::move_unit(MountedUnitId::new(2), RegionId::new(2), 1),
                    CompositionCommand::merge_split(RegionId::new(1), RegionId::new(2)),
                    CompositionCommand::reorder_stack(
                        RegionId::new(1),
                        vec![MountedUnitId::new(2), MountedUnitId::new(1)],
                        MountedUnitId::new(2),
                    ),
                ],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    let unit_three = unit(3);
    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(12),
                state.revision(),
                vec![
                    CompositionCommand::attach_target(PresentationTargetDefinition::new(
                        PresentationTargetId::new(2),
                        TargetProfileId::new("fixture.secondary").unwrap(),
                    )),
                    CompositionCommand::create_root(
                        CompositionRootDefinition::new(
                            CompositionRootId::new(2),
                            PresentationTargetId::new(2),
                            RegionId::new(4),
                            true,
                        ),
                        vec![RegionDefinition::new(
                            RegionId::new(4),
                            None,
                            RegionKind::Stack {
                                ordered_units: vec![],
                                active_unit: MountedUnitId::new(3),
                            },
                        )],
                    ),
                    CompositionCommand::mount_unit(unit_three.clone(), RegionId::new(4), 0),
                ],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(13),
                state.revision(),
                vec![
                    CompositionCommand::move_root(
                        CompositionRootId::new(2),
                        PresentationTargetId::new(1),
                        false,
                    ),
                    CompositionCommand::detach_target(PresentationTargetId::new(2)),
                    CompositionCommand::move_unit(MountedUnitId::new(3), RegionId::new(1), 2),
                    CompositionCommand::close_root(CompositionRootId::new(2)),
                ],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(14),
                state.revision(),
                vec![CompositionCommand::unmount_unit(MountedUnitId::new(3))],
            ),
            policies(&allow, &allow),
        )
        .unwrap();
    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(15),
                state.revision(),
                vec![
                    CompositionCommand::mount_unit(unit_three, RegionId::new(1), 2),
                    CompositionCommand::activate_unit(RegionId::new(1), MountedUnitId::new(3)),
                ],
            ),
            policies(&allow, &allow),
        )
        .unwrap();

    assert_eq!(state.snapshot().targets().len(), 1);
    assert_eq!(state.snapshot().roots().len(), 1);
    assert_eq!(state.snapshot().mounted_units().len(), 3);
}
