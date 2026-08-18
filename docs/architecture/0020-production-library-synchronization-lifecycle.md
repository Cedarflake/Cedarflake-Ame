# ADR 0020: Run continuous library synchronization with the desktop lifecycle

- Status: Accepted
- Date: 2026-08-18

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
- stop observers and polling before window destruction, with a bounded user-visible close path;
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

The Rust application layer owns one `LibrarySynchronizationRuntime`. Its production adapter starts a
Windows observer for every currently available registered root, stops observers whose root is removed,
retired, unavailable, or reconfigured, drains normalized observations into the durable queue, and runs
only the R2c-D path worker. Subtree, root, and freshness-gap rows remain unleased for R2c-F.

Draining observer memory and committing the durable queue form an explicit handoff. The runtime retains
one drained plan in memory until enqueue succeeds and retries that plan before polling the observer
again. A database failure therefore degrades the visible source state without losing the only copy of
the observations. A cold start and every unavailable-to-available transition enqueue a root
`FreshnessUnknown` intent before the runtime can report synchronized freshness. R2c-F must complete the
authoritative recovery work before that state can clear.

The runtime publishes a bounded `LibrarySynchronizationSnapshot` containing the running flag, catalog
revision, applied-mutation count, and one status per configured root. Each root status contains the
durable root generation, availability, freshness and cause, observer health, bounded queue counts, and
an optional issue code. The bridge exposes only start, poll, and stop application calls. Adapter,
watcher, SQLite, absolute-path inspection, and queue-row types do not cross it.

Flutter starts synchronization after Rust and the initial catalog are ready. A single timer-driven
service prevents overlapping polls, retains the last trustworthy revision on a polling failure, and
projects known roots as degraded instead of replacing the catalog with an empty state. Stopping is
idempotent and waits for an active poll before calling the Rust stop use case.

The unified gallery subscribes to synchronization snapshots. It schedules at most one refresh at a
time, coalesces newer revisions, and retries briefly when another query transition owns the controller.
It requests a background refresh only after the published revision exceeds the visible revision.
Existing assets remain visible while the new bounded window and timeline load.

Refresh continuity uses stable identity:

- the visible anchor carries `asset_id`, the preferred current `location_id`, and its original global
  ordinal; the catalog preserves that physical location when it is still active, follows another
  location for the same asset after a rename, and falls back near the original ordinal only when the
  asset no longer exists;
- explicit selection stores asset IDs and rebinds to the new query revision;
- complete-query select-all is cleared when the result-set revision changes;
- an open viewer remains independent from the bounded detail window, resolves its asset directly,
  follows the preferred location across a rename, and closes only after an authoritative lookup proves
  that the asset no longer exists;
- the active source, filters, layout preferences, preview state, and logical scroll anchor remain owned
  by their existing accepted UI contracts.

Source rows render the four Chinese product states `已同步`, `正在更新图库`, `需要核对`, and
`目录不可用`. A bridge failure before the first per-root snapshot projects configured available roots
as `需要核对` rather than leaving them indefinitely in `正在更新图库`. The existing `更新图库` action
invokes the application scan use case; it does not enumerate or mutate files in Flutter.

Window management enables close prevention only so shutdown can be coordinated. Close requests share
one memoized operation, run registered shutdown actions in reverse order, wait no longer than six
seconds, and then destroy the window even if shutdown reports an error or exceeds the bound. Source
media is never modified by this lifecycle.

## Validation gates

- Rust fixtures prove observer start, path-event publication, enqueue-failure retention, cold-start and
  availability-transition continuity gaps, unavailable and removed-root handling, idempotent stop,
  retained evidence gaps, and deterministic time;
- SQLite fixtures prove root metrics isolation, preferred-location asset anchors, rename resolution,
  direct stable-asset lookup, and nearest-ordinal fallback after removal;
- Flutter service fixtures prove DTO mapping, non-overlapping polling, failure degradation, and one
  Rust stop call across repeated shutdown requests;
- gallery controller, selection, navigation, viewer, layout, and production-screen fixtures prove
  background revision refresh without blanking, rename continuity, and authoritative-removal closure;
- window fixtures prove reverse-order, idempotent coordinated shutdown and destruction after the
  configured timeout;
- bridge generation, format, Clippy with warnings denied, Dart analysis, complete Rust and Flutter
  tests, Windows controlled integration, and repository Daily pass;
- the Windows release gate proves generated bridge and packaged desktop startup compatibility.

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
- Root, subtree, overflow, watcher-gap, and audit recovery remain pending work and truthfully show
  `NeedsReconciliation` until R2c-F completes them.
- The six-second window-close bound favors a responsive exit. Durable queue state and generation guards
  make unfinished work safe to resume on the next start.

## Replacement strategy

Replace the polling bridge with a bounded push transport only if it preserves the same application-owned
snapshot, non-overlap, degradation, revision, and shutdown contracts. A replacement cannot expose raw
watcher events to Flutter, move reconciliation policy into presentation, or weaken stable-identity
refresh and durable recovery semantics.
