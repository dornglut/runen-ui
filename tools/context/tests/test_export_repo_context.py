from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
MODULE_PATH = REPOSITORY_ROOT / "tools" / "context" / "export_repo_context.py"
SPEC = importlib.util.spec_from_file_location("runenui_context_export", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load context exporter from {MODULE_PATH}")
EXPORTER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXPORTER
SPEC.loader.exec_module(EXPORTER)
PROFILES_DIRECTORY = REPOSITORY_ROOT / "tools" / "context" / "profiles"


class ContextProfileTests(unittest.TestCase):
    def files_for_profile(self, profile_name: str, root: Path = REPOSITORY_ROOT) -> set[Path]:
        profile = EXPORTER.load_profile(profile_name, PROFILES_DIRECTORY)
        return set(EXPORTER.iter_context_files(root=root, profile=profile))

    def assert_bounded_profile_exclusions(self, files: set[Path]) -> None:
        self.assertNotIn(Path("Cargo.lock"), files)
        self.assertNotIn(Path("docs/history/public-repository-migration.md"), files)
        self.assertNotIn(Path("docs/reports/m5-accesskit-mapping-review.md"), files)

    def test_profile_inventory_is_small_and_offline_review_is_default(self) -> None:
        self.assertEqual(EXPORTER.DEFAULT_PROFILE, "offline-review")
        self.assertEqual(
            EXPORTER.list_profiles(PROFILES_DIRECTORY),
            ["full-audit", "implementation-review", "offline-review"],
        )

    def test_offline_review_contains_authority_without_implementation_source(self) -> None:
        files = self.files_for_profile("offline-review")
        required = {
            Path("README.md"),
            Path("AGENTS.md"),
            Path("ARCHITECTURE.md"),
            Path("TESTING.md"),
            Path("docs/documentation-architecture.md"),
            Path("docs/status.md"),
            Path("docs/roadmap.md"),
            Path(".github/pull_request_template.md"),
            Path(".github/ISSUE_TEMPLATE/milestone-slice.yml"),
            Path("crates/runenui_core/README.md"),
        }
        self.assertTrue(required.issubset(files), required - files)
        self.assert_bounded_profile_exclusions(files)
        self.assertNotIn(Path("crates/runenui_core/src/lib.rs"), files)

    def test_implementation_review_contains_source_tests_and_validation_tooling(self) -> None:
        files = self.files_for_profile("implementation-review")
        required = {
            Path("AGENTS.md"),
            Path("docs/status.md"),
            Path("crates/runenui_core/src/lib.rs"),
            Path("crates/runenui_runtime/src/lib.rs"),
            Path("tests/external_widget/src/lib.rs"),
            Path("xtask/src/main.rs"),
        }
        self.assertTrue(required.issubset(files), required - files)
        self.assert_bounded_profile_exclusions(files)

    def test_full_audit_includes_governance_provenance_lockfile_and_licenses(self) -> None:
        files = self.files_for_profile("full-audit")
        required = {
            Path(".github/workflows/ci.yml"),
            Path("AGENTS.md"),
            Path("CODE_OF_CONDUCT.md"),
            Path("CONTRIBUTING.md"),
            Path("Cargo.lock"),
            Path("LICENSE-APACHE"),
            Path("LICENSE-MIT"),
            Path("SECURITY.md"),
            Path("docs/history/public-repository-migration.md"),
            Path("docs/reports/m5-accesskit-mapping-review.md"),
        }
        self.assertTrue(required.issubset(files), required - files)

    def test_full_audit_excludes_legacy_even_when_present(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            (root / "legacy").mkdir()
            (root / "legacy" / "old.rs").write_text("fn old() {}\n", encoding="utf-8")
            (root / "Cargo.lock").write_text("version = 4\n", encoding="utf-8")

            files = self.files_for_profile("full-audit", root)

        self.assertIn(Path("Cargo.lock"), files)
        self.assertNotIn(Path("legacy/old.rs"), files)


if __name__ == "__main__":
    unittest.main()
