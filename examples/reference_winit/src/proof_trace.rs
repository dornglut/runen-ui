use core::num::NonZeroUsize;

use runenui_core::UiApp;
use runenui_runtime::{AppRuntime, RuntimeConfig, TraceSinkReceiveError, TraceSinkReceiver};

const PROOF_TRACE_SINK_CAPACITY: usize = 4096;

/// Mounts one application runtime with the ordinary runtime defaults plus a
/// subordinate canonical JSONL sink when native proof capture is enabled.
///
/// The sink does not replace retained canonical trace authority and preserves
/// the default redacted committed-text/preedit payload policy.
pub fn mount<App: UiApp>(
    state: App::State,
    enabled: bool,
) -> (AppRuntime<App>, Option<TraceSinkReceiver>) {
    if !enabled {
        return (AppRuntime::<App>::mount(state), None);
    }

    let config = RuntimeConfig::default();
    let sink_capacity = NonZeroUsize::new(PROOF_TRACE_SINK_CAPACITY)
        .unwrap_or_else(|| unreachable!("proof trace sink capacity is non-zero"));
    let trace_config = config.trace_config().with_sink_capacity(sink_capacity);
    let mut runtime =
        AppRuntime::<App>::mount_with_config(state, config.with_trace_config(trace_config));
    let receiver = runtime.take_trace_sink_receiver();
    (runtime, receiver)
}

/// Drains every currently available canonical trace record without blocking.
pub fn drain(receiver: Option<&TraceSinkReceiver>) {
    let Some(receiver) = receiver else {
        return;
    };
    loop {
        match receiver.try_recv() {
            Ok(line) => eprintln!("RUNENUI_TRACE {}", line.as_str()),
            Err(TraceSinkReceiveError::Empty | TraceSinkReceiveError::Closed) => break,
            Err(error) => {
                eprintln!("reference_winit canonical trace drain stopped: {error}");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mount;
    use crate::DemoApp;

    #[test]
    fn proof_mode_is_the_only_path_that_exposes_a_trace_sink() {
        let (_runtime, ordinary_receiver) = mount::<DemoApp>((), false);
        assert!(ordinary_receiver.is_none());

        let (_runtime, proof_receiver) = mount::<DemoApp>((), true);
        assert!(proof_receiver.is_some());
    }
}
