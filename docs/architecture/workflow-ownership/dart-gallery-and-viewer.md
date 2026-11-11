# Dart gallery queries, paging and image lifetimes

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Preview image construction

`LibraryPreviewImages` is a presentation contract for Flutter image providers over derived preview
paths. `LibraryReadBindings` selects the platform factory; tiles, filmstrip thumbnails and viewer
fallbacks consume it without filesystem access. The factory preserves `FileImage` identity and
native decoding. Existing callers retain resize keys, eviction, repair admission and image-reveal
lifetimes. Original-image reads continue through the guarded source scheduler and leases; this
factory neither replaces that path nor grants source-read authority.

`LibraryViewerLaunch` captures the mounted thumbnail's decoded provider key and sampling quality
with its geometry. `LibraryImageReveal` publishes that key only after a frame is available and
retires its registration on disposal. Both embedded and routed viewers reuse it for entry before
blending in the guarded original; they do not start an unresized preview decode during the flight.

## Query and image lifetimes

- `library_viewer_session.dart` owns viewer selection, navigation requests, stable-asset lookups,
  and preview demand. Closing or reopening creates a new selection identity even for the same
  asset. Old continuations, error handlers, finalizers, and post-frame callbacks cannot change the
  new session; the screen composes existing viewer controls rather than owning those state machines.
- Viewport/query ownership and committed root-removal refresh live in dedicated application
  coordinators. The viewport has one query-generation and revision owner for retained pages, time
  navigation, and directly requested visible ranges. Root removal distinguishes the one database
  unregister command from display-only refresh retries. Gallery widgets consume immutable state
  and may not compensate for an unresolved catalog or task-lifecycle invariant.
- `library_page_operation.dart` owns cursor-read completion independently of viewer selection.
  Same-direction requests share the current read; opposite directions wait only for predecessors
  registered earlier in the same query generation. A new generation does not wait for old reads,
  and superseded queued work cannot execute. The viewport revalidates query, revision, and cursor
  before a queued read; the viewer alone decides whether a completed page may change selection.
  Retired navigation remains silent, while a current explicit action with no target reports failure.
- `library_retained_catalog_window.dart` owns retained page identity updates, directional insertion,
  cursor advancement and whole-page eviction. It keeps the 5,000/3,500 high/low watermarks and at
  least one page. Newer observations replace matching locations without duplicating their positions.
  Forward publication uses the response's next cursor; backward publication retains the last kept
  page's next cursor. The viewport validates query/revision and publication authority before merging,
  resets the window on accepted replacement, and retains directional completion/error semantics.
  Duplicate-review windows keep their separate checkpoint and capacity policy. The same window
  serves exact and similar groups; typed member slices carry each mode's decision snapshot only
  after query-generation admission. Similarity query/selection state belongs to
  `LibrarySimilarReviewController`; `LibrarySimilarChoices` serializes writes and owns their errors.
  Late writes retain their original group, degree and revision and cannot replace a newer view.
  Exclusion undo retains its operation identity independently of the currently open group.
- `library_time_navigation_requests.dart` owns pending/active target reuse, latest navigation
  intent, explicit visible-range ownership, blocked retry and operation-matched loading release.
  The viewport retains query compatibility, publication generation, query-transition authority,
  catalog reads and immutable page projection. A result needs both current publication authority
  and acceptance by the navigation owner; those generations are not interchangeable. Query
  replacement and disposal settle waiters without letting an old finalizer clear newer loading.
  Physical reads remain serialized, and passive demand cannot replace a compatible explicit jump.
- `library_time_snapshot_reader.dart` owns the single semantic read admitted after an explicit
  date navigation encounters a stale strict anchor, coherent-result validation and immutable
  projection. Passive prefetch cannot acquire that authority unless the same request is promoted
  by an explicit target. A stale passive read refreshes through the mounted query projection,
  retaining its original navigation/publication authority. Promotion during that recovery retires
  the passive result and transfers the same request to one semantic resolution; it cannot lose
  the explicit target by publishing the previous visible anchor. The query snapshot reader retires
  results and errors whose caller authority expired, and publication revalidates that authority.
  Cancellation, query/publication supersession and disposal fence both
  reads. `library_time_seek_alignment.dart` retains the semantic target through its admitted
  publication; current geometry and complete target-row coverage govern alignment. Query, layout,
  controller and later gesture changes retire that presentation authority. The presentation
  `library_time_seek_scheduler.dart` owns one admitted read and the latest pending pointer target;
  continuous motion cannot starve the admitted read. The rail retains its Material gesture through
  same-query publications, while completed reads cannot align over a held pointer.
- `library_preview_order.dart` owns the pure priority, optional demand-rank and arrival comparison.
  The queue owns active-location exclusion, demand/source validity, bounded execution and result
  acceptance; selection order cannot confer publication authority. `LibraryState` separately
  projects explicit removal, query-refresh and primary-scan precedence without owning execution.
