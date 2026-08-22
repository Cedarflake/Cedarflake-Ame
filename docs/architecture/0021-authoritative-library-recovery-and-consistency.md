# ADR 0021: Recover library freshness through bounded authoritative reconciliation

- Status: Accepted
- Date: 2026-08-18
- Last amended: 2026-08-21

## Context

ADR 0016 through ADR 0020 establish normalized change evidence, the Windows observer, a durable
leased queue, atomic path-level publication, and the production desktop lifecycle. Path work can
normally converge without scanning a whole root. Directory changes, watcher evidence gaps, queue
overflow, cold start, and source recovery may still require stronger authority than one path
inspection can provide.

Recovery must not convert a temporarily unreadable directory, cloud placeholder, interrupted scan,
or database failure into mass removal. It must also remain bounded for very large libraries and
must not allow a full scan to erase evidence that arrived after the scan started.

## Decision drivers

- regain trustworthy freshness without weakening ADR 0007 file-identity rules;
- reconcile a bounded subtree or root at one catalog revision;
- preserve the last trustworthy catalog when enumeration or inspection is incomplete;
- escalate oversized work to the existing resumable full-scan pipeline;
- coordinate full scans with the durable change queue and root generation;
- keep observation and recovery read-only with respect to source media;
- run recovery outside watcher callbacks and the desktop polling lock;
- prevent elapsed time, ordinary restart, or recoverable watcher interruption from triggering a
  complete root scan.

## Considered options

### Treat every gap as independent path work

Rejected. A missing rename half, watcher overflow, or directory-level notification does not provide
an authoritative finite path set. Path-only work could leave stale descendants or publish false
absence.

### Run a complete root scan for every non-path intent

Rejected. This is safe but unnecessarily expensive for bounded directory changes and creates poor
behavior for large libraries.

### Reconcile bounded scopes and continue oversized scopes as metadata inventory

Accepted as amended by ADR 0023. A bounded authoritative worker handles ordinary subtree and small-
root recovery. When either the enumerated source set or active catalog set exceeds one batch, the
application continues the same authority as a pageable metadata-inventory run. It does not request
a full scan.

## Decision

The application owns an authoritative recovery worker separate from the path worker. Production
runs this worker on a cancellable background thread outside the desktop polling mutex and leases one
subtree, root, or freshness-gap intent only when the root generation is current, a trustworthy
catalog is published, no full scan is active, and a healthy observer has established the current
live-notification boundary. Evidence completeness and observer transport health are distinct. A
root-scoped rescan signal, incomplete rename, known event loss, bounded ingress overflow, cold start,
or watcher restart gap starts the smallest trustworthy ADR 0023 metadata-inventory scope while the
observer covers later changes. A failed or restarting observer leaves its continuity epoch unresolved
until the restarted source is healthy. The worker enumerates one bounded page through the filesystem
adapter with an absolute ceiling of 4,096 directory entries and 128 affected paths. Policies cannot
raise these per-page ceilings; larger scopes continue through durable inventory pages.

A complete root scan is authorized only for first import, an explicit user `更新图库` request, or
resumption of a previously started full scan. Elapsed time, normal file events, ordinary process
restart, retries, watcher interruption, inventory size, and automatic recovery failure never
authorize one. Ame schedules no periodic consistency scan.

Production foreground path polling cannot reclaim the lease held by that active authoritative
worker, even when the bounded filesystem work crosses the nominal lease duration. Only an
authoritative lease pass recovers an expired authoritative row after no in-process worker owns it.
ADR 0020's Windows process mutex makes that in-memory owner authoritative for the one production
process permitted to open the user's catalog lifecycle; another production coordinator cannot
reclaim the row while the live process holds the mutex. After owner loss, Windows releases the
process mutex and a replacement process can recover the expired row through a new SQLite
connection. Crash recovery therefore remains durable without making a slow live worker consume the
retry budget. The readiness check includes an expired final attempt so the recovery pass can record
durable exhausted work instead of leaving the row permanently leased. A lowered retry limit is
normalized by bounded queue maintenance without making authoritative work eligible for path
leasing.

