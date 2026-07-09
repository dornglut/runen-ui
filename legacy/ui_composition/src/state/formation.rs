use std::collections::BTreeSet;

use crate::{
    CompositionDefinitionV1, CompositionHistory, CompositionRejection, CompositionTransactionId,
    StateRevision,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionState {
    pub(crate) definition: CompositionDefinitionV1,
    pub(crate) revision: StateRevision,
    pub(crate) applied_transactions: BTreeSet<CompositionTransactionId>,
    pub(crate) history: CompositionHistory,
}

impl CompositionState {
    pub fn form(definition: CompositionDefinitionV1) -> Result<Self, CompositionRejection> {
        let diagnostics = crate::validation::validate_definition(&definition);
        if !diagnostics.is_empty() {
            return Err(CompositionRejection::new(diagnostics));
        }
        let definition = definition.normalized();
        Ok(Self {
            revision: StateRevision::new(definition.revision().raw()),
            definition,
            applied_transactions: BTreeSet::new(),
            history: CompositionHistory::default(),
        })
    }

    pub const fn revision(&self) -> StateRevision {
        self.revision
    }
    pub fn snapshot(&self) -> crate::CompositionSnapshot<'_> {
        crate::CompositionSnapshot::new(self)
    }
    pub fn definition(&self) -> &CompositionDefinitionV1 {
        &self.definition
    }
    pub fn history(&self) -> &CompositionHistory {
        &self.history
    }
    pub fn applied_transaction_ids(&self) -> impl Iterator<Item = CompositionTransactionId> + '_ {
        self.applied_transactions.iter().copied()
    }
}
