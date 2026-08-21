//! Deterministic repository structure and authority audit.

mod matrix;
mod source;
mod workspace;

use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use self::{matrix::MatrixMetrics, source::SourceMetrics, workspace::WorkspaceMetrics};

const SCHEMA_VERSION: u32 = 2;
const PRIVATE_ARCHIVE_URL: &str = "github.com/Crystonix/runen-ui-private-archive";
const HISTORICAL_OWNER_TOKEN: &str = "Crystonix/runen-ui";
const CURRENT_REPOSITORY_DECLARATION: &str =
    "repository = \"https://github.com/dornglut/runen-ui\"";
const ACCEPTED_REUSABLE_WORKFLOW_REVISION: &str = "624cb41adeed21a6461eb838bc7330bd0a5079fd";
const REUSABLE_WORKFLOW_OWNER_AND_DIRECTORY: &str =
    "dornglut/github-workflows/.github/workflows";
const REUSABLE_RUST_WORKFLOW: &str = "reusable-rust-cargo-validate.yml";
const ACTIVE_WORKFLOW_DIRECTORY: &str = ".github/workflows";
const CI_WORKFLOW_PATH: &str = ".github/workflows/ci.yml";
const ISSUE_TEMPLATE_DIRECTORY: &str = ".github/ISSUE_TEMPLATE";
const MIGRATION_HISTORY_PATH: &str = "docs/history/public-repository-migration.md";

const REQUIRED_ENTRYPOINTS: &[&str] = &[
    "README.md",
    "AGENTS.md",
    "ARCHITECTURE.md",
    "TESTING.md",
    "docs/README.md",
    "docs/documentation-architecture.md",
    "docs/roadmap.md",
    "docs/status.md",
    "docs/architecture/README.md",
    "docs/architecture/public-api.md",
    "docs/conformance/README.md",
    "docs/conformance/m4-conformance-matrix.md",
    "docs/conformance/m5-conformance-matrix.md",
    "docs/conformance/m6-conformance-matrix.md",
];

const RETIRED_AUTHORITY_PATHS: &[&str] = &[
    "docs/architecture.md",
    "docs/documentation-retention-plan.md",
    "docs/feature-support-matrix.md",
    "docs/status-map.md",
    "docs/work-tracking.md",
    "docs/architecture/m4-conformance-matrix.md",
    "docs/architecture/m4-directional-focus-corpus.md",
    "docs/architecture/m4c-delivery-and-routed-transaction-charter.md",
    "docs/architecture/m5-accesskit-mapping-review.md",
    "docs/architecture/m5-conformance-matrix.md",
    "docs/architecture/m5-semantics-and-testing-charter.md",
    "docs/architecture/m6-conformance-matrix.md",
];

const DURABLE_CURRENT_AUTHORITY_FILES: &[&str] = &[
    "README.md",
    "AGENTS.md",
    "ARCHITECTURE.md",
    "TESTING.md",
    "docs/README.md",
    "docs/documentation-architecture.md",
    "docs/roadmap.md",
    "docs/status.md",
    "docs/vocabulary.md",
    "docs/architecture/README.md",
    "docs/architecture/public-api.md",
];

const REQUIRED_ISSUE_TEMPLATE_FILES: &[&str] = &[
    "config.yml",
    "defect.yml",
    "milestone-slice.yml",
    "proposal.yml",
];

const EXPECTED_ACTIVE_WORKFLOW_FILES: &[&str] = &["ci.yml"];

fn accepted_reusable_workflow_reference() -> String {
    format!(
        "{REUSABLE_WORKFLOW_OWNER_AND_DIRECTORY}/{REUSABLE_RUST_WORKFLOW}@{ACCEPTED_REUSABLE_WORKFLOW_REVISION}"
    )
}

