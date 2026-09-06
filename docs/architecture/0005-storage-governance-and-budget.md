# ADR 0005: Govern storage and the bounded preview lifecycle

- Status: Accepted
- Date: 2026-08-07
- Last updated: 2026-09-06
- Related: ADR 0006, ADR 0014

## Context

Ame cannot assume that the system drive has enough space for previews derived from a roughly
259 GB image library. The user needs configurable catalog and preview locations without placing
application data inside source trees or requiring a second full copy of the collection.

The original version of this decision established safe external storage, restart-bound
configuration, and an admission-only preview budget. That implementation prevents unbounded growth
but stops producing new previews when the budget is full. It does not complete the product contract
for automatic cleanup, manual cleanup, bounded preview sizes, restart recovery, or preview-root
transition.

Gallery geometry and preview storage also have different lifecycles. Orientation-corrected width
and height are durable catalog evidence needed to reconstruct the photo wall after restart. Preview
files are rebuildable artifacts that may be regenerated, invalidated, or deleted. Cache operations
must never erase dimensions or move the viewport.

## Decision

### Storage control plane

- A small SQLite settings database remains at the operating-system configuration location. It
  stores the configured catalog file, versioned preview-cache root, and preview byte budget.
- Active storage is resolved once per process. Saving a changed location or budget does not change
  paths used by the running process; the UI reports that a restart is required.
- Ame creates target storage directories only after validating the complete update. Catalog and
  preview paths that overlap an imported source root are rejected.
- Catalog relocation remains rejected after the active catalog contains a library root. A future
  catalog-migration workflow must copy, verify, switch, roll back, report progress, support
  cancellation, and recover from interruption before removing that restriction.
- Before any first-root registration, configured and process-active catalog paths must still
  identify the same location. A pending catalog switch requires restart before another import;
  otherwise the import would be committed to the database that the next process stops opening.
  Configuration save and scan registration share one bounded process-local admission. The internal
  registry mutex is released before filesystem or SQLite work, and the permit ends at registration
  or settings commit rather than spanning a scan. Whichever command commits first determines the
  other command's existing restart-required or migration-required rejection. Preview-only and
  budget-only changes do not create this catalog conflict, nor do physically equivalent paths.
- The supported preview budget is 64 MiB through 1 TiB. The initial default is 4 GiB.

### Durable image geometry

Orientation-corrected width and height remain durable catalog columns and the authoritative source
for aspect ratio after restart. They are associated with compatible source state and media-
inspection engine identity. ADR 0006 owns orientation interpretation; ADR 0014 owns manifest and
layout use of the resulting dimensions.

Preview absence, pending work, failure, retry, regeneration, cleanup, eviction, relocation, or a
missing cache file never clears or rewrites durable dimensions. A temporarily unavailable source
retains its last trustworthy dimensions with separate availability evidence. Confirmed content
edits, same-path replacements, incompatible inspection contracts, and authoritative removals use
normal catalog reconciliation and one atomic revision. Unknown dimensions use ADR 0014's stable
fallback until a complete newer revision or compatible bounded geometry-evidence epoch supplies
trustworthy evidence.

### Preview artifact identity and variants

Ame owns a preview-artifact index separate from gallery layout and durable user data. Its current
v3 representation records enough evidence to identify and account for an artifact:

- every compatible active location reference plus compatible file identity, source revision, and
  catalog-wide source generation;
- preview algorithm and version;
- orientation contract;
- one bounded physical-pixel size bucket and actual encoded dimensions;
- artifact path, byte size, and rebuildable lifecycle state;
- coarsened last-use evidence that does not require a persistent write on every scroll tick.

An absolute path alone is not artifact identity. Preview artifacts use a finite measured bucket set,
not a distinct key for every logical tile width. Ame selects the smallest compatible bucket that
satisfies the requested physical display size and scale. A larger request may generate a larger
bucket; redundant or superseded variants become reclaimable. Concrete bucket values require Profile
evidence covering quality, decode latency, storage, and resize churn.

