# ADR 0018: Persist normalized library changes in a leased SQLite queue

- Status: Accepted
- Date: 2026-08-17

## Context

ADR 0016 defines platform-independent library-change intents and ADR 0017 produces those intents
from a bounded Windows observer. R2c-C must preserve the normalized work across application exit,
stabilize duplicate bursts, prevent stale workers from publishing over newer evidence, and expose
bounded retry and health state. An in-memory debouncer cannot satisfy those guarantees.

The queue contains durable task state. It belongs to the Rust application and persistence boundary,
not to `notify`, Flutter, the source tree, or rebuildable preview storage. R2c-D will consume leased
work and publish catalog deltas; this decision does not define that later reconciliation transaction.

## Decision drivers

- crash recovery without replaying an unverifiable partial watcher history;
- deterministic path, subtree, and root supersession after ADR 0016 normalization;
- a stale-result guard that remains valid across lease expiry and process restart;
- bounded ingress, retained unresolved work, lease batches, retry attempts, and cleanup;
- forward migration without rewriting or discarding existing catalog state;
- replacement behind an Ame-owned queue port.

## Considered options

### Dependency-owned debounce

Rejected. `notify` debouncer crates provide in-memory timing policy but do not own Ame root
generations, catalog revisions, retry evidence, crash recovery, or authoritative freshness gaps.

### A second queue database or external service

Rejected for this slice. It would add another transaction and packaging boundary without a
measured need. The existing application-managed SQLite catalog already owns durable task state and
has WAL, busy-timeout, migration, and recovery policy.

### SQLite-backed Ame queue

Accepted. The queue is exposed through `LibraryChangeQueue` and implemented by `SqliteCatalog`.
Application code persists a complete `LibraryChangePlanningResult` through
`enqueue_library_change_plan`; presentation and watcher dependency types never cross this port.

## Decision

Schema v17 adds `library_change_queue` and `library_change_root_state`. Queue rows use SQLite
`AUTOINCREMENT` IDs so an ID is not reused after terminal cleanup. Every row persists its root and
generation, intent kind and scope, current and optional previous relative paths, origin,
first/recent observation times and sequences, coalesced count, state, stabilization deadline,
attempt and retry state, monotonic lease generation, structured last failure, enqueue and success
catalog revisions, optional catch-up source and watermark, and supersession link. Observation
sequences are stored as decimal text so the full Ame `u64` range round-trips without signed SQLite
ordering assumptions. A sequence is monotonic only within one change-source instance. Planning may
use it inside that bounded source batch, but durable merging across enqueue and restart boundaries
treats the current enqueue transaction as newer than rows it loaded from durable storage. The
incoming observation time, sequence, and origin replace the stored latest tuple even if the wall
clock moved backward or the source origin changed; the earliest observed time and sequence remain
range evidence. It never compares raw sequence values or wall-clock timestamps as a cross-source
instance clock. A high sequence or future-skewed timestamp from an older source instance therefore
cannot replace later ingress or postpone its stabilization deadline after the watcher restarts.

The root-state table records the highest accepted generation plus an active or retired tombstone.
A newer generation atomically supersedes unresolved older work. An older generation, or the same
generation after root unregister, cannot enqueue or lease work. Re-registering a root must advance
the generation before queue ingress. Schema migration seeds generation authority for every
registered root, scan registration establishes a missing initial authority or advances an inactive
tombstone, and unregister retires the exact stored generation even when that root never queued
work. Enqueue also requires the root to remain present in the authoritative `library_roots` table;
it fails closed when authority is missing and cannot reactivate an inactive tombstone. Retired
tombstones permanently retain the highest accepted generation, so terminal-row cleanup and later
re-registration cannot erase the ordering authority or admit the retired generation again.
Schema v17 records a singleton contract marker only after this authority is complete. A prerelease
v17 database without that marker fails closed on open because a previously deleted high-generation
tombstone cannot be reconstructed safely as generation 1.

Queue states are `pending`, `leased`, `retry_wait`, `completed`, and `superseded`. Leasing is
transactional, increments both attempt count and `lease_generation`, and has an explicit expiry.
Completion or retry must match the current row ID and lease generation. A later same-target event
supersedes a leased row and inserts replacement work, so the old worker receives `Superseded`
instead of acknowledging stale output. An expired lease becomes retry-wait with structured failure
evidence and bounded exponential backoff. The default policy allows eight attempts, begins at one
second, and caps at five minutes; absolute policy bounds reject more than 32 attempts or a one-hour
delay. New source evidence reopens an exhausted row with a fresh bounded attempt budget.
If a valid runtime policy lowers the attempt limit, existing retry-wait work at the new limit is
reported as exhausted immediately and its obsolete retry deadline is cleared on the next lease
pass.

The initial configurable stabilization default is 500 ms. Controlled fixtures place repeated
create, modify, and remove evidence 50-100 ms apart and prove that it becomes one final-state
reconciliation after the deadline. This is validation evidence for the initial default, not a
permanent timing promise; later controlled Windows burst measurements may tune the policy without a
schema or port change. SQLite is never called from the `notify` callback: the existing bounded
observer drain first produces ADR 0016 intents, then the application persists the plan.

