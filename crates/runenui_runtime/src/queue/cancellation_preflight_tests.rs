//! Cross-family and cross-root cancellation demand qualification for U475.
//! These tests use the private queue ledger but not a second runtime authority.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "The ledger fixtures require exact checked invariant assertions"
)]

use super::{QueueReservationError, RootQuotaId, WorkQueue, WorkSequence};
use crate::{
    RuntimeLimits,
    work::{WorkGeneration, WorkRegistry},
};

struct LedgerFixture {
    queue: WorkQueue<u8>,
    roots: Vec<RootQuotaId>,
    generations: Vec<WorkGeneration>,
    independent: RootQuotaId,
}

/// Every root retains one host request, framework service, local task, send
/// task and repeating timer. One outstanding host response is materialized so
/// its response-credit release is distinguishable from cancellation.
fn fixture(root_count: usize) -> LedgerFixture {
    let total = root_count
        .checked_mul(8)
        .and_then(|credits| credits.checked_add(1))
        .expect("fixture has bounded capacity");
    let mut queue = WorkQueue::<u8>::new(total);
    let generations = WorkRegistry::<u8, runenui_core::NoHostProtocol>::new(RuntimeLimits::default())
        .preview_generations(root_count * 5)
        .expect("generations fit")
        .0;
    let mut roots = Vec::with_capacity(root_count);
    for group in generations.chunks_exact(5) {
        let reservation = queue.reserve(8).expect("eight terminal credits fit");
        let root = reservation.quota();
        queue
            .retain_root_host_request_lease(root, group[0])
            .expect("host cancellation credit");
        queue
            .retain_root_framework_service_lease(root, group[1])
            .expect("framework response and cancellation credits");
        queue
            .retain_root_local_task_lease(root, group[2])
            .expect("local cancellation credit");
        queue
            .retain_root_send_task_lease(root, group[3])
            .expect("send cancellation credit");
        queue
            .retain_root_timer_lease(root, group[4], true)
            .expect("repeating timer credits");
        let quota = queue.root_quotas.get_mut(&root).expect("live root");
        assert_eq!(quota.remaining, 1);
        quota.remaining -= 1;
        let host = quota.host_requests.get_mut(&group[0]).expect("live host");
        host.start_pending = false;
        host.response = true;
        roots.push(root);
    }
    let independent = queue.reserve(1).expect("independent credit fits").quota();
    assert_eq!(queue.reserved, total);
    LedgerFixture {
        queue,
        roots,
        generations,
        independent,
    }
}

fn assert_preserved(fixture: &LedgerFixture, root_index: usize) {
    let root = fixture.roots[root_index];
    let group = &fixture.generations[root_index * 5..][..5];
    let quota = fixture.queue.root_quotas.get(&root).expect("root survives");
    assert_eq!(quota.remaining, 0);
    assert_eq!(quota.live_work_leases, 5);
    assert!(quota.host_requests.contains_key(&group[0]));
    assert!(quota.framework_services.contains_key(&group[1]));
    assert!(quota.local_tasks.contains_key(&group[2]));
    assert!(quota.send_tasks.contains_key(&group[3]));
    assert!(quota.timers.contains_key(&group[4]));
    assert_eq!(fixture.queue.root_remaining(fixture.independent), Ok(1));
    assert_eq!(
        fixture.queue.next_sequence().map(WorkSequence::get),
        Some(1),
        "cancellation preflight and refusal do not sequence work"
    );
}