fn expected_ci_workflow() -> String {
    format!(
        "name: CI\n\non:\n  pull_request:\n  push:\n    branches:\n      - main\n\npermissions:\n  contents: read\n\nconcurrency:\n  group: ci-${{{{ github.workflow }}}}-${{{{ github.ref }}}}\n  cancel-in-progress: true\n\njobs:\n  validate:\n    name: RunenUI validation\n    uses: {}\n",
        accepted_reusable_workflow_reference()
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Severity {
    Fatal,
    Diagnostic,
}

impl Severity {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Fatal => "fatal",
            Self::Diagnostic => "diagnostic",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Finding {
    severity: Severity,
    code: &'static str,
    path: Option<String>,
    message: String,
}

impl Finding {
    fn fatal(
        code: &'static str,
        path: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Fatal,
            code,
            path: path.into(),
            message: message.into(),
        }
    }

    fn diagnostic(
        code: &'static str,
        path: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Diagnostic,
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AuditMetrics {
    matrix: MatrixMetrics,
    workspace: WorkspaceMetrics,
    source: SourceMetrics,
    authority_files: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AuditReport {
    findings: Vec<Finding>,
    metrics: AuditMetrics,
}

impl AuditReport {
    fn finalize(&mut self) {
        self.findings.sort();
    }

    fn fatal_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Fatal)
            .count()
    }

    fn diagnostic_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == Severity::Diagnostic)
            .count()
    }

    fn is_success(&self) -> bool {
        self.fatal_count() == 0
    }

    fn render_failure(&self) -> String {
        let mut output = format!(
            "repository authority audit failed with {} fatal finding(s)",
            self.fatal_count()
        );
        for finding in self
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Fatal)
        {
            let _ = write!(
                output,
                "\n[{}] {}{}: {}",
                finding.severity.as_str(),
                finding.code,
                finding
                    .path
                    .as_deref()
                    .map_or_else(String::new, |path| format!(" ({path})")),
                finding.message
            );
        }
        output
    }

    fn render_human(&self) -> String {
        let mut output = String::new();
        let status = if self.is_success() { "PASS" } else { "FAIL" };
        let _ = writeln!(output, "repository audit: {status}");
        let _ = writeln!(output, "schema version: {SCHEMA_VERSION}");
        let _ = writeln!(output, "fatal findings: {}", self.fatal_count());
        let _ = writeln!(output, "diagnostics: {}", self.diagnostic_count());
        let _ = writeln!(
            output,
            "matrix: {} rows, {} owner-accepted, {} implementation-complete, {} proof-complete, {} blocked",
            self.metrics.matrix.total_rows,
            self.metrics.matrix.owner_accepted,
            self.metrics.matrix.implementation_complete,
            self.metrics.matrix.proof_complete,
            self.metrics.matrix.blocked
        );
        let _ = writeln!(
            output,
            "workspace: {} members, {} production crates",
            self.metrics.workspace.members, self.metrics.workspace.production_crates
        );
        let _ = writeln!(
            output,
            "source: {} production modules, {} test modules",
            self.metrics.source.production_modules, self.metrics.source.test_modules
        );
        let _ = writeln!(
            output,
            "authority: {} files",
            self.metrics.authority_files
        );

        for finding in &self.findings {
            let _ = writeln!(
                output,
                "[{}] {}{}: {}",
                finding.severity.as_str(),
                finding.code,
                finding
                    .path
                    .as_deref()
                    .map_or_else(String::new, |path| format!(" ({path})")),
                finding.message
            );
        }

        output
    }

    fn render_json(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(output, "{{");
        let _ = writeln!(output, "  \"schema_version\": {SCHEMA_VERSION},");
        let _ = writeln!(
            output,
            "  \"status\": \"{}\",",
            if self.is_success() { "pass" } else { "fail" }
        );
        let _ = writeln!(output, "  \"metrics\": {{");
        let _ = writeln!(output, "    \"matrix\": {{");
        let _ = writeln!(
            output,
            "      \"total_rows\": {},",
            self.metrics.matrix.total_rows
        );
        let _ = writeln!(
            output,
            "      \"owner_accepted\": {},",
            self.metrics.matrix.owner_accepted
        );
        let _ = writeln!(
            output,
            "      \"implementation_complete\": {},",
            self.metrics.matrix.implementation_complete
        );
        let _ = writeln!(
            output,
            "      \"proof_complete\": {},",
            self.metrics.matrix.proof_complete
        );
        let _ = writeln!(output, "      \"blocked\": {}", self.metrics.matrix.blocked);
        let _ = writeln!(output, "    }},");
        let _ = writeln!(output, "    \"workspace\": {{");
        let _ = writeln!(
            output,
            "      \"members\": {},",
            self.metrics.workspace.members
        );
        let _ = writeln!(
            output,
            "      \"production_crates\": {}",
            self.metrics.workspace.production_crates
        );
        let _ = writeln!(output, "    }},");
        let _ = writeln!(output, "    \"source\": {{");
        let _ = writeln!(
            output,
            "      \"production_modules\": {},",
            self.metrics.source.production_modules
        );
        let _ = writeln!(
            output,
            "      \"test_modules\": {}",
            self.metrics.source.test_modules
        );
        let _ = writeln!(output, "    }},");
        let _ = writeln!(output, "    \"authority\": {{");
        let _ = writeln!(output, "      \"files\": {}", self.metrics.authority_files);
        let _ = writeln!(output, "    }}");
        let _ = writeln!(output, "  }},");
        let _ = writeln!(output, "  \"findings\": [");

        for (index, finding) in self.findings.iter().enumerate() {
            let comma = if index + 1 == self.findings.len() {
                ""
            } else {
                ","
            };
            let path = finding.path.as_deref().map_or_else(
                || "null".to_owned(),
                |path| format!("\"{}\"", json_escape(path)),
            );
            let _ = writeln!(
                output,
                "    {{\"severity\":\"{}\",\"code\":\"{}\",\"path\":{},\"message\":\"{}\"}}{}",
                finding.severity.as_str(),
                finding.code,
                path,
                json_escape(&finding.message),
                comma
            );
        }

        let _ = writeln!(output, "  ]");
        let _ = writeln!(output, "}}");
        output
    }
}

