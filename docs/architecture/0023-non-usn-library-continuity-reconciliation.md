# ADR 0023: Reconcile library continuity without the USN journal

- Status: Accepted
- Date: 2026-08-21
- Supersedes: ADR 0022

## Context

ADR 0016 through ADR 0021 established normalized change evidence, recursive Windows observation,
a durable change queue, atomic incremental catalog publication, the desktop lifecycle, and bounded
authoritative recovery. ADR 0022 then added the Windows USN change journal as the preferred source
for changes made while Ame was not running.

The retained target-library acceptance run could not open either volume journal with Ame's normal
desktop token. Both roots returned `usn_volume_open_failed`; no journal range was read and no USN
checkpoint proved downtime continuity. The fallback converted each startup boundary into root-level
authoritative work. Large roots then exceeded the bounded 4,096-entry or 128-path recovery window
and could enter the resumable full-scan pipeline. This made ordinary startup and watcher evidence
loss capable of causing long-running media scans, elevated disk I/O, unstable freshness labels, and
little useful progress feedback.

Ame will not request elevation, show a UAC prompt, or maintain separate privileged and unprivileged
synchronization products. It also will not depend on a per-volume journal that is unavailable to
the normal application token on the target workstation. Without a journal or an always-running
service, changes made while the process is stopped cannot be reconstructed from an event stream.
Trustworthy downtime reconciliation therefore requires inspecting current filesystem metadata.

This inspection must remain distinct from a full library scan. It does not decode media, generate
previews, read source bytes, hydrate cloud placeholders, or replace the accepted catalog snapshot.
It discovers current names and metadata, then routes only changed candidates through Ame's existing
identity-aware final-state reconciliation.

## Decision drivers

- provide one synchronization model for every supported root without elevation or UAC;
- keep running-time changes near real time through the existing Windows watcher;
- detect create, modify, delete, rename, and move operations made while Ame was closed;
- never turn ordinary startup, elapsed time, watcher restart, overflow, or retry into an automatic
  full media scan;
- keep the last trustworthy catalog visible while continuity is being re-established;
- publish removals only after complete authority exists for the affected scope;
- bound memory, transactions, queue growth, cancellation latency, and background I/O;
- preserve source bytes and avoid cloud-placeholder hydration;
- expose stable product states and useful development diagnostics without notification spam;
- preserve released and prerelease catalog data through a forward migration.

## Considered options

### Elevate Ame and keep USN as the primary source

Rejected. Continuous library synchronization is a normal application capability and must not
require administrator consent at every start. A cancelled or unavailable UAC prompt would create a
second degraded product mode, while the target acceptance evidence already proved that the normal
token cannot use the journal.

### Use USN when available and metadata reconciliation otherwise

Rejected. Two continuity authorities would double lifecycle, migration, recovery, diagnostic, and
acceptance states. The privileged path would remain unverified on the target workstation and would
not remove the need for the metadata path.

### Use directory modification times only

Rejected as the sole authority. Directory modification evidence can prioritize structural work,
but an in-place file-content modification need not update its parent directory. Folder timestamps
alone cannot prove catalog freshness.

### Run a full library scan on startup or watcher evidence loss

Rejected. Full scans may inspect media and generate derived evidence for every item. Their cost and
cloud-placeholder behavior are disproportionate to continuity discovery and caused the observed
long-running recovery loop.

### Use the live watcher plus a metadata inventory

Accepted. The watcher covers changes while Ame runs. A bounded metadata inventory covers the
unobserved interval after every process start and any later interval whose watcher evidence becomes
incomplete. Both sources produce candidates for the same final-state reconciler; neither is catalog
truth by itself.

## Decision

### One non-privileged continuity model

Production uses the patched `notify` 8.2.0 Windows adapter from ADR 0017 for live recursive
observation. Ame does not initialize the ADR 0022 USN adapter, read or create journal checkpoints,
request elevation, or expose a privileged synchronization mode.

The application owns one continuity epoch per root generation. An epoch begins only after the live
watcher is healthy. It combines:

1. live watcher observations received during that epoch;
2. a metadata inventory that covers the unobserved or incomplete interval; and
3. the durable path, subtree, and root work produced by both sources.

The active catalog remains visible throughout the epoch. A root becomes synchronized only after the
metadata authority required by the epoch is complete and every retained queue row at or below its
publication boundary is terminal.

