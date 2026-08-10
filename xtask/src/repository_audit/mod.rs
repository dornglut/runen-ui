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

const SCHEMA_VERSION: u32 = 1;
const PUBLIC_ISSUE_PREFIX: &str = "https://github.com/dornglut/runen-ui/issues/";
const PRIVATE_ARCHIVE_URL: &str = "github.com/Crystonix/runen-ui-private-archive";
const HISTORICAL_OWNER_TOKEN: &str = "Crystonix/runen-ui";
const CURRENT_REPOSITORY_DECLARATION: &str =
    "repository = \"https://github.com/dornglut/runen-ui\"";
const ACCEPTED_REUSABLE_WORKFLOW_REVISION: &str = "624cb41adeed21a6461eb838bc7330bd0a5079fd";
const REUSABLE_WORKFLOW_OWNER_AND_DIRECTORY: &str = "dornglut/github-workflows/.github/workflows";
const REUSABLE_RUST_WORKFLOW: &str = "reusable-rust-cargo-validate.yml";
const ACTIVE_WORKFLOW_DIRECTORY: &str = ".github/workflows";
const CI_WORKFLOW_PATH: &str = ".github/workflows/ci.yml";
const EXPECTED_ACTIVE_WORKFLOW_FILES: &[&str] = &["ci.yml"];
const WORK_TRACKING_PATH: &str = "docs/work-tracking.md";
const MIGRATION_HISTORY_PATH: &str = "docs/history/public-repository-migration.md";
const ISSUE_TEMPLATE_DIRECTORY: &str = ".github/ISSUE_TEMPLATE";
const REQUIRED_ENTRYPOINTS: &[&str] = &["README.md", "AGENTS.md", "ARCHITECTURE.md", "TESTING.md"];
const REQUIRED_ISSUE_TEMPLATE_FILES: &[&str] = &[
    "config.yml",
    "defect.yml",
    "milestone-slice.yml",
    "proposal.yml",
];

fn accepted_reusable_workflow_reference() -> String {
    format!(
        "{REUSABLE_WORKFLOW_OWNER_AND_DIRECTORY}/{REUSABLE_RUST_WORKFLOW}@{ACCEPTED_REUSABLE_WORKFLOW_REVISION}"
    )
}

fn accepted_reusable_workflow_call() -> String {
    format!("uses: {}", accepted_reusable_workflow_reference())
}