- `library_folder_tree.dart` owns published folder windows and their cache lifetime. Query revision
  invalidation retires loading authority without hiding retained windows; an accepted page replaces
  or appends atomically. A parent proving a leaf or complete child absence retires corresponding
  descendants, while incomplete paging cannot prove absence and an older parent cannot disprove a
  newer child window. Configured-root removal retires all of that root's branches. The folder read
  controller checks root membership, revision, scope, cursor and the exact pending branch slot;
  superseded, retired or disposed reads cannot republish or clear a newer loading state.
  `library_folder_root_loader.dart` serializes initial root-window discovery and coalesces queued
  catalog revisions; it uses that same controller authority without reading source directories.
- `library_preview_feedback.dart` owns constrained preview-failure and retry-progress layout. It
  measures labels with the current text scale and composes Material buttons/progress with the shared
  tooltip and semantics boundary. Source identity, retry admission, cache eviction and result
  acceptance remain with the existing photo tile and preview application owners.
- `library_preview_failure.dart` distinguishes failed materialization commands from failed
  artifacts. A command exception cannot revoke an existing ready artifact; a source-authorized
  failed asset can. Queue completion still reports the command outcome and validates publication
  authority. A missing active catalog location retires the old request without publishing failure.
  `library_preview_sizing.dart` separately owns verified sizes and failed exact-source/size attempts.
  Verified sizes retain at most 256 recently used exact-source observations across demand changes;
  `library_preview_store.dart` independently retains at most 256 recent ready overlays in addition
  to demanded entries. These caches grant no request or publication authority. Failed size attempts
  remain scoped to current demand. Request completion carries the actual attempted size; failure
  is never successful size verification. Changed demand/source, explicit retry or authority reset
  releases failed attempts. Root availability retains its separate root-failure owner's cooldown.
- `library_album_heading.dart` retains one selected album's display metadata while its asynchronous
  details load. Confirmed metadata replaces that label, including renames and removal. It grants no
  query, membership or command authority, and another album's late result cannot replace the selection.
- `LibraryQueryActivity` is a sealed idle/loading/failed projection owned by the viewport. Failure
  retains its requested query separately from the still-visible gallery and primary task error.
  Primary scan publication separately distinguishes uncommitted work, committed display reload,
  and visible completion. A superseded reload is not visible completion, and a committed task retry
  cannot regain source-scan authority. Task-surface selection composes primary and per-root work;
  it does not serialize their visibility or infer execution from a retained checkpoint.
- `library_query_refresh.dart` owns admission for user queries, passive synchronization reads,
  and committed display-refresh obligations. User queries may supersede an attempt; passive reads
  defer while an attempt or committed obligation exists. Committed refresh waits for the latest
  query and exclusive publication owner before reading the current scope. It continues only after
  proven supersession, not after an arbitrary failure or unchanged busy result. Disposal settles
  every in-flight waiter, including replaced attempts. Catalog revision disagreement remains a
  genuine read failure, and retry cannot repeat the committed source scan.
  Explicit date navigation shares user-query admission, so committed refresh waits for its result.
  Paging and prefetch recovery retain the initiating user intent; a newer query or date request
  retires that recovery without retrying it behind the request that is waiting for it to release.
  A deferred directory selection captures a separate user-intent generation. A newer user query,
  including A-B-A replacement, external authority invalidation or disposal retires it; passive
  refresh does not. A covered-directory result consumes that intent once and uses the existing
  coherent query read and failure/retry path, with no source-scan authority.
  Passive admission precedes presentation capture, so a rejected request cannot retire an active
  gallery projection. A passive request waiting for a gesture retains its original publication
  authority and revalidates it before reading. Expired paging cursors use the same position-aware
  refresh boundary, but retain their original query/publication authority; unlike a committed scan
  obligation, they cannot resume against a later user query or erase its failure.