pub fn parse_output_format(
    arguments: impl Iterator<Item = String>,
) -> Result<OutputFormat, String> {
    let arguments = arguments.collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(OutputFormat::Human),
        [flag, format] if flag == "--format" && format == "json" => Ok(OutputFormat::Json),
        _ => Err("usage: cargo xtask audit-repository [--format json]".to_owned()),
    }
}

pub fn run(root: &Path, format: OutputFormat) -> Result<(), String> {
    let report = build_report(root)?;
    match format {
        OutputFormat::Human => print!("{}", report.render_human()),
        OutputFormat::Json => print!("{}", report.render_json()),
    }

    if report.is_success() {
        Ok(())
    } else {
        Err(report.render_failure())
    }
}

pub fn validate_fatal(root: &Path) -> Result<(), String> {
    let report = build_report(root)?;
    if report.is_success() {
        eprintln!(
            "> repository authority audit passed ({} diagnostics available through `cargo xtask audit-repository`)",
            report.diagnostic_count()
        );
        Ok(())
    } else {
        Err(report.render_failure())
    }
}

fn build_report(root: &Path) -> Result<AuditReport, String> {
    let mut report = AuditReport::default();

    if let Err(error) = super::validate_repository_metadata(root) {
        report.findings.push(Finding::fatal(
            "metadata.license_or_publish_policy",
            Some("Cargo.toml".to_owned()),
            error,
        ));
    }

    audit_repository_governance(root, &mut report.findings)?;
    audit_documentation_authority(root, &mut report.findings)?;
    let matrix = matrix::audit(root, &mut report.findings)?;
    let workspace = workspace::audit(root, &mut report.findings)?;
    let source = source::audit(root, &mut report.findings)?;
    let authority_files = collect_authority_files(root)?.len();

    report.metrics = AuditMetrics {
        matrix,
        workspace,
        source,
        authority_files,
    };
    report.finalize();
    Ok(report)
}

