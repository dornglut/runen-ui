use runenui_core::{Element, element};

use crate::app::{Counter, CounterAction};

struct CounterScreen;

impl CounterScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        element! {
            column gap=8_u16 {
                text "Counter" id="counter.title"
                text { counter.count.to_string() } id="counter.value"

                row gap=8_u16 {
                    button "-" id="counter.decrement" action=CounterAction::Decrement
                    button "+" id="counter.increment" action=CounterAction::Increment
                    button "Reset" id="counter.reset" action=CounterAction::Reset
                }
            }
        }
    }
}

struct WinScreen;

impl WinScreen {
    fn root(counter: &Counter) -> Element<CounterAction> {
        let count = counter.count;

        element! {
            column gap=8_u16 {
                text "You win" id="counter.win.title"
                text { format!("Count: {count}") } id="counter.value"
                button "Reset" id="counter.reset" action=CounterAction::Reset
            }
        }
    }
}

pub fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.has_won() {
        WinScreen::root(counter)
    } else {
        CounterScreen::root(counter)
    }
}