The worker combines the final filesystem paths with every currently published location in the
affected subtree. It prepares additions, modifications, identity-preserving moves, replacements,
and authoritative absences through the same ADR 0007 reconciliation path as R2c-D, revalidates the
complete set, and publishes all mutations plus queue completion in one catalog-delta transaction.
Directory claim, staging, checkpoint, and publication transactions acquire the SQLite writer before
reading mutable scan state. Foreground empty path polls remain read-only, so a background scan cannot
be invalidated by a deferred read snapshot and ordinary polling does not create a competing writer.
Unreadable entries, containment failures, placeholders, database failures, or source races retry
the durable intent without publishing a partial removal set. A cloud placeholder is unresolved
whether or not a prior catalog location exists; recovery neither opens it nor records successful
freshness for that scope.

When the bounded set is exceeded, the worker restores the lease to pending and creates or advances
one metadata-inventory run for the same scope. Production starts bounded work only for an
authoritative queue row that is currently due; future and exhausted retry rows remain projected
without creating an empty worker every poll. Production runs at most one inventory or recovery
worker at a time on a background thread. Failures use a per-root exponential retry from one second
to five minutes; another root is not delayed by that state.
Bounded authoritative selection rotates a per-root cursor after each chosen root and wraps at the
end of the current root snapshot. A continuously ready first root therefore cannot starve another
healthy root with due authoritative work.
Shutdown cancels watcher, metadata inventory, path, subtree, and bounded root work and keeps the
desktop close path bounded. The next process establishes a new live watcher boundary and starts a
new metadata-inventory epoch instead of resuming stale non-scan execution. A running full scan is
different: shutdown requests a durable checkpoint without abandoning its frozen queue authority,
and the next process resumes that scan. If the stop bound expires, the runtime retains the worker
handle in an explicit stopping state and rejects an in-process restart until a later join proves the
old worker ended.

A full scan captures the current root generation and the highest unresolved queue ID in the same
transaction that creates its scan run. A transactional guard plus a unique partial index allow only
one running or paused scan for a root. Pending and retry rows through that watermark are frozen for
that scan without consuming their retry attempts. Publication requires the same active generation,
publishes the catalog snapshot, and completes only queue work through the captured watermark in one
transaction. Evidence arriving later remains unresolved. Abandonment releases only rows frozen by
that scan, leaving independent worker leases unchanged.

Scan lifecycle ownership is durable. Foreground scans remain owned by the Flutter controller;
authoritative recovery scans remain owned by the production synchronization runtime. Each owner
loads only its own recoverable rows, so one scan ID cannot be resumed concurrently by both
lifecycles. Production rotates a persisted-ID cursor over recoverable authoritative scans one row at
a time, wrapping after the last row, so recovery remains bounded without stranding another root.

Schema v18 adds the scan generation, queue watermark, previous-snapshot requirement, scan ownership
of frozen queue rows, foreground-versus-authoritative lifecycle ownership, and a historical
last-successful-authoritative-pass field. The field retains its prerelease
`last_consistency_audit_unix_ms` storage name for migration compatibility, but no runtime policy
uses it to schedule work. Schema v18 also has an explicit contract marker and validates the single-scan
ownership index. An early prerelease v18 database with no conflicting active scans receives the
missing index and lifecycle owner atomically; its reserved `sync-recovery-` IDs identify draft
authoritative rows, while ambiguous overlapping ownership fails closed. Migration from v17
normalizes catalog and scan relative paths to slash-separated form and invalidates pre-v18 running
or paused scans whose queue authority cannot be reconstructed. Because a v17 location identifier
was derived from its historical path spelling, every rescan resolves the active location by root
plus normalized relative path and retains that location identifier before considering file
identity or state changes.

The full-scan pipeline distinguishes structural path-set gaps from bounded failures at exact file
paths. An unreadable directory entry, uncertain containment, cloud placeholder encountered during
enumeration, or other condition that prevents the scanner from proving the root's path set persists
the previous-snapshot requirement and defensively blocks publication, queue completion, and
freshness advancement after restart. A limited replacement scan likewise remains stale instead of
replacing a previously published root with a partial snapshot.

