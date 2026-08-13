# ADR 0017: Use a patched notify 8.2.0 behind the Windows change-source adapter

- Status: Accepted
- Date: 2026-08-13
- Corrected: 2026-08-14

## Context

R2c-B needed a maintained recursive Windows filesystem observation library. R2c-A recorded the
dependency decision before admission, while deliberately avoiding an unused Cargo dependency or a
premature platform implementation.

The watcher supplies hints only. ADR 0016 owns normalized observations and ADR 0007 owns file
identity and final reconciliation. No dependency event, error, path, thread, or lifecycle type may
cross the adapter.

## Decision drivers

- recursive Windows observation through a narrow Rust API;
- active maintenance and credible real-world adoption;
- license and Rust-toolchain compatibility;
- explicit event-loss and recovery signaling;
- bounded callback, shutdown, restart, and packaging behavior;
- replacement without catalog, application-contract, or Flutter migration.

## Considered options

### Patched `notify` 8.2.0 recommended watcher

Selected for validation. The stable release uses Windows `ReadDirectoryChangesW`, supports
recursive directory watching, and is used by several mature Rust applications. Version 8.2.0
declares Rust 1.77 as its minimum and CC0-1.0 as its license, compatible with Ame's pinned Rust
1.97.1 toolchain and distribution policy. Audit found that the published Windows backend ignores
zero-byte and `ERROR_NOTIFY_ENUM_DIR` completions, silently unwatches a removed root, and drops
other completion and rearm errors. Those gaps require the narrow upstream-derived backport recorded
below; unpatched 8.2.0 is not admissible.

### `notify` pre-release 9.x

Rejected for this slice. A release-candidate line provides no R2c requirement that justifies
admitting pre-release API and packaging churn over the current stable release.

### `notify-debouncer-mini` or `notify-debouncer-full`

Not admitted. R2c-C needs application-owned durable coalescing, generation protection, leasing,
retry, and crash recovery. A dependency-owned in-memory debounce policy cannot become the authority
for those semantics. R2c-B may use only the base watcher.

### Direct `ReadDirectoryChangesW` implementation

Rejected initially. It would add Windows buffer parsing, rename correlation, cancellation, and new
`unsafe` maintenance without evidence that the mature adapter is insufficient.

### Polling watcher

Reserved as an explicit fallback experiment, not a default for the approximately 79,000-location
target catalog. Fixed polling would add idle enumeration cost and still would not remove the need
for consistency reconciliation.

## Decision

R2c-B admits the stable `notify` version 8.2.0 source behind a Windows `LibraryChangeSource`
adapter, with a repository-owned Cargo patch under `rust/vendor/notify`. The patch is based on the
crates.io source with checksum
`4d3d07927151ff8575b7087f245456e549fea62edf0ec4e565a5ee50c8402bc3` and backports upstream
Windows fixes `75d72fd1`, `21abf764`, and `d01dc40d`. Its exact scope and replacement instructions
are recorded in `rust/vendor/notify/AME-PATCHES.md`. The dependency remains Windows-targeted,
disables default features, and uses neither a debouncer nor a polling fallback. `notify` and all
transitive event, error, path, thread, and watcher types remain inside the adapter.

The vendored Windows backend retains the dependency's existing `unsafe` Win32 boundary. The
backport may only parse `FILE_NOTIFY_INFORMATION` after a nonzero successful completion, must rearm
before delivering normal records, owns each overlapped request until its completion callback, and
must release or close every handle on start, rearm, unwatch, and stop paths. Focused real-Windows
overflow, root-removal, error, and bounded-stop tests are required whenever that file changes.
The backport exists only because no stable upstream release contains these fixes; the 9.x release
candidate remains rejected, and a future stable release is preferred over carrying this patch.

The adapter must:

- create one bounded recursive watcher lifecycle per available configured root;
- convert callbacks immediately into Ame observations without decoding media, walking a subtree,
  running a long transaction, or invoking Flutter;
- treat every event as a hint and map `Event::need_rescan()` or callback error to an Ame evidence
  gap and degraded health;
- keep dependency paths relative to the configured root before entering ADR 0016 normalization;
- preserve paired rename evidence only when the dependency provides a trustworthy pair;
- stop accepting callbacks before bounded shutdown and never hang desktop close;
- expose restartable structured health while leaving catch-up and consistency recovery to later
  application slices;
- avoid polling unless a separately measured fallback policy accepts its idle cost.

