use std::error::Error;

use runenui_core::{FontFamily, LogicalLength, Typography};

use crate::{FontSourcePolicy, TextConstraints, TextLayoutState, TextRequest, TextSystem};

const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");
const FIXTURE_LINE: &str = "multiline responsiveness fixture — retained text layout\n";

fn typography() -> Typography {
    Typography::new(
        FontFamily::named("Cantarell")
            .unwrap_or_else(|_| unreachable!("controlled family name is valid")),
        LogicalLength::new(16.0).unwrap_or_else(|_| unreachable!("controlled size is finite")),
    )
}

fn system() -> TextSystem {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(
        system.register_font_bytes(CANTARELL.to_vec()).is_ok(),
        "controlled Cantarell fixture registers"
    );
    system
}

fn request(text: String) -> TextRequest {
    TextRequest::new(
        text,
        typography(),
        TextConstraints::limited(
            LogicalLength::new(760.0).unwrap_or_else(|_| unreachable!("fixture width is finite")),
        ),
    )
}

#[test]
fn issue_269_large_run_cluster_coverage() -> Result<(), Box<dyn Error>> {
    let mut failures = Vec::new();

    for (label, lines) in [
        ("below_u16", 1129usize),
        ("above_u16", 1130usize),
        ("4000_lines", 4000usize),
        ("16000_lines", 16000usize),
    ] {
        let text = FIXTURE_LINE.repeat(lines);
        let mut system = system();
        let mut state = TextLayoutState::new();
        system.layout_text(&mut state, &request(text.clone()))?;
        let coverage = state
            .retained_cluster_coverage_for_test()
            .unwrap_or_else(|| unreachable!("layout was retained"));

        eprintln!(
            "issue269_parley_large_run label={label} lines={lines} source_len={} layout_lines={} cluster_count={} max_end={} first_non_monotonic={:?}",
            coverage.source_len,
            coverage.line_count,
            coverage.cluster_count,
            coverage.max_end,
            coverage.first_non_monotonic,
        );

        if coverage.max_end != text.len() || coverage.first_non_monotonic.is_some() {
            failures.push((
                label,
                text.len(),
                coverage.max_end,
                coverage.first_non_monotonic,
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "retained Parley cluster ranges must cover the full source monotonically; failures={failures:?}"
    );
    Ok(())
}
