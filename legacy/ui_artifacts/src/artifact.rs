use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeArtifact {
    pub manifest: UiRuntimeArtifactManifest,
    pub tables: UiRuntimeArtifactTables,
}

impl UiRuntimeArtifact {
    pub fn new(manifest: UiRuntimeArtifactManifest, tables: UiRuntimeArtifactTables) -> Self {
        Self { manifest, tables }
    }

    pub fn from_program(program: &UiProgram) -> Self {
        let manifest = UiRuntimeArtifactManifest::from_program(program);
        let tables = UiRuntimeArtifactTables::from_program(program, &manifest.source_map);
        Self { manifest, tables }
    }
}
