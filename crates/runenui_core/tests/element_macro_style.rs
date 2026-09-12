use runenui_core::{Brush, BrushValue, Color, EdgeInsets, LogicalLength, Radius, View, button};

#[test]
fn typed_builder_style_remains_ergonomic() {
    let four = LogicalLength::new(4.0).unwrap_or_default();
    let element = button::<()>("Save")
        .foreground(Color::WHITE)
        .background(Color::BLACK)
        .padding(EdgeInsets::all(four))
        .radius(Radius::all(four))
        .into_element();
    assert_eq!(
        element
            .style()
            .background()
            .and_then(BrushValue::as_literal),
        Some(&Brush::solid(Color::BLACK))
    );
}
