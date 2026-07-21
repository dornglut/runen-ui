# Public Repository Migration

> **Category: Historical record**

RunenUI moved active development from the private historical repository
[`Crystonix/runen-ui-private-archive`](https://github.com/Crystonix/runen-ui-private-archive)
to the public repository
[`Crystonix/runen-ui`](https://github.com/Crystonix/runen-ui) on 2026-07-21.
After the public authority cutover, the public repository is the sole active
execution authority. The private repository is historical evidence only and
must not own current branches, issues, pull requests, blockers, or next actions.

## Repository and branch boundary

- Historical repository: `Crystonix/runen-ui-private-archive`
- Active repository: `Crystonix/runen-ui`
- Historical default branch: `master`
- Active default branch: `main`
- Imported cleaned-history boundary: `a33ee9f53b69c92e7869351e39f44f667fc7dae7`
- Public cutover starting head: `dfc46af32163cd1404bbf629afe30fe68e4d4bf5`

Active instructions use `main` only. Historical references to `master` are
permitted only when they identify a recorded state in the former repository.
There is no dual-branch compatibility policy.

## Accepted imported milestone history

| Delivery | Historical pull request | Accepted squash or closure commit |
|---|---|---|
| M4A canonical queue/pump/trace foundation | [archive PR #74](https://github.com/Crystonix/runen-ui-private-archive/pull/74) | `94bdae4de65c9a68f4fc731af113d03fb134209d` |
| M4B application work scheduler | [archive PR #75](https://github.com/Crystonix/runen-ui-private-archive/pull/75) | `7a0a5786bb796aeb81d4eac3fe87d254efba540a` |
| M4C0 conformance ownership | [archive PR #76](https://github.com/Crystonix/runen-ui-private-archive/pull/76) | `f87b08a1f607493c7e33a3d9151f465a431ad632` |
| M4C1 routed semantic-command kernel | [archive PR #77](https://github.com/Crystonix/runen-ui-private-archive/pull/77) | `44ceee29c73cea1237fefbd30db4baf2cd97b93d` |
| M4C1 governance/work-tracking closure | [archive PR #94](https://github.com/Crystonix/runen-ui-private-archive/pull/94) | `64f6d8b7994246ea5c79d8961c2d19986af3059e` |
| Runtime/trace/surface authority decomposition | [archive PR #95](https://github.com/Crystonix/runen-ui-private-archive/pull/95) | `1eb2d5f914e11717fe5fb838a3102c72ea1dc20f` |
| M4C2 displayed-generation surface context | [archive PR #99](https://github.com/Crystonix/runen-ui-private-archive/pull/99) | `9dbf2b6bc781b4e29e3e9ce10388742eccc90124` |
| M4C2 owner-acceptance closure | [archive PR #100](https://github.com/Crystonix/runen-ui-private-archive/pull/100) | `01c18ea7ba3572426c535e1b930bfcc83a1992c0` |

M4C2's accepted feature head is
`8127c6143948354f2820f4779c92d2fa9daf79ca`. The feature head and squash commit
are recorded separately because squash merges do not preserve feature-head
ancestry.

## Open-work issue mapping

Public issue numbering restarted. Completed private issues were not recreated as
false closed public history. Open work was rewritten from current accepted truth:

| Historical issue | Public authority |
|---|---|
| [archive #78 — M4 umbrella](https://github.com/Crystonix/runen-ui-private-archive/issues/78) | [public #3](https://github.com/Crystonix/runen-ui/issues/3) |
| [archive #81 — M4C3](https://github.com/Crystonix/runen-ui-private-archive/issues/81) | [public #4](https://github.com/Crystonix/runen-ui/issues/4) |
| [archive #82 — M4C4](https://github.com/Crystonix/runen-ui-private-archive/issues/82) | [public #5](https://github.com/Crystonix/runen-ui/issues/5) |
| [archive #83 — M4C5](https://github.com/Crystonix/runen-ui-private-archive/issues/83) | [public #6](https://github.com/Crystonix/runen-ui/issues/6) |
| [archive #84 — M4D1](https://github.com/Crystonix/runen-ui-private-archive/issues/84) | [public #7](https://github.com/Crystonix/runen-ui/issues/7) |
| [archive #85 — M4D2](https://github.com/Crystonix/runen-ui-private-archive/issues/85) | [public #8](https://github.com/Crystonix/runen-ui/issues/8) |
| [archive #86 — M4D3](https://github.com/Crystonix/runen-ui-private-archive/issues/86) | [public #9](https://github.com/Crystonix/runen-ui/issues/9) |
| [archive #90 — Element/Widget concentration](https://github.com/Crystonix/runen-ui-private-archive/issues/90) | [public #10](https://github.com/Crystonix/runen-ui/issues/10) |
| [archive #92 — repository audit](https://github.com/Crystonix/runen-ui-private-archive/issues/92) | [public #11](https://github.com/Crystonix/runen-ui/issues/11) |
| [archive #93 — event output capacity](https://github.com/Crystonix/runen-ui-private-archive/issues/93) | [public #12](https://github.com/Crystonix/runen-ui/issues/12) |

[Public issue #2](https://github.com/Crystonix/runen-ui/issues/2) is new and owns
the authority cutover itself.

Completed archive issues #79, #80, #87, #88, #89, and #91 remain historical
records. Their accepted outcomes are represented by imported commits and current
public documentation, not duplicate public issues.

## Reference policy

- Current work links to public issues and pull requests.
- Historical references use the qualified form `archive issue #N` or
  `archive PR #N` and link to this mapping. Explicit private-repository URLs are
  centralized in this historical record.
- Bare historical `#N` references are prohibited because public numbering may
  eventually reuse the same number for unrelated work.
- The [work-tracking contract](../work-tracking.md) and public M4 umbrella own
  current execution state.
