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
unchanged. Subsequent rebuilt-client interactions are recorded under
[boundary paging](#filmstrip-boundary-paging--2026-11-05); hosted checks retain their own commit scope.
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
Scrollbar input and layout correction remain in use. Loading/retry is a temporary first or last item
in the same virtualized list, with the thumbnail slot size and spacing. It scrolls with the images
without shrinking the viewport. The leading slot participates in anchor restoration and visible
preview bounds; a missing selection reveals its inline feedback after the lazy list publishes the
extended scroll range. The native track and both end buttons retain the full strip width.

Selection changes retain edge-page requests and their failure feedback; only a selection-location
request is retired by a new selection. The visible anchor survives selection changes and renewed
catalog cursors. Retry admits the replacement request before clearing its failure, keeping the same
card slot; removing that slot for one frame previously produced a negative left offset and rebound.

All eleven strip cases pass, including both directions of card movement, selection during pending
paging and failed-page feedback, exact thumbnail bounds through cursor renewal and retry, held-thumb
paging and missing-selection feedback. The extended existing cases failed before the repair on a
missing right loading card and duplicate left-page requests. Formatting and strict analysis of the
three changed Dart files pass. Earlier eighteen query-transition and
eleven refresh cases retain their scope. In the live ordinary-gallery client, native-thumb dragging
displayed the inline card at each loading edge and published another page; scrolling away showed
the full-width thumbnail list, and the main image remained unchanged. After rebuilding the client,
selecting a visible thumbnail during right and left paging retained its screen position through
page publication in the ordinary gallery. The manga smart album retained the selected thumbnail
through right paging and loaded the subsequent previews. Automatic synchronization remained healthy.
The separately reported
development-output interruption is covered by the [native diagnostic repair](desktop-lifecycle-continuity.md#disconnected-development-diagnostics--2026-11-05).
The strip, paging and position owners contain 372, 108 and 57 production lines, with no inline tests;
the strip interaction file contains 730 dedicated test lines. Current-head hosted checks remain
separate.

Hosted run `42001105569` at `feaf501c` passed Flutter, native and release workloads but failed Static
on a missing conditional block in the strip fixture. Adding that block preserves the test behavior;
the affected file passes formatting and strict Dart analysis. Current-head CI remains separate.

## Viewing controls and rotation — 2026-11-08

The rebuilt Debug client displays the image-action group at the upper right, with a short divider
before the window controls. Organization mode also displays the current image's category. The
existing viewer regression run passed twelve cases, including action continuity through full screen
and rotation with navigation. Desktop rotation preserved 100% actual size, adapted automatic fit
to the oriented image, and reset when changing images. The checked original image retained its
SHA-256. The final toolbar uses the installed Material Symbols 90-degree clockwise rotation icon;
the rebuilt client displays it and the updated organization-mode controls.

## Gallery navigation ownership — 2026-11-08

Explicit rail input now retires earlier query-position reads and pending layout restoration.
Date loading shares user-query admission, so a committed background refresh waits for the date
result. Paging and prefetch recovery remain bound to their original user intent and release when
another query or date request replaces it. Native layout clamping is distinguished from user
scrolling; selecting another collection resets the scroll position and retains the start edge
through layout publication, including its first date heading.

Three regressions cover an older refresh completing after consecutive mouse clicks, switching
from the middle of a longer smart album to a shorter album with a delayed manifest, and a committed
refresh arriving during explicit date resolution. Before repair, the first case restored the old
position, the category case stopped at 1496 px, and live background refresh retired date reads.
The twelve position cases, thirty-nine application scheduling/date cases and fifty-three existing
scroll/query-transition/timeline cases passed in focused runs; no full local suite was required.

The rebuilt Debug client on `local-primary` accepted its first rail click from November 2026 to
April 2025. Consecutive date targets retained the final selection through background updates.
Switching from the December 2025 portion of 8444 illustrations to the ten-item photo category
displayed its complete first heading, June 25, 2026, with the marker at the start. Validation used
the existing library and browsing operations only. Hosted checks remain scoped to the pushed commit.

The refresh coordinator, viewport facade, query-position owner and screen contain 176, 1312, 289
and 1986 production lines respectively, with no inline tests. The affected dedicated date and
position test files contain 458 and 396 lines; orchestration changes reuse the existing owners.

### Manual scrolling during classification updates

A retained complete rail rejected gallery-origin positions while a new revision used estimated
layout. The reproduced wheel case moved content from 1800 to 2600 px while the marker remained at
0.4091. Current user scrolling now carries interaction intent through that retained projection.
Paging previously inspected the position before wheel movement and dropped demand during a busy
read. The dedicated page-demand owner checks the resulting position, resumes after admission and
limits attempts to one per cursor/revision; failures retain the existing explicit retry behavior.
Its widget lifetime survives replacement between estimated and complete gallery layouts.

The focused publication, input and integrated timeline run passed 49 cases. The added cases cover
retained-marker movement, crossing a loading threshold in one wheel event, and continuation after
busy paging or query refresh without another event or an unchanged-cursor retry loop. The rebuilt
Debug client on `local-primary` followed manual scrolling from November 2026 through May and January
2026 to December 2025 while the manga category grew from 2450 to over 3300 images. Subsequent page
content appeared without extra input to trigger loading. The earliest-date click reached August
2022, and subsequent upward scrolling moved the marker with the displayed earlier rows. Verification
used browsing only.

The page-demand owner, interaction widget, rail navigation and gallery wall contain 71, 160, 827 and
1555 production lines, with no inline tests; the two affected dedicated test files contain 319 and
556 lines. The wall change only preserves the interaction owner's key across its renderers.
The preceding `c34a94d2` hosted run `42470075424` passed Flutter tests but encountered HTTP 504
downloads in Static and the three Windows build/integration jobs. Current-head hosted results remain
separate from that infrastructure failure and the local checks above.

### Continuous timeline dragging and coherent layout publication

The `codex/gallery-navigation-snapshots` repair is based on `c1797887` and implements the
[navigation stability plan](../plans/gallery-navigation-stability.md). Earlier click and wheel
evidence did not cover the subsequently reproduced held-drag failure. Initial Windows Debug
interaction evidence below used the 2026-11-08 21:40 build; the subsequent query-cost checks used
the 2026-11-09 Debug build accompanying this record.

The local-primary catalog contained 63,072 locations and 47,828 displayed duplicate representatives.
Both equal-height and square layouts moved during native mouse drags and displayed shaped unloaded
cards immediately at distant targets. Post-frame observations recorded changed scroll pixels while
the Material pointer was still held. The native input API supplies short drags, not a configurable
multi-second hold; the operator separately reported normal interaction during the same validation.
Quick distant-and-return gestures retained the returned images after the old read completed.

The rebuilt 2026-11-09 client repeated equal-height and square distant dragging: shaped placeholders
appeared immediately, and filled cards retained the same clipped row boundaries. The real profile
contained 63,073 locations and 47,829 displayed items. Smart navigation displayed its initial spinner;
the album landing displayed the shared loading line until the chosen category arrived. Switching
from the middle of manga to photos began at the photo category's first date. Returning to albums
retained the category list while classification increased its counts. Gallery, favorites, duplicate
groups, organization entry/exit and the empty ignored queue completed their visible transitions.
Leaving organization restored the prior browsing position. Verification changed no organization
decisions or source files, restored the original equal-height preference, and closed the client
normally. Native input still uses the short-drag limitation described above.

Image arrival initially exposed a second position error: a background refresh began between a new
snapshot's publication and its first frame, rejected the previous revision's displayed position and
restored a row-start fallback. The final client preserves the displayed location and row fraction
through that gap. Square cards retained the same clipped first row before and after loading;
equal-height cards retained their row boundaries through loading and later classification updates.
Switching from the December 2025 portion of the growing manga category to the 218-image photo
category displayed its first July 22, 2026 heading and the rail's start position. Wheel movement
continued through visible rows. These checks used browsing and derived preview reads only.

The initial native detail reads took roughly 3–8 seconds under concurrent classification. Follow-up
review found repeated membership scans across the page, timeline and layout, plus an unindexed
bounded-detail join. The transaction-local selection now supplies navigation and geometry once;
detail reads use the existing location index and retain both root and location identity.

Serial native-bridge measurements on the established local-primary profile retained 47,829 displayed
items. Full gallery snapshots took 1.33–1.39 seconds after session initialization, versus 5.25–5.43
seconds at `e7655cbe`. A switch sequence measured 2.90 seconds on its first gallery read and 1.36
seconds on return. Five growing smart categories took 2.03–2.32 seconds after shared group counting,
versus 3.18–3.43 seconds immediately before that change. The pending organization queue took 2.05
seconds, empty deferred/ignored queues 0.51–0.53 seconds, and favorites 0.49 seconds. These are native
read timings, not input-to-paint or cold-machine startup measurements; classification membership
grew during acceptance. No manual albums existed in the real profile; their contracts retain
fixture coverage without creating user data for this check.

Fresh-process reads with warm system caches took 4.60 seconds for a page and 4.96 seconds for a full
snapshot. Earlier loaded-workstation samples ranged from 10.77 to 13.92 seconds. Session preparation
still contributes visible startup latency; this change does not bypass catalog validation or claim
instant startup. Cold detail arrival still waits for reads, while geometry and placeholders remain
interactive. The inline layout retains its 64 MiB item budget; larger block-protocol queries were
not accepted here.

The no-compatible-fingerprint regression uses the real migrated schema and compares a fixed
32-item page across 256 and 4,096 locations. The grouped-only implementation required 31,952 and
485,072 SQLite VM steps; the restored fast path passes the unchanged bounded-cost assertion.
An isolated 63,072-row comparison likewise dropped from about 6.06 million to 16,000 steps. The
existing category snapshot cost assertion also passes unchanged after shared selection, including
name ordering. These isolated costs are not workstation UI timings.

Focused checks cover coherent reads, duplicate evidence, name/date sections, category costs,
held gestures, root-removal positions and concurrent update publication. Tests for the retired
separate removal timeline were replaced by atomic refresh and stale-result coverage. A synthetic
loading-flag case was removed; blocked-read interaction cases retain loading behavior coverage.
Temporary product probes were removed. Hosted results must be evaluated for the published commit.

Pre-merge review and [the first hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42487135679)
exposed missing distant-detail demand in provisional equal-height layouts and an unretired paging
timer. Both layouts now request the estimated visible range, offscreen detail windows supply no
false visible anchor, and disposal cancels queued paging. The affected interaction, resize and late
navigation checks pass locally with their original visible-position requirements; fixtures now use
the global loaded-window offset instead of assuming that each detail window starts at zero.
The next hosted run exposed repeated demand on partially covered viewports and a stale interaction
value after release. Passive disjoint demand now requires no visible loaded anchor, and complete
geometry returns the rail to the actual scroll position after release. Root removal retains an
unaffected root's displayed geometry until its coherent replacement arrives.

One attached verification process exited after about 21 minutes with `0xC000013A`
(`STATUS_CONTROL_C_EXIT`), captured by a retained process handle. No application crash event was
found. The source of the console termination remains unconfirmed; it is not recorded as a repaired
application crash. The independently launched final Debug client remained available during the
subsequent interaction checks. Raw process and timing logs remain ignored under `build`.

The viewport facade, screen, wall, navigation widget and position owner contain 1,209, 1,995, 1,559,
772 and 176 production lines respectively. The seek scheduler contains 96 lines; native compact
layout and read-projection owners contain 312 and 265, with a 2,986-line SQLite facade. The committed
root-removal projection contains 51 lines. These owners contain no inline tests. Dedicated query
snapshot, root-removal position, native snapshot and query-index files contain 388, 332, 195 and 135
lines. The changes retain existing query, geometry and publication owners.
The interaction and viewport-feedback owners contain 163 and 284 production lines; their dedicated
interaction, resize and pending-navigation checks contain 319, 1,332 and 458 lines, without inline tests.
