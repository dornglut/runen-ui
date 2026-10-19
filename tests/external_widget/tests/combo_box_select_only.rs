#![allow(refining_impl_trait)]
//! Select-only ComboBox uses one focus owner and app-selected Option callbacks.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, LogicalPoint, NoHostProtocol, PointerButton,
    PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SceneShape,
    SemanticAction,
    SemanticCommand, SemanticRole, SemanticValue, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, MountedNodeId, PumpBudget,
    SurfaceBuildContext,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action { Request(ComboBoxRequest), Choose(u8) }
#[derive(Default)]
struct State { open: bool, chosen: u8, log: Vec<ComboBoxRequest> }
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        combo_box("Choose a value",
            ElementId::new("choice.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("One", state.chosen == 1)
                    .id("choice.one").on_activate(|| Action::Choose(1)),
                option_item("Two", state.chosen == 2)
                    .id("choice.two").on_activate(|| Action::Choose(2)),
            ],
            Action::Request,
        )
        .id("choice.owner")
        .open(state.open)
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Request(request) => {
                match &request {
                    ComboBoxRequest::Open => state.open = true,
                    ComboBoxRequest::Close => state.open = false,
                }
                state.log.push(request);
            }
            Action::Choose(value) => {
                state.chosen = value;
                state.open = false;
            }
        }
    }
}
fn settle(app: &mut AppRuntime<App>) {
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent());
}
fn id(app: &mut AppRuntime<App>, id: &str) -> MountedNodeId {
    let id = ElementId::new(id).unwrap_or_else(|_| unreachable!("valid test ID"));
    app.index().nodes().iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("authored control mounted"))
        .id().clone()
}
fn command(app: &mut AppRuntime<App>, target: MountedNodeId, cmd: SemanticCommand) {
    app.submit_command(target, cmd, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("semantic action admitted"));
    settle(app);
}
#[test]
fn select_only_keeps_real_focus_on_owner_while_selection_remains_application_owned() {
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&mut app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert!(app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open]);
    assert_eq!(app.focus().focused_node(), Some(&owner));
    let one = id(&mut app, "choice.one");
    let two = id(&mut app, "choice.two");
    assert!(!app.index().node(&one).is_some_and(|node| node.is_focusable()));
    assert!(!app.index().node(&two).is_some_and(|node| node.is_focusable()));
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("ComboBox semantic publication: {e:?}"));
    let semantics = published.semantic_publication().snapshot();
    assert!(semantics.nodes().iter().any(|node| node.role() == SemanticRole::ComboBox));
    assert_eq!(semantics.nodes().iter().filter(|n| n.role() == SemanticRole::Option).count(), 2);
    assert!(semantics.nodes().iter().all(|node|
        node.role() != SemanticRole::Option
            || !node.supported_actions().contains(&SemanticAction::RequestFocus)
    ));
    let combo = semantics.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox semantic owner"));
    let first = semantics.nodes().iter()
        .find(|node| node.role() == SemanticRole::Option && node.name() == Some("One"))
        .unwrap_or_else(|| unreachable!("first Option"));
    assert_eq!(combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .map(|link| link.target()), Some(first.id()));
    command(&mut app, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(app.state().chosen, 0, "active descendant is not committed value");
    let updated = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("active semantic publication: {error:?}"));
    let snapshot = updated.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("current ComboBox semantic"));
    let second = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::Option && node.name() == Some("Two"))
        .unwrap_or_else(|| unreachable!("second Option"));
    assert_eq!(combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .map(|link| link.target()), Some(second.id()),
        "transient active Option changes while the app value remains uncommitted");
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert_eq!(app.state().chosen, 2);
    assert!(!app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open],
        "keyboard and pointer activation use the same ordinary Option callback");
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn controls_identifies_actual_listbox_and_active_option_without_focus_repair() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        chosen: 1,
        log: Vec::new(),
    });
    settle(&mut app);
    assert!(app.focus().focused_node().is_none());
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("current popup semantics: {error:?}"));
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("focused-or-not ComboBox semantics"));
    let controlled = combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("ComboBox Controls relation"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == controlled.target()).map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::ListBox),
        "M5 requires the actual ListBox, not a surrounding Popover Group");
    let active = combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("active proxy resolves without synthetic focus"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == active.target()).and_then(|node| node.name()),
        Some("One"));
}

