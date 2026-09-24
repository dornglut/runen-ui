use std::{error::Error, sync::Arc};

use runenui_core::{
    __runtime::RuntimeNamespace, CompositionRange, FontFamily, FontFamilyName, LogicalLength,
    LogicalPoint, LogicalRect, LogicalTransform, TextAffinity, TextDisplayPosition, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextPreeditPosition, TextRange,
    TextSelection, Typography,
};
use runenui_text::{
    FontSourcePolicy, TextCaretMap, TextCaretMapError, TextConstraints, TextDisplaySelection,
    TextLayoutDecision, TextLayoutState, TextNavigation, TextNavigationMode, TextParagraphStyle,
    TextPreeditProjection, TextRequest, TextSystem,
};

const CANTARELL: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("fixtures/RunenUIFixtureArabic-Regular.ttf");
const DEVANAGARI: &[u8] = include_bytes!("fixtures/RunenUIFixtureDevanagari-Regular.ttf");

const fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(41), TextDocumentRevision::new(revision))
}

fn typography() -> Result<Typography, Box<dyn Error>> {
    Ok(Typography::new(
        FontFamily::named("Cantarell")?,
        LogicalLength::new(20.0)?,
    ))
}

fn text_system() -> Result<TextSystem, Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(system.register_font_bytes(CANTARELL.to_vec())? > 0);
    assert!(system.register_font_bytes(ARABIC.to_vec())? > 0);
    assert!(system.register_font_bytes(DEVANAGARI.to_vec())? > 0);
    system.set_generic_family_mapping(
        runenui_core::GenericFontFamily::SansSerif,
        &[
            FontFamilyName::new("Cantarell")?,
            FontFamilyName::new("RunenUI Fixture Arabic")?,
            FontFamilyName::new("RunenUI Fixture Devanagari")?,
        ],
    )?;
    Ok(system)
}

fn layout(
    system: &mut TextSystem,
    state: &mut TextLayoutState,
    text: &str,
    width: Option<f32>,
) -> Result<(), Box<dyn Error>> {
    let constraints = match width {
        Some(width) => TextConstraints::limited(LogicalLength::new(width)?),
        None => TextConstraints::unbounded(),
    };
    system.layout_text(
        state,
        &TextRequest::new(text, typography()?, constraints)
            .with_paragraph_style(TextParagraphStyle::default()),
    )?;
    Ok(())
}

fn document_position(source: &str, offset: usize, affinity: TextAffinity) -> TextDisplayPosition {
    TextDisplayPosition::Document(
        TextPosition::new(snapshot(1), source, offset, affinity)
            .unwrap_or_else(|_| unreachable!("fixture position is scalar-aligned")),
    )
}

fn map_for(source: &str, width: Option<f32>) -> Result<TextCaretMap, Box<dyn Error>> {
    let mut system = text_system()?;
    let mut state = TextLayoutState::new();
    layout(&mut system, &mut state, source, width)?;
    Ok(state.caret_map(snapshot(1))?)
}

#[test]
fn scalar_coordinates_are_narrowed_to_grapheme_and_shaping_stops() -> Result<(), Box<dyn Error>> {
    let source = "e\u{301} office 👩\u{200d}💻";
    let map = map_for(source, None)?;

    let inside_combining = document_position(source, 1, TextAffinity::Downstream);
    assert_eq!(
        map.validate_position(&inside_combining),
        Err(TextCaretMapError::NotCaretStop)
    );

    let zwj = source.find('\u{200d}').ok_or("fixture contains ZWJ")?;
    let inside_emoji = document_position(source, zwj, TextAffinity::Downstream);
    assert_eq!(
        map.validate_position(&inside_emoji),
        Err(TextCaretMapError::NotCaretStop)
    );

    let legal = map.legal_positions();
    assert!(!legal.is_empty());
    assert!(
        legal
            .iter()
            .all(|position| map.validate_position(position).is_ok())
    );
    let expected_offsets = legal
        .iter()
        .filter_map(|position| match position {
            TextDisplayPosition::Document(position) => Some(position.byte_offset()),
            TextDisplayPosition::Preedit(_) => None,
        })
        .fold(Vec::new(), |mut offsets, offset| {
            if offsets.last() != Some(&offset) {
                offsets.push(offset);
            }
            offsets
        });
    assert_eq!(map.legal_byte_offsets(), expected_offsets);
    assert!(!legal.contains(&inside_combining));
    assert!(!legal.contains(&inside_emoji));

    let size = map.artifact().size();
    let clip = LogicalRect::try_new(0.0, 0.0, size.width().max(1.0), size.height().max(1.0))?;
    for step in 0_u8..=64 {
        let point =
            LogicalPoint::new(size.width() * (f32::from(step) / 64.0), size.height() * 0.5)?;
        if let Some(hit) = map.hit_test(snapshot(1), point, clip, LogicalTransform::IDENTITY)? {
            map.validate_position(&hit)?;
        }
    }
    Ok(())
}

