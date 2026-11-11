# Official Material motion adoption

The [roadmap](../roadmap.md#material-motion-adoption) owns disposition and the
[execution plan](../plans/material-motion.md) owns scope and order.

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
tests. The reused file-operation suite contains 717 dedicated test lines. Affected static checks
and the Windows Debug build pass. The latest checkbox adjustment and subsequent deletion-latency
follow-up still require the final live-client update; the isolated inventory is not a claim of that
desktop interaction result.
