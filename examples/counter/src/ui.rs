use runenui_core::{Color, Element, View, button, children, column, row, text};

use crate::app::{Counter, CounterAction};

const SCREEN_BACKGROUND: Color = Color::rgb(24, 28, 36);
const CONTROL_BACKGROUND: Color = Color::rgb(54, 64, 82);
const RESET_BACKGROUND: Color = Color::rgb(92, 58, 58);
const WIN_BACKGROUND: Color = Color::rgb(38, 82, 58);

fn count_background(count: i32) -> Color {
    let step = u8::try_from(count.rem_euclid(10)).unwrap_or_default();
    Color::rgb(40_u8.saturating_add(step.saturating_mul(12)), 56, 104)
}

struct CounterScreen;

impl CounterScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        column(children![
            text("Counter").id("counter.title"),
            text(counter.count.to_string())
                .id("counter.value")
                .background(count_background(counter.count))
                .padding(8_u16),
            row(children![
                button("-")
                    .id("counter.decrement")
                    .key("counter.decrement")
                    .background(CONTROL_BACKGROUND)
                    .padding(8_u16)
                    .on_activate(|| CounterAction::Decrement),
                button("+")
                    .id("counter.increment")
                    .key("counter.increment")
                    .background(CONTROL_BACKGROUND)
                    .padding(8_u16)
                    .on_activate(|| CounterAction::Increment),
                button("Reset")
                    .id("counter.reset")
                    .key("counter.reset")
                    .background(RESET_BACKGROUND)
                    .padding(8_u16)
                    .on_activate(|| CounterAction::Reset),
            ])
            .key("counter.controls")
            .gap(8_u16),
        ])
        .key("counter.screen")
        .background(SCREEN_BACKGROUND)
        .padding(16_u16)
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
            text(format!("Count: {count}"))
                .id("counter.value")
                .background(count_background(count))
                .padding(8_u16),
            button("Reset")
                .id("counter.reset")
                .key("counter.reset")
                .background(RESET_BACKGROUND)
                .padding(8_u16)
                .on_activate(|| CounterAction::Reset),
        ])
        .key("win.screen")
        .background(WIN_BACKGROUND)
        .padding(16_u16)
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
