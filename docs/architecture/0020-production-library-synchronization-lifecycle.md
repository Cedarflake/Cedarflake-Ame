# ADR 0020: Run continuous library synchronization with the desktop lifecycle

- Status: Accepted
- Date: 2026-08-18
- Last amended: 2026-08-22

## Context

ADR 0016 through ADR 0019 define normalized filesystem evidence, the Windows change source, the
durable queue, and atomic path-level catalog publication. R2c-E must turn those isolated contracts
into the normal desktop experience without moving reconciliation policy into Flutter, introducing a
permanent task entry, or blanking the accepted unified gallery whenever the catalog revision changes.

The production lifecycle must also remain truthful when a source is unavailable or the observer is
unhealthy. Closing the desktop window must stop background observers and queue polling without
trapping the user indefinitely. Authoritative subtree, root, and evidence-gap reconciliation remains
R2c-F work and must not be approximated by a path worker.

## Decision drivers

- start one bounded synchronization lifecycle with the desktop application;
- keep filesystem observation, queue policy, and incremental publication in Rust;
- expose only Ame-owned, bounded synchronization snapshots across the desktop bridge;
- preserve the accepted gallery interaction state across an incremental catalog revision;
- identify refresh anchors and explicit selections by logical asset identity rather than location;
- present simple Chinese freshness and degraded states without claiming health early;
- keep one production process as the exclusive owner of the current user's catalog lifecycle;
- remove the desktop window immediately on close while background teardown remains bounded;
- preserve only full-scan checkpoints across shutdown and re-establish all other change evidence on
  the next start;
- leave unsupported authoritative recovery work durable for R2c-F.

## Considered options

### Watch source directories from Flutter

Rejected. Flutter would acquire filesystem and reconciliation policy, duplicate root lifecycle state,
and make desktop presentation responsible for correctness and retry semantics.

### Reload the complete gallery whenever any event arrives

Rejected. Watcher events are hints rather than catalog publications, and a full blanking reload would
discard the accepted scroll, selection, and viewer behavior for ordinary bounded changes.

### Poll one bounded Rust synchronization snapshot and refresh on published revision

Accepted. Rust owns observer instances, durable work, incremental publication, and freshness. Flutter
polls a small snapshot and refreshes only after the catalog exposes a newer revision.

## Decision

The Windows runner acquires a named operating-system mutex before console, COM, Flutter, Rust, or
catalog initialization. The mutex uses the global Windows kernel-object namespace plus the current
user SID, so one user cannot start concurrent Ame processes in separate desktop sessions while a
different Windows user remains independent. The owner retains the handle for the complete process
lifetime. A duplicate launch exits successfully before starting application work; failure to
establish the ownership boundary fails closed. Windows releases the handle when the owner exits,
including abnormal termination, so a later process can recover the durable catalog normally.

The Rust application layer owns one `LibrarySynchronizationRuntime`. Its production adapter starts a
Windows observer for every currently available registered root, stops observers whose root is removed,
retired, unavailable, or reconfigured, drains normalized observations into the durable queue, and runs
only the R2c-D path worker. Subtree, root, and freshness-gap rows remain unleased for R2c-F.

Draining observer memory and committing the durable queue form an explicit handoff. The runtime retains
one drained plan in memory until enqueue succeeds and retries that plan before polling the observer
again. A database failure therefore degrades the visible source state without losing the only copy of
the observations. Under ADR 0023, a cold start and every unavailable-to-available transition first
establish live observation, then start a new metadata-inventory continuity epoch. Inventory pages
produce bounded path or subtree candidates through the same final-state reconciler, while complete
scope authority is required before a removal may publish. Watcher evidence loss extends or replaces
that epoch instead of entering USN catch-up or starting a full scan. The runtime cannot report
synchronized freshness until the continuity epoch and its retained queue work are complete.

The runtime publishes a bounded `LibrarySynchronizationSnapshot` containing the running flag, catalog
revision, applied-mutation count, and one status per configured root. Each root status contains the
durable root generation, availability, freshness and cause, observer health, bounded queue counts, and
an optional issue code. The bridge exposes only start, poll, and stop application calls. Adapter,
watcher, SQLite, absolute-path inspection, and queue-row types do not cross it.
The runtime retains the last observer issue code while the root still has unresolved authoritative
work. A successful watcher restart does not clear that diagnostic; only a root snapshot that is
actually `Synchronized` clears it.

