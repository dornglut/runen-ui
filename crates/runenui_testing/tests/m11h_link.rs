#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;

use runenui_core::{
    ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint, LogicalKey, NoHostProtocol,
    PhysicalKey, PointerButton, PointerButtons, PointerDeviceKind, PointerId, PointerPhase,
    SemanticAction, SemanticCommand, SemanticRole, UiApp, View, link,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Debug)]
struct Model {
    enabled: bool,
    navigations: usize,
}

struct LinkApp;

impl UiApp for LinkApp {
    type State = Model;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        link("Reference")
            .id("reference.link")
            .key("reference.link")
            .enabled(state.enabled)
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(LogicalLength::from(140_u16)))
                    .with_height(LayoutDimension::length(LogicalLength::from(28_u16))),
            )
            .on_activate(|| ())
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        state.navigations += 1;
    }
}

fn settle(harness: &mut TestHarness<LinkApp>) {
    let budget = SettleBudget::new(
        NonZeroUsize::new(16).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    );
    assert_eq!(harness.run_until_idle(budget).outcome(), SettleOutcome::Idle);
}

fn mounted(enabled: bool) -> TestHarness<LinkApp> {
    let mut h = TestHarness::<LinkApp>::mount(Model {
        enabled,
        navigations: 0,
    });
    assert!(h.publish().is_ok());
    h
}

fn query() -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Link)
        .with_name("Reference")
}

fn id() -> ElementId {
    ElementId::new("reference.link").unwrap_or_else(|_| unreachable!("authored id is valid"))
}

#[test]
fn semantic_link_action_uses_application_action_and_never_button_role() {
    let mut h = mounted(true);
    let target = h
        .unique_semantic_target(&query())
        .unwrap_or_else(|_| unreachable!("one exact Link semantic target"));
    assert!(
        h.submit_semantic_action(&target, SemanticAction::Activate)
            .is_ok()
    );
    settle(&mut h);
    assert_eq!(h.state().navigations, 1);
    assert!(h.publish().is_ok());
    assert_eq!(h.query_semantics(&query()).unwrap_or_else(|_| unreachable!("snapshot exists")).len(), 1);
    assert_eq!(
        h.query_semantics(&SemanticQuery::new().with_role(SemanticRole::Button))
            .unwrap_or_else(|_| unreachable!("snapshot exists"))
            .len(),
        0,
    );
}

#[test]
fn focused_enter_and_pointer_primary_activation_converge_on_link() {
    let mut keyboard = mounted(true);
    keyboard
        .submit_automation_command(id(), SemanticCommand::RequestFocus)
        .unwrap_or_else(|_| unreachable!("link is focusable"));
    settle(&mut keyboard);
    keyboard
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Enter reaches focused Link"));
    settle(&mut keyboard);
    assert_eq!(keyboard.state().navigations, 1);

    let mut pointer = mounted(true);
    let publication = pointer
        .publication()
        .unwrap_or_else(|| unreachable!("Link is published"));
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id()))
        .unwrap_or_else(|| unreachable!("published link has authored identity"));
    let bounds = node.bounds();
    let point = LogicalPoint::new(bounds.x() + 2.0, bounds.y() + 2.0)
        .unwrap_or_else(|_| unreachable!("bounds are finite"));
    assert_eq!(publication.hit_test_scene().target_at(point), Some(node.id()));
    let pointer_id = PointerId::new(71).unwrap_or_else(|| unreachable!("pointer is nonzero"));
    let down = pointer
        .pointer_event(pointer_id, PointerDeviceKind::Mouse, PointerPhase::Down, point)
        .unwrap_or_else(|_| unreachable!("published pointer context exists"))
        .with_buttons(PointerButtons::new([PointerButton::Primary]))
        .with_changed_button(PointerButton::Primary);
    pointer
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("pointer down admits"));
    settle(&mut pointer);
    let up = pointer
        .pointer_event(pointer_id, PointerDeviceKind::Mouse, PointerPhase::Up, point)
        .unwrap_or_else(|_| unreachable!("published pointer context exists"))
        .with_changed_button(PointerButton::Primary);
    pointer
        .submit_pointer(up)
        .unwrap_or_else(|_| unreachable!("pointer up admits"));
    settle(&mut pointer);
    assert_eq!(pointer.state().navigations, 1);
}

#[test]
fn disabled_link_remains_semantic_but_refuses_activation() {
    let mut h = mounted(false);
    let query = query().with_disabled(true);
    let target = h
        .unique_semantic_target(&query)
        .unwrap_or_else(|_| unreachable!("disabled link retains semantic Link role"));
    assert!(
        h.submit_semantic_action(&target, SemanticAction::Activate)
            .is_err()
    );
    settle(&mut h);
    assert_eq!(h.state().navigations, 0);
}
