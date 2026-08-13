# ADR 0016: Normalize continuous library changes before reconciliation

- Status: Accepted
- Date: 2026-08-13

## Context

R1 proves that a complete explicit scan can reconcile unchanged files, edits, identity-proven
renames, replacements, and removals. R2c must add continuous synchronization without treating a
filesystem notification as authoritative state or creating a second reconciliation policy.

Platform events can be duplicated, reordered, incomplete, or delivered after a path changes again.
They also arrive through platform-specific APIs whose event kinds and rename semantics must not
enter Ame's domain, persistence, bridge, or presentation contracts. The active catalog currently
publishes complete root snapshots, so this decision defines the platform-independent contract only;
durable queue storage and atomic delta publication remain later R2c slices.

## Decision drivers

- converge on final filesystem evidence instead of trusting event order or kind;
- reuse ADR 0007 file-identity semantics for edits, renames, and replacements;
- prevent work from an obsolete root configuration from publishing;
- bound ingress and degrade visibly when evidence may be incomplete;
- retain the last trustworthy catalog for unavailable, partial, or failed inspection;
- keep Flutter presentation-only and keep source media read-only.

## Considered options

### Apply each raw event directly

Rejected. A delayed delete could remove a replacement, a rename could appear as two unrelated
changes, and duplicate bursts could create unbounded work. Event delivery is not final-state
evidence.

### Put watcher kinds and coalescing rules in Flutter

Rejected. Flutter would acquire platform, catalog, and identity policy and could no longer present
one stable application contract across watcher replacement or recovery paths.

### Normalize hints in Rust and reconcile final evidence

Accepted. Adapters translate raw signals into bounded Ame observations. The application turns an
already collected bounded batch into deterministic intents; later slices persist, lease, stabilize,
and execute those intents.

## Decision

Expose a selective Rust `synchronization` facade containing Ame-owned values and two deterministic
application functions. No third-party or Windows type crosses this boundary.

Each observation carries a logical `root_id`, nonzero monotonic root configuration generation,
application sequence, observation time, origin, scope, normalized relative path, and optional prior
rename path. Generations prevent work for a removed or reconfigured root from entering the current
plan. Sequence and time are evidence for deterministic merging; neither makes the event
authoritative.

Normalization applies these rules:

1. slash-normalize relative paths and reject absolute, drive-qualified, or parent-escaping paths;
2. coalesce create, modify, remove, duplicate, and reordered signals for one path into one final
   reconciliation intent;
3. retain a reliably paired rename as one candidate, but degrade an unpaired rename into old- and
   new-path reconciliation; when the old path is absent, treat the signal as an evidence gap;
4. promote directory signals to subtree scope and let a parent subtree supersede contained path or
   nested-subtree work while retaining their observation evidence; a root-directory signal
   supersedes all narrower work;
5. ignore observations from another root or generation;
6. replace the batch with one root `FreshnessUnknown` intent when paths are invalid, event evidence
   has a known gap, source health is degraded, or configured capacity is exceeded.

The planner is deliberately not the durable queue or debounce implementation. R2c-C owns the
stabilization window, persistence, leasing, retry, backoff, supersession of in-flight work, and
bounded retention. R2c-A establishes the deterministic policy those mechanisms must preserve.

Catalog freshness is separate from root availability and has four states: `Synchronized`,
`Updating`, `NeedsReconciliation`, and `Unavailable`. A failed source or bounded-capacity overflow
cannot claim synchronized state. An unavailable root keeps pending intent and its last trustworthy
catalog rather than publishing mass removal.

Reconciliation decisions consume inspected final state, not notification kind. They follow ADR
0007 in this order:

1. matching file identity preserves the asset through rename or edit;
2. matching identity plus unchanged source state retains compatible derived evidence;
3. matching identity plus changed state invalidates derived evidence;
4. conflicting known identity at the same path is a replacement even when size and time match;
5. absent identity permits only conservative same-path unchanged fallback;
6. removal requires authoritative absence;
7. retryable, terminal, skipped, or non-authoritative absence preserves the last trustworthy
   catalog evidence.

The contract describes future evidence disposition as retain, invalidate, no reusable evidence,
remove from current projection, or preserve last trustworthy state. It does not implement future
fingerprint, similarity, or classification engines.

## Consequences and risks

- R2c-B can replace its watcher without changing application, catalog, or Flutter types.
- R2c-C must persist the same generation and ordering evidence; an in-memory-only queue is not
  sufficient.
- R2c-D must add atomic delta persistence instead of misusing the complete-snapshot scan publisher.
- Path lexical normalization does not prove a filesystem entry exists or resolve links. The
  filesystem adapter must inspect and revalidate the entry before publication.
- A root-level fallback is more expensive than path reconciliation but is required when bounded
  evidence cannot prove completeness.
- Caller-selected planning limits cannot exceed the contract ceilings of 4,096 observations and
  1,024 intents. Parent coverage is resolved before the intent ceiling is applied.

## Validation evidence

- Deterministic Rust fixtures cover create/modify coalescing, transient create/remove, paired and
  unpaired rename, directory and root supersession, stale generations, unavailable roots, evidence
  gaps, invalid paths, Chinese and long paths, event storms, intent capacity, identity-preserving
  edit and rename, same-state replacement, authoritative removal, and failure preservation.
- The fixtures run without a platform watcher, database migration, Flutter policy, or source-media
  access.
- Forty-two focused synchronization tests pass, including 20 adversarial blue-team fixtures and
  the selective Rust facade contract. The attack matrix and residual boundaries are recorded in
  `docs/acceptance/r2c-a-blue-team.md`.
- The repository lint gate passes with 131 Dart files unchanged, Flutter analysis clean, and Rust
  Clippy clean for all targets and features with warnings denied.
- The complete Daily gate passes: 173 Rust tests pass with five authorization- or
  performance-bound tests intentionally ignored; all Flutter tests, controlled Windows scan
  integration, native Windows accessibility integration, generated bridge compatibility, and
  tracked-diff whitespace validation pass.

## Replacement strategy

Add contract versions or new intent and outcome variants behind the same facade. Preserve the
generation, final-evidence, bounded-degradation, and last-trustworthy-state invariants. A later ADR
may revise persistence or batching without exposing dependency event types or moving policy into
Flutter.