fn key(physical: runenui_core::PhysicalKey, logical: runenui_core::LogicalKey,
    composition: runenui_core::KeyboardCompositionState) -> runenui_core::KeyboardEvent {
    runenui_core::KeyboardEvent::new(
        runenui_core::KeyboardPhase::Down, physical, logical,
        runenui_core::KeyModifiers::NONE, false,
        runenui_core::KeyLocation::Standard, composition, None,
    )
}
fn active_label(app: &mut AppRuntime<App>) -> Option<String> {
    let publication = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("current ComboBox semantics: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let owner = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)?;
    let target = owner.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)?
        .target();
    snapshot.nodes().iter().find(|node| node.id() == target)
        .and_then(|node| node.name()).map(str::to_owned)
}
#[test]
fn keyboard_ime_escape_and_enter_keep_focus_and_committed_value_separate() {
    use runenui_core::{KeyboardCompositionState as Composition, LogicalKey, PhysicalKey};
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&mut app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("arrow ingress"));
    settle(&mut app);
    assert!(app.state().open);
    assert_eq!(active_label(&mut app).as_deref(), Some("One"));
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Active)).unwrap_or_else(|_| unreachable!("composition ingress"));
    settle(&mut app);
    assert_eq!(active_label(&mut app).as_deref(), Some("One"));
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("navigation ingress"));
    settle(&mut app);
    assert_eq!(active_label(&mut app).as_deref(), Some("Two"));
    assert_eq!(app.state().chosen, 0, "navigation is not committed selection");
    app.submit_keyboard(key(PhysicalKey::Escape, LogicalKey::Escape,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("Escape ingress"));
    settle(&mut app);
    assert!(!app.state().open);
    assert_eq!(app.state().chosen, 0, "Escape never commits the active option");
    assert_eq!(app.focus().focused_node(), Some(&owner));

    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("reopen ingress"));
    settle(&mut app);
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("second selection ingress"));
    settle(&mut app);
    app.submit_keyboard(key(PhysicalKey::Enter, LogicalKey::Enter,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("accept ingress"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 2,
        "Enter must activate the existing Option callback, never copy the active authored ID");
    assert!(!app.state().open);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn semantic_expand_and_collapse_actions_follow_application_open_state() {
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&mut app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("closed ComboBox semantics: {e:?}"));
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox role"));
    assert!(combo.supported_actions().contains(&SemanticAction::Expand));
    assert!(!combo.supported_actions().contains(&SemanticAction::Collapse));
    command(&mut app, owner.clone(), SemanticCommand::Expand);
    assert!(app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open]);
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("expanded ComboBox semantics: {e:?}"));
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox role"));
    assert!(combo.supported_actions().contains(&SemanticAction::Collapse));
    command(&mut app, owner.clone(), SemanticCommand::Collapse);
    assert!(!app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open, ComboBoxRequest::Close]);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn collapsed_display_glyphs(chosen: u8) -> Vec<u32> {
    let mut app = AppRuntime::<App>::mount(State {
        chosen,
        ..State::default()
    });
    app.register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!("checked font fixture"));
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    )
    .unwrap_or_else(|_| unreachable!("checked font family"));
    settle(&mut app);
    let publication = app
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(500.0, 240.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("closed combo paint: {error:?}"));
    publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .flat_map(|run| {
            publication
                .paint_scene()
                .shaped_text_resource(run.resource_ref())
                .into_iter()
                .flat_map(|resource| resource.glyphs().iter().map(|glyph| glyph.id()))
        })
        .collect()
}

