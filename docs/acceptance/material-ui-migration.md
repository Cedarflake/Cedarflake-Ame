# Standalone Material migration

## Search placement during sidebar toggles, 2026-11-26

The Windows Debug client retained the search field's position and width through sidebar collapse
and expansion. Dragging the sidebar from 247 to 319 logical pixels moved the search field with its
boundary; dragging back restored the original alignment. The existing brand visibility, notification
button and window controls remained intact. The client was hot-reloaded and left open at its original
sidebar width. Owned-file formatting and analysis passed. This geometry check uses the real desktop
interaction rather than adding permanent pixel assertions.

## Flutter 3.50 compatibility, 2026-11-17

The upgrade uses Flutter 3.50.0 / Dart 3.14.0, Material 1.11.1, Cupertino 1.1.7,
Riverpod 3.5.0, shared_preferences 2.5.6 and matching Dart/Rust/codegen FRB 2.13.0.
Freezed 4.0.3, analyzer 14.7.0 and build_runner 2.17.1 regenerate the existing DTOs;
the bridge content hash remains `356806720`. Hosted SDK pins and the local SDK match.
The application retains its Dart 3.12 language mode. The native window/display API migration
is deferred; the [plugin evaluation](#windowdisplay-plugin-evaluation-2026-11-18) records its boundary.

The [official release notes](https://docs.flutter.dev/release/release-notes/release-notes-3.50.0)
were compared with the installed SDK and Ame's compatibility owners:

| Area | Comparison and disposition |
| --- | --- |
| Image streams | New frame-callback guarding does not dispose a frame arriving after retirement or a codec created after retirement. Substituting SDK `MultiFrameImageStreamCompleter` in the five existing cases passed three and failed those two resource-release assertions. Retain `library_source_image_stream.dart`; its five cases pass. |
| Tooltips | The SDK includes [sibling-conflict invalidation](https://github.com/flutter/flutter/pull/190431); [distinct traversal parents](https://github.com/flutter/flutter/pull/190344) remains unmerged. Official Tooltip plus explicit target semantics passes focus/hover and live-message changes, but its visible overlay remains after the owning retained surface disables `TickerMode`. The same two-frame retirement check passes with Ame's adapter. Keep the adapter and retain that regression. |
| Preview motion | The Hero fix concerns Navigator resizing. Ame's retained viewer uses its own source-frame lease, clipping and return geometry. No equivalent replacement was established. |
| Spring motion | Ame already delegates the physics and tokens to official `SpringSimulation` and Material `Springs`; its remaining wrapper owns interruption and inactive/reduced-motion state. |
| Loading indicator | Material 1.11.1 still does not export the proposed `LoadingIndicator`; retain the attributed compatibility renderer. |
| Windows scheduling | The release's TimerThread predicate change is already present in 3.47.8. The newer platform-task cancellation fix does not replace the UI-thread or image-I/O policy. Keep the accepted renderer/thread settings; no new frame-rate claim is made. |

Dependency resolution updated 63 entries. Newly resolved Dart native-binding/tooling packages
`jni`, `jni_util`, `objective_c`, `hooks`, `code_assets` and `record_use` retain Dart's BSD license;
`analyzer_buffer` uses MIT. Their published license headers and JNI's optional desktop CMake
configuration were inspected. The Windows build resolves the generated JNI plugin entry.
No application Java API or new source-media workflow is introduced.

Local diagnosis passed 11 production widget cases across source-image retirement, tooltip lifetime,
workbench semantics and timeline semantics; 39 asynchronous bridge contracts; the 622-module Dart
dependency check; owned-file analysis; and formatting. The Rust Release library builds.
The Windows Debug integration rebuilt in 189 seconds and passed both suites, including all ten
native UIA checkpoints for gallery, menus, viewer and Settings. Its process/job exited cleanly
and its scratch directory was removed. Three runner-lifecycle and two real-engine retirement
cases passed; the latter also confirms the matching image-I/O thread at normal priority.

Dart 3.14 launches a separate `dartvm.exe`. The offline engine-input preflight now requires it;
the existing missing-artifact and orchestration fixture accepts the complete SDK and rejects its
absence. This owner has 119 production lines and no inline tests; its dedicated fixture has 251
lines. The tooltip owner remains 386 production lines with no inline tests and 188 dedicated-test
lines. The hosted full suite and production Release checks passed in
[run 44143907656](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44143907656)
for head `a83ef068`: Flutter, four Rust groups, static checks, Windows scanning, native UIA
and production build verification all succeeded.

The first hosted Flutter batch found three bootstrap cases still expecting `FlutterError` from
disposed notifiers. Flutter 3.50 delegates `ChangeNotifier.debugAssertNotDisposed` to `listen`,
which reports `StateError`. The four disposal assertions now require that exact error type while
retaining lifecycle, subscription and joined-shutdown checks. All seven cases in the two affected
bootstrap suites pass locally; application disposal behavior is unchanged.
The hosted analyzer also identified a redundant `flutter/rendering.dart` import in the viewer
filmstrip after the Material export update; the redundant import was removed.

### Flutter upgrade integration, 2026-11-18

[PR 70](https://github.com/Cedarflake/Cedarflake-Ame/pull/70) integrated `a83ef068` into `main`
as `8acb927c` after the full CI run passed. Their file trees match. The local and remote
`codex/flutter-3-50` branches were deleted and the local checkout was fast-forwarded to `main`.
The optional automated security review did not run because the account's review quota was
exhausted; it was not counted as passed verification.

### Window/display plugin evaluation, 2026-11-18

The `window_manager` 0.7.0 / `screen_retriever` 0.4.0 migration, using nativeapi 0.6.1,
was withdrawn before committing it. A rapid maximize/restore, full-screen round trip and
minimize/restore sequence exposed a system-painted blue caption and intermittent blank or
transparent content. Skipping the minimized child resize and narrowing nativeapi's region
cleanup did not resolve the reproduction. A root cause was not established.

The retained product uses `window_manager` 0.5.2 and `screen_retriever` 0.2.2 with their existing
startup, placement, close and drag adapters. Migration-only host channels, manifest changes,
native adapters, fixtures and dependency overrides were removed. The Flutter 3.50 baseline
and other dependency updates in `8e76e082` remain intact.

After restoration, offline dependency resolution and the isolated Windows debug build passed
(40.4 seconds). The real client imported 700 reusable fixture images and passed a manual sequence
covering startup, maximize/restore, viewer full-screen entry/exit, minimize/restore and normal
close. The stable window states retained app-drawn chrome and image content. The client exited
with code 0; the scratch wrapper verified unchanged lengths and modification times across the
10,000-file reusable source corpus. This check did not repeat the rapid nativeapi probe sequence.

## Material workbench composition, 2026-11-16

### Material workbench integration, 2026-11-17

[PR 69](https://github.com/Cedarflake/Cedarflake-Ame/pull/69) integrated `bb344c82` into `main`
as `cb5efadd` after explicit authorization to merge before CI completion. Their file trees match.
The local and remote `codex/material-workbench` branches were deleted and the local checkout was
fast-forwarded to `main`. The final update includes the refreshed gallery screenshot and the
`所有图片` sidebar entry and content heading. All five existing shell checks passed; the rebuilt
isolated client displayed both labels without wrapping or clipping and retained its two roots.

At merge, [PR run 44079685276](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44079685276)
was queued. The subsequent
[main run 44080318567](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44080318567)
owns verification of the merged tree. The preceding run at `54b44d67` passed Static, Flutter Tests,
Windows Scan, Windows Accessibility, Windows Build Verification and three Rust groups. Its Schema,
Queue and Organization group was unfinished when that run was superseded and canceled. These
partial results do not establish a completed full gate for the final revision.

### Interrupted sidebar and workspace motion, 2026-11-17

The workspace transition now starts at the active pane's displayed inset. Sidebar motion settles
when a different workspace takes over that boundary; hidden panes cannot overwrite the sample.
Destination-width content layout, final bounds and existing motion drivers are unchanged.

The retained-workbench fixture reproduced a jump from x=206 to x=80 when Settings interrupted
sidebar collapse. The same case now preserves its boundary on entry and on immediate return;
both collapse and expansion retain the gallery's scroll owner and offset. All 24 affected
workbench, sidebar, right-edge and duplicate-layout checks passed, including reduced motion,
enlarged text, stationary right-edge controls and unchanged content-layout counts during motion.
Formatting and scoped analysis passed. The Profile payload was rebuilt with the receipt-verified
native library. The isolated Windows client exercised switching to Settings during both sidebar
collapse and expansion, then returning before the workspace transition finished. Each transition
continued from an intermediate boundary, retaining the gallery position; normal animation speed
was restored after slow-motion inspection. The width-transition and sidebar owners contain
171 and 183 production lines, and the retained-workbench fixture contains 242 test lines.

### Folder refresh and import recovery, 2026-11-17

Refreshing a loaded folder branch appended a 40-pixel loading row below its existing children,
displacing later sources on every background publication. The controlled pending-read regression
failed against the prior implementation with the pending source moving from y=272 to y=312.
It now retains the same bounds during the read and after publishing changed folder names. Initial
loading and explicit paging retain their existing feedback.

All 14 focused folder-continuity and task-feedback checks passed, together with owned-file
formatting and scoped analysis. Storage-overlap recovery opens the directory picker without
repeating the rejected scan; canceling retains existing roots. The ordinary unavailable-folder
case still retries the original path. Storage admission and native contracts are unchanged.

The isolated Profile client reproduced the recurring displacement with an expanded `local-primary`
branch above the failed source. The rebuilt client retained its position across 18 observations
during background updates and displayed the storage-conflict reason. The final recovery-button
build opened the native directory picker after a rejected storage-overlap selection. Canceling
removed the failed pending row and retained both existing roots and the gallery position.
The local build reused the receipt-verified native library. Earlier clients exited normally with
all 700 fixture lengths and modification times unchanged; no new Dart exception, overflow or AXTree
error was logged. The navigation owner contains 555 production lines; the two affected fixtures
contain 411 and 452 test lines.

### Dialog section transitions, 2026-11-17

Album icon choices and file-organization explanations share the retained size/fade owner promoted
from folder disclosure to `AmeDisclosure`. Existing sidebar callers retain the same implementation.
Section padding participates in the height transition, with full content width retained throughout;
Material continues to position and size the dialog surface and actions.

Formatting, scoped analysis, the 622-module dependency check and 23 affected album, file-operation,
disclosure and tooltip checks passed. The album fixture checks intermediate opening/closing bounds,
the stationary dialog center, moving actions, restored geometry, left alignment and saving the
chosen icon. The Windows variant also retains semantics while hovering a choice and confirms its
tooltip retires before the closing section finishes. Existing disclosure coverage checks reversal,
reduced motion, clipped selection ink and immediate input/focus retirement.

The isolated Profile client exercised expansion, selection, saving and manual collapse. Copy
settings retained left alignment while their organization explanation expanded. Hovering album
choices initially exposed native AXTree orphan updates; the chooser and choices now use the
existing Windows-compatible `AmeTooltip`. The same hover/collapse and dialog-dismissal sequence
with accessibility reads produced no AXTree, Dart exception or overflow records in the rebuilt
client. Previous clients exited normally and preserved all 700 fixture lengths and times.
The shared disclosure, album editor and file dialog contain 104, 206 and 289 production lines;
the extended album fixture contains 398 test lines.

### Duplicate empty-result placement, 2026-11-17

Duplicate review now uses the shared gallery viewport to center empty and failed results across
the complete content pane. Populated grids retain their header spacing, and initial member reads
retain both their centered indicator and delayed loading line.

Formatting, scoped analysis and 11 existing layout/loading checks passed. The width and text-scale
matrix checks the empty message against the pane center while retaining toolbar clearance, card
geometry and pending-member feedback. The isolated Profile client showed the corrected placement
in exact, similar and ignored views, with no new Flutter exception or overflow. The page and shared
viewport contain 392 and 68 production lines; the layout fixture contains 410 test lines.

### Album sidebar refresh continuity, 2026-11-17

Album navigation retains its published page during dependency-driven reloads after edits. A
controlled creation/read fixture reproduced removal of the existing `First` row while the next
album page was pending. The corrected case retains that element, its bounds and the old count
without a loading bar, then publishes all three albums and the new count together.

Formatting, scoped analysis and four sidebar-selection and creation/undo checks passed. The
isolated Profile client created two albums consecutively; the first row stayed in place as the
second appeared and the count changed. No new Flutter exception or overflow was logged. The
previous client exited normally with all 700 reusable fixture lengths and times unchanged.

### Segmented settings groups, 2026-11-17

Settings groups now use the segmented composition specified in
[the settings contract](../product/gallery/settings.md#settings-page). The official AndroidX
`ListItemDefaults`, `ListTokens` and `ShapeTokens` establish the two-pixel gap, four-pixel interior
corners and sixteen-pixel outside corners. Installed `material_ui` 1.11.0 has no segmented-list
component; its existing Card shape/clipping and ListTile controls compose the required geometry.
The existing surface fill and row contents remain unchanged.

Formatting, scoped analysis and all 19 existing settings-page, album-settings and analysis-settings
checks passed. They cover selection, saving, disabled controls, keyboard semantics and storage
feedback. The Flutter-only Profile build completed with the receipt-verified native library reused.
The isolated Windows client exercised theme selection in both appearances, toggled and restored
the album prompt switch, and scrolled through single- and multiple-row groups, organization records
and storage rows with longer content. Separators are absent; gaps and outside/interior corners are
consistent, with no new Flutter exception or layout overflow. The previous client exited normally
with all 700 fixture lengths and modification times unchanged. The updated client remains open on
the light Settings page.

### Navigation appearance, 2026-11-17

Primary rail, menu and Settings symbols use 24 logical pixels. The brand asset box is 36 pixels,
with a 20-pixel wordmark. Navigation destinations use the component's standard padding.
Installed `material_ui` 1.11.0 and the upstream NavigationRail implementation use four pixels
between the indicator and label, plus 12 pixels below each destination. The earlier additional
12-pixel separation is removed. Selected destinations provide the filled Material Symbol through
`selectedIcon`, retaining unchanged symbol weight and size.
Search retains the component's `surfaceContainerHigh` background default.

Navigation uses the shared selection pair in
[ADR 0029](../architecture/0029-functional-workbench-navigation.md#material-surface-composition).
Light appearance uses a deeper accent container and dark appearance uses a more chromatic container;
both blend 40% of that accent into the default secondary container. The full-strength containers
were too prominent. Both retain colors derived from the existing system accent. All five theme
checks passed again after the blend adjustment. The contrast case covers five accent seeds in both
appearances, including eight- and twelve-percent selection state layers, with a minimum 4.5:1 text
contrast requirement.

Owned-file formatting and scoped analysis passed. The isolated Profile client reused the verified
native library and exercised Gallery, Albums, Duplicates and Settings navigation. Light and dark
appearances retained centered, unclipped symbols and branding. The preceding client exited
normally and preserved all 700 reusable source file lengths and modification times. The updated
client remains open; no new Flutter exception or layout overflow was logged. The spacing and fill
follow-up exercised all three destinations and return from Settings: only the active destination
uses its filled symbol, and theme colors remain correct in both appearances. The initial attempt
to set fill through partial icon themes lost the default colors; `selectedIcon` avoids that override.
The selection-color follow-up checked Gallery and Settings in both appearances, then imported the
existing 700-file corpus and checked source-row name/status contrast, Favorites, and exact-group
navigation in the isolated client. Primary and contextual navigation use the same selected palette;
switching destinations restores the previous row. No new Flutter exception or overflow was logged.
That client exited normally with the 700 source lengths and modification times unchanged. The final
40% blend was checked in Gallery and Settings in both appearances; that isolated client remains open.

The affected workbench, navigation and source-row owners contain 364, 557 and 546 production lines
with no inline tests. The retained workbench navigation and semantics fixtures contain 215 and 213
lines; the updated theme fixture contains 160 lines.

### Standard Material surface roles, 2026-11-17

The follow-up to `de1e6272` uses Material's large-screen bright/dim roles for content and outer
navigation, with `surfaceContainer` between them, as specified in
[ADR 0029](../architecture/0029-functional-workbench-navigation.md#material-surface-composition).
These roles remain fixed across both appearances. Search uses the installed component's default
background. The existing theme fixture checks foreground contrast across all three regions.
All five theme checks, owned-file formatting and scoped analysis passed.

The Flutter job in [run 44066385574](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44066385574)
at `7907172c` found one obsolete shell expectation for `surfaceContainerLowest`. The production
composition follows the accepted `surfaceBright` role; the shell assertion now checks that role
while retaining rounded clipping, sidebar transition and content-inset checks. All five shell
checks, formatting and scoped analysis passed. This correction changes no product behavior.

The isolated Profile client reused the receipt-verified native library with a rebuilt Flutter
payload. Gallery and Settings were checked in both appearances, including sidebar collapse and
expansion. Both appearances showed distinct outer, contextual and content regions; the search
background remained distinct from the surrounding chrome. No new Flutter exception or layout
overflow was logged.

### Empty-result and selection continuity, 2026-11-17

The follow-up to `ac88a70e` keeps published empty results mounted while a query refreshes. The
viewport previously removed them for every loading interval, including reads too short to show
loading feedback. The published snapshot and heading now remain together until the next result
arrives; only the delayed top line accompanies a refresh. Initial reads retain the centered indicator.

Sidebar selection keeps `已选择 (N)` and its actions in one row. At compact widths the label uses
`已选 (N)` instead of truncating its meaning. Parenthesized counts retain the normal body-small
typography; Select all uses the shared circular outline. Creation returns after selection clears. This
supersedes the two-row arrangement below, which changed the list's vertical position on selection.

Formatting, scoped analysis and 15 focused checks passed. Production-composition coverage retains
the same empty-result element and rectangle through short and longer query reads, then replaces it
with an empty album and populated gallery. Existing checks cover loading geometry, initial waiting,
sidebar motion at normal/enlarged text, review notices, and source/album selection at 220 pixels.
The album check now asserts unchanged row bounds and a shared count/action centerline.
The header refinement passed four affected checks again; its final width/typography cases confirm
the count style is identical before and after selection and that resizing retains selection.
Empty-result coverage includes consecutive
passive timeline refreshes with an idle query-activity state, matching the background synchronization
path, as well as an explicit collection change.

The Profile build completed in 78.9 seconds. The isolated Windows client imported the existing
700-file corpus and exercised source selection, album creation, Favorites/album navigation, minimum
sidebar width, section collapse/reopen and Escape. The selected row retained its position, the
heading stayed on one line, and the empty result remained visible through sidebar operations and
background updates. Native semantics exposed the full selected-count label. No Flutter exception
or layout overflow was logged. The updated isolated client remains open; current-head CI is pending.
The final header build completed in 47.3 seconds. Native inspection confirmed the full and compact
labels at 260/220 pixels, consistent count sizing, and cancellation through the circular Select all
control. The preceding isolated client exited normally with all 700 source file lengths and times
unchanged; the final client remains open with a temporary album.

The screen composition, viewport and section header contain 2,010, 65 and 155 production lines,
with no inline tests. The screen change only supplies the existing snapshot-publication state;
rendering remains in the viewport owner. The affected loading-geometry and album-selection test
files contain 239 and 149 lines.

### Compact creation entries, 2026-11-17

The follow-up to `41c5d564` restores folder import and album creation to the shared section-heading
plus controls and removes the dedicated large-action component. Library/Favorites now start at the
same top inset as Duplicates navigation. Existing import admission, album revision checks and section
disclosure remain connected to their original owners.

Formatting and scoped analysis passed. The existing creation/undo, import-browsing, navigation,
selection-focus and album-selection cases passed together (27 checks). Import-browsing retains its
disabled-action assertion with the restored IconButton type. Native inspection reproduced a vertically
squeezed selection count at the 220-pixel sidebar minimum; that revision placed the count above the
complete action row, superseded by the single-row continuity follow-up above. The album-selection
workflow ran at that minimum, checked readable count height and retained cancellation, selection,
collapse/reopen and removal assertions; it passed.

The final Profile build completed in 88.3 seconds. Native checks opened the folder picker through
the heading plus and imported the existing 700-file corpus, then created an album through the
corresponding album control. The completed client exited normally and preserved all 700 source
lengths and modification times. The final client confirmed creation, readable selection counts and
retained selection through section collapse at the 220-pixel minimum. Both normal and minimum
widths retained the compact creation controls and the aligned first navigation row. That client
subsequently exited normally and preserved all 700 source lengths and modification times.
Commit `ac88a70e` passed [Quality / CI run 44018820656](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44018820656).

The navigation, album, section-header and reorder owners contain 553, 286, 138 and 134 production
lines, with no inline tests. The retained navigation, creation/undo, import-browsing, selection-focus
and album-selection fixtures contain 1,093, 112, 105, 91 and 131 dedicated test lines respectively.

### Hosted qualification, 2026-11-17

Implementation head `d4b1c66738d5cd6879aa1fa63432e1c3ff7b82ae` passed
[Quality / CI run 43946356308](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43946356308):
Static, all four Rust groups, all 1,866 Flutter tests, Windows Scan, Windows Accessibility,
Windows production-bundle verification and the aggregate Windows Gate. All ten native UIA phases
completed, with normal process/job retirement and scratch removal. The closeout changes only this
record and the roadmap; product, test, tool and workflow inputs retain that verified head's contents.

### Navigation density refinement, 2026-11-17

The follow-up to `1323a3ea` gives Library, Favorites, folders, albums, smart albums and duplicate
destinations the same 48-pixel minimum row height, body-large labels and 24-pixel icon column.
Source status remains body-small and centered with its name. Source actions consume their actual
width; empty rows share the navigation rhythm. The first duplicate destination follows the
primary-action centerline. The existing selection shape, color roles and motion remain unchanged.

Scoped formatting and analysis passed, followed by 36 existing navigation, album and retained
date-navigation checks. The final Profile build completed in 62.7 seconds. Native verification used
the existing 700-file isolated corpus: source selection/menu, album creation and selection, Favorites,
light/dark appearance, Settings, Duplicates, sidebar folding and maximize/restore. Selection rows and
trailing controls aligned; folded content retained its gutter and search-width cap. No Flutter
exception or layout overflow was logged. The completed client exited normally and preserved all
700 source lengths and modification times; its scratch lease was retired before opening the update.

The affected navigation, source, album and smart-album owners have 556, 544, 289 and 213 production
lines, with no inline tests. Existing navigation-semantics and smart-album fixtures contain 1,093
and 384 dedicated test lines. No query, persistence, image-painting or animation lifecycle changed.

### Tonal and typographic refinement, 2026-11-17

The follow-up to `fca882dc` uses the system accent's tonal-spot palette and separates
primary-container sidebar actions from secondary-container navigation selection. Matching
on-container foregrounds preserve contrast; regular workspace headings and navigation text leave
emphasis on primary actions and selected labels. The existing three surfaces, row shapes, gaps,
search-width cap and motion/rendering owners remain unchanged.

Formatting and scoped analysis passed. All 16 existing theme, retained-workbench and duplicate
layout checks passed, including contrast in both appearances, 400/800/1200/1280 widths, doubled
text and bounded preview retirement. The Profile build completed in 45.7 seconds and reused the
unchanged receipt-verified optimized native library with isolated storage and in-memory preferences.
The native client imported the existing 700-file corpus, switched light/dark appearance and selected
a folder, created and selected a temporary album, collapsed/reopened its section, switched to
Duplicates, selected a group and maximized/restored without losing that selection. Gallery sidebar
folding retained the outer inset and search cap. Viewer entry, overlay-filmstrip display and return
retained the current image and gallery position. No Flutter exception or layout overflow was logged.

Run `43937512673` at `83faa357` exposed six obsolete color expectations: four system-accent
scenarios still used the superseded fidelity palette, and the two selected-row menu cases expected
the former primary foreground. The expectations now follow the accepted tonal-spot and
on-secondary-container roles. Run `43940151561` at `b5643406` passed those scenarios and reported
one remaining old semibold expectation for the Settings page title. That expectation now uses
the accepted regular workspace-heading weight; section and setting labels retain their emphasis.
All 38 existing application-theme, menu and Settings checks passed locally, retaining startup/update
ordering, fallback, popup isolation, disabled-menu behavior and Settings operations. These follow-ups
change tests only; the native client and visual evidence above remain applicable.

The modified theme owner has 114 production lines, navigation 554, source tile 551 and workbench
345, with no inline tests. Their existing theme, workbench and review-layout fixtures have 142,
215 and 393 dedicated lines respectively. This refinement adds no query, lifecycle or animation work;
the completed filmstrip frame-time evidence below remains applicable.

### Workbench implementation and qualification

Scope: [Material workbench plan](../plans/material-workbench.md), branch
`codex/material-workbench`, starting from `88b5cc2c`. The installed Flutter 3.47.8 / Dart 3.13.7
and `material_ui` 1.11.0 supply the SearchBar leading/trailing slots, circular IconButton shape,
NavigationRail, Material ink and rounded content surfaces. No package or native contract changed.
The [Android canonical layouts](https://developer.android.com/develop/ui/views/layout/canonical-layouts)
inform the supporting-pane composition; Ame retains its existing desktop actions and state owners.

MW1 resolves navigation, chrome and content from the current color scheme. The frame owns a single
continuous top/sidebar backing surface. Search options are trailing, the short workbench title is
`Ame`, and the settings selection is circular. The retained gallery and sidebar animation drivers
are unchanged. Workbench composition has 322 production lines after extracting the 63-line frame;
the surface role owner has 34 lines. These owners contain no inline tests; the existing dedicated
workbench test has 171 lines.

A current Debug Windows build completed in 146.8 seconds. Its optimized native build uses Rust
debug assertions and a DLL-hash-bound isolated-storage receipt. The existing `build/test/sources/mixed`
corpus supplied 10,000 images to a leased temporary profile; no real library was opened. The actual
client showed three distinct surfaces in light and dark appearance, trailing search choices,
keyboard text input, source import, sidebar show/hide, Albums/Favorites navigation and circular
Settings selection. The title and continuous background correction were hot-reloaded in that client.
A Material ink assertion exposed the need for an inner transparent sidebar Material after removing
the old opaque backing; that ownership was restored and the next reload completed without it.

The focused theme hierarchy/contrast, retained-workspace, search focus and sidebar right-edge
checks passed before the backing-surface change. The updated frame is checked against the retained
workspace/search/edge suites: all 12 checks passed, with an interactive album ListTile included in
the retained-workspace fixture. The first new search assertion used the wrong literal (`全部图库` instead of the existing
`整个图库`); correcting the assertion preserved the product wording. Later slices and integrated
qualification are tracked only in the roadmap.

MW2 keeps destination changes in `LibraryDuplicateWorkspace` and moves only their presentation to
the contextual sidebar. Its compact fallback uses the installed NavigationBar. Group image and
details composition has its own 99-line owner; the existing 263-line member-card owner retains
viewer launch geometry and file actions. The page has 396 production lines, with no inline tests;
the dedicated 387-line layout fixture covers the shared card/grid boundary. The group grid requests a 144-pixel logical preview
for visible representatives and one prefetched row, retaining immediate retirement when hidden.

The 18 focused review checks passed across 400/800/1200/1280 widths, ordinary/doubled text,
continuous group/member paging, late responses, preferred decisions, ignore/restore, similarity
degree, selection and comparison. The preview-demand fixture scrolls by actual card rows;
the compact cards request nine representatives across visible rows and one prefetched row.
In the same isolated Windows client, 72 exact groups and six standard-similarity groups rendered
with the new sidebar. The degree popup retained its existing choices and neighboring cleanup action.
The compact-card refinement retains the established widths and uses a square preview beside its
details. A fresh isolated client using the existing 640x480 corpus subset displayed six groups in
two columns, with approximately 148-pixel card height. Clicking the image retained its pixels and
added a circular check and thin card outline. A separate 20-pixel end gutter kept the native
scrollbar outside both group cards and member rows. The earlier client exited normally and preserved
all 10,000 fixture files by size and modification time. No source cleanup operation was executed.
The final nine layout checks passed again after the square-preview composition, covering retained
preview state, square bounds, compact height, separate scrollbar space, paging and 2x text.
The actual client also retained preferred-file decisions and comparison selections through preview
and comparison returns, and showed the ignored group in its contextual destination before restoration.

MW3 uses the shared canvas/chrome roles and puts the rounded, elevated strip in its existing retained
slot. The slot has 91 production lines; the viewer, strip and frame have 336, 375 and 159, without
inline tests. The existing viewer and strip fixtures have 1,161 and 732 lines respectively; their
geometry assertions now account for the strip's insets. No source, zoom, copy or paging policy changed.
All 31 focused viewer/strip checks passed, including unchanged image transforms through toggling,
full-screen action retention, decoded-pixel retention, bounded paging, held-scrollbar rebasing,
hidden request retirement and reduced-motion scrollbar input.

The isolated client showed the straight canvas and rounded strip in both appearances. At 100%,
opening the strip kept the main image in place; 125% zoom, rotation, full screen and return preserved
their existing behavior. Selecting the last strip image updated both image and ordinal. Gallery and
duplicate-member preview returns restored their original surfaces. The More menu retained its copy
entries. Normal, Windows snapped and maximized workbench layouts showed no content overflow; the
compact and doubled-text boundaries also passed their existing widget fixtures. Both owned client
runs exited with code 0, preserved all 10,000 fixture files by size and modification time, and retired
their leased temporary profiles. The final client logs contained no Flutter exception, assertion or
overflow diagnostic. Hosted verification and integration disposition remain in the roadmap.

The sidebar refinement uses `material_ui` 1.11.0 `FilledButton.icon` for `添加文件夹` and
`新建相册`, retaining the original picker/editor callbacks and disabled states. The installed
`filled_button.dart` exposes nullable activation and style; its button and icon children retain
framework focus, semantics and variable icon weight;
the [Material button reference](https://m3.material.io/components/buttons/overview) was consulted,
but its script-only response did not establish SDK capability. No replacement input handling or
dependency was added. The shared action has 52 production lines; source navigation, album navigation
and source tiles initially had 551, 254 and 534 lines, with no inline tests. Existing import, album creation
and album-selection fixtures had 105, 112 and 82 lines. Their six focused checks passed, including
disabled import with retained browsing, album creation/undo and selection/removal cancellation.
The obsolete assertion placing creation beside removal was removed with that layout requirement.
Sidebar navigation now uses framework stadium shapes and smaller minimum heights; text retains
its scaling behavior. The existing navigation/scroll fixtures have 1,093 and 332 lines; their
focused run with album selection passed all 25 cases after the shape and height changes.
The later header/disclosure refinement moves the toggle to the global hamburger, places the plain
mark beside `Ame`, aligns search with the content boundary and adds independent album section
disclosure. The navigation reference was consulted; its script-only response does not establish
component support. Installed framework SizeTransition/FadeTransition and the existing Ame motion
owner supply row animation; the original lazy reorder scroll remains intact. The focused
album-selection, smart-album and viewer-geometry run passed nine checks. The retained-workspace and
duplicate-viewer semantics fixtures passed four checks, including reduced motion. Current workbench,
album navigation, reorder list and row disclosure owners had 326, 271, 133 and 69 production lines;
their existing workbench and album-selection fixtures had 185 and 95 lines. These owners have no
inline tests.

The filmstrip repaint fixture reproduced six extra main-canvas paints across six animation frames.
The installed framework [RepaintBoundary](https://api.flutter.dev/flutter/widgets/RepaintBoundary-class.html)
now separates moving and static painting without changing the transition. The new 70-line regression
and existing pixel/paging fixtures passed all 18 checks: unchanged image surfaces do not repaint
during movement; source replacement, decoded-pixel retention, paging and hidden-demand retirement
remain covered. The slot has 95 production lines and no inline tests.

A third Debug client exited with `0xC0000409` on 2026-11-17; Dart reported an out-of-memory abort.
It had received a user-initiated real-root import and repeated hot reloads. The preceding result file
belonged to an earlier successful run and does not cover that exit. No system resource-exhaustion
event was returned by the scoped check; the allocation cause remains unconfirmed. A new Debug build
completed in 27.5 seconds and opened a fresh isolated profile. Its header/brand screenshot reflects
the refinement; the subsequent sidebar and filmstrip interaction checks are recorded below.

The first hosted run for `01abdfdf` reported `use_null_aware_elements` in the workbench frame.
The optional top-bar child now uses the corresponding null-aware element; focused analysis of
that owner reports no issues. Flutter also reported obsolete long-title/surface expectations and
loss of the return card when a hidden provisional gallery resized from 1000 to 980 pixels. Geometry
publication now remains active while hidden; preview demand remains retired. All five existing
viewer-geometry cases and all five updated shell cases passed without weakening return-position
assertions. The gallery owner has 1,597 production lines and its dedicated geometry fixture has
488 lines, with no inline tests.

The `cd7946a6` hosted run reported two test assemblies missing the required global-bar builder,
an unavailable duplicate-navigation target, and undeclared preference consumers in the duplicate
layout. The assemblies now provide the global bar; duplicate visibility uses its layout constraints
consistently with the parent workbench, and the two exact preference imports are registered under
ADR 0037. All 15 focused workbench, disclosure, retained-semantics and duplicate cases passed.
Dependency verification covered 623 modules with no frozen exceptions; changed-file analysis found
no issues. Current-head hosted results remain the delivery gate.

The subsequent boundary check reproduced the sidebar's rectangular material painting over the
shared backing's lower-left corner. The sidebar now renders on transparent Material. It also found
the brand drag region overlapping the hamburger target; the drag region now starts after that target.
The production-shell regression exercises the actual hamburger, immediate movement, eight-pixel
collapsed inset, search alignment and expansion. Its five existing cases passed. A disposable
production-widget render checked expanded, collapsed, Settings and transition geometry in both
themes: at 1280 by 800, the content's collapsed/Settings bounds are (80, 64) to (1272, 792), and the
transition's right and bottom edges stay fixed. All eight renders preserve complete content corners.
This render uses test bindings and does not establish native desktop input or font fidelity.

Primary-destination geometry now has a separate 154-line transition owner. Workbench, album
navigation and row disclosure have 340, 287 and 81 production lines; their dedicated workbench,
album-selection and shell fixtures have 215, 123 and 367 lines, with no inline tests. The workbench
case covers rapid destination reversal, retained scroll/state and stable settings layout during the
transition. The album-selection case now also checks the empty row's intermediate disclosure height.

The isolated Debug client received the `e75cd95f` Dart payload through attach and restart. Resetting
the desktop helper resolved its `foreground window did not report a process id` input failure.
Native checks covered Gallery and Duplicates sidebar collapse/expansion, manual/smart album empty
disclosure, Settings width changes and light/dark surfaces. Collapsed content retained its left
gutter and complete corners. Importing the existing 700-image 640x480 subset produced six merged
representatives. Viewer filmstrip show/hide preserved the image position and 100% zoom; thumbnail
selection and viewer return remained functional. This run used isolated derived storage and its
interaction logs contained no Flutter exception or overflow. It later exited with code 0, preserved
all 10,000 fixture files by size and modification time, and retired its scratch profile. Shutdown
logged two AXTree update errors; no corresponding failure occurred during the interaction checks.
The later Profile qualification below covers the same filmstrip implementation.

The brand sizing check found that the image's default scale-down fit kept the 28-pixel asset small
inside larger layout bounds. Contain fit now uses the requested 44-pixel box; the density asset's
visible silhouette occupies approximately 32 pixels. Native inspection confirmed the larger mark
beside the 22-pixel wordmark and its alignment with the primary action. Folder, manual-album and
smart-album headings now have 24 pixels above them, while the action retains 32 pixels below and
the row spacing is unchanged. The new-album button still opened the existing editor and cancellation
returned to the same sidebar.

The `e75cd95f` Flutter job failed four sidebar geometry expectations that still assumed no collapsed
gutter. Their expected boundary and destination widths now include the accepted eight-pixel inset;
fixed-right-edge, hit-target, no-relayout, reversal and reduced-motion assertions remain in place.
The affected four cases and six existing application/album cases passed locally. Analysis of the
changed Dart owners reported no issues. The same hosted run passed Static, Windows Scan and Windows
Accessibility; remaining/current-head hosted results stay with the run and roadmap.

The compact-sidebar refinement aligns navigation backgrounds with the primary-action background,
using 16-pixel side insets and shared eight-pixel corners for rows and drag proxies. Existing icon
and heading alignment is retained. Single-line minimum height is 40 pixels; status rows retain 56.
The three primary destinations have 12 additional pixels between entries, and both sidebar states
cap search at 720 pixels. The existing sidebar selection, drag, navigation-semantics and application
fixtures passed all 30 cases; scoped analysis reported no issues. The updated Debug client built in
37.2 seconds. Native Profile inspection confirmed aligned primary-action/selection backgrounds,
the 720-pixel collapsed search cap, and the retained content gutter. Folder names and statuses now
center as one text block, the heading shows the imported-root count, and section controls share the
row-menu trailing column. Group preview/detail slots align at the top while actions remain below.
The updated nine card-layout cases passed, including 400-pixel width with doubled text. Selection
retains preview state, size and position within its card; a wrapping toolbar may shift the whole grid.
Source-tile and navigation owners have 551 and 554 production lines with no inline tests; the
existing dedicated card-layout fixture has 393 lines.

The 78.5-second Profile build was inspected with the existing 700-image corpus in isolated storage.
The folder heading changed from zero to one after import, its selected status row stayed centered,
and exact groups retained compact previews, selection and detail actions. The client exited with
code 0 and preserved all 700 fixtures by size and modification time; its scratch profile was retired.
Two filmstrip show/hide cycles retained the 100% image bounds. At 165 Hz, the capture contained
340 frames, including hover/tooltip frames: UI p95/p99/max were 0.619/7.600/10.679 ms; raster values
were 0.763/5.554/24.084 ms. Four UI frames and three raster frames exceeded the 6.061-ms budget.
The four interaction bursts had UI p95 0.487–0.720 ms and raster p95 0.710–0.842 ms; these timings
do not establish display FPS or remove the retained occasional slow frames. RSS changed from
255.8 MiB to 264.3 MiB, peaking at 269.5 MiB. No Flutter exception or overflow was logged.
The final 61.1-second Profile build also aligned section disclosure with the trailing menu/download
column in the native window; that isolated client remains open for inspection.

The `0f7483ad` hosted Flutter failure asserted absolute preview bounds after the compact doubled-text
toolbar wrapped. The fixture now protects card-relative placement and retained size/state; all nine
layout cases pass. The Flutter, Static and Accessibility jobs passed on `fca882dc`; that result
does not replace its earlier intermittent initialization failure described below.

### Native accessibility preparation follow-up, 2026-11-17

Run `43920047435` stopped at the eight-second native-semantics-ready deadline before window discovery
or assertions. The last record was `loading-assemblies` at 44 ms, while runtime preparation had
already loaded both UIA assemblies in 147 ms. The native activation helper still compiled through
`Add-Type` on the first application phase, after the client started. Its existing idempotent
initialization now runs in the owned preparation process and records that cost separately under
`loading-assemblies`; the same process retains the compiled type for native phases. Preparation
continues to observe zero windows/elements. All phase assertions, the eight-second per-phase limit,
the whole-run clock and cleanup ownership remain intact.

The real preparation regression passed in 616 ms, including compilation, and rejected an application
phase. The persistent-session fixture completed its first/repeated preparation in 580/17 ms and
rejected mismatched ownership, reported failure, partial results and a blocked invocation while
retiring the owned process. The preparation owner has 51 production lines, no inline tests and a
66-line dedicated fixture. The Windows Accessibility and Windows Scan jobs passed in run
`43940151561` at `b5643406`. The earlier timeout's underlying operating-system/compiler scheduling
cause is not inferred.

Run `43943022745` at `c1fb5622` passed the complete Flutter suite, Static and Windows Scan.
Its UIA worker exited 245 ms into `settings-menu-open`, before producing that phase's record;
the preceding eight native phases passed. The session now retains the worker exit code and bounded
exception position in its owned failure evidence. Controlled malformed-request and exit-code-23
cases verified diagnosis, ownership rejection and process retirement; retained preparation took
588/24 ms. Native assertions, deadlines and failure status are unchanged. All ten native phases
passed at `d4b1c667`; the worker exit did not recur. Its original cause remains unconfirmed, with
the bounded diagnostic retained for a recurrence.

The `0f7483ad` Profile client uses the unchanged receipt-verified optimized debug-assertion native
library and in-memory preferences. An initial fresh catalog failed with `unable to open database
file`; an independent SQLite write in the same leased directory succeeded. Confined process-level
TEMP/TMP inside the leased profile allowed the same client to initialize normally. This observation
isolates a temporary-directory dependency but does not identify its underlying Windows error.
No product storage policy or global environment setting changed. The failed client exited with
code 0 and preserved all 700 source fixtures; its scratch profile was retired.

Date: 2026-11-08

The [roadmap](../roadmap.md#current-execution-queue) owns delivery status;
[ADR 0001](../architecture/0001-flutter-rust-desktop-stack.md#standalone-material-library) owns the
package, compatibility and rollback decision.

## Scope and provenance

Baseline: `27bde80b`; branch: `codex/material-ui-migration`. The official installed Dart fix
`migrate_design_widgets` previewed and then migrated 312 files: 150 production files, 149 test files
and 13 integration files. Its unconstrained dependency was replaced with `material_ui` 1.10.0;
the resolved Cupertino dependency is 1.1.6. Both installed packages retain BSD-3-Clause notices.
Other package versions, Flutter 3.47.8/Dart 3.13.7, Skia, native contracts and storage are unchanged.

The shared Chinese localization delegates now come from the standalone Material package. Used
Riverpod consumers and window-manager gesture/platform APIs do not read legacy Material theme or
localization state. No compatibility bridge or replacement control was introduced.

An attempted application language-version increase to Dart 3.13 changed unrelated formatting and
rejected an existing generated Freezed formal parameter. The application retains its prior language
mode; the new package constraints and Flutter minimum enforce the compatible SDK. Generated output
was not edited. Only owned files were formatted after restoring that language mode.

## Verification

The presentation static entrypoint passed: formatting, dependency checks for 547 modules with zero
frozen migration exceptions, and Dart analysis with no issues. The existing dependency fixture now
admits both standalone libraries in presentation and rejects them in adapters, including a private
Cupertino import. Existing core-layer rejection and composition policy remain.

All 56 focused checks passed across the existing application/localization, theme, menu, checkbox,
filter, search and dependency-boundary suites. The 11 existing image-viewer checks also passed,
including wheel animation, actual-size persistence and panning bounds. Diff review found 310 Dart
files changed only by import replacement and formatting; the remaining Dart owners are localization and the boundary
rule/fixture. No generated bridge or native source changed.

The first Windows build stopped at the existing memory guard during Rust compilation. The previous
successful build's complete input digest was verified against the unchanged baseline, and both
native DLL copies matched its `24f7f44d` SHA-256 prefix and isolated-storage configuration. The current
Flutter payload was then rebuilt with `flutter_assemble` and installed into that verified Debug
bundle under the same resource guard. This local check reuses native output; fresh complete builds
remain the hosted CI owner's responsibility.

The isolated native client passed dark/light theme switching, shared menus, the bottom filter tray,
Chinese date selection, applying an empty date result and clearing back to 368 images, checkbox
selection and Escape, viewer next-image/Escape, and maximize/restore/minimize. Windows IME candidate
composition retained the gallery; committing the Chinese text submitted the search. In this
intentionally model-free profile, acknowledging the missing-model feedback cleared the search.
The client closed normally with exit code 0; all 368 retained preview hashes were unchanged, and
the scratch profile was retired. No Flutter exception or layout-overflow diagnostic was logged.

A second isolated run at `ab512fd8` assigned quarterly dates from 2022 through 2025 to 368 temporary
preview copies. Timeline clicks and drags reached October 2022, 2024 and 2023 without reverting after
release; gallery scrolling updated the indicator. Viewer wheel zoom, slider dragging and actual-size
reset changed the rendered scale. Filmstrip dragging reached both ends without changing the main
image; selecting ordinals 365 and 1 displayed the matching image and retained the strip position.
Closing and reopening the viewer preserved the gallery location. This client also exited normally,
retired its scratch profile and preserved all 368 retained preview hashes, with no Flutter errors.

Native image-panning input did not produce an observable displacement in the automation tool. The
existing gesture/bounds test passed; sustained desktop image dragging remains unconfirmed.

## Integration disposition

[PR 54](https://github.com/Cedarflake/Cedarflake-Ame/pull/54) integrated reviewed head `da113d4d`
into `main` as `f907b033` on 2026-11-08 after explicit authorization to merge before CI completion.
The merged file tree exactly matches the reviewed head. Review against `27bde80b` found no confirmed
new defect; the desktop panning verification limit above remains.

At merge, the [implementation run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42448216115)
at `ab512fd8` had passed Static, Flutter Tests, Windows Scan and Windows Accessibility; Windows Build
Verification and all four Rust groups were still running. The
[PR run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42449217822) at `da113d4d` was also
running. Only acceptance documentation differs between these heads. Their Actions results retain
the eventual hosted outcome; local native-output reuse does not establish a fresh Windows build.
