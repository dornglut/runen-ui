use runenui_core::{Element, ElementKind, IntoElement, button, children, column, text};

#[test]
fn zero_one_many_iterator_conditional_and_nested_children_are_supported() {
    let zero: Element<()> = column(Vec::<Element<()>>::new()).into_element();
    let one: Element<()> = column([text("one")]).into_element();
    let iterator: Element<()> = column((0..32).map(|index| text(index.to_string()))).into_element();
    let conditional: Element<()> = column(Some(button("optional"))).into_element();
    let nested: Element<()> =
        column(children![column([text("nested")]), button("end")]).into_element();

    let lengths = [zero, one, iterator, conditional, nested].map(|element| match element.kind() {
        ElementKind::Container(container) => container.children().len(),
        _ => 0,
    });
    assert_eq!(lengths, [0, 1, 32, 1, 2]);
}
