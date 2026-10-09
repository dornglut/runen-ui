//! Standard password TextField exercising the accepted M10/M8 secret authority.

use runenui_core::{
    CommandOrigin, CommittedTextEvent, EditIntent, EditResolution, Effects, IntoUpdateOutput,
    NoHostProtocol, SemanticCommand, SemanticEditableMode, StyleEnvironment, TextAffinity,
    TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection,
    TextSensitivity, UiApp, UpdateOutput, View,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget,
    SurfaceBuildContext, SurfacePublication,
};
use runenui_text::{
    FontSourcePolicy, TextConstraints, TextLayoutState, TextMaskedProjection, TextRequest,
    TextSystem,
};
use runenui_core::Typography;

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");
const SECRET: &str = "Secrét漢🌐";

#[derive(Clone)]
struct PasswordState {
    text: String,
    revision: u64,
    selection: usize,
    secret: bool,
}

enum Action {
    Edit(Box<EditIntent>),
    ToggleSecret,
}

struct PasswordApp;

fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(940), TextDocumentRevision::new(revision))
}

impl UiApp for PasswordApp {
    type State = PasswordState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snap = snapshot(state.revision);
        let pos = TextPosition::new(
            snap,
            &state.text,
            state.selection,
            if state.selection == state.text.len() {
                TextAffinity::Upstream
            } else {
                TextAffinity::Downstream
            },
        ).unwrap_or_else(|_| unreachable!("app selection is checked"));
        let field = runenui_core::text_field(
            snap,
            state.text.clone(),
            TextSelection::collapsed(pos),
            SemanticEditableMode::SingleLine,
            |edit| Action::Edit(Box::new(edit)),
        ).unwrap_or_else(|_| unreachable!("app text is checked"));
        let field = if state.secret {
            field.password().unwrap_or_else(|_| unreachable!("single-line password"))
        } else {
            field
        };
        field.id("form.password").placeholder("Password")
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Edit(intent) => {
                state.text.replace_range(
                    intent.replacement().start()..intent.replacement().end(),
                    intent.replacement_text(),
                );
                state.selection = intent.proposed_selection().active();
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(),
                    snapshot(state.revision),
                ))
            }
            Action::ToggleSecret => {
                state.secret = !state.secret;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

fn mounted() -> AppRuntime<PasswordApp> {
    let mut app = AppRuntime::<PasswordApp>::mount(PasswordState {
        text: SECRET.to_owned(),
        revision: 0,
        selection: SECRET.len(),
        secret: true,
    });
    assert!(app.register_text_font_bytes(FONT.to_vec()).is_ok());
    assert!(app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")
            .unwrap_or_else(|_| unreachable!("font is named"))],
    ).is_ok());
    app
}

fn publication(runtime: &mut AppRuntime<PasswordApp>) -> SurfacePublication {
    let env = StyleEnvironment::default();
    runtime.publish_surface(&SurfaceBuildContext::tight(
        &env,
        LogicalSize::try_new(280.0, 60.0)
            .unwrap_or_else(|_| unreachable!("geometry is valid")),
    )).unwrap_or_else(|_| unreachable!("password surface publishes"))
}

fn glyphs(surface: &SurfacePublication) -> Vec<u32> {
    surface.paint_scene().items().iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .flat_map(|run| {
            surface.paint_scene().shaped_text_resource(run.resource_ref())
                .into_iter().flat_map(|resource| resource.glyphs().iter().map(|g| g.id()))
        })
        .collect()
}

#[test]
fn secret_password_publication_masks_shaped_resources_and_semantic_value() {
    let mut runtime = mounted();
    let surface = publication(&mut runtime);
    let editable = surface.semantic_publication().snapshot().nodes()[0]
        .editable().unwrap_or_else(|| unreachable!("secret semantics present"));
    assert_eq!(editable.sensitivity(), TextSensitivity::Secret);
    assert_eq!(editable.value(), None);
    assert_eq!(editable.selection().active().byte_offset(), SECRET.len());
    assert_eq!(
        surface.semantic_publication().snapshot().nodes()[0]
            .placeholder(),
        Some("Password")
    );

    let mask = TextMaskedProjection::document(snapshot(0), SECRET)
        .unwrap_or_else(|_| unreachable!("mask is valid"));
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(system.register_font_bytes(FONT.to_vec()).is_ok());
    assert!(system.set_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")
            .unwrap_or_else(|_| unreachable!("font is named"))],
    ).is_ok());
    let mut layout = TextLayoutState::new();
    let artifact = system.layout_text(
        &mut layout,
        &TextRequest::new(mask.display_text(), Typography::default(), TextConstraints::unbounded()),
    ).unwrap_or_else(|_| unreachable!("mask shapes")).into_artifact();
    let expected: Vec<u32> = artifact.lines().iter()
        .flat_map(|line| line.runs())
        .flat_map(|run| run.shaped_resource().glyphs().iter().map(|g| g.id()))
        .collect();
    assert!(!expected.is_empty());
    assert_eq!(glyphs(&surface), expected);
    assert!(!format!("{:?}", surface.paint_scene()).contains(SECRET));
}

#[test]
fn password_submit_and_public_rebuild_keep_m10_authority_and_shaping_consistent() {
    let mut runtime = mounted();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime.submit_command(
        owner, SemanticCommand::RequestFocus, CommandOrigin::programmatic(),
    ).unwrap_or_else(|_| unreachable!("focus command is accepted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    publication(&mut runtime);

    runtime.submit_text(
        CommittedTextEvent::new("!", None)
            .unwrap_or_else(|_| unreachable!("insertion is non-empty")),
    ).unwrap_or_else(|_| unreachable!("secret field accepts insertion"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, format!("{SECRET}!"));
    let after_edit = publication(&mut runtime);
    assert_eq!(
        after_edit.semantic_publication().snapshot().nodes()[0]
            .editable().and_then(|e| e.value()),
        None
    );
    assert!(!runtime.trace().export_jsonl().contains(SECRET));

    runtime.submit_action(Action::ToggleSecret)
        .unwrap_or_else(|_| unreachable!("rebuild is admitted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let public = publication(&mut runtime);
    let editable = public.semantic_publication().snapshot().nodes()[0]
        .editable().unwrap_or_else(|| unreachable!("public semantics are present"));
    assert_eq!(editable.sensitivity(), TextSensitivity::Public);
    assert_eq!(editable.value(), Some(runtime.state().text.as_str()));
    assert_ne!(glyphs(&after_edit), glyphs(&public));
}
