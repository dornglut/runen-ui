//! Bounded diagnostic wall-clock timing for canonical repository validation.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
struct PhaseTiming {
    label: &'static str,
    elapsed: Duration,
    succeeded: bool,
}

#[derive(Debug)]
pub struct ValidationTimings {
    started_at: Instant,
    phases: Vec<PhaseTiming>,
}

impl ValidationTimings {
    pub fn start() -> Self {
        Self {
            started_at: Instant::now(),
            phases: Vec::new(),
        }
    }

    pub fn measure<T>(
        &mut self,
        label: &'static str,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let started_at = Instant::now();
        let result = operation();
        self.phases.push(PhaseTiming {
            label,
            elapsed: started_at.elapsed(),
            succeeded: result.is_ok(),
        });
        result
    }

    pub fn report(&self, succeeded: bool) {
        let total = self.started_at.elapsed();
        let measured = self
            .phases
            .iter()
            .map(|phase| phase.elapsed)
            .sum::<Duration>();
        let unattributed = total.saturating_sub(measured);

        eprintln!("> validation timing summary (diagnostic wall clock)");
        for phase in &self.phases {
            eprintln!(
                "> timing: {} = {} [{}]",
                phase.label,
                format_duration(phase.elapsed),
                if phase.succeeded { "PASS" } else { "FAIL" }
            );
        }
        eprintln!(
            "> timing: in-xtask unattributed overhead = {}",
            format_duration(unattributed)
        );
        eprintln!(
            "> timing: in-xtask total = {} [{}]",
            format_duration(total),
            if succeeded { "PASS" } else { "FAIL" }
        );
    }
}

fn format_duration(duration: Duration) -> String {
    format!("{}.{:03}s", duration.as_secs(), duration.subsec_millis())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ValidationTimings, format_duration};

    #[test]
    fn duration_format_is_bounded_and_millisecond_precise() {
        assert_eq!(format_duration(Duration::new(62, 345_678_901)), "62.345s");
    }

    #[test]
    fn failed_phase_is_still_recorded() {
        let mut timings = ValidationTimings::start();
        let result = timings.measure("expected failure", || Err::<(), _>("failed".to_owned()));

        assert_eq!(result, Err("failed".to_owned()));
        assert_eq!(timings.phases.len(), 1);
        assert_eq!(timings.phases[0].label, "expected failure");
        assert!(!timings.phases[0].succeeded);
    }
}