fn audit_repository_governance(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let manifest = read_to_string(root, "Cargo.toml")?;
    if !manifest.contains(CURRENT_REPOSITORY_DECLARATION) {
        findings.push(Finding::fatal(
            "metadata.current_repository_identity",
            Some("Cargo.toml".to_owned()),
            format!("workspace metadata must contain {CURRENT_REPOSITORY_DECLARATION:?}"),
        ));
    }
    if manifest.contains(HISTORICAL_OWNER_TOKEN) {
        findings.push(Finding::fatal(
            "metadata.historical_repository_identity",
            Some("Cargo.toml".to_owned()),
            "active workspace metadata must use the dornglut/runen-ui repository identity",
        ));
    }

    for relative in REQUIRED_ENTRYPOINTS {
        if !root.join(relative).is_file() {
            findings.push(Finding::fatal(
                "repository.required_entrypoint_missing",
                Some((*relative).to_owned()),
                "required repository/documentation entrypoint is missing",
            ));
        }
    }

    for relative in RETIRED_AUTHORITY_PATHS {
        if root.join(relative).exists() {
            findings.push(Finding::fatal(
                "authority.retired_path_present",
                Some((*relative).to_owned()),
                "retired duplicate-authority path must not reappear after the documentation cutover",
            ));
        }
    }

    audit_issue_template_inventory(root, findings)?;
    audit_ci_workflow(root, findings)?;
    Ok(())
}

fn audit_issue_template_inventory(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let found = collect_direct_file_names(&root.join(ISSUE_TEMPLATE_DIRECTORY))?;
    let expected = REQUIRED_ISSUE_TEMPLATE_FILES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    if found != expected {
        findings.push(Finding::fatal(
            "repository.issue_template_inventory",
            Some(ISSUE_TEMPLATE_DIRECTORY.to_owned()),
            format!("expected issue-template files {expected:?}, found {found:?}"),
        ));
    }

    let issue_config = read_to_string(root, ".github/ISSUE_TEMPLATE/config.yml")?;
    for required in [
        "blank_issues_enabled: false",
        "https://github.com/dornglut/runen-ui/security/policy",
        "https://github.com/dornglut/engineering/issues/new",
    ] {
        if !issue_config.contains(required) {
            findings.push(Finding::fatal(
                "repository.issue_template_config",
                Some(".github/ISSUE_TEMPLATE/config.yml".to_owned()),
                format!("issue configuration is missing required contract {required:?}"),
            ));
        }
    }
    Ok(())
}

fn audit_ci_workflow(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let found = collect_direct_workflow_names(&root.join(ACTIVE_WORKFLOW_DIRECTORY))?;
    let expected = EXPECTED_ACTIVE_WORKFLOW_FILES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    if found != expected {
        findings.push(Finding::fatal(
            "repository.workflow_inventory",
            Some(ACTIVE_WORKFLOW_DIRECTORY.to_owned()),
            format!("expected active workflow files {expected:?}, found {found:?}"),
        ));
    }

    let actual = read_to_string(root, CI_WORKFLOW_PATH)?;
    if normalize_newlines(&actual) != normalize_newlines(&expected_ci_workflow()) {
        findings.push(Finding::fatal(
            "repository.workflow_contract",
            Some(CI_WORKFLOW_PATH.to_owned()),
            format!(
                "CI must remain the exact read-only immutable reusable-workflow caller pinned to {}",
                accepted_reusable_workflow_reference()
            ),
        ));
    }
    Ok(())
}

fn audit_documentation_authority(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let files = collect_authority_files(root)?;
    for relative in &files {
        let relative_text = path_text(relative);
        let contents = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("failed to read authority file {relative_text}: {error}"))?;

        if contents.contains(PRIVATE_ARCHIVE_URL) && relative_text != MIGRATION_HISTORY_PATH {
            findings.push(Finding::fatal(
                "authority.private_archive_reference",
                Some(relative_text.clone()),
                format!("private archive URLs are permitted only in {MIGRATION_HISTORY_PATH}"),
            ));
        }

        if !is_historical_owner_exemption(relative) && contents.contains(HISTORICAL_OWNER_TOKEN) {
            findings.push(Finding::fatal(
                "authority.active_historical_owner_reference",
                Some(relative_text.clone()),
                "active authority files must use the dornglut/runen-ui repository identity",
            ));
        }

        if !is_historical_master_exemption(relative)
            && (contents.contains("`master`") || contents.contains("refs/heads/master"))
        {
            findings.push(Finding::fatal(
                "authority.active_master_reference",
                Some(relative_text.clone()),
                "active authority files must use `main`; `master` is permitted only in explicit historical records",
            ));
        }
    }

    for relative in DURABLE_CURRENT_AUTHORITY_FILES {
        let contents = read_to_string(root, relative)?;
        audit_durable_current_document(relative, &contents, findings);
    }

    Ok(())
}

