//! Mounted production shaping proof for the bounded secret-text layout guard.

macro_rules! admitted {
    ($result:expr, $fallback:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "expected accepted runtime admission, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return $fallback };
        value
    }};
    ($result:expr) => {{
        let result = $result;
        assert!(
            result.is_ok(),
            "expected accepted runtime admission, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

use runenui_core::{
    EditableContribution, EditingSessionPolicy, Element, FontFamilyName, GenericFontFamily,
    NoHostProtocol, StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextLeafMeasure, TextPosition, TextSelection, TextSensitivity,
    Typography, UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput,
};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext};
use runenui_text::{
    FontSourcePolicy, TextConstraints, TextLayoutState, TextMaskedProjection, TextRequest,
    TextSystem,
};

const FONT: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

const fn snapshot() -> TextDocumentSnapshot {
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
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<()>> {
        let cursor = TextPosition::new(
            snapshot(),
            self.source,
            self.source.len(),
            TextAffinity::Downstream,
        )
        .ok()?;
        EditableContribution::new_read_only(
            snapshot(),
            self.source,
            TextSelection::collapsed(cursor),
            TextSensitivity::Secret,
            false,
            EditingSessionPolicy::PreserveExact,
        )
        .ok()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        // Deliberately supply the literal source to prove the runtime lowers
        // it to a masked M8 request rather than merely masking paint.
        WidgetMeasure::Text(TextLeafMeasure::new(self.measure))
    }
}

struct SecretApp;

impl UiApp for SecretApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The single editable secret leaf has one initial owner reconciliation.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

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

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

fn published(case: Case) -> Option<runenui_runtime::SurfacePublication> {
    let mut runtime = admitted!(AppRuntime::<SecretApp>::mount(case), None);
    assert!(runtime.register_text_font_bytes(FONT.to_vec()).is_ok());
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell")
                    .unwrap_or_else(|_| unreachable!("font family validates"))],
            )
            .is_ok()
    );
    let env = StyleEnvironment::default();
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(
        &env,
        LogicalSize::try_new(260.0, 60.0)
            .unwrap_or_else(|_| unreachable!("surface size validates")),
    ));
    assert!(
        publication.is_ok(),
        "secret widget publishes a masked surface: {:?}",
        publication.as_ref().err()
    );
    let Ok(publication) = publication else {
        return None;
    };
    Some(publication)
}

fn glyph_ids(surface: &runenui_runtime::SurfacePublication) -> Vec<u32> {
    surface
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .flat_map(|run| {
            surface
                .paint_scene()
                .shaped_text_resource(run.resource_ref())
                .into_iter()
                .flat_map(|resource| resource.glyphs().iter().map(|glyph| glyph.id()))
        })
        .collect()
}

#[test]
fn secret_editable_leaf_shaped_resources_match_bullets_not_original_source() {
    let source = "sécret漢字";
    let Some(surface) = published(Case {
        secret: source,
        displayed: source,
    }) else {
        return;
    };
    let mask = TextMaskedProjection::document(snapshot(), source)
        .unwrap_or_else(|_| unreachable!("fixture source masks"));
    let mut text = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(text.register_font_bytes(FONT.to_vec()).is_ok());
    assert!(
        text.set_generic_family_mapping(
            GenericFontFamily::SansSerif,
            &[FontFamilyName::new("Cantarell")
                .unwrap_or_else(|_| unreachable!("font family validates"))],
        )
        .is_ok()
    );
    let mut retained = TextLayoutState::new();
    let expected = text
        .layout_text(
            &mut retained,
            &TextRequest::new(
                mask.display_text(),
                Typography::default(),
                TextConstraints::unbounded(),
            ),
        )
        .unwrap_or_else(|_| unreachable!("controlled expected mask shapes"))
        .into_artifact();
    let expected_glyphs: Vec<u32> = expected
        .lines()
        .iter()
        .flat_map(runenui_text::TextLine::runs)
        .flat_map(|run| {
            run.shaped_resource()
                .glyphs()
                .iter()
                .map(|glyph| glyph.id())
        })
        .collect();
    assert!(
        !expected_glyphs.is_empty(),
        "font must shape visible mask bullets"
    );
    assert_eq!(glyph_ids(&surface), expected_glyphs);
    let published_debug = format!("{:?}", surface.paint_scene());
    assert!(!published_debug.contains(source));
}

#[test]
fn foreign_secret_descriptor_cannot_shape_literal_widget_content() {
    let Some(surface) = published(Case {
        secret: "owner-password",
        displayed: "unauthorized-plaintext",
    }) else {
        return;
    };
    assert_eq!(glyph_ids(&surface), [] as [u32; 0]);
    let diagnostics = format!("{:?}", surface.layout_report());
    assert!(!diagnostics.contains("owner-password"));
    assert!(!diagnostics.contains("unauthorized-plaintext"));
}
