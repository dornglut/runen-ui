# ADR 0017: M9 zero-duration forever and relative timing clarification

> **Category:** ADR
>
> **Status:** Proposed target amendment pending exact-head owner acceptance
>
> **Decision date:** 2026-09-13
>
> **Milestone:** M9
>
> **Reviewed baseline:** `3527103253e5354a56ef1f8255e8701f3c30c6ee`
>
> **Owner:** #204
>
> **Acceptance:** this amendment becomes accepted only after the exact M9B0R2
> package containing it is explicitly accepted by the repository owner,
> squash-merged, and accepted-main validated. Acceptance clarifies target
> architecture only; it does not claim M9B production implementation or promote
> any `M9MOTION-*` conformance row.

## Context

ADR 0010 defines one runtime-owned deterministic motion model and exact finite
zero-duration behavior: zero duration with zero delay reaches the terminal value in
the same successfully committed candidate that starts motion, while zero duration
with positive delay holds the initial value until the first candidate at or after the
deadline and then commits terminal value and completion atomically. ADR 0010 also
requires `Repeat::Forever` timelines to never naturally complete.

ADR 0016 clarifies motion identity, interpolation, reduced-motion validity and checked
staged failure, but it does not define ordinary sampling for the combined case
`duration == 0` plus `Repeat::Forever`.

That combination has no positive iteration interval from which normalized progress can
be derived, no terminal boundary, and infinitely many zero-length iteration boundaries
at one instant. Choosing an arbitrary held keyframe, synthetic frame, hidden iteration
counter, completion rule, or dependency convention would add observable target
semantics not owned by the accepted ADRs.

The M9B source-first audit also found a separate checked-time normalization gap.
Individually representable delay and duration values do not imply that a finite
relative terminal offset is representable. A transition may overflow `delay +
duration`; a finite repeated timeline may overflow `delay + duration * iterations`.
The public description contract must distinguish that relative-specification failure
from candidate-start overflow at a late runtime monotonic instant.

## Relationship to accepted authority

This ADR narrowly amends ADR 0010 and ADR 0016 for timing normalization only. It does
not change:

- the accepted monotonic clock or one-candidate-instant authority;
- finite zero-duration transition/timeline terminal behavior;
- delay, keyframe, finite-repeat, completion, replacement or cancellation ordering;
- `Repeat::Forever` positive-duration lifecycle or no-natural-completion rule;
- reduced-motion source/strategy defaults and validity;
- interpolation, easing, style precedence, publication atomicity, redraw or trace
  ownership;
- any M9 conformance row status.

## Decision

### Zero-duration forever timelines are invalid authored motion

An explicit timeline with `Repeat::Forever` must have a strictly positive duration.
`duration == 0` with `Repeat::Forever` rejects during explicit-timeline specification
validation before any mounted/runtime motion state exists.

This is a description/specification error, not a runtime fallback. Runtime does not:

- select keyframe `0` or keyframe `1` as an arbitrary forever sample;
- invent a phantom frame or minimum duration;
- count infinitely many zero-length iterations;
- treat the source as naturally completed; or
- reinterpret it through reduced-motion policy.

Finite timelines retain the existing zero-duration rule unchanged, including finite
repeat counts greater than one. Their complete finite schedule has a terminal boundary
and therefore samples the exact final keyframe and completes at the already accepted
same-candidate/deadline boundary.

### Finite relative schedules are validated before acceptance

Every accepted motion duration and delay remains individually representable in the
runtime-relative `u64` nanosecond domain.

In addition, description construction validates the complete finite relative schedule
using checked integer arithmetic in that same domain:

- for a style transition, `terminal_offset = delay + duration` must be representable;
- for an explicit finite timeline with total iteration count `n`,
  `terminal_offset = delay + duration * n` must be representable;
- multiplication is checked before addition and neither operation wraps, saturates,
  clamps, or uses floating-point timing arithmetic.

A finite relative schedule that cannot be represented rejects during description/spec
validation. No runtime record is created and no candidate needs to choose a partial
terminal deadline.

For `Repeat::Forever`, there is no finite terminal offset to precompute. Its delay must
still be individually representable and its duration must be strictly positive as
specified above.

### Candidate-start overflow remains staged runtime failure

Relative-specification validity does not guarantee that a motion can start at every
possible runtime instant. When staging a candidate start/replacement, runtime still
uses checked monotonic arithmetic for any required absolute deadline derived from the
candidate start instant.

For example, a valid relative finite schedule may still fail when
`start_instant + terminal_offset` exceeds the monotonic instant domain. That failure is
an ordinary checked staged-planning failure under ADR 0010/0016:

- it does not partially create/replace/complete motion state;
- it does not commit sampled effective values or dependent publication products;
- it does not consume a start instant; and
- it remains observable through the canonical diagnostic/trace authority required by
  M9B.

This distinction keeps public description validity independent from the current clock
position while still proving every finite relative schedule is internally
representable.

## Conformance impact

This amendment adds no M9 row and changes no row status.

It narrows existing M9B observations only:

- `M9MOTION-03`: checked duration/delay/repeat specification rejects zero-duration
  forever timelines and unrepresentable finite relative schedules before mutation;
- `M9MOTION-04`: finite checked time arithmetic includes checked finite relative
  terminal-offset construction, while positive-duration forever timelines retain no
  natural terminal boundary; and
- `M9MOTION-09`: late candidate-start overflow remains atomic staged failure and
  cannot partially advance motion/declaration/publication state.

The conformance matrix remains unchanged until normal M9B implementation, proof,
owner acceptance, merge, accepted-main validation and later reconciliation.

## Consequences

- Every accepted forever timeline has a well-defined positive iteration interval.
- Finite zero-duration behavior remains exactly as already accepted.
- Finite motion descriptions cannot hide a relative terminal-time overflow that would
  otherwise surface only after runtime state exists.
- A late monotonic start can still fail independently, preserving checked runtime time
  semantics without making the authored description depend on current clock state.
- No new timer, frame, iteration-counter, renderer, dependency, or reduced-motion
  authority is introduced.

## Rejected alternatives

### Hold keyframe 0 for zero-duration forever

Rejected because that would invent `HoldInitial`-like behavior even when reduced
motion is false and would make an invalid ordinary timeline silently behave like a
preference-suppressed one.

### Hold keyframe 1 for zero-duration forever

Rejected because a forever source has no accepted terminal completion boundary and
ADR 0010 explicitly says forever timelines never naturally complete.

### Invent a minimum non-zero duration or one synthetic frame

Rejected because wall/frame cadence is not motion authority and a hidden duration
would make logical samples host-dependent.

### Allow relative arithmetic to wrap or saturate

Rejected because checked monotonic time is inherited M4 authority. Wrapping,
saturation, clamping, or floating-point timing would create an alternate time domain.

### Reject descriptions based on the current start instant

Rejected because authored/specification validity is a relative contract. Absolute
start overflow depends on runtime clock position and therefore belongs to staged
candidate admission, where atomic failure is already required.
