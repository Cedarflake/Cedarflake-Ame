# Dart composition, feedback and interaction lifetimes

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Classification composition

`LibraryClassificationServices` selects the classification, configuration, review and installation
ports. `LibraryClassificationBindings` composes their existing application owners and required
Provider identities. One configuration service serves automatic startup and explicit configuration;
one lifecycle supplies environment updates and configuration retirement. Construction performs no
native work or installation admission. Startup and authorized recovery remain in the existing
post-mount sequence. Shutdown joins model installation, retires environment observation and joins
classification before the root Provider container disposes its notifiers. Session, maintenance and
installation handles retain their application lifetimes; native adapters translate failures into
Ame values. Missing bindings fail during acquisition and release acquired lifetimes.

## Workbench destinations

`workbench_presentation_bindings.dart` connects the library's Settings destination and record-feedback
slots to their owning Settings surfaces, and connects the Settings default-album choice to the library
picker. Features declare typed presentation callbacks and required Providers; they do not import one
another's pages or private widgets. The choice result distinguishes cancellation from clearing an
album. Feedback watches the existing calling `WidgetRef`, preserving precedence, confirmation,
retry and dismissal authority. The workbench retains visited pages, scroll position and focus.

## Presentation and application owners

- `CatalogReclamationController` owns application-facing reclamation state, polling, command epochs,
  ordered full-storage snapshots, and terminal usage refresh. A snapshot started before or during a
  newer command cannot replace that command's result. Settings persistence and preview cleanup
  remain separate workflows; neither keeps another copy of the reclamation state machine.
  Accepted full snapshots publish storage usage and reclamation phase together. Configuration-save
  receipts have independent typed merge authority over configuration only; stale accompanying usage
  cannot replace a newer measurement, and a read begun before a confirmed save preserves that save.
  Retired-preview ownership is lifecycle evidence, not a configuration field: only ordered full
  status reads replace that list. Save completion re-reads full status to reconcile backend ownership
  changes without deriving cleanup policy from configured paths. A failed post-save refresh retains
  confirmed configuration and retries the read independently; it never repeats the successful save.
  `StorageSnapshotCoordinator` separately owns full-read scheduling and completion: same-version
  requests share one in-flight read; epoch or configuration changes coalesce one latest follow-up.
  Commands suspend, rather than discard, required refreshes and wake them after settlement. Disposal
  settles waiting callers without waiting for a stale backend read. Snapshot acceptance and
  configuration policy remain in the reclamation controller, not the scheduling module. The
  controller, configuration projection and snapshot coordinator live together under storage
  application; the Settings widget renders their state without owning a second read lifecycle.
- `LibraryExactAnalysisController` retains stream/run identity, cancellation admission and terminal
  completion; `LibraryExactAnalysisScheduler` retains publication generations, idle admission and
  foreground yielding. Duplicate service bindings select the native ports without changing those
  owners. `LibraryExactAnalysisBindings` owns their single production pair, joins scheduler close
  before disposing both notifiers, and shares the exact-analysis port with duplicate queries.
  Repeated close and partial-startup cleanup cannot construct or retire a replacement runtime.
- Menus, loading feedback, task live regions, and startup orchestration use repository-owned shared
  components so one defect fix does not create a second interaction contract.
- `library_bottom_feedback.dart` owns the single bottom feedback position and admission between
  record operations, existing library tasks and notifications. `organization_record_feedback.dart`
  projects archive and cleanup task state without inserting rows into Settings; active work precedes
  retained failures. `organization_record_actions.dart` owns confirmation and retry entrypoints shared
  by Settings and this feedback. Application controllers retain cancellation, result admission and
  publication. `AmeFeedbackCard` supplies the existing notification geometry to both task feedback
  and notifications; notification lifetime and history remain with their existing owners.
- `library_gallery_loading_region.dart` owns stable gallery loading-feedback geometry. It composes
  the existing Material linear indicator above the unchanged child viewport with framework Stack
  layout and pointer passthrough. The screen supplies the existing loading projection; neither
  owner changes scroll position, query authority or asynchronous loading lifetime to hide movement.
- `library_gallery_states.dart` owns empty-gallery selection after the screen preserves published
  items. Query loading precedes a scoped published-result reload, then an incomplete first import;
  a root without a published baseline cannot prove empty results. `library_pending_source.dart`
  projects an unlisted source from the primary task snapshot, so query activity cannot animate
  a paused import. Both are read-only presentation
  owners; they neither publish staged content nor acquire scan authority.