Flutter starts synchronization after Rust and the initial catalog are ready. A single timer-driven
service prevents overlapping polls, retains the last trustworthy revision on a polling failure, and
projects known roots as degraded instead of replacing the catalog with an empty state. Stopping is
idempotent and waits for an active poll before calling the Rust stop use case.

The unified gallery subscribes to synchronization snapshots. It schedules at most one refresh at a
time, coalesces newer revisions, and retries briefly when another query transition owns the controller.
It requests a background refresh only after the published revision exceeds the visible revision.
The screen subscribes before sampling the service's current snapshot and queues that current revision,
so a delta published during application startup cannot be lost merely because it preceded widget
construction. Existing assets remain visible while the new bounded window and timeline load. The
controller reports whether a refresh was applied, temporarily busy, superseded, or failed. Only busy
or superseded work receives the short automatic retry. A catalog or bridge failure stops that retry,
keeps the target revision pending, and presents one localized retry surface until a later snapshot or
an explicit user retry starts another attempt. Revisions observed during an active attempt retain the
maximum pending value; if that attempt fails, the new snapshot starts exactly one coalesced follow-up
attempt without converting the original permanent failure into a timer loop. An active, paused,
cancelling, failed, cancelled, or completed scan surface retains priority over synchronization refresh
failure so its progress, controls, and acknowledgement cannot be hidden. Failed and cancelled scan
feedback retains its scan retry action and also provides an explicit acknowledgement that clears only
the transient task feedback. Failures before a scan identifier is allocated provide the same working
acknowledgement rather than a dead action. The synchronization failure remains pending and is shown
after the task feedback is dismissed.

Refresh continuity uses stable identity:

- the visible anchor carries `asset_id`, the preferred current `location_id`, and its original global
  ordinal; the catalog preserves that physical location when it is still active, follows another
  location for the same asset after a rename, and falls back near the original ordinal only when the
  asset no longer exists;
- explicit selection stores asset IDs and rebinds to the new query revision;
- complete-query select-all is cleared when the result-set revision changes;
- an open viewer remains independent from the bounded detail window, resolves its asset directly,
  follows the preferred location across a rename, and closes only after an authoritative lookup proves
  that the asset no longer exists; a delayed lookup is guarded by both the requested asset and location
  so it cannot overwrite newer same-asset navigation;
- the active source, filters, layout preferences, preview state, and logical scroll anchor remain owned
  by their existing accepted UI contracts.

Source rows render the four Chinese product states `已同步`, `正在更新图库`, `更新受阻`, and
`目录不可用`. `需要核对` is not a product state. A bridge failure before the first per-root snapshot
projects configured available roots as `更新受阻` rather than leaving them indefinitely in
`正在更新图库`. The existing `更新图库` action explicitly invokes the application-owned full scan for
that root; it is not an incremental watcher or metadata-inventory action, and Flutter does not
enumerate or mutate files itself.
`正在更新图库` includes recoverable evidence gaps while the healthy observer and durable recovery
pipeline remain able to converge automatically. `更新受阻` is reserved for a failed observer,
exhausted durable work, bridge failure, or another condition whose automatic recovery path is no
longer healthy; it must not be used merely because authoritative work is queued.

The row label remains deliberately compact. Normal updating, automatic recovery, retry wait, and
successful convergence do not publish notifications. A notification is created only after the
condition becomes `NeedsReconciliation`, a bridge/catalog refresh fails, or another error requires
user awareness or action. Active errors are keyed by root and update in place across cause, health,
or issue-code changes instead of producing alternating history records. Starting an automatic retry
does not resolve that active error or change the row back to `正在更新图库`; only a synchronized root
snapshot proves convergence and resolves it. Notification details retain
the bounded affected-work counts, source display path, stable technical code, and a connected retry
action only when the failed application operation can actually be replayed. Root recovery remains
automatic and never exposes an action that starts a manual full scan. Successful automatic recovery resolves the active error
without adding a success notice, and acknowledgement never changes the Rust-owned freshness state.
Scan-task state remains separate and retains presentation priority.

SQLite busy or locked results caused by another bounded Ame publication are treated as transient
writer contention for 30 seconds. During that interval Flutter retains the prior trustworthy
synchronization snapshot, publishes no notification, and retries through the existing non-overlapping
poll. Background metadata inventory or authoritative recovery applies the same per-root grace while
retaining its durable work and bounded retry schedule. Contention that outlives the bound becomes one sticky
catalog failure and remains visible until a synchronized snapshot proves recovery. Debug builds log each slow call and failure code plus one
bounded per-root state heartbeat without exposing this diagnostic vocabulary in the ordinary UI.
Catalog operations that read state before mutating it acquire an immediate SQLite write transaction
before the first read. This makes the configured busy timeout serialize with an existing writer
instead of allowing a deferred WAL snapshot to fail immediately during its later write upgrade.
The path queue performs a read-only readiness probe first and does not open a write transaction when
there is no due work or retry maintenance. Full-scan directory claims, staging, checkpoints,
publication, queue leases, completion, retry, and cleanup use the same writer-before-read boundary.

