use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiRuntimeArtifactId(String);

impl UiRuntimeArtifactId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiRuntimeTargetProfile {
    Editor,
    Game,
    WorldSpace,
    #[default]
    Headless,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RuntimeSchemaRef {
    pub schema_id: String,
    pub schema_version: u32,
}

impl RuntimeSchemaRef {
    pub fn from_schema_ref(schema: &UiSchemaRef) -> Self {
        Self {
            schema_id: schema.id.as_str().to_owned(),
            schema_version: schema.version.value(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RuntimeRouteRef {
    pub route_id: String,
    pub payload_schema: RuntimeSchemaRef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePackageRecord {
    pub package_id: String,
    #[serde(default)]
    pub control_kind_ids: Vec<String>,
    #[serde(default)]
    pub kernel_ids: Vec<String>,
    #[serde(default)]
    pub control_node_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeCapabilityRecord {
    pub capability_id: String,
    #[serde(default)]
    pub declared_by_controls: Vec<String>,
    #[serde(default)]
    pub required_by_interactions: Vec<String>,
    #[serde(default)]
    pub required_by_bindings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeArtifactManifest {
    pub artifact_id: UiRuntimeArtifactId,
    pub artifact_version: u32,
    pub target_profile: UiRuntimeTargetProfile,
    pub target_profile_version: u32,
    pub program_id: String,
    pub program_version: u32,
    pub cache_key: ArtifactCacheKey,
    pub packages: Vec<RuntimePackageRecord>,
    pub package_ids: Vec<String>,
    pub control_kind_ids: Vec<String>,
    pub schema_ids: Vec<RuntimeSchemaRef>,
    pub route_ids: Vec<RuntimeRouteRef>,
    pub kernel_ids: Vec<String>,
    pub capability_ids: Vec<String>,
    pub capabilities: Vec<RuntimeCapabilityRecord>,
    pub source_map: CompiledSourceMap,
    pub diagnostics: Vec<UiRuntimeArtifactDiagnostic>,
}

impl UiRuntimeArtifactManifest {
    pub fn from_program(program: &UiProgram) -> Self {
        Self::from_program_for_target(program, UiRuntimeTargetProfile::Headless, 1)
    }

    pub fn from_program_for_target(
        program: &UiProgram,
        target_profile: UiRuntimeTargetProfile,
        target_profile_version: u32,
    ) -> Self {
        let source_map = CompiledSourceMap::from_program(program);
        let packages = runtime_packages(program);
        let package_ids = packages
            .iter()
            .map(|package| package.package_id.to_owned())
            .collect::<Vec<_>>();
        let control_kind_ids = sorted_control_kind_ids(program);
        let schema_ids = sorted_schema_ids(program);
        let route_ids = sorted_route_ids(program);
        let kernel_ids = sorted_kernel_ids(program);
        let capabilities = runtime_capabilities(program);
        let capability_ids = capabilities
            .iter()
            .map(|capability| capability.capability_id.to_owned())
            .collect::<Vec<_>>();
        let cache_key = stable_cache_key(
            program,
            target_profile,
            target_profile_version,
            &package_ids,
            &control_kind_ids,
            &schema_ids,
            &route_ids,
            &kernel_ids,
            &capability_ids,
            &source_map,
        );
        let program_id = program.id.as_str().to_owned();
        let program_version = program.version.value();
        let diagnostics = program
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let source_map_index = diagnostic
                    .source_map
                    .as_ref()
                    .and_then(|entry| source_map.index_for_entry(entry));
                UiRuntimeArtifactDiagnostic {
                    code: diagnostic.code.to_owned(),
                    message: diagnostic.message.to_owned(),
                    severity: diagnostic.severity.into(),
                    source_map_index,
                }
            })
            .collect();

        Self {
            artifact_id: UiRuntimeArtifactId::new(format!("{program_id}@{program_version}")),
            artifact_version: 1,
            target_profile,
            target_profile_version,
            program_id,
            program_version,
            cache_key,
            packages,
            package_ids,
            control_kind_ids,
            schema_ids,
            route_ids,
            kernel_ids,
            capability_ids,
            capabilities,
            source_map,
            diagnostics,
        }
    }

    pub fn push_diagnostic(&mut self, diagnostic: UiRuntimeArtifactDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
}
