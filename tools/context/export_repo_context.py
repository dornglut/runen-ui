#!/usr/bin/env python3
"""
Export selected repository files into a single line-numbered context file.

File: tools/context/export_repo_context.py
Function: main
"""

from __future__ import annotations

import argparse
import fnmatch
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

DEFAULT_PROFILE = "ai-core"
DEFAULT_EXTENSIONS = {
    ".rs",
    ".toml",
    ".md",
    ".mdx",
    ".py",
    ".ron",
    ".wgsl",
}
DEFAULT_INCLUDE_FILENAMES = {
    "Cargo.toml",
}
DEFAULT_EXCLUDE_GLOBS = (
    ".git/**",
    "**/.git/**",
    "target/**",
    "**/target/**",
    "node_modules/**",
    "**/node_modules/**",
    "dist/**",
    "**/dist/**",
    "build/**",
    "**/build/**",
    ".astro/**",
    "**/.astro/**",
    "**/*-content.txt",
    "Cargo.lock",
)


@dataclass(frozen=True)
class ContextProfile:
    name: str
    description: str
    include: tuple[str, ...]
    exclude: tuple[str, ...]
    extensions: frozenset[str]
    include_filenames: frozenset[str]


@dataclass(frozen=True)
class ExportStats:
    total_bytes: int
    warnings: tuple[str, ...]


def normalize_glob(pattern: str) -> str:
    return pattern.strip().removeprefix("./")


def normalize_extension(extension: str) -> str:
    value = extension.strip()
    if not value:
        raise SystemExit("empty extension is not allowed")
    return value if value.startswith(".") else f".{value}"


def path_matches(pattern: str, relative: Path) -> bool:
    path = relative.as_posix()
    normalized = normalize_glob(pattern)
    if fnmatch.fnmatchcase(path, normalized):
        return True
    if normalized.startswith("**/") and fnmatch.fnmatchcase(path, normalized[3:]):
        return True
    if normalized.endswith("/**"):
        prefix = normalized[:-3]
        return path == prefix or path.startswith(f"{prefix}/")
    return False


def matches_any(patterns: tuple[str, ...], relative: Path) -> bool:
    return any(path_matches(pattern, relative) for pattern in patterns)


def should_include_by_type(
    relative: Path,
    extensions: frozenset[str],
    include_filenames: frozenset[str],
) -> bool:
    if relative.name in include_filenames:
        return True
    return relative.suffix in extensions


def load_profile(profile_name: str, profiles_dir: Path) -> ContextProfile:
    profile_path = profiles_dir / f"{profile_name}.toml"
    if not profile_path.exists():
        available = ", ".join(list_profiles(profiles_dir)) or "none"
        raise SystemExit(
            f"Context profile '{profile_name}' does not exist at {profile_path}. "
            f"Available profiles: {available}"
        )

    with profile_path.open("rb") as source:
        raw = tomllib.load(source)

    include = tuple(normalize_glob(item) for item in raw.get("include", []))
    exclude = tuple(normalize_glob(item) for item in raw.get("exclude", []))
    extensions = frozenset(
        normalize_extension(item)
        for item in raw.get("extensions", sorted(DEFAULT_EXTENSIONS))
    )
    include_filenames = frozenset(
        raw.get("include_filenames", sorted(DEFAULT_INCLUDE_FILENAMES))
    )

    if not include:
        raise SystemExit(f"Context profile '{profile_name}' must define at least one include glob.")

    return ContextProfile(
        name=profile_name,
        description=str(raw.get("description", "")),
        include=include,
        exclude=exclude,
        extensions=extensions,
        include_filenames=include_filenames,
    )


def with_overrides(
    profile: ContextProfile,
    extra_includes: tuple[str, ...],
    extra_excludes: tuple[str, ...],
    extra_extensions: tuple[str, ...],
    extra_include_filenames: tuple[str, ...],
) -> ContextProfile:
    return ContextProfile(
        name=profile.name,
        description=profile.description,
        include=profile.include + extra_includes,
        exclude=profile.exclude + extra_excludes,
        extensions=frozenset(set(profile.extensions) | set(extra_extensions)),
        include_filenames=frozenset(
            set(profile.include_filenames) | set(extra_include_filenames)
        ),
    )


def list_profiles(profiles_dir: Path) -> list[str]:
    if not profiles_dir.exists():
        return []
    return sorted(path.stem for path in profiles_dir.glob("*.toml"))


def iter_context_files(root: Path, profile: ContextProfile) -> list[Path]:
    files: list[Path] = []
    exclude_patterns = DEFAULT_EXCLUDE_GLOBS + profile.exclude

    for path in root.rglob("*"):
        if not path.is_file():
            continue

        relative = path.relative_to(root)

        if matches_any(exclude_patterns, relative):
            continue

        if not matches_any(profile.include, relative):
            continue

        if not should_include_by_type(
            relative,
            extensions=profile.extensions,
            include_filenames=profile.include_filenames,
        ):
            continue

        files.append(relative)

    return sorted(files, key=lambda item: item.as_posix())


def file_size(root: Path, relative: Path) -> int:
    return (root / relative).stat().st_size


def build_budget_warnings(
    root: Path,
    files: list[Path],
    max_files: int | None,
    max_bytes: int | None,
) -> tuple[str, ...]:
    warnings: list[str] = []
    total_bytes = sum(file_size(root, relative) for relative in files)

    if max_files is not None and len(files) > max_files:
        warnings.append(f"file budget exceeded: {len(files)} files > {max_files}")

    if max_bytes is not None and total_bytes > max_bytes:
        warnings.append(f"byte budget exceeded: {total_bytes} bytes > {max_bytes}")

    return tuple(warnings)


