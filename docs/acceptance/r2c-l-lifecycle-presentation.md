# R2c-L lifecycle, presentation, and diagnostics acceptance

Status: implementation and local verification complete; independent audit pending

Date: 2026-08-22

## Scope

R2c-L closes the lifecycle and presentation boundary of ADR 0023. It owns the four-state sidebar
projection, silent normal synchronization, one active blocked notification per root, structured
development diagnostics, immediate window hiding, and the distinction between resumable full scans
and restart-only non-scan continuity work. R2c-M owns replacement performance and final closeout.

## Implemented contract

- The sidebar continues to expose only `已同步`, `正在更新图库`, `更新受阻`, and `目录不可用`.
  `需要核对` is absent from production and test presentation strings.
- Rust synchronization snapshots expose an Ame-owned phase enum for watcher startup, metadata
  enumeration, metadata comparison, queue publication, retry wait, bounded reconciliation, full
  scan, blocked, synchronized, and unavailable states.
- The metadata inventory worker reports enumeration, comparison, and candidate-publication phase
  transitions through a process-local bounded callback. The callback does not cross a persistence,
  dependency, or presentation boundary.
- Flutter retains one phase start time per root generation and resets it only when the phase
  changes. Development logs include the active root phase, phase elapsed milliseconds, pending,
  retry, and freshness-gap counts, source status, and stable issue code.
- Normal update, automatic retry, and successful convergence remain silent. A blocked condition
  uses one root-stable notification key; a cause transition updates the active record in place.
  Notification detail includes the localized phase, elapsed time, bounded queue counts, source
  path, and technical issue code.
- An automatic retry cannot replace an existing blocked projection with an updating projection.
  The blocked phase, cause, issue code, and phase start remain stable until a synchronized snapshot
  proves convergence.
- Window close hides the desktop surface before waiting for shutdown. Bounded and metadata
  inventory workers receive cancellation and retain no process-local authority after restart.
  Full scans alone keep their persisted checkpoint and remain eligible for resume.

## Verification evidence

- synchronization runtime: 19 passed;
- metadata inventory: 15 passed, including enumeration, comparison, and queue-publication phase
  reporting;
- production recovery lifecycle: 11 passed, including non-scan cancellation, full-scan retention,
  stale continuity cancellation, and bounded stop ownership;
- Flutter synchronization: 10 passed, including typed phase mapping, stable phase timing, stable
  blocked projection, and structured development logging;
- gallery synchronization and notification integration: 15 passed, including root-keyed cause
  updates, silent normal synchronization, convergence, and no manual scan fallback;
- notification controller: four passed;
- navigation semantics: five passed;
- window lifecycle: three passed, including hide-before-wait and bounded shutdown timeout;
- repository lint: passed with formatting, Clippy warnings denied, and Dart analysis;
- complete Daily: Rust 452 total, 445 passed, seven existing explicit ignores; all Flutter tests,
  Windows Scan 2/2, Windows Accessibility 2/2, bridge compatibility, and whitespace passed;
- Windows Release: passed with the Release build and packaged bridge smoke integration 2/2.

All filesystem fixtures use disposable directories and isolated catalogs. No real library root was
accessed, no placeholder was hydrated, and no source media was modified. Independent read-only
audit remains required before R2c-L can be accepted or merged into `codex/r2c`.

## Next boundary

After independent audit and merge, R2c-M measures replacement event-to-visible latency and
metadata-only startup continuity, repeats migration and source-safety evidence, and performs the
final R2c closeout gates.
