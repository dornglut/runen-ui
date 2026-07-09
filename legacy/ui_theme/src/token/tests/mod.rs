//! Theme token resolution tests.

use super::*;
use crate::{ThemeTokens, UiColor};

fn token_value(
    id: &str,
    layer: ThemeTokenLayer,
    value: ThemeTokenValue,
    source: &str,
) -> ThemeTokenDeclaration {
    ThemeTokenDeclaration::value(id, value.family(), layer, value, source)
}

mod activation;
mod alias;
mod diagnostics;
mod packet;
mod precedence;
mod selector;