Coalescing applies before retained-work capacity:

- equal unleased work merges observation ranges and resets the stabilization deadline;
- a root intent absorbs narrower unleased work;
- a parent subtree absorbs child paths and nested subtrees only when every affected rename path is
  inside that subtree;
- a directory rename treats both its old and new paths as subtree roots for descendant overlap,
  lease invalidation, and conflicting nested rename evidence;
- a paired rename remains one row with both paths;
- create followed by remove remains a reconciliation of final filesystem state rather than being
  discarded;
- conflicting rename evidence sharing either old or new paths and capacity overflow degrade to one
  root `FreshnessUnknown` row;
- leased overlapping work compares every affected old and new path, and ambiguous overlap degrades
  to the same root gap instead of discarding half of a rename.

The capacity check measures normalized retained work. A valid lower runtime limit does not create a
false overflow when an incoming parent subtree or root reconciliation first absorbs enough existing
rows to fit the new bound.

The default and absolute unresolved-work bound is 4096 rows per root generation. Lease batches are
64 by default and at most 128. Stabilization, retry, and lease durations have absolute bounds.
Cleanup removes only completed or superseded queue rows. The default terminal retention is seven
days, and every non-empty enqueue transaction first removes at most 128 eligible rows; callers may
also request an explicit cleanup of at most 1024 rows. Pending, leased, retry-wait, unresolved
freshness-gap work, and root-generation tombstones are never removed by cleanup.
Structured metrics expose every state count, ready work, expired leases, exhausted retries,
unresolved freshness gaps, health, and oldest ready delay without mutating work.

## Validation gates

- schema v16 migrates to v17 without losing prior rows, and every older committed migration chain
  still reaches the current schema;
- multiple R2c-B plans for one path become one durable row with complete observation evidence;
- dropping and reopening the catalog after enqueue leases the same work after the stabilization
  deadline;
- a watcher sequence reset, wall-clock rollback, or origin change cannot make older source-instance
  evidence become the durable latest tuple or delay newer ingress;
- paired rename paths, subtree supersession, root capacity degradation, and root generations remain
  deterministic across restart;
- root removal, terminal cleanup, zero-work lifecycles, and re-registration still require a
  strictly newer generation before queue ingress;
- a prerelease v17 catalog without complete root-generation authority fails closed on reopen;
- a later event on either affected rename path or directory-rename subtree prevents the earlier
  lease from completing;
- lowering capacity before a parent subtree or root event measures the normalized retained result;
- lease expiry, structured retry, retry exhaustion, and new-evidence reopening are bounded;
- queue metrics and retention cleanup are structured, non-mutating, and bounded;
- repository Daily passes without accessing a real library or modifying source media.

## Validation evidence

Thirty-six focused queue tests cover the v16 migration, root-state seeding, prerelease v17
authority-marker rejection, repeated plans,
process and watcher restart recovery, equal-time origin changes, wall-clock rollback, paired and
directory rename persistence and overlap, divergent nested rename evidence, create/remove
final-state work, subtree and normalized capacity supersession under a lowered policy, lifecycle
generation activation including roots with no queue work, permanent removed-root rejection after
cleanup and re-registration, missing-authority failure, lease-expiry recovery, explicit and
policy-adjusted retry exhaustion, new-evidence reopening, delay metrics, explicit bounded cleanup,
and automatic retention. The full Rust suite reached schema v17 through every historical migration
fixture with 239 passing tests and five existing explicit authorization- or performance-bound
ignores. Clippy passed for all targets and features with warnings denied.

The 2026-08-17 lock-aware Daily gate passed formatting, Rust, Dart analysis, every Flutter test,
the controlled Windows picker-and-scan integration, both native accessibility scenarios, generated
bridge compatibility, release guardrails, and whitespace validation. The first sandboxed Daily
attempt reached the Flutter SDK lock without creating a Dart child and was stopped; the identical
repository command then passed outside the workspace-only sandbox as required by the local
toolchain contract. No real-library root was accessed and no source media was read or modified by
R2c-C fixtures.

## Consequences and risks

- R2c-C stores and schedules work but does not inspect source paths or publish catalog deltas. The
  catalog remains unchanged until R2c-D adds identity-aware reconciliation and atomic publication.
- Coalescing is intentionally conservative. Ambiguous rename evidence spends a root audit instead
  of risking a lost old path.
- Durable recency uses transaction ingress precedence across source instances; observation time,
  source-local sequence, and origin are retained as evidence, not used as cross-restart clocks.
- SQLite writer contention can delay enqueue or leasing and returns existing structured busy or
  locked errors; it never silently drops an intent while claiming synchronization.
- Per-root unresolved work is bounded. Root state grows only with distinct historical root IDs;
  those compact tombstones are deliberately permanent generation authority.

## Replacement strategy

Replace only the `LibraryChangeQueue` adapter. A replacement must preserve stable change identity,
root generations, coalescing evidence, lease-generation stale guards, retry state, catalog
revisions, catch-up fields, and terminal retention semantics. Changing storage must use a verified
forward migration and cannot require a catalog, Flutter, or watcher contract rewrite.
