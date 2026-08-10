# ADR 0014: Query-wide gallery layout manifest and unified navigation

- Status: Accepted for validation
- Date: 2026-08-09
- Amends: ADR 0010 and ADR 0011

## Context

The production gallery currently places one bounded asset window between estimated leading and
trailing extents. Unloaded ranges use generic square placeholders, and a distant time-rail target
replaces the materialized window only after the interaction settles.

That model protected the catalog and UI from repeated queries while dragging, but real-library
evidence with 79,013 locations exposed a deeper continuity problem:

- normal wheel scrolling can cross the current window before the next window is ready;
- placeholder geometry differs from the final equal-height justified rows;
- preview completion can change row composition and move content already visible to the user;
- a fast time-rail drag can show a regular square grid or an almost empty canvas for an extended
  period, followed by a second jump when real dimensions and previews arrive;
- window resizing can repeatedly recompute layout, request new decode widths, and correct scroll
  position through separate update paths.

These are not independent rendering defects. Layout identity, catalog-detail availability,
thumbnail availability, and navigation are currently coupled too closely. Further wheel-, drag-,
or resize-specific debounce patches would retain the same unstable model.

ADR 0009 remains authoritative for the unified Material 3 gallery and equal-height rows. ADR 0010
remains authoritative for one gallery `ScrollController`, Material `Slider` interaction semantics,
and WinUI-compatible annotations. ADR 0011 remains authoritative for revision-safe bounded keyset
retrieval. This decision replaces only their interim assumption that an estimated virtual extent
plus one replacement asset window is sufficient presentation state.

## Decision drivers

- The same logical image position must survive wheel scrolling, time-rail dragging, time-rail
  clicking, preview publication, incremental catalog updates, and window resizing.
- Geometry must be deterministic before a thumbnail is decoded.
- A cold cache may show placeholders, but it must not show a different layout or an unresponsive
  blank canvas.
- Full asset records, paths, and decoded media must remain bounded and lazy.
- Catalog requests and preview work must be cancellable or made harmless through revision and
  generation checks.
- Original media remains read-only and cloud-only placeholders must not be hydrated.

## Decision

### Query-wide compact layout manifest

The Rust application layer owns a revision-bound `GalleryLayoutManifest` for the complete active
query. It is a derived presentation index, not a second catalog and not durable user data.

The manifest contains only the fields needed to establish stable order and geometry:

- query identity and catalog revision;
- global ordinal and stable location identity;
- orientation-corrected aspect ratio or an explicit unknown-dimension marker;
- date-group identity and the evidence needed by time annotations;
- media and availability flags that affect inclusion or placeholder presentation.

It must not contain absolute paths, full metadata records, preview bytes, decoded images, analysis
results, or presentation widgets. Rust returns the manifest through bounded, revision-checked
chunks. Flutter stores it in compact typed structures rather than one object graph of full asset
records. A configured internal byte budget and synthetic 79,000-, 250,000-, and 1,000,000-item
measurements determine whether a single compact manifest or a hierarchical block representation is
used. Exceeding the budget must degrade to block summaries plus bounded exact blocks, never to
materializing every asset record.

The manifest is invalidated by query identity or catalog revision. A newer revision is assembled
separately and published atomically; a partial manifest never replaces the last trustworthy one.

### Deterministic layout snapshot

`GalleryLayoutSnapshot` derives all row geometry from one manifest, viewport width, selected layout,
and density setting. It records row membership, item rectangles, cumulative row offsets, date
anchors, and total extent in compact arrays.

- Equal-height justified rows are computed across detail-page boundaries, not independently inside
  each loaded page.
- An unknown aspect ratio uses one documented stable fallback until a later catalog revision
  supplies trustworthy dimensions. Preview readiness never supplies layout dimensions.
- The placeholder, failed-preview state, and decoded thumbnail use the exact same item rectangle.
- Publishing or evicting a preview must not change row membership, item bounds, total extent, or the
  user's scroll position.
- Only a query, sort, filter, layout, density, catalog revision, or viewport-width change may create
  a new layout snapshot.

Image width and height remain durable catalog columns and are the authoritative source for aspect
ratio after restart; reopening Ame must not decode source media merely to recover tile proportions.
Completed layout snapshots may be retained as rebuildable cache data, but the cache key must include
query identity, catalog revision, sort and filter identity, layout algorithm version, density, and
the exact effective viewport width. A cache miss recomputes from the manifest, and a cached snapshot
may never override newer catalog dimensions.