#[test]
fn closed_select_only_displays_app_selected_option_using_retained_m8_text_not_static_name() {
    let unselected = collapsed_display_glyphs(0);
    let one = collapsed_display_glyphs(1);
    let two = collapsed_display_glyphs(2);
    assert!(!unselected.is_empty());
    assert!(unselected.len() > one.len(),
        "the unselected fallback prompt must not replace the chosen option");
    assert_eq!(one.len(), two.len(),
        "both selected options use their own ordinary three-letter label");
    assert_ne!(one, two,
        "changing only app-authored selected Option changes retained text glyphs");
}

// Physical pointer tests use projected displayed hit geometry, not retained
// layout/debug frame bounds. The target must resolve to this exact mounted Option.
fn displayed_option_center(
    publication: &runenui_runtime::SurfacePublication,
    option: &MountedNodeId,
) -> LogicalPoint {
    let scene = publication.hit_test_scene();
    let region = scene.regions().iter()
        .find(|region| region.target() == option && matches!(region.shape(), SceneShape::Rect(_)))
        .unwrap_or_else(|| unreachable!("current Option has a rectangular projected hit region"));
    let SceneShape::Rect(bounds) = region.shape() else {
        unreachable!("selected a projected rectangle")
    };
    assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
    let local = LogicalPoint::new(
        bounds.width().mul_add(0.5, bounds.x()),
        bounds.height().mul_add(0.5, bounds.y()),
    ).unwrap_or_else(|_| unreachable!("finite current Option region center"));
    let point = region.local_to_surface().transform_point(local)
        .unwrap_or_else(|| unreachable!("non-singular current Option projection"));
    assert_eq!(scene.target_at(point), Some(option),
        "the correlated displayed hit scene must target the exact Option");
    point
}

#[test]
fn physical_option_click_uses_standard_option_activation_without_moving_owner_focus() {
    let mut app = AppRuntime::<App>::mount(State::default());
    app.register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!("known test font"));
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    ).unwrap_or_else(|_| unreachable!("known test font family"));
    settle(&mut app);
    let owner = id(&mut app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert!(app.state().open);

    let publish = |app: &mut AppRuntime<App>| app
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("current popup published: {error:?}"));

    let option = id(&mut app, "choice.two");
    let frame = publish(&mut app);
    let point = displayed_option_center(&frame, &option);
    let pointer = PointerId::new(331).unwrap_or_else(|| unreachable!("nonzero pointer id"));

    app.submit_pointer(PointerEvent::new(
        pointer,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        frame.input_context().clone(),
    )).unwrap_or_else(|_| unreachable!("current hover admitted"));
    settle(&mut app);
    let hover = publish(&mut app);
    let down_point = displayed_option_center(&hover, &option);
    app.submit_pointer(
        PointerEvent::new(
            pointer, PointerDeviceKind::Mouse, PointerPhase::Down,
            down_point, hover.input_context().clone(),
        )
        .with_buttons(PointerButtons::new([PointerButton::Primary]))
        .with_changed_button(PointerButton::Primary),
    ).unwrap_or_else(|_| unreachable!("current primary down admitted"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 0, "Down must not commit selection");
    let down = publish(&mut app);
    let up_point = displayed_option_center(&down, &option);
    app.submit_pointer(
        PointerEvent::new(
            pointer, PointerDeviceKind::Mouse, PointerPhase::Up,
            up_point, down.input_context().clone(),
        )
        .with_changed_button(PointerButton::Primary),
    ).unwrap_or_else(|_| unreachable!("matching primary Up admitted"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 2, "normal Option activation owns physical click");
    assert!(!app.state().open, "application closes popup after choosing");
    assert_eq!(app.focus().focused_node(), Some(&owner),
        "nonfocusable proxy never moves the M4 real focus owner");
}

#[test]
fn closed_combo_exposes_app_selected_option_as_neutral_semantic_text_value() {
    for (selected, expected) in [(0, None), (1, Some("One")), (2, Some("Two"))] {
        let mut app = AppRuntime::<App>::mount(State {
            chosen: selected,
            ..State::default()
        });
        settle(&mut app);
        let publication = app.publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(500.0, 240.0).unwrap_or_else(|_| unreachable!()),
        )).unwrap_or_else(|error| unreachable!("closed semantic projection: {error:?}"));
        let snapshot = publication.semantic_publication().snapshot();
        let combo = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)
            .unwrap_or_else(|| unreachable!("one ComboBox semantic owner"));
        assert!(snapshot.nodes().iter().all(|node| node.role() != SemanticRole::Option),
            "closed popup never retains semantic Option descendants");
        let observed = match combo.value() {
            Some(SemanticValue::Text(value)) => Some(value.as_str()),
            None => None,
            Some(_) => unreachable!("ComboBox selected value is textual"),
        };
        assert_eq!(observed, expected, "M5 projection follows the application-selected Option");
    }
}

