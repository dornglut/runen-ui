//! Mounted production shaping proof for the bounded secret-text layout guard.
//! Password input activation, caret/IME and service integration remain gated in #319.

use runenui_core::{
    EditableContribution, EditingSessionPolicy, Element, FontFamilyName, GenericFontFamily,
    NoHostProtocol, StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextLeafMeasure, TextPosition, TextSelection, TextSensitivity, Typography,
    UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput,
};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext};
use runenui_text::{
    FontSourcePolicy, TextConstraints, TextLayoutState, TextMaskedProjection, TextRequest, TextSystem,
};

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn snapshot() -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(721), TextDocumentRevision::new(1))
}

#[derive(Clone)]
struct Case {
    secret: &'static str,
    displayed: &'static str,
}

#[derive(Debug)]
struct EditableSecretLeaf {
    source: &'static str,
    measure: &'static str,
}

impl Widget<()> for EditableSecretLeaf {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn editable(&self, _: &Self::State) -> Option<EditableContribution<()>> {
        let cursor = TextPosition::new(
            snapshot(), self.source, self.source.len(), TextAffinity::Downstream,
        ).ok()?;
        EditableContribution::new_read_only(
            snapshot(),
            self.source,
            TextSelection::collapsed(cursor),
            TextSensitivity::Secret,
            false,
            EditingSessionPolicy::PreserveExact,
        ).ok()
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        // Deliberately supply the literal source to prove the runtime lowers
        // it to a masked M8 request rather than merely masking paint.
        WidgetMeasure::Text(TextLeafMeasure::new(self.measure))
    }
}

struct SecretApp;

impl UiApp for SecretApp {
    type State = Case;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(EditableSecretLeaf {
            source: state.secret,
            measure: state.displayed,
        })
        .id("secret.editable")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn published(case: Case) -> runenui_runtime::SurfacePublication {
    let mut runtime = AppRuntime::<SecretApp>::mount(case);
    assert!(runtime.register_text_font_bytes(FONT.to_vec()).is_ok());
    assert!(runtime.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")
            .unwrap_or_else(|_| unreachable!("font family validates"))],
    ).is_ok());
    let env = StyleEnvironment::default();
    runtime.publish_surface(&SurfaceBuildContext::tight(
        &env,
        LogicalSize::try_new(260.0, 60.0)
            .unwrap_or_else(|_| unreachable!("surface size validates")),
    )).unwrap_or_else(|_| unreachable!("secret widget publishes a masked surface"))
}

fn glyph_ids(surface: &runenui_runtime::SurfacePublication) -> Vec<u32> {
    surface.paint_scene().items().iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .flat_map(|run| {
            surface.paint_scene().shaped_text_resource(run.resource_ref())
                .into_iter().flat_map(|resource| resource.glyphs().iter().map(|glyph| glyph.id()))
        })
        .collect()
}

#[test]
fn secret_editable_leaf_shaped_resources_match_bullets_not_original_source() {
    let source = "sécret漢字";
    let surface = published(Case { secret: source, displayed: source });
    let mask = TextMaskedProjection::document(snapshot(), source)
        .unwrap_or_else(|_| unreachable!("fixture source masks"));
    let mut text = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(text.register_font_bytes(FONT.to_vec()).is_ok());
    assert!(text.set_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")
            .unwrap_or_else(|_| unreachable!("font family validates"))],
    ).is_ok());
    let mut retained = TextLayoutState::new();
    let expected = text.layout_text(
        &mut retained,
        &TextRequest::new(mask.display_text(), Typography::default(), TextConstraints::unbounded()),
    ).unwrap_or_else(|_| unreachable!("controlled expected mask shapes")).into_artifact();
    let expected_glyphs: Vec<u32> = expected.lines().iter()
        .flat_map(|line| line.runs())
        .flat_map(|run| run.shaped_resource().glyphs().iter().map(|glyph| glyph.id()))
        .collect();
    assert!(!expected_glyphs.is_empty(), "font must shape visible mask bullets");
    assert_eq!(glyph_ids(&surface), expected_glyphs);
    let published_debug = format!("{:?}", surface.paint_scene());
    assert!(!published_debug.contains(source));
}

#[test]
fn foreign_secret_descriptor_cannot_shape_literal_widget_content() {
    let surface = published(Case {
        secret: "owner-password",
        displayed: "unauthorized-plaintext",
    });
    assert!(glyph_ids(&surface).is_empty());
    let diagnostics = format!("{:?}", surface.layout_report());
    assert!(!diagnostics.contains("owner-password"));
    assert!(!diagnostics.contains("unauthorized-plaintext"));
}
