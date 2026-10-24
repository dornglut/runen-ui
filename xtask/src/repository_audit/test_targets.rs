//! Fail-closed coverage for explicit Cargo test suites and their source files.
//! With Cargo autotests disabled, sources not registered in a suite are inert.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use super::Finding;

const MANIFEST: &str = "tests/external_widget/Cargo.toml";
const TEST_DIRECTORY: &str = "tests/external_widget/tests";
const SUITES: &[&str] = &[
    "semantic_application",
    "input_control_editing",
    "scene_contracts",
    "presentation",
    "harness_trace_identity",
];

fn failure(findings: &mut Vec<Finding>, path: &str, message: impl Into<String>) {
    findings.push(Finding::fatal(
        "repository.external_widget_test_registration",
        Some(path.to_owned()),
        message,
    ));
}

fn quoted<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.trim().strip_prefix(field)?.strip_suffix('"')
}

fn audit_manifest(contents: &str, findings: &mut Vec<Finding>) {
    if !contents
        .lines()
        .any(|line| line.trim() == "autotests = false")
    {
        failure(
            findings,
            MANIFEST,
            "expected explicit Cargo tests with autotests = false",
        );
    }
    let mut targets = BTreeMap::<String, usize>::new();
    for block in contents.split("[[test]]").skip(1) {
        let mut name = None;
        let mut path = None;
        for line in block.lines() {
            if line.trim_start().starts_with('[') {
                break;
            }
            name = name.or_else(|| quoted(line, "name = \""));
            path = path.or_else(|| quoted(line, "path = \""));
        }
        match (name, path) {
            (Some(name), Some(path))
                if path == format!("tests/suites/{name}.rs") && SUITES.contains(&name) =>
            {
                *targets.entry(name.to_owned()).or_default() += 1;
            }
            _ => failure(
                findings,
                MANIFEST,
                "unexpected, missing or malformed explicit [[test]] suite target",
            ),
        }
    }
    for suite in SUITES {
        if targets.get(*suite) != Some(&1) {
            failure(
                findings,
                MANIFEST,
                format!("expected exactly one Cargo target for suite {suite}"),
            );
        }
    }
}

fn audit_suite(
    root: &Path,
    suite: &str,
    registered: &mut BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let relative = format!("{TEST_DIRECTORY}/suites/{suite}.rs");
    let contents = fs::read_to_string(root.join(&relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    let mut expected_mod: Option<String> = None;
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(file) = trimmed
            .strip_prefix("#[path = \"../")
            .and_then(|suffix| suffix.strip_suffix("\"]"))
        {
            if expected_mod.is_some() {
                failure(findings, &relative, "two consecutive #[path] declarations");
            }
            let name = file.strip_suffix(".rs").unwrap_or("");
            if name.is_empty() || name.contains('/') || name.contains('\\') {
                failure(findings, &relative, format!("invalid suite import: {file}"));
            }
            expected_mod = Some(name.to_owned());
        } else if let Some(module) = trimmed
            .strip_prefix("mod ")
            .and_then(|suffix| suffix.strip_suffix(';'))
        {
            if expected_mod.as_deref() != Some(module) {
                failure(
                    findings,
                    &relative,
                    format!("module {module} lacks a matching preceding #[path]"),
                );
            }
            registered
                .entry(format!("{module}.rs"))
                .or_default()
                .push(suite.to_owned());
            expected_mod = None;
        } else if !trimmed.is_empty() {
            failure(
                findings,
                &relative,
                format!("unexpected suite entrypoint syntax: {trimmed}"),
            );
        }
    }
    if expected_mod.is_some() {
        failure(findings, &relative, "dangling #[path] without module");
    }
    Ok(())
}

// A suite may import a module which itself imports ordinary sibling test
// modules. Cargo compiles those indirect modules too; do not count them as
// unregistered or import them a second time in the suite entrypoint.
fn nested_imports(contents: &str, path: &str, findings: &mut Vec<Finding>) -> Vec<String> {
    let mut pending = None;
    let mut imports = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(file) = trimmed
            .strip_prefix("#[path = \"")
            .and_then(|suffix| suffix.strip_suffix("\"]"))
        {
            if pending.replace(file).is_some() {
                failure(findings, path, "unpaired nested #[path] attribute");
            }
        } else if let Some(module) = trimmed
            .strip_prefix("mod ")
            .and_then(|suffix| suffix.strip_suffix(';'))
        {
            if let Some(file) = pending.take() {
                if Path::new(file)
                    .extension()
                    .is_none_or(|extension| extension != "rs")
                    || file.contains('/')
                    || file.contains('\\')
                {
                    failure(
                        findings,
                        path,
                        format!("nonlocal nested proof import: {file}"),
                    );
                } else {
                    imports.push(file.to_owned());
                }
            } else {
                failure(
                    findings,
                    path,
                    format!("nested module {module} lacks a checked sibling #[path] import"),
                );
            }
        } else if pending.is_some() && !trimmed.is_empty() {
            failure(findings, path, "nested #[path] does not precede a module");
            pending = None;
        }
    }
    if pending.is_some() {
        failure(findings, path, "dangling nested #[path]");
    }
    imports
}

