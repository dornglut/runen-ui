use runenui_core::{Element, View, button, children, column, row, text};

use crate::app::{Counter, CounterAction};

struct CounterScreen;

impl CounterScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        column(children![
            text("Counter").id("counter.title"),
            text(counter.count.to_string()).id("counter.value"),
            row(children![
                button("-")
                    .id("counter.decrement")
                    .key("counter.decrement")
                    .on_activate(|| CounterAction::Decrement),
                button("+")
                    .id("counter.increment")
                    .key("counter.increment")
                    .on_activate(|| CounterAction::Increment),
                button("Reset")
                    .id("counter.reset")
                    .key("counter.reset")
                    .on_activate(|| CounterAction::Reset),
            ])
            .key("counter.controls")
            .gap(8_u16),
        ])
        .key("counter.screen")
        .gap(8_u16)
        .into_element()
    }
}

struct WinScreen;

impl WinScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        let count = counter.count;

        column(children![
            text("You win").id("counter.win.title"),
            text(format!("Count: {count}")).id("counter.value"),
            button("Reset")
                .id("counter.reset")
                .key("counter.reset")
                .on_activate(|| CounterAction::Reset),
        ])
        .key("win.screen")
        .gap(8_u16)
        .into_element()
    }
}

pub fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.has_won() {
        WinScreen::root(counter)
    } else {
        CounterScreen::root(counter)
    }
}
