#![allow(clippy::panic)]

use runenui_core::{
    CommandOrigin, CommittedTextEvent, EditIntent, EditResolution, Effects, IntoUpdateOutput,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
    LayoutDimension, LayoutStyle, LogicalKey, LogicalLength, LogicalPoint, NoHostProtocol,
    PhysicalKey, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, SemanticCommand, SemanticEditableMode, StyleEnvironment, TextAffinity,
    TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp,
    UpdateOutput, View,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget, SurfaceBuildContext,
};

#[derive(Clone)]
struct FormState {
    text: String,
    revision: u64,
    selection: usize,
    mode: SemanticEditableMode,
    placeholder: String,
    submits: usize,
}

enum Action {
    Edit(Box<EditIntent>),
    SetValue(String),
    SetPlaceholder(String),
    Submit,
}

struct FormApp;

impl UiApp for FormApp {
    type State = FormState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot = snapshot(state.revision);
        let selection = TextPosition::new(
            snapshot,
            &state.text,
            state.selection,
            if state.selection == state.text.len() && !state.text.is_empty() {
                TextAffinity::Upstream
            } else {
                TextAffinity::Downstream
            },
        )
        .unwrap_or_else(|_| unreachable!("application selection remains valid"));
        runenui_core::text_field(
            snapshot,
            state.text.clone(),
            TextSelection::collapsed(selection),
            state.mode,
            |intent| Action::Edit(Box::new(intent)),
        )
        .unwrap_or_else(|_| unreachable!("application field source remains valid"))
        .id("form.field")
        .with_layout(
            LayoutStyle::default().with_width(LayoutDimension::length(LogicalLength::from(48_u16))),
        )
        .placeholder(state.placeholder.clone())
        .on_submit(|| Action::Submit)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Edit(intent) => {
                let span = intent.replacement();
                state
                    .text
                    .replace_range(span.start()..span.end(), intent.replacement_text());
                state.selection = intent.proposed_selection().active();
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(),
                    snapshot(state.revision),
                ))
            }
            Action::SetValue(value) => {
                state.text = value;
                state.selection = state.text.len();
                state.revision += 1;
                UpdateOutput::effects(Effects::none())
            }
            Action::SetPlaceholder(value) => {
                state.placeholder = value;
                UpdateOutput::effects(Effects::none())
            }
            Action::Submit => {
                state.submits += 1;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

const fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(24), TextDocumentRevision::new(revision))
}

fn app(mode: SemanticEditableMode) -> AppRuntime<FormApp> {
    app_with_source(mode, "ab")
}

fn app_with_source(mode: SemanticEditableMode, source: &str) -> AppRuntime<FormApp> {
    AppRuntime::mount(FormState {
        text: source.to_owned(),
        revision: 0,
        selection: source.len(),
        mode,
        placeholder: "Type here".to_owned(),
        submits: 0,
    })
}

fn focus(runtime: &mut AppRuntime<FormApp>) {
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("field is focusable"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
}

fn commit(runtime: &mut AppRuntime<FormApp>, text: &str) {
    runtime
        .submit_text(
            CommittedTextEvent::new(text, None)
                .unwrap_or_else(|_| unreachable!("fixture committed text is nonempty")),
        )
        .unwrap_or_else(|_| unreachable!("active field admits text"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn public_single_line_edit_normalizes_m10_text_and_enter_submits_without_mutation() {
    let mut runtime = app(SemanticEditableMode::SingleLine);
    focus(&mut runtime);
    commit(&mut runtime, "X\r\nY\nZ\rQ");
    assert_eq!(runtime.state().text, "abX Y Z Q");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().selection, runtime.state().text.len());

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code("Enter".to_owned()),
            LogicalKey::Enter,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Enter is routed"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().submits, 1);
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().text, "abX Y Z Q");
}

#[test]
fn public_multiline_field_preserves_committed_newlines_through_m10() {
    let mut runtime = app(SemanticEditableMode::Multiline);
    focus(&mut runtime);
    commit(&mut runtime, "X\nY");
    assert_eq!(runtime.state().text, "abX\nY");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().submits, 0);
}

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn register_font(runtime: &mut AppRuntime<FormApp>) {
    assert!(
        runtime
            .register_text_font_bytes(FONT.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled font registers"))
            > 0
    );
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell")
                    .unwrap_or_else(|_| unreachable!("font family validates"))],
            )
            .unwrap_or_else(|_| unreachable!("controlled font mapping registers"))
    );
}

fn publication(runtime: &mut AppRuntime<FormApp>) -> runenui_runtime::SurfacePublication {
    let env = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &env,
            LogicalSize::try_new(48.0, 180.0)
                .unwrap_or_else(|_| unreachable!("surface bounds are finite")),
        ))
        .unwrap_or_else(|_| unreachable!("public TextField surface publishes"))
}

