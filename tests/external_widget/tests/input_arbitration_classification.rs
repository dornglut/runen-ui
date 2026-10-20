use std::time::Duration;

use runenui_core::{
    ApplicationCommand, ApplicationCommandId, CommandOrigin, Element, ElementId, FocusGroup,
    FocusGroupTypeAhead, HitContribution, HitContributionContext, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, LogicalLength,
    LogicalPoint, LogicalRect, LogicalSize, NoHostProtocol, PhysicalKey, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SemanticCommand,
    ShortcutBinding, ShortcutChord, ShortcutRepeatPolicy, StyleEnvironment, UiApp, View, Widget,
    WidgetActivation, WidgetActivationContext, WidgetActivationOutput, WidgetMeasure, button,
    column, shortcut_scope,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputPumpBatch, MountedNodeId, PumpBudget, RuntimeConfig,
    RuntimeStatus, SurfaceBuildContext, TraceConfig, TraceRecordKind, UiDefaultDisposition,
    UiInputClaimReason, UiInputConflict, UiInputFinality, UiInputProcessingRejection,
    UiInputSettlement,
};

const FULL: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

struct App;

#[derive(Debug)]
struct HitProbe;

impl Widget<()> for HitProbe {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn activation(&self, (): &()) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn activate(
        &mut self,
        (): &mut (),
        _: &mut WidgetActivationContext<()>,
    ) -> WidgetActivationOutput<()> {
        WidgetActivationOutput::action(())
    }
    fn measure(&self, (): &(), _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(32_u16), LogicalLength::from(32_u16))
    }
    fn hit_test(&self, (): &(), context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("finite hit geometry")),
        )
    }
}

impl UiApp for App {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(replaced: &bool) -> impl View<()> {
        let group = column(vec![
            Element::new(HitProbe)
                .id("alpha")
                .key(if *replaced { "replacement" } else { "original" })
                .focusable(true)
                .focus_group_search_text("alpha"),
            button("beta")
                .id("beta")
                .key("beta")
                .on_activate(|| ())
                .into_element()
                .focus_group_search_text("beta"),
        ])
        .into_element()
        .focus_group(
            FocusGroup::new().with_type_ahead(
                FocusGroupTypeAhead::new(Duration::from_millis(500))
                    .unwrap_or_else(|_| unreachable!("bounded search timeout")),
            ),
        );
        let shortcut = ShortcutBinding::new(
            ShortcutChord::logical(LogicalKey::Enter, KeyModifiers::NONE),
            ShortcutRepeatPolicy::IgnoreRepeat,
            ApplicationCommand::new(
                ApplicationCommandId::from_static("fixture.enter")
                    .unwrap_or_else(|_| unreachable!("valid command")),
                true,
            ),
        );
        let modified_arrow = ShortcutBinding::new(
            ShortcutChord::logical(LogicalKey::ArrowUp, KeyModifiers::CONTROL),
            ShortcutRepeatPolicy::IgnoreRepeat,
            ApplicationCommand::new(
                ApplicationCommandId::from_static("fixture.modified-arrow")
                    .unwrap_or_else(|_| unreachable!("valid shortcut command")),
                true,
            ),
        );
        shortcut_scope([shortcut, modified_arrow], [group])
    }

    fn update(replaced: &mut bool, (): ()) {
        *replaced = true;
    }
}

fn target(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!("valid fixture ID"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted fixture"))
        .id()
        .clone()
}

fn runtime(trace_capacity: usize, focused: &str) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        false,
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("mount"));
    let focused = target(&mut runtime, focused);
    runtime
        .submit_command(
            focused,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus admission"));
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("focus"));
    runtime
}

fn keyboard(logical: LogicalKey, composition: KeyboardCompositionState) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("FixtureKey")),
        logical,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        composition,
        None,
    )
}

fn settlement(batch: &InputPumpBatch) -> &UiInputSettlement {
    let mut settled = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(value) => Some(value),
            _ => None,
        });
    let first = settled
        .next()
        .unwrap_or_else(|| unreachable!("one reached input"));
    assert_eq!(settled.next(), None);
    first
}

