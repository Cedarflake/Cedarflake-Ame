# Image rendering verification

Date: 2026-10-22

Scope: original-image sampling, cropped gallery previews, card density and equal-height row packing.
The [gallery contract](../product/gallery-ui.md) owns the behavior. No schema, native decoder,
source-read authority, dependency or licensing boundary changes.

## Focused regression evidence

The canonical Flutter test entrypoint passes 125 cases across the following files. Tests ran
serially using the installed Flutter 3.44.9 SDK. The final row-filling correction reran the affected
packing, manifest and resize cases; unchanged viewer and preview evidence was retained.

| Test file stem | Cases | Boundary |
| --- | ---: | --- |
| `justified_gallery_layout_test` | 7 | Neighbor packing, filled intermediate rows, width floors and sparse final rows |
| `library_gallery_layout_snapshot_test` | 8 | Manifest/loaded geometry agreement, dimensions and anchor gaps |
| `library_gallery_layout_test` | 6 | Item ordinals, grouping and uniform square sizes |
| `library_gallery_resize_test` | 20 | Reflow, shape/density changes and retained logical positions |
| `library_gallery_size_continuity_test` | 20 | Size round trips, edge intent and transitions between scrollable and short galleries |
| `unified_library_dimensions_test` | 4 | Recovered dimensions, timeline-demand replacement and retained viewport position |
| `library_image_scaling_test` | 6 | Actual reduced-image pixels, cropped portrait pixels and bounded size requests in both layouts |
| `library_image_viewer_test` | 11 | Zoom input, sampling changes and reuse of the decoded original |
| `library_viewer_image_test` | 7 | Original/preview loading and source lifetime |
| `library_viewer_zoom_geometry_test` | 5 | Fit, zoom bounds and physical scale at display densities 1, 1.5 and 2 |
| `library_photo_tile_test` | 14 | Preview display, loading, failure and retry behavior |
| `library_preview_coordinator_test` | 17 | Preview demand, cache-size upgrades and publication ownership |

The reduction fixture renders alternating three-pixel black and four-pixel white stripes at 1/8
size. Fixed bicubic sampling produced black/white aliasing; mipmapped reduction passes the bounded
gray-pixel assertion. Cropped portrait checks retain the central image region and request the
1024-pixel bucket rather than exposing the image's top and bottom.

The short-gallery fixtures start with small cards so that the ten-item gallery fits before enlarging
it. Both layouts and manifest modes retain the original assertions for top position, end intent and
zero scroll extent after shrinking. These 20 cases pass after adapting to the larger card sizes.

The deep-timeline fixture uses the rotated slider's decreasing value to seek toward the end of the
gallery. Its four-case suite passes with the existing queue-capacity and delayed dimension-publication
assertions; no production navigation behavior changes.

## Windows client evidence

The rebuilt Debug application reused the 72 generated images in `build/test/templates` with an
isolated derived catalog. At an 816-logical-pixel gallery width, square small/medium/large choices
produced six/four/three columns, approximately 131/200/268 pixels wide. Shape and density changes
retained the visible item. Equal-height rows retained their selected height and the existing crop.
The reported intermediate panorama rows were revisited after reconstructing the layout; each filled
the full 816-pixel width. A 1500 by 12000 original loaded from its thumbnail and displayed at fit
and enlarged scales; no loading indicator remained after the original frame appeared.

The owned application exited normally. The original preference file was restored and its SHA-256
matched the pre-run backup. Original user catalogs and media were outside this verification profile.
Disposable derived storage was removed after process retirement and preference restoration.

## Ownership and limits

Production line counts: row packing 160, manifest layout 441, gallery wall 1520, photo tile 460,
cover-size policy 32, viewer image 236, viewer 393, and zoom geometry 84. These owners contain no
inline tests. Changed dedicated test files contain 131, 266, 230, 273, 814, 137, 306 and 798 lines for
packing, manifest layout, loaded layout, image scaling, viewer, zoom geometry, size continuity and
recovered dimensions respectively.
Preview-size policy and shared row packing own the new decisions; the gallery/viewer facades retain
their lifetimes.

Preview requests retain the existing 1024-pixel maximum and bounded scheduling/cache rules. Extreme
aspect ratios or high-density displays can still exceed the detail available in that bucket. This
client check establishes rendering and interactions, not a new low-end-device performance result.
The normal push-triggered hosted workflow owns full lint, Daily, integration and Release verification.

## Compact square card controls — 2026-10-28

