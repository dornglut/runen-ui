use std::{borrow::Cow, collections::HashMap, sync::Weak};

use parley::setting::Tag;
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily as ParleyFontFamily,
    FontFamilyName as ParleyFontFamilyName, FontFeature as ParleyFontFeature,
    FontFeatures as ParleyFontFeatures, FontStyle as ParleyFontStyle,
    FontVariation as ParleyFontVariation, FontVariations as ParleyFontVariations,
    FontWeight as ParleyFontWeight, FontWidth as ParleyFontWidth, Language, Layout, LayoutContext,
    OverflowWrap as ParleyOverflowWrap, StyleProperty, TextWrapMode as ParleyTextWrapMode,
    WordBreak as ParleyWordBreak,
};
use runenui_core::{FontFamily, FontStyle, LogicalLength, ResourceRef, Typography};

use crate::{
    FontSourceSnapshot, ShapedTextResource, TextAlignment, TextArtifact, TextLayoutError,
    TextOverflowWrap, TextRequest, TextWordBreak, TextWrapMode,
    font_source_config::backend_generic_family, layout_extract,
};

pub fn shape_text(
    font_context: &mut FontContext,
    layout_context: &mut LayoutContext,
    request: &TextRequest,
) -> Result<Layout<[u8; 4]>, TextLayoutError> {
    let mut builder = layout_context.ranged_builder(font_context, request.text(), 1.0, false);

    for property in typography_properties(request.typography())? {
        builder.push_default(property);
    }
    let paragraph = request.paragraph_style();
    if let Some(language) = paragraph.language() {
        builder.push_default(StyleProperty::Locale(Some(parley_language(language))));
    }
    builder.push_default(StyleProperty::TextWrapMode(match paragraph.wrap_mode() {
        TextWrapMode::Wrap => ParleyTextWrapMode::Wrap,
        TextWrapMode::NoWrap => ParleyTextWrapMode::NoWrap,
    }));
    builder.push_default(StyleProperty::WordBreak(match paragraph.word_break() {
        TextWordBreak::Normal => ParleyWordBreak::Normal,
        TextWordBreak::BreakAll => ParleyWordBreak::BreakAll,
        TextWordBreak::KeepAll => ParleyWordBreak::KeepAll,
    }));
    builder.push_default(StyleProperty::OverflowWrap(
        match paragraph.overflow_wrap() {
            TextOverflowWrap::Normal => ParleyOverflowWrap::Normal,
            TextOverflowWrap::Anywhere => ParleyOverflowWrap::Anywhere,
            TextOverflowWrap::BreakWord => ParleyOverflowWrap::BreakWord,
        },
    ));

    for span in request.metric_spans() {
        let range = span.range();
        for property in typography_properties(span.typography())? {
            builder.push(property, range.clone());
        }
        if let Some(language) = span.language() {
            builder.push(
                StyleProperty::Locale(Some(parley_language(language))),
                range,
            );
        }
    }

    Ok(builder.build(request.text()))
}

pub fn relayout_text(
    layout: &mut Layout<[u8; 4]>,
    resources: &mut HashMap<ResourceRef, Weak<ShapedTextResource>>,
    source_snapshot: FontSourceSnapshot,
    request: &TextRequest,
) -> Result<TextArtifact, TextLayoutError> {
    let paragraph = request.paragraph_style();
    let max_inline = if request.constraints().is_min_content() {
        Some(layout.calculate_content_widths().min)
    } else {
        request.constraints().max_inline().map(LogicalLength::get)
    };
    layout.break_all_lines(max_inline);
    layout.align(
        match paragraph.alignment() {
            TextAlignment::Start => Alignment::Start,
            TextAlignment::End => Alignment::End,
            TextAlignment::Center => Alignment::Center,
            TextAlignment::Justify => Alignment::Justify,
        },
        AlignmentOptions::default(),
    );

    layout_extract::extract_layout(layout, source_snapshot, resources)
        .ok_or(TextLayoutError::InvalidArtifact)
}

fn typography_properties(
    typography: &Typography,
) -> Result<Vec<StyleProperty<'static, [u8; 4]>>, TextLayoutError> {
    let families: Vec<ParleyFontFamilyName<'static>> = typography
        .families()
        .iter()
        .map(|family| {
            Ok(match family {
                FontFamily::Named(name) => {
                    ParleyFontFamilyName::Named(Cow::Owned(name.as_str().to_owned()))
                }
                FontFamily::Generic(family) => ParleyFontFamilyName::Generic(
                    backend_generic_family(*family)
                        .ok_or(TextLayoutError::UnsupportedGenericFamily)?,
                ),
            })
        })
        .collect::<Result<_, TextLayoutError>>()?;
    let variations = typography
        .variations()
        .iter()
        .map(|variation| {
            ParleyFontVariation::new(Tag::from_bytes(variation.tag().bytes()), variation.value())
        })
        .collect();
    let features = typography
        .features()
        .iter()
        .map(|feature| {
            ParleyFontFeature::new(Tag::from_bytes(feature.tag().bytes()), feature.value())
        })
        .collect();

    Ok(vec![
        StyleProperty::FontFamily(ParleyFontFamily::List(Cow::Owned(families))),
        StyleProperty::FontSize(typography.size().get()),
        StyleProperty::FontWeight(ParleyFontWeight::new(typography.weight().get())),
        StyleProperty::FontWidth(ParleyFontWidth::from_ratio(typography.width().ratio())),
        StyleProperty::FontStyle(match typography.style() {
            FontStyle::Normal => ParleyFontStyle::Normal,
            FontStyle::Italic => ParleyFontStyle::Italic,
            FontStyle::Oblique(angle) => {
                ParleyFontStyle::Oblique(angle.map(runenui_core::FontObliqueAngle::degrees))
            }
        }),
        StyleProperty::FontVariations(ParleyFontVariations::List(Cow::Owned(variations))),
        StyleProperty::FontFeatures(ParleyFontFeatures::List(Cow::Owned(features))),
    ])
}

fn parley_language(language: &crate::TextLanguage) -> Language {
    Language::parse(language.as_str())
        .unwrap_or_else(|_| unreachable!("TextLanguage stores a validated canonical language"))
}