- `library_gallery_reflow.dart` owns capture/resolution of a query-bound viewport anchor and the
  pending geometry change. A retained scroll origin binds a framework ScrollPosition and its
  captured offset; later movement invalidates reuse of the old anchor. The wall validates that
  origin both when scheduling and when applying a transition, then resolves a replacement anchor
  from the still-displayed snapshot when needed. The screen uses the same origin before freezing
  a subsequent dimension-recovery epoch. Explicit transition generations stay distinct from
  geometry generations; reflow alone cannot widen the frozen visible range or promote prefetch.
- `library_gallery_prepend_compensation.dart` owns the position adjustment for earlier rows in a
  window-local layout. Its shared-item offset preserves later wheel input even when trailing rows
  are trimmed. A full query manifest owns all row positions and cannot inherit this compensation.
  Query/revision/scroll-position replacement and explicit date/query/layout intent retire the old
  operation; matching cleanup cannot clear a newer prepend owner. The screen composes the page
  request and rendered-frame boundary, while application paging retains read/publication authority.
- `library_gallery_position.dart` owns visible-position identity, the displayed-geometry resolver
  and layout-transition position admission. Both local-window layout and first-manifest takeover
  resolve later scrolling before replacing the displayed resolver. Query and revision must match;
  detached geometry cannot supply an anchor. A transition without an admissible position retires
  through the existing generation-bound layout acknowledgement without moving the viewport.
- `library_gallery_interaction.dart` composes the existing `AmeScrollMotion` and framework scroll
  notifications for both gallery renderers. It owns wheel/direct-drag page admission, movement
  direction and mounted deferred dispatch; paging errors still require explicit retry. The wall
  retains layout publication, hidden-view admission and the existing scroll-view identity.
  `library_gallery_viewport_feedback.dart` owns query-bound visible ranges and bounded, center-first
  preview demand. Local layout offsets become global item ordinals before row lookup. Complete
  manifests and estimated square geometry retain their distinct range inputs; this shared owner
  cannot grant query, revision, layout or loading authority.
- `library_gallery_layout_continuity.dart` owns the anchor retained across consecutive manual
  density/shape choices. Re-selecting the horizontal center cell after each column-count change
  can accumulate a row of drift, so one image and its fractional viewport point survive compatible
  layout acknowledgements. Native layout corrections retain this anchor; scrolling, query/revision
  publication, recovered dimensions, explicit navigation, detachment and disposal release it.
  Top/bottom intent takes precedence over image fractions; an initially non-scrollable gallery
  prefers the top, while a previously retained bottom survives temporary extent collapse. The
  transition admits this edge only for its captured scroll origin, and both renderers use it through
  the shared reflow contracts. Only the current generation may publish its final scroll origin.
  The screen composes this lifecycle with existing layout transitions; framework scrolling and bounded rendering remain
  the owning primitives.
- `library_synchronization_feedback.dart` owns the immutable message, detail, severity and existing
  manual-action projection of synchronization status. First-import and explicit-manual decisions
  remain distinct; blocked recovery takes precedence over automatic-progress explanations. A
  retry-wait total includes exhausted work and cannot establish a scheduled retry. The notification
  owner consumes this projection; the screen composes the existing user-confirmed update flow. The feedback
  policy neither restarts work nor converts ordinary persistence failures into scan authorization.
- `library_time_rail_presentation.dart` owns the retained painted rail frame and its Material input
  lifetime. It retains only projection/value data within compatible presentation context, never
  catalog authority or stale layout metrics. `LibraryTimeNavigation` retains seek scheduling and
  same-revision geometry; the screen retains gallery/viewer composition and the rail's width.
  A complete frame cannot be replaced by interim estimated geometry in the same query. The
  prior query's painted frame remains the motion origin while its replacement is pending under
  the same scroll controller. Input requires the painted frame, navigation projection and current
  timeline to share a query identity; old-query coordinates cannot navigate a new query. An initial
  estimate for a new query may replace the old frame without waiting for complete geometry. The
  presentation owner retires the old Material input when its navigation projection changes; the
  existing navigation cancellation path releases the matching unfinished gesture value and seek
  intent. New input resolves the painted month and within-month fraction through the current
  timeline projection, including when full layout geometry is pending. Removed months resolve to
  the nearest remaining month, while explicit endpoints retain top/bottom intent. Retained pixels
  never authorize scrolling a current layout. The orientation adapter binds callbacks to their
  mounted input lifetime instead of forwarding a retired callback into its replacement.
  A released date-navigation request retains its alignment through its resulting publication.
  Cancellation carries the old intent generation, so its deferred notification cannot retire a
  newer interaction.
- `library_sort_rail_projection.dart` maps admitted sections to actual gallery rows; an arrival
  range covers every row containing new items, including a row shared with earlier items. The
  annotation visibility owner gives highlighted labels precedence only during a collision,
  preserving the existing equal-priority date policy. Range drawing retains a bounded minimum
  stroke without moving the navigation anchors or adding Slider stops.
