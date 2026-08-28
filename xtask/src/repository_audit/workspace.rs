use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use super::{Finding, path_text};

const ROOT_MANIFEST: &str = "Cargo.toml";
const WORKSPACE_STRUCTURE_PATH: &str = "docs/architecture/workspace-structure.md";
const CORE_PACKAGE: &str = "runenui_core";
const RUNTIME_PACKAGE: &str = "runenui_runtime";
const RENDER_WGPU_PACKAGE: &str = "runenui_render_wgpu";
const TESTING_PACKAGE: &str = "runenui_testing";
const REFERENCE_WINIT_PACKAGE: &str = "reference_winit";
const EXTERNAL_WIDGET_PACKAGE: &str = "runenui_external_widget_conformance";
const XTASK_PACKAGE: &str = "xtask";
const RENDER_WGPU_FORBIDDEN_DEPENDENCIES: &[&str] =
    &["winit", "accesskit", "accesskit_winit", "raw-window-handle"];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct WorkspaceMetrics {
    pub(super) members: usize,
    pub(super) production_crates: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceMember {
    relative: PathBuf,
    package: String,
    dependencies: BTreeSet<String>,
    dev_dependencies: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DependencySection {
    Production,
    Dev,
}

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<WorkspaceMetrics, String> {
    let root_manifest = read(root, ROOT_MANIFEST)?;
    let member_paths = parse_workspace_members(&root_manifest)
        .ok_or_else(|| format!("failed to parse [workspace].members from {ROOT_MANIFEST}"))?;

    let mut members = Vec::new();
    let mut package_names = BTreeSet::new();

    for relative in member_paths {
        let manifest_relative = relative.join("Cargo.toml");
        let manifest_path = root.join(&manifest_relative);
        if !manifest_path.is_file() {
            findings.push(Finding::fatal(
                "workspace.member_missing_manifest",
                Some(path_text(&relative)),
                "workspace member does not contain Cargo.toml",
            ));
            continue;
        }

        let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
            format!(
                "failed to read workspace member manifest {}: {error}",
                manifest_relative.display()
            )
        })?;
        let Some(package) = parse_package_name(&manifest) else {
            findings.push(Finding::fatal(
                "workspace.member_missing_package_name",
                Some(path_text(&manifest_relative)),
                "workspace member manifest does not define [package].name",
            ));
            continue;
        };
        if !package_names.insert(package.clone()) {
            findings.push(Finding::fatal(
                "workspace.duplicate_package_name",
                Some(path_text(&manifest_relative)),
                format!("workspace package name `{package}` is duplicated"),
            ));
        }
        if package == TESTING_PACKAGE && manifest.contains("internal-test-seams") {
            findings.push(Finding::fatal(
                "workspace.testing_internal_seam_dependency",
                Some(path_text(&manifest_relative)),
                "runenui_testing must consume ordinary public runtime APIs and must not enable or mention `internal-test-seams` in its manifest",
            ));
        }

        let (dependencies, dev_dependencies) = parse_dependency_names(&manifest);
        if package == RENDER_WGPU_PACKAGE {
            validate_renderer_external_dependencies(
                &relative,
                &dependencies,
                &dev_dependencies,
                findings,
            );
        }
        members.push(WorkspaceMember {
            relative,
            package,
            dependencies,
            dev_dependencies,
        });
    }

    let documented = documented_package_names(&read(root, WORKSPACE_STRUCTURE_PATH)?);
    for member in &members {
        if !documented.contains(&member.package) {
            findings.push(Finding::fatal(
                "workspace.undocumented_member",
                Some(path_text(&member.relative)),
                format!(
                    "workspace package `{}` is not listed in {WORKSPACE_STRUCTURE_PATH}",
                    member.package
                ),
            ));
        }
    }
    for package in documented.difference(&package_names) {
        findings.push(Finding::fatal(
            "workspace.documented_member_missing",
            Some(WORKSPACE_STRUCTURE_PATH.to_owned()),
            format!("documented package `{package}` is not a workspace member"),
        ));
    }

    let member_map = members
        .iter()
        .map(|member| (member.package.as_str(), member))
        .collect::<BTreeMap<_, _>>();
    for member in &members {
        validate_dependency_direction(member, &member_map, findings);
    }

    let production_crates = members
        .iter()
        .filter(|member| member.relative.starts_with("crates"))
        .count();

    Ok(WorkspaceMetrics {
        members: members.len(),
        production_crates,
    })
}

