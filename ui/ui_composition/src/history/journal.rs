use serde::{Deserialize, Serialize};

use crate::{
    AuthorizedTransaction, CompositionCommand, CompositionDiagnosticCode as Code,
    CompositionDiagnosticRecord as Record, CompositionDiagnosticStage as Stage,
    CompositionDiagnosticSubject as Subject, CompositionPolicies, CompositionRejection,
    CompositionState, CompositionTransaction, CompositionTransactionId, StateRevision,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionJournalEntry {
    transaction: CompositionTransaction,
    inverse_commands: Vec<CompositionCommand>,
    applied_revision: StateRevision,
}

impl CompositionJournalEntry {
    pub(crate) fn new(
        transaction: CompositionTransaction,
        inverse_commands: Vec<CompositionCommand>,
        applied_revision: StateRevision,
    ) -> Self {
        Self {
            transaction,
            inverse_commands,
            applied_revision,
        }
    }
    pub fn transaction(&self) -> &CompositionTransaction {
        &self.transaction
    }
    pub const fn applied_revision(&self) -> StateRevision {
        self.applied_revision
    }
    pub fn inverse_commands(&self) -> &[CompositionCommand] {
        &self.inverse_commands
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompositionHistory {
    journal: Vec<CompositionJournalEntry>,
    undo: Vec<usize>,
    redo: Vec<usize>,
}

impl CompositionHistory {
    pub(crate) fn record(&mut self, entry: CompositionJournalEntry) {
        let index = self.journal.len();
        self.journal.push(entry);
        self.undo.push(index);
        self.redo.clear();
    }
    pub fn journal(&self) -> &[CompositionJournalEntry] {
        &self.journal
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

impl CompositionState {
    pub fn undo(
        &mut self,
        id: CompositionTransactionId,
        policies: CompositionPolicies<'_>,
    ) -> Result<crate::CompositionCommit, CompositionRejection> {
        let entry_index = *self
            .history
            .undo
            .last()
            .ok_or_else(|| unavailable(id, "There is no structural transaction to undo."))?;
        let transaction = CompositionTransaction::new(
            id,
            self.revision,
            self.history.journal[entry_index].inverse_commands.clone(),
        );
        let authorized: AuthorizedTransaction = self.authorize_history(transaction, policies)?;
        let commit = self.apply_authorized_mode(authorized, false)?;
        self.history.undo.pop();
        self.history.redo.push(entry_index);
        Ok(commit)
    }

    pub fn redo(
        &mut self,
        id: CompositionTransactionId,
        policies: CompositionPolicies<'_>,
    ) -> Result<crate::CompositionCommit, CompositionRejection> {
        let entry_index = *self
            .history
            .redo
            .last()
            .ok_or_else(|| unavailable(id, "There is no structural transaction to redo."))?;
        let transaction = CompositionTransaction::new(
            id,
            self.revision,
            self.history.journal[entry_index]
                .transaction
                .commands_cloned(),
        );
        let authorized = self.authorize_history(transaction, policies)?;
        let commit = self.apply_authorized_mode(authorized, false)?;
        self.history.redo.pop();
        self.history.undo.push(entry_index);
        Ok(commit)
    }
}

fn unavailable(id: CompositionTransactionId, message: &'static str) -> CompositionRejection {
    CompositionRejection::single(Record::error(
        Code::HistoryUnavailable,
        Stage::History,
        Subject::Transaction(id),
        message,
    ))
}