#[test]
fn ligature_components_do_not_override_grapheme_boundaries() -> Result<(), Box<dyn Error>> {
    let source = "क्षि";
    let mut system = text_system()?;
    let mut state = TextLayoutState::new();
    system.layout_text(
        &mut state,
        &TextRequest::new(
            source,
            Typography::new(
                FontFamily::named("RunenUI Fixture Devanagari")?,
                LogicalLength::new(20.0)?,
            ),
            TextConstraints::unbounded(),
        ),
    )?;
    let map = state.caret_map(snapshot(1))?;
    for (offset, _) in source.char_indices().skip(1) {
        let position = document_position(source, offset, TextAffinity::Downstream);
        assert_eq!(
            map.validate_position(&position),
            Err(TextCaretMapError::NotCaretStop),
            "scalar boundary {offset} inside the controlled Devanagari grapheme must not become a caret stop"
        );
    }

    let start = document_position(source, 0, TextAffinity::Downstream);
    let moved = map.navigate(
        &TextDisplaySelection::new(start.clone(), start),
        TextNavigation::NextLogical,
        TextNavigationMode::Move,
        None,
    )?;
    let TextDisplayPosition::Document(active) = moved.selection().active() else {
        return Err("document navigation returned a synthetic position".into());
    };
    assert_eq!(active.byte_offset(), source.len());
    map.validate_position(moved.selection().active())?;
    Ok(())
}

#[test]
fn utf16_conversion_rejects_stale_split_and_non_caret_positions() -> Result<(), Box<dyn Error>> {
    let source = "a🙂e\u{301}";
    let map = map_for(source, None)?;
    let split = TextPosition::from_utf16_offset(snapshot(1), source, 2, TextAffinity::Downstream);
    assert_eq!(
        split,
        Err(runenui_core::TextPositionError::Utf16SplitScalar)
    );

    let combining =
        TextPosition::from_utf16_offset(snapshot(1), source, 4, TextAffinity::Downstream)?;
    assert_eq!(
        map.validate_position(&TextDisplayPosition::Document(combining)),
        Err(TextCaretMapError::NotCaretStop)
    );

    let stale = TextPosition::from_utf16_offset(snapshot(2), source, 0, TextAffinity::Downstream)?;
    assert_eq!(
        map.validate_position(&TextDisplayPosition::Document(stale)),
        Err(TextCaretMapError::SnapshotMismatch)
    );
    let drifted_source = "a🙂e\u{301} extra";
    let drifted = TextPosition::new(
        snapshot(1),
        drifted_source,
        drifted_source.len(),
        TextAffinity::Upstream,
    )?;
    assert_eq!(
        map.validate_position(&TextDisplayPosition::Document(drifted)),
        Err(TextCaretMapError::OutOfBounds)
    );
    Ok(())
}