fn expand_nested_imports(
    root: &Path,
    registered: &mut BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let mut scanned = BTreeSet::new();
    loop {
        let todo = registered
            .iter()
            .flat_map(|(source, suites)| {
                suites
                    .iter()
                    .map(move |suite| (source.clone(), suite.clone()))
            })
            .filter(|pair| !scanned.contains(pair))
            .collect::<Vec<_>>();
        if todo.is_empty() {
            break;
        }
        for (source, suite) in todo {
            scanned.insert((source.clone(), suite.clone()));
            let relative = format!("{TEST_DIRECTORY}/{source}");
            let content = fs::read_to_string(root.join(&relative))
                .map_err(|error| format!("failed to read nested source {relative}: {error}"))?;
            for included in nested_imports(&content, &relative, findings) {
                registered.entry(included).or_default().push(suite.clone());
            }
        }
    }
    Ok(())
}

fn check_coverage(
    sources: &BTreeSet<String>,
    registered: &BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    for source in sources {
        match registered.get(source) {
            None => failure(
                findings,
                TEST_DIRECTORY,
                format!("unregistered public conformance test source: {source}"),
            ),
            Some(suites) if suites.len() != 1 => failure(
                findings,
                TEST_DIRECTORY,
                format!("test {source} imported more than once: {suites:?}"),
            ),
            Some(_) => {}
        }
    }
    for source in registered.keys() {
        if !sources.contains(source) {
            failure(
                findings,
                TEST_DIRECTORY,
                format!("suite imports absent test source: {source}"),
            );
        }
    }
}

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let contents = fs::read_to_string(root.join(MANIFEST))
        .map_err(|error| format!("failed to read {MANIFEST}: {error}"))?;
    audit_manifest(&contents, findings);

    let mut sources = BTreeSet::new();
    let directory = root.join(TEST_DIRECTORY);
    let entries = fs::read_dir(&directory)
        .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to inspect test source: {error}"))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 external-widget test source".to_owned())?;
        if path.is_dir() {
            if name != "suites" {
                failure(
                    findings,
                    TEST_DIRECTORY,
                    format!("untracked nested test directory: {name}"),
                );
            }
            continue;
        }
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            sources.insert(name);
        }
    }

    let suite_directory = format!("{TEST_DIRECTORY}/suites");
    let suite_entries = fs::read_dir(root.join(&suite_directory))
        .map_err(|error| format!("failed to inspect {suite_directory}: {error}"))?;
    for entry in suite_entries {
        let entry = entry.map_err(|error| format!("failed to inspect suite file: {error}"))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 external-widget suite file".to_owned())?;
        if path.is_dir() {
            failure(
                findings,
                &suite_directory,
                format!("untracked nested suite directory: {name}"),
            );
        } else if path.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
            && !SUITES.iter().any(|suite| name == format!("{suite}.rs"))
        {
            failure(
                findings,
                &suite_directory,
                format!("unregistered suite source: {name}"),
            );
        }
    }

    let mut registered = BTreeMap::new();
    for suite in SUITES {
        audit_suite(root, suite, &mut registered, findings)?;
    }
    expand_nested_imports(root, &mut registered, findings)?;
    check_coverage(&sources, &registered, findings);
    if root.join("crates/runenui_runtime").is_dir() {
        audit_runtime(root, findings)?;
    }
    Ok(())
}

