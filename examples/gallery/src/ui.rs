use runenui_core::{
    Color, EdgeInsets, Element, FlexContainerStyle, FlexDirection, LayoutContainer,
    LayoutDimension, LayoutStyle, LogicalLength, OverflowPolicy, OverflowStyle,
    SemanticEditableMode, TextAffinity, TextPosition, TextSelection, View, button,
    checkbox, children, column, row, text, text_field,
};

use crate::app::{GalleryAction, GalleryState, Page};

fn side_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default().with_direction(FlexDirection::Column),
        ))
        .with_width(LayoutDimension::length(LogicalLength::from(140_u16)))
}

fn main_layout() -> LayoutStyle {
    LayoutStyle::default()
        .with_container(LayoutContainer::Flex(
            FlexContainerStyle::default().with_direction(FlexDirection::Column),
        ))
        .with_width(LayoutDimension::Fill)
        .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll))
}

fn padding(amount: u16) -> EdgeInsets {
    EdgeInsets::all(LogicalLength::from(amount))
}

fn basics(state: &GalleryState) -> Element<GalleryAction> {
    column(children![
        text("Button").id("gallery.basics.heading"),
        button("Activate")
            .id("gallery.basics.activate")
            .key("gallery.basics.activate")
            .background(Color::rgb(72, 100, 146))
            .padding(padding(10))
            .on_activate(|| GalleryAction::Increment),
        text(format!("Activations: {}", state.presses))
            .id("gallery.basics.value"),
        button("Disabled").disabled()
            .id("gallery.basics.disabled")
            .background(Color::rgb(65, 65, 75))
            .padding(padding(10)),
        text("Use Tab, Enter, Space, and pointer activation."),
    ])
    .key("gallery.page.basics")
    .gap(12_u16)
    .into_element()
}

fn inputs(state: &GalleryState) -> Element<GalleryAction> {
    let snapshot = state.document_snapshot();
    let caret = TextPosition::new(
        snapshot,
        &state.document,
        state.caret,
        TextAffinity::Upstream,
    )
    .unwrap_or_else(|_| unreachable!("gallery state retains valid UTF-8 caret"));
    let field = text_field(
        snapshot,
        state.document.clone(),
        TextSelection::collapsed(caret),
        SemanticEditableMode::SingleLine,
        GalleryAction::Edit,
    )
    .unwrap_or_else(|_| unreachable!("gallery state retains valid single-line document"));
    column(children![
        text("Checkbox").id("gallery.inputs.heading"),
        checkbox("Enable preview", state.checked)
            .id("gallery.inputs.checkbox")
            .key("gallery.inputs.checkbox")
            .on_activate(|| GalleryAction::Toggle),
        text(format!("Checked: {}", state.checked)),
        text("TextField"),
        field
            .placeholder("Type a value")
            .id("gallery.inputs.textfield")
            .key("gallery.inputs.textfield")
            .background(Color::rgb(50, 57, 73))
            .padding(padding(10)),
        text(format!("Application value: {}", state.document)),
        text("Editing is provided by the ordinary M10 transaction path."),
    ])
    .key("gallery.page.inputs")
    .gap(12_u16)
    .into_element()
}

pub fn root(state: &GalleryState) -> Element<GalleryAction> {
    let page = match state.page {
        Page::Basics => basics(state),
        Page::Inputs => inputs(state),
    };
    row(children![
        column(children![
            text("RunenUI Gallery").id("gallery.title"),
            button("Basics")
                .id("gallery.nav.basics")
                .key("gallery.nav.basics")
                .on_activate(|| GalleryAction::Navigate(Page::Basics)),
            button("Inputs")
                .id("gallery.nav.inputs")
                .key("gallery.nav.inputs")
                .on_activate(|| GalleryAction::Navigate(Page::Inputs)),
        ])
        .key("gallery.navigation")
        .with_layout(side_layout())
        .gap(12_u16),
        column(children![
            text(state.page.title()).id("gallery.current.page"),
            page,
        ])
        .key("gallery.preview")
        .with_layout(main_layout())
        .gap(12_u16),
    ])
    .id("gallery.root")
    .key("gallery.root")
    .with_layout(
        LayoutStyle::default()
            .with_container(LayoutContainer::Flex(
                FlexContainerStyle::default().with_direction(FlexDirection::Row),
            ))
            .with_width(LayoutDimension::Fill)
            .with_height(LayoutDimension::Fill)
            .with_overflow(OverflowStyle::all(OverflowPolicy::Scroll)),
    )
    .foreground(Color::rgb(238, 240, 246))
    .background(Color::rgb(24, 28, 36))
    .padding(padding(16))
    .gap(16_u16)
    .into_element()
}