fn audit_durable_current_document(relative: &str, contents: &str, findings: &mut Vec<Finding>) {
    const FORBIDDEN_URLS: &[&str] = &[
        "https://github.com/dornglut/runen-ui/issues/",
        "https://github.com/dornglut/runen-ui/pull/",
        "https://github.com/dornglut/runen-ui/actions/runs/",
    ];

    for prefix in FORBIDDEN_URLS {
        if contents.contains(prefix) {
            findings.push(Finding::fatal(
                "authority.volatile_github_state",
                Some(relative.to_owned()),
                format!("durable current authority must not mirror live RunenUI GitHub state via {prefix:?}"),
            ));
        }
    }

    if contains_full_lowercase_sha(contents) {
        findings.push(Finding::fatal(
            "authority.volatile_commit_sha",
            Some(relative.to_owned()),
            "durable current authority must not embed full commit SHAs; delivery/history evidence belongs in GitHub or history/report records",
        ));
    }

    for marker in ["CI #", "workflow run #", "current head:", "remote head:", "next unblocked issue:"] {
        if contents.contains(marker) {
            findings.push(Finding::fatal(
                "authority.volatile_execution_marker",
                Some(relative.to_owned()),
                format!("durable current authority contains volatile execution marker {marker:?}"),
            ));
        }
    }
}

fn contains_full_lowercase_sha(contents: &str) -> bool {
    let bytes = contents.as_bytes();
    if bytes.len() < 40 {
        return false;
    }

    (0..=bytes.len() - 40).any(|start| {
        let candidate = &bytes[start..start + 40];
        candidate.iter().copied().all(is_lower_hex)
            && start.checked_sub(1).is_none_or(|index| !is_lower_hex(bytes[index]))
            && bytes
                .get(start + 40)
                .copied()
                .is_none_or(|byte| !is_lower_hex(byte))
    })
}

const fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')
}

fn collect_direct_file_names(directory: &Path) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| format!("failed to inspect {} entry: {error}", directory.display()))?;
        if entry.path().is_file() {
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| format!("non-UTF-8 file name below {}", directory.display()))?;
            names.insert(name);
        }
    }
    Ok(names)
}

fn collect_direct_workflow_names(directory: &Path) -> Result<BTreeSet<String>, String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?;
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_file()
            && matches!(
                path.extension().and_then(OsStr::to_str),
                Some("yml" | "yaml")
            )
        {
            let name = entry.file_name().into_string().map_err(|_| {
                format!("non-UTF-8 workflow file name below {}", directory.display())
            })?;
            names.insert(name);
        }
    }
    Ok(names)
}

