# ADR 0005: Govern storage and the bounded preview lifecycle

- Status: Accepted
- Date: 2026-08-07
- Last updated: 2026-08-12
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

Ame owns a preview-artifact index separate from gallery layout and durable user data. Its
replaceable persistence representation records enough evidence to identify and account for an
artifact:

- every compatible active location reference plus compatible source state;
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
policies. The queue maps them to base concurrency limits of one, two, and four; `medium` remains the
default and preserves the accepted two-request baseline. The existing priority replacement path may
temporarily use one additional slot so a viewer or newly visible item is not blocked behind obsolete
work. Lowering the preference does not cancel active decodes; it prevents new work from starting
until the active count falls below the new limit. Raw worker counts and queue depth remain internal.

Each request and publication carries compatible location, source-state, catalog-revision, algorithm,
orientation, and size-bucket identity. Generation writes a temporary artifact outside source trees,
revalidates ownership, and atomically installs the completed file and index evidence. Obsolete work
cannot publish over a newer generation or restore a stale path. No preview operation recalls an
offline placeholder.

The artifact index keeps location ownership as a many-to-many reference instead of a single mutable
owner column. Identity-proven hard links, renamed locations, and other compatible active locations
may reference one rebuildable artifact. Reclamation excludes an artifact when any reference is in
protected demand, and removal resets every active location that still names the artifact. Schema
v15 migrates v14 rows by rebuilding those references from every catalog location that publishes
the same artifact path; it does not infer ownership from the v14 row's last writer alone. Normal
scan publication then keeps the reference set aligned with the active snapshot.

A cache hit and a newly generated artifact have different publication lifecycles. A compatible
cache hit remains installed when the current source revalidation fails. A new encoding stays in a
managed temporary namespace, including its reserved budget bytes, until source revalidation
succeeds; only then is it atomically renamed into the final namespace. Failed validation removes
only the staged file and cannot delete an artifact already used by another location.

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
dimensions; CMYK, greater-than-8-bit, malformed, or unsupported JPEGs fall back to the existing
`image` decoder, as do all other formats.

This is a narrow admission rather than a new media authority. `jpeg-decoder` is in maintenance mode,
so its types and errors do not cross the preview-store boundary, its optional Rayon feature remains
disabled, and concurrency remains owned by Ame's bounded presentation queue rather than the decoder.
Keeping the current `image` full decode was rejected for large ordinary JPEG previews because it always
materializes the full source raster. ImageMagick or libjpeg-turbo process adapters were not selected
for this slice because they add a native runtime, packaging, cancellation, and crash-isolation
boundary. Windows Imaging Component was not selected because it would make the first optimization
Windows-specific and introduce a new unsafe platform boundary.

Legacy `ame-jpeg-thumbnail-v1` artifacts are adopted only on current preview demand. Ame derives
the exact legacy key from the current source-state evidence and requested bucket, validates that
the cached file is a bounded decodable JPEG, and reads source orientation metadata without decoding
the full raster. Only a source with the normal orientation can reuse the legacy pixels because v1
did not apply the current EXIF orientation contract. After validating compatibility, Ame atomically
renames the legacy file to the current managed v2 key; it does not re-encode, copy, grow
the cache, or add a second cache walk. The existing startup inventory enables legacy lookup only
when a bare v1 artifact is present, so a pure v2 cache does not add a legacy file probe to each
miss. Invalid, oriented, or failed-promotion entries remain untouched and use the normal current
generation path.

The exact legacy filename contract is Ame-owned rebuildable storage. Startup inventory and the
settings surface include its bytes in the same preview budget as current artifacts. An atomic v1
to v2 rename keeps accounting constant instead of reserving the same bytes twice. Explicit manual
cleanup and pressure-triggered automatic reclamation may delete recognized v1 artifacts while
continuing to exclude unrelated files. Bounded startup recovery does not treat an unadopted v1
artifact as an unreferenced v2 artifact, so ordinary startup preserves its on-demand reuse value.

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

### Preview-root transition

Preview relocation uses a switch-and-regenerate workflow because previews are rebuildable:

1. validate and persist a pending target outside every source root;
2. keep the old root active until restart and successful target initialization;
3. activate the target and atomically reset old-root preview entries to pending without changing
   durable dimensions;
4. regenerate in the target only through normal demand;
5. retain explicit ownership of the old root until the user starts verified cleanup.

Activation failure leaves the old root authoritative. Ame never silently deletes it or calls its
space reclaimed. A future copy-based preview migration is optional and requires integrity
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

Current deterministic lifecycle evidence covers:

- EXIF Orientation 1 through 8, unknown dimensions, source edit, same-path replacement, identity-
  proven rename or move, temporary unavailability, and authoritative removal fixtures;
- proof that pending, ready, failed, retry, stale, missing-file, regeneration, cleanup, and eviction
  transitions preserve final geometry and logical scroll position;
- proof that an unknown-to-known dimension recovery is coalesced separately from preview readiness
  and replaces layout geometry while preserving the logical viewport anchor;
- proof that current and recognized legacy artifacts share usage accounting, manual cleanup, and
  pressure reclamation without deleting foreign files or eagerly discarding legacy reuse at startup;
- proof that all compatible location references protect a shared artifact, reclamation resets every
  referencing location, and v14 last-writer ownership migrates without losing active references;
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

Performance validation is a separate evidence class. It requires a bounded, read-only,
source-readable workload that records cold and warm preview latency, cache-byte growth, bucket
demand and reuse, reclamation duration, regeneration, and boundary churn while preserving source
bytes and entries. A retained-gallery Profile whose preview adapter rejects source-media
materialization can validate frame time, memory, query publication, and retained-detail behavior,
but it cannot satisfy these preview metrics. Catalog-parity evidence that leaves all previews
pending cannot satisfy them either.

The scaled-JPEG admission additionally requires the fixed large-image fixture, EXIF Orientation 1
through 8, corrupt-input fallback, and an optimized-build comparison against the existing full
decode. The initial 6000 by 4000 synthetic JPEG comparison on 2026-08-11 measured 90.286 ms for full
decode plus resize and 40.636 ms for scaled decode plus resize, a 2.22x improvement. This is a
bounded synthetic result, not yet target-library latency evidence.

Legacy adoption additionally requires byte-identical promotion without source changes or
re-encoding, rejection of non-normal orientation, constant accounting across atomic promotion,
inclusive usage reporting, manual and pressure cleanup, startup preservation, and proof that
foreign entries remain in place.

## Replacement and rollback strategy

The settings repository, catalog repository, preview index, scheduler, and artifact store remain
separate Ame-owned ports. If reclamation, variants, or relocation regress interaction, integrity, or
recovery, Ame may temporarily return to admission-only reservation while preserving durable
dimensions, source configuration, and the forward-compatible preview index. Bucket policy,
persistence, or codec adapters can be replaced without a catalog or gallery rewrite.
The scaled-JPEG adapter can be removed independently to restore the existing `image` path without a
catalog migration or cache invalidation because the artifact and orientation contracts are
unchanged.
Legacy adoption can be disabled independently; already adopted files remain valid current v2
artifacts, while untouched v1 files remain rebuildable derived data governed by the same preview
budget and explicit cleanup boundary.