#[test]
fn public_type_ahead_focus_default_is_claimed_before_its_exact_settlement() {
    for trace_capacity in [0, 512] {
        let mut runtime = runtime(trace_capacity, "beta");
        let destination = target(&mut runtime, "alpha");
        let before = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("snapshot"));
        let receipt = runtime
            .submit_keyboard(keyboard(
                LogicalKey::Character(String::from("a")),
                KeyboardCompositionState::Inactive,
            ))
            .unwrap_or_else(|_| unreachable!("keyboard admission"));
        let batch = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("input boundary"));
        assert_eq!(runtime.focus().focused_node(), Some(&destination));
        let settled = settlement(&batch);
        assert_eq!(settled.sequence(), receipt.sequence());
        assert_eq!(settled.scope(), before.scope());
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("committed search")
        };
        assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
        assert_eq!(facts.reasons(), [UiInputClaimReason::FocusNavigation]);
        assert_eq!(facts.default_disposition(), UiDefaultDisposition::Applied);
        assert!(
            matches!(batch.ordered_records(), [InputArbitrationRecord::OwnershipChanged(change), InputArbitrationRecord::InputSettled(input)]
            if change.before_revision() == before.revision() && change.after().revision() == input.ownership_revision())
        );
        assert_eq!(runtime.state(), &false);
        if trace_capacity == 0 {
            assert_eq!(runtime.trace().len(), 0);
        } else {
            assert!(
                runtime
                    .trace()
                    .records()
                    .any(|record| record.work_sequence() == Some(receipt.sequence())
                        && matches!(
                            record.kind(),
                            TraceRecordKind::FocusTransitionCommitted {
                                reason: runenui_core::FocusReason::GroupNavigation
                            }
                        ))
            );
        }
    }
}

#[test]
fn public_composition_suppresses_shortcut_and_generic_activation_without_claim() {
    for trace_capacity in [0, 512] {
        let mut runtime = runtime(trace_capacity, "alpha");
        let receipt = runtime
            .submit_keyboard(keyboard(
                LogicalKey::Enter,
                KeyboardCompositionState::Active,
            ))
            .unwrap_or_else(|_| unreachable!("keyboard admission"));
        let batch = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("input boundary"));
        let settled = settlement(&batch);
        assert_eq!(settled.sequence(), receipt.sequence());
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("committed activation default")
        };
        assert_eq!(facts.conflict(), UiInputConflict::ObservedNonexclusive);
        assert_eq!(facts.reasons(), []);
        assert_eq!(facts.default_disposition(), UiDefaultDisposition::None);
        assert_eq!(runtime.state(), &false);
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("no default activation"));
        assert_eq!(runtime.state(), &false);
        if trace_capacity == 0 {
            assert_eq!(runtime.trace().len(), 0);
        } else {
            assert!(runtime.trace().records().any(|record| {
                record.work_sequence() == Some(receipt.sequence())
                    && record.kind() == &TraceRecordKind::KeyboardShortcutCompositionSuppressed
            }));
            assert!(!runtime.trace().records().any(|record| {
                record.work_sequence() == Some(receipt.sequence())
                    && matches!(
                        record.kind(),
                        TraceRecordKind::KeyboardEnterActivationDerived
                    )
            }));
            assert!(
                !runtime
                    .trace()
                    .records()
                    .any(|record| record.work_sequence() == Some(receipt.sequence())
                        && matches!(record.kind(), TraceRecordKind::KeyboardShortcutMatched))
            );
        }
    }
}

#[test]
fn public_stale_displayed_pointer_target_is_not_a_missing_target_or_retarget() {
    for trace_capacity in [0, 512] {
        let mut runtime = runtime(trace_capacity, "beta");
        let environment = StyleEnvironment::default();
        let size = LogicalSize::try_new(200.0, 100.0).unwrap_or_else(|_| unreachable!("viewport"));
        let publication = runtime
            .publish_surface(&SurfaceBuildContext::tight(&environment, size))
            .unwrap_or_else(|_| unreachable!("logical publication"));
        let old = target(&mut runtime, "alpha");
        let bounds = publication
            .frame()
            .nodes()
            .iter()
            .find(|node| node.id() == &old)
            .unwrap_or_else(|| unreachable!("published target"))
            .bounds();
        let point = LogicalPoint::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )
        .unwrap_or_else(|_| unreachable!("finite hit point"));
        runtime
            .submit_action(())
            .unwrap_or_else(|_| unreachable!("replacement"));
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("reconciliation"));
        assert_ne!(target(&mut runtime, "alpha"), old);
        let before = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("snapshot"));
        let pointer = PointerId::new(1).unwrap_or_else(|| unreachable!("pointer ID"));
        let receipt = runtime
            .submit_pointer(
                PointerEvent::new(
                    pointer,
                    PointerDeviceKind::Mouse,
                    PointerPhase::Down,
                    point,
                    publication.input_context().clone(),
                )
                .with_buttons(PointerButtons::new([PointerButton::Primary]))
                .with_changed_button(PointerButton::Primary),
            )
            .unwrap_or_else(|_| unreachable!("pointer admission"));
        let batch = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("rejected input boundary"));
        let settled = settlement(&batch);
        assert_eq!(settled.sequence(), receipt.sequence());
        assert_eq!(settled.scope(), before.scope());
        assert_eq!(settled.pointer_id(), Some(pointer));
        assert_eq!(
            settled.finality(),
            &UiInputFinality::ProcessingRejected(UiInputProcessingRejection::StaleTarget)
        );
        assert_eq!(batch.final_ownership(), &before);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert_eq!(runtime.state(), &true);
        let next = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("no replay"));
        assert!(
            !next
                .ordered_records()
                .iter()
                .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_)))
        );
        if trace_capacity == 0 {
            assert_eq!(runtime.trace().len(), 0);
        } else {
            assert!(
                runtime
                    .trace()
                    .records()
                    .any(|record| record.work_sequence() == Some(receipt.sequence())
                        && matches!(
                            record.kind(),
                            TraceRecordKind::PointerIngressRejected {
                                outcome: runenui_runtime::TracePointerRejection::NoTarget,
                                ..
                            }
                        ))
            );
        }
    }
}

