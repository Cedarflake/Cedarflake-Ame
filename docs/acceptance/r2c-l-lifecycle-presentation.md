# R2c-L lifecycle, presentation, and diagnostics acceptance

Status: accepted after independent audit

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
- Durable exhausted work projects its stored failure code after process restart. Active recovery
  follows a typed recovery-blocked flag, so it cannot hide an exhausted queue, while a protected
  live worker crossing its nominal lease and transient persistence contention continue to expose
  the worker's real phase.
- Normal update, automatic retry, and successful convergence remain silent. A blocked condition
  uses one root-and-generation-stable notification key; a cause transition updates the active
  record in place. Notification detail includes the localized phase, a structured phase start used
  to calculate current elapsed time when opened, bounded queue counts, source path, and technical
  issue code.
- An automatic retry cannot replace an existing blocked projection with an updating projection.
  The blocked phase, cause, issue code, and phase start remain stable until a synchronized snapshot
  proves convergence.
- Window close hides the desktop surface before waiting for shutdown. Bounded and metadata
  inventory workers receive cancellation and retain no process-local authority after restart.
  Full scans alone keep their persisted checkpoint and remain eligible for resume.

## Verification evidence

- synchronization runtime: 20 passed, including durable exhausted-failure projection after runtime
  recreation;
- metadata inventory: 15 passed, including enumeration, comparison, and queue-publication phase
  reporting;
- production recovery lifecycle: 12 passed, including protected live work beyond nominal lease,
  exhausted-work blocking, transient contention phase reporting, non-scan cancellation, full-scan
  retention, stale continuity cancellation, and bounded stop ownership;
- Flutter synchronization: 11 passed, including typed phase mapping, stable phase timing,
  generation-bounded blocked projection, and structured development logging;
- gallery synchronization and notification integration: 16 passed, including generation-bounded
  active conditions, root-keyed cause updates, silent normal synchronization, convergence, and no
  manual scan fallback;
- notification controller: four passed;
- notification presentation: five passed, including current elapsed-time calculation when details
  open;
- navigation semantics: five passed;
- window lifecycle: three passed, including hide-before-wait and bounded shutdown timeout;
- repository lint: passed with formatting, Clippy warnings denied, and Dart analysis;
- complete Daily: Rust 454 total, 447 passed, seven existing explicit ignores; all Flutter tests,
  Windows Scan 2/2, Windows Accessibility 2/2, bridge compatibility, and whitespace passed;
- Windows Release: passed with the Release build and packaged bridge smoke integration 2/2.

All filesystem fixtures use disposable directories and isolated catalogs. No real library root was
accessed, no placeholder was hydrated, and no source media was modified. Independent read-only
audit reported no Critical, High, Medium, or Low findings.

## Next boundary

After merge, R2c-M measures replacement event-to-visible latency and metadata-only startup
continuity, repeats migration and source-safety evidence, and performs the final R2c closeout gates.
