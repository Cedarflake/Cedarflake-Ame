# ADR 0021: Recover library freshness through bounded authoritative reconciliation

- Status: Accepted
- Date: 2026-08-18

## Context

ADR 0016 through ADR 0020 establish normalized change evidence, the Windows observer, a durable
leased queue, atomic path-level publication, and the production desktop lifecycle. Path work can
normally converge without scanning a whole root. Directory changes, watcher evidence gaps, queue
overflow, cold start, source recovery, and consistency audits still require stronger authority than
one path inspection can provide.

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
- schedule a low-frequency consistency audit without claiming freshness before completion.

## Considered options

### Treat every gap as independent path work

Rejected. A missing rename half, watcher overflow, or directory-level notification does not provide
an authoritative finite path set. Path-only work could leave stale descendants or publish false
absence.

### Run a complete root scan for every non-path intent

Rejected. This is safe but unnecessarily expensive for bounded directory changes and creates poor
behavior for large libraries.

### Reconcile bounded scopes and escalate atomically when the scope exceeds limits

Accepted. A bounded authoritative worker handles ordinary subtree and small-root recovery. It
defers the durable intent and requests a resumable full scan before publishing when either the
enumerated source set or the active catalog set exceeds the accepted bounds.

## Decision

The application owns an authoritative recovery worker separate from the path worker. Production
runs this worker on a cancellable background thread outside the desktop polling mutex and leases one
subtree, root, or freshness-gap intent only when the root generation is current, a trustworthy
catalog is published, no full scan is active, and a healthy observer has established the current
live-notification boundary. A failed or restarting observer therefore leaves its continuity gap
durable until the restarted source is healthy. The worker enumerates through the filesystem adapter
with an absolute ceiling of 4,096 directory entries and 128 affected paths. The default audit
interval is seven days. Policies cannot raise these ceilings or disable the interval bound.

The worker combines the final filesystem paths with every currently published location in the
affected subtree. It prepares additions, modifications, identity-preserving moves, replacements,
and authoritative absences through the same ADR 0007 reconciliation path as R2c-D, revalidates the
complete set, and publishes all mutations plus queue completion in one catalog-delta transaction.
Unreadable entries, containment failures, placeholders, database failures, or source races retry
the durable intent without publishing a partial removal set. A cloud placeholder is unresolved
whether or not a prior catalog location exists; recovery neither opens it nor records a successful
audit for that scope.

When the bounded set is exceeded, the worker restores the lease to pending and records one full-scan
request. Production starts bounded work only for an authoritative queue row that is currently due;
future and exhausted retry rows remain projected without creating an empty worker every poll.
Production runs at most one recovery scan at a time on a background thread. Failures use a per-root
exponential retry from one second to five minutes; a bounded re-escalation preserves that failure
history until recovery succeeds, and another root is not delayed by that state.
Shutdown requests cancellation and keeps the desktop close path bounded. If that bound expires,
the runtime retains the worker handle in an explicit stopping state and rejects restart until a
later join proves the old worker ended.

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
of frozen queue rows, foreground-versus-authoritative lifecycle ownership, and last successful
consistency-audit time. It also gives v18 an explicit contract marker and validates the single-scan
ownership index. An early prerelease v18 database with no conflicting active scans receives the
missing index and lifecycle owner atomically; its reserved `sync-recovery-` IDs identify draft
authoritative rows, while ambiguous overlapping ownership fails closed. Migration from v17
normalizes catalog and scan relative paths to slash-separated form and invalidates pre-v18 running
or paused scans whose queue authority cannot be reconstructed. Because a v17 location identifier
was derived from its historical path spelling, every rescan resolves the active location by root
plus normalized relative path and retains that location identifier before considering file
identity or state changes.

The full-scan pipeline preserves the active catalog whenever any entry in a previously published
root is unreadable, a placeholder, or otherwise uninspectable, including a newly appeared path with
no prior location. This completeness gap is persisted in the scan checkpoint and defensively
blocks publication, queue completion, and audit advancement after restart. A limited replacement
scan likewise remains stale instead of replacing a previously published root with a partial
snapshot.

After a successful authoritative root reconciliation or full scan, SQLite records the audit time in
the same publication transaction. The runtime enqueues the next root audit only when the source is
available, the watcher is healthy, the queue is otherwise idle, no full scan is pending, and the
bounded interval has elapsed. The root remains updating or needs reconciliation until that work
actually publishes.

## Validation gates

- controlled temporary fixtures prove bounded subtree addition and removal, directory rename
  identity continuity, and escalation before an oversized scope publishes;
- queue fixtures prove generation and high-watermark capture, preservation of later evidence, and
  selective release after scan abandonment;
- scan fixtures prove corrupt rescans and limited replacement scans preserve the last trustworthy
  catalog, including a new full-scan placeholder and restart-safe checkpoint rejection;
- migration fixtures prove v17 to v18 path normalization, preservation of healthy and placeholder
  legacy identifiers, invalidation of unverifiable running scans, and repair or fail-closed handling
  of a prerelease v18 database;
- runtime fixtures prove cold-start recovery, degraded-source restart continuity, background-only
  authoritative work, foreground-versus-authoritative recovery isolation, bounded multi-root
  rotation, managed stop timeout, due-only scheduling, delayed audit completion, and bounded
  per-root retry;
- complete format, Clippy, Rust, Flutter, Windows integration, bridge, Daily, and Windows release
  gates pass before the slice is merged;
- no real-library root is accessed by this implementation validation.

## Validation evidence

The controlled commands and results are recorded in
`docs/acceptance/r2c-f-recovery-consistency.md`.

## Consequences and risks

- Ordinary directory changes and evidence gaps can regain trustworthy freshness without a normal
  full-root scan.
- Oversized recovery reuses the existing resumable scanner and remains durable across restart.
- Full scans now coordinate explicitly with the change queue; their publication cannot silently
  consume later evidence.
- A source item that cannot be inspected may delay freshness, but it cannot cause the accepted
  catalog to disappear or reuse incompatible derived evidence.
- The consistency audit is intentionally low frequency. Conditional downtime journal catch-up and
  large-library latency evidence remain R2c-G and R2c-H concerns.

## Replacement strategy

A future journal or snapshot adapter may provide a cheaper authoritative candidate set, but it must
retain the same root-generation, queue-watermark, bounded enumeration, atomic publication,
previous-snapshot preservation, and explicit fallback contracts. It cannot publish removals from
incomplete evidence or mutate source media.