#[test]
fn public_unmatched_search_and_route_only_escape_never_manufacture_navigation_claims() {
    for trace_capacity in [0, 512] {
        for (logical, default) in [
            (
                LogicalKey::Character(String::from("z")),
                UiDefaultDisposition::None,
            ),
            (LogicalKey::Escape, UiDefaultDisposition::Queued),
        ] {
            let mut runtime = runtime(trace_capacity, "beta");
            let before = runtime
                .input_ownership()
                .unwrap_or_else(|_| unreachable!("snapshot"));
            let receipt = runtime
                .submit_keyboard(keyboard(logical, KeyboardCompositionState::Inactive))
                .unwrap_or_else(|_| unreachable!("keyboard admission"));
            let batch = runtime
                .pump(ONE)
                .unwrap_or_else(|_| unreachable!("input boundary"));
            let settled = settlement(&batch);
            assert_eq!(settled.sequence(), receipt.sequence());
            let UiInputFinality::Committed(facts) = settled.finality() else {
                unreachable!("committed nonclaiming route")
            };
            assert_eq!(facts.conflict(), UiInputConflict::ObservedNonexclusive);
            assert_eq!(facts.reasons(), []);
            assert_eq!(facts.default_disposition(), default);
            assert_eq!(batch.final_ownership(), &before);
            runtime
                .pump(FULL)
                .unwrap_or_else(|_| unreachable!("follow-up"));
            assert_eq!(runtime.state(), &false);
            if trace_capacity == 0 {
                assert_eq!(runtime.trace().len(), 0);
            }
        }
    }
}

#[test]
fn public_same_owner_search_claims_its_default_without_a_phantom_revision() {
    let mut runtime = runtime(0, "alpha");
    let before = runtime
        .input_ownership()
        .unwrap_or_else(|_| unreachable!("snapshot"));
    let receipt = runtime
        .submit_keyboard(keyboard(
            LogicalKey::Character(String::from("a")),
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("keyboard admission"));
    let batch = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("input boundary"));
    let settled = settlement(&batch);
    assert_eq!(settled.sequence(), receipt.sequence());
    let UiInputFinality::Committed(facts) = settled.finality() else {
        unreachable!("committed search")
    };
    assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
    assert_eq!(facts.reasons(), [UiInputClaimReason::FocusNavigation]);
    assert_eq!(facts.default_disposition(), UiDefaultDisposition::Applied);
    assert_eq!(batch.final_ownership(), &before);
    assert!(matches!(
        batch.ordered_records(),
        [InputArbitrationRecord::InputSettled(_)]
    ));
    assert_eq!(runtime.trace().len(), 0);
}

