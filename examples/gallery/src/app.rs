use runenui_core::{
    EditIntent, EditKind, EditResolution, Effects, NoHostProtocol, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, UiApp, UpdateOutput, View,
};

use crate::ui;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Basics,
    Inputs,
}

impl Page {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Basics => "Basics",
            Self::Inputs => "Inputs",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GalleryState {
    pub page: Page,
    pub presses: u32,
    pub checked: bool,
    pub document: String,
    pub document_revision: u64,
    pub caret: usize,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            page: Page::Basics,
            presses: 0,
            checked: false,
            document: String::new(),
            document_revision: 1,
            caret: 0,
        }
    }
}

impl GalleryState {
    pub const fn document_snapshot(&self) -> TextDocumentSnapshot {
        TextDocumentSnapshot::new(
            TextDocumentId::new(1),
            TextDocumentRevision::new(self.document_revision),
        )
    }
}

#[derive(Clone, Debug)]
pub enum GalleryAction {
    Navigate(Page),
    Increment,
    Toggle,
    Edit(EditIntent),
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
    ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
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
                let valid = !matches!(intent.kind(), EditKind::Undo | EditKind::Redo)
                    && span.snapshot() == snapshot
                    && span.start() <= span.end()
                    && span.end() <= state.document.len()
                    && state.document.is_char_boundary(span.start())
                    && state.document.is_char_boundary(span.end())
                    && !intent.replacement_text().chars().any(
                        runenui_core::TextNewlinePolicy::is_hard_line_break
                    );
                let Some(revision) = state.document_revision.checked_add(1) else {
                    return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                };
                if !valid {
                    return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                }
                let mut changed = state.document.clone();
                changed.replace_range(span.start()..span.end(), intent.replacement_text());
                let caret = intent.proposed_selection().active();
                if caret > changed.len() || !changed.is_char_boundary(caret) {
                    return UpdateOutput::edit(EditResolution::rejected(request, snapshot));
                }
                state.document = changed;
                state.document_revision = revision;
                state.caret = caret;
                UpdateOutput::edit(EditResolution::accepted(request, state.document_snapshot()))
            }
        }
    }
}