The existing generic square placeholder slivers are removed after this snapshot owns the complete
visible range. An unloaded item is represented by a static placeholder inside its final rectangle.
Indeterminate progress indicators are reserved for explicit foreground work and are not repeated
across a large scrolling wall.

### Logical viewport anchor

The stable navigation value is a `GalleryViewportAnchor`, not a raw pixel offset. It contains:

- query identity and catalog revision;
- stable location identity when still present;
- global ordinal as a fallback;
- fractional position inside the owning row;
- fractional position inside the viewport.

Pixels are derived from the current layout snapshot. A resize or compatible catalog refresh
re-resolves the same anchor and applies one layout-time correction before the new snapshot is
painted. If the location disappeared, the nearest surviving ordinal becomes the explicit fallback.

### One navigation coordinator

One `GalleryNavigationCoordinator` owns all writes to the gallery `ScrollController`. It accepts
four typed intents:

- relative movement from wheel, touchpad, keyboard, or accessibility actions;
- continuous direct manipulation from the annotated time rail;
- discrete jumps from time annotations, search results, sources, or restored state;
- relayout caused by viewport or presentation-setting changes.

The coordinator keeps interaction state separate from data readiness. There is no scrub-only
controller, loaded-window scroll position, or second canonical offset. Rail labels, the primary
line, visible date headings, page prefetch, and restored position all derive from the same logical
anchor and layout snapshot.

### Interaction contracts

| Input | Position behavior | Detail loading | Preview behavior |
| --- | --- | --- | --- |
| Wheel, touchpad, keyboard | Native relative scrolling on the one `Scrollable` | Prefetch bounded pages before and after the viewport; never replace the complete visible model | Visible and near-viewport work has priority; expensive decode may be deferred at high velocity |
| Time-rail drag | Update the exact manifest-backed position at most once per frame | Latest-wins target page requests at a measured bounded cadence; release promotes the final target | Reuse cached previews; otherwise show final-geometry placeholders |
| Time-rail click or other distant jump | Jump directly to the resolved anchor without animating through the library | Request the target page and guard pages with highest priority | Publish cached or newly decoded previews without changing geometry |
| Window resize | Preserve the logical anchor while one latest layout snapshot replaces the previous snapshot atomically | Reuse already loaded details; discard obsolete width computations | Reuse decode-width buckets and request only missing sizes after layout stabilizes |

`Scrollable.recommendDeferredLoadingForContext`, `ScrollPosition.isScrollingNotifier`, and
`ScrollController.onAttach` from the repository-pinned Flutter SDK may be used to schedule expensive
preview work. They must not defer manifest geometry or convert the gallery into a different layout.

### Bounded asset-detail windows

ADR 0011 keyset queries continue to retrieve full `LibraryAsset` details in bounded pages. The
presentation keeps a small revision-bound page cache indexed by global range instead of treating
one page as the gallery itself.

- Current, preceding, following, and target guard pages are retained under an explicit memory
  budget.
- Ordinary scrolling expands or evicts this cache without replacing the layout snapshot.
- Requests carry query revision, range, priority, and navigation generation.
- Obsolete responses may populate a compatible cache but must never publish a stale active range or
  move the viewport.
- Catalog failures retain the last trustworthy snapshot and expose a bounded stale or failed state.

Page size, guard distance, and prefetch horizon are benchmarked values, not widget constants.

### Preview store and scheduling

Preview state is removed from the collection object that owns layout. A `PreviewStore` publishes
changes by stable location identity, allowing one tile to repaint without replacing the complete
asset list or recomputing rows.

The scheduler uses explicit priorities:

1. the current viewer item;
2. visible gallery items;
3. near-viewport items in the current movement direction;
4. retained guard pages;
5. optional idle warming within the cache budget.

Queued work is cancellable. Active work carries a generation and its result is ignored when the
query, revision, target width, or source state is no longer compatible. Concurrency is bounded and
measured separately for local files and cloud-backed roots. No scheduler action may recall an
offline placeholder.

### Resize computation

Resize events may produce at most one pending layout request per frame, and only the newest viewport
width is eligible to publish. The old snapshot remains coherent until the replacement is ready.
Layout and anchor correction publish as one transaction from the presentation's perspective.

The layout algorithm first receives focused profiling in Dart using compact typed arrays. Moving it
to an isolate or the Rust application layer requires measured evidence that transfer and
synchronization cost is lower than the UI-thread cost; it is not assumed in advance.

## Migration sequence