#[test]
fn approved_a1_b1_generic_navigation_and_activation_preserve_exact_nonclaim_finality() {
    for trace_capacity in [0, 1, 512] {
        for (logical, physical, modifiers, composition) in [
            (
                LogicalKey::ArrowDown,
                PhysicalKey::ArrowDown,
                KeyModifiers::NONE,
                KeyboardCompositionState::Active,
            ),
            (
                LogicalKey::Enter,
                PhysicalKey::Enter,
                KeyModifiers::NONE,
                KeyboardCompositionState::Active,
            ),
            (
                LogicalKey::Tab,
                PhysicalKey::Tab,
                KeyModifiers::NONE,
                KeyboardCompositionState::Active,
            ),
            (
                LogicalKey::ArrowDown,
                PhysicalKey::ArrowDown,
                KeyModifiers::NONE.with_control(),
                KeyboardCompositionState::Inactive,
            ),
            (
                LogicalKey::ArrowDown,
                PhysicalKey::ArrowDown,
                KeyModifiers::NONE.with_shift(),
                KeyboardCompositionState::Inactive,
            ),
            (
                LogicalKey::ArrowDown,
                PhysicalKey::ArrowDown,
                KeyModifiers::NONE.with_alt(),
                KeyboardCompositionState::Inactive,
            ),
        ] {
            let mut runtime = runtime(trace_capacity, "alpha");
            let focused = target(&mut runtime, "alpha");
            let ownership = runtime.input_ownership().unwrap_or_else(|_| unreachable!());
            let receipt = runtime
                .submit_keyboard(KeyboardEvent::new(
                    KeyboardPhase::Down,
                    physical,
                    logical,
                    modifiers,
                    false,
                    KeyLocation::Standard,
                    composition,
                    None,
                ))
                .unwrap_or_else(|_| unreachable!("A1/B1 key admitted"));
            let batch = runtime
                .pump(ONE)
                .unwrap_or_else(|_| unreachable!("one native envelope"));
            let settled = settlement(&batch);
            assert_eq!(settled.sequence(), receipt.sequence());
            assert_eq!(settled.scope(), ownership.scope());
            let UiInputFinality::Committed(facts) = settled.finality() else {
                unreachable!("approved unclaimed key committed")
            };
            assert_eq!(facts.conflict(), UiInputConflict::ObservedNonexclusive);
            assert_eq!(facts.reasons(), []);
            assert_eq!(facts.default_disposition(), UiDefaultDisposition::None);
            assert_eq!(batch.final_ownership(), &ownership);
            runtime
                .pump(FULL)
                .unwrap_or_else(|_| unreachable!("settle no defaults"));
            assert_eq!(runtime.focus().focused_node(), Some(&focused));
            assert_eq!(runtime.state(), &false);
        }
    }
}

#[test]
fn approved_b1_unmodified_direction_and_shift_tab_still_reserve_m4_navigation() {
    for trace_capacity in [0, 1, 512] {
        for (physical, logical, modifiers) in [
            (
                PhysicalKey::ArrowDown,
                LogicalKey::ArrowDown,
                KeyModifiers::NONE,
            ),
            (
                PhysicalKey::Tab,
                LogicalKey::Tab,
                KeyModifiers::NONE.with_shift(),
            ),
        ] {
            let mut runtime = runtime(trace_capacity, "alpha");
            let receipt = runtime
                .submit_keyboard(KeyboardEvent::new(
                    KeyboardPhase::Down,
                    physical,
                    logical,
                    modifiers,
                    false,
                    KeyLocation::Standard,
                    KeyboardCompositionState::Inactive,
                    None,
                ))
                .unwrap_or_else(|_| unreachable!("ordinary navigation admitted"));
            let batch = runtime
                .pump(ONE)
                .unwrap_or_else(|_| unreachable!("one native envelope"));
            let settled = settlement(&batch);
            assert_eq!(settled.sequence(), receipt.sequence());
            let UiInputFinality::Committed(facts) = settled.finality() else {
                unreachable!("ordinary M4 focus default accepted")
            };
            assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
            assert_eq!(facts.reasons(), [UiInputClaimReason::FocusNavigation]);
            assert_eq!(facts.default_disposition(), UiDefaultDisposition::Queued);
            runtime
                .pump(FULL)
                .unwrap_or_else(|_| unreachable!("focus default eventually executes"));
            assert_eq!(runtime.status(), RuntimeStatus::Running);
        }
    }
}
#[test]
fn approved_b1_explicit_modified_arrow_application_shortcut_remains_exclusive() {
    for trace_capacity in [0, 1, 512] {
        let mut runtime = runtime(trace_capacity, "alpha");
        let receipt = runtime
            .submit_keyboard(KeyboardEvent::new(
                KeyboardPhase::Down,
                PhysicalKey::ArrowUp,
                LogicalKey::ArrowUp,
                KeyModifiers::CONTROL,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            ))
            .unwrap_or_else(|_| unreachable!("modified application shortcut admitted"));
        let batch = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("exact modified-shortcut input"));
        let settled = settlement(&batch);
        assert_eq!(settled.sequence(), receipt.sequence());
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("approved application shortcut committed")
        };
        assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
        assert_eq!(facts.reasons(), [UiInputClaimReason::ApplicationShortcut]);
        assert_eq!(facts.default_disposition(), UiDefaultDisposition::Queued);
        runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("application shortcut follow-up"));
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        if trace_capacity == 512 {
            assert!(runtime.trace().records().any(|record| {
                record.work_sequence() == Some(receipt.sequence())
                    && matches!(record.kind(), TraceRecordKind::KeyboardShortcutMatched)
            }));
        }
    }
}