fn collect_authority_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_authority_files_from(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_authority_files_from(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to inspect an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            if !super::is_ignored_directory(&path) {
                collect_authority_files_from(root, &path, files)?;
            }
            continue;
        }

        if matches!(
            path.extension().and_then(OsStr::to_str),
            Some("md" | "yml" | "yaml")
        ) {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("failed to relativize {}: {error}", path.display()))?;
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn is_historical_owner_exemption(relative: &Path) -> bool {
    relative == Path::new("CHANGELOG.md")
        || relative.starts_with("docs/adr")
        || relative.starts_with("docs/history")
        || relative.starts_with("docs/reports")
}

fn is_historical_master_exemption(relative: &Path) -> bool {
    relative == Path::new("CHANGELOG.md")
        || relative.starts_with("docs/history")
        || relative.starts_with("docs/reports")
}

fn read_to_string(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(output, "\\u{:04x}", u32::from(character));
            }
            character => output.push(character),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        process,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{
        AuditReport, OutputFormat, PRIVATE_ARCHIVE_URL, build_report, contains_full_lowercase_sha,
        expected_ci_workflow, json_escape, parse_output_format,
    };

    static NEXT_TEMP_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        path: std::path::PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Result<Self, String> {
            let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "runenui-repository-audit-{label}-{}-{sequence}",
                process::id()
            ));
            fs::create_dir_all(&path)
                .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
            let fixture = Self { path };
            fixture.write_baseline()?;
            Ok(fixture)
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative: &str, contents: &str) -> Result<(), String> {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            fs::write(&path, contents)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))
        }

        #[allow(clippy::too_many_lines)]
        fn write_baseline(&self) -> Result<(), String> {
            self.write(
                "Cargo.toml",
                "[workspace]\nmembers = [\n  \"crates/runenui_core\",\n  \"crates/runenui_runtime\",\n  \"xtask\",\n]\n\n[workspace.package]\nrepository = \"https://github.com/dornglut/runen-ui\"\nlicense = \"MIT\"\npublish = false\n",
            )?;
            self.write(
                "LICENSE",
                "MIT License\n\nCopyright (c) 2026 Crystonix\n\nPermission is hereby granted, free of charge\nTHE SOFTWARE IS PROVIDED \"AS IS\"\n",
            )?;
            for relative in [
                "README.md",
                "AGENTS.md",
                "ARCHITECTURE.md",
                "TESTING.md",
                "docs/README.md",
                "docs/documentation-architecture.md",
                "docs/roadmap.md",
                "docs/status.md",
                "docs/vocabulary.md",
                "docs/architecture/README.md",
                "docs/architecture/public-api.md",
                "docs/conformance/README.md",
            ] {
                self.write(relative, "# Authority\n")?;
            }
            self.write(
                ".github/ISSUE_TEMPLATE/config.yml",
                "blank_issues_enabled: false\ncontact_links:\n  - name: Security\n    url: https://github.com/dornglut/runen-ui/security/policy\n    about: Private security reporting.\n  - name: Engineering\n    url: https://github.com/dornglut/engineering/issues/new\n    about: Cross-repository decisions.\n",
            )?;
            self.write(".github/ISSUE_TEMPLATE/defect.yml", "name: Defect\n")?;
            self.write(".github/ISSUE_TEMPLATE/milestone-slice.yml", "name: Milestone slice\n")?;
            self.write(".github/ISSUE_TEMPLATE/proposal.yml", "name: Proposal\n")?;
            self.write(".github/workflows/ci.yml", &expected_ci_workflow())?;
            self.write(
                "crates/runenui_core/Cargo.toml",
                "[package]\nname = \"runenui_core\"\n\n[dependencies]\n",
            )?;
            self.write("crates/runenui_core/src/lib.rs", "#![forbid(unsafe_code)]\n")?;
            self.write(
                "crates/runenui_runtime/Cargo.toml",
                "[package]\nname = \"runenui_runtime\"\n\n[dependencies]\nrunenui_core = { path = \"../runenui_core\" }\n",
            )?;
            self.write(
                "crates/runenui_runtime/src/queue.rs",
                "pub(crate) struct WorkQueue<Action> { value: Option<Action> }\n",
            )?;
            self.write("crates/runenui_runtime/src/trace/store.rs", "pub struct Trace;\n")?;
            self.write(
                "crates/runenui_runtime/src/runtime/surface_publication.rs",
                "pub(crate) struct SurfacePublicationState;\n",
            )?;
            self.write("xtask/Cargo.toml", "[package]\nname = \"xtask\"\n\n[dependencies]\n")?;
            self.write("xtask/src/main.rs", "fn main() {}\n")?;
            self.write(
                "docs/architecture/workspace-structure.md",
                "| Package | Current ownership | Must not own |\n|---|---|---|\n| `runenui_core` | Core | Runtime |\n| `runenui_runtime` | Runtime | Platform |\n| `xtask` | Tooling | Runtime |\n",
            )?;
            for (path, id, slice) in [
                ("docs/conformance/m4-conformance-matrix.md", "PTR-01", "M4C3"),
                ("docs/conformance/m5-conformance-matrix.md", "SEM-ID-01", "M5A"),
                ("docs/conformance/m6-conformance-matrix.md", "SCENE-PUB-01", "M6A"),
            ] {
                self.write(
                    path,
                    &format!(
                        "```text\n1 total unique rows\n0 owner-accepted\n0 implementation-complete\n0 proof-complete\n1 blocked\n0 duplicate IDs\n0 invalid statuses\n0 invalid schemas\n```\n\n| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |\n|---|---|---|---|---|---|---|---|\n| {id} | observation | positive | negative | diagnostic | {slice} | blocked | Required |\n"
                    ),
                )?;
            }
            self.write(
                "docs/history/public-repository-migration.md",
                &format!("https://{PRIVATE_ARCHIVE_URL}\n"),
            )?;
            Ok(())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn assert_fatal_code(report: &AuditReport, expected: &str) {
        assert!(
            report.findings.iter().any(|finding| finding.code == expected),
            "expected fatal code {expected:?}, got {:?}",
            report
                .findings
                .iter()
                .map(|finding| finding.code)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn output_format_contract_is_exact() {
        assert_eq!(
            parse_output_format(std::iter::empty()),
            Ok(OutputFormat::Human)
        );
        assert_eq!(
            parse_output_format(["--format".to_owned(), "json".to_owned()].into_iter()),
            Ok(OutputFormat::Json)
        );
        assert!(parse_output_format(["--json".to_owned()].into_iter()).is_err());
    }

    #[test]
    fn sha_detector_requires_exact_lowercase_hex_run() {
        assert!(contains_full_lowercase_sha(
            "x 0123456789abcdef0123456789abcdef01234567 y"
        ));
        assert!(!contains_full_lowercase_sha(
            "0123456789abcdef0123456789abcdef0123456"
        ));
        assert!(!contains_full_lowercase_sha(
            "0123456789ABCDEF0123456789ABCDEF01234567"
        ));
    }

    #[test]
    fn json_escape_handles_control_characters() {
        assert_eq!(json_escape("a\n\"b\\c"), "a\\n\\\"b\\\\c");
    }

    #[test]
    fn baseline_fixture_passes_and_json_is_deterministic() -> Result<(), String> {
        let fixture = Fixture::new("baseline")?;
        let first = build_report(fixture.path())?;
        let second = build_report(fixture.path())?;
        assert!(first.is_success(), "{}", first.render_failure());
        assert_eq!(first.render_json(), second.render_json());
        Ok(())
    }

    #[test]
    fn workflow_contract_rejects_mutation() -> Result<(), String> {
        let fixture = Fixture::new("workflow")?;
        fixture.write(
            ".github/workflows/ci.yml",
            &expected_ci_workflow().replace("contents: read", "contents: write"),
        )?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "repository.workflow_contract");
        Ok(())
    }

    #[test]
    fn retired_authority_path_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("retired")?;
        fixture.write("docs/work-tracking.md", "# retired\n")?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "authority.retired_path_present");
        Ok(())
    }

    #[test]
    fn live_issue_link_in_durable_current_authority_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("issue")?;
        fixture.write(
            "README.md",
            "https://github.com/dornglut/runen-ui/issues/999\n",
        )?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "authority.volatile_github_state");
        Ok(())
    }

    #[test]
    fn commit_sha_in_durable_current_authority_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("sha")?;
        fixture.write(
            "docs/status.md",
            "accepted 0123456789abcdef0123456789abcdef01234567\n",
        )?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "authority.volatile_commit_sha");
        Ok(())
    }

    #[test]
    fn private_archive_link_outside_history_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("archive")?;
        fixture.write("README.md", &format!("https://{PRIVATE_ARCHIVE_URL}\n"))?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "authority.private_archive_reference");
        Ok(())
    }

    #[test]
    fn historical_owner_link_in_active_authority_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("owner")?;
        fixture.write(
            "README.md",
            "https://github.com/Crystonix/runen-ui/issues/3\n",
        )?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "authority.active_historical_owner_reference");
        Ok(())
    }

    #[test]
    fn old_active_branch_terminology_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("master")?;
        fixture.write("AGENTS.md", "Start from current `master`.\n")?;
        let report = build_report(fixture.path())?;
        assert_fatal_code(&report, "authority.active_master_reference");
        Ok(())
    }
}