fn workspace_dependency_set<'a>(
    dependencies: &'a BTreeSet<String>,
    members: &BTreeMap<&str, &WorkspaceMember>,
) -> BTreeSet<&'a str> {
    dependencies
        .iter()
        .filter(|dependency| members.contains_key(dependency.as_str()))
        .map(String::as_str)
        .collect()
}

fn validate_dependency_direction(
    member: &WorkspaceMember,
    members: &BTreeMap<&str, &WorkspaceMember>,
    findings: &mut Vec<Finding>,
) {
    let workspace_dependencies = workspace_dependency_set(&member.dependencies, members);
    let workspace_dev_dependencies = workspace_dependency_set(&member.dev_dependencies, members);

    let allowed = match member.package.as_str() {
        CORE_PACKAGE | XTASK_PACKAGE => BTreeSet::new(),
        RUNTIME_PACKAGE => BTreeSet::from([CORE_PACKAGE]),
        REFERENCE_WINIT_PACKAGE => {
            BTreeSet::from([CORE_PACKAGE, RUNTIME_PACKAGE, RENDER_WGPU_PACKAGE])
        }
        RENDER_WGPU_PACKAGE | TESTING_PACKAGE | "counter" | EXTERNAL_WIDGET_PACKAGE => {
            BTreeSet::from([CORE_PACKAGE, RUNTIME_PACKAGE])
        }
        package if member.relative.starts_with("crates") => {
            findings.push(Finding::fatal(
                "workspace.production_package_unclassified",
                Some(path_text(&member.relative)),
                format!("production package `{package}` has no reviewed dependency-direction rule"),
            ));
            BTreeSet::new()
        }
        _ => BTreeSet::from([CORE_PACKAGE, RUNTIME_PACKAGE]),
    };

    for dependency in workspace_dependencies.difference(&allowed) {
        findings.push(Finding::fatal(
            "workspace.forbidden_dependency_direction",
            Some(path_text(&member.relative.join("Cargo.toml"))),
            format!(
                "workspace package `{}` must not depend on workspace package `{dependency}`",
                member.package
            ),
        ));
    }

    let mut allowed_dev = allowed.clone();
    if member.package == EXTERNAL_WIDGET_PACKAGE {
        allowed_dev.insert(TESTING_PACKAGE);
    }
    for dependency in workspace_dev_dependencies.difference(&allowed_dev) {
        findings.push(Finding::fatal(
            "workspace.forbidden_dev_dependency_direction",
            Some(path_text(&member.relative.join("Cargo.toml"))),
            format!(
                "workspace package `{}` must not dev-depend on workspace package `{dependency}`",
                member.package
            ),
        ));
    }

    if member.package == RUNTIME_PACKAGE && !workspace_dependencies.contains(CORE_PACKAGE) {
        findings.push(Finding::fatal(
            "workspace.runtime_core_dependency_missing",
            Some(path_text(&member.relative.join("Cargo.toml"))),
            "runenui_runtime must depend on runenui_core",
        ));
    }

    if member.package == RENDER_WGPU_PACKAGE {
        for dependency in [CORE_PACKAGE, RUNTIME_PACKAGE] {
            if !workspace_dependencies.contains(dependency) {
                findings.push(Finding::fatal(
                    "workspace.renderer_public_dependency_missing",
                    Some(path_text(&member.relative.join("Cargo.toml"))),
                    format!(
                        "runenui_render_wgpu must depend on public workspace package `{dependency}`"
                    ),
                ));
            }
        }
    }

    if member.package == TESTING_PACKAGE {
        for dependency in [CORE_PACKAGE, RUNTIME_PACKAGE] {
            if !workspace_dependencies.contains(dependency) {
                findings.push(Finding::fatal(
                    "workspace.testing_public_dependency_missing",
                    Some(path_text(&member.relative.join("Cargo.toml"))),
                    format!(
                        "runenui_testing must depend on public workspace package `{dependency}`"
                    ),
                ));
            }
        }
    }

    if member.package == REFERENCE_WINIT_PACKAGE
        && !workspace_dependencies.contains(RENDER_WGPU_PACKAGE)
    {
        findings.push(Finding::fatal(
            "workspace.reference_host_renderer_dependency_missing",
            Some(path_text(&member.relative.join("Cargo.toml"))),
            "reference_winit must consume runenui_render_wgpu as the standalone M7 native host",
        ));
    }
}

