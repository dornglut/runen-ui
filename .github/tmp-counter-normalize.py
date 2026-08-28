from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new)


audit = Path("xtask/src/repository_audit/workspace.rs")
text = audit.read_text()
text = replace_once(
    text,
    'const RENDER_WGPU_PACKAGE: &str = "runenui_render_wgpu";\nconst TESTING_PACKAGE: &str = "runenui_testing";',
    'const RENDER_WGPU_PACKAGE: &str = "runenui_render_wgpu";\nconst WINIT_PACKAGE: &str = "runenui_winit";\nconst TESTING_PACKAGE: &str = "runenui_testing";',
    "package constants",
)
start_marker = '        REFERENCE_WINIT_PACKAGE => {'
end_marker = '        package if member.relative.starts_with("crates") => {'
if text.count(start_marker) != 1 or text.count(end_marker) != 1:
    raise RuntimeError("dependency-direction authority markers changed")
start = text.index(start_marker)
end = text.index(end_marker, start)
current = text[start:end]
required = [
    "REFERENCE_WINIT_PACKAGE",
    "RENDER_WGPU_PACKAGE",
    '"counter"',
    "EXTERNAL_WIDGET_PACKAGE",
]
if not all(token in current for token in required):
    raise RuntimeError("dependency-direction authority block no longer matches reviewed shape")
replacement = '''        WINIT_PACKAGE => BTreeSet::from([CORE_PACKAGE, RUNTIME_PACKAGE]),
        REFERENCE_WINIT_PACKAGE | "counter" => BTreeSet::from([
            CORE_PACKAGE,
            RUNTIME_PACKAGE,
            RENDER_WGPU_PACKAGE,
            WINIT_PACKAGE,
        ]),
        RENDER_WGPU_PACKAGE | TESTING_PACKAGE | EXTERNAL_WIDGET_PACKAGE => {
            BTreeSet::from([CORE_PACKAGE, RUNTIME_PACKAGE])
        }
'''
audit.write_text(text[:start] + replacement + text[end:])

counter_main = Path("examples/counter/src/main.rs")
text = counter_main.read_text()
old = 'assert!(!surface.contains("paint="));'
count = text.count(old)
if count != 2:
    raise RuntimeError(f"Counter no-paint assertions: expected two matches, found {count}")
counter_main.write_text(text.replace(old, 'assert!(surface.contains("paint="));'))