The selection mark and duplicate action use matching 28-pixel rounded squares, each inside a
transparent 40-pixel target. `AmeSelectionCheckbox` retains Flutter's input, focus, checked semantics
and state fill, replacing its painted tick with the existing Material Symbols check. `FittedBox`
scales the framework checkbox; `ClipRect` bounds its hover and focus response. The duplicate action
retains its IconButton and existing stacked-image symbol. Neither control has a separate outer fill.
The pinned Flutter 3.47.8 source exposes `Checkbox.width` but no mark-size parameter; its
[Checkbox API](https://api.flutter.dev/flutter/material/Checkbox-class.html) was checked alongside
the [Material component entry](https://m3.material.io/components/checkbox/overview), whose content
requires JavaScript. Composing the installed Checkbox and IconButton preserves their input ownership
without a new control package or custom gesture/semantics implementation.

Scoped analysis and nine focused widget checks pass, covering edge clicks, keyboard toggling,
checked semantics, retained card bounds at 1x/2x text and duplicate-badge publication lifetime.
The Windows Debug build completed in 58.5 seconds. The retained isolated test profile verifies both
square controls, selection and deselection counts, the library check symbol, opening the duplicate
locations tray and Escape dismissal with the selected item retained. No original source media or
user catalog was changed.
Hosted checks remain owned by the published implementation commit.

## Time navigation after catalog publication — 2026-11-05

The current Windows client reproduced a date click that showed loading and returned to the original
November 2026 view. The shared gallery snapshot skipped publication when geometry matched, including
when its catalog identity was obsolete. Reuse now requires a snapshot matching the current query and
revision. A full-screen regression with an already available replacement manifest failed against the
old guard (58 pixels, still at the beginning) and passes with current geometry admitted. Six gallery
cases and 21 date-publication/input-lifetime cases pass. The screen remains the existing owner; this
changes its reuse condition without adding a navigation policy.
The affected screen has 1,904 production lines, no inline tests; its dedicated timeline file has 649 lines.

The rebuilt client still reproduced the symptom. Runtime tracing then identified exact-analysis
retirement refreshing the catalog while a date request was active, advancing the publication
generation and cancelling that request. Exact-analysis publication now retains its pending refresh
until the existing browse admission allows it. Its disposal cancels the scheduled retry. All nine
publication cases pass, including blocked browsing, subsequent admission and disposal while waiting.
The real ordinary gallery subsequently reached January 2025 and retained that position through later
background updates. Temporary tracing was removed and the clean Debug client rebuilt. Subsequent
smart-album checks reached a later date but also reproduced a jump returning to the former position
while classification continued. Runtime tracing identified the remaining gap: the date result was
published before its layout resolver, so the next refresh captured no anchor and loaded index zero
after the rail had aligned to index 371. Query position capture now falls back to the admitted date
result's loaded asset and global offset until layout can provide a position. Query, timeline and
revision must agree; stale or unloaded targets cannot provide that fallback. All twenty transition
cases pass, including pre-layout capture and stale-target rejection. The mapping owner has 42
production lines, the transition 284, and its dedicated fixture 482; neither owner has inline tests.
The clean Debug client retained September and December 2025 in the manga smart album through
continuing classification updates, including a count increase from 2,825 to 3,196. After switching
to the ordinary gallery, March and then September 2025 also loaded and remained at their targets
through subsequent updates. No diagnostic tracing remains. Exact-commit hosted checks remain pending.

## Filmstrip page continuity — 2026-11-05

Prepending and trimming the retained window previously painted one frame at the wrong local offset;
the following `jumpTo` cancelled the native scrollbar drag. The shared filmstrip now rebases through
`ScrollPosition.correctBy` inside its `LayoutBuilder`, before the child viewport lays out. The pinned
SDK performs this correction without notifying listeners or retiring the drag; Material still owns
the thumb, keyboard, wheel and touchpad interaction. Two Windows pointer regressions fail on the old
code and retain the same tile rectangle on the first published frame with the fix, then continue
scrolling without releasing the mouse. All 16 affected filmstrip, preview-frame and scrollbar cases pass.

The only scrolling-strip composition is the shared gallery viewer, covering the ordinary library,
favorites, manual albums and smart albums. Duplicate review reuses the thumbnail only; its static
cards do not own a filmstrip drag. Paging limits, preview fade/retention and source identity remain
unchanged. Rebuilt-client interactions and exact-commit hosted checks remain pending.
The strip has 299 production lines, no inline tests; its dedicated interaction file has 542 lines.

## Filmstrip boundary paging — 2026-11-05

The live client displayed repeated loading at the retained-window edge. The strip treated an
unchanged page as success and immediately requested it again. Background exact-analysis and
synchronization refresh could also retire a page read; their admission now waits for page reads.
The existing query transition previously anchored an open viewer to its main image even after the
strip had been independently scrolled. It now accepts the visible strip's stable anchor, preserving
its fractional offset while keeping the main-image selection and underlying gallery position separate.

`LibraryFilmstripPaging` owns request retirement, progress validation and retry. No-progress results
stop automatic demand. `LibraryFilmstripPosition` owns the strip anchor through refresh; native
Scrollbar input and layout correction remain in use. Loading/retry occupies a separate card on the
requested side, outside image bounds. Eleven strip cases, eighteen query-transition cases and eleven
refresh cases pass, including left/right retry, held-thumb paging, viewer independence and deferred
publication. The clean Windows Debug client rebuilt successfully. In a real smart album, dragging
to either edge displayed the loading card on that side, published another page and retired the
indicator; the main image remained unchanged through repeated classification updates. Hosted results
remain pending. The strip, paging and position owners contain 339, 105 and 50 production lines,
with no inline tests; the strip interaction file contains 663 dedicated test lines.

The first card composition also inset the scrollbar at the loading edge. The scrollbar now wraps
the complete row, keeping its native track and both end buttons across the full strip width while
the card occupies only the thumbnail area. Both directional cases assert unchanged end-button
bounds during loading; all eleven strip cases pass. The rebuilt ordinary-gallery client displayed
the right loading card with both arrows retained at the outer edges.
