# R2c-E production UI and lifecycle validation

- Date: 2026-08-18
- Scope: production observer lifecycle, bounded bridge state, gallery revision refresh, and desktop shutdown
- Source-media access: controlled temporary fixtures only, read-only
- Real-library access: none

## Contract under test

ADR 0020 connects the R2c-A through R2c-D contracts to the normal desktop lifecycle. Rust owns root
observers, durable queue ingress, path-level publication, root freshness, and shutdown. Flutter polls
one bounded synchronization snapshot, refreshes only after a published catalog revision, and preserves
stable gallery interaction state without deriving filesystem policy.

The controlled fixtures prove:

| Boundary | Required result |
| --- | --- |
| Desktop start | Start one synchronization runtime after Rust and the initial catalog are ready |
| Startup publication | Refresh a current synchronization revision even when it was published before screen subscription |
| Continuity authority | Persist a root freshness gap on cold start and after an unavailable interval |
| Registered roots | Start one observer for each available root and stop removed or reconfigured roots |
| Queue handoff | Retain a drained observer plan across SQLite enqueue failure and retry it first |
| Path publication | Drain live path evidence through the durable queue and advance the catalog revision |
| Unsupported scope | Retain subtree, root, and freshness-gap work for R2c-F without consuming retries |
| Unavailable root | Preserve cached catalog state, avoid observer startup, and show `目录不可用` |
| Degraded source | Preserve the last trustworthy revision and show `需要核对` |
| Root metrics | Report bounded queue counts for only the requested root generation |
| Stable anchor | Prefer the requested active location, follow a rename, and fall back near the prior ordinal after removal |
| Background refresh | Keep old assets visible until the newer revision and timeline publish together |
| Refresh contention | Retry only a busy or superseded controller transition and keep one coalesced target |
| Refresh failure | Stop automatic retries, retain the pending revision, and show one localized retry surface |
| In-flight newer revision | Retain the maximum revision and start one coalesced follow-up after failure |
| Scan task priority | Keep scan progress, controls, completion, and acknowledgement above a pending refresh error |
| Selection | Preserve explicit asset selection across a revision and clear complete-query select-all |
| Viewer | Remain independent from the bounded detail window, follow a preferred location across rename, and close only after authoritative removal |
| Navigation state | Preserve source, filters, layout, preview demand, and logical scroll position |
| Product status | Render `已同步`, `正在更新图库`, `需要核对`, or `目录不可用`, including start failure before root metrics exist |
| Manual refresh | Route `更新图库` through the application scan use case |
| Polling | Prevent overlapping polls and coalesce newer catalog revisions |
| Stop | Wait for an active poll and invoke Rust stop only once |
| Window close | Run shutdown actions once in reverse order and keep destruction bounded to six seconds |
| Bridge | Generate typed start, poll, stop, freshness, and stable-anchor contracts |
| Source safety | Perform no source-media mutation or cloud hydration |

## Focused verification evidence

```text
cargo test --manifest-path rust/Cargo.toml library_synchronization::tests -- --nocapture
8 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml root_metrics_are_isolated_from_other_roots -- --nocapture
1 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml gallery_asset_anchor -- --nocapture
2 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml missing_asset_anchor_falls_back_to_the_nearest_surviving_ordinal -- --nocapture
1 passed; 0 failed

./tool/quality_test_flutter.ps1 -TestPath test/features/library/application/library_synchronization_test.dart
4 passed; 0 failed
```

The complete Flutter run additionally proved:

```text
ame_shutdown_coordinator_test.dart: 2 passed
library_controller_test.dart: 40 passed
gallery_selection_test.dart: 4 passed
library_navigation_semantics_test.dart: 5 passed
unified_library_screen_test.dart: 31 passed
library_viewer_position_test.dart: 6 passed
window_manager_actions_test.dart: 2 passed
```

## Complete repository evidence

```text
./tool/bridge_generate.ps1
typed bridge generation and release Rust compilation: passed

./tool/quality_lint.ps1
format, release guardrails, Clippy with warnings denied, and Dart analysis: passed

./tool/quality_verify_daily.ps1
Rust: 288 total; 283 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled picker and scan integration: 2 passed
Windows native accessibility integration: 2 passed
bridge compatibility, release guardrails, and whitespace: passed

./tool/release_verify_windows.ps1
Windows x64 Release build: passed
release bridge and system accent smoke integration: 2 passed
```

The first focused Flutter invocation reached the documented workspace-only SDK lock before creating
a Dart or `flutter_tester` child. It was stopped, and the identical repository command passed with the
scoped sandbox approval required by `AGENTS.md`; no SDK lock was deleted and no unrelated process was
terminated. The complete Daily and Windows release gates were then run directly with that same scoped
approval.

The audit-hardening run additionally exercised a revision already published before screen
subscription, a real screen subscription across an identity-preserving rename and an authoritative
removal, a SQLite-triggered queue enqueue failure, preferred-location selection for a multi-location
asset even when another location is the only loaded detail, removal fallback at the prior ordinal, a
delayed asset lookup racing same-asset navigation, a bridge failure before the first root status, and
destruction after the configured shutdown timeout.

The 2026-08-19 final integration audit hardening made synchronization refresh outcomes explicit as
applied, busy, superseded, or failed. A controlled permanent catalog failure produced one automatic
attempt, remained at one attempt after four seconds of virtual time, displayed the localized stale
surface, and performed exactly one additional attempt after the user selected retry. The focused
controller suite passed 39 tests and the viewer-position suite passed nine tests. The latter includes
an in-flight revision 2 failure with revisions 3 and 4 coalesced into exactly one successful revision
4 attempt, plus a held scan whose progress, pause, cancel, completion, and dismissal remain visible
before the pending synchronization retry surface returns. The complete
repository Daily passed with 397 Rust tests total, all Flutter test files, both Windows integrations,
and the shared quality gates. The Windows Release gate then built the x64 application and passed both
packaged bridge smoke tests.

## Remaining boundary

R2c-E does not claim authoritative subtree or root consistency after overflow, evidence gaps, watcher
failure, or a low-frequency audit mismatch. Those rows remain durable and truthfully project
`NeedsReconciliation`. R2c-F owns the escalation ladder, authoritative reconciliation, cancellation,
rollback, repeated-change recovery, and health recovery proof. Downtime journal catch-up remains the
conditional R2c-G slice, and large-library reliability evidence remains R2c-H.