#[test]
fn mixed_family_preflight_counts_every_root_but_releases_only_the_caller_credit() {
    let mut fixture = fixture(2);
    for root in &fixture.roots {
        let (cancellations, releases) = fixture
            .queue
            .preflight_root_cancellations(&fixture.generations, *root)
            .expect("two roots across five families are valid");
        assert_eq!(cancellations.host_requests, 2);
        assert_eq!(cancellations.framework_services, 2);
        assert_eq!(cancellations.local_tasks, 2);
        assert_eq!(cancellations.send_tasks, 2);
        assert_eq!(cancellations.timers, 2);
        assert_eq!(releases.host_requests, 1);
        assert_eq!(releases.framework_services, 1);
    }
    for i in 0..2 {
        assert_preserved(&fixture, i);
    }
    for i in 0..2 {
        fixture
            .queue
            .release_root_work_leases(fixture.roots[i], &fixture.generations[i * 5..][..5])
            .expect("full mixed-family batch retires atomically");
    }
    assert_eq!(fixture.queue.root_quotas.len(), 1);
    assert_eq!(fixture.queue.reserved, 1);
    assert_eq!(fixture.queue.root_remaining(fixture.independent), Ok(1));
    assert_eq!(fixture.queue.next_sequence().map(WorkSequence::get), Some(1));
}

#[test]
fn mixed_family_batch_refuses_corrupt_reservations_without_partial_retirement() {
    for failure in ["global reserve", "occupied frontier", "live lease count"] {
        let mut fixture = fixture(2);
        let original_reserved = fixture.queue.reserved;
        let root = fixture.roots[0];
        match failure {
            "global reserve" => fixture.queue.reserved = 1,
            "occupied frontier" => {
                fixture
                    .queue
                    .root_quotas
                    .get_mut(&root)
                    .expect("root survives")
                    .pending_envelopes = 1;
            }
            "live lease count" => {
                fixture
                    .queue
                    .root_quotas
                    .get_mut(&root)
                    .expect("root survives")
                    .live_work_leases = 4;
            }
            _ => unreachable!("enumerated failures only"),
        }
        assert_eq!(
            fixture
                .queue
                .release_root_work_leases(root, &fixture.generations[..5]),
            Err(QueueReservationError::Stale),
            "{failure}"
        );
        assert_eq!(fixture.queue.root_quotas.len(), 3);
        assert_preserved_except_injected_counter(&fixture);
        match failure {
            "global reserve" => fixture.queue.reserved = original_reserved,
            "occupied frontier" => {
                fixture
                    .queue
                    .root_quotas
                    .get_mut(&root)
                    .expect("root survives")
                    .pending_envelopes = 0;
            }
            "live lease count" => {
                fixture
                    .queue
                    .root_quotas
                    .get_mut(&root)
                    .expect("root survives")
                    .live_work_leases = 5;
            }
            _ => unreachable!("enumerated failures only"),
        }
        assert_preserved(&fixture, 0);
        fixture
            .queue
            .release_root_work_leases(root, &fixture.generations[..5])
            .expect("repair permits original batch to retire");
        assert_eq!(fixture.queue.reserved, 9);
        assert_eq!(fixture.queue.root_remaining(fixture.independent), Ok(1));
        assert_preserved(&fixture, 1);
    }
}

fn assert_preserved_except_injected_counter(fixture: &LedgerFixture) {
    for i in 0..2 {
        let root = fixture.roots[i];
        let group = &fixture.generations[i * 5..][..5];
        let quota = fixture.queue.root_quotas.get(&root).expect("root survives");
        assert_eq!(quota.remaining, 0);
        assert!(quota.host_requests.contains_key(&group[0]));
        assert!(quota.framework_services.contains_key(&group[1]));
        assert!(quota.local_tasks.contains_key(&group[2]));
        assert!(quota.send_tasks.contains_key(&group[3]));
        assert!(quota.timers.contains_key(&group[4]));
    }
    assert_eq!(fixture.queue.root_remaining(fixture.independent), Ok(1));
    assert_eq!(fixture.queue.next_sequence().map(WorkSequence::get), Some(1));
}