const RUNTIME_MANIFEST: &str = "crates/runenui_runtime/Cargo.toml";
const RUNTIME_TEST_DIRECTORY: &str = "crates/runenui_runtime/tests";

fn runtime_failure(findings: &mut Vec<Finding>, path: &str, message: impl Into<String>) {
    findings.push(Finding::fatal(
        "repository.runtime_test_registration",
        Some(path.to_owned()),
        message,
    ));
}

fn audit_runtime_coverage(
    sources: &BTreeSet<String>,
    registered: &BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    for source in sources {
        match registered.get(source) {
            None => runtime_failure(
                findings,
                RUNTIME_TEST_DIRECTORY,
                format!("unregistered runtime test source: {source}"),
            ),
            Some(owners) if owners.len() != 1 => runtime_failure(
                findings,
                RUNTIME_TEST_DIRECTORY,
                format!("runtime test {source} registered more than once: {owners:?}"),
            ),
            Some(_) => {}
        }
    }
    for source in registered.keys() {
        if !sources.contains(source) {
            runtime_failure(
                findings,
                RUNTIME_TEST_DIRECTORY,
                format!("runtime suite imports absent test source: {source}"),
            );
        }
    }
}

// Both direct Cargo test roots and suite #[path] imports are executable owners.
// This catches new inert tests whenever runtime keeps autotests = false.
type RuntimeManifestRegistration = (BTreeSet<String>, BTreeMap<String, Vec<String>>);

fn audit_runtime_manifest(
    root: &Path,
    findings: &mut Vec<Finding>,
) -> Result<RuntimeManifestRegistration, String> {
    let contents = fs::read_to_string(root.join(RUNTIME_MANIFEST))
        .map_err(|error| format!("failed to read {RUNTIME_MANIFEST}: {error}"))?;
    if !contents
        .lines()
        .any(|line| line.trim() == "autotests = false")
    {
        runtime_failure(
            findings,
            RUNTIME_MANIFEST,
            "expected explicit runtime test registration with autotests = false",
        );
    }

    let mut suites = BTreeSet::<String>::new();
    let mut registered = BTreeMap::<String, Vec<String>>::new();
    for block in contents.split("[[test]]").skip(1) {
        let mut name = None;
        let mut path = None;
        for line in block.lines() {
            if line.trim_start().starts_with('[') {
                break;
            }
            name = name.or_else(|| quoted(line, "name = \""));
            path = path.or_else(|| quoted(line, "path = \""));
        }
        match (name, path) {
            (Some(name), Some(path)) if path == format!("tests/suites/{name}.rs") => {
                if !suites.insert(name.to_owned()) {
                    runtime_failure(
                        findings,
                        RUNTIME_MANIFEST,
                        format!("duplicate runtime Cargo suite target: {name}"),
                    );
                }
            }
            (Some(name), Some(path)) if path == format!("tests/{name}.rs") => {
                registered
                    .entry(format!("{name}.rs"))
                    .or_default()
                    .push(format!("Cargo:{name}"));
            }
            _ => runtime_failure(
                findings,
                RUNTIME_MANIFEST,
                "unexpected, missing or malformed runtime [[test]] target",
            ),
        }
    }

    Ok((suites, registered))
}

