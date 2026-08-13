# R2c-B Windows observation validation

- Date: 2026-08-14
- Scope: recursive Windows change-source adapter and bounded per-root lifecycle
- Source access: temporary fixture directories only

## Acceptance contract

R2c-B is accepted only when controlled running-time filesystem changes cross the native Windows
watcher boundary as Ame-owned observations and R2c-A intents without decoding media, invoking
Flutter, walking subtrees in the callback, using SQLite, blocking the poll path, or retaining an
unbounded event stream.

The implementation uses the patched `notify` 8.2.0 source admitted by ADR 0017 with default
features disabled in the Windows target. One `LibraryChangeObserver` owns each available root.
Callback ingress uses `sync_channel` with a
caller-selected positive capacity no greater than 4,096. Polling drains no more than the R2c-A
observation limit and never sleeps; a degraded or failed source is first isolated and restarts only
when an explicit clock reaches a bounded exponential-backoff deadline.

## Failure and safety matrix

| Scenario | Required result | Evidence |
| --- | --- | --- |
| Create, modify, rename, remove | Relative Ame observations produce ADR 0016 intents | Real temporary recursive watcher fixture |
| Paired Windows rename callbacks | Old and new paths remain one reliable rename candidate | Direct `From`/`To` and `Both` fixtures |
| Incomplete rename | Root evidence gap; never new-path-only reconciliation | Expired rename-pair fixture |
| Rename halves exceed correlation grace | Reject reliable pairing and preserve a root evidence gap | Delayed `From`/`To` fixture |
| Native Windows notification overflow | `Flag::Rescan`, degraded health, and root evidence gap | Real blocked-callback 16 KiB overflow fixture |
| Rescan or callback error | Degraded or failed health plus root evidence gap | Forced dependency event/error fixtures |
| Event storm | `try_send` never blocks; overflow count and evidence gap are retained | Capacity-one ingress fixture |
| Ambiguous directory removal | Subtree reconciliation rather than a single file-path claim | `RemoveKind::Any` fixture |
| Root generation changes | Late observations cannot enter the current plan | Fake lifecycle generation fixture |
| Start or runtime failure | Structured error and bounded explicit-clock restart backoff | Fake factory/source fixtures |
| Root removal or window close | Failed health plus root gap, then stop within two seconds | Real removed-root runtime and stop fixtures |
| Delivered degraded gap | Isolate the source and recover through bounded restart | Explicit-clock lifecycle fixture |
| Stop failure | Do not start a replacement while the old watcher may still be live | Failed-stop lifecycle fixture |
| Metadata disappears before callback handling | Preserve conservative work and mark an evidence gap | Vanished-entry fixture |
| Chinese and long paths | Preserve relative UTF-8 path evidence | Real Chinese path and direct long-path fixtures |
| Source safety | No media decoding or mutation | Callback implementation inspection and unchanged sentinel |

## Verification evidence

- Three vendored backend tests prove that access-denied completions never enter the record parser
  and that only a nonempty successful completion can contain normal records.
- Eight application lifecycle tests cover initial failure, explicit-clock restart, runtime failure,
  stale generation, idempotent stop, bounded slow stop, failed-stop isolation, and invalid planning
  bounds.
- Fourteen adapter tests cover event translation, real recursive Windows changes, evidence gaps,
  bounded ingress, directory ambiguity, health severity, root removal, shutdown-boundary callbacks,
  expired rename pairs, vanished metadata, Chinese paths, long paths, unavailable roots, and invalid
  capacities, native notification-buffer loss, and root-loss health projection.
- Rust Clippy passes for all targets and features with warnings denied.
- The complete 2026-08-14 Daily gate passes all 202 Rust tests: 197 pass and five authorization- or
  performance-bound tests remain intentionally ignored; all Flutter tests, controlled Windows scan
  integration, native Windows accessibility integration, bridge compatibility, and whitespace
  validation pass.
- The 2026-08-14 Windows Release gate builds the packaged application and passes both release-bridge
  smoke tests against the packaged Rust library.

## Residual boundaries

- No durable queue, lease, debounce, retry persistence, or crash recovery exists yet; R2c-C owns
  those semantics.
- No catalog row or revision changes in this slice; R2c-D owns atomic incremental publication.
- The production Flutter lifecycle and freshness presentation remain R2c-E work.
- Startup catch-up, consistency audit, and real-library event acceptance remain later R2c slices and
  require separate authorization where real roots are involved.
