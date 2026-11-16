# Standalone Material migration

## Material workbench composition, 2026-11-16

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
album navigation, reorder list and row disclosure owners have 326, 271, 133 and 69 production lines;
their existing workbench and album-selection fixtures have 185 and 95 lines. These owners have no
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
the refinement; the user retained control for inspection. Final sidebar click-through and filmstrip
frame-time qualification are not yet recorded.

The first hosted run for `01abdfdf` reported `use_null_aware_elements` in the workbench frame.
The optional top-bar child now uses the corresponding null-aware element; focused analysis of
that owner reports no issues. Flutter also reported obsolete long-title/surface expectations and
loss of the return card when a hidden provisional gallery resized from 1000 to 980 pixels. Geometry
publication now remains active while hidden; preview demand remains retired. All five existing
viewer-geometry cases and all five updated shell cases passed without weakening return-position
assertions. The gallery owner has 1,597 production lines and its dedicated geometry fixture has
488 lines, with no inline tests. The current-head hosted rerun remains the delivery gate.

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
