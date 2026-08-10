# ADR 0010: WinUI-compatible annotated gallery scrollbar behavior

- Status: Accepted
- Date: 2026-08-08
- Last amended: 2026-08-09
- Supersedes: the earlier dual-level navigation and annotation-cluster experiments

## Context

Earlier time-navigation experiments introduced product-specific redistribution, clustered date
markers, hover menus, and a second fine-scroll surface. These additions diverged from Microsoft
Photos, created ambiguous interaction states, and repeatedly produced visual and runtime defects.

Microsoft's public `AnnotatedScrollBar` specification defines a simpler model: one rail is connected
to the scrollable container, labels are placed at explicit scroll offsets, a fixed-size passive thumb
shows the current position, pointer hover shows a passive preview thumb and detail label, and
colliding labels are hidden. The rail itself handles click and drag input. Arrow buttons move by a
small scroll change.

Flutter Material does not provide an annotated vertical scrollbar. The repository-pinned Flutter
SDK does provide `Slider` pointer, keyboard, focus, and semantics behavior that can own rail input
without introducing a second scroll state.

## Decision

### Scroll ownership and mapping

- The gallery owns one `ScrollController`, whose attached `ScrollPosition` is the sole current
  scroll state.
- The catalog's complete `LibraryTimeline` establishes the rail's full range on the first render.
  Loading, prepending, or appending a bounded asset window must not extend or shrink that range.
- The presentation derives one stable virtual gallery extent from the complete timeline, viewport,
  selected layout, and thumbnail size. Unloaded leading and trailing ranges are represented by lazy,
  statically painted placeholder slivers; they do not allocate one widget per absent asset.
- Each loaded window records its global starting item offset. Local gallery movement is projected
  through that offset into the complete timeline, so appending another page does not move the
  current rail position backward.
- Within the loaded window, item-to-scroll conversion uses the rendered gallery row offsets from
  the current layout metrics. It must not approximate a target as an item-count percentage of the
  total pixel extent because justified rows contain different numbers of images.
- Gallery offsets remain the logical source of truth. When persistent year labels would overlap,
  the rail derives one monotonic reversible visual projection that expands only the conflicting
  annotation intervals. Marker dots, labels, the current-position line, hover previews, pointer
  input, and Slider semantics all use that same projection or its inverse.
- Clicking or dragging within the materialized window writes the corresponding offset directly to
  the same gallery position. Pointer updates are coalesced to at most one gallery jump per rendered
  frame so duplicate events cannot force repeated layout work inside one frame.
- Direct manipulation outside the materialized window writes the projected global gallery offset
  immediately, so the viewport and primary line move in the same frame even when content is not yet
  available. Direct manipulation never starts a catalog query or replaces the bounded asset window.
  Releasing the pointer commits only the final target through one time-anchor request. A completed
  request fills the existing virtual range instead of resetting the scroll position or changing its
  total extent.
- Normal wheel, touchpad, keyboard, and programmatic scrolling update the same virtual coordinate
  immediately. Entering an unloaded range records only the latest target while scrolling remains
  active; one bounded request is submitted after a short idle interval. Intermediate positions do
  not replace the asset window. No second controller or scrub-only scroll state exists; the passive
  current-position line follows that controller.
- Viewport-size changes recompute the bounded layout without publishing `setState` through the
  complete unified screen. A narrow layout snapshot updates the time rail and scroll anchoring, and
  unchanged anchor offsets do not write back to the scroll position. Preview decode widths use a
  small set of bounded cache buckets so pixel-by-pixel window resizing cannot create a new image-
  cache identity and decode request for every intermediate width.
- Outside the minimum label-spacing adjustment, time density remains proportional to the complete
  catalog projection. Square layouts use the complete item count and equal-height layouts use the
  catalog's bounded aspect-ratio aggregate, so a denser period still owns more rail extent from the
  initial render.

### Flutter control composition

- A controlled Material `Slider` owns rail pointer, keyboard, focus, and semantics behavior. Its
  visual thumb and track are hidden.
- A fixed-height primary-color line visualizes the real current position. It is passive and does not
  own a second drag state.
- Pointer hover shows a fixed-height neutral preview line and the detail label for the corresponding
  gallery offset while the rail is idle. Starting direct manipulation removes the neutral preview;
  during the drag, one detail label follows the primary current-position line instead. Ending the
  drag clears that label and does not leave a stale neutral line behind.
- First-party Material `IconButton` and `Icon` controls provide the endpoint arrows. In production,
  each activation moves by one gallery photo row plus its spacing.

### Annotations