fn expected_ci_workflow_lines() -> Vec<String> {
    vec![
        "name: CI".to_owned(),
        "on:".to_owned(),
        "  pull_request:".to_owned(),
        "  push:".to_owned(),
        "    branches:".to_owned(),
        "      - main".to_owned(),
        "permissions:".to_owned(),
        "  contents: read".to_owned(),
        "concurrency:".to_owned(),
        "  group: ci-${{ github.workflow }}-${{ github.ref }}".to_owned(),
        "  cancel-in-progress: true".to_owned(),
        "jobs:".to_owned(),
        "  validate:".to_owned(),
        "    name: RunenUI validation".to_owned(),
        format!("    {}", accepted_reusable_workflow_call()),
    ]
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
    modeled_public_issues: usize,
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
            "authority: {} files, {} modeled public issues",
            self.metrics.authority_files, self.metrics.modeled_public_issues
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
        let _ = writeln!(output, "      \"files\": {},", self.metrics.authority_files);
        let _ = writeln!(
            output,
            "      \"modeled_public_issues\": {}",
            self.metrics.modeled_public_issues
        );
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
    let matrix = matrix::audit(root, &mut report.findings)?;
    let workspace = workspace::audit(root, &mut report.findings)?;
    let source = source::audit(root, &mut report.findings)?;
    let (authority_files, modeled_public_issues) =
        audit_authority_documents(root, &mut report.findings)?;

    report.metrics = AuditMetrics {
        matrix,
        workspace,
        source,
        authority_files,
        modeled_public_issues,
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
            error,
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
                "required repository entrypoint is missing",
            ));
        }
    }

    let issue_directory = root.join(ISSUE_TEMPLATE_DIRECTORY);
    let found_issue_files = collect_direct_file_names(&issue_directory)?;
    let expected_issue_files = REQUIRED_ISSUE_TEMPLATE_FILES
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<BTreeSet<_>>();
    if found_issue_files != expected_issue_files {
        findings.push(Finding::fatal(
            "repository.issue_template_inventory",
            Some(ISSUE_TEMPLATE_DIRECTORY.to_owned()),
            format!(
                "expected issue-template files {expected_issue_files:?}, found {found_issue_files:?}"
            ),
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

    audit_active_workflow_inventory(root, findings)?;
    audit_ci_workflow_contract(root, findings)?;

    Ok(())
}

fn audit_authority_documents(
    root: &Path,
    findings: &mut Vec<Finding>,
) -> Result<(usize, usize), String> {
    let work_tracking = read_to_string(root, WORK_TRACKING_PATH)?;
    let modeled_issues = issue_numbers(&work_tracking);

    if !modeled_issues.contains(&3) {
        findings.push(Finding::fatal(
            "authority.public_issue_model_missing_umbrella",
            Some(WORK_TRACKING_PATH.to_owned()),
            "the public authority model must include M4 umbrella issue #3",
        ));
    }

    let files = collect_authority_files(root)?;
    for relative in &files {
        let relative_text = path_text(relative);
        let contents = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("failed to read authority file {relative_text}: {error}"))?;

        for issue in issue_numbers(&contents) {
            if !modeled_issues.contains(&issue) {
                findings.push(Finding::fatal(
                    "authority.public_issue_not_modeled",
                    Some(relative_text.clone()),
                    format!(
                        "public issue #{issue} is linked outside the issue set documented by {WORK_TRACKING_PATH}"
                    ),
                ));
            }
        }

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

    Ok((files.len(), modeled_issues.len()))
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

fn collect_active_workflow_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let directory = root.join(ACTIVE_WORKFLOW_DIRECTORY);
    let entries = fs::read_dir(&directory)
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
        let is_workflow = path.is_file()
            && matches!(
                path.extension().and_then(OsStr::to_str),
                Some("yml" | "yaml")
            );
        if is_workflow {
            let name = entry.file_name().into_string().map_err(|_| {
                format!("non-UTF-8 workflow file name below {}", directory.display())
            })?;
            names.insert(name);
        }
    }

    Ok(names)
}

fn normalize_workflow_lines(contents: &str) -> Vec<&str> {
    contents
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect()
}

fn is_full_lowercase_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn audit_active_workflow_inventory(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let found = collect_active_workflow_files(root)?;
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
    Ok(())
}

fn audit_ci_workflow_contract(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    let path = root.join(CI_WORKFLOW_PATH);
    if !path.is_file() {
        return Ok(());
    }

    let contents = read_to_string(root, CI_WORKFLOW_PATH)?;
    let lines = normalize_workflow_lines(&contents);
    let expected = expected_ci_workflow_lines();

    audit_ci_identity_and_events(&lines, &expected, findings);
    audit_ci_permissions_concurrency_and_job(&lines, &expected, findings);
    audit_ci_reusable_reference(&lines, findings);
    audit_ci_unexpected_fields(&lines, &expected, findings);

    Ok(())
}

fn audit_ci_identity_and_events(lines: &[&str], expected: &[String], findings: &mut Vec<Finding>) {
    if lines.first().copied() != Some(expected[0].as_str()) {
        findings.push(Finding::fatal(
            "repository.workflow_identity",
            Some(CI_WORKFLOW_PATH.to_owned()),
            "workflow name must be exactly `CI`",
        ));
    }

    let permissions_index = workflow_header_index(lines, "permissions:");
    let concurrency_index = workflow_header_index(lines, "concurrency:");
    let jobs_index = workflow_header_index(lines, "jobs:");
    let event_end = [permissions_index, concurrency_index, jobs_index]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(lines.len());
    let expected_events = expected[1..6]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if lines.get(1..event_end) != Some(expected_events.as_slice()) {
        findings.push(Finding::fatal(
            "repository.workflow_trigger",
            Some(CI_WORKFLOW_PATH.to_owned()),
            "workflow must contain only unconfigured pull_request and push triggers",
        ));
        if !lines.iter().any(|line| *line == expected[5]) {
            findings.push(Finding::fatal(
                "repository.workflow_branch",
                Some(CI_WORKFLOW_PATH.to_owned()),
                "push trigger must be restricted to main",
            ));
        }
    }
}

fn audit_ci_permissions_concurrency_and_job(
    lines: &[&str],
    expected: &[String],
    findings: &mut Vec<Finding>,
) {
    let permissions_index = workflow_header_index(lines, "permissions:");
    let concurrency_index = workflow_header_index(lines, "concurrency:");
    let jobs_index = workflow_header_index(lines, "jobs:");
    let permission_end = [concurrency_index, jobs_index]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(lines.len());
    let expected_permissions = expected[6..8]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if permissions_index.and_then(|start| lines.get(start..permission_end))
        != Some(expected_permissions.as_slice())
    {
        findings.push(Finding::fatal(
            "repository.workflow_permission",
            Some(CI_WORKFLOW_PATH.to_owned()),
            "workflow must declare only top-level contents: read permission",
        ));
    }
    let expected_concurrency = expected[8..11]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if concurrency_index.and_then(|start| lines.get(start..jobs_index.unwrap_or(lines.len())))
        != Some(expected_concurrency.as_slice())
    {
        findings.push(Finding::fatal(
            "repository.workflow_concurrency",
            Some(CI_WORKFLOW_PATH.to_owned()),
            "workflow must preserve the exact CI concurrency group and cancellation policy",
        ));
    }
    let expected_jobs = expected[11..]
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if jobs_index.and_then(|start| lines.get(start..)) != Some(expected_jobs.as_slice()) {
        findings.push(Finding::fatal(
            "repository.workflow_job",
            Some(CI_WORKFLOW_PATH.to_owned()),
            "workflow must contain only the named reusable validate job",
        ));
    }
}

fn audit_ci_reusable_reference(lines: &[&str], findings: &mut Vec<Finding>) {
    let reusable_calls = lines
        .iter()
        .filter_map(|line| line.trim_start().strip_prefix("uses: "))
        .collect::<Vec<_>>();
    let accepted_call = accepted_reusable_workflow_call();
    let accepted_reference = accepted_reusable_workflow_reference();
    if reusable_calls != [accepted_reference.as_str()] {
        for call in &reusable_calls {
            match call.rsplit_once('@') {
                Some((_, revision)) if !is_full_lowercase_commit_sha(revision) => findings.push(
                    Finding::fatal(
                        "repository.workflow_profile_revision",
                        Some(CI_WORKFLOW_PATH.to_owned()),
                        format!("reusable workflow must use a full lowercase commit SHA, found {revision:?}"),
                    ),
                ),
                None => findings.push(Finding::fatal(
                    "repository.workflow_profile_revision",
                    Some(CI_WORKFLOW_PATH.to_owned()),
                    "reusable workflow reference must include an immutable revision",
                )),
                Some(_) => {}
            }
        }
        findings.push(Finding::fatal(
            "repository.workflow_profile_revision",
            Some(CI_WORKFLOW_PATH.to_owned()),
            format!(
                "expected accepted reusable workflow {accepted_call:?}, found {reusable_calls:?}"
            ),
        ));
    }
}

fn audit_ci_unexpected_fields(lines: &[&str], expected: &[String], findings: &mut Vec<Finding>) {
    let expected_lines = expected.iter().map(String::as_str).collect::<Vec<_>>();
    if lines != expected_lines {
        let unexpected = lines
            .iter()
            .filter(|line| !expected_lines.contains(line))
            .copied()
            .collect::<Vec<_>>();
        if !unexpected.is_empty() {
            findings.push(Finding::fatal(
                "repository.workflow_unexpected_field",
                Some(CI_WORKFLOW_PATH.to_owned()),
                format!("unexpected workflow field or ordering drift: {unexpected:?}"),
            ));
        }
    }
}

fn workflow_header_index(lines: &[&str], header: &str) -> Option<usize> {
    lines.iter().position(|line| *line == header)
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

fn issue_numbers(contents: &str) -> BTreeSet<u64> {
    let mut issues = BTreeSet::new();
    let mut remaining = contents;

    while let Some(index) = remaining.find(PUBLIC_ISSUE_PREFIX) {
        let after_prefix = &remaining[index + PUBLIC_ISSUE_PREFIX.len()..];
        let digits = after_prefix
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if let Ok(issue) = digits.parse::<u64>() {
            issues.insert(issue);
        }
        remaining = after_prefix;
    }

    issues
}

fn is_historical_owner_exemption(relative: &Path) -> bool {
    relative == Path::new("CHANGELOG.md")
        || relative.starts_with("docs/adr")
        || relative.starts_with("docs/history")
}

fn is_historical_master_exemption(relative: &Path) -> bool {
    relative == Path::new("CHANGELOG.md") || relative.starts_with("docs/history")
}

fn read_to_string(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
        AuditReport, OutputFormat, PRIVATE_ARCHIVE_URL, audit_active_workflow_inventory,
        build_report, expected_ci_workflow_lines, issue_numbers, json_escape, parse_output_format,
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

        fn remove(&self, relative: &str) -> Result<(), String> {
            fs::remove_file(self.path.join(relative))
                .map_err(|error| format!("failed to remove {relative}: {error}"))
        }

        fn write_ci_workflow(&self, contents: &str) -> Result<(), String> {
            self.write(".github/workflows/ci.yml", contents)
        }

        fn write_additional_workflow(&self, name: &str) -> Result<(), String> {
            self.write(&format!(".github/workflows/{name}"), "name: Extra\n")
        }

        fn write_baseline(&self) -> Result<(), String> {
            self.write(
                "Cargo.toml",
                "[workspace]\nmembers = [\n  \"crates/runenui_core\",\n  \"crates/runenui_runtime\",\n  \"xtask\",\n]\n\n[workspace.package]\nrepository = \"https://github.com/dornglut/runen-ui\"\nlicense = \"MIT\"\npublish = false\n",
            )?;
            self.write(
                "LICENSE",
                "MIT License\n\nCopyright (c) 2026 Crystonix\n\nPermission is hereby granted, free of charge\nTHE SOFTWARE IS PROVIDED \"AS IS\"\n",
            )?;
            self.write("README.md", "# RunenUI\n")?;
            self.write("AGENTS.md", "Start from current `main`.\n")?;
            self.write("ARCHITECTURE.md", "# Architecture\n")?;
            self.write("TESTING.md", "# Testing\n")?;
            self.write(
                ".github/ISSUE_TEMPLATE/config.yml",
                "blank_issues_enabled: false\ncontact_links:\n  - name: Security\n    url: https://github.com/dornglut/runen-ui/security/policy\n    about: Private security reporting.\n  - name: Engineering\n    url: https://github.com/dornglut/engineering/issues/new\n    about: Cross-repository decisions.\n",
            )?;
            self.write(".github/ISSUE_TEMPLATE/defect.yml", "name: Defect\n")?;
            self.write(
                ".github/ISSUE_TEMPLATE/milestone-slice.yml",
                "name: Milestone slice\n",
            )?;
            self.write(".github/ISSUE_TEMPLATE/proposal.yml", "name: Proposal\n")?;
            self.write_ci_workflow(&accepted_ci_workflow())?;
            self.write(
                "crates/runenui_core/Cargo.toml",
                "[package]\nname = \"runenui_core\"\n\n[dependencies]\n",
            )?;
            self.write(
                "crates/runenui_core/src/lib.rs",
                "#![forbid(unsafe_code)]\n",
            )?;
            self.write(
                "crates/runenui_runtime/Cargo.toml",
                "[package]\nname = \"runenui_runtime\"\n\n[dependencies]\nrunenui_core = { path = \"../runenui_core\" }\n",
            )?;
            self.write(
                "crates/runenui_runtime/src/queue.rs",
                "pub(crate) struct WorkQueue<Action> { value: Option<Action> }\n",
            )?;
            self.write(
                "crates/runenui_runtime/src/trace/store.rs",
                "pub struct Trace;\n",
            )?;
            self.write(
                "crates/runenui_runtime/src/runtime/surface_publication.rs",
                "pub(crate) struct SurfacePublicationState;\n",
            )?;
            self.write(
                "xtask/Cargo.toml",
                "[package]\nname = \"xtask\"\n\n[dependencies]\n",
            )?;
            self.write("xtask/src/main.rs", "fn main() {}\n")?;
            self.write(
                "docs/architecture/workspace-structure.md",
                "| Package | Current ownership | Must not own |\n|---|---|---|\n| `runenui_core` | Core | Runtime |\n| `runenui_runtime` | Runtime | Platform |\n| `xtask` | Tooling | Runtime |\n",
            )?;
            self.write(
                "docs/architecture/m4-conformance-matrix.md",
                "```text\n1 total unique rows\n0 owner-accepted\n0 implementation-complete\n0 proof-complete\n1 blocked\n0 duplicate IDs\n0 invalid statuses\n0 invalid schemas\n```\n\n| ID | Required observation | Positive proof owner | Negative proof owner | Trace proof owner | Delivery slice | Status | M4 gate |\n|---|---|---|---|---|---|---|---|\n| PTR-01 | observation | positive | negative | trace | M4C3 | blocked | Required |\n",
            )?;
            self.write(
                "docs/architecture/m5-conformance-matrix.md",
                "```text\n1 total unique rows\n0 owner-accepted\n0 implementation-complete\n0 proof-complete\n1 blocked\n0 duplicate IDs\n0 invalid statuses\n0 invalid schemas\n```\n\n| ID | Required observation | Positive proof owner | Negative proof owner | Diagnostic / trace proof owner | Delivery slice | Status | Gate |\n|---|---|---|---|---|---|---|---|\n| SEM-ID-01 | observation | positive | negative | diagnostic | M5A | blocked | Required |\n",
            )?;
            self.write(
                "docs/work-tracking.md",
                "[umbrella](https://github.com/dornglut/runen-ui/issues/3)\n",
            )?;
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

    fn accepted_ci_workflow() -> String {
        format!("{}\n", expected_ci_workflow_lines().join("\n"))
    }

    fn replace_once(contents: &str, from: &str, to: &str) -> String {
        assert!(contents.contains(from), "missing mutation anchor {from:?}");
        contents.replacen(from, to, 1)
    }

    fn assert_fatal_code(report: &AuditReport, label: &str, expected: &str) {
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == expected),
            "{label}: expected fatal code {expected:?}, got {:?}",
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
    fn public_issue_parser_is_sorted_and_unique() {
        let issues = issue_numbers(
            "https://github.com/dornglut/runen-ui/issues/11 and https://github.com/dornglut/runen-ui/issues/3 and https://github.com/dornglut/runen-ui/issues/11",
        );
        assert_eq!(issues.into_iter().collect::<Vec<_>>(), [3, 11]);
    }

    #[test]
    fn json_escape_handles_control_characters() {
        assert_eq!(json_escape("a\n\"b\\c"), "a\\n\\\"b\\\\c");
    }

    #[test]
    fn baseline_fixture_passes_and_json_is_deterministic() -> Result<(), String> {
        let fixture = Fixture::new("baseline")?;
        let mut inventory_findings = Vec::new();
        audit_active_workflow_inventory(fixture.path(), &mut inventory_findings)?;
        assert!(inventory_findings.is_empty());
        let first = build_report(fixture.path())?;
        let second = build_report(fixture.path())?;
        assert!(first.is_success(), "{}", first.render_failure());
        assert_eq!(first.render_json(), second.render_json());
        Ok(())
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One table keeps every required workflow mutation visible together.
    fn ci_workflow_contract_rejects_prohibited_mutations() -> Result<(), String> {
        let accepted = accepted_ci_workflow();
        let accepted_revision = "624cb41adeed21a6461eb838bc7330bd0a5079fd";
        let job_anchor = "    name: RunenUI validation\n";
        let mutations = vec![
            (
                "predecessor revision",
                replace_once(
                    &accepted,
                    accepted_revision,
                    "b6caad377102ca73794efaf734a65903b8efa829",
                ),
                "repository.workflow_profile_revision",
            ),
            (
                "older revision",
                replace_once(
                    &accepted,
                    accepted_revision,
                    "79405c457b5b99d5cb9957c9bcdc475109e1e3bf",
                ),
                "repository.workflow_profile_revision",
            ),
            (
                "alternate full SHA",
                replace_once(
                    &accepted,
                    accepted_revision,
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                ),
                "repository.workflow_profile_revision",
            ),
            (
                "mutable main revision",
                replace_once(&accepted, accepted_revision, "main"),
                "repository.workflow_profile_revision",
            ),
            (
                "mutable tag revision",
                replace_once(&accepted, accepted_revision, "v1"),
                "repository.workflow_profile_revision",
            ),
            (
                "shortened revision",
                replace_once(&accepted, accepted_revision, "624cb41adeed"),
                "repository.workflow_profile_revision",
            ),
            (
                "missing revision",
                replace_once(&accepted, accepted_revision, ""),
                "repository.workflow_profile_revision",
            ),
            (
                "uppercase revision",
                replace_once(
                    &accepted,
                    accepted_revision,
                    "624CB41ADEED21A6461EB838BC7330BD0A5079FD",
                ),
                "repository.workflow_profile_revision",
            ),
            (
                "wrong workflow owner",
                replace_once(
                    &accepted,
                    "dornglut/github-workflows",
                    "other/github-workflows",
                ),
                "repository.workflow_profile_revision",
            ),
            (
                "wrong workflow repository",
                replace_once(&accepted, "github-workflows", "other-workflows"),
                "repository.workflow_profile_revision",
            ),
            (
                "wrong workflow filename",
                replace_once(&accepted, "reusable-rust-cargo-validate.yml", "other.yml"),
                "repository.workflow_profile_revision",
            ),
            (
                "Python profile swap",
                replace_once(
                    &accepted,
                    "reusable-rust-cargo-validate.yml",
                    "reusable-python-repository-validate.yml",
                ),
                "repository.workflow_profile_revision",
            ),
            (
                "missing pull request",
                replace_once(&accepted, "  pull_request:\n", ""),
                "repository.workflow_trigger",
            ),
            (
                "missing push",
                replace_once(&accepted, "  push:\n", ""),
                "repository.workflow_trigger",
            ),
            (
                "workflow dispatch",
                replace_once(
                    &accepted,
                    "  pull_request:\n",
                    "  workflow_dispatch:\n  pull_request:\n",
                ),
                "repository.workflow_trigger",
            ),
            (
                "schedule",
                replace_once(
                    &accepted,
                    "  pull_request:\n",
                    "  schedule:\n    - cron: weekly\n  pull_request:\n",
                ),
                "repository.workflow_trigger",
            ),
            (
                "pull request target",
                replace_once(
                    &accepted,
                    "  pull_request:\n",
                    "  pull_request_target:\n  pull_request:\n",
                ),
                "repository.workflow_trigger",
            ),
            (
                "pull request branch filter",
                replace_once(
                    &accepted,
                    "  pull_request:\n",
                    "  pull_request:\n    branches:\n      - main\n",
                ),
                "repository.workflow_trigger",
            ),
            (
                "pull request path filter",
                replace_once(
                    &accepted,
                    "  pull_request:\n",
                    "  pull_request:\n    paths:\n      - crates/**\n",
                ),
                "repository.workflow_trigger",
            ),
            (
                "pull request type filter",
                replace_once(
                    &accepted,
                    "  pull_request:\n",
                    "  pull_request:\n    types:\n      - opened\n",
                ),
                "repository.workflow_trigger",
            ),
            (
                "push branch drift",
                replace_once(&accepted, "      - main", "      - trunk"),
                "repository.workflow_branch",
            ),
            (
                "additional push branch",
                replace_once(&accepted, "      - main", "      - main\n      - release"),
                "repository.workflow_trigger",
            ),
            (
                "missing permissions",
                replace_once(&accepted, "permissions:\n  contents: read\n", ""),
                "repository.workflow_permission",
            ),
            (
                "write permission",
                replace_once(&accepted, "  contents: read", "  contents: write"),
                "repository.workflow_permission",
            ),
            (
                "expanded permission",
                replace_once(
                    &accepted,
                    "  contents: read",
                    "  contents: read\n  actions: read",
                ),
                "repository.workflow_permission",
            ),
            (
                "job permission",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    permissions:\n      contents: read\n    name: RunenUI validation\n",
                ),
                "repository.workflow_job",
            ),
            (
                "workflow name",
                replace_once(&accepted, "name: CI", "name: Validation"),
                "repository.workflow_identity",
            ),
            (
                "concurrency group",
                replace_once(
                    &accepted,
                    "  group: ci-${{ github.workflow }}-${{ github.ref }}",
                    "  group: other",
                ),
                "repository.workflow_concurrency",
            ),
            (
                "missing concurrency",
                replace_once(
                    &accepted,
                    "concurrency:\n  group: ci-${{ github.workflow }}-${{ github.ref }}\n  cancel-in-progress: true\n",
                    "",
                ),
                "repository.workflow_concurrency",
            ),
            (
                "cancellation false",
                replace_once(
                    &accepted,
                    "  cancel-in-progress: true",
                    "  cancel-in-progress: false",
                ),
                "repository.workflow_concurrency",
            ),
            (
                "missing cancellation",
                replace_once(&accepted, "  cancel-in-progress: true\n", ""),
                "repository.workflow_concurrency",
            ),
            (
                "job key",
                replace_once(&accepted, "  validate:\n", "  check:\n"),
                "repository.workflow_job",
            ),
            (
                "job name",
                replace_once(&accepted, "    name: RunenUI validation", "    name: Other"),
                "repository.workflow_job",
            ),
            (
                "second job",
                format!(
                    "{accepted}\n  second:\n    uses: dornglut/github-workflows/.github/workflows/reusable-rust-cargo-validate.yml@{accepted_revision}\n"
                ),
                "repository.workflow_job",
            ),
            (
                "with",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    with:\n      mode: strict\n",
                ),
                "repository.workflow_job",
            ),
            (
                "secrets",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    secrets:\n      token: value\n",
                ),
                "repository.workflow_job",
            ),
            (
                "inherited secrets",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    secrets: inherit\n",
                ),
                "repository.workflow_job",
            ),
            (
                "local steps",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    steps:\n      - run: true\n",
                ),
                "repository.workflow_job",
            ),
            (
                "runner",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    runs-on: ubuntu-latest\n",
                ),
                "repository.workflow_job",
            ),
            (
                "condition",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    if: always()\n",
                ),
                "repository.workflow_job",
            ),
            (
                "dependency",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    needs: other\n",
                ),
                "repository.workflow_job",
            ),
            (
                "continue on error",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    continue-on-error: true\n",
                ),
                "repository.workflow_job",
            ),
            (
                "strategy",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    strategy: {}\n",
                ),
                "repository.workflow_job",
            ),
            (
                "matrix",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    matrix: {}\n",
                ),
                "repository.workflow_job",
            ),
            (
                "environment",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    environment: production\n",
                ),
                "repository.workflow_job",
            ),
            (
                "container",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    container: rust\n",
                ),
                "repository.workflow_job",
            ),
            (
                "services",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    services: {}\n",
                ),
                "repository.workflow_job",
            ),
            (
                "timeout",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    timeout-minutes: 5\n",
                ),
                "repository.workflow_job",
            ),
            (
                "duplicated cargo validate",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    steps:\n      - run: cargo validate\n",
                ),
                "repository.workflow_job",
            ),
            (
                "other repository command",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    steps:\n      - run: cargo xtask audit-repository\n",
                ),
                "repository.workflow_job",
            ),
            (
                "unexpected top-level field",
                format!("{accepted}\ndescription: extra\n"),
                "repository.workflow_unexpected_field",
            ),
            (
                "unexpected job field",
                replace_once(
                    &accepted,
                    job_anchor,
                    "    name: RunenUI validation\n    custom: true\n",
                ),
                "repository.workflow_unexpected_field",
            ),
            (
                "reordered sections",
                replace_once(
                    &accepted,
                    "permissions:\n  contents: read\nconcurrency:\n  group: ci-${{ github.workflow }}-${{ github.ref }}\n  cancel-in-progress: true\n",
                    "concurrency:\n  group: ci-${{ github.workflow }}-${{ github.ref }}\n  cancel-in-progress: true\npermissions:\n  contents: read\n",
                ),
                "repository.workflow_permission",
            ),
            (
                "altered indentation",
                replace_once(&accepted, "  pull_request:", "   pull_request:"),
                "repository.workflow_trigger",
            ),
            (
                "added comment",
                format!("{accepted}\n# comment\n"),
                "repository.workflow_unexpected_field",
            ),
        ];

        for (label, workflow, expected_code) in mutations {
            let fixture = Fixture::new(label)?;
            fixture.write_ci_workflow(&workflow)?;
            let report = build_report(fixture.path())?;
            assert!(!report.is_success(), "{label}: mutation was accepted");
            assert_fatal_code(&report, label, expected_code);
        }

        for (label, action) in [
            ("missing active workflow", 0_u8),
            ("additional yml workflow", 1_u8),
            ("additional yaml workflow", 2_u8),
            ("renamed active workflow", 3_u8),
        ] {
            let fixture = Fixture::new(label)?;
            match action {
                0 => fixture.remove(".github/workflows/ci.yml")?,
                1 => fixture.write_additional_workflow("extra.yml")?,
                2 => fixture.write_additional_workflow("extra.yaml")?,
                3 => {
                    fixture.remove(".github/workflows/ci.yml")?;
                    fixture.write_additional_workflow("renamed.yml")?;
                }
                _ => unreachable!(),
            }
            let report = build_report(fixture.path())?;
            assert!(!report.is_success(), "{label}: mutation was accepted");
            assert_fatal_code(&report, label, "repository.workflow_inventory");
        }

        Ok(())
    }

    #[test]
    fn malformed_matrix_status_and_summary_are_fatal() -> Result<(), String> {
        let fixture = Fixture::new("matrix")?;
        fixture.write(
            "docs/architecture/m4-conformance-matrix.md",
            "```text\n2 total unique rows\n0 owner-accepted\n0 implementation-complete\n0 proof-complete\n2 blocked\n0 duplicate IDs\n0 invalid statuses\n0 invalid schemas\n```\n\n| ID | Required observation | Positive proof owner | Negative proof owner | Trace proof owner | Delivery slice | Status | M4 gate |\n|---|---|---|---|---|---|---|---|\n| PTR-01 | observation | positive | negative | trace | M4C3 | unsupported | Required |\n",
        )?;
        let report = build_report(fixture.path())?;
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "matrix.invalid_status" })
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "matrix.inconsistent_summary" })
        );
        Ok(())
    }

    #[test]
    fn undocumented_workspace_member_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("workspace")?;
        fixture.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\n  \"crates/runenui_core\",\n  \"crates/runenui_runtime\",\n  \"crates/extra\",\n  \"xtask\",\n]\n\n[workspace.package]\nrepository = \"https://github.com/dornglut/runen-ui\"\nlicense = \"MIT\"\npublish = false\n",
        )?;
        fixture.write(
            "crates/extra/Cargo.toml",
            "[package]\nname = \"extra\"\n\n[dependencies]\n",
        )?;
        fixture.write("crates/extra/src/lib.rs", "#![forbid(unsafe_code)]\n")?;
        let report = build_report(fixture.path())?;
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "workspace.undocumented_member" })
        );
        Ok(())
    }

    #[test]
    fn old_active_branch_terminology_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("master")?;
        fixture.write("AGENTS.md", "Start from current `master`.\n")?;
        let report = build_report(fixture.path())?;
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "authority.active_master_reference" })
        );
        Ok(())
    }

    #[test]
    fn private_archive_link_outside_migration_history_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("archive")?;
        fixture.write("README.md", &format!("https://{PRIVATE_ARCHIVE_URL}\n"))?;
        let report = build_report(fixture.path())?;
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "authority.private_archive_reference" })
        );
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
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "authority.active_historical_owner_reference" })
        );
        Ok(())
    }

    #[test]
    fn unmodeled_public_issue_link_is_fatal() -> Result<(), String> {
        let fixture = Fixture::new("issue")?;
        fixture.write(
            "README.md",
            "https://github.com/dornglut/runen-ui/issues/999\n",
        )?;
        let report = build_report(fixture.path())?;
        assert!(
            report
                .findings
                .iter()
                .any(|finding| { finding.code == "authority.public_issue_not_modeled" })
        );
        Ok(())
    }
}