fn validate_renderer_external_dependencies(
    relative: &Path,
    dependencies: &BTreeSet<String>,
    dev_dependencies: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    for forbidden in RENDER_WGPU_FORBIDDEN_DEPENDENCIES {
        if dependencies.contains(*forbidden) || dev_dependencies.contains(*forbidden) {
            findings.push(Finding::fatal(
                "workspace.renderer_forbidden_host_dependency",
                Some(path_text(&relative.join("Cargo.toml"))),
                format!(
                    "runenui_render_wgpu must not depend on native host/accessibility package `{forbidden}`"
                ),
            ));
        }
    }
}

fn parse_workspace_members(contents: &str) -> Option<Vec<PathBuf>> {
    let mut in_workspace = false;
    let mut collecting = false;
    let mut members = Vec::new();

    for line in contents.lines() {
        let trimmed = strip_comment(line).trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') && !collecting {
            in_workspace = trimmed == "[workspace]";
            continue;
        }
        if !in_workspace && !collecting {
            continue;
        }

        if !collecting {
            if !trimmed.starts_with("members") || !trimmed.contains('[') {
                continue;
            }
            collecting = true;
        }

        for value in quoted_values(trimmed) {
            members.push(PathBuf::from(value));
        }
        if trimmed.contains(']') {
            return Some(members);
        }
    }

    None
}

fn parse_package_name(contents: &str) -> Option<String> {
    let mut in_package = false;
    for line in contents.lines() {
        let trimmed = strip_comment(line).trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package
            && let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == "name"
        {
            return quoted_values(value).into_iter().next();
        }
    }
    None
}

fn parse_dependency_names(contents: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut production = BTreeSet::new();
    let mut dev = BTreeSet::new();
    let mut section = None;

    for line in contents.lines() {
        let trimmed = strip_comment(line).trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = dependency_section(trimmed);
            continue;
        }
        let Some(section) = section else {
            continue;
        };
        if trimmed.is_empty() {
            continue;
        }
        if let Some((name, value)) = trimmed.split_once('=') {
            let name = name.trim().trim_matches('"');
            if !name.is_empty() {
                let package = dependency_package_name(name, value);
                match section {
                    DependencySection::Production => {
                        production.insert(package);
                    }
                    DependencySection::Dev => {
                        dev.insert(package);
                    }
                }
            }
        }
    }

    (production, dev)
}

fn dependency_package_name(name: &str, value: &str) -> String {
    let package = value
        .trim()
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .and_then(|fields| {
            fields.split(',').find_map(|field| {
                let (key, value) = field.split_once('=')?;
                if key.trim() == "package" {
                    quoted_values(value).into_iter().next()
                } else {
                    None
                }
            })
        });

    package.unwrap_or_else(|| name.to_owned())
}

fn dependency_section(header: &str) -> Option<DependencySection> {
    if header == "[dev-dependencies]" || header.ends_with(".dev-dependencies]") {
        Some(DependencySection::Dev)
    } else if matches!(header, "[dependencies]" | "[build-dependencies]")
        || header.ends_with(".dependencies]")
        || header.ends_with(".build-dependencies]")
    {
        Some(DependencySection::Production)
    } else {
        None
    }
}

fn documented_package_names(contents: &str) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();
    let mut in_package_table = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("| Package |") {
            in_package_table = true;
            continue;
        }
        if !in_package_table {
            continue;
        }
        if !trimmed.starts_with('|') {
            break;
        }
        if trimmed
            .chars()
            .all(|character| matches!(character, '|' | '-' | ':' | ' '))
        {
            continue;
        }
        let first_cell = trimmed
            .trim_matches('|')
            .split('|')
            .next()
            .map(str::trim)
            .unwrap_or_default();
        if let Some(package) = first_cell
            .strip_prefix('`')
            .and_then(|value| value.strip_suffix('`'))
        {
            packages.insert(package.to_owned());
        }
    }

    packages
}

fn quoted_values(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = line;

    while let Some(start) = remaining.find('"') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        values.push(after_start[..end].to_owned());
        remaining = &after_start[end + 1..];
    }

    values
}

