# Transition performance without visual changes

The [roadmap](../roadmap.md#transition-performance) owns execution status. This work starts on
`codex/motion-performance` from `db8dce5f`, after the gallery group discovery integration.

## Outcome and fixed behavior

Reduce transition work and missed frames across Ame while preserving the accepted presentation.
The [state-motion decision](../architecture/0001-flutter-rust-desktop-stack.md#state-motion),
[design principles](../product/gallery-ui.md#design-principles-and-rationale) and individual surface
contracts remain authoritative. The earlier [motion adoption plan](material-motion.md) describes the
delivered effects; this follow-up optimizes their execution rather than adding another motion design.

Keep existing spring tokens, curves, durations, paths, scale, opacity timing, clipping, corners,
colors, spacing and final geometry. Preserve rapid reversal, cancellation, focus, pointer and keyboard
input, reduced motion, source retirement, selection, scroll anchors and return navigation. Keep
image-quality handoff and card-overlay timing, including partially clipped cards. No hidden delay,
shortened animation, disabled effect or lower image quality may stand in for reduced rendering cost.

Retain the pinned Flutter/material_ui versions, Skia selection and shared `AmeMotion` boundary.
Framework-owned menu, dialog, sheet and control motion stays framework-owned. Reuse public framework
primitives; no new motion package, renderer migration, feature redesign or device-specific effect
mode is included. Source files, catalog policy and file-operation authority are unchanged.

## Surface inventory

The inventory includes shared owners and every caller, not only the ordinary gallery. Confirm the
remaining implicit framework transitions during baseline collection.

| Surface | Existing owners to trace | Continuity to preserve |
| --- | --- | --- |
| Card/viewer entry and return | `LibraryViewerTransition`, `LibraryViewerImageFlight`, `LibraryViewerChrome`, `LibraryViewerCardOverlays` | Gallery, albums, smart albums, exact/similar review and member-tray entry; crop, corners, opaque travelling image and return target |
| Viewer controls and image changes | `LibraryViewerFilmstripSlot`, `LibraryImageReveal`, `LibraryViewerImage`, `LibraryViewerFade` | Filmstrip overlay geometry, image quality, adjacent navigation and controls |
| Gallery reflow and duplicate merging | `LibraryGalleryLayoutMotion`, `LibraryGalleryExitVisuals`, `LibraryGalleryScrollSurface` | Both layouts, visible-card bounds, background publication and stable timeline/viewport |
| Sidebar and folder disclosure | `LibrarySidebarTransition`, `LibrarySidebarContent`, `LibraryFolderDisclosure`, `LibrarySidebarReorderList` | Shared search/sidebar geometry, child expansion, selection retirement and drag collapse |
| State and selection controls | `AmeStateTransition`, `AmeTransitionIcon`, `AmeFlipTransition`, gallery header and selection actions | Icon/arrow motion, retained faces, action timing and all shared callers |
| Menus, dialogs, trays and notices | `AmePopupMenuSurface`, `AmeDialog`, shared feedback and overlay owners | Material transition, anchor, dismissal, focus return and unchanged layout |
| Loading and temporary content | `AmeLoadingIndicator`, `AmeProgressVisibility`, `LibraryVirtualGalleryPlaceholder`, task surfaces | Existing quiet interval, minimum visibility, fade, priority and loading semantics |
| Continuous input alongside transitions | `AmeScrollMotion`, `LibraryTimeNavigation`, `LibraryViewerZoom`, `LibraryComparisonView` | Existing wheel inertia, direct dragging, zoom/rotation and comparison synchronization |

## Implementation sequence

| Slice | Work | Exit evidence |
| --- | --- | --- |
| MP0 | Capture the current Profile baseline and complete the caller inventory | Reproducible interactions, measured frame budget, traces and visual reference for each surface family |
| MP1 | Optimize the measured highest-cost viewer, gallery and sidebar paths | Paired before/after frame and memory results; identical presentation and interruption behavior |
| MP2 | Address measured shared-control, overlay and loading costs; check continuous-input paths | Covered caller inventory; inactive effects stop work and caches/retained visuals stay bounded |
| MP3 | Repeat affected desktop interactions, audit, record results and deliver | Measured improvements, no observed behavioral/style regression, scoped checks and exact-head CI disposition |

Within MP1/MP2, prioritize measured cost. An already efficient surface needs a recorded assessment,
not a rewrite. Commit and push each independently reversible optimization after its affected checks;
do not combine unrelated rendering changes merely because they share this plan.

## Measurement and optimization method

Use the installed Windows toolchain serially. Profile-mode measurements compare the same machine,
renderer, window geometry, DPI, refresh rate, data and background workload. Separate cold image load,
warm transition and background-publication runs. Use the retained `local-primary` library for
read-only browsing; use the established fixture catalog for repeatable states. The deferred
constrained-device work is not reopened.

Reuse the FrameTiming, GC and memory measurement techniques in the existing retained-gallery
profiling owner. Its historical frozen-file guard and scroll-only scenario are not a substitute for
current transition coverage. Prefer a bounded development measurement path over a new general
benchmark framework. Raw traces remain under ignored `build/performance`; concise observations go
in the existing [motion acceptance record](../acceptance/material-motion.md).

Record UI/raster p50, p95, p99 and worst frame, frames over the display budget and consecutive missed
frames, transition latency, allocations/GC, peak and settled memory, and idle ticking. Pair timings
with actual visible frames: a missing image, missing effect or unpainted frame is a failure. Record
source/render-load differences separately from animation overhead.

Trace rebuild, layout, paint/compositing and image work before editing. Candidate repairs include
stable child subtrees, localized listeners, paint-only transforms where geometry permits them,
reuse of existing layers/paths, and retirement of inactive tickers. Apply repaint boundaries and
bounded caching only where traces justify the memory/compositing tradeoff. Do not cache entire
libraries, retain decoded originals indefinitely, or change blending/clipping to improve numbers.

## Verification and completion

Use the [UI verification map](../acceptance/quality/ui-verification.md#ui-workflow-verification) and
existing motion regressions. Add only a missing boundary check for a demonstrated defect; do not
assert every animation frame or duplicate framework tests. Compare opening/closing and interrupted
reversal at matched progress, including partially visible cards, both gallery layouts, enlarged
text, light/dark themes and reduced motion where affected. Check input and position during loading
or background publication, then confirm that retired surfaces release retained visual resources.

Every inventory family must have an assessment. Changed hot paths require repeatable improvement
and visual/behavior preservation; unchanged efficient paths require no speculative rewrite. Reuse
valid evidence for unaffected paths. Record unresolved data/query latency under R10 rather than
hiding it with altered animation.

References: [Material motion](https://m3.material.io/styles/motion/overview/how-it-works),
[Flutter profiling](https://docs.flutter.dev/perf/ui-performance), and
[Flutter rendering performance guidance](https://docs.flutter.dev/perf/best-practices).
