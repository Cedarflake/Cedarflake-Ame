# Official Material motion adoption

The [roadmap](../roadmap.md#material-motion-adoption) owns disposition and the
[execution plan](../plans/material-motion.md) owns scope and order.

## Transition performance — 2026-11-13

The [performance plan](../plans/motion-performance.md) preserves the delivered motion. Baseline
`532fbc21`, intermediate render changes through `2e0aab44`, and the final buffer/layout changes
accompanying this record were measured with Flutter 3.47.8 Profile, Skia, a 165-Hz
display and the same 1,269 × 714 window. `local-primary` contained 63,170 locations / 47,926 merged
entries. No source-file operation was run. Raw frame/GC/timeline captures are ignored under
`build/performance`; the client used the production entrypoint with in-memory view preferences.

`LibraryGalleryMotionItem` now builds its content from layout constraints and retains it while only
position changes. Existing geometry, opacity, exit-image lifetime and scroll-interruption owners
are unchanged. The existing duplicate-publication regression checks zero further tile builds on
its translation frame, including manual and background publication; resize, reversal, action
retirement and reduced-motion checks also pass (six retained cases). The owner has 364 production
lines, no inline tests, and 343 dedicated test lines.

`LibrarySidebarContent` now preserves destination layout constraints and applies the existing
alignment and clip during painting. The installed framework's `LayoutBuilder` schedules layout for
an animated builder in its build scope even when that builder only changes a transform; the
render-object listener removes that repeated layout. It updates paint and semantic geometry, uses
the same offset for hit testing, reuses the framework clip layer, and detaches its listener on
retirement. No motion token, input policy or source/caching lifetime changed.

A temporary old/new renderer comparison matched painted pixels and child bounds for both used
alignments, both directions and four progress values (16 comparisons). The retained boundary case
checks movement without another layout, clipped hit rejection and reverse movement. The existing
sidebar/gallery checks passed at 1x/2x text, three widths, interrupted reversal and reduced motion;
the combined affected run passed 19 cases. Scoped formatting and analysis passed. This sidebar
owner has 159 production lines, no inline tests and 193 dedicated transition-test lines.

CPU samples traced periodic UI pauses to exact comparison of compact manifest columns during
background publication. `galleryBuffersEqual` compares eight bytes at a time and checks the remaining
bytes without copying or hashing; query, identity, dates, dimensions and flags retain exact equality.
After this change, idle-refresh UI p95/max fell from 25.09/27.91 to 4.43/5.07 ms in 23.35/37.28-second
captures. Remaining sidebar admission pauses were traced to row reconstruction. Normalizing dates
only when the raw date changes and returning the stored aspect ratio directly remove repeated work
without changing packing, grouping, scroll anchors or asynchronous publication.

Buffer views, final-byte mismatches, row reuse, changed geometry and malformed-date grouping passed
their focused checks. The final manifest and row-layout run passed 44 retained cases; the separate
buffer check passed two cases. Formatting and scoped analysis passed. The buffer owner has 28
production/37 dedicated-test lines; manifest storage has 782/277 and the snapshot owner 485/401,
with no inline tests. No controller, bridge or catalog rule was extended.

| Warm sidebar burst | Before UI p50 / p95 / p99 / max (ms) | Final UI p50 / p95 / p99 / max (ms) | UI frames over 6.06 ms, before → final |
| --- | --- | --- | --- |
| Equal-height collapse | 1.91 / 3.15 / 9.47 / 25.82 | 1.75 / 3.11 / 6.61 / 10.04 | 2 of 57 → 2 of 66 |
| Equal-height expand | 1.75 / 2.59 / 22.26 / 29.56 | 1.42 / 2.65 / 8.77 / 10.12 | 2 of 66 → 2 of 65 |
| Grid collapse | 1.48 / 8.48 / 24.34 / 24.34 | 1.81 / 4.49 / 6.07 / 8.22 | 5 of 49 → 2 of 69 |
| Grid expand | 1.38 / 5.80 / 20.19 / 20.19 | 1.11 / 3.65 / 7.00 / 8.85 | 2 of 46 → 2 of 60 |

Equal-height comparisons start at `532fbc21`; grid comparisons start at `2e0aab44`, before the
buffer/layout calculation changes. Bursts contain at least 25 frames with no gap above 60 ms.
Full captures retain isolated slow frames; initial shader work is separate from warm runs. Final
raster p95 was 1.32/1.22 ms for equal-height and 1.24/1.24 ms for grid, with no over-budget raster
frames in those bursts. UI outliers remain above the 165-Hz budget; the measured improvement is
shorter pauses, not elimination of every missed frame. Burst duration includes input and admission
work and does not change the animation duration.

Both layouts and the centered album landing retained their endpoint and intermediate geometry.
The final grid client immediately showed placeholders at a distant date, filled them without moving
the viewport, and returned to the original top heading without a subsequent jump. The input helper
supplies short drag strokes; this run does not establish prolonged-drag latency.

Final equal-height RSS was 367.7/376.0/357.6 MiB (initial/peak/settled), versus baseline
370.8/382.0/378.0 MiB. Final grid RSS was 438.6/457.0/437.3 MiB, versus intermediate
427.9/475.4/413.8 MiB. Final Dart heaps were 34.5/38.0 MiB. These complete captures include background
work and different durations; they do not establish allocation-rate or GC improvement. Final
equal-height/grid captures contained 138/116 VM `Scavenge` starts across threads over 33.99/30.65
seconds. No image cache, retained-image budget or full-layout cache was added or enlarged.

The inventory below combines current Profile samples with source/lifecycle inspection. All
non-gallery/non-sidebar implementations remain unchanged; previous visual and lifecycle checks
below retain their original scope rather than becoming newly executed acceptance.

| Surface family | Assessment and disposition |
| --- | --- |
| Card/viewer entry and return | Normal gallery, albums, smart albums, group/member entry converge on the shared viewer and flight owners. Warm open/close UI p95 was 0.83/1.00 ms; raster p95 1.32/1.02 ms. Preserve image/quality leases, clipping, overlay fade and route ownership. |
| Viewer controls/image changes | Filmstrip open/close UI p95 was 0.90/1.08 ms. Its stable child and hidden ticker retirement remain; image reveal retains its existing quality handoff. |
| Gallery reflow | Keep the first optimization and bounded exit visuals (64 images, four million pixels, disposal on completion). Both equal-height and grid callers share this owner. |
| Sidebar/folders | Keep the paint-only viewport change. Folder disclosure UI p95 was 0.83/0.68 ms with no over-budget burst frames; retain its framework size/fade and closing-child retirement. |
| State/selection controls | Group flip UI p95 was 3.56/3.58 ms, with no over-budget UI burst frames. Retain hidden-face lifetime, icon transitions, header/selection actions and shared album/sidebar callers. |
| Menus/dialogs/trays/notices | Menu/filter-tray UI p95 was 0.46–0.59 ms; album-tray p95 was 0.80 ms. Keep framework route motion and stable-child feedback/task expansion. First-use raster spikes remain visible in full captures. |
| Loading/temporary content | Duplicate-entry loading burst UI/raster p95 was 1.12/1.15 ms. Loading already repaints a bounded region, shares morph shapes and stops its ticker when inactive/reduced; retain quiet/minimum-visible intervals and feedback priority. |
| Continuous input | Viewer zoom-out UI/raster p95 was 1.58/0.79 ms with no over-budget burst frames. Wheel and timeline input remained connected; comparison reuses the zoom owner. Preserve direct manipulation, inertia and scroll anchoring. |

The scoped source review covered the changed render and buffer/layout paths, callers, listener/layer
retirement, opacity publication, input/semantic transforms and exact geometry equivalence, without
a confirmed new defect. This initial assessment omitted dedicated first-open and native-window
measurements; the sidebar and warm-viewer captures did not close those paths.

Supplementary measurements used the unchanged `af76eeff` Profile binary in a fresh process. First
viewer entry had UI/raster p95 of 3.07/4.51 ms and maxima of 12.94/20.55 ms. The initial maximized
viewer capture reached 27.93 ms raster; its trace includes shader compilation and image uploads.
These are current costs, not paired improvement measurements or a settled root-cause diagnosis.

| Additional native window path | Current result |
| --- | --- |
| Equal-height gallery maximize / restore | UI p95 2.06 / 1.97 ms; UI max 13.33 / 11.25 ms; restored heading, first image and geometry |
| Grid gallery maximize / restore | UI p95 3.50 / 2.54 ms; UI max 8.38 / 7.23 ms; restored heading, first image and geometry |
| Grid border resizing | A short corner drag reduced the width from 1,269 to 1,089 pixels while retaining the leading image; repeated expansion and prolonged dragging are not covered |
| Viewer fullscreen and minimize / restore | Same image and position retained. The combined capture contains a 37.20-ms UI frame during restore; targeted attribution remains open |

The initial border attempt changed the timeline instead of the window, and an expansion gesture
was rejected for ending outside the target window. Neither establishes resize acceptance. Raw
supplementary captures use the `*-window-*-final`, `viewer-first-open-final` and
`gallery-border-grid-final` names under ignored `build/performance`.

Local completion remains open for first-use/restore-frame diagnosis and repeated native resizing.
Broader query latency and resource qualification retain their R10 owner. PR 63 contains this batch;
its `af76eeff` Quality / CI run 43436034710 was still running when these measurements were recorded.

## Loading-feedback integration — 2026-11-12

[PR 59](https://github.com/Cedarflake/Cedarflake-Ame/pull/59), head
`4539a2d10cde8bb2af7df8806da0e377336f908d`, was merged into `main` as
`e8fb5e13ec1f90328a7bfa353fb31c37f5ea5b81` with explicit authorization to merge without waiting
for CI. The local and remote `codex/loading-indicator` branches were deleted after verifying
the integrated ancestry. No final-head hosted pass is claimed by this integration record.

The [scoped checks](#expressive-loading--2026-11-11) retain desktop and focused verification,
including viewer continuity, selection actions and the filmstrip overlay. The
[category-switch measurements](#classification-navigation-and-foreground-reads) and
[file-operation measurements](#file-operation-preparation-and-publication) retain approximately
two-second waits. Alpha 3 package verification belongs to the
[release evidence](unsigned-windows-release.md).

## Package admission — 2026-11-10

The branch starts from R8 head `fd57185f`; plan commit `f7b94ec1` records its boundary.
`flutter pub get` resolved `material_ui` 1.11.0 with archive digest
`474c135fd46b56ff1d35ca982155e8863d28fbc6464d1192acf4ee53a738680e`.
Only that dependency changed. Its BSD-3-Clause license, required Dart 3.13/Flutter 3.47 minimum,
and installed public `Springs` API were checked against the existing Flutter 3.47.8/Dart 3.13.7
toolchain. Application language mode, renderer and native contracts remain unchanged.

The retained theme and menu suites passed through the serial repository entrypoint. They cover
both brightness variants, typography, popup animation, keyboard navigation, dismissal and tooltip
lifecycle. This establishes scoped package compatibility; desktop and full hosted results belong
to the later connected-motion verification, not this package-only check.

Sources: [official release notes](https://pub.dev/packages/material_ui/changelog#1110) and
[official spring tokens](https://github.com/flutter/packages/blob/main/packages/material_ui/lib/src/generated/motion_defaults_m3e.g.dart).

## Local state transitions

The shared driver uses the installed official tokens and framework simulation. Folder/root symbols,
expansion/menu arrows and review controls now consume it. Framework buttons still own hit targets,
selection, focus and semantics. Root availability symbols and album selection behavior are retained.

The focused shared-driver check initially exposed stale reduced-motion state when restoring
animations and changing the target in the same frame. Reading the current inherited preference in
the update path resolved it. Reversal now preserves displayed value and velocity, reaches exact
endpoints, and reports dismissed/completed correctly; reduced motion and inactive surfaces retire
their tickers. Existing folder navigation, selection-focus, header reversal and settings interaction
checks passed. Settings arrow assertions now inspect the visible rotation rather than a replaced
glyph; their keyboard, persistence and disabled-state assertions remain.

## Connected sidebar and viewer motion

Sidebar geometry now uses the shared spatial driver while retaining one progress value for its
boundary, heading and search alignment. The existing multi-width/text-scale checks passed: gallery
layout receives destination widths only, right-edge controls stay fixed, and reversal starts at the
current boundary. Normal and group viewer entry/exit share effects springs. The installed Flutter
route simulation hook retains framework navigation and dismissal ownership.

The existing viewer checks passed for immediate gallery input/position restoration, closing-source
retirement, interrupted reopening, retained semantics and reduced motion. Completion assertions
settle the spring rather than assume the superseded fixed duration; intermediate opacity and fixed
content bounds are still checked. Scoped static analysis passed. Desktop interaction and hosted
verification are recorded below.

Review found that the choice trigger's value-based key recreated its arrow on selection. A focused
reproduction observed an immediate reset to zero instead of a closing transition. Retaining trigger
identity preserves the arrow and focus while publishing the new choice immediately; both field and
menu presentations now exercise this path.

The largest changed production owner, `library_source_navigation_tile.dart`, has 528 production
lines and no inline tests; its change is limited to icon composition. The existing settings
presentation suite has 1,243 dedicated test lines and no production code. No responsibilities or
test bodies were relocated to satisfy a line limit.

## Folder children and format choices

`LibraryFolderDisclosure` owns one branch's spatial/effects controllers and closing-child lifetime.
Closing immediately excludes input, focus and semantics; reversal resumes the current progress,
and completion releases the retained subtree. The navigation owner only composes root and nested
branches, preserving its existing page limits. It has 546 production lines and no inline tests.

Desktop review reproduced selection ink remaining after a selected child's text faded. Material
was painting that ink on the outer sidebar, outside the disclosure's clipping/opacity. A pixel
regression reproduced blue ink in the unrevealed area; placing its transparent Material host inside
the transition makes both opening and closing clip the ink. Reversal, retirement, reduced motion,
nested navigation and replacement/failure/empty paging checks pass. The updated Windows client
retains the selected folder's gallery while closing its branch, without the detached selection patch.
The reported displacement was subsequently reproduced by dragging an expanded root row: its proxy
appeared near the top center of the application. The application scroll theme inserted a render
wrapper around the captured drag proxy, breaking its overlay positioning. A sidebar-local overlay
keeps that proxy below route-only themes. The same regression exposed the framework auto-scroller's
oversized-target assertion for expanded branches. `LibraryFolderReorderItem` now collapses the root
before framework drag measurement, with a one-frame handoff that retires on early release or list
replacement. The sidebar regression fails on the previous implementation and passes with correct
row coordinates, no retained children, preserved relative-order writes and failure rollback.
The updated client automatically collapses an expanded root dragged by its label and allows it to
be reopened after dropping. Repeating from its arrow with a child selected retains that child's
gallery; neither path displaces the folder proxy over the gallery. A follow-up regression reproduced
accidental collapse after a five-logical-pixel mouse movement over the label or a button. Folder
sorting now uses the framework's Material tap tolerance, including device gesture settings, before
starting its collapse/handoff. The existing click, drag, stale-order and rollback checks pass.
The rebuilt isolated client also accepts five-pixel movements on the expansion button, more button
and folder label as clicks, while an explicit longer drag collapses the branch before sorting.
The disclosure owner has 109 production lines, the drag owner 195, and their dedicated suites 174
and 224 lines respectively, without inline tests.

The format picker no longer offers custom extension entry or unsupported AVIF/HEIC choices. BMP,
TIF/TIFF and ICO join the existing supported choices; already selected legacy extensions remain
visible and removable. Existing combined-filter apply/cancel/clear checks pass.

## Duplicate transitions and browse continuity — 2026-11-11

The bounded gallery motion owner admits explicit duplicate-display switches and same-scope
background merges. Departing cards retain painted pixels only; their controls and source leases
retire immediately. Captures are bounded to 64 visible cards and 4,194,304 pixels, coalesce during
continuous updates, and dispose on completion, interruption or unmount. Existing resize, reversal,
direct-scroll and reduced-motion checks pass. Production-wall checks cover both layouts, including
monotonic retained-card movement and an offscreen merge that leaves visible cards still.

The reported date jump was traced to anchor resolution following a removed copy's representative,
which can belong to another date. Chronological duplicate publication now keeps the browsed date
vicinity from the coherent compact manifests and preserves an existing top/bottom edge. The query
transition regressions cover manual/background merging alongside stale completions, new navigation
and separate viewer identity. Existing first-click, held-drag and scroll/rail synchronization checks
pass; no timeline seek or scrolling algorithm was replaced.

Windows verification used the existing 10,000-file isolated corpus and a Debug client built from
`356fbc95` plus this change. Its optimized-debug native DLL digest is
`8CC805B913002FEF51A17D48BEE4BB192CE7A13056BB0AFD676F083859CEA54C`.
After automatic duplicate analysis reduced the displayed set to 72, the viewport remained at the
newest date and the rail at the top. In equal-height layout, expanding and merging at July 2017
retained that vicinity; grid layout retained July 2013. New scrolling and rail navigation worked
after each publication. A bounded live trace confirmed that rail values continued to identify the
same date vicinity while its tick distribution changed. Native input exposes short drag strokes;
continuous held-pointer behavior retains its focused automated regression.

The format tray displayed supported extensions, allowed JPG and PNG together, and cancelled without
changing the query. The album-confirmation switch changed both ways. The earlier native sidebar/search,
ordinary/group viewer, choice-menu and folder drag checks remain applicable. No normal catalog or
original-library source was used. The client closed normally with exit code 0; all 10,000 fixture
lengths/modification times were unchanged and the isolated derived storage was removed.

Production ownership is 56 lines for publication admission, 84 for duplicate browse position,
108 for exit pixels and 358 for motion. The gallery wall (1,580) and screen (1,998) receive composition
changes only. Dedicated motion, production-wall and query-transition suites contain 328, 161 and
579 lines; these owners contain no inline tests. The visible-position value/resolver owner has 193
production lines and carries the captured scroll edge with the saved location and fractions.

## Hosted verification

The [full run for `6e6b3806`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42829360806)
passed Static, the Windows build, all four Rust partitions and Windows Scan. Flutter and Windows
Accessibility exposed viewer-exit assertions tied to the superseded 120/128-ms duration.
Their existing checks now wait for viewer retirement within bounded frame loops, retaining immediate
input exclusion and per-frame geometry assertions. All five local viewer-geometry checks pass.
The [run for `8dbbb356`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42851228865)
passed Windows Accessibility and Windows Scan. Its Flutter batch exposed four filter-restoration
regressions: sampling the edge at publication used the filtered/clamped position instead of the
saved browsing intent. The visible-position value now captures that edge before the query and carries
it through identity resolution. All existing filter-restoration expectations pass unchanged, together
with query lifetime, held-drag and scroll synchronization checks. A rebuilt isolated Windows client
also restored the exact December 2016 viewport after filtering 700 fixture images to 70 PNG images,
which had clamped the filtered viewport to its bottom. Content and timeline remained at the restored
position after settling.

## Branch delivery — 2026-11-11

The [full CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42861922144) for
`c3a68fd83eec329a87a0e749ff4c8f2f623d86a4` completed successfully: Static, all four Rust partitions,
Flutter, Windows Scan, Windows Accessibility, Build Verification and the aggregate Windows Gate.
Release and synthetic workloads were not selected for this presentation scope.

Final source review covered spring reversal/status, closing-child input and focus retirement,
the root-drag handoff and click tolerance, bounded exit-pixel disposal, and query-scoped duplicate
anchoring. No remaining issue was identified within those boundaries. The native interaction and
unchanged-fixture evidence above remain applicable to this product revision.

Subsequent changes synchronize R8's integration record and close out documentation only. UTF-8,
local-link and whitespace checks passed; the delivered product tree matches the tested commit.
M1–M4 was delivered on `codex/material-motion`; its later integration is recorded below.

## Held timeline marker — 2026-11-11

The same-query publication regression reproduced a stationary held marker moving from 300 to
332.7 logical pixels when the complete layout temporarily became estimated. Reprojecting the
displayed value through layout rows introduced the displacement. The rail now retains the actual
pointer value in its gesture scale while navigation continues through the current query projection.
Query, layout-kind and controller replacement still retire that input authority.

The extended existing regression preserves the marker through both estimated and complete
publication; all 18 publication and drag cases pass, covering equal-height and square layouts.
Formatting and affected-owner analysis pass. The isolated 700-file Windows client also displayed
immediate distant placeholders in both layouts, filled them without moving the date or marker,
and accepted a reverse drag. Native tooling supplies short drag strokes; continuous multi-second
desktop holding remains unverified. The controlled regression holds through both publications.
No source file operation was executed during these checks.
## Card-to-viewer motion — 2026-11-11

The activated thumbnail supplies bounded geometry to the existing embedded viewer and duplicate
review route. The current image moves and scales without another decoded image or source read.
Backgrounds and controls fade separately; an exit-frame pixel assertion confirms that the returning
image remains opaque. The mounted-card registry resolves the current image after adjacent navigation;
removed/offscreen targets and rotation retain a fade fallback. Reduced motion is immediate.

Seventeen focused transition, route and duplicate-review cases passed, including retained scroll,
interrupted return, source cancellation, stale targets and unchanged review behavior. Earlier viewer
position and wall-geometry checks remain applicable to the unchanged layout contracts. Formatting,
scoped analysis and whitespace checks passed. Hosted checks retain their separate revision scope.

The Windows Debug client used the existing 700-file fixture subset and isolated derived storage.
Its native DLL remained unchanged (`FA18D9B84C71864E175492AF099A97FC4426EF3EE77C506DFCE659523EF24729`).
Observed entry and return in the ordinary gallery and duplicate members preserved toolbar position
and image shape. After advancing to the second member, return moved toward that member's visible
thumbnail. Slow-motion inspection confirmed clear image pixels while surrounding controls faded;
the debug speed was restored to 1 after each observation. No source operations were executed.

Production owners contain 203 lines for image-flight painting, 184 for origin registration, 45 for
chrome/backdrop opacity and 182 for embedded transition lifetime. The existing image viewer (318),
source-image owner (256) and duplicate page (386) receive composition changes; these owners contain
no inline tests. The dedicated embedded-transition suite contains 389 lines.

Card overlay follow-up: badges and selection overlays fade out with viewer entry and reveal only
when return finishes. Duplicate badges retain their paint while the gallery is muted; existing
input/focus exclusion and tooltip retirement remain active. The existing transition regression now
checks the ordered fade, including no badge visibility while the image is returning. Badge behavior
and retained duplicate-viewer semantics were also checked. The isolated desktop client showed
badges during the opening fade and hidden during return, then restored on the unchanged cards.

## Loading feedback — 2026-11-11

Gallery refresh and paging previously changed the display delay while work remained active,
restarting visibility. The shared presentation owner now keeps one active interval, hides work
finished within 250 ms, fades longer work and reverses short interrupted fades. The 500-ms selection
preparation rule remains; no request, completion or cancellation waits for an animation.

All loading-indicator constructors in feature presentation now use this owner, including previews,
filmstrip, gallery/time rail, folders/albums, search, file-operation dialogs/history, model/scan/update
feedback and storage/organization settings. The initial-gallery message uses the same gate around
its existing label and spinner; the persistent storage-capacity gauge remains unchanged. Gallery
overlays, search status, history lists, duplicate locations and settings retain fixed progress slots.
Task-card progress space collapses through the existing spatial spring.

Focused checks cover short completion, uninterrupted phase changes, fade reversal, disposal, reduced
motion, actual accessibility exclusion, unchanged viewport/tile/scroll geometry and connected input.
Existing selection, retained-scan, timeline, filmstrip, smart-album, settings and feedback checks pass.
The feedback regression checks intermediate card heights in both themes. Formatting, affected-owner
analysis and whitespace checks pass. These checks do not run new downloads or physical file actions.

The rebuilt isolated Windows Debug client completed import of the existing 700-file subset, retained
its gallery position, and displayed the completed album/sidebar and settings states. Long import
progress remained visible; quick sidebar reads settled into content without a retained spinner.
Timing boundaries are established by the controlled regressions, not native screenshot sampling.
The native DLL is unchanged from the viewer verification above. The preceding client closed with
exit code 0, preserved all 10,000 fixture file lengths/modification times and retired scratch storage.

The shared visibility owner contains 157 production lines; the feedback template contains 123.
The storage section (723) and screen (1,995) receive presentation composition changes only, with no
inline tests. Dedicated loading timing, geometry and feedback suites contain 135, 173 and 220 lines.
The [full run for `e6b5c455`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42945787064)
passed; subsequent viewer/loading changes retain their own final-head hosted verification boundary.

## Integration — 2026-11-11

[PR 58](https://github.com/Cedarflake/Cedarflake-Ame/pull/58) merged the cleanup and inherited motion
work at `ca82d675a4281f7b2bb1f79ee9a76af3af4a2b07`. Both `codex/duplicate-cleanup` and
`codex/material-motion` were verified as ancestors, then deleted locally and remotely; local `main`
was synchronized. The user explicitly selected integration before final CI completion.

At integration, the [combined-head run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42955596711)
for `b4a4531c` remained in progress without a reported failure, and the
[merge run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42956932239) had started.
The focused and desktop evidence above remains scoped to its recorded revisions. Continuous
multi-second desktop holding remains unverified as recorded under
[held timeline marker](#held-timeline-marker--2026-11-11).

## Expressive loading — 2026-11-11

The [loading plan](../plans/loading-indicator.md) adds the shared temporary Material renderer,
icon-only preparation feedback in dialogs/trays, and coordinated gallery/review loading. The
timeline retains its circular indicator and geometry without the left divider. Review lists retain
their virtual extent and demand while omitting blank-card decoration. No native, catalog or source
operation contract changes are included.

Focused checks pass for hidden/reduced-motion ticker retirement, delayed admission, quick completion,
fade reversal, unchanged viewport geometry, content/timeline exclusion, pending-read cancellation,
retry and retained/batched location lists. Existing review layouts and cleanup-confirmation checks
also pass, including unchanged action admission. A bounded rendering probe sampled 28 frames across
one complete morph cycle; the largest painted extent was 44 pixels within the 48-pixel allocation.
This rendering probe establishes shape bounds, not Windows interaction acceptance.

The rebuilt Windows Debug client imported the existing 700-file subset into isolated derived
storage. Gallery/timeline layout, six duplicate groups, member-list continuation, the 210-location
tray, album selection and both history-list empty states remained usable. The deletion confirmation
showed its 694-image count without the former horizontal bar; it was cancelled without submitting
a file operation. Fast reads settled without a retained loading indicator.

A temporary 12-second preparation delay allowed native-window observation of the small morphing
icon in the action row, disabled confirmation and unchanged button positions in dark mode. No
loading caption appeared. The delay was removed and the production code hot-reloaded afterward.
The previous light theme was restored. The client then exited normally with code 0, all 700 source
file lengths/modification times matched, and scratch storage was retired. Content/timeline mutual
exclusion and reduced motion are covered by the controlled regressions above; the fixture's fast
native queries did not provide a long simultaneous-wait observation. Final-head hosted checks
remain owned by the pushed commit.

The renderer, progress facade, dialog helper and gallery loading region contain 142, 208, 132 and
90 production lines respectively. The screen (1,998) and review page (387) receive composition-only
changes; none of these owners contains inline tests. Dedicated renderer/priority tests contain 50
and 195 lines; existing dialog and review tests retain their behavioral checks.

### Selection admission and feedback

Album/Favorite writes validate the frozen groups and affected decision field inside the write
transaction. Unrelated catalog/album changes and independent Favorite changes no longer veto an
album write. Selected content, membership, representative and later same-field decisions still
reject stale authority. Initial all-matching preparation retains its catalog guard; explicit file
operations already resolve current selected state before confirmation and revalidate each file.

Five focused native admission/source-selection checks and the changed batch cases passed, including
receipt replay and transactional rejection. All 25 selection-action widget checks passed; preparation
survives a same-intent catalog publication, cancellation retires it, and failures use notifications.
The toolbar no longer paints an error or progress line. Slow preparation uses the shared bottom
feedback host after 500 ms. The rebuilt normal `local-primary` Debug client opens the album picker
for a selected screenshot and shows preparation in the bottom host without adding toolbar content.
The picker was cancelled without submitting membership or source-file changes. Affected static
analysis and formatting pass. Commit `cda7a999` was pushed; its hosted workflow was pending at the
last observation, and the follow-up below has its own final-head check.

The native admission owner has 77 production lines; batch orchestration has 275 and no inline tests.
The selection widget has 425 lines, its activity owner 35 and no inline tests. Reused selection-action
checks contain 732 dedicated lines; native admission and batch-mutation files contain 65 and 322.

### Organization waiting, intent and feedback ownership

Continuation metadata now reads alongside the gallery query. Completed same-scope results are
published before a coalesced follow-up, while scope changes reject old results. Exit supersedes
pending queue or continuation reads without waiting for them; their late successes and failures
cannot reopen the viewer, replace the current session or display a stale error. Native session
retirement continues after its local authority has been revoked.

Selection quietly prepares its summary, updates the Favorite label before activation and reuses
that target on activation: uniform selections act directly, while mixed selections offer add/remove
choices. The always-open menu introduced in `348588d1` was withdrawn. The Material `IndexedStack`
reserves the label variants' bounds so the neighboring actions do not shift when the read completes.
The existing 25 selection checks cover uniform and mixed Favorite actions, album add/move/removal,
defer/ignore restoration, cancellation and unrelated catalog publication. Known single-image
decisions remain actionable during background refresh. Completed file feedback yields to organization
failures and results; running file work retains priority. Changing a selection clears its previous
preparation notice. Entry, scope change, exit and continuation failures now identify their
corresponding operation.

Affected interaction checks pass for pending-read exit, stale result/error rejection, concurrent
catalog refresh, automatic Favorite choice, selection cancellation, notification precedence and
workspace restoration. The previous test requiring a busy Exit to ignore input was replaced with
an exit-during-query regression. Static analysis, formatting and the Windows Debug build pass.
The normal `local-primary` client displayed 63,072 images, entered organization with a usable
continuation button, and returned to its original gallery. Exit was visually complete in a 2,486 ms
observation including an intentional 1,800 ms wait and capture. A mode-entry capture at 1,751 ms
still showed query loading; a later capture showed the complete controls. These are sampled
observations, not isolated query timings or proof that large-library reads are instantaneous.
The normal-client Favorite follow-up verified a previously unfavorited image becoming favorited by
direct activation, the selected favorited image showing `取消收藏`, and a mixed selection opening
only the two applicable commands. Direct unfavorite restored that image's original decision.
The assigned image's album picker retained `移出相册` and `移至其他相册`, selected its current album,
and disabled a redundant move to that same album. The picker was dismissed without assignment changes.
No physical file operation was submitted.

The continuation reader, transition, mode and selection owners contain 70/43/433/458 production
lines and no inline tests. The reused mode/widget suites contain 529/744 dedicated test lines.

### Dialog and viewer follow-up

Preparation feedback now occupies the content center; its visibility owner stays mounted through
completion so a short fade cannot be cut off by the dialog. Failed file preparation shows only its
explanation and return action. The controlled file-action check covers delayed preparation,
failure during the fade, immediate action availability and final indicator retirement. Existing
cleanup, image reveal, timeline endpoint and conflict-pagination checks pass. The earlier hosted
Flutter failure was an offscreen `Previous` test tap; scrolling that existing control into view
retains the paging assertion, which passes at both text scales.

The source-image check publishes the original during entry: the preview stays unchanged until
entry completes, then the original blends over it. Return freezes an incomplete quality blend.
Pixel regressions cover continuous clipping at a partially hidden card and corner geometry under
viewer scaling; existing cancellation, unreadable-source fallback and interrupted-return checks pass.
The Windows Debug client on `local-primary` verified full and partially clipped cards opening and
returning with visible image pixels, retained gallery position and rounded card edges. Slow motion
was restored to 1 afterward. Frame-timing guarantees come from the controlled checks; desktop
screenshots are sampled observations. No physical file operation was submitted.

The 2026-11-12 filmstrip follow-up places the strip above the fixed toolbar without reserving image
height. The existing `AmeStateTransition` drives a clipped vertical translation; retained thumbnail
elements lose input, focus, preview demand and child ticking as closing starts. The existing viewer
and filmstrip-frame suites passed 18 checks, including unchanged image geometry during entry,
actual-size/fullscreen preservation, closing frames, retained pixels and hidden repair retirement.
In the rebuilt normal Windows client, opening and closing the strip kept the same square image at
65% with unchanged upper/side bounds. The image viewer and strip slot contain 326/77 production
lines with no inline tests; their reused viewer/frame suites contain 1007/320 dedicated test lines.

The local review artifact `build/review/dialog-errors-2026-11-11/index.html` contains 24 abnormal
states in both themes, with cropped images and full-window originals. These are current production
widgets rendered with isolated fixtures and Windows fonts, not native-client captures. They cover
file preparation/destination errors, album forms, removal, cleanup, history/report reads, operation
and notification details, filter/date validation and unavailable roots. Shared error templates are
represented once per distinct layout; Windows-owned system prompts are outside this inventory.

The dialog, image reveal, source-image, image-flight and origin owners contain 133, 156, 274, 222 and
192 production lines, with no inline tests. Their reused file-action, source-image and transition
suites contain 690, 488 and 533 dedicated test lines. The final Debug build and affected static
analysis pass; current-head hosted results belong to the normal branch workflow.

### Dialog layout review

The revised local inventory at `build/review/dialog-errors-2026-11-11-v2/index.html` contains the
24 abnormal states and the normal deletion confirmation in both themes (50 images). It uses the
same production-widget rendering and Windows-font procedure above. Failure explanations now share
leading alignment, error color and section spacing; history read failures occupy the content area
without an empty-result message. The retry button and the intrinsic-width explanation group share
a center. The pinned date picker bypasses `headerHeadlineStyle` in landscape input mode; a scoped
text theme supplies a compact two-line date/weekday without changing the framework calendar.

The held-drag captures at `build/review/sidebar-drag-light.png` and `sidebar-drag-dark.png` show the
production folder row in its rounded, clipped proxy. Existing drag, rollback, album, filter, cleanup
and notification-detail regressions pass. The retained file-history case now also proves read
failure, retry and the return of actionable records. Deletion checks preserve unchecked initial
state, label activation and separate confirmation after replacing the row-wide ink response with
the framework checkbox's own feedback. No real source mutation was submitted for this review.

The shared dialog helper and history owner contain 210 and 311 production lines; neither has inline
tests. The reused file-operation suite contained 717 dedicated test lines at this review. Affected
static checks and the Windows Debug build pass. The subsequent rebuilt client on `local-primary`
confirmed label activation, unchecked initial state, compact spacing and no row-wide background;
cancelling the dialog preserved the selected image and viewport without a physical operation.

### File-operation preparation and publication

Explicit selections now drive indexed location and compatible-evidence reads. The original query
walked unrelated locations, and joining the display-evidence view materialized the library's
fingerprints. The bounded-cost fixture initially still exposed a full scan without SQLite statistics;
driving the indexed join from selection rows removed that dependency. Gallery, merged-group and
physical-location preparation retain their frozen scope and current-state validation.

Anchored refresh resolves its window inside the same ordered projection already used for layout
and timeline. Rapid native publications share a refresh; final completion waits for the native
worker, selection closure and the final gallery read. The feedback remains one active operation
until that point, without an intermediate completed-file summary.

Read-only native-DLL measurements used the existing 63,072-location `local-primary` catalog,
47,828 merged items, one selected image, a 500-item anchored page and two warmed repetitions:

| Boundary | Before | After |
| --- | --- | --- |
| Selection preparation | 1,911–2,542 ms | 1.7–3.0 ms |
| Anchored full snapshot | 2,220–2,926 ms | 2,200–2,689 ms |

The full refresh remains approximately two seconds; it is not a demonstrated substantial speedup.
Further paired anchored/unanchored reads measured 1,985–2,052 / 1,974–2,360 ms. Isolated SQLite
profiling attributed about 1.2–1.3 seconds to the merged layout query, versus about 0.2 seconds
without folding and 5–9 ms for 500 detail rows. Alternative full-group/union plans were slower and
were not adopted; the existing unanalyzed-file fast path remains intact. Initial cold queries were
slower and are excluded from the warmed comparison, not presented as startup improvements.

Native checks cover bounded selected-file work, frozen group membership, one-transaction snapshot
publication and anchor/fallback order across eight sort/direction combinations. Application checks
cover coalesced publication and joined shutdown through a held final gallery read. The rebuilt
Windows Debug client displayed the prepared deletion count and enabled action without a sustained
loader; confirmation was cancelled. No real source operation was submitted in these measurements.

The preceding hosted run for `0179e137` exposed two obsolete loading-layer finders and an assertion
before the original-image error's next frame. The same visibility, position, source-release and
retry assertions pass in the three affected suites after targeting the shared visibility layer
and pumping that frame. Final-head hosted verification belongs to the normal branch workflow.

The publication coordinator and action controller contain 58 and 223 production lines. Selected
query construction contains 118 production and 126 inline-test lines; anchor, projection and
snapshot owners contain 238, 301 and 361 production lines with no inline tests. Reused operation
and anchor suites contain 781 and 233 dedicated test lines.

### Thumbnail handoff across viewer entries

The mounted card now supplies its decoded, resized provider and sampling quality to both viewer
entry paths. Controlled checks prove that embedded gallery viewers and routed review viewers use
the card's existing pixels on their first visible frame without decoding another preview. Existing
source-reveal and flight checks retain delayed quality blending, cancellation and return clipping.

The rebuilt Windows Debug client on `local-primary` opened and returned from smart-album, exact-group
and similar-group member cards with visible pixels. Ordinary and user-album galleries share the
embedded path; no user album was created for this check. These client observations complement the
first-frame checks, rather than measuring individual animation frames. No physical file operation
was submitted.

The launch, reveal and viewer-image owners contain 244, 168 and 280 production lines, with no inline
tests; the reused viewer-image suite contains 586 dedicated test lines. Affected analysis, focused
checks and the Windows Debug build pass. Hosted verification remains attached to the pushed commit.

### Review refresh during classification

Background catalog publications previously invalidated each unfinished review read. The shared
review window now admits the current same-query result and coalesces publications into a subsequent
refresh. Explicit refresh, view changes and disposal still retire the old generation. The focused
window regression covers first-page admission, cached-range handoff, one queued replacement and
explicit supersession; existing bounded paging, failure/retry and disposal checks pass.

The [hosted run for `f8d15003`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43035643833)
exposed two outdated widget expectations and an unhandled test continuation. The location-sheet
case now completes the active read before its coalesced replacement and retains row/status checks;
the preference case checks loading opacity while retaining card identity and scroll position.
The category-read case awaits its observation continuation. All 31 checks in the three affected
suites and scoped static analysis pass locally; final-head hosted verification remains pending.

The normal `local-primary` Debug client displayed similar groups and the empty ignored view while
classification continued. Read-only SQL timings on its existing catalog measured similar-group
count/page reads at 5,444/4,656 ms and empty ignored reads at 2,779/3,348 ms. These two queries each
rebuild the scoped similarity projection; that remaining cost is not reduced by the read-lifecycle
repair. The classification queue was processing a new taxonomy after automatic discovery added
`代码截图`; this observation does not change category-discovery policy.

The subsequent SQL repair admits groups from two current, non-excluded contents and computes
representatives only for the requested page. A single read-only snapshot of `local-primary`
returned identical counts and both first/second pages for 3,427 groups. Count/first-page reads fell
from 3,222/2,679 ms to 377/177 ms; the empty ignored view fell from 1,762/2,056 ms to 5/2 ms.
These are SQLite query timings, not end-to-end client latency. Existing preference, exclusion,
pagination and seek checks pass; the bounded-read regression also covers group and ignored reads
against unrelated fingerprints. The query/projection owners contain 193/147 production lines,
with no inline tests; the reused dedicated suite contains 464 lines. Rebuilt-client observations
follow below. No schema, source-file or cleanup authorization changed.

Exact-review reads now start from compatible fingerprints, or directly from saved decisions for
the ignored view, and resolve current locations through their source-generation index. An empty
count skips the page projection within the same transaction. A paired read-only snapshot returned
identical counts and pages for 12,571 groups: count/page reads changed from 2,257/17 ms to
1,716/13 ms; the empty ignored view changed from 2,213/2,149 ms to under 1/1 ms. The nine existing
review and paging cases pass, including source changes, preference retention and filtered seeks.
The group-query owner contains 176 production lines and no inline tests; existing dedicated review
and paging fixtures provide the behavior coverage.

The review-window owner contains 237 production lines with no inline tests; its reused suite has
229 dedicated test lines. The client checks submitted no source-file or organization operation.

### Date headings after navigation and first entry

An admitted date read aligned its first photo row rather than the preceding heading. Date-navigation
offsets now include that heading, with day-level targets inside each month; continuous rail movement
retains its exact pixel offset. Initial complete snapshots separately retain the start-edge intent
through the wall's first correction instead of placing the first photo above its date.

Focused checks cover delayed date reads in both layouts, same-month date targets, continuous held
input, publication, endpoint alignment and complete-snapshot startup. Existing filter and smart-album
position checks pass. The rebuilt Windows Debug client on `local-primary` displayed the first date
on startup and after a far-position seek followed by return to the top. No source operation was run.

The layout, navigation and projection owners contain 455, 774 and 99 production lines; the screen
and wall contain 2,002 and 1,582, with no inline tests. Their changes pass existing edge intent to
the layout owner without adding another lifecycle flag. The focused navigation and projection suites
contain 250 and 148 dedicated test lines; startup coverage reuses the filter-position fixture.

### Classification navigation and foreground reads

Saved settings and category navigation no longer wait for the classification runtime to open.
Navigation publishes completed reads while coalescing intervening catalog/model updates into one
subsequent read. A catalog with no explicit legacy category choices checks for a current location
per predicted category; catalogs with those choices retain the existing precedence query. The
read-only paired catalog probe returned the same seven categories in 675 ms versus 1,223 ms.
Automatic curated expansion and immutable prediction compatibility remain unchanged.

Gallery snapshot, timeline, review-page and category reads use the existing foreground-read lease.
Background analysis yields at its existing admission/cancellation boundaries while these reads run;
completion and failure release only the corresponding lease. This does not forcibly terminate an
already executing native call.

The rebuilt normal `local-primary` Debug client displayed 63,071 images and the saved category list.
With classification still publishing updates, switching to the 6,173-image comic category and the
81-image photo category completed in observed upper bounds of 2,537 and 2,212 ms, including desktop
input and accessibility capture. The exact-review page displayed all 12,571 groups. These remaining
two-second category waits are retained measurements; the user accepted the current interaction.
The rebuilt client remains open. No source-file or organization operation was submitted.

The 25 existing native classification query cases pass, including legacy choices, conflicts, removed
sources and retained display evidence. The Dart lifecycle cases and 20 affected navigation/review
checks pass.
The navigation reader/configuration/provider/foreground owners contain 44/86/62/73 production lines;
native navigation/projection contain 71/157, with no inline tests. The new reader has 60 dedicated
test lines; existing interaction, snapshot and lifecycle fixtures supply the remaining coverage.
