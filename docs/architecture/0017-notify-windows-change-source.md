# ADR 0017: Validate notify 8.2.0 behind the Windows change-source adapter

- Status: Accepted for validation
- Date: 2026-08-13

## Context

R2c-B needs a maintained recursive Windows filesystem observation library. R2c-A records the
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

### `notify` 8.2.0 recommended watcher

Selected for validation. The stable release uses Windows `ReadDirectoryChangesW`, supports
recursive directory watching, exposes a rescan flag when events may have been missed, and is used by
several mature Rust applications. Version 8.2.0 declares Rust 1.77 as its minimum and CC0-1.0 as its
license, compatible with Ame's pinned Rust 1.97.1 toolchain and distribution policy.

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

R2c-B may admit exact stable `notify` version 8.2.0 behind a Windows
`LibraryChangeSource` adapter after its focused gate is implemented. R2c-A does not add it to
`Cargo.toml` because no production code uses it yet.

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

## Consequences and risks

- Native notification buffers can overflow and large directories may miss events. The adapter
  cannot claim completeness; explicit degradation and consistency recovery remain mandatory.
- Recursive watch behavior can vary when watched paths are removed or renamed. Root generation and
  authoritative reconciliation contain the uncertainty.
- The stable dependency line may raise its MSRV in a future minor release. Ame pins the admitted
  version and reviews upgrades deliberately.
- The base library does not provide Ame's durable debounce, queue, or recovery semantics; this is an
  intentional application responsibility.

## Replacement strategy

Replace only the adapter with a newer stable `notify`, a direct Windows implementation, or another
maintained watcher. ADR 0016 observations, queue state, reconciliation results, catalog schema,
bridge, and Flutter remain unchanged. A failed R2c-B validation reopens this decision without
weakening freshness or source-safety rules.