### Startup metadata inventory

Every available root starts a new metadata-inventory run after its watcher is established. The run
enumerates current relative paths and records only bounded filesystem evidence required for change
discovery:

- normalized relative path and entry kind;
- size and modification evidence;
- Windows file identity when available;
- attributes required to identify offline or recall-on-access placeholders;
- the root generation and inventory epoch that own the evidence.

The inventory must not open media content, decode an image, generate a preview, hash source bytes,
follow an untrusted reparse directory outside the root, or hydrate a placeholder. It stages derived
evidence in application storage outside source roots and compares it with the published catalog in
bounded pages.

Positive candidates may publish before the complete inventory finishes only after the ordinary
path reconciler rechecks final state. This permits additions and modifications to become visible as
they are discovered. Absence is different: removal or subtree replacement requires a complete,
uncancelled inventory of the owning scope and a final watcher-boundary check. A partial, failed,
cancelled, superseded, or over-budget page never authorizes a removal.

Watcher observations continue entering the durable queue while inventory runs. They supersede or
recheck overlapping staged evidence through the existing root-generation, affected-path, lease,
and catalog-revision guards. If the watcher reports an evidence gap during an inventory, the run
cannot close freshness. The application starts a newer continuity epoch or extends the affected
scope and discards superseded staging; it does not start a full scan.

The first implementation may visit every entry in a root. Without USN, another persistent journal,
or an always-running service, that metadata cost is required to prove changes made while Ame was
closed, including in-place modifications. Directory timestamps and retained catalog evidence may
prioritize work, but they cannot suppress the final metadata comparison required for freshness.

### Evidence loss and scalable authoritative work

A live create, modify, delete, or reliably paired rename remains path-scoped. Directory changes
remain subtree-scoped. Watcher rescan flags, incomplete rename evidence, ingress overflow, watcher
restart gaps, and startup downtime request metadata inventory for the smallest trustworthy scope.

The existing 4,096-entry and 128-path limits remain bounds for one synchronous authoritative batch;
they are no longer escalation thresholds for a full scan. Work beyond either ceiling becomes a
pageable metadata-inventory run with a durable scope and cursor. Pages may discover candidates, but
scope-wide absence publishes only after the complete run succeeds.

Repeated transport, filesystem, catalog, or inventory failure preserves the last trustworthy
catalog and durable work. It becomes a blocked root condition with a structured issue code. It does
not silently claim freshness and does not start a full scan.

### Full-scan authority

Only these authorities may start or continue a complete media scan:

- first import of a newly configured root;
- an explicit user `更新图库` request for that root; or
- resumption of a full scan that already has a durable checkpoint.

Ordinary process start, elapsed time, live file events, watcher evidence loss, watcher restart,
queue overflow, retry exhaustion, metadata-inventory size, source availability recovery, database
contention, and automatic reconciliation failure never authorize a full scan. If automatic
continuity cannot converge, Ame reports a blocked condition and leaves the explicit full-scan
decision to the user.

Every full-scan request carries a typed reason. Production rejects any reason outside the allowlist,
and deterministic tests prove that all automatic synchronization paths are unable to create one.

Create-new and resume-existing scans use separate application, persistence, and desktop-bridge
entrypoints. Resume is a fail-closed transaction: it requires the exact existing scan, active root,
root generation, owner, parameters, and running or paused state. It never inserts a root, scan,
frontier, or queue lease. A checkpoint removed by root unregistration or another terminal lifecycle
transition cannot be recreated by a stale recovery coordinator.

### Shutdown and restart

Closing the application hides the desktop window immediately. A running explicit or first-import
full scan persists its checkpoint and may resume in the next process. Watchers, metadata inventories,
path work, subtree work, and bounded root reconciliation are cancelled and must not carry in-memory
authority across the process boundary.

On the next start, Ame creates a new continuity epoch after the watcher is healthy. Old unresolved
non-scan rows are coalesced or superseded into that new authority rather than trusting work captured
before additional closed-process changes could occur. Staged inventory data from an interrupted run
is derived and may be discarded. Catalog data, user decisions, previews with valid owners, and the
last trustworthy active snapshot remain intact.

### Product state and diagnostics

The sidebar exposes only four product states:

- `已同步`;
- `正在更新图库`;
- `更新受阻`;
- `目录不可用`.

