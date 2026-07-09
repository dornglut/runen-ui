//! File: domain/ui/ui_controls/src/base_control/contribution.rs
//! Crate: ui_controls

use crate::ControlKindId;

use super::{ControlDef, ControlPreset};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiControls {
    contributions: Vec<ControlContribution>,
}

impl UiControls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, contribution: ControlContribution) {
        self.contributions.push(contribution);
    }

    pub fn with_contribution(mut self, contribution: ControlContribution) -> Self {
        self.add(contribution);
        self
    }

    pub fn contributions(&self) -> &[ControlContribution] {
        &self.contributions
    }

    pub fn len(&self) -> usize {
        self.contributions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contributions.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlContribution {
    def: ControlDef,
}

impl ControlContribution {
    pub fn new(def: ControlDef) -> Self {
        Self { def }
    }

    pub fn def(&self) -> &ControlDef {
        &self.def
    }

    pub fn control_kind_id(&self) -> ControlKindId {
        self.def.control_kind_id()
    }

    pub fn kind_suffix(&self) -> &str {
        self.def.kind_suffix()
    }

    pub fn preset(&self) -> ControlPreset {
        self.def.preset()
    }
}