#[test]
fn map_correlates_hit_caret_selection_and_candidate_geometry() -> Result<(), Box<dyn Error>> {
    let source = "abc אבג xyz";
    let map = map_for(source, None)?;
    let start = document_position(source, 0, TextAffinity::Downstream);
    let end = document_position(source, source.len(), TextAffinity::Upstream);
    let selection = TextDisplaySelection::new(start.clone(), end);
    let rects = map.selection_rects(&selection)?;
    assert!(!rects.is_empty());
    assert!(rects.iter().all(|item| item.rect().width() > 0.0));
    let legal = map.legal_positions();
    let has_discontiguous_bidi_selection = legal.iter().enumerate().any(|(start_index, start)| {
        legal.iter().skip(start_index + 1).any(|end| {
            map.selection_rects(&TextDisplaySelection::new(start.clone(), end.clone()))
                .is_ok_and(|rects| rects.len() > 1)
        })
    });
    assert!(has_discontiguous_bidi_selection);

    let caret = map.caret_rect(&start, LogicalLength::new(1.0)?)?;
    let candidate = map.candidate_rect(&start)?;
    assert_eq!(candidate.x().to_bits(), caret.x().to_bits());
    assert_eq!(candidate.y().to_bits(), caret.y().to_bits());
    assert_eq!(candidate.width().to_bits(), 0.0_f32.to_bits());

    let clip = LogicalRect::try_new(0.0, 0.0, 10_000.0, 10_000.0)?;
    let transform = LogicalTransform::translation(100.0, 50.0)?;
    let hit = map
        .hit_test(
            snapshot(1),
            LogicalPoint::new(
                caret.x() + 100.0,
                caret.height().mul_add(0.5, caret.y()) + 50.0,
            )?,
            clip,
            transform,
        )?
        .ok_or("eligible caret point must hit")?;
    map.validate_position(&hit)?;

    let outside = map.hit_test(
        snapshot(1),
        LogicalPoint::new(-1.0, -1.0)?,
        LogicalRect::try_new(0.0, 0.0, 100.0, 100.0)?,
        LogicalTransform::IDENTITY,
    )?;
    assert!(outside.is_none());
    assert_eq!(
        map.hit_test(
            snapshot(2),
            LogicalPoint::new(0.0, 0.0)?,
            LogicalRect::try_new(0.0, 0.0, 100.0, 100.0)?,
            LogicalTransform::IDENTITY,
        ),
        Err(TextCaretMapError::SnapshotMismatch)
    );
    assert_eq!(
        map.hit_test(
            snapshot(1),
            LogicalPoint::new(0.0, 0.0)?,
            LogicalRect::try_new(-1.0, -1.0, 2.0, 2.0)?,
            LogicalTransform::try_new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)?,
        ),
        Err(TextCaretMapError::NonInvertibleTransform)
    );
    Ok(())
}

#[test]
fn captured_nearest_position_uses_retained_layout_outside_hit_bounds() -> Result<(), Box<dyn Error>>
{
    let source = "alpha beta gamma delta epsilon אבג office";
    let map = map_for(source, Some(90.0))?;
    assert!(map.artifact().lines().len() > 1);
    let transform = LogicalTransform::translation(20.0, 30.0)?;
    let points = [
        LogicalPoint::new(-10_000.0, 20.0)?,
        LogicalPoint::new(10_000.0, 20.0)?,
        LogicalPoint::new(40.0, -10_000.0)?,
        LogicalPoint::new(40.0, 10_000.0)?,
        LogicalPoint::new(40.0, 50.0)?,
    ];
    for point in points {
        let position = map.nearest_position(snapshot(1), point, transform)?;
        map.validate_position(&position)?;
        assert!(map.legal_positions().contains(&position));
    }

    // Initial admission remains clipped even though captured mapping is not.
    assert_eq!(
        map.hit_test(
            snapshot(1),
            LogicalPoint::new(-10_000.0, 20.0)?,
            LogicalRect::try_new(0.0, 0.0, 100.0, 100.0)?,
            transform,
        )?,
        None
    );
    assert_eq!(
        map.nearest_position(snapshot(2), LogicalPoint::new(40.0, 50.0)?, transform,),
        Err(TextCaretMapError::SnapshotMismatch)
    );
    assert_eq!(
        map.nearest_position(
            snapshot(1),
            LogicalPoint::new(40.0, 50.0)?,
            LogicalTransform::try_new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)?,
        ),
        Err(TextCaretMapError::NonInvertibleTransform)
    );
    Ok(())
}

