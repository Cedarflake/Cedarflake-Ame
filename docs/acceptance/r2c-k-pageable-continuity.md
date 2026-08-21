# R2c-K pageable continuity acceptance

Status: implementation and repository verification complete; independent audit pending

Date: 2026-08-22

## Scope

R2c-K connects the schema v20 metadata inventory to the production synchronization coordinator. It
owns continuity epochs, automatic root and subtree routing, bounded candidate backpressure,
live-evidence supersession, retry exhaustion, cancellation, and per-root fairness. Product wording,
notification policy, development diagnostics, and immediate window-close behavior remain R2c-L.

## Implemented contract

- A healthy watcher establishes a monotonically increasing in-memory continuity revision before
  startup, availability-recovery, watcher-restart, rescan, incomplete-rename, or overflow authority
  is scheduled.
- Root freshness gaps and legacy startup or consistency-audit authority enter metadata inventory
  directly. A bounded subtree that exceeds 4,096 entries or 128 affected paths retains its durable
  retry row and continues as an inventory scope rather than creating a full scan.
- SQLite allocates each root-generation inventory epoch inside the same immediate transaction that
  supersedes an older active run. Wall-clock rollback cannot reuse an epoch, and an exact active run
  identity resumes its durable comparison and absence cursors.
- Enumeration remains metadata-only and may stage the complete root without holding it in memory.
  Production then admits at most one comparison or absence page per worker turn and rotates the
  authoritative root cursor before the next page.
- Inventory candidate enqueue verifies the exact authoritative lease and protects that control row
  from absorbing its own path output. The configured retained-work limit excludes that one control
  reservation while the absolute 4,096-row limit remains binding; production pages therefore admit
  at most 4,095 candidates.
- If the path queue lacks capacity, comparison and absence cursors do not advance. The authority is
  deferred without consuming an attempt, foreground path work drains, and continuation resumes from
  the same run identity.
- Positive candidates use the existing final-state path reconciler and may publish between inventory
  pages. Removal candidates are not loaded until complete enumeration grants absence authority, and
  every candidate still rechecks current filesystem and catalog state before publication.
- Cloud Files identity is resolved from attributes plus reparse tag without following the target.
  Fully local Cloud Files remain readable file entries; offline, partial, or recall content remains
  present without hydration. Directory enumeration retains `RecallOnOpen`, while exact paths use a
  no-follow, no-recall `FileAttributeTagInfo` query instead of scanning siblings. Source reads keep a
  live validation handle, open content with no-recall, and recheck availability and matching file
  identity before consuming bytes. Matching unavailable entries preserve trustworthy locations
  without removal work or retry exhaustion; changed evidence remains unresolved.
- A later watcher gap increments the continuity revision, cancels the older worker, supersedes its
  queue authority and staged candidates, and starts a higher durable epoch. A stale worker cannot
  enqueue another candidate page after losing its lease.
- Cancellation terminates the derived run, defers non-scan authority, and preserves the last
  trustworthy catalog. Restart creates a new continuity epoch; only an already-started full scan
  remains eligible for checkpoint resume.
- Inventory failures use the queue's durable retry and exhaustion policy. Exhausted work remains a
  degraded root with its structured issue and does not create or resume a new full scan.
- The existing rotating root cursor applies to every bounded-authoritative and inventory page task,
  preventing a large first root from owning every worker turn.
- Failure to create a background worker immediately defers the already-acquired lease. The lease is
  never left waiting for nominal expiry merely because worker startup failed.

## Focused verification

- metadata inventory: 20 passed, covering atomic next-epoch allocation, clock rollback, startup
  convergence, capacity one with a protected authority, incomplete-run continuation, newer-gap
  supersession, cancellation, durable retry exhaustion, absence preservation, placeholders,
  reparse rejection, hard-link identity claims, cleanup, and terminalization;
- cloud-placeholder classification: two passed, covering the production reparse-plus-recall shape
  and an existing catalog location completing inventory without removal or retry work;
- authoritative recovery: eight passed, including oversized subtree transition to pageable
  inventory, unchanged full-scan ownership, exact additions and removals, cancellation, placeholders,
  rename identity, and bounded policy validation;
- synchronization lifecycle: 34 selected, 32 passed and two authorization-bound R2c-H tests
  ignored, including continuity-revision cancellation, root fairness, watcher-before-authority,
  availability recovery, retry isolation, bounded shutdown, and recoverable-scan rotation;
- complete Rust suite: 451 total, 444 passed, seven existing explicit ignores, zero failures;
- `cargo check --all-targets --all-features`: passed with no warnings;
- repository lint: passed, including formatting, Clippy with warnings denied, and Dart analysis;
- complete Daily: passed, including the Rust suite, all Flutter tests, Windows Scan 2/2, Windows
  Accessibility 2/2, bridge compatibility, guardrails, and whitespace validation;
- Windows Release: passed, including the Release build and packaged bridge smoke test 2/2.

The fixtures use disposable directories and isolated catalogs. They do not access `local-primary`
or `cloud-primary`, hydrate cloud content, or modify any real source media. Independent read-only
audit evidence remains required before R2c-K is accepted.

## Next boundary

R2c-L owns the four-state presentation mapping, blocked-notification deduplication, phase and elapsed
development diagnostics, and immediate window hiding with non-scan cancellation. R2c-K does not
claim those presentation or desktop-close behaviors.
