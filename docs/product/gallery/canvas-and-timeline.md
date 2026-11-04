# Gallery canvas and timeline

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Gallery canvas and timeline

- Default layout is a dense, aspect-preserving justified photo wall with date headers.
- Only the visible region and a bounded overscan area are rendered and decoded.
- The right-side time rail represents the complete filtered result set, not only loaded widgets.
- During updates in the same query, a complete rail remains visible until replacement geometry
  agrees with the gallery extent. Temporary window estimates cannot replace that complete frame;
  estimates remain available on first load and after a query change. Retained paint grants no
  stale navigation authority. New clicks, drags and keyboard input resolve the visible date through
  the current result, without waiting for complete layout geometry. Geometry replacement retires
  interrupted gestures and their position override, while ordinary gallery scrolling remains native
  and immediate.
- Material 3 defines a vertical standard `Slider` with an optional stops configuration. The Slider
  owns pointer, keyboard, focus, hover, handle, track, and semantic behavior; Ame must not recreate
  those behaviors in a parallel custom control.
- The repository-pinned Flutter 3.44.9 `Slider` implements the Material interaction and visual core
  but does not expose the specification's native vertical-orientation API. Ame therefore uses the
  already validated thin orientation adapter around the official `Slider`; replacing it with a
  hand-built gesture or semantics implementation requires new measured evidence and an ADR change.
  Endpoint spacing belongs to the track shape inside the full pointer surface. Outer Slider
  padding must not leave visible endpoint marks outside the clickable region.
- Material divisions are equidistant selectable stops and therefore cannot represent Ame's
  nonuniform month offsets without changing their semantics. Year/month marks are a narrow
  annotation layer driven by the complete-result timeline data; they do not own dragging, focus, or
  value selection.
- The gallery `ScrollController` is the sole authoritative scroll-position state. The Slider value
  is a projection of that global offset and writes back to the same controller; no second timeline
  position, independently synchronized scroll model, or page-local Slider state may be introduced.
- The complete query has one compact, revision-bound layout manifest containing stable ordering,
  orientation-corrected aspect ratios, date groups, and availability flags. Rust supplies it in
  bounded chunks, and Flutter stores compact typed data rather than full asset records, paths,
  metadata, or previews. Its memory cost and fallback representation must pass the gates in ADR
  0014.
- Orientation-corrected width and height are durable catalog evidence, not preview-cache metadata.
  They remain authoritative across restart, preview failure or cleanup, viewport resize, and an
  identity-proven rename or move. A temporarily unavailable source retains its last trustworthy
  dimensions together with separate availability evidence. Source replacement, incompatible media
  inspection, or a confirmed content edit requires reinspection and one atomic catalog revision;
  an authoritative location removal is the only normal path that removes its dimensions.
- One deterministic layout snapshot derives final row membership, item rectangles, cumulative row
  offsets, date anchors, and total extent from that manifest. Placeholder, failed-preview, and
  decoded states use the same rectangle. Preview completion or eviction with already-known
  dimensions must never recompose rows. When compatible decoding first recovers previously unknown
  dimensions, Ame freezes the actual viewport-center card and current logical range, coalesces that
  geometry evidence, and atomically replaces the snapshot with a pre-paint anchor correction.
  Reflow cannot widen its own recovery range, and later evidence remains deferred until native
  scrolling establishes a new idle range instead of chaining one relayout into another.
- Full `LibraryAsset` details are queried in bounded revision-safe keyset pages. The current
  controller is known to merge those pages into a growing `state.assets` list; Profile evidence
  determines its target-library cost before a high/low-watermark cache replaces that retained-list
  baseline. No single page or 160-item replacement window is the gallery's global presentation
  model.
- Preview readiness lives in an identity-keyed store outside layout state. Visible and near-
  viewport previews receive priority, obsolete generations cannot publish, and expensive decoding
  may be deferred during high-velocity scrolling. Within each priority, stationary demand begins at
  the actual center card and expands toward both sides; recovered geometry waits until native user
  scrolling is idle.
- A preview is a rebuildable, budgeted artifact whose identity includes stable location and source
  state, preview algorithm and version, orientation contract, and a bounded physical-pixel size
  bucket. Ame selects the smallest compatible bucket that satisfies the current display size and
  scale, rather than generating an unbounded variant for every logical pixel width. Preview
  absence, failure, staleness, regeneration, or eviction never changes durable dimensions.
- Wheel, touchpad, keyboard, accessibility, and ballistic movement remain native relative activity
  on Flutter's one `Scrollable` and do not enter an asynchronous intent queue. Slider drag, date
  click, restored position, source navigation, search navigation, and resize submit programmatic
  intents to one coordinator only where their writes require arbitration. Both paths preserve one
  query- and revision-bound logical viewport anchor rather than synchronizing several pixel offsets.