def write_manifest(
    out,
    root: Path,
    profile: ContextProfile,
    files: list[Path],
    total_bytes: int,
    warnings: tuple[str, ...],
) -> None:
    out.write("===== CONTEXT EXPORT MANIFEST =====\n")
    out.write(f"Profile: {profile.name}\n")
    out.write(f"Description: {profile.description}\n")
    out.write(f"Root: {root}\n")
    out.write(f"Included files: {len(files)}\n")
    out.write(f"Total source bytes: {total_bytes}\n")
    out.write("Include globs:\n")
    for pattern in profile.include:
        out.write(f"  - {pattern}\n")
    out.write("Exclude globs:\n")
    for pattern in DEFAULT_EXCLUDE_GLOBS + profile.exclude:
        out.write(f"  - {pattern}\n")
    out.write("Extensions:\n")
    for extension in sorted(profile.extensions):
        out.write(f"  - {extension}\n")
    out.write("Include filenames:\n")
    for filename in sorted(profile.include_filenames):
        out.write(f"  - {filename}\n")
    if warnings:
        out.write("Warnings:\n")
        for warning in warnings:
            out.write(f"  - {warning}\n")
    else:
        out.write("Warnings: none\n")
    out.write("===== END MANIFEST =====\n")


def write_context_file(
    root: Path,
    output: Path,
    profile: ContextProfile,
    files: list[Path],
    warnings: tuple[str, ...],
) -> ExportStats:
    total_bytes = sum(file_size(root, relative) for relative in files)
    with output.open("w", encoding="utf-8") as out:
        write_manifest(
            out=out,
            root=root,
            profile=profile,
            files=files,
            total_bytes=total_bytes,
            warnings=warnings,
        )
        for relative in files:
            absolute = root / relative
            out.write(f"\n===== FILE: ./{relative.as_posix()} =====\n")

            with absolute.open("r", encoding="utf-8", errors="replace") as source:
                for line_number, line in enumerate(source, start=1):
                    out.write(f"{line_number:6}\t{line}")

                    if not line.endswith("\n"):
                        out.write("\n")

            out.write(f"\n===== END FILE: ./{relative.as_posix()} =====\n")

    return ExportStats(total_bytes=total_bytes, warnings=warnings)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export repository source/docs into a line-numbered context file."
    )
    parser.add_argument(
        "--root",
        default=".",
        help="Repository root to export from. Defaults to current directory.",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Output file. Defaults to ./<repo-folder-name>-content.txt.",
    )
    parser.add_argument(
        "--profile",
        default=DEFAULT_PROFILE,
        help=f"Context profile name. Defaults to {DEFAULT_PROFILE}.",
    )
    parser.add_argument(
        "--profiles-dir",
        default=None,
        help="Directory containing context profile TOML files. Defaults to tools/context/profiles.",
    )
    parser.add_argument(
        "--list-profiles",
        action="store_true",
        help="List available context profiles and exit.",
    )
    parser.add_argument(
        "--include",
        action="append",
        default=[],
        help="Additional include glob. Can be passed multiple times.",
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        help="Additional exclude glob. Can be passed multiple times.",
    )
    parser.add_argument(
        "--exclude-dir",
        action="append",
        default=[],
        help="Compatibility shortcut for excluding a directory name. Can be passed multiple times.",
    )
    parser.add_argument(
        "--extension",
        action="append",
        default=[],
        help="Additional file extension to include, such as json or .json. Can be passed multiple times.",
    )
    parser.add_argument(
        "--include-filename",
        action="append",
        default=[],
        help="Additional exact filename to include regardless of extension. Can be passed multiple times.",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=None,
        help="Optional maximum included file count.",
    )
    parser.add_argument(
        "--max-bytes",
        type=int,
        default=None,
        help="Optional maximum source byte count before line-numbering overhead.",
    )
    parser.add_argument(
        "--warn-only",
        action="store_true",
        help="Print budget warnings without failing when max limits are exceeded.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    root = Path(args.root).resolve()
    profiles_dir = (
        Path(args.profiles_dir).resolve()
        if args.profiles_dir
        else root / "tools" / "context" / "profiles"
    )

    if args.list_profiles:
        for profile_name in list_profiles(profiles_dir):
            print(profile_name)
        return

    base_profile = load_profile(args.profile, profiles_dir)
    extra_excludes = tuple(normalize_glob(pattern) for pattern in args.exclude) + tuple(
        pattern
        for directory in args.exclude_dir
        for pattern in (f"{directory}/**", f"**/{directory}/**")
    )
    profile = with_overrides(
        profile=base_profile,
        extra_includes=tuple(normalize_glob(pattern) for pattern in args.include),
        extra_excludes=extra_excludes,
        extra_extensions=tuple(normalize_extension(extension) for extension in args.extension),
        extra_include_filenames=tuple(args.include_filename),
    )
    output = Path(args.output) if args.output else root / f"{root.name}-content.txt"

    files = iter_context_files(root=root, profile=profile)
    warnings = build_budget_warnings(
        root=root,
        files=files,
        max_files=args.max_files,
        max_bytes=args.max_bytes,
    )

    if warnings and not args.warn_only:
        for warning in warnings:
            print(f"warning: {warning}", file=sys.stderr)
        raise SystemExit("context export budget exceeded; pass --warn-only to write anyway")

    stats = write_context_file(
        root=root,
        output=output,
        profile=profile,
        files=files,
        warnings=warnings,
    )

    print(f"Wrote {output}")
    print(f"Profile: {profile.name}")
    print(f"Exported {len(files)} files")
    print(f"Total source bytes: {stats.total_bytes}")
    for warning in stats.warnings:
        print(f"warning: {warning}")


if __name__ == "__main__":
    main()