`需要核对` is not a product state. Internally retained prerelease freshness values remain readable
for bridge and migration compatibility, but presentation maps an automatically recoverable epoch to
`正在更新图库` and a failed or exhausted path to `更新受阻`.

Within one blocked condition, starting another automatic attempt does not temporarily project
`正在更新图库`. The blocked state remains until a synchronized snapshot proves convergence. This
prevents alternating labels and duplicate error history.

Normal updating, retry, and successful convergence do not create notifications. A blocked or failed
condition creates one active notification per root; cause transitions update that record in place.
Detailed counts, phase, elapsed time, source path, and stable issue code belong in the notification
detail and structured development diagnostics, not in the compact sidebar label.

Development builds expose the current root phase, including watcher startup, inventory enumeration,
inventory comparison, queue publication, retry wait, and blocked state, together with bounded counts
and elapsed time. These diagnostics do not change product policy or expose raw dependency types.

### Persistence and migration

A forward schema migration adds exact-shape metadata-inventory run and staging contracts with root
generation, epoch, scope, cursor, completion authority, and bounded cleanup. Inventory staging is
derived data and is never placed inside a source root.

Schema v19 USN checkpoint, lineage, and handoff objects remain valid migration input. Production
stops creating new USN evidence. The migration may remove ownerless derived checkpoints and terminal
lineage, but it must preserve active catalog locations, asset identity, compatible preview ownership,
user decisions, and any unresolved handoff evidence until replacement authority is durable. Legacy
`StartupCatchUp` and USN gap rows become inventory-required work without pretending that a journal
range was consumed. Malformed existing authority continues to fail closed.

## Validation gates

- controlled live create, modify, delete, rename, move, and same-path replacement reach the visible
  catalog through the watcher and durable queue without a full scan;
- application-close fixtures mutate files while Ame is stopped, then prove startup inventory finds
  additions, in-place modifications, removals, renames, directory moves, Chinese paths, long paths,
  unavailable entries, and placeholders;
- inventory race fixtures cover live changes during enumeration, event supersession, overflow,
  cancellation, process interruption, root-generation change, and final absence authority;
- positive candidates may publish early, while removals never publish from an incomplete scope;
- an oversized root or subtree continues in bounded pages and never requests a full scan;
- typed-reason fixtures prove that only first import, explicit user refresh, and checkpoint resume
  can start a full scan;
- restart fixtures prove non-scan work starts a new epoch while a full scan alone resumes;
- state fixtures prove no `需要核对` presentation and no `正在更新图库` / `更新受阻` oscillation;
- notification fixtures prove normal update, retry, and success remain silent and blocked errors
  deduplicate by root;
- development diagnostics expose phase, elapsed time, counts, and issue code without affecting the
  release UI;
- migration fixtures preserve v19 catalogs and reject malformed inventory or legacy authority;
- controlled Windows event-to-visible P95 is no greater than one second;
- the retained approximately 79,000-location workload displays the cached gallery immediately,
  performs metadata-only continuity work without reading media bytes or hydrating placeholders,
  and initially targets no more than 45 seconds per available root on the recorded workstation;
- complete Daily, Windows Release, migration, source-safety, cancellation, memory, and storage gates
  pass before the R2c integration branch is submitted to `main`.

## Consequences and risks

- Running-time behavior remains near real time and no longer depends on an administrative API.
- Every process start has O(N) metadata work when strict downtime correctness is required. This is
  an explicit consequence of rejecting USN, elevation, and an always-running service.
- Metadata inventory is cheaper and safer than a full media scan but still causes directory and
  metadata I/O. Paging, prioritization, cancellation, and measured budgets are required.
- A root may remain `正在更新图库` while metadata authority is incomplete, but the cached catalog is
  immediately usable and the phase is observable in development diagnostics.
- Repeated inability to enumerate a root produces `更新受阻`; Ame does not hide the failure behind
  repeated full scans.
- Retaining v19 migration compatibility adds temporary schema complexity. Deprecated USN objects can
  be removed only after their retained authority has been safely converted or released.

## Replacement strategy

A future always-running service or platform index may replace startup metadata inventory only if it
works without elevation and preserves watcher-first boundaries, root generation, enqueue-before-
advance ordering, final-state reconciliation, authoritative removal completeness, source safety,
and the explicit full-scan allowlist. Reintroducing USN or UAC requires a new ADR and explicit user
approval; ADR 0022 does not become active again automatically.