#[test]
fn logical_and_visual_navigation_diverge_in_mixed_bidi_and_preserve_direction()
-> Result<(), Box<dyn Error>> {
    let source = "abc אבג xyz";
    let map = map_for(source, None)?;
    let hebrew_start = source.find('א').ok_or("fixture contains Hebrew")?;
    let position = document_position(source, hebrew_start, TextAffinity::Downstream);
    map.validate_position(&position)?;
    let selection = TextDisplaySelection::new(position.clone(), position);

    let logical = map.navigate(
        &selection,
        TextNavigation::NextLogical,
        TextNavigationMode::Move,
        None,
    )?;
    let visual = map.navigate(
        &selection,
        TextNavigation::NextVisual,
        TextNavigationMode::Move,
        None,
    )?;
    assert_ne!(logical.selection(), visual.selection());

    let extended = map.navigate(
        &selection,
        TextNavigation::NextVisualWord,
        TextNavigationMode::Extend,
        None,
    )?;
    assert_eq!(extended.selection().anchor(), selection.anchor());
    assert!(!map.is_selection_collapsed(extended.selection())?);
    Ok(())
}

#[test]
fn controlled_ltr_rtl_and_mixed_corpora_keep_all_public_geometry_legal()
-> Result<(), Box<dyn Error>> {
    for source in ["abc", "אבג", "abc אבג xyz"] {
        let map = map_for(source, None)?;
        let legal = map.legal_positions();
        assert!(!legal.is_empty());
        for position in &legal {
            map.validate_position(position)?;
            let caret = map.caret_rect(position, LogicalLength::new(1.0)?)?;
            assert!(caret.height() > 0.0);
        }

        let selection = TextDisplaySelection::new(
            document_position(source, 0, TextAffinity::Downstream),
            document_position(source, source.len(), TextAffinity::Upstream),
        );
        assert!(!map.selection_rects(&selection)?.is_empty());
    }
    Ok(())
}

#[test]
fn wrapped_line_affinity_and_vertical_preference_are_deterministic() -> Result<(), Box<dyn Error>> {
    let source = "alpha beta gamma delta epsilon";
    let map = map_for(source, Some(90.0))?;
    assert!(map.artifact().lines().len() > 1);
    let wrap_offset = map
        .artifact()
        .lines()
        .iter()
        .flat_map(runenui_text::TextLine::runs)
        .flat_map(runenui_text::TextRun::clusters)
        .find(|cluster| cluster.is_soft_line_break())
        .map(|cluster| cluster.text_range().end)
        .ok_or("fixture must contain a soft-wrap boundary")?;
    let upstream = document_position(source, wrap_offset, TextAffinity::Upstream);
    let downstream = document_position(source, wrap_offset, TextAffinity::Downstream);
    map.validate_position(&upstream)?;
    map.validate_position(&downstream)?;
    let upstream_rect = map.caret_rect(&upstream, LogicalLength::new(1.0)?)?;
    let downstream_rect = map.caret_rect(&downstream, LogicalLength::new(1.0)?)?;
    assert_ne!(upstream_rect.y().to_bits(), downstream_rect.y().to_bits());
    let start = document_position(source, 0, TextAffinity::Downstream);
    let selection = TextDisplaySelection::new(start.clone(), start);
    let down = map.navigate(
        &selection,
        TextNavigation::NextLine,
        TextNavigationMode::Move,
        None,
    )?;
    let preferred = down.preferred_inline().ok_or("vertical move keeps x")?;
    let down_again = map.navigate(
        down.selection(),
        TextNavigation::NextLine,
        TextNavigationMode::Move,
        Some(preferred),
    )?;
    assert_eq!(down_again.preferred_inline(), Some(preferred));
    assert_ne!(down.selection(), &selection);
    Ok(())
}

#[test]
fn word_and_hard_line_navigation_return_valid_positions() -> Result<(), Box<dyn Error>> {
    let source = "alpha beta\ngamma delta";
    let map = map_for(source, Some(80.0))?;
    let offset = source.find("beta").ok_or("fixture contains beta")?;
    let start = document_position(source, offset, TextAffinity::Downstream);
    map.validate_position(&start)?;
    let selection = TextDisplaySelection::new(start.clone(), start);

    for operation in [
        TextNavigation::PreviousLogicalWord,
        TextNavigation::NextLogicalWord,
        TextNavigation::PreviousVisualWord,
        TextNavigation::NextVisualWord,
        TextNavigation::LineStart,
        TextNavigation::LineEnd,
        TextNavigation::HardLineStart,
        TextNavigation::HardLineEnd,
    ] {
        let result = map.navigate(&selection, operation, TextNavigationMode::Move, None)?;
        map.validate_position(result.selection().active())?;
    }
    Ok(())
}