The application distinguishes absent, pending, generating, ready, failed, stale, and evictable
conditions. Persistence may combine states only when no behavior or recovery evidence is lost.
Failures retain structured evidence and support explicit retry. A ready index entry whose file is
missing returns to pending demand instead of becoming a permanent gallery failure.

### Demand, generation, and publication

ADR 0014 owns demand priority: viewer, visible, movement-direction-near, guard, then optional idle
warming. Queue and decode concurrency remain bounded. High-velocity movement may defer expensive
generation without deferring final layout geometry.

The user-facing preview loading preference exposes only `small`, `medium`, and `large` resource
policies. The queue maps them to hard concurrency limits of one, two, and three; `medium` remains the
default and preserves the accepted two-request baseline. Viewer, visible, movement, guard, and idle
priorities determine which pending request starts next but never create an overflow slot. Obsolete
active work remains harmless through publication guards and continues to occupy its existing slot
until it finishes. Lowering the preference does not cancel active decodes; it prevents new work from
starting until the active count falls below the new limit. Raw worker counts and queue depth remain
internal.

Preview and bounded detail publication share one process-owned validated catalog-session contract.
The session retains only immutable catalog identity, schema, and write-admission evidence; each
operation still opens its own short-lived SQLite connection and no connection, transaction, or
admission permit is held across media decode. Repeated preview requests and detail-window
completion must not rerun migrations or the complete schema contract. A database identity or
schema-cookie change invalidates the session, performs one serialized full validation outside the
media operation, and retries the open once; any second stale result fails closed. Validation is
single-flight per catalog path: concurrent callers share the same success or structured failure,
validation for one path never holds the registry lock while doing SQLite work, and a failed result
uses only a short bounded backoff before one caller may retry. Storage-path activation invalidates
the owner rather than reusing evidence for another catalog.

An explicit preview retry has an observable request lifetime. The tile enters a retrying state in
the frame that accepts the action, prevents duplicate activation, and leaves that state on every
terminal outcome: ready, failed, cancelled, superseded, context invalidation, or disposal. Queue
deduplication attaches the caller to the existing compatible request rather than manufacturing a
second decode. Structured diagnostics distinguish Dart queue wait from active execution. Rust
diagnostics separate access and store setup, catalog work, source revalidation, materialization,
artifact commit, reclamation, and catalog publication; the materialization outcome identifies a
cache hit, generation, or failure without printing a complete source path.

Each request and publication carries exact root, active scan, location, file identity, source
revision, catalog-wide source generation, algorithm, orientation, and size-bucket identity. The
global catalog revision still protects gallery queries, but it is deliberately not a preview lease:
an unrelated catalog write must not invalidate otherwise exact preview work. A schema-v31 location
whose revision is NULL may adopt the revision observed from the requested source exactly once under
a conditional SQLite update; it is never treated as already unchanged.

Generation reads through one already-open source handle, validates it before decode, and validates
the same handle again after decode. Publication then holds the restrictive final source guard while
installing the artifact and committing the exact SQLite lease. A source-state change, replaced
handle, inactive scan, mismatched root or location, generation change, or revision change makes the
work superseded. Superseded work removes only its staged output and never writes a `failed` preview
state. No preview operation recalls an offline placeholder.

The current cache namespace is `ame-jpeg-thumbnail-v3-source-revision`; its key includes both source
revision and source generation. An explicit `ForceRegenerate` request bypasses compatible-cache
reuse both before decode and at the post-decode race check. On Windows, replacing an existing v3
target uses the `preview_cache/installation.rs` owner with same-directory staging and an
exclusively created, managed temporary backup passed to `ReplaceFileW` with zero flags. Windows documents
`REPLACEFILE_WRITE_THROUGH` as unsupported, and this rebuildable cache does not claim power-loss
write-through durability. Installing a missing target or restoring the backup uses `MoveFileExW`
with zero flags: the same-volume operation cannot overwrite a target that appeared concurrently.
The target is never deleted first. With the explicit backup, Windows error 1176 retains the old
target; error 1177 may move it to the owned backup. A failed restoration reports
`preview_replace_restore_failed` and retains that recoverable managed backup instead of promising
that every terminal error restores the old pathname. Staged bytes are discarded separately. Old
bytes are released from accounting only after physical deletion; a failed post-install backup
cleanup leaves those bytes counted until managed-cache recovery or reclamation removes them.
No path other than an exclusively claimed backup is passed as the replacement backup or deleted
by this owner. The narrow unsafe Windows boundary keeps all NUL-terminated UTF-16 buffers alive
through synchronous calls and never retains their pointers. It resolves the existing managed-cache
parent with Rust's filesystem adapter before joining the unchanged leaf name, so native installation
and restoration receive extended-length paths even when the target does not yet exist. Embedded
NULs are rejected. This does not change cache keys, confer source access, or claim protection against
replacement of a cache parent. Correctness cannot depend on the host executable's long-path opt-in.
A missing target after failed installation preserves the original installation error rather than
replacing it with the subsequent metadata error. Only successful physical installation
permits exact SQLite lease publication.