#[test]
fn controlled_font_publication_keeps_single_line_unwrapped_and_multiline_wrapping() {
    let source = "A sufficiently long line of ordinary text to require soft wrapping";
    let mut single = app_with_source(SemanticEditableMode::SingleLine, source);
    let mut multi = app_with_source(SemanticEditableMode::Multiline, source);
    for runtime in [&mut single, &mut multi] {
        register_font(runtime);
    }
    let single_surface = publication(&mut single);
    let multi_surface = publication(&mut multi);
    let lines = |surface: &runenui_runtime::SurfacePublication| {
        let retained = surface
            .layout_report()
            .root()
            .unwrap_or_else(|| unreachable!("retained layout is published"))
            .text_measurements()
            .iter()
            .filter(|measurement| measurement.retained_for_paint())
            .collect::<Vec<_>>();
        assert_eq!(retained.len(), 1);
        let runs = surface
            .paint_scene()
            .items()
            .iter()
            .filter_map(|item| item.primitive().as_shaped_text_run())
            .collect::<Vec<_>>();
        let refs = runs
            .iter()
            .map(|run| run.resource_ref().clone())
            .collect::<Vec<_>>();
        assert_eq!(retained[0].retained_resource_refs(), refs.as_slice());
        for run in &runs {
            assert!(
                surface
                    .paint_scene()
                    .shaped_text_resource(run.resource_ref())
                    .is_some()
            );
        }
        let semantic = &surface.semantic_publication().snapshot().nodes()[0];
        assert_eq!(
            semantic.editable().and_then(|edit| edit.value()),
            Some(source)
        );
        runs.len()
    };
    assert_eq!(
        lines(&single_surface),
        1,
        "single-line source must not soft-wrap"
    );
    assert!(
        lines(&multi_surface) > 1,
        "multiline source should soft-wrap"
    );

    single
        .submit_action(Action::SetValue("changed by application".to_owned()))
        .unwrap_or_else(|_| unreachable!("ordinary application rebuild is accepted"));
    single.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let rebuilt = publication(&mut single);
    assert_eq!(
        rebuilt.semantic_publication().snapshot().nodes()[0]
            .editable()
            .and_then(|edit| edit.value()),
        Some("changed by application"),
    );
    assert!(
        rebuilt
            .paint_scene()
            .items()
            .iter()
            .any(|item| item.primitive().as_shaped_text_run().is_some())
    );
}

