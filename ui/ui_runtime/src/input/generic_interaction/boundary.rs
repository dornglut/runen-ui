//! No-bypass boundary assertion counters for generic interaction replay.

/// Boundary counters for no-bypass evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InteractionBoundaryAssertions {
    /// Host command executions observed during replay.
    pub host_commands_executed: u32,

    /// Product/app/editor/game state mutations observed during replay.
    pub product_mutations: u32,

    /// Overlay, popup, dropdown, tooltip, or layering events observed.
    pub overlay_events: u32,

    /// Full text-editing transactions observed during replay.
    pub text_edit_transactions: u32,
}

impl InteractionBoundaryAssertions {
    /// Returns true when replay produced no host/product/overlay/text-edit bypass.
    pub const fn no_bypass_evidence(self) -> bool {
        self.host_commands_executed == 0
            && self.product_mutations == 0
            && self.overlay_events == 0
            && self.text_edit_transactions == 0
    }
}
