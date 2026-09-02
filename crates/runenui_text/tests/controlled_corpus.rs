use std::{collections::HashSet, error::Error};

use runenui_core::{FontFamily, FontFamilyName, GenericFontFamily, LogicalLength, Typography};
use runenui_text::{
    FontSourcePolicy, TextArtifact, TextConstraints, TextDirection, TextLanguage, TextLayoutState,
    TextParagraphStyle, TextRequest, TextSystem,
};

const CANTARELL: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
const DEVANAGARI: &[u8] = include_bytes!("fixtures/RunenUIFixtureDevanagari-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("fixtures/RunenUIFixtureArabic-Regular.ttf");

fn named_typography(family: &str) -> Result<Typography, Box<dyn Error>> {
    Ok(Typography::new(
        FontFamily::named(family)?,
        LogicalLength::new(20.0)?,
    ))
}

fn register_corpus(system: &mut TextSystem) -> Result<(), Box<dyn Error>> {
    for font in [CANTARELL, DEVANAGARI, ARABIC] {
        assert!(system.register_font_bytes(font.to_vec())? > 0);
    }

    let families = [
        FontFamilyName::new("Cantarell")?,
        FontFamilyName::new("RunenUI Fixture Devanagari")?,
        FontFamilyName::new("RunenUI Fixture Arabic")?,
    ];
    assert!(system.set_generic_family_mapping(GenericFontFamily::SansSerif, &families)?);
    Ok(())
}

fn shape(
    system: &mut TextSystem,
    text: &str,
    typography: Typography,
    paragraph: TextParagraphStyle,
) -> Result<TextArtifact, Box<dyn Error>> {
    let mut state = TextLayoutState::new();
    Ok(system
        .layout_text(
            &mut state,
            &TextRequest::new(text, typography, TextConstraints::unbounded())
                .with_paragraph_style(paragraph),
        )?
        .into_artifact())
}

fn glyph_ids(artifact: &TextArtifact) -> impl Iterator<Item = u32> + '_ {
    artifact
        .lines()
        .iter()
        .flat_map(|line| line.runs())
        .flat_map(|run| run.shaped_resource().glyphs())
        .map(|glyph| glyph.id())
}

#[test]
fn controlled_generic_fallback_uses_each_exact_bundled_source() -> Result<(), Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    register_corpus(&mut system)?;

    let artifact = shape(
        &mut system,
        "RunenUI क्षि سلام",
        Typography::default(),
        TextParagraphStyle::default(),
    )?;
    let mut saw_cantarell = false;
    let mut saw_devanagari = false;
    let mut saw_arabic = false;

    for run in artifact.lines().iter().flat_map(|line| line.runs()) {
        let bytes = run.shaped_resource().font().bytes();
        saw_cantarell |= bytes == CANTARELL;
        saw_devanagari |= bytes == DEVANAGARI;
        saw_arabic |= bytes == ARABIC;
    }

    assert!(saw_cantarell);
    assert!(saw_devanagari);
    assert!(saw_arabic);
    Ok(())
}

#[test]
fn arabic_joining_is_contextual_and_right_to_left() -> Result<(), Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(system.register_font_bytes(ARABIC.to_vec())? > 0);
    let typography = named_typography("RunenUI Fixture Arabic")?;
    let paragraph = TextParagraphStyle::default().with_language(TextLanguage::new("ar")?);

    let joined = shape(&mut system, "سلام", typography.clone(), paragraph.clone())?;
    assert!(
        joined
            .lines()
            .iter()
            .flat_map(|line| line.runs())
            .any(|run| run.direction() == TextDirection::RightToLeft)
    );

    let mut isolated_ids = HashSet::new();
    for scalar in ["س", "ل", "ا", "م"] {
        let isolated = shape(&mut system, scalar, typography.clone(), paragraph.clone())?;
        isolated_ids.extend(glyph_ids(&isolated));
    }
    assert!(glyph_ids(&joined).any(|glyph_id| !isolated_ids.contains(&glyph_id)));
    Ok(())
}

#[test]
fn devanagari_conjunct_and_matra_shape_beyond_scalar_count() -> Result<(), Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(system.register_font_bytes(DEVANAGARI.to_vec())? > 0);
    let text = "क्षि";
    let artifact = shape(
        &mut system,
        text,
        named_typography("RunenUI Fixture Devanagari")?,
        TextParagraphStyle::default().with_language(TextLanguage::new("hi-Deva")?),
    )?;

    assert!(glyph_ids(&artifact).count() < text.chars().count());
    Ok(())
}

#[test]
fn mixed_bidi_exposes_both_visual_run_directions() -> Result<(), Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    register_corpus(&mut system)?;
    let artifact = shape(
        &mut system,
        "abc سلام xyz",
        Typography::default(),
        TextParagraphStyle::default(),
    )?;

    let mut saw_ltr = false;
    let mut saw_rtl = false;
    for run in artifact.lines().iter().flat_map(|line| line.runs()) {
        saw_ltr |= run.direction() == TextDirection::LeftToRight;
        saw_rtl |= run.direction() == TextDirection::RightToLeft;
    }
    assert!(saw_ltr);
    assert!(saw_rtl);
    Ok(())
}

#[test]
fn combining_sequence_remains_one_logical_cluster() -> Result<(), Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(system.register_font_bytes(CANTARELL.to_vec())? > 0);
    let text = "e\u{301}";
    let artifact = shape(
        &mut system,
        text,
        named_typography("Cantarell")?,
        TextParagraphStyle::default().with_language(TextLanguage::new("en")?),
    )?;
    let clusters = artifact
        .lines()
        .iter()
        .flat_map(|line| line.runs())
        .flat_map(|run| run.clusters())
        .collect::<Vec<_>>();

    let cluster = clusters
        .first()
        .ok_or("combining fixture must produce one logical cluster")?;
    assert_eq!(clusters.len(), 1);
    assert_eq!(cluster.text_range(), 0..text.len());
    Ok(())
}