When enumeration is complete but a bounded set of known files cannot be decoded or changes during
final validation, the authoritative scan does not discard independent trustworthy work. It removes
the unverified staged location, retains the prior published location evidence when one exists,
omits an unverified new location, and atomically publishes the remaining root snapshot together
with exact path-level retry intents. Those retry rows are inserted after the scan's captured root
work is completed but inside the same SQLite publication transaction. The root therefore remains
updating and later degrades if retries are exhausted; it cannot be projected as synchronized while
a file path remains unresolved. A restart restores persisted media and final-validation race issues
into the same exact retry contract. More retry paths than the bounded queue can represent, an
unidentifiable affected path, or incomplete directory enumeration continues to fail closed by
retaining the prior snapshot.

After a successful authoritative root reconciliation or full scan, SQLite may continue recording
the historical authoritative-pass timestamp for schema compatibility and diagnostics. That value
does not create future work. Legacy unresolved root-reconcile rows whose origin is
`consistency_audit` are retired as obsolete coordination data without filesystem enumeration,
catalog publication, or a revision change. Historical path retries that reused that origin remain
valid unresolved evidence; new full-scan path retries use the startup-recovery origin. Other origins
and unresolved evidence remain untouched.

Pending or retryable authoritative work that the runtime can process automatically projects as
updating. `NeedsReconciliation` is reserved for a failed observer, an exhausted or otherwise
degraded durable queue, or another condition for which automatic recovery no longer has a healthy
execution path. This keeps the compact UI state aligned with whether user intervention is actually
required without weakening the last-trustworthy-catalog rule. A live in-process authoritative
worker remains updating even after its nominal database lease time passes; lease-age metrics cannot
turn active automatic work into a false manual-action state.

## Validation gates

- controlled temporary fixtures prove bounded subtree addition and removal, directory rename
  identity continuity, and escalation before an oversized scope publishes;
- queue fixtures prove generation and high-watermark capture, preservation of later evidence, and
  selective release after scan abandonment;
- scan fixtures prove foreground corrupt rescans and limited replacement scans preserve the last
  trustworthy catalog, structural and placeholder completeness gaps remain stale, and bounded
  authoritative media or final-validation failures publish independent evidence with exact durable
  path retries;
- restart fixtures prove persisted exact-path issue evidence is restored into retry work while
  unknown, excessive, or structural issue evidence still fails closed;
- migration fixtures prove v17 to v18 path normalization, preservation of healthy and placeholder
  legacy identifiers, invalidation of unverifiable running scans, and repair or fail-closed handling
  of a prerelease v18 database;
- runtime fixtures prove cold-start recovery, degraded-source restart continuity, background-only
  authoritative work, foreground-versus-authoritative recovery isolation, bounded multi-root
  rotation for both bounded and full-scan work, live-worker lease isolation across nominal expiry,
  independent-connection final-attempt crash normalization, policy-lowering exhaustion, managed
  stop timeout, due-only scheduling, legacy-audit retirement, resumable full-scan shutdown, and
  bounded per-root retry;
- the packaged Windows release fixture proves a second same-user process exits before runtime
  initialization and that a replacement process starts after the original owner exits;
- complete format, Clippy, Rust, Flutter, Windows integration, bridge, Daily, and Windows release
  gates pass before the slice is merged;
- no real-library root is accessed by this implementation validation.

## Validation evidence

The controlled commands and results are recorded in
`docs/acceptance/r2c-f-recovery-consistency.md`.

## Consequences and risks

- Ordinary directory changes and evidence gaps can regain trustworthy freshness without a normal
  full-root scan.
- Oversized recovery continues as bounded metadata-inventory pages and does not invoke the media
  scanner.
- Full scans now coordinate explicitly with the change queue; their publication cannot silently
  consume later evidence.
- A structurally unresolved source item delays authoritative publication. An isolated media decode
  failure or final-validation race preserves only the last trustworthy location evidence, remains
  explicit durable retry work, and cannot make the root appear synchronized.
- Periodic consistency scans are not part of the freshness model. Watcher-first metadata inventory
  provides the non-privileged downtime and evidence-loss recovery path.

## Replacement strategy

A future service or platform index may provide a cheaper authoritative candidate set, but it must
work without elevation and retain the same root-generation, continuity-epoch, bounded enumeration,
atomic publication, previous-snapshot preservation, and explicit full-scan allowlist. It cannot
publish removals from incomplete evidence or mutate source media.