- `library_query_projection.dart` owns the current gallery's optional projection registration.
  A committed application refresh may use that projection to capture a stable visible identity;
  without a mounted gallery it still reads the catalog. Detaching an old registration cannot
  revoke a replacement, and application disposal admits no further read. The viewport retains
  committed admission, coherent query reads, revision checks and catalog-publication authority.
  Explicit folder selection requests a fresh position through that same projection rather than
  carrying an anchor from the former folder or waiting for its scroll gesture.
  A read also retains the projection's position generation. A later user scroll retires that
  authority before catalog publication; a committed refresh resumes after the gesture using its
  new position. Query, catalog-publication and position generations remain independent.
  Passive synchronization carries the same position token through its coherent read and final
  publication; obsolete success or failure cannot replace a later gesture. Application
  `library_query_transition.dart` owns the typed query-read baseline and exact-read retirement.
  Consecutive query replacements retain their original published baseline; a date read or external
  authority transfer retires that baseline and its loading state before taking ownership. A late
  query finalizer cannot release a newer transition or leave visible-range admission blocked.
  `library_gallery_query_transition.dart` owns position capture, identity/fallback resolution and
  presentation-generation cleanup for user queries, passive refresh and committed import display.
  It preserves the resolved page's global offset and the visible row's fraction. A retired gallery
  cannot restore pixels or clear a newer pending position; its retirement does not undo an accepted
  application publication. The screen composes the resulting layout transition without owning a
  second asynchronous query-position lifecycle. A gesture waiter belongs to its presentation
  generation; a newer query or disposal retires it without issuing its old read.
  `LibraryGalleryPositionResolver` captures the geometry still on screen before a newer snapshot's
  first frame. Its current, coherent manifest can rebase that location and both fractions into the
  new revision; another query or a future geometry revision cannot supply that position.
  Consecutive explicit filters retain a separate image/fraction anchor across absent results and
  extent clamping. That intent is not catalog-publication authority and does not drive passive
  refresh positioning. Scrolling, date navigation, source/folder or ordering changes and disposal
  retire it; a failed filter can retry it, and late completions cannot recreate retired intent.
- `library_gallery_dimension_publication.dart` owns query/revision-scoped recovered dimensions,
  visible-range classification, the frozen recovery anchor and quiet/maximum-delay publication.
  Scrolling reclassifies pending versus deferred dimensions; query replacement retires both and
  the manifest overlay cache. Navigation and disposal cancel scheduled publication. The screen
  supplies position capture and pending-transition state, retains actual scroll objects, and
  composes the resulting layout transition. This owner grants no catalog-publication authority.
- `library_gallery_page_demand.dart` retains directional edge demand while the current query is
  busy and admits each available cursor/revision once. Moving away, changing direction or query,
  and a page failure retire that demand. Gallery interaction checks the resulting scroll position
  and keeps its lifetime across estimated/full layout replacement; application paging retains
  read serialization, cursor validation and publication authority.
- `LibrarySourceImage` composes the installed Flutter SDK's immutable-buffer and image-decoder
  primitives through the application-owned source-reader port. The scheduler owns bounded latest
  intent and read completion; the image provider owns source-lease cache identity and the native
  buffer-copy lifetime. `application/viewer_source.rs` owns catalog admission and lease capacity,
  while `library_source_image_stream.dart` owns retired stream errors and late codec/frame resource
  release without replacing Flutter's decoding or animation scheduler.
  `LibraryImageComparison` owns one latest-intent scheduler per pane and disposes both on exit.
  The panes share the source-reader port and its native two-lease limit, not a superseding selection
  queue. Each image retains its guard until its buffer copy finishes; retired panes cannot publish
  decoded frames. Preview demand belongs to the comparison surface; `LibraryComparisonView` owns
  active-pane selection and linking between two existing `LibraryViewerZoom` instances.
  `LibraryViewerImage` composes its current source frame with an image-only transformation builder;
  pending-load feedback and failure/retry controls stay outside that transformed child without
  another loading-state owner. A fallback preview retains the image transform independently.
  `library_viewer_zoom_geometry.dart` owns fitted versus actual-pixel scale, bounded zoom, slider
  mapping and source-pixel percentage. `LibraryViewerZoom` owns zoom intent, constraints and animation;
  `LibraryViewerImageInteraction` shares framework gestures, smooth wheel zoom and keyboard bindings
  between normal viewing and comparison. `LibraryViewerFrame` owns their shared title bar, caption
  controls, full-screen lifetime and return behavior. `LibraryViewerToolbar` composes the common bottom
  zoom controls with each surface's leading actions. These presentation owners grant no source authority.
  `local_files/viewer_source_guard.rs` owns the held Windows namespace. Opaque bridge methods
  only translate acquisition and idempotent close. A same-path edit with unchanged size and restored
  modification time must resolve a different stream when its source generation changes.
  Preview-only updates do not invalidate the viewer's source stream, and superseded source streams
  cannot publish into the current image widget. No layer may fall back to an unchecked source path.
  `LibraryViewerSourceScope` supplies a scheduler/buffer-loader pair through an immutable inherited
  dependency boundary. Explicit image-widget dependencies take precedence. The viewer entry reads
  the application scheduler Provider when no inherited override exists; the image provider requires
  that scheduler explicitly and never selects a native reader. Only effective dependency changes
  retire the image provider. One root Provider container owns one scheduler, inherited by child
  scopes. Container disposal retires pending requests and marks active requests cancelled; the image
  provider still closes an acquired lease only after the buffer copy finishes. Admission that returns
  after retirement closes its lease before settling. The foreground-read port retains the same shared
  application activity counter for native previews and source leases.
