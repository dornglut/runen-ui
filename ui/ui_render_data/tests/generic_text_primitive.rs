use ui_math::{UiPoint, UiRect};
use ui_render_data::{GlyphRunPrimitive, UiDrawKey, UiPaint, UiPrimitive, UiSortKey};
use ui_text::{
    FontId, TextBlockId, TextClusterRange, TextDirectionPolicy, TextGlyph, TextOverflowEvidence,
    TextRunId, TextStyle, TextVisualRun,
};

#[test]
fn glyph_run_primitive_carries_visual_run_evidence() {
    let visual_run = TextVisualRun {
        visual_run_id: 0,
        line_index: 0,
        run_id: TextRunId(1),
        span_id: None,
        font_id: FontId(0),
        style: TextStyle::default(),
        direction: TextDirectionPolicy::Ltr,
        glyphs: vec![TextGlyph {
            draw_order: 0,
            line_index: 0,
            run_id: TextRunId(1),
            span_id: None,
            font_id: FontId(0),
            glyph_key: "char:A".to_owned(),
            cluster_range: TextClusterRange::new(0, 1),
            origin: UiPoint::new(0.0, 9.0),
            advance: 8.0,
            bounds: UiRect::new(0.0, 0.0, 8.0, 12.0),
            source_text_preview: "A".to_owned(),
            replacement: false,
        }],
        bounds: UiRect::new(0.0, 0.0, 8.0, 12.0),
    };

    let primitive = GlyphRunPrimitive::from_visual_runs(
        TextBlockId(7),
        vec![visual_run],
        TextOverflowEvidence::none(),
        None,
        UiPaint::WHITE,
        UiDrawKey::new(1, None),
        UiSortKey::new(0, 0, 0),
    );

    assert_eq!(primitive.block_id, TextBlockId(7));
    assert_eq!(primitive.line_count, 1);
    assert_eq!(primitive.glyph_count, 1);
    let UiPrimitive::GlyphRun(projected) = UiPrimitive::from(primitive) else {
        panic!("generic text primitive should project as glyph run primitive");
    };
    assert_eq!(projected.visual_runs[0].glyphs[0].glyph_key, "char:A");
}