#[test]
fn cross_family_preflight_rejects_stale_terminal_authority_in_all_five_families() {
    for family in 0..5 {
        let mut fixture = fixture(2);
        let root = fixture.roots[1];
        let generation = fixture.generations[5 + family];
        let quota = fixture.queue.root_quotas.get_mut(&root).expect("root survives");
        match family {
            0 => quota.host_requests.get_mut(&generation).expect("host").cancellation = false,
            1 => {
                quota
                    .framework_services
                    .get_mut(&generation)
                    .expect("framework")
                    .cancellation = false;
            }
            2 => quota.local_tasks.get_mut(&generation).expect("local").invalidated = true,
            3 => quota.send_tasks.get_mut(&generation).expect("send").invalidated = true,
            4 => quota.timers.get_mut(&generation).expect("timer").invalidated = true,
            _ => unreachable!("five families"),
        }
        assert!(matches!(
            fixture
                .queue
                .preflight_root_cancellations(&fixture.generations, fixture.roots[0]),
            Err(QueueReservationError::Stale)
        ));
        assert_eq!(fixture.queue.reserved, 17);
        assert_preserved(&fixture, 0);
        assert_preserved(&fixture, 1);
    }
}

#[test]
fn cross_root_or_cross_family_generation_alias_refuses_before_credit_consumption() {
    for alias in ["across roots", "across families", "duplicate input"] {
        let mut fixture = fixture(2);
        let generation = fixture.generations[0];
        match alias {
            "across roots" => {
                let credits = fixture.queue.root_quotas[&fixture.roots[0]].host_requests[&generation];
                fixture
                    .queue
                    .root_quotas
                    .get_mut(&fixture.roots[1])
                    .expect("second root")
                    .host_requests
                    .insert(generation, credits);
            }
            "across families" => {
                let credits = fixture.queue.root_quotas[&fixture.roots[0]].local_tasks
                    [&fixture.generations[2]];
                fixture
                    .queue
                    .root_quotas
                    .get_mut(&fixture.roots[0])
                    .expect("first root")
                    .local_tasks
                    .insert(generation, credits);
            }
            "duplicate input" => {}
            _ => unreachable!("enumerated alias cases"),
        }
        let mut invalidated = fixture.generations.clone();
        if alias == "duplicate input" {
            invalidated.push(generation);
        }
        assert!(matches!(
            fixture
                .queue
                .preflight_root_cancellations(&invalidated, fixture.roots[0]),
            Err(QueueReservationError::Stale)
        ));
        assert_eq!(fixture.queue.reserved, 17);
        assert_preserved(&fixture, 0);
        assert_preserved(&fixture, 1);
    }
}

/// Opt-in current-head release-mode diagnostic; not a latency acceptance limit.
#[test]
#[ignore = "opt-in release-mode U475 cross-root cancellation cost measurements"]
fn bench_u475_cross_root_cancellation_preflight_scaling() {
    use std::time::Instant;
    const REPEATS: usize = 30;
    for size in [32usize, 256, 1024, 2048] {
        let mut samples = Vec::with_capacity(REPEATS);
        let mut max_rss_kib = 0usize;
        for _ in 0..REPEATS {
            let fixture = fixture(size);
            let started = Instant::now();
            let (counts, released) = fixture
                .queue
                .preflight_root_cancellations(&fixture.generations, fixture.roots[0])
                .expect("mixed cancellation set remains valid");
            samples.push(started.elapsed().as_micros());
            assert_eq!(counts.host_requests, size);
            assert_eq!(counts.framework_services, size);
            assert_eq!(counts.local_tasks, size);
            assert_eq!(counts.send_tasks, size);
            assert_eq!(counts.timers, size);
            assert_eq!(released.host_requests, 1);
            assert_eq!(released.framework_services, 1);
            if let Ok(status) = std::fs::read_to_string("/proc/self/status")
                && let Some(kib) = status
                    .lines()
                    .find_map(|line| line.strip_prefix("VmRSS:"))
                    .and_then(|line| line.split_whitespace().next())
                    .and_then(|value| value.parse::<usize>().ok())
            {
                max_rss_kib = max_rss_kib.max(kib);
            }
        }
        samples.sort_unstable();
        println!(
            "U475_METRIC,cross-root-cancellation,off,{size},{REPEATS},{},{},{},{max_rss_kib}",
            samples[REPEATS / 2],
            samples[(95 * REPEATS).div_ceil(100) - 1],
            samples[REPEATS - 1]
        );
    }
}