#[test]
fn terminal_newline_and_empty_selection_use_no_guessed_rectangle() -> Result<(), Box<dyn Error>> {
    for source in ["", "a\n"] {
        let map = map_for(source, None).map_err(|error| {
            format!("controlled source {source:?} failed to build caret map: {error}")
        })?;
        let start = document_position(
            source,
            0,
            if source.is_empty() {
                TextAffinity::Upstream
            } else {
                TextAffinity::Downstream
            },
        );
        map.validate_position(&start)?;
        if source.is_empty() {
            let artifact = map.artifact();
            assert!(
                artifact
                    .lines()
                    .iter()
                    .all(|line| line.text_range() == (0..0)),
                "empty source must expose only real 0..0 source ranges"
            );
            assert!(
                artifact.lines().iter().all(|line| line.runs().is_empty()),
                "Parley's synthetic empty-source shaping input must not become a RunenUI paint resource"
            );
        }
        assert!(
            map.selection_rects(&TextDisplaySelection::new(start.clone(), start))?
                .is_empty()
        );
        if source.ends_with('\n') {
            let before_newline = document_position(source, 1, TextAffinity::Downstream);
            let end = document_position(source, source.len(), TextAffinity::Upstream);
            assert!(
                map.selection_rects(&TextDisplaySelection::new(before_newline, end))?
                    .is_empty()
            );
        }
    }
    Ok(())
}

#[test]
fn preedit_projection_distinguishes_synthetic_and_durable_positions() -> Result<(), Box<dyn Error>>
{
    let document = "abXYZcd";
    let replacement = TextRange::new(snapshot(1), document, 2, 5)?;
    let namespace = RuntimeNamespace::__runtime_new();
    let generation = namespace.__runtime_composition_generation(9);
    let selected = CompositionRange::new("かな", 0, "か".len())?;
    let projection = Arc::new(TextPreeditProjection::new(
        snapshot(1),
        document,
        replacement,
        generation.clone(),
        "かな",
        Some(selected),
    )?);
    assert_eq!(projection.display_text(), "abかなcd");
    assert_eq!(projection.document_text(), document);

    let mut system = text_system()?;
    let mut state = TextLayoutState::new();
    layout(&mut system, &mut state, projection.display_text(), None)?;
    let map = state.preedit_caret_map(projection.clone())?;
    let synthetic = TextDisplayPosition::Preedit(TextPreeditPosition::new(
        snapshot(1),
        generation,
        projection.preedit(),
        "か".len(),
        TextAffinity::Downstream,
    )?);
    map.validate_position(&synthetic)?;
    assert!(map.candidate_rect(&synthetic)?.height() > 0.0);
    let projected_selection = map
        .preedit_selection()?
        .ok_or("projection retains its checked selection")?;
    assert!(matches!(
        projected_selection.anchor(),
        TextDisplayPosition::Preedit(_)
    ));
    assert!(matches!(
        projected_selection.active(),
        TextDisplayPosition::Preedit(_)
    ));

    let hidden = TextDisplayPosition::Document(TextPosition::new(
        snapshot(1),
        document,
        3,
        TextAffinity::Downstream,
    )?);
    assert_eq!(
        map.validate_position(&hidden),
        Err(TextCaretMapError::HiddenDocumentPosition)
    );

    let foreign = TextDisplayPosition::Preedit(TextPreeditPosition::new(
        snapshot(1),
        namespace.__runtime_composition_generation(10),
        projection.preedit(),
        0,
        TextAffinity::Downstream,
    )?);
    assert_eq!(
        map.validate_position(&foreign),
        Err(TextCaretMapError::ForeignComposition)
    );

    let synthetic_start_upstream = TextDisplayPosition::Preedit(TextPreeditPosition::new(
        snapshot(1),
        projection.generation().clone(),
        projection.preedit(),
        0,
        TextAffinity::Upstream,
    )?);
    assert_eq!(
        map.validate_position(&synthetic_start_upstream),
        Err(TextCaretMapError::InvalidAffinity)
    );
    assert!(matches!(
        projection.position_from_display_offset(2, TextAffinity::Upstream)?,
        TextDisplayPosition::Document(_)
    ));
    assert!(matches!(
        projection.position_from_display_offset(2, TextAffinity::Downstream)?,
        TextDisplayPosition::Preedit(_)
    ));

    let mut mismatched_state = TextLayoutState::new();
    layout(&mut system, &mut mismatched_state, document, None)?;
    assert!(matches!(
        mismatched_state.preedit_caret_map(projection),
        Err(TextCaretMapError::DisplayTextMismatch)
    ));
    let debug = format!("{map:?}");
    assert!(!debug.contains(document));
    assert!(!debug.contains("かな"));
    let projection_debug = format!(
        "{:?}",
        map.preedit_projection()
            .ok_or("map retains the exact projection")?
    );
    assert!(!projection_debug.contains(document));
    assert!(!projection_debug.contains("かな"));
    Ok(())
}