Window management enables close prevention only so shutdown can be coordinated. Close requests share
one memoized operation and hide the window before waiting for background work, so a slow scanner never
looks like a frozen or flashing close. Registered shutdown actions then run in reverse order for no
longer than six seconds, after which the hidden window is destroyed even if teardown reports an error
or exceeds the bound.

Shutdown preserves different work according to its authority. A running foreground or authoritative
full scan records its traversal checkpoint and remains recoverable; the next process resumes it.
Watcher instances, metadata inventories, path reconciliation, subtree reconciliation, and bounded
root recovery are cancelled instead of continuing in-memory authority across the process boundary.
The next process first establishes a new watcher boundary and starts a new metadata-inventory epoch.
Old unresolved non-scan rows are coalesced or superseded into that new authority; only a full scan
resumes its prior checkpoint. Source media is never modified by this lifecycle.

## Validation gates

- Rust fixtures prove observer start, path-event publication, enqueue-failure retention, cold-start and
  availability-transition continuity gaps, unavailable and removed-root handling, idempotent stop,
  retained evidence gaps, and deterministic time;
- SQLite fixtures prove root metrics isolation, preferred-location asset anchors, rename resolution,
  direct stable-asset lookup, nearest-ordinal fallback after removal, read-only empty queue polling,
  and deterministic waiting when a second connection already owns the writer boundary;
- Flutter service fixtures prove DTO mapping, non-overlapping polling, failure degradation, and one
  Rust stop call across repeated shutdown requests;
- gallery controller, selection, navigation, viewer, layout, and production-screen fixtures prove
  background revision refresh without blanking, maximum-revision coalescing across an in-flight
  failure, scan-task priority, bounded failure handling, rename continuity, and authoritative-removal
  closure;
- notification fixtures prove bounded history and queue state, active-condition deduplication,
  unread icon switching without a numeric badge, detailed reconciliation evidence, application-use-
  case action routing, task-surface priority, and transient dismissal;
- window fixtures prove immediate hide, reverse-order idempotent coordinated shutdown, continuation
  through an individual teardown failure, and destruction after the configured timeout;
- scan lifecycle fixtures prove the desktop shutdown action reaches the active foreground scanner,
  waits for its Rust stream to close after a durable checkpoint, leaves the scan recoverable for the
  next process, and cannot override an explicit user pause or cancellation; non-scan recovery
  restarts only after a new watcher and metadata-inventory continuity epoch;
- the packaged Windows gate proves duplicate same-user processes cannot cross the application
  initialization boundary and that a replacement process starts after the owner exits;
- bridge generation, format, Clippy with warnings denied, Dart analysis, complete Rust and Flutter
  tests, Windows controlled integration, and repository Daily pass;
- the Windows release gate proves generated bridge, packaged desktop startup, and process ownership
  compatibility.

## Validation evidence

The exact controlled commands and results are recorded in
`docs/acceptance/r2c-e-production-ui-lifecycle.md`. No real-library root or cloud placeholder is used
by the R2c-E validation.

## Consequences and risks

- Normal desktop use now observes and publishes supported path changes without manual re-import.
- A published catalog revision updates the gallery without treating raw watcher events as catalog
  truth.
- Flutter uses bounded polling rather than receiving unbounded filesystem event streams. The polling
  interval adds a small visibility delay but makes overlap, shutdown, and degradation explicit.
- A runtime start failure cannot mutate catalog state; the last trustworthy catalog remains visible.
- Root, subtree, overflow, and watcher-gap recovery remain pending work and truthfully show
  `NeedsReconciliation` until R2c-F completes them.
- The window disappears immediately on close. The six-second hidden teardown bound prevents resource
  leakage while a new metadata-inventory epoch, full-scan checkpoints, and generation guards make
  unfinished work safe on the next start.

## Replacement strategy

Replace the polling bridge with a bounded push transport only if it preserves the same application-owned
snapshot, non-overlap, degradation, revision, and shutdown contracts. A replacement cannot expose raw
watcher events to Flutter, move reconciliation policy into presentation, or weaken stable-identity
refresh and durable recovery semantics.