#[test]
fn controlled_font_pointer_hit_and_selection_share_m10_caret_geometry() {
    let mut runtime = app(SemanticEditableMode::SingleLine);
    register_font(&mut runtime);
    let initial = publication(&mut runtime);
    let owner = runtime.index().nodes()[0].id().clone();
    let point = LogicalPoint::new(2.0, 2.0)
        .unwrap_or_else(|_| unreachable!("pointer coordinates are finite"));
    assert_eq!(initial.hit_test_scene().target_at(point), Some(&owner));
    let selected_before = initial.semantic_publication().snapshot().nodes()[0]
        .editable()
        .unwrap_or_else(|| unreachable!("field publishes M10 selection"))
        .selection();
    assert_eq!(selected_before.active().byte_offset(), 2);

    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("field is focusable"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let focused = publication(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    assert!(
        focused.paint_scene().items().iter().any(|item| {
            matches!(
                item.primitive(),
                runenui_core::PaintPrimitive::Fill {
                    shape: runenui_core::SceneShape::Rect(rect),
                    ..
                } if rect.width().to_bits() == 1.0_f32.to_bits()
            )
        }),
        "focused field paints the authoritative M10 caret"
    );

    let pointer = PointerId::new(5).unwrap_or_else(|| unreachable!("pointer id is valid"));
    let context = focused.input_context().clone();
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                point,
                context.clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("pointer press is routed"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                point,
                context,
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!("pointer release is routed"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let clicked = publication(&mut runtime);
    assert_eq!(clicked.hit_test_scene().target_at(point), Some(&owner));
    let selection = clicked.semantic_publication().snapshot().nodes()[0]
        .editable()
        .unwrap_or_else(|| unreachable!("clicked field retains M10 selection"))
        .selection();
    assert!(
        selection.active().byte_offset() < selected_before.active().byte_offset(),
        "pointer click near the start must place the authoritative caret before the end"
    );
}

#[test]
fn placeholder_is_passive_m8_paint_not_editable_source_intrinsic_or_hit_authority() {
    let mut runtime = app_with_source(SemanticEditableMode::SingleLine, "");
    register_font(&mut runtime);
    let owner = runtime.index().nodes()[0].id().clone();
    let first = publication(&mut runtime);
    let field = &first.semantic_publication().snapshot().nodes()[0];
    assert_eq!(field.placeholder(), Some("Type here"));
    assert_eq!(
        field.editable().and_then(|editable| editable.value()),
        Some("")
    );
    assert_eq!(runtime.state().revision, 0);
    let original_size = first
        .layout_report()
        .root()
        .unwrap_or_else(|| unreachable!("field layout is retained"))
        .desired_content_size();
    let first_hint = first
        .paint_scene()
        .items()
        .iter()
        .find_map(|item| {
            item.primitive().as_shaped_text_run().map(|run| {
                assert_eq!(item.opacity().get(), 0.5);
                run.resource_ref().clone()
            })
        })
        .unwrap_or_else(|| unreachable!("empty source renders a visual-only shaped hint"));
    assert!(
        first
            .paint_scene()
            .shaped_text_resource(&first_hint)
            .is_some()
    );
    let point = LogicalPoint::new(2.0, 2.0).unwrap_or_else(|_| unreachable!("point is finite"));
    assert_eq!(first.hit_test_scene().target_at(point), Some(&owner));

    runtime
        .submit_action(Action::SetPlaceholder(
            "A much longer visual hint".to_owned(),
        ))
        .unwrap_or_else(|_| unreachable!("placeholder is application-authored"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let relabeled = publication(&mut runtime);
    assert_eq!(runtime.index().nodes()[0].id(), &owner);
    assert_eq!(
        relabeled
            .layout_report()
            .root()
            .unwrap_or_else(|| unreachable!("field layout is retained"))
            .desired_content_size(),
        original_size
    );
    assert_eq!(
        relabeled.semantic_publication().snapshot().nodes()[0]
            .editable()
            .and_then(|editable| editable.value()),
        Some("")
    );
    assert!(relabeled.paint_scene().items().iter().any(|item| {
        item.primitive().as_shaped_text_run().is_some()
            && item.opacity()
                == runenui_core::SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("half-opacity is valid"))
    }));

    focus(&mut runtime);
    commit(&mut runtime, "Q");
    let filled = publication(&mut runtime);
    assert_eq!(runtime.state().text, "Q");
    // Accepted publications remain immutable; the new source publication must
    // not lease or expose the retired placeholder's shaped resource.
    assert!(
        first
            .paint_scene()
            .shaped_text_resource(&first_hint)
            .is_some()
    );
    assert!(
        filled
            .paint_scene()
            .shaped_text_resource(&first_hint)
            .is_none()
    );
    assert!(filled.paint_scene().items().iter().all(|item| {
        item.primitive().as_shaped_text_run().is_none()
            || item.opacity() == runenui_core::SceneOpacity::OPAQUE
    }));
    assert_eq!(
        filled.semantic_publication().snapshot().nodes()[0].placeholder(),
        Some("A much longer visual hint")
    );

    runtime
        .submit_action(Action::SetValue(String::new()))
        .unwrap_or_else(|_| unreachable!("application can clear durable text"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let cleared = publication(&mut runtime);
    assert_eq!(runtime.state().text, "");
    assert!(cleared.paint_scene().items().iter().any(|item| {
        item.primitive().as_shaped_text_run().is_some()
            && item.opacity()
                == runenui_core::SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("half-opacity is valid"))
    }));
}

#[test]
fn active_ime_preedit_suppresses_placeholder_without_substituting_document_source() {
    let mut runtime = app_with_source(SemanticEditableMode::SingleLine, "");
    register_font(&mut runtime);
    focus(&mut runtime);
    let idle = publication(&mut runtime);
    assert!(idle.paint_scene().items().iter().any(|item| {
        item.primitive().as_shaped_text_run().is_some()
            && item.opacity()
                == runenui_core::SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("half-opacity is valid"))
    }));
    let generation = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition begins"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_composition_update(generation.generation().clone(), "é".to_owned(), None)
        .unwrap_or_else(|_| unreachable!("preedit is admitted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let composing = publication(&mut runtime);
    assert!(
        composing.paint_scene().items().iter().all(|item| {
            item.primitive().as_shaped_text_run().is_none()
                || item.opacity() == runenui_core::SceneOpacity::OPAQUE
        }),
        "preedit owns visual text while composition is active"
    );
    assert_eq!(runtime.state().text, "");
    let composing_semantics = &composing.semantic_publication().snapshot().nodes()[0];
    // M10 deliberately fails closed: its retained display artifact belongs to
    // the transient preedit, not the exact application-owned source revision.
    // Placeholder metadata must not be substituted for either missing fact.
    assert!(composing_semantics.editable().is_none());
    assert!(composing_semantics.value().is_none());
    assert_eq!(composing_semantics.placeholder(), Some("Type here"));
    runtime
        .cancel_composition(generation.generation().clone())
        .unwrap_or_else(|_| unreachable!("composition cancels"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let restored = publication(&mut runtime);
    assert_eq!(
        restored.semantic_publication().snapshot().nodes()[0]
            .editable()
            .and_then(|editable| editable.value()),
        Some("")
    );
    assert!(restored.paint_scene().items().iter().any(|item| {
        item.primitive().as_shaped_text_run().is_some()
            && item.opacity()
                == runenui_core::SceneOpacity::new(0.5)
                    .unwrap_or_else(|_| unreachable!("half-opacity is valid"))
    }));
}
