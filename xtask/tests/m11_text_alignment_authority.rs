//! Regression proof that text-leaf placement remains one generic runtime-owned path.

const CORE: &str = include_str!("../../crates/runenui_core/src/widget_protocol.rs");
const TEXT_REQUEST: &str = include_str!("../../crates/runenui_text/src/request.rs");
const LAYOUT: &str = include_str!("../../crates/runenui_runtime/src/surface/taffy_layout.rs");
const CACHE: &str = include_str!("../../crates/runenui_runtime/src/surface/cache.rs");
const PLANNING: &str = include_str!("../../crates/runenui_runtime/src/surface/planning.rs");
const PAINT: &str = include_str!("../../crates/runenui_runtime/src/surface/resolve.rs");
const DISPLAYED: &str = include_str!("../../crates/runenui_runtime/src/surface/transaction.rs");

#[test]
fn text_leaf_contract_has_one_owner_and_no_legacy_parallel_variant() {
    assert!(CORE.contains("Text(TextLeafMeasure)"));
    assert!(!CORE.contains("Text { content: String }"));
    assert!(TEXT_REQUEST.contains("use runenui_core::{TextAlignment, Typography};"));
    assert!(!TEXT_REQUEST.contains("pub enum TextAlignment"));
    // Topology legitimately retains WidgetTypeId for identity. The text-layout
    // path itself must not select a built-in type or implement special Button rules.
    assert!(!LAYOUT.contains("WidgetTypeId"));
    for source in [LAYOUT, CACHE, PLANNING, PAINT, DISPLAYED] {
        assert!(!source.contains("ButtonWidget"));
    }
    assert!(!PAINT.contains("widget_type_id == "));

}

#[test]
fn final_taffy_placement_is_shared_by_text_consumers() {
    assert!(LAYOUT.contains("RunMode::PerformLayout"));
    assert!(LAYOUT.contains("with_paragraph_style(paragraph)"));
    assert!(LAYOUT.contains("self.final_text_origins[index]"));
    assert!(LAYOUT.contains("baselines = text_baselines(artifact, placed_top)"));
    assert!(PLANNING.contains("text_origins,"));
    assert!(CACHE.contains(".text_origins\n            .get(position)"));
    assert!(PAINT.contains("layout.text_origins[mounted_preorder]"));
    assert!(DISPLAYED.contains("self.cache.layout.text_origins.get(position)"));
    assert!(!PAINT.contains("content_width - artifact.width"));
    assert!(!LAYOUT.contains("content_width - artifact.width"));
}