The platform contract is verified against Microsoft's
[ReplaceFileW failure states](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew),
[MoveFileExW flags](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw),
and [File.Replace backup overwrite contract](https://learn.microsoft.com/en-us/dotnet/api/system.io.file.replace).
Controlled tests inject 1176/1177, failed restoration, retained-backup cleanup failure, and existing
backup/target collisions without touching source media.

The artifact index keeps location ownership as a many-to-many reference instead of a single mutable
owner column. Identity-proven hard links, renamed locations, and other compatible active locations
may reference one rebuildable artifact. Reclamation excludes an artifact when any reference is in
protected demand, and removal resets every active location that still names the artifact. Schema
v15 migrates v14 rows by rebuilding those references from every catalog location that publishes
the same artifact path; it does not infer ownership from the v14 row's last writer alone. Normal
scan publication then keeps the reference set aligned with the active snapshot. v16 transactionally
removes historical references that cannot be proven by a ready preview in the current active scan,
while preserving every valid shared reference and marking newly unreferenced artifacts stale.

Root unregistration detaches every reference owned by the removed root before deleting its
locations. Successful replacement-scan publication detaches locations that leave the active
snapshot and then rebuilds references for retained ready previews. After either transition, a ready
artifact with no remaining location references becomes stale for priority reclamation. Abandoning
an unpublished scan does not detach by location identity because its staged row may share that
identity with the still-authoritative active snapshot.

A cache hit and a newly generated artifact have different publication lifecycles. A compatible
cache hit remains installed when the current source revalidation fails. A new encoding stays in a
managed temporary namespace, including its reserved budget bytes, until source revalidation
succeeds; only then is it atomically renamed into the final namespace. Failed validation removes
only the staged file and cannot delete an artifact already used by another location.

When a compatible current artifact already exists but the catalog has no durable dimensions, Ame
reads bounded source header and orientation evidence to recover the display dimensions without
decoding the full source raster or rewriting the artifact. This cache-hit inspection remains
source-read-only, applies the same dimension and orientation limits as generation, and still enters
ADR 0014's separate geometry-evidence epoch rather than making artifact readiness a layout event.

When generation is also the first successful inspection of an item whose durable dimensions were
unknown, its orientation-corrected width and height are persisted as geometry evidence. ADR 0014
permits that evidence to enter one settled, identity-checked layout epoch. The artifact becoming
ready is not itself the geometry trigger, and items with already-known dimensions never reflow due
to preview lifecycle changes.

High-resolution JPEG preview generation uses `jpeg-decoder` 0.3.2 with default features disabled
behind a private Ame adapter. The crate is dual MIT or Apache-2.0 licensed, pure Rust, supports a
bounded output buffer, and can reduce JPEG inverse-DCT output by 1/8, 1/4, or 1/2 before the final
bounded resize. This avoids allocating and resizing the complete source raster for the ordinary RGB
and grayscale JPEG path. The adapter retains the orientation contract and original display
dimensions. Successfully decoded color modes outside the scaled RGB/grayscale path may use the
existing `image` adapter for conversion. This uncommon path validates the bounded raster before
releasing it and invoking conversion. Header, allocation, or pixel decoder errors are structured
failures, not fallback signals: the pinned generic JPEG decoder tolerates missing entropy by
filling pixels, which cannot establish a valid preview. Other admitted formats retain their
bounded generic decoder path.

The ordinary media suite checks all seven source formats, wrong extensions, empty/forged content,
and deterministic header/pixel damage through staging, commit, reuse, and failure cleanup. The
separate synthetic format workload measures real cold generation and warm reuse; it does not
measure SQLite scheduling, UI latency, or retained-library performance. Current JPEG artifacts
are static RGB posters: transparent-input and first-frame GIF fixtures characterize that behavior,
not alpha-preserving output or validation of every animation frame.

This is a narrow admission rather than a new media authority. `jpeg-decoder` is in maintenance mode,
so its types and errors do not cross the preview-store boundary, its optional Rayon feature remains
disabled, and concurrency remains owned by Ame's bounded presentation queue rather than the decoder.
Keeping the current `image` full decode was rejected for large ordinary JPEG previews because it always
materializes the full source raster. ImageMagick or libjpeg-turbo process adapters were not selected
for this slice because they add a native runtime, packaging, cancellation, and crash-isolation
boundary. Windows Imaging Component was not selected because it would make the first optimization
Windows-specific and introduce a new unsafe platform boundary.

Legacy `ame-jpeg-thumbnail-v1` and `ame-jpeg-thumbnail-v2-orientation-*` artifacts lack the complete
schema-v31 revision-and-generation proof. Inventory recognizes them only as Ame-managed,
rebuildable bytes for accounting, bounded recovery, reclamation, and explicit cleanup. They are
never reused, promoted, or published as v3 evidence. This prevents a weak legacy key from restoring
pixels after an observed same-path replacement while still excluding unrelated files from cleanup.
Startup does not read source content or scan source roots to repair legacy entries; current preview
demand regenerates v3 evidence through the normal bounded path.

### Budget and automatic reclamation

The preview store counts managed artifacts at startup and reserves capacity atomically before
publication. Capacity uses a high watermark and lower reclamation target. Reclamation runs as
bounded background work outside pointer, scroll, layout, and paint critical paths, in this order:

1. interrupted temporary files and unreferenced managed artifacts;
2. obsolete algorithm or orientation-contract artifacts;
3. incompatible and superseded size variants;
4. least-recently-used artifacts outside current demand.

The active viewer item, visible demand, movement-direction guard demand, and in-flight atomic
publication are pinned during automatic reclamation. If pinned or non-reclaimable artifacts still
prevent a reservation, the request returns a structured isolated capacity failure instead of
growing without bound or deleting source media. High and low watermarks must prevent repeated
deletion and regeneration at the capacity boundary.

### Manual cleanup and restart recovery

Manual cleanup is an explicit foreground operation with progress, cancellation, completion, and
failure states. It removes managed preview artifacts and resets compatible preview entries to
pending. It preserves dimensions, metadata, source configuration, user decisions, operation
history, row membership, item rectangles, total extent, and logical scroll anchor. Visible previews
then regenerate through normal demand priority.

Startup performs bounded reconciliation of accounted bytes, interrupted temporary files, missing
ready files, and unreferenced managed artifacts. It does not scan source roots, hydrate placeholders,
or block opening the last trustworthy catalog on a complete cache walk. Work beyond the startup
allowance continues as observable bounded background maintenance.

Startup artifact recovery and ordinary catalog-window reads share the application-owned
`preview_health` boundary. A missing-file or byte-size observation is only a candidate hint, not
permission to invalidate an artifact. The final filesystem observation and conditional catalog
update hold the same exclusion used by preview publication. Otherwise a same-key forced
regeneration could publish between an old negative observation and its database write. The
SQLite adapter attempts both write admission and its short transaction without waiting; contention
returns typed deferred work and releases preview exclusion. It must not hold that exclusion while
waiting for another database writer. Location updates still require the exact active source lease;
artifact updates require current key/path ownership. Neither path clears dimensions, interprets
access denial as a missing file, nor turns deferred work into a successful repair.

### Catalog database page reclamation

Configured-root removal owns logical catalog cleanup; SQLite page reclamation is a separate derived-
storage maintenance operation. A successful removal commits and returns to presentation before
reclamation is scheduled. Reclamation never deletes rows, changes catalog revision, weakens shared-
asset or preview ownership, removes queue or operation history, or touches source media. Its only
authority is to compact pages that SQLite already reports on the freelist.

Fresh catalogs set `auto_vacuum=INCREMENTAL` before creating any table. Existing catalogs in
`auto_vacuum=NONE` are not synchronously vacuumed by schema migration or startup. When reclaimable
pages are at least 64 MiB and at least 25 percent of the main database, Ame schedules one serialized
background operation. It first checks caller-available capacity using the conservative SQLite
upper bound of twice the current main-database bytes plus a 64 MiB explicit safety margin; every
multiplication and addition is overflow checked. Insufficient capacity is a retryable structured
failure and does not change the already committed removal.

The one-time `NONE` to `INCREMENTAL` conversion uses an independent connection with zero busy
timeout, sets the pragma before ordinary `VACUUM`, and runs only after acquiring a recovery-priority
maintenance permit. Failure to acquire an idle SQLite window backs off without blocking the
removal caller. After preemption, a root-removal writer waits at most five seconds for the
maintenance permit to exit and otherwise returns `catalog_user_interactive_write_timeout` as an
explicit retryable failure; the timed-out priority waiter is removed so lower-priority work cannot
remain starved. Ordinary `VACUUM` supplies SQLite's transaction and crash consistency; Ame does not
perform an unproven copy-and-replace sequence. A progress handler and interrupt handle make user
cancellation and higher-priority root removal observable. Conversion progress is indeterminate
because SQLite virtual-machine instruction counts do not provide a trustworthy byte denominator.
Every completed, interrupted, or failed write-maintenance attempt invalidates the process-owned
catalog-session metadata for that path. The next catalog request must reopen and fully validate the
database instead of relying on prepared state from before `VACUUM`; the session cache owns no live
SQLite connection across the maintenance window.

An existing `FULL` catalog switches online to `INCREMENTAL` through SQLite's supported pragma
transition and does not require the temporary capacity or full rewrite used by `NONE`. This mode
transition is completed before the task may report success, even when the freelist is already below
the reclamation threshold. Busy or preempted maintenance attempts remain `WaitingForIdle` with a
25 ms exponential delay capped at 3.2 seconds; transient contention never becomes a permanent
failure, and cancellation during any delay publishes `Cancelled`.

Once incremental mode is established, each `incremental_vacuum(256)` call reopens a maintenance
connection and reacquires the permit. The connection and permit are released between batches so
Live, Journal, foreground scan, and user-interactive work can run. Progress is calculated only from
successive `page_count`, `freelist_count`, and `page_size` evidence. Reclamation stops when the
freelist reaches 8 MiB or five percent, avoiding cleanup thrash while retaining reusable pages.
Interrupted conversion leaves the pre-conversion database transactionally usable; interrupted
incremental work retains already committed batches. Startup may reschedule remaining eligible work.
The reclamation operation registry owns scheduling, the current execution token, coalesced next
request, attempt control, and terminal publication under one per-catalog transition boundary.
Accepting a new request and retiring a worker cannot leave queued work without an executor.
Every execution has independent cancellation identity. A late cancel, preempt, or completion cannot
write into a later execution or turn a terminal snapshot back into active work. Preemption targets
only the captured attempt; the interrupted worker publishes waiting status. Registry locks are
released before filesystem work, SQLite operations, thread creation, or interrupt callbacks.
Before publishing `Completed`, a separate zero-busy-timeout maintenance attempt runs
`wal_checkpoint(TRUNCATE)` under the same preemptible admission. Ame parses all three SQLite result
columns; a busy result or SQLite busy error returns to capped waiting, while a catalog without a WAL
passes normally. Only a successful checkpoint can complete the operation, so retained WAL bytes are
not reported as reclaimed while an older reader still prevents truncation.

Storage status reports total catalog files separately from estimated live main-database bytes and
reclaimable freelist bytes. WAL and shared-memory sidecars remain part of total on-disk usage but
are never mislabeled as freelist. The Windows capacity adapter contains one isolated
`GetDiskFreeSpaceExW` call. Its unsafe boundary accepts a live NUL-terminated UTF-16 directory
buffer and one valid writable `u64`; all other output pointers are null and no pointer escapes the
call.

### Preview-root transition

Preview relocation uses a switch-and-regenerate workflow because previews are rebuildable:

1. validate and persist a pending target outside every source root;
2. keep the old root active until restart and successful target initialization;
3. atomically reset old-root locations, retained handoffs, and artifact ownership in the catalog
   without changing durable dimensions;
4. only after that commit, retire pending ownership in the settings database and activate the target;
5. regenerate in the target only through normal demand and retain explicit ownership of the old
   root until the user starts verified cleanup.

The settings and catalog databases do not share one transaction. Pending ownership is the durable
restart obligation: interruption before or after catalog reset leaves that obligation available,
and the next activation repeats the idempotent reset before consuming it. Activation failure can
fall back only to a previous root freshly verified outside all current source roots. A former
cache path that now overlaps a source is not initialized. A reset already committed before a
settings failure is harmless because the old cache remains rebuildable through normal demand.
Ame never silently deletes the old root or calls its space reclaimed. A future copy-based preview migration is optional and requires integrity
verification, atomic activation, cancellation, progress, interrupted-run recovery, and rollback.

## Consequences and risks

- Preview storage can recover from exhaustion instead of permanently refusing all new artifacts.
- Clearing previews or changing cache roots cannot change gallery geometry.
- Multiple bounded variants improve display fitness but add index and migration complexity.
- Coarsened usage evidence makes reclamation approximate and requires measured tuning.
- Reclamation competes for filesystem and database resources, so background work and pinned demand
  must remain bounded to preserve interaction quality.
- Preview evidence is rebuildable, but schema migrations and rollback must preserve durable
  dimensions and user data.
- Until the complete lifecycle is implemented, the current admission-only behavior remains an
  honest runtime fallback and product text must describe that actual behavior.

## Validation evidence

The existing storage baseline is covered by settings-database reload, budget-bound, path-overlap,
restart-notice, atomic-reservation, Windows integration, and unchanged-source tests.

Recorded lifecycle evidence before the schema-v31/v3 amendment covers:

- EXIF Orientation 1 through 8, unknown dimensions, source edit, same-path replacement, identity-
  proven rename or move, temporary unavailability, and authoritative removal fixtures;
- proof that pending, ready, failed, retry, stale, missing-file, regeneration, cleanup, and eviction
  transitions preserve final geometry and logical scroll position;
- proof that an unknown-to-known dimension recovery is coalesced separately from preview readiness
  and replaces layout geometry while preserving the logical viewport anchor;
- legacy artifact usage accounting, manual cleanup, and pressure reclamation without deleting
  foreign files; its former v1-to-v2 adoption conclusion is superseded by the v3 decision above;
- proof that all compatible location references protect a shared artifact, reclamation resets every
  referencing location, and v14 last-writer ownership migrates without losing active references;
- proof that root unregistration and successful replacement publication remove retired location
  references, stale only zero-reference artifacts, and leave active references intact when a staged
  scan is abandoned, including a v15-to-v16 fixture that reconciles historical ownership without
  dropping valid shared references;
- proof that new encodings remain staged until post-decode source revalidation, while failed
  revalidation never deletes a compatible cache hit;
- measured bounded variant selection without per-pixel key growth;
- high-to-low-watermark reclamation that preserves pinned demand and does not thrash;
- stale-publication guards for query, revision, source state, algorithm, orientation, and bucket;
- bounded startup recovery of temporary, missing, unreferenced, and misaccounted artifacts;
- truthful manual-cleanup progress, cancellation, interruption recovery, and visible regeneration;
- preview-root activation failure and successful switch-and-regenerate fixtures;
- adapter, migration, application, Flutter geometry, Windows integration, daily, and Windows Release
  gates with source bytes and entries unchanged.

The schema-v31/v3 amendment is not accepted merely because its code and focused tests exist. Fresh
evidence must cover v30-to-v31 preservation and NULL baselining, catalog-wide/hardlink generation,
v1/v2 cleanup-only behavior, forced regeneration across both cache checks, same-handle source
staleness, exact-lease rejection and unrelated-revision tolerance, replacement-failure retention and
accounting, concurrency, corrupt or wrong-extension input, and the complete repository and Windows
gates. Those gates remain pending as of 2026-09-05.

Catalog page reclamation is likewise not accepted merely because its implementation and test
fixtures exist. Fresh evidence must cover fresh-schema incremental-auto-vacuum ordering, unchanged
legacy `NONE` startup migration, one-time conversion, bounded 256-page incremental batches,
cancellation and preemption, the five-second user-interactive timeout without a leaked priority
waiter, capacity overflow and insufficient-space failure, catalog-session invalidation after every
write-maintenance outcome, committed-removal failure isolation, shared ownership, settings progress,
complete repository gates, and a Windows run against an isolated disposable copy. Those gates
remain pending as of 2026-09-05.

Performance validation is a separate evidence class. It requires a bounded, read-only,
source-readable workload that records cold and warm preview latency, cache-byte growth, bucket
demand and reuse, reclamation duration, regeneration, and boundary churn while preserving source
bytes and entries. A retained-gallery Profile whose preview adapter rejects source-media
materialization can validate frame time, memory, query publication, and retained-detail behavior,
but it cannot satisfy these preview metrics. Catalog-parity evidence that leaves all previews
pending cannot satisfy them either.

The authorized `local-primary` Release-mode workload on 2026-08-13 completed this evidence class
without scanning a root or writing to the live catalog or preview cache. It created an online
catalog backup in isolated derived storage, selected 512 of 30,629 active local locations, and
completed 24 cold and compatible warm requests for each display bucket. Cold P95 latency was 193 ms
at 128 px, 201 ms at 256 px, and 211 ms at 512 px; warm P95 latency was 14 ms, 16 ms, and 16 ms.
All 72 compatible warm requests reused the same artifact without increasing cache bytes. Natural
1024 px pressure reached 57,068,214 bytes from 447 locations under a 64 MiB budget. Reclamation
removed 3,478,072 bytes in 234 ms and settled at 53,590,142 bytes, below the 80 percent low
watermark. One evicted artifact regenerated in 19 ms, its warm reuse took 13 ms, and immediate
boundary churn remained zero. Peak working set was 126,844,928 bytes, all 512 selected entries
retained their expected file state, 16 byte samples remained identical, and no preview request
failed. These measurements accept the current bucket and reclamation policy; they do not authorize
a speculative codec, concurrency, cache, or gallery rewrite.

The scaled-JPEG admission additionally requires the fixed large-image fixture, EXIF Orientation 1
through 8, corrupt-input fallback, and an optimized-build comparison against the existing full
decode. The initial 6000 by 4000 synthetic JPEG comparison on 2026-08-11 measured 90.286 ms for full
decode plus resize and 40.636 ms for scaled decode plus resize, a 2.22x improvement. This is a
bounded synthetic result, not yet target-library latency evidence.

Legacy cleanup compatibility additionally requires inclusive usage reporting, bounded recovery,
manual and pressure cleanup, proof that foreign entries remain in place, and proof that v1/v2 files
cannot satisfy or be promoted into a v3 demand.

## Replacement and rollback strategy

The settings repository, catalog repository, preview index, scheduler, and artifact store remain
separate Ame-owned ports. If reclamation, variants, or relocation regress interaction, integrity, or
recovery, Ame may temporarily return to admission-only reservation while preserving durable
dimensions, source configuration, and the forward-compatible preview index. Bucket policy,
persistence, or codec adapters can be replaced without a catalog or gallery rewrite.
The scaled-JPEG adapter can be removed independently to restore the existing `image` path without a
catalog migration or cache invalidation because the artifact and orientation contracts are
unchanged.
Recognition of v1/v2 files can be removed only with a migration that preserves managed-byte
accounting and foreign-file exclusion. Those artifacts remain cleanup-only rebuildable data and
never become current v3 evidence.
