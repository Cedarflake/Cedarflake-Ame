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
M1–M4 is delivered on `codex/material-motion`; the branch has been pushed and remains unmerged.