fn audit_runtime(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let (suites, mut registered) = audit_runtime_manifest(root, findings)?;

    let suite_dir = format!("{RUNTIME_TEST_DIRECTORY}/suites");
    for entry in fs::read_dir(root.join(&suite_dir))
        .map_err(|error| format!("failed to inspect {suite_dir}: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to inspect runtime suite: {error}"))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 runtime suite name".to_owned())?;
        if path.is_dir()
            || (path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                && !name
                    .strip_suffix(".rs")
                    .is_some_and(|stem| suites.contains(stem)))
        {
            runtime_failure(
                findings,
                &suite_dir,
                format!("unregistered runtime suite source: {name}"),
            );
        }
    }

    for suite in &suites {
        let relative = format!("{suite_dir}/{suite}.rs");
        let content = fs::read_to_string(root.join(&relative))
            .map_err(|error| format!("failed to read {relative}: {error}"))?;
        let mut pending: Option<&str> = None;
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(file) = trimmed
                .strip_prefix("#[path = \"../")
                .and_then(|tail| tail.strip_suffix("\"]"))
            {
                if pending.replace(file).is_some() {
                    runtime_failure(findings, &relative, "consecutive runtime #[path] imports");
                }
            } else if let Some(module) = trimmed
                .strip_prefix("mod ")
                .and_then(|tail| tail.strip_suffix(';'))
            {
                let file = pending.take();
                if file.and_then(|path| path.strip_suffix(".rs")) != Some(module)
                    || file.is_some_and(|path| path.contains('/') || path.contains('\\'))
                {
                    runtime_failure(
                        findings,
                        &relative,
                        format!("runtime module {module} lacks matching local #[path]"),
                    );
                } else if let Some(file) = file {
                    registered
                        .entry(file.to_owned())
                        .or_default()
                        .push(suite.to_owned());
                }
            } else if !trimmed.is_empty() {
                runtime_failure(
                    findings,
                    &relative,
                    format!("unexpected runtime suite entrypoint syntax: {trimmed}"),
                );
            }
        }
        if pending.is_some() {
            runtime_failure(findings, &relative, "dangling runtime #[path] import");
        }
    }

    let mut sources = BTreeSet::new();
    for entry in fs::read_dir(root.join(RUNTIME_TEST_DIRECTORY))
        .map_err(|error| format!("failed to inspect {RUNTIME_TEST_DIRECTORY}: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to inspect runtime test: {error}"))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 runtime test name".to_owned())?;
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            sources.insert(name);
        } else if path.is_dir() && name != "suites" && name != "surface_context" {
            runtime_failure(
                findings,
                RUNTIME_TEST_DIRECTORY,
                format!("unexpected runtime test directory: {name}"),
            );
        }
    }
    audit_runtime_coverage(&sources, &registered, findings);
    Ok(())
}

// Explicit test roots are an execution contract: with Cargo autotests disabled,
// every owned source must be reachable from exactly one declared test target.
struct ExplicitTestPackage {
    name: &'static str,
    manifest: &'static str,
    tests_dir: &'static str,
    direct: &'static [&'static str],
    suites: &'static [&'static str],
    finding_code: &'static str,
}

const TESTING_PACKAGE: ExplicitTestPackage = ExplicitTestPackage {
    name: "testing",
    manifest: "crates/runenui_testing/Cargo.toml",
    tests_dir: "crates/runenui_testing/tests",
    direct: &["counter", "m5e_counter_origins"],
    suites: &["controls_semantics", "motion_time", "harness_boundary"],
    finding_code: "repository.testing_test_registration",
};

const CORE_PACKAGE: ExplicitTestPackage = ExplicitTestPackage {
    name: "core",
    manifest: "crates/runenui_core/Cargo.toml",
    tests_dir: "crates/runenui_core/tests",
    direct: &[],
    suites: &["element_authoring", "input_protocols", "style_resolution"],
    finding_code: "repository.core_test_registration",
};

fn explicit_test_failure(
    package: &ExplicitTestPackage,
    findings: &mut Vec<Finding>,
    path: &str,
    message: impl Into<String>,
) {
    findings.push(Finding::fatal(
        package.finding_code,
        Some(path.to_owned()),
        message,
    ));
}

