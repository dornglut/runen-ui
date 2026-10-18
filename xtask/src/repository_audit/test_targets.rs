//! Fail-closed coverage for the external-widget package's explicit Cargo suites.
//! Cargo autotests=false makes new test files inert until a suite imports them.

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

// Explicit, reviewed existing omissions; #483 owns staged public-test repair.
// A file must leave this quarantine as soon as its suite import is accepted.
// #477/#482 own the lifecycle file and must resolve the M11 CTRL-85/86 conflict.
const PENDING_REMEDIATION: &[&str] = &[
    "combo_box_boundaries.rs",
    "combo_box_editable.rs",
    "combo_box_editable_parity.rs",
    "combo_box_focus_lifecycle.rs",
    "combo_box_option_proxy.rs",
    "combo_box_reveal_downstream.rs",
    "combo_box_reveal_editable.rs",
    "combo_box_scroll_viewport.rs",
    "combo_box_select_only.rs",
    "spin_button.rs",
    "text_field.rs",
    "tree_view.rs",
    "tree_view_domain_identity.rs",
    "tree_view_downstream.rs",
    "tree_view_scroll.rs",
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

fn check_coverage(
    sources: &BTreeSet<String>,
    registered: &BTreeMap<String, Vec<String>>,
    findings: &mut Vec<Finding>,
) {
    for source in sources {
        match registered.get(source) {
            None if PENDING_REMEDIATION.contains(&source.as_str()) => {}
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
    for pending in PENDING_REMEDIATION {
        if !sources.contains(*pending) || registered.contains_key(*pending) {
            failure(
                findings,
                TEST_DIRECTORY,
                format!("obsolete or invalid #483 quarantine entry: {pending}; retire it when registered"),
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
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF-8 external-widget test source".to_owned())?;
        sources.insert(name);
    }

    let mut registered = BTreeMap::new();
    for suite in SUITES {
        audit_suite(root, suite, &mut registered, findings)?;
    }
    check_coverage(&sources, &registered, findings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{audit_manifest, check_coverage};

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
        let files = ["one.rs"].into_iter().map(str::to_owned).collect();
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
    fn quarantine_is_checked_and_must_retire_when_registered() {
        let files = super::PENDING_REMEDIATION
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        let mut registered = BTreeMap::new();
        let mut findings = Vec::new();
        // This historical fixture has an explicit #483 exemption until qualified.
        check_coverage(&files, &registered, &mut findings);
        assert_eq!(findings.len(), 0);
        // The quarantine cannot remain after the source becomes an executed module.
        registered.insert(
            "combo_box_boundaries.rs".to_owned(),
            vec!["semantic_application".to_owned()],
        );
        findings.clear();
        check_coverage(&files, &registered, &mut findings);
        assert!(findings.iter().any(|item| item.message.contains("obsolete")));
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
}