1. Add the manifest contract, revision handling, chunked Rust query, compact Flutter storage, and
   memory and query benchmarks without changing the visible gallery.
2. Separate preview readiness from `LibraryAsset` collection replacement and prove that a preview
   publication cannot change layout geometry.
3. Render the existing equal-height gallery from a complete `GalleryLayoutSnapshot`; use identical
   rectangles for unloaded, failed, and ready preview states.
4. Replace the single replacement window with the bounded asset-detail page cache while preserving
   ADR 0011 keyset and stale-revision behavior.
5. Route wheel, time-rail drag, time-rail click, restoration, and source navigation through the one
   coordinator; add latest-wins request generations and priority prefetch.
6. Add atomic logical-anchor preservation for live window resizing and decode-width reuse.
7. Remove aggregate unloaded geometry, generic square placeholder slivers, settle-only wheel seeks,
   and other superseded compensating paths only after parity tests pass.

Each step must remain independently testable and must not require source-media changes or a full
preview build.

## Validation gates

This decision becomes **Accepted** only when all of the following pass:

- A deterministic fixture proves preview pending, ready, failed, retry, and eviction transitions do
  not change any row or item rectangle.
- Normal wheel scrolling through page boundaries does not replace the gallery with a square grid,
  blank canvas, or different row composition.
- Rapid drag, drag reversal, repeated distant clicks, and release publish only the latest compatible
  target; stale requests cannot move the viewport.
- Time-rail input and its primary line update within one display frame while catalog and preview
  work remains bounded.
- A distant cold-cache jump immediately shows final-geometry placeholders and remains interactive;
  source-backed previews fill in without another layout transition.
- Live resize preserves the logical item and viewport fraction with no more than two logical pixels
  of post-layout drift after settling.
- Profile-mode frame evidence on the project workstation records P95 build and raster times within
  the 60 Hz frame budget for wheel, scrub, and resize scenarios, and records every UI-thread stall
  over 50 ms for investigation rather than hiding it in an average.
- Synthetic 79,000-, 250,000-, and 1,000,000-item manifests record build time, peak memory, retained
  bytes per item, resize recomputation time, page-cache bounds, and cancellation behavior.
- The authorized 79,013-location library passes slow wheel, fast wheel, time-rail drag and reversal,
  annotation click, repeated resize, preview failure, missing root, and stale-revision scenarios.
- Focused unit, widget, accessibility, bridge, migration, Windows Debug and Release, and repository
  quality gates pass serially.
- Source-byte samples, source entries, and cloud-placeholder availability remain unchanged.

Replacement conditions:

- If the compact manifest exceeds its measured memory budget, replace only its storage
  representation with a hierarchical block index while preserving exact visible-block geometry and
  the public coordinator contract.
- If Dart layout misses the frame and resize gates, move the pure layout calculation behind a
  measured isolate or Rust port without moving widget, gesture, or scroll ownership out of Flutter.
- If any step cannot preserve the single authoritative scroll position, stop and revise this ADR
  rather than adding another synchronization state.

## Consequences and risks

- Ame retains compact query-wide presentation evidence while keeping full records and media bounded.
  This intentionally trades a small measured memory cost for stable geometry and continuous input.
- Cold caches still cannot display source pixels instantly. The guaranteed outcome is a responsive
  gallery with final geometry, not an impossible zero-latency full-library preview build.
- Resize now has one potentially expensive deterministic calculation. Coalescing, typed storage,
  profiling, and an explicit computation replacement boundary prevent per-widget work from
  becoming the architecture.
- A manifest revision is derived and rebuildable. Its failure cannot invalidate the last published
  catalog or durable user decisions.
- This decision removes interaction-specific patching pressure, but migration must be incremental;
  deleting the current fallback before the new path passes parity would recreate the blank-gallery
  failure.
- Source media remains read-only, and no new permission to scan, hydrate, move, rename, or delete a
  source is introduced.

## Evidence available at acceptance-for-validation

- The authorized catalog contains 79,013 active locations across the two read-only roots.
- A 2026-08-09 interaction recording and the user's direct observations show wheel-induced layout
  repacking, slow placeholder fill, a square placeholder wall during rapid time navigation, and a
  square-to-justified transition when details arrive.
- ADR 0010 already records the missing compact query-wide layout index as the remaining continuity
  gap; the new evidence confirms that the gap is release-blocking rather than optional.
- The repository-pinned Flutter SDK exposes the framework scroll-activity and deferred-loading
  primitives named above, so the design can preserve framework-owned scrolling and accessibility.