fn strip_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or(line)
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        CORE_PACKAGE, EXTERNAL_WIDGET_PACKAGE, REFERENCE_WINIT_PACKAGE, RENDER_WGPU_PACKAGE,
        RUNTIME_PACKAGE, TESTING_PACKAGE, WorkspaceMember, documented_package_names,
        parse_dependency_names, parse_package_name, parse_workspace_members,
        validate_dependency_direction, validate_renderer_external_dependencies,
    };

    fn member(
        relative: &str,
        package: &str,
        dependencies: BTreeSet<String>,
        dev_dependencies: BTreeSet<String>,
    ) -> WorkspaceMember {
        WorkspaceMember {
            relative: relative.into(),
            package: package.to_owned(),
            dependencies,
            dev_dependencies,
        }
    }

    #[test]
    fn workspace_member_parser_preserves_declared_order() {
        let members = parse_workspace_members(
            "[workspace]\nmembers = [\n \"crates/core\",\n # \"crates/ignored\",\n \"xtask\",\n]\n",
        );
        assert_eq!(members, Some(vec!["crates/core".into(), "xtask".into()]));
    }

    #[test]
    fn manifest_parser_distinguishes_production_and_dev_dependencies() {
        let manifest = "[package]\nname = \"runtime\"\n[dependencies]\ncore_alias = { package = \"core\", path = \"../core\" }\n[dev-dependencies]\nfixture = \"1\"\n";
        assert_eq!(parse_package_name(manifest).as_deref(), Some("runtime"));
        assert_eq!(
            parse_dependency_names(manifest),
            (
                BTreeSet::from(["core".to_owned()]),
                BTreeSet::from(["fixture".to_owned()])
            )
        );
    }

    #[test]
    fn renamed_dependency_uses_canonical_identity_for_direction_checks() {
        let (dependencies, dev_dependencies) = parse_dependency_names(
            "[dependencies]\nruntime_alias = { package = \"runenui_runtime\", path = \"../runenui_runtime\" }\n",
        );
        let core = member(
            "crates/runenui_core",
            CORE_PACKAGE,
            dependencies,
            dev_dependencies,
        );
        let runtime = member(
            "crates/runenui_runtime",
            RUNTIME_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
        ]);
        let mut findings = Vec::new();

        validate_dependency_direction(&core, &members, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "workspace.forbidden_dependency_direction")
        );
    }

    #[test]
    fn testing_package_requires_public_core_runtime_production_dependencies() {
        let core = member(
            "crates/runenui_core",
            CORE_PACKAGE,
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let runtime = member(
            "crates/runenui_runtime",
            RUNTIME_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let testing = member(
            "crates/runenui_testing",
            TESTING_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned(), RUNTIME_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (testing.package.as_str(), &testing),
        ]);
        let mut findings = Vec::new();

        validate_dependency_direction(&testing, &members, &mut findings);

        assert!(findings.is_empty());
    }

    #[test]
    fn renderer_requires_public_core_runtime_and_rejects_host_accessibility_dependencies() {
        let core = member(
            "crates/runenui_core",
            CORE_PACKAGE,
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let runtime = member(
            "crates/runenui_runtime",
            RUNTIME_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let renderer = member(
            "crates/runenui_render_wgpu",
            RENDER_WGPU_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned(), RUNTIME_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (renderer.package.as_str(), &renderer),
        ]);
        let mut findings = Vec::new();
        validate_dependency_direction(&renderer, &members, &mut findings);
        validate_renderer_external_dependencies(
            &renderer.relative,
            &renderer.dependencies,
            &renderer.dev_dependencies,
            &mut findings,
        );
        assert!(findings.is_empty());

        let missing_runtime = member(
            "crates/runenui_render_wgpu",
            RENDER_WGPU_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let missing_members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (missing_runtime.package.as_str(), &missing_runtime),
        ]);
        let mut missing_findings = Vec::new();
        validate_dependency_direction(&missing_runtime, &missing_members, &mut missing_findings);
        assert!(
            missing_findings
                .iter()
                .any(|finding| finding.code == "workspace.renderer_public_dependency_missing")
        );

        let mut forbidden_findings = Vec::new();
        validate_renderer_external_dependencies(
            &renderer.relative,
            &BTreeSet::from([
                CORE_PACKAGE.to_owned(),
                RUNTIME_PACKAGE.to_owned(),
                "winit".to_owned(),
                "raw-window-handle".to_owned(),
            ]),
            &BTreeSet::from(["accesskit".to_owned()]),
            &mut forbidden_findings,
        );
        assert_eq!(
            forbidden_findings
                .iter()
                .filter(|finding| finding.code == "workspace.renderer_forbidden_host_dependency")
                .count(),
            3
        );
    }

    #[test]
    fn reference_winit_may_consume_renderer_but_generic_examples_may_not() {
        let core = member(
            "crates/runenui_core",
            CORE_PACKAGE,
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let runtime = member(
            "crates/runenui_runtime",
            RUNTIME_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let renderer = member(
            "crates/runenui_render_wgpu",
            RENDER_WGPU_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned(), RUNTIME_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let reference = member(
            "examples/reference_winit",
            REFERENCE_WINIT_PACKAGE,
            BTreeSet::from([
                CORE_PACKAGE.to_owned(),
                RUNTIME_PACKAGE.to_owned(),
                RENDER_WGPU_PACKAGE.to_owned(),
            ]),
            BTreeSet::new(),
        );
        let members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (renderer.package.as_str(), &renderer),
            (reference.package.as_str(), &reference),
        ]);
        let mut findings = Vec::new();
        validate_dependency_direction(&reference, &members, &mut findings);
        assert!(findings.is_empty());

        let missing_renderer = member(
            "examples/reference_winit",
            REFERENCE_WINIT_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned(), RUNTIME_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let missing_members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (renderer.package.as_str(), &renderer),
            (missing_renderer.package.as_str(), &missing_renderer),
        ]);
        let mut missing_findings = Vec::new();
        validate_dependency_direction(&missing_renderer, &missing_members, &mut missing_findings);
        assert!(missing_findings.iter().any(|finding| {
            finding.code == "workspace.reference_host_renderer_dependency_missing"
        }));

        let generic = member(
            "examples/other",
            "other",
            BTreeSet::from([
                CORE_PACKAGE.to_owned(),
                RUNTIME_PACKAGE.to_owned(),
                RENDER_WGPU_PACKAGE.to_owned(),
            ]),
            BTreeSet::new(),
        );
        let generic_members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (renderer.package.as_str(), &renderer),
            (generic.package.as_str(), &generic),
        ]);
        let mut generic_findings = Vec::new();
        validate_dependency_direction(&generic, &generic_members, &mut generic_findings);
        assert!(
            generic_findings
                .iter()
                .any(|finding| finding.code == "workspace.forbidden_dependency_direction")
        );
    }

    #[test]
    fn external_fixture_may_consume_testing_only_as_a_dev_dependency() {
        let core = member(
            "crates/runenui_core",
            CORE_PACKAGE,
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let runtime = member(
            "crates/runenui_runtime",
            RUNTIME_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let testing = member(
            "crates/runenui_testing",
            TESTING_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned(), RUNTIME_PACKAGE.to_owned()]),
            BTreeSet::new(),
        );
        let external = member(
            "tests/external_widget",
            EXTERNAL_WIDGET_PACKAGE,
            BTreeSet::from([CORE_PACKAGE.to_owned(), RUNTIME_PACKAGE.to_owned()]),
            BTreeSet::from([TESTING_PACKAGE.to_owned()]),
        );
        let members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (testing.package.as_str(), &testing),
            (external.package.as_str(), &external),
        ]);
        let mut findings = Vec::new();

        validate_dependency_direction(&external, &members, &mut findings);
        assert!(findings.is_empty());

        let production_external = member(
            "tests/external_widget",
            EXTERNAL_WIDGET_PACKAGE,
            BTreeSet::from([
                CORE_PACKAGE.to_owned(),
                RUNTIME_PACKAGE.to_owned(),
                TESTING_PACKAGE.to_owned(),
            ]),
            BTreeSet::new(),
        );
        let production_members = BTreeMap::from([
            (core.package.as_str(), &core),
            (runtime.package.as_str(), &runtime),
            (testing.package.as_str(), &testing),
            (production_external.package.as_str(), &production_external),
        ]);
        let mut production_findings = Vec::new();
        validate_dependency_direction(
            &production_external,
            &production_members,
            &mut production_findings,
        );
        assert!(
            production_findings
                .iter()
                .any(|finding| finding.code == "workspace.forbidden_dependency_direction")
        );
    }

    #[test]
    fn workspace_documentation_parser_reads_package_table_only() {
        let document = "| Package | Current ownership | Must not own |\n|---|---|---|\n| `core` | values | runtime |\n\n| Other | Value |\n|---|---|\n| `ignored` | value |\n";
        assert_eq!(
            documented_package_names(document),
            BTreeSet::from(["core".to_owned()])
        );
    }
}