- Wheel and touchpad movement use native relative scrolling. Crossing a detail-page boundary
  prefetches bounded pages before and after the viewport without replacing the canvas. A cold page
  paints non-interactive card backgrounds in its final equal-height rectangles behind the shared
  loading line, as specified by ADR 0014. Detail and preview arrival preserve those rectangles and
  the precise position already reached by the rail in an unchanged layout.
- Mouse-wheel deceleration is shared across Ame scroll regions through `AmeScrollMotion` and
  `AmeScrollController`, including navigation, settings, details, tasks and the viewer filmstrip.
  Each region retains its own native scroll position. Wheel movement is immediate except for a
  90 ms ease-out tail of at most 24 logical pixels; repeated input preserves its total distance,
  and reversal, pointer-down, direct positioning or layout correction retires the old tail.
  Reduced-motion mode and precision touchpad input retain native movement. Vertical mouse-wheel
  input also scrolls horizontal regions; native horizontal/Shift input has first claim, and an
  inner region at its boundary leaves the event available to its parent.
  The pinned SDK's `ScrollController.createScrollPosition` and `PrimaryScrollController` provide
  the extension points. Material `ScrollBehavior` has no controller factory, and `ScrollPhysics`
  alone does not animate mouse-wheel jumps. `AmeScrollMotionTheme` carries a separate primary
  controller into captured Material routes without replacing their menus, focus or semantics.
  Framework controls that require their own private controllers retain those controllers.
- A cold thumbnail reveals its first decoded frame with a 120 ms ease-out opacity transition
  inside the existing rectangle. Synchronous cache hits appear immediately. Active or recent
  scrolling, disabled animations and initially inactive ticker scopes skip the reveal; further scrolling
  finishes an active reveal immediately. Muting an already visible surface preserves its current
  blend through viewer entry or exit. Image opacity owns the effect without a whole-wall layer,
  staggered delays, shimmer or changes to row geometry.
- Scroll-triggered detail paging uses one thin linear progress indicator at the top of the gallery.
  It overlays the gallery without resizing its viewport or intercepting gallery input. It does not
  add circular loaders to the photo wall or its boundaries.
- Slider drag writes the exact manifest-backed position at most once per rendered frame. Detail
  requests are latest-wins, bounded, cancellable or generation-guarded, and issued at a measured
  cadence outside the pointer-to-scroll critical path. Release promotes the final target, but it is
  not the first opportunity to prepare its detail page.
- A distant date click jumps directly to the resolved logical anchor rather than animating through
  the library. Cached details and previews appear immediately; otherwise the same final geometry
  remains scrollable behind the shared loading line while the target and guard pages load.
- Window resizing coalesces to one newest layout request per frame. The prior snapshot remains
  coherent until the replacement snapshot and its logical-anchor correction publish atomically.
  Preview decoding uses bounded width buckets, and obsolete intermediate-width computations cannot
  publish.
- Generic square placeholder slivers, aggregate-only unloaded geometry, settle-only wheel seeks,
  and interaction-specific replacement-window paths are temporary implementation debt and must be
  removed only after the ADR 0014 parity tests pass. They are no longer accepted target behavior.
- Month points and year labels use their real content scroll offsets. They are not evenly spaced:
  months containing more rendered gallery height occupy more rail distance, while dense anchors
  may cluster. Year-text collision handling never redistributes the scroll axis or month points;
  marker thinning uses only dot collisions because text has a separate column. The current-position
  indicator has a fixed visual height and does not represent the viewport height.
- Layout changes interpolate retained date labels, month points and the current-position line over
  180 ms. Sidebar changes, resized viewports, gallery layout changes and query-condition changes
  share this behavior. Wheel, slider and keyboard position input stays immediate; reduced motion
  skips the transition. The visual rail retains its prior frame while a changed query awaits its
  layout, then uses that frame as the transition origin. Prior-query ticks cannot accept input;
  navigation resumes when the painted frame and navigation projection belong to the current query.
- Year-label collision handling follows the annotated-scrollbar rule: retain the first collection
  label, remove colliding upper labels, and keep at least 4 px between visible labels.
- During drag, the active date label follows the current-position line, the gray hover preview is
  suppressed, and timeline marker dots remain visible rather than disappearing beneath the line.
- A year or month jump immediately changes the manifest-backed position and requests the bounded
  target detail page and guard pages. A stale response from an older query, request generation, or
  layout must not change the current gallery geometry or position.
- Source, search, date-sort, and duplicate-state changes recompute the time distribution.
- Unknown capture time has an explicit section and deterministic fallback ordering.
- No user-visible pagination or page transitions are introduced.
- Opening an image and returning restores its prior gallery item and scroll position.
