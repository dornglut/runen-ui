//! High-level architecture fixture wrapper.

use serde::{Deserialize, Serialize};
use ui_artifacts::UiRuntimeArtifact;
use ui_program::UiProgram;

use crate::{HeadlessFixture, HeadlessFixtureRun};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiArchitectureFixture {
    pub fixture_id: String,
    pub headless: HeadlessFixture,
}

impl UiArchitectureFixture {
    pub fn minimal(fixture_id: impl Into<String>) -> Self {
        let fixture_id = fixture_id.into();
        Self {
            headless: HeadlessFixture::label_inspector(fixture_id.clone()),
            fixture_id,
        }
    }

    pub fn program(&self) -> &UiProgram {
        &self.headless.program
    }

    pub fn compile(&self) -> UiRuntimeArtifact {
        self.headless.compile()
    }

    pub fn run(&self) -> HeadlessFixtureRun {
        self.headless.run()
    }
}
