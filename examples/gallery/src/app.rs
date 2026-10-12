use runenui_core::{
    EditChangeMap, EditIntent, EditKind, EditResolution, EditSelection, Effects, IntoUpdateOutput,
    NoHostProtocol, TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot,
    TextRange, UiApp, UpdateOutput, View,
};

use crate::ui;

const MAX_GALLERY_HISTORY_ENTRIES: usize = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Basics,
    Inputs,
}

impl Page {
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Basics => "Basics",
            Self::Inputs => "Inputs",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoryEntry {
    document: String,
    selection: EditSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GalleryState {
    pub page: Page,
    pub presses: u32,
    pub checked: bool,
    pub document: String,
    pub document_revision: u64,
    pub selection: EditSelection,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            page: Page::Basics,
            presses: 0,
            checked: false,
            document: String::new(),
            document_revision: 1,
            selection: EditSelection::collapsed("", 0, TextAffinity::Upstream)
                .unwrap_or_else(|_| unreachable!("the empty gallery document has a valid caret")),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
}

impl GalleryState {
    #[must_use]
    pub const fn document_snapshot(&self) -> TextDocumentSnapshot {
        TextDocumentSnapshot::new(
            TextDocumentId::new(1),
            TextDocumentRevision::new(self.document_revision),
        )
    }

    fn record_edit(&mut self, intent: &EditIntent, kind: EditKind) {
        // Durable history belongs only to the application. Keep runtime-owned
        // navigation and transaction sequencing separate from this journal.
        match kind {
            EditKind::Undo => {
                retain_history(
                    &mut self.redo,
                    HistoryEntry {
                        document: self.document.clone(),
                        selection: intent.proposed_selection(),
                    },
                );
                let _ = self.undo.pop();
            }
            EditKind::Redo => {
                retain_history(
                    &mut self.undo,
                    HistoryEntry {
                        document: self.document.clone(),
                        selection: intent.proposed_selection(),
                    },
                );
                let _ = self.redo.pop();
            }
            _ => {
                let inverse = intent.inverse().unwrap_or_else(|| {
                    unreachable!("ordinary edit inverse was checked before mutation")
                });
                retain_history(
                    &mut self.undo,
                    HistoryEntry {
                        document: self.document.clone(),
                        selection: inverse.selection(),
                    },
                );
                self.redo.clear();
            }
        }
    }
}

fn retain_history(history: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    history.push(entry);
    if history.len() > MAX_GALLERY_HISTORY_ENTRIES {
        let _ = history.remove(0);
    }
}

#[derive(Clone, Debug)]
pub enum GalleryAction {
    Navigate(Page),
    Increment,
    Toggle,
    Edit(Box<EditIntent>),
}

pub struct GalleryApp;

impl UiApp for GalleryApp {
    type State = GalleryState;
    type Action = GalleryAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        ui::root(state)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            GalleryAction::Navigate(page) => {
                state.page = page;
                UpdateOutput::effects(Effects::none())
            }
            GalleryAction::Increment => {
                state.presses = state.presses.saturating_add(1);
                UpdateOutput::effects(Effects::none())
            }
            GalleryAction::Toggle => {
                state.checked = !state.checked;
                UpdateOutput::effects(Effects::none())
            }
            GalleryAction::Edit(intent) => {
                let request = intent.request().clone();
                let snapshot = state.document_snapshot();
                let span = intent.replacement();
                let kind = intent.kind();

                // A queued M10 successor still carries the original document
                // snapshot. Its runtime-issued predecessor links it to the
                // already-resolved earlier transaction in the same FIFO.
                let correlated_snapshot = span.snapshot().document() == snapshot.document()
                    && (span.snapshot() == snapshot || intent.predecessor().is_some());
                if !correlated_snapshot
                    || span.start() > span.end()
                    || span.end() > state.document.len()
                    || !state.document.is_char_boundary(span.start())
                    || !state.document.is_char_boundary(span.end())
                {
                    return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                }

                let history = match kind {
                    EditKind::Undo => Some(&state.undo),
                    EditKind::Redo => Some(&state.redo),
                    _ => None,
                };
                if history.is_some_and(Vec::is_empty) {
                    // Empty application-owned history is a successful no-op.
                    return UpdateOutput::edit(EditResolution::accepted(request, snapshot));
                }

                let Some(revision) = state.document_revision.checked_add(1) else {
                    return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                };
                let next_snapshot = TextDocumentSnapshot::new(
                    snapshot.document(),
                    TextDocumentRevision::new(revision),
                );

                let (next_document, next_selection) = match kind {
                    EditKind::Undo | EditKind::Redo => {
                        let source = if matches!(kind, EditKind::Undo) {
                            &state.undo
                        } else {
                            &state.redo
                        };
                        let entry = source.last().unwrap_or_else(|| {
                            unreachable!("empty history was handled before revision admission")
                        });
                        (entry.document.clone(), entry.selection)
                    }
                    _ => {
                        if intent
                            .replacement_text()
                            .chars()
                            .any(runenui_core::TextNewlinePolicy::is_hard_line_break)
                            || intent.inverse().is_none()
                        {
                            return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                        }
                        let mut changed = state.document.clone();
                        changed.replace_range(span.start()..span.end(), intent.replacement_text());
                        (changed, intent.proposed_selection())
                    }
                };
                if next_selection.bind(next_snapshot, &next_document).is_err() {
                    return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                }

                let resolution = if matches!(kind, EditKind::Undo | EditKind::Redo) {
                    let entire_document =
                        TextRange::new(snapshot, &state.document, 0, state.document.len())
                            .unwrap_or_else(|_| {
                                unreachable!("the entire UTF-8 document is a valid range")
                            });
                    EditResolution::transformed(
                        request,
                        next_snapshot,
                        EditChangeMap::new(entire_document, next_document.len()),
                    )
                } else {
                    EditResolution::accepted(request, next_snapshot)
                };

                state.record_edit(&intent, kind);

                state.document = next_document;
                state.document_revision = revision;
                state.selection = next_selection;
                UpdateOutput::edit(resolution)
            }
        }
    }
}