- Each date annotation keeps its exact logical projected scroll offset. Year labels are shown at
  the first date of a year and may move only enough to maintain the minimum visual gap. The
  resulting monotonic visual projection moves the corresponding marker and every interactive
  position consistently; inverse mapping restores the exact logical gallery offset before a seek
  is committed. Remaining dates may use small dot markers. Every visible annotation retains its
  standard dot when the current-position line intersects it, including the unknown-date annotation.
  The primary line is drawn beneath annotation dots so direct manipulation never makes the nearest
  visible point disappear.
- An annotation outside the rail bounds is hidden. Visible annotations must also retain a small
  minimum visual gap; merely avoiding geometric overlap is not sufficient.
- Collision resolution examines annotations from the bottom upward. The lower annotation remains
  visible; the first annotation is also preserved unless it collides directly with the final
  annotation.
- When display avoidance cannot fit the available rail, conflicting annotations are hidden rather
  than merged, counted, or replaced with another control.
- There are no annotation clusters, badges, red counters, three-dot markers, hover menus, or date
  selection overlays.

### Reference boundary

- Observable behavior is based on Microsoft's public `AnnotatedScrollBar` specification and the
  WinUI Gallery example at the fixed local reference commits
  `e551a456523117071150cb66290bdab7c485b1b1` and
  `3669519356c67f1376152c33ed8ea45003a91f3a`.
- Ame implements the behavior independently in Dart. WinUI source, tests, XAML, comments, assets,
  identifiers, and implementation structure are not copied into Ame.
- The external reference repositories remain outside Ame's Git history.

## Consequences and risks

- Dense or short time ranges may have fewer visible annotations when the minimum spacing cannot fit
  at all. Their logical positions remain available through continuous hover, click, and drag input
  instead of a separate menu.
- Cross-window dragging moves the global gallery canvas at display cadence without catalog or
  Riverpod window churn. Source-backed images appear after the final target is committed. Static
  placeholders make the unloaded state explicit without an animated skeleton competing with
  scrolling work.
- Static placeholders are an interim parity gap: unlike Lap and WinUI, Ame does not yet retain a
  complete lightweight layout index that can keep source-backed items visible throughout a
  cross-window drag. Closing that gap requires a compact query-wide identity, aspect-ratio, date,
  and row-geometry index while asset details and thumbnails remain bounded and lazy. It must not be
  addressed by materializing every full asset record or by restarting catalog queries during drag.
- The global extent is an aggregate projection. Exact row geometry is used inside a materialized
  window, while unloaded equal-height ranges remain approximate until their bounded window is
  loaded. A persisted global row index may improve pre-load precision later without changing the
  single-controller contract.
- Flutter's `Slider` is an interaction adapter rather than the visible control. Changes to the
  pinned SDK require focused verification of its rotated hit testing, keyboard direction, focus,
  and semantics behavior.
- Source media remains read-only; this decision changes presentation and navigation only.

## Validation evidence

- The official Material 3 Slider catalog remains the selected interaction model. The repository-
  pinned Flutter 3.44.9 `Slider` source exposes `onChangeStart`, controlled `onChanged`, and
  `onChangeEnd`, allowing Ame to distinguish idle hover from direct manipulation without replacing
  framework-owned gesture behavior.
- The local Microsoft UI Xaml specification documents explicit scroll-offset labels, passive fixed
  thumb geometry, rail click and drag, hover detail labels, small-change arrows, and collision
  hiding.
- The local WinUI Gallery example derives label offsets from rendered row geometry and connects the
  annotated rail directly to the scroll view.
- Pure tests cover the catalog's linear projection, reversible visual projection, and deterministic
  collision fallback.
- Widget tests cover passive current and hover lines, a drag-following date label without a stale
  hover line, persistent points beneath the primary line, absence of cluster menus, Slider release
  without date snapping, frame-coalesced gallery-controller movement, immediate global placeholder
  movement, no catalog seek during direct manipulation, one final seek after pointer release, and
  one final cross-window seek only after ordinary gallery scrolling settles.
- Pure presentation tests cover stable preview decode buckets across small viewport-width changes.
- Pure geometry tests verify that moving a bounded asset window does not change the complete scroll
  range and that fully materialized results retain their exact rendered extent.
- Application tests verify the smaller timeline window and that a replacement window removes only
  obsolete pending preview work rather than clearing the complete preview queue.
- Windows runtime inspection remains required for final visual and interaction acceptance.

## Replacement strategy

A future first-party Flutter annotated scrollbar may replace the presentation adapter. It must
preserve the single-controller contract, linear real-offset mapping, passive current thumb, hover
detail behavior, and collision hiding. Reintroducing annotation redistribution, clusters, menus, or
a second scroll position requires a new user-approved architecture decision.
