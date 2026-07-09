//! Theme token declaration graph.

use super::ThemeTokenDeclaration;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeTokenGraph {
    pub declarations: Vec<ThemeTokenDeclaration>,
}

impl ThemeTokenGraph {
    pub fn new(declarations: Vec<ThemeTokenDeclaration>) -> Self {
        Self { declarations }
    }
}