fn native_combo_arrow(
    phase: runenui_core::KeyboardPhase,
    modifiers: runenui_core::KeyModifiers,
    composition: runenui_core::KeyboardCompositionState,
) -> runenui_core::KeyboardEvent {
    use runenui_core::{KeyLocation, KeyboardEvent, LogicalKey, PhysicalKey};
    KeyboardEvent::new(
        phase,
        PhysicalKey::ArrowDown,
        LogicalKey::ArrowDown,
        modifiers,
        false,
        KeyLocation::Standard,
        composition,
        None,
    )
}

#[test]
fn select_only_combo_claims_only_its_recognized_native_arrow_key() {
    use runenui_core::{KeyModifiers, KeyboardCompositionState, KeyboardPhase};
    use runenui_runtime::{
        InputArbitrationRecord, RuntimeConfig, TraceConfig, UiDefaultDisposition,
        UiInputClaimReason, UiInputConflict, UiInputFinality,
    };

    for trace_capacity in [0, 1, 512] {
        for (phase, modifiers, composition, recognized) in [
            (KeyboardPhase::Down, KeyModifiers::NONE, KeyboardCompositionState::Inactive, true),
            (KeyboardPhase::Down, KeyModifiers::CONTROL, KeyboardCompositionState::Inactive, false),
            (KeyboardPhase::Up, KeyModifiers::NONE, KeyboardCompositionState::Inactive, false),
            (KeyboardPhase::Down, KeyModifiers::NONE, KeyboardCompositionState::Active, false),
        ] {
            let mut app = AppRuntime::<App>::mount_with_config(
                State { open: true, chosen: 0, log: Vec::new() },
                RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
            );
            settle(&mut app);
            let owner = id(&mut app, "choice.owner");
            command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
            let scope = app.input_ownership()
                .unwrap_or_else(|_| unreachable!("live input scope"))
                .scope()
                .clone();
            let receipt = app.submit_keyboard(native_combo_arrow(phase, modifiers, composition))
                .unwrap_or_else(|_| unreachable!("native ComboBox arrow admitted"));
            let batch = app.pump(PumpBudget::new(1, 0, 0, 0))
                .unwrap_or_else(|error| unreachable!("observed input: {error:?}"));
            let [InputArbitrationRecord::InputSettled(settled)] = batch.ordered_records() else {
                unreachable!("exactly one reached native keyboard event")
            };
            assert_eq!(settled.sequence(), receipt.sequence());
            assert_eq!(settled.scope(), &scope);
            let UiInputFinality::Committed(facts) = settled.finality() else {
                unreachable!("nonterminal native event commits")
            };
            assert_eq!(facts.conflict(), if recognized {
                UiInputConflict::ExclusiveUi
            } else {
                UiInputConflict::ObservedNonexclusive
            });
            assert_eq!(facts.reasons(), if recognized {
                &[UiInputClaimReason::ExplicitWidgetClaim][..]
            } else {
                &[]
            });
            assert_eq!(facts.default_disposition(), if recognized {
                UiDefaultDisposition::Prevented
            } else {
                UiDefaultDisposition::None
            });
            assert_eq!(app.state().chosen, 0);
            assert_eq!(app.focus().focused_node(), Some(&owner));
            assert_eq!(app.state().log, []);
            if trace_capacity == 0 {
                assert_eq!(app.trace().len(), 0);
            }
        }
    }
}
