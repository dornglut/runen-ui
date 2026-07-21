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

    def test_normal_profiles_exclude_cargo_lock(self) -> None:
        for profile_name in (
            "ai-core",
            "current-work",
            "domain-work",
            "implementation-work",
            "workspace-planning",
        ):
            with self.subTest(profile=profile_name):
                self.assertNotIn(Path("Cargo.lock"), self.files_for_profile(profile_name))

    def test_full_audit_includes_governance_ci_lockfile_and_licenses(self) -> None:
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