fn audit_explicit_test_manifest(
    package: &ExplicitTestPackage,
    contents: &str,
    registered: &mut BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    if !contents
        .lines()
        .any(|line| line.trim() == "autotests = false")
    {
        explicit_test_failure(
            package,
            findings,
            package.manifest,
            format!("expected explicit {} targets with autotests = false", package.name),
        );
    }
    let mut targets = BTreeMap::<String, usize>::new();
    for block in contents.split("[[test]]").skip(1) {
        let mut name = None;
        let mut path = None;
        for line in block.lines() {
            if line.trim_start().starts_with('[') {
                break;
            }
            name = name.or_else(|| quoted(line, "name = \""));
            path = path.or_else(|| quoted(line, "path = \""));
        }
        match (name, path) {
            (Some(name), Some(path))
                if package.direct.contains(&name) && path == format!("tests/{name}.rs") =>
            {
                *targets.entry(name.to_owned()).or_default() += 1;
                registered
                    .entry(format!("{name}.rs"))
                    .or_default()
                    .push(format!("Cargo:{name}"));
            }
            (Some(name), Some(path))
                if package.suites.contains(&name) && path == format!("tests/suites/{name}.rs") =>
            {
                *targets.entry(name.to_owned()).or_default() += 1;
            }
            _ => explicit_test_failure(
                package,
                findings,
                package.manifest,
                format!("unexpected, missing or malformed explicit {} [[test]] target", package.name),
            ),
        }
    }
    for target in package.direct.iter().chain(package.suites.iter()) {
        if targets.get(*target) != Some(&1) {
            explicit_test_failure(
                package,
                findings,
                package.manifest,
                format!("expected exactly one Cargo target for {} suite {target}", package.name),
            );
        }
    }
}

fn audit_explicit_test_suite(
    root: &Path,
    package: &ExplicitTestPackage,
    suite: &str,
    registered: &mut BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let relative = format!("{}/suites/{suite}.rs", package.tests_dir);
    let contents = fs::read_to_string(root.join(&relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    let mut pending = None;
    for line in contents.lines() {
        let trimmed = line.trim();
        if let Some(file) = trimmed
            .strip_prefix("#[path = \"../")
            .and_then(|tail| tail.strip_suffix("\"]"))
        {
            let module = file.strip_suffix(".rs").unwrap_or("");
            if module.is_empty() || module.contains('/') || module.contains('\\') {
                explicit_test_failure(
                    package,
                    findings,
                    &relative,
                    format!("invalid {} suite import: {file}", package.name),
                );
            }
            if pending.replace(module.to_owned()).is_some() {
                explicit_test_failure(
                    package,
                    findings,
                    &relative,
                    format!("two consecutive {} #[path] declarations", package.name),
                );
            }
        } else if let Some(module) = trimmed
            .strip_prefix("mod ")
            .and_then(|tail| tail.strip_suffix(';'))
        {
            if pending.as_deref() == Some(module) {
                registered
                    .entry(format!("{module}.rs"))
                    .or_default()
                    .push(suite.to_owned());
            } else {
                explicit_test_failure(
                    package,
                    findings,
                    &relative,
                    format!("{} module {module} lacks a matching preceding #[path]", package.name),
                );
            }
            pending = None;
        } else if !trimmed.is_empty() {
            explicit_test_failure(
                package,
                findings,
                &relative,
                format!("unexpected {} suite entrypoint syntax: {trimmed}", package.name),
            );
        }
    }
    if pending.is_some() {
        explicit_test_failure(
            package,
            findings,
            &relative,
            format!("dangling {} #[path] without module", package.name),
        );
    }
    Ok(())
}

fn check_explicit_test_coverage(
    package: &ExplicitTestPackage,
    sources: &BTreeSet<String>,
    registered: &BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    for source in sources {
        match registered.get(source) {
            None => explicit_test_failure(
                package,
                findings,
                package.tests_dir,
                format!("unregistered {} test source: {source}", package.name),
            ),
            Some(owners) if owners.len() != 1 => explicit_test_failure(
                package,
                findings,
                package.tests_dir,
                format!("{} test {source} imported more than once: {owners:?}", package.name),
            ),
            Some(_) => {}
        }
    }
    for source in registered.keys() {
        if !sources.contains(source) {
            explicit_test_failure(
                package,
                findings,
                package.tests_dir,
                format!("{} suite imports absent test source: {source}", package.name),
            );
        }
    }
}

fn audit_explicit_test_package(
    root: &Path,
    package: &ExplicitTestPackage,
    findings: &mut Vec<Finding>,
) -> Result<(), String> {
    let contents = fs::read_to_string(root.join(package.manifest))
        .map_err(|error| format!("failed to read {}: {error}", package.manifest))?;
    let mut registered = BTreeMap::new();
    audit_explicit_test_manifest(package, &contents, &mut registered, findings);

    let suite_dir = format!("{}/suites", package.tests_dir);
    for entry in fs::read_dir(root.join(&suite_dir))
        .map_err(|error| format!("failed to inspect {suite_dir}: {error}"))?
    {
        let entry = entry.map_err(|error| format!("failed to inspect {} suite: {error}", package.name))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-UTF-8 {} suite file", package.name))?;
        if path.is_dir()
            || (path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                && !package.suites.iter().any(|suite| name == format!("{suite}.rs")))
        {
            explicit_test_failure(
                package,
                findings,
                &suite_dir,
                format!("unregistered {} suite source: {name}", package.name),
            );
        }
    }
    for suite in package.suites {
        audit_explicit_test_suite(root, package, suite, &mut registered, findings)?;
    }

    let mut sources = BTreeSet::new();
    for entry in fs::read_dir(root.join(package.tests_dir))
        .map_err(|error| format!("failed to inspect {}: {error}", package.tests_dir))?
    {
        let entry = entry.map_err(|error| format!("failed to inspect {} source: {error}", package.name))?;
        let path = entry.path();
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("non-UTF-8 {} source file", package.name))?;
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            sources.insert(name);
        } else if path.is_dir() && name != "suites" {
            explicit_test_failure(
                package,
                findings,
                package.tests_dir,
                format!("unexpected {} test directory: {name}", package.name),
            );
        }
    }
    check_explicit_test_coverage(package, &sources, &registered, findings);
    Ok(())
}

