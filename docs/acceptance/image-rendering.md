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