The application owns one `LibraryChangeObserver` lifecycle per configured available root. Its
public Windows facade accepts only Ame values and a root path string. Polling that facade performs
one non-blocking bounded drain and may make at most one restart attempt when the caller-provided
clock reaches the current exponential-backoff deadline; it never sleeps on the UI call path.

The adapter callback performs only lexical relative-path conversion, dependency-event mapping, an
adjacent Windows rename-pair correlation, atomic health accounting, and `try_send` into a bounded
standard channel. It does not read media bytes, enumerate a subtree, call SQLite, or invoke Flutter.
The Windows backend emits rename `From` and `To` callbacks separately, so the adapter grants only a
50 ms non-blocking correlation interval and rejects a `To` half that arrives after that deadline.
A missing half, rescan flag, callback error, invalid path, or channel overflow produces one
coalesced root evidence gap. A short delivery gate linearizes callback enqueue with stop while the
bounded `try_send` remains non-blocking. Health severity is monotonic until the source restarts, so
a later rescan cannot hide a prior callback failure. After a degraded batch delivers its root gap,
the observer isolates and stops that source before scheduling the existing bounded restart; the
same source cannot remain permanently degraded or repeatedly enqueue root work.

Directory create and modify hints use metadata only when the entry still exists. An ambiguous
remove is conservatively promoted to subtree reconciliation because the deleted entry can no longer
prove whether it was a file or directory. A dependency event for the configured root itself becomes
failed health plus a root evidence gap, stops the watcher, and makes unavailable restart attempts
retryable without interpreting the root contents as removed catalog state. Sequence numbers
saturate instead of wrapping.

Dependency default features must be reviewed when the Cargo entry is added. Windows packaging must
not gain an external service or source-tree artifact.

## Validation gates

- controlled Windows create, modify, remove, paired/unpaired rename, and directory changes produce
  ADR 0016 observations and no media reads in the callback;
- root removal and generation change prevent late callbacks from publishing current work;
- forced watcher error and rescan indication mark the source degraded and request authoritative
  reconciliation;
- callback ingress, channel capacity, restart backoff, and memory remain bounded under an event
  storm;
- watcher stop and application close complete within a measured bound;
- Chinese, long, unavailable, and cloud-placeholder paths retain existing safety behavior;
- focused adapter tests, repository Daily gate, Windows Release gate, and source immutability checks
  pass.

Focused evidence includes three vendored completion-classification tests, eight deterministic
application lifecycle tests, and fourteen adapter tests.
The real adapter test creates a temporary recursive root, performs directory creation plus file
create, modify, rename, and removal, plans the resulting Ame observations through ADR 0016, verifies
an untouched sentinel, and stops within the two-second bound. Other fixtures force rescan, callback
error, incomplete rename, bounded-channel overflow, native notification-buffer overflow, stale
generation, root removal, Chinese and long paths, health monotonicity, metadata disappearance,
stop-boundary callbacks, expired rename pairs, invalid planning bounds, failed-stop isolation, and
restart backoff. The 2026-08-14 Daily gate passed all 202 Rust tests with 197 passing and five
authorization- or performance-bound tests ignored, all Flutter tests, the controlled Windows scan
integration, native Windows accessibility integration, bridge compatibility, and whitespace
validation. The Windows Release gate built the packaged application and passed both release-bridge
smoke tests.

## Consequences and risks

- Native notification buffers can overflow and large directories may miss events. The patched
  backend converts the Windows loss signal into `Flag::Rescan`, but the adapter still cannot claim
  completeness; explicit degradation and consistency recovery remain mandatory.
- Recursive watch behavior can vary when watched paths are removed or renamed. Root generation and
  authoritative reconciliation contain the uncertainty.
- The stable dependency line may raise its MSRV in a future minor release. Ame pins the admitted
  version and reviews upgrades deliberately.
- The base library does not provide Ame's durable debounce, queue, or recovery semantics; this is an
  intentional application responsibility.
- R2c-B keeps observation in memory only. R2c-C must persist normalized work before any lifecycle
  may claim crash recovery, and R2c-D must provide catalog delta publication before observations can
  update the visible catalog.

## Replacement strategy

Replace the patched source with a newer stable `notify` only after the native overflow, root-loss,
rearm-error, and shutdown fixtures pass unchanged; otherwise replace only the adapter with a direct
Windows implementation or another maintained watcher. ADR 0016 observations, queue state,
reconciliation results, catalog schema, bridge, and Flutter remain unchanged. A failed R2c-B
validation reopens this decision without weakening freshness or source-safety rules.