pub(super) fn audit_testing(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    audit_explicit_test_package(root, &TESTING_PACKAGE, findings)
}

pub(super) fn audit_core(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    audit_explicit_test_package(root, &CORE_PACKAGE, findings)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{audit_manifest, check_coverage};

    #[test]
    fn runtime_registration_guard_rejects_inert_and_duplicate_sources() {
        let sources = ["focus_visible.rs", "m11_password_field.rs"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let registered = BTreeMap::from([(
            "focus_visible.rs".to_owned(),
            vec![
                "focus_keyboard_input".to_owned(),
                "another_suite".to_owned(),
            ],
        )]);
        let mut findings = Vec::new();
        super::audit_runtime_coverage(&sources, &registered, &mut findings);
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.message.contains("unregistered")));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("more than once"))
        );

        let registered = BTreeMap::from([
            (
                "focus_visible.rs".to_owned(),
                vec!["focus_keyboard_input".to_owned()],
            ),
            (
                "m11_password_field.rs".to_owned(),
                vec!["m10_semantic_integrity".to_owned()],
            ),
        ]);
        findings.clear();
        super::audit_runtime_coverage(&sources, &registered, &mut findings);
        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn nested_module_imports_are_counted_without_direct_suite_registration() {
        let mut findings = Vec::new();
        let imports = super::nested_imports(
            "#[path = \"input_arbitration_finality.rs\"]\nmod arbitration_finality;\n",
            "tests/external_widget/tests/input.rs",
            &mut findings,
        );
        assert_eq!(imports, ["input_arbitration_finality.rs"]);
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn unchecked_plain_nested_modules_fail_closed() {
        let mut findings = Vec::new();
        let imports = super::nested_imports(
            "mod untracked;\n",
            "tests/external_widget/tests/input.rs",
            &mut findings,
        );
        assert_eq!(imports, Vec::<String>::new());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("lacks a checked sibling"));
    }

    #[test]
    fn full_coverage_is_required_and_unknown_imports_fail() {
        let files = ["one.rs", "two.rs"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let mut registered = BTreeMap::new();
        registered.insert("one.rs".to_owned(), vec!["semantic_application".to_owned()]);
        registered.insert(
            "gone.rs".to_owned(),
            vec!["input_control_editing".to_owned()],
        );
        let mut findings = Vec::new();
        check_coverage(&files, &registered, &mut findings);
        assert_eq!(findings.len(), 2);
        assert!(
            findings
                .iter()
                .any(|item| item.message.contains("unregistered"))
        );
        assert!(findings.iter().any(|item| item.message.contains("absent")));
    }

    #[test]
    fn duplicate_imports_fail_even_when_every_file_is_covered() {
        let files = std::iter::once("one.rs").map(str::to_owned).collect();
        let registered = BTreeMap::from([(
            "one.rs".to_owned(),
            vec![
                "semantic_application".to_owned(),
                "composite_controls".to_owned(),
            ],
        )]);
        let mut findings = Vec::new();
        check_coverage(&files, &registered, &mut findings);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("more than once"));
    }

    #[test]
    fn missing_explicit_cargo_target_fails() {
        let mut findings = Vec::new();
        audit_manifest(
            "[package]\nautotests = false\n[[test]]\nname = \"unknown\"\npath = \"tests/suites/unknown.rs\"\n",
            &mut findings,
        );
        assert!(
            findings
                .iter()
                .any(|item| item.message.contains("unexpected"))
        );
        assert!(
            findings
                .iter()
                .any(|item| item.message.contains("harness_trace_identity"))
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the filesystem mutation proof exercises each tested registration transition through the real repository audit"
    )]
    fn testing_registration_mutations_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
        use std::{
            fs,
            path::PathBuf,
            sync::atomic::{AtomicUsize, Ordering},
        };

        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
        let fixture = Fixture(std::env::temp_dir().join(format!(
            "runenui-testing-registration-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        )));
        let testing = fixture.0.join("crates/runenui_testing");
        let tests = testing.join("tests");
        let suites = tests.join("suites");
        fs::create_dir_all(&suites)?;
        fs::write(
            testing.join("Cargo.toml"),
            r#"[package]
autotests = false

[[test]]
name = "counter"
path = "tests/counter.rs"

[[test]]
name = "m5e_counter_origins"
path = "tests/m5e_counter_origins.rs"

[[test]]
name = "controls_semantics"
path = "tests/suites/controls_semantics.rs"

[[test]]
name = "motion_time"
path = "tests/suites/motion_time.rs"

[[test]]
name = "harness_boundary"
path = "tests/suites/harness_boundary.rs"
"#,
        )?;
        for source in [
            "counter.rs",
            "m5e_counter_origins.rs",
            "controls.rs",
            "clock.rs",
            "harness.rs",
        ] {
            fs::write(tests.join(source), "")?;
        }
        let controls = "#[path = \"../controls.rs\"]\nmod controls;\n";
        let clock = "#[path = \"../clock.rs\"]\nmod clock;\n";
        let harness = "#[path = \"../harness.rs\"]\nmod harness;\n";
        fs::write(suites.join("controls_semantics.rs"), controls)?;
        fs::write(suites.join("motion_time.rs"), clock)?;
        fs::write(suites.join("harness_boundary.rs"), harness)?;

        let mut findings = Vec::new();
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(findings.is_empty(), "valid fixture: {findings:?}");

        fs::write(tests.join("new_negative_proof.rs"), "")?;
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("unregistered testing test source"))
        );

        findings.clear();
        let with_new =
            format!("{harness}#[path = \"../new_negative_proof.rs\"]\nmod new_negative_proof;\n");
        fs::write(suites.join("harness_boundary.rs"), &with_new)?;
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(
            findings.is_empty(),
            "new proof registered once: {findings:?}"
        );

        fs::write(
            suites.join("controls_semantics.rs"),
            format!("{controls}#[path = \"../new_negative_proof.rs\"]\nmod new_negative_proof;\n"),
        )?;
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("imported more than once"))
        );

        findings.clear();
        fs::write(suites.join("controls_semantics.rs"), controls)?;
        fs::remove_file(tests.join("new_negative_proof.rs"))?;
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("imports absent test source"))
        );

        fs::write(tests.join("new_negative_proof.rs"), "")?;
        let manifest = fs::read_to_string(testing.join("Cargo.toml"))?;
        fs::write(
            testing.join("Cargo.toml"),
            manifest.replace("autotests = false", "autotests = true"),
        )?;
        findings.clear();
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains("autotests = false"))
        );

        fs::write(testing.join("Cargo.toml"), manifest)?;
        fs::write(suites.join("unexpected.rs"), "")?;
        findings.clear();
        super::audit_testing(&fixture.0, &mut findings)?;
        assert!(findings.iter().any(|finding| {
            finding
                .message
                .contains("unregistered testing suite source")
        }));
        Ok(())
    }


    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "filesystem mutations test the exact source registration state transitions"
    )]
    fn core_registration_mutations_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
        use std::{
            fs,
            path::PathBuf,
            sync::atomic::{AtomicUsize, Ordering},
        };

        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);
        let fixture = Fixture(std::env::temp_dir().join(format!(
            "runenui-core-registration-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        )));
        let core = fixture.0.join("crates/runenui_core");
        let tests = core.join("tests");
        let suites = tests.join("suites");
        fs::create_dir_all(&suites)?;
        let manifest = r#"[package]
autotests = false

[[test]]
name = "element_authoring"
path = "tests/suites/element_authoring.rs"

[[test]]
name = "input_protocols"
path = "tests/suites/input_protocols.rs"

[[test]]
name = "style_resolution"
path = "tests/suites/style_resolution.rs"
"#;
        fs::write(core.join("Cargo.toml"), manifest)?;
        for (suite, source) in [
            ("element_authoring", "element.rs"),
            ("input_protocols", "input.rs"),
            ("style_resolution", "style.rs"),
        ] {
            fs::write(tests.join(source), "")?;
            fs::write(
                suites.join(format!("{suite}.rs")),
                format!("#[path = \"../{source}\"]\nmod {};\n", source.trim_end_matches(".rs")),
            )?;
        }
        let mut findings = Vec::new();
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.is_empty(), "valid core fixture: {findings:?}");

        fs::write(tests.join("new_proof.rs"), "")?;
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.iter().any(|f| f.message.contains("unregistered core test source")));

        findings.clear();
        let registered = "#[path = \"../element.rs\"]\nmod element;\n#[path = \"../new_proof.rs\"]\nmod new_proof;\n";
        fs::write(suites.join("element_authoring.rs"), registered)?;
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.is_empty(), "registered once: {findings:?}");

        fs::write(
            suites.join("input_protocols.rs"),
            "#[path = \"../input.rs\"]\nmod input;\n#[path = \"../new_proof.rs\"]\nmod new_proof;\n",
        )?;
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.iter().any(|f| f.message.contains("imported more than once")));

        findings.clear();
        fs::write(suites.join("input_protocols.rs"), "#[path = \"../input.rs\"]\nmod input;\n")?;
        fs::remove_file(tests.join("new_proof.rs"))?;
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.iter().any(|f| f.message.contains("imports absent test source")));

        fs::write(tests.join("new_proof.rs"), "")?;
        fs::write(core.join("Cargo.toml"), manifest.replace("autotests = false", "autotests = true"))?;
        findings.clear();
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.iter().any(|f| f.message.contains("autotests = false")));

        fs::write(core.join("Cargo.toml"), manifest)?;
        fs::write(suites.join("unexpected.rs"), "")?;
        findings.clear();
        super::audit_core(&fixture.0, &mut findings)?;
        assert!(findings.iter().any(|f| f.message.contains("unregistered core suite source")));
        Ok(())
    }
}
