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
verification remain for M4.

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
gallery; neither path displaces the folder proxy over the gallery. The disclosure owner has 109 production lines, the drag owner 158,
and their dedicated suites 174 and 201 lines respectively, without inline tests.

The format picker no longer offers custom extension entry or unsupported AVIF/HEIC choices. BMP,
TIF/TIFF and ICO join the existing supported choices; already selected legacy extensions remain
visible and removable. Existing combined-filter apply/cancel/clear checks pass.