#[test]
fn empty_preedit_selection_maps_to_one_durable_boundary() -> Result<(), Box<dyn Error>> {
    let document = "abXYZcd";
    let replacement = TextRange::new(snapshot(1), document, 2, 5)?;
    let namespace = RuntimeNamespace::__runtime_new();
    let projection = Arc::new(TextPreeditProjection::new(
        snapshot(1),
        document,
        replacement,
        namespace.__runtime_composition_generation(11),
        "",
        Some(CompositionRange::new("", 0, 0)?),
    )?);
    let mut system = text_system()?;
    let mut state = TextLayoutState::new();
    layout(&mut system, &mut state, projection.display_text(), None)?;
    let map = state.preedit_caret_map(projection)?;
    let selection = map
        .preedit_selection()?
        .ok_or("empty preedit retains a collapsed display selection")?;
    assert!(map.is_selection_collapsed(&selection)?);
    assert!(matches!(
        selection.active(),
        TextDisplayPosition::Document(_)
    ));
    map.validate_position(selection.active())?;
    Ok(())
}

#[test]
fn width_relinebreak_is_copy_on_write_for_previously_issued_maps() -> Result<(), Box<dyn Error>> {
    let source = "one two three four five six seven";
    let mut system = text_system()?;
    let mut state = TextLayoutState::new();
    layout(&mut system, &mut state, source, Some(400.0))?;
    let old_map = state.caret_map(snapshot(1))?;
    assert!(old_map.is_correlated_with(state.artifact().ok_or("state retains artifact")?));
    assert!(old_map.shares_layout_with(&old_map.clone()));
    let old_lines = old_map.artifact().lines().len();

    let outcome = system.layout_text(
        &mut state,
        &TextRequest::new(
            source,
            typography()?,
            TextConstraints::limited(LogicalLength::new(70.0)?),
        ),
    )?;
    assert_eq!(outcome.decision(), TextLayoutDecision::Relinebroken);
    let new_map = state.caret_map(snapshot(1))?;
    assert!(new_map.artifact().lines().len() > old_lines);
    assert_eq!(old_map.artifact().lines().len(), old_lines);
    assert!(!old_map.shares_layout_with(&new_map));
    assert!(!std::ptr::eq(old_map.artifact(), new_map.artifact()));
    Ok(())
}

#[test]
fn directional_document_selection_retains_endpoint_affinity() -> Result<(), Box<dyn Error>> {
    let source = "abc";
    let anchor = TextPosition::new(snapshot(1), source, 3, TextAffinity::Upstream)?;
    let active = TextPosition::new(snapshot(1), source, 0, TextAffinity::Downstream)?;
    let selection = TextSelection::new(anchor, active)?;
    let display = TextDisplaySelection::from_document(selection);
    assert_eq!(display.anchor().affinity(), TextAffinity::Upstream);
    assert_eq!(display.active().affinity(), TextAffinity::Downstream);
    assert_ne!(display.anchor(), display.active());
    Ok(())
}
