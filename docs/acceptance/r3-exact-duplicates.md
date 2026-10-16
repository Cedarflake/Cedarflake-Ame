# R3 exact-content verification

Status: R3-01 through R3-04 client checks recorded; affected Actions verification remains open.

Date: 2026-10-16.

The [execution plan](../plans/r3-exact-duplicates.md) owns the live checkpoint. This record covers
the implementation on `codex/r3-exact-duplicates`. The initial controlled run began at `aeaec3d`;
later sections cover subsequent deliveries. It does not close R3.

## Controlled client run

The existing `build/test` corpus contains 10,000 files, 10,921,494,393 encoded bytes and 72 expected
exact-content groups. Dimensions, dates, encoded sizes, formats and non-uniform content vary.
The retained catalog was upgraded from schema 39 to 40 in place. No additional corpus was created.
Reads were limited to this corpus, one content file at a time, with a 256 KiB hashing buffer and a
256 MiB per-file limit. Derived writes stayed in `build/test/derived`.

The Debug client exposed a provisional group/location sheet. Observations:

- Cancellation retained 574 complete fingerprints and read 633,723,673 bytes. The next window
  observation, 657 ms after the stop input began, displayed the stopped state and retained groups;
  this is an observation bound, not an instrumented cancellation-latency measurement.
- Continuation processed exactly the remaining 9,426 files, read 10,287,770,720 bytes, skipped none
  and completed in 122,399 ms according to the persisted run timestamps.
- All 10,000 published digests and byte lengths matched the pre-existing manifest, yielding the
  expected 72 groups. The cancelled and completed runs together read each encoded file once.
- Closing the panel retained the running task. Gallery scrolling, opening an image in the viewer,
  returning, and reopening progress worked while analysis continued.
- A 210-location group displayed actual paths and local availability. Advancing its bounded location
  page displayed a different set of members. Automated boundary coverage remains assigned to CI.

The first run used a provisional sheet. ADR 0029 now governs the accepted function rail,
collapsible Library sidebar and dedicated Duplicates workspace; the later check below covers it.

The app exited after the run. Original settings, preferences and original catalog component hashes
were restored and verified. The profile lease, temporary profile copies and disposable preview
cache were removed after process retirement and workspace/reparse checks. The reusable corpus,
retained test catalog/fingerprints and deferred incident records remain intact.

## Automated coverage and remaining checks

Added regressions cover complete-content hashing, read failure/cancellation, stale publication,
hardlinks, multi-volume copies, group/location paging, result reuse, migration preservation and
rollback, async result ownership and workspace/task lifetime. Actions will execute them after submission.
Owned-file formatting, the repository formatting check and whitespace checks passed locally.

## Workbench client check

The updated Debug client built successfully and reopened the retained schema-40 test catalog.
The function rail opened Duplicates, loaded stored groups and displayed physical locations. Selecting
one location opened File Explorer at its containing fixture folder. The Library sidebar collapsed
while its toggle and the function rail remained reachable. Returning from Duplicates retained the visible
2012-03-04 date section and the collapsed sidebar state.

A user-triggered analysis after reopening completed in 156 ms according to its persisted run and
outcome timestamps: checked files 0, newly fingerprinted files 0, skipped files 0 and content bytes
read 0. Existing compatible fingerprints were reused. This does not measure the full app startup.
The run did not enumerate or rehash the source corpus.

The application exited, original settings/preferences and original catalog hashes were restored and
verified, and disposable cache/profile copies were removed. No new corpus was created. These
checks used controlled files; no real source roots or cloud placeholders were included.

Workspace retention, hidden preview-demand retirement, focus, narrow-window controls, persisted
sidebar preference and Explorer invocation regressions are included for Actions. Native narrow-window
resizing was not established by this check. The later gallery-mode check below covers folding and
badges. Durable preferred-copy/ignore decisions remain R3-04 in the execution plan.

## Automatic scheduling check

The Debug build with automatic scheduling reopened the same 10,000-file catalog. Without opening
the Duplicates workspace or requesting analysis, the persisted run completed in 146 ms, with
zero checked files, zero newly fingerprinted files, zero skipped files and zero content bytes read.
The existing 72 groups were retained. This measures the analysis run, not full application startup.

The closed-process incremental check then added two temporary copies of one existing 494,579-byte
fixture to the same source root. The ordinary Debug client synchronized both new locations and
automatically fingerprinted exactly those two files: 989,158 bytes, no skips, 127 ms analysis time.
Both published the same digest as their source. The remaining 10,000 fingerprints were reused.
After the two temporary copies were removed with Ame closed, reopening removed both locations and
finished analysis with zero files and zero content bytes read in 85 ms. The source fixture digest
was unchanged. These timings cover analysis only; no full startup/synchronization timing was taken.
The owned processes retired, original configuration/catalog hashes were verified and restored,
and the temporary copies, profile backups and preview caches were cleaned.

The scheduler has focused regressions for idle admission, browsing/synchronization cancellation,
foreground read leases, waiting for native retirement, late admission during shutdown, new
publications during older runs, paused restart and failure/retry policy. Catalog-delta regressions
exercise additions, in-place changes, replacements, moves and removals through normal publication,
including stale evidence rejection and unchanged-fingerprint reuse after reopening. Actions owns
their execution; they have not been run as a local CI suite.

The Windows interaction tool repeatedly failed with `foreground window did not report a process id`,
including after the window was manually brought forward. Pause/resume button interaction was
unverified in that run and was subsequently checked below. The owned idle test process was terminated for cleanup, so that run does
not verify graceful analysis shutdown. Original configuration and catalog hashes were restored and
verified; temporary profile backups and preview cache were removed, retaining the shared corpus
and test catalog.

For commit `b017ab1`, hosted Windows Scan, Unsigned Release and all five synthetic workloads passed.
Flutter reported two workbench tests that still searched for Flutter's raw `Tooltip` after the
control moved to the existing Windows-compatible `AmeTooltip`. Their actions now use the stable
sidebar-toggle key, and their label assertions inspect the shared tooltip. The native accessibility
check still fails after closing the Settings theme menu: Flutter's route is closed and the Settings
page is hit-testable, while native UIA reports an empty popup subtree. A reachable Flutter-semantics
assertion now distinguishes framework restoration from native export before the unchanged UIA
contract runs. Static and Rust subsequently finished with 1,911 passed, 64 failed and 24 ignored
tests. The failing logs include closed module-topology declarations, fingerprint-publication fixtures
and historical-schema fixtures; these are being traced to their owning contracts. The new head
requires its own Actions results.

The follow-up preserves the same inventory/live-gap DDL checks for schema 40 and 41, with unknown
versions still rejected. Historical fixture downgrades now remove the R3 evidence projection and
analysis schema before restoring older contracts. Incremental-analysis fixtures ask the catalog to
allocate distinct source generations, and module-topology expectations include the two declared
R3 modules with their exact visibility and attributes. Existing negative guard/marker checks remain
and now cover both R3 schema versions. Actions owns execution of these corrected boundaries.
The next run at `82af2f5` stopped before Rust tests because a fully specified historical
location-anchor query omitted the new `duplicate_mode` field. The fixture now explicitly retains
All images. Synthetic jobs also stopped in their Rust build phase; their workload results remain
pending. Windows scan and unsigned Release passed on that head.

## Gallery modes and automatic publication

The retained test catalog upgraded from schema 40 to 41 without reading source content. The
independent existing manifest has 10,000 files and 72 unique digest/length pairs. The updated Debug
client showed 10,000 entries in All images and 72 in Merge duplicates, with duplicate badges in
both modes. A 600-location badge opened a bounded path dialog; its location action opened Explorer
in the fixture folder. Closing the dialog retained the gallery position. Sorting and dates follow
the scoped representative, so switching modes can change the displayed date; hidden-member anchor
mapping is covered by the query regression, not claimed as an instrumented native anchor check.

Native pause changed the page to the paused state without removing stored results. After graceful
exit and reopening, pause and the merged display preference remained selected. Continue returned
to completed analysis. A subsequent check exercised gallery publication after new analysis:

- With analysis paused, two 494,579-byte copies were added within the same reusable source root.
  Ordinary synchronization admitted them, and the merged gallery showed 74 entries while they
  remained unanalysed.
- Continuing analysis read exactly those two files, 989,158 bytes, with no skips in 91 ms according
  to persisted analysis timestamps. The gallery returned to 72 entries through its committed query
  refresh without a manual library update. The following unchanged run read zero bytes in 82 ms.
  These durations measure analysis, not end-to-end synchronization or application startup.
- Both temporary copies matched the original fixture digest before removal; the original fixture
  was unchanged. The existing 10,000-file corpus and retained catalog are reused throughout.

Regressions cover atomic evidence admission and revision change, publication rollback, stale
cursors and annotation requests, scoped representatives, page/timeline/layout consistency, hidden
anchors, hardlinks, multiple volumes, changed/removed members, persisted display mode, bounded
annotation batches and retired responses. Analysis-to-gallery publication also covers terminal
stream retirement, retained zero-read evidence, coalesced completions and disposal. Actions owns
automated execution; local checks are formatting, diff review and the affected Debug client.

One earlier test process terminated in `flutter_windows.dll` at offset `0x3e755`, with Windows
Application events `0xc0000005` and `0xc000041d` after a native accessibility capture. A later process
completed the 74-to-72 publication and removal of both temporary catalog locations, then also exited
unexpectedly. Its stderr records rejected `ui::AXTree` updates, including an unreachable node and
nodes left pending. A fresh client reproduced the Settings-menu accessibility failure: after Escape,
the page remained visible, while UIA retained an empty popup subtree and the engine rejected an
unreachable node. Adding a workspace semantics boundary did not resolve it and was removed.
The earlier fixture did not include the gallery timeline. Framework inspection identified the
rejected node as the Material Slider's always-mounted value-indicator OverlayPortal. Adding the
actual timeline slider reproduced the unreachable-node failure in the serialized-update regression.
The slider now retires when its retained workspace disables TickerMode, preserving its externally
owned value and the gallery state. Both focused regressions pass, including three Settings-menu
open/close and gallery-return cycles with the same timeline owner and value.

The rebuilt ordinary Debug entrypoint also restored the full native Settings control tree after
closing the theme menu. Returning to the gallery retained the 2017 middle position; the client
remained responsive and exited on request, with no AXTree, FATAL or unhandled-error matches in
this run's stderr. On `515f5e6`, hosted Windows Accessibility, Windows Scan, unsigned Release and all
five synthetic workloads passed. Static validation stopped at two Dart informational lints before
Rust tests ran; Flutter reported the three gallery regressions described below. No cause is inferred
for the older deferred R2c incident.

The latest isolated client exited on request. Original preferences, settings and original catalog
hashes were restored and verified. An older terminated test descendant remains listed by Windows
with zero threads and zero handles; no additional desktop process was terminated for cleanup.
The reusable corpus and catalog, plus the logs of the native failure, remain available.

## Card content width

The gallery now measures the complete duplicate count using the active label font and text scale,
reserving its icon, button padding, selection target and a gap before row layout. The catalog's
total location count bounds the number of digits before annotations arrive. Both layout paths and
square mode use this floor, with a 128-logical-pixel base minimum. Annotation arrival and selection
visibility therefore do not change card geometry.

The existing Material `TextButton.icon` and `Checkbox` remain the action owners. The pinned Flutter
3.44.9 implementation provides a flexible label and an icon-label gap up to 8 pixels; the compact
checkbox fits inside the reserved 40-pixel target. Ame supplies the pre-layout measurement and
separate action slots. No replacement control or dependency was introduced. The official
[Material component index](https://m3.material.io/components) and
[TextButton.icon API](https://api.flutter.dev/flutter/material/TextButton/TextButton.icon.html)
were checked against the installed implementations.

Twelve focused widget cases cover one- through nineteen-digit counts at normal and double text
size, complete single-line glyphs, containment, separate selection targets and unchanged geometry
when annotations arrive or selection appears. Layout regressions cover both shapes, narrow
viewports, width-sensitive snapshot reuse and retained date anchors. These focused checks passed;
the complete automated suite remains assigned to Actions.

The ordinary Debug client reused the 10,000-file corpus. A narrow portrait card displayed its
23-location badge and checkbox fully. Selecting it changed the count to one; opening and closing
its location list preserved that selection and scroll position. The client exited on request with
no overflow, AXTree, fatal or unhandled-error matches in stderr. Original preferences, settings and
catalog hashes were restored and verified; the six disposable profile/cache targets were removed.
The reusable source corpus and catalog were retained.

## Card-anchored location inspection

Duplicate badges now use the installed Material `MenuAnchor` with a bounded, lazy location panel.
The panel separates filenames from folders, marks the current location and retains full-path
tooltips and exact-file Explorer actions. Framework positioning, dismissal and focus return remain
in charge. The official Material component index, Flutter MenuAnchor/RawMenuAnchor documentation
and Windows dialogs/flyouts guidance were checked against Flutter 3.44.9; ADR 0029 records the choice.

All ten focused duplicate workflow tests pass, including a 400-by-600 window at double text scale,
upward edge placement, Escape/focus return, outside-click consumption and retained semantics.
The ordinary Debug build succeeded and opened the retained 10,000-file test catalog. Launching its
normal visible window through the native app launcher restored automated activation. Clicking a
badge near the bottom opened the panel upward, displayed the current-image marker and returned focus
to the card on Escape. No user foreground handoff or new source corpus was needed.

## Group review and durable choices

The review workspace uses framework `Card`, `ListTile`, lazy `ListView`, `SegmentedButton` and
`MaterialPageRoute`, with shared preview tiles and the existing image viewer. The official Material
segmented-button guidance and the installed Flutter 3.44.9 constructor/selection API were inspected.
The two-state review filter uses its single-selection set directly; no replacement navigation,
selection or scrolling implementation was introduced. Copy is grouped in `LibraryDuplicateStrings`.

Schema 42 keeps content-keyed user choices outside derived fingerprint storage. Repository
regressions cover restart, in-scope fallback, changed/removed/replaced preferences, compatible
reanalysis with a new member, ignored filtering across 64/65-group boundaries, no-op revisions and
failed migration preservation. The bridge was generated by the canonical generator. Seven focused
controller/widget cases pass, including delayed reads/writes, failed saves, page retention across
publication, saving/paging interleaving, ignore/restore and opening/navigating/returning from the
full-window viewer. Seventeen
preview coordinator cases pass, including active review members outside the photo-wall window and
retired late completions. The ten existing duplicate-workflow cases also pass after connecting the
review service. Rust and complete automated verification remain assigned to Actions.

Native Debug checks reused `build/test`: the 72-group catalog upgraded without source enumeration;
setting the second member of a 210-location group as preferred changed the displayed representative.
Ignoring moved that group into the separate list, and both choices survived closing and reopening.
The saved preference could be cleared and the ignored group restored. Member previews outside the
gallery window initially remained placeholders; the preview queue's admission callback now uses
the active review demand and those previews render. The existing viewer opens originals across the
full window with one caption bar, moves to the next member with the keyboard and returns to the
group. No source-media write or placeholder download is part of this workflow.

The review follow-up prevents page navigation while a choice is saving: otherwise a completed save
could retire the new page's read while leaving its cursor advanced. Both controller guards and
disabled navigation buttons enforce this boundary; the controlled pending-write regression passes.
After native process retirement, original preferences/settings and catalog hashes were restored and
verified. Ten resolved profile/cache/log targets were removed; the reusable source corpus and
test catalog remain.

## Gallery ordinal recovery

The subsequent hosted resize suite exposed an outdated fixture input: its independently built
snapshots still used the default cell width while the mounted wall measured the count label with
the active test font. The affected widget fixtures now supply that same measured width, retaining
all pixel, identity, extent, wheel-order and stale-transition assertions. The focused resize file
passes all 25 cases. Product layout behavior is unchanged by this fixture correction.

The two hosted anchor failures reproduced locally. With fractional square-row extents, converting
a rendered row offset back to an item index selected the preceding row after floating-point
rounding. The captured location was item 67 while its ordinal was 62. The identity guard then
rejected restoration on a second layout change, and cancelling a pending sort fell back to the
first item. Loaded-window layout entries now retain their integer start ordinal; anchor capture
and target-row lookup use those ordinals directly. Identity and query-generation guards remain.

The third failing fixture compared row start offsets after dimensions were recovered. Under the
narrower workbench width, the cards changed widths within the same number of fixed-height rows,
so those offsets legitimately stayed equal. It now asserts changed rendered card widths and keeps
the existing wheel-position preservation checks. The 43-case affected screen file and six layout
cases passed locally, including fractional rows, date boundaries and both layout shapes. The two
Dart lint findings were corrected at their declarations. Full automated validation of this repair
belongs to the next Actions run.

Large existing owners were extended only at registration/dispatch boundaries: `sqlite_catalog.rs`
has 3,622 production lines, no inline tests and 8,009 lines in its existing dedicated test file;
`migrations.rs` has 9,848 production lines and 5,267 inline-test lines. New analysis, query,
publication and migration behavior has dedicated owners. The existing gallery shell remains the viewer/query owner; navigation and sidebar preference
ownership moved into `LibraryWorkbench`, and hidden-gallery preview retirement has its own widget.
The changed shell has 2,014 production lines and 4,147 dedicated-test lines; the gallery wall has
2,014 production lines and 1,738 lines in the affected resize/visibility test file. Neither owner
has inline tests. The workbench, analysis page and activity widget own their new responsibilities.
Card measurement and action layout have separate 53-line and 54-line owners, with 142 dedicated
widget-test lines. The photo tile now has 452 production lines and no inline tests.
The loaded-window layout owner has 425 production lines and 230 dedicated-test lines, with no
inline tests; it owns ordinal assignment and target-row lookup.
The topology-only edit in `local_files.rs` changes its inline test declarations; it retains 3,694
production lines, 3,512 inline-test lines and 436 dedicated-test lines without new production logic.

R3 review persistence has a separate 262-line owner and 329 dedicated regression lines; its migration
has 77 dedicated test lines. The review page has 316 production lines and its controller 169, with
260 dedicated controller/widget test lines. The shared preview coordinator has 317 production lines
and 727 dedicated test lines. These owners have no inline tests. Analysis status, review cards,
visible preview demand and viewer navigation are cohesive separate presentation owners.

## Hosted closeout corrections

The `36b11e8` run passed Windows Scan and Windows Accessibility. The Rust test build stopped at
two review-test calls that passed a connection to a transaction-only helper. The no-op revision
regression now reads its before/after values directly from the connection, retaining its assertions.
Commit `9f4b0e7` carries that correction; its hosted results remain in progress.

The schema-42 follow-up preserves the existing inventory and live-gap repair contracts. Their
explicit supported-version ranges previously ended at 41, so current-schema recovery skipped
those repairs before validating retained terminal state. Schema 42 changes neither contract's DDL.
It now receives the same structure/marker checks, including damaged-trigger and incomplete-marker
rejection; version 43 remains rejected. Historical downgrade fixtures and two current-version
expectations also include schema 42. Existing current-schema repair tests exercise this admission
through catalog reopening. These checks run in Actions without another local library or CI run.
The repair-shape owner has 108 lines with no inline test cases and 55 dedicated test lines.
The scan and incremental-change owners remain 749 and 1,504 lines; only schema expectations changed
in their 5,516-line and 3,802-line dedicated test files, with no new application behavior.

## Duplicate-location dialog

The retained client exposed clipped paths and a close control outside the anchored menu surface.
The badge now opens the SDK's centered `Dialog` through `showDialog`; ADR 0029 records the component
choice and its 560-pixel maximum width. Filename and folder text wrap fully in this dialog, while
review cards keep their compact presentation. The location query, availability rules, exact-file
reveal action and bounded paging remain unchanged.

The 10-case focused duplicate suite passes with dialog containment and long-path wrapping checked
at 1000x760 and 400x600, including double text scaling in the narrow case. The assertions cover the
rendered text and close-button bounds, scrolling to the row's final status, focus return, Escape,
outside-click consumption, reveal arguments and retained accessibility updates.

The shared Windows tooltip also no longer treats button focus as a visual trigger or a reason to
keep the overlay visible. Two tooltip lifecycle cases pass, including autofocus without a popup,
hover display, exit while the button remains focused, and the retained semantic tooltip. The
duplicate-dialog cases assert no automatic close tooltip and no lingering badge tooltip on return.
This owner now has 386 production lines, no inline tests and 140 dedicated lifecycle-test lines.

The rebuilt Debug client reused `build/test` and the existing catalog. Its 150-location group was
checked for complete path wrapping, fixed header/footer, next-page loading, list scrolling and
Escape dismissal without moving the gallery. The updated test window remains available for visual
review, so its active profile-restoration backup is retained until the session is closed. The
change introduces no catalog migration, source read workflow or source mutation.

## Sidebar and gallery presentation

The Library sidebar toggle now sits above Settings in the function rail. Its 180-millisecond
transition moves the sidebar boundary and the gallery's left clip while keeping the right gallery
edge stationary. The gallery receives the destination width once per toggle. The original left
alignment translated right-side controls by the sidebar width during the transition; narrow headers
also placed their wrapped toolbar on the left. Right alignment corrects both boundaries.
Nine focused cases pass: six actual header/time-rail fixtures at 800/1000/1280 pixels and normal/double
text scaling check their right edges before, throughout and after both transitions; three transition
cases retain final-width layout counts, rapid reversal and immediate reduced-motion placement.
The six coordinate cases failed before the correction. The reduced-motion fixture flushes the SDK's
focus-change layout callback without advancing time before checking that no ticker remains.
The changed header/transition owners have 509/148 production lines and no inline tests; the two
dedicated transition/coordinate test files have 116/110 lines.
The rebuilt Debug was checked with the retained test catalog: collapse and expansion keep the
toolbar and time rail in the same right-side positions, with the original expanded state restored.

All-images mode omits duplicate badges and annotation requests. Merged mode retains an icon-only
location action; normal local availability adds no status line. Four card-boundary cases pass at
normal and double text scaling, including delayed annotation arrival and selection bounds. The
128-pixel minimum card width remains. Updated shell, resize and duplicate-dialog regressions run
in Actions. The earlier native inspection stopped at physical Escape. The subsequent authorized
2026-10-17 Debug session verifies the restored rounded gallery boundary and circular card controls;
the active isolated client and its restoration backup remain intact.

## Duplicate workspace layout

The workspace retains responsive group cards and single-column member rows. Header actions align
right, category/count geometry stays stable, and loading appears at the toolbar/results boundary
using the gallery's two-pixel overlay. Shared empty/error content replaces the plain centered text.
The group-list header places its heading/count block on the left, category switch at the content
center, and analysis actions on the right in one row. Equal flexible regions keep the switch
centered despite different title and action widths. Compact widths and enlarged text stack the
controls below the heading while retaining the switch's centered alignment.
The existing Material SegmentedButton remains unchanged; the official
[segmented-button page](https://m3.material.io/components/segmented-buttons/overview) requires
JavaScript, and the pinned SDK confirms its shrink-wrapped sizing when expandedInsets is absent.
LayoutBuilder, Row and Column provide the requested composition without a custom control.
Seven layout cases pass across 400/1200/1280-pixel surfaces and normal/double text scaling,
including independent horizontal centering, action alignment, stable category geometry and unchanged
member-row behavior. The initial narrow fixture used a multi-card finder for a single coordinate;
it now selects the first card while retaining the alignment assertion. The updated Debug verifies
the title/count, centered switch and right-side actions in one row with the retained 72-group
catalog. The header has 144 production lines, no inline tests and 308 dedicated layout-test lines.
Members use vertically centered star and folder IconButtons on the right, without a footer; normal
row height is 112 logical pixels in the retained Windows client. Icon semantic labels and hover
tooltips name preference/reveal actions. Group cards retain their lower-right `查看重复图片` action.
Material `Card`/`InkWell`, `IconButton` and `GridView.builder` supply their interaction primitives.
The pinned SDK supports selected icons; ADR 0029 records the composition and alternatives.

Continuous virtual scrolling replaces review Previous/Next controls. Each window retains at most
six 64-record payload batches and 64 cursor checkpoints. A background replacement has the same
independent bounds and publishes its visible range atomically. Viewer navigation reads adjacent
members across batch boundaries. Visible preview demand remains bounded and retires when hidden.

Review scrolling uses the shared AmeScrollController for both the wheel wrapper and its GridView.
The prior ordinary explicit controller bypassed the wrapper's inherited controller, so both group
and member views moved the complete wheel delta immediately. Four focused widget cases cover both
layouts with normal and reduced motion: normal input moves immediately and finishes through the
shared deceleration tail, while reduced motion stays immediate; both settle at the original distance.
The two normal-motion cases reproduced the missing tail before the wiring correction, and all four
pass after it. A subsequent dialog inspection found its explicit `primary: false` also bypassed the
inherited controller. Removing that opt-out connects the location list to the same motion owner;
two dedicated dialog cases pass for normal/reduced motion and preserve the total wheel distance.
The updated Debug reused the retained test catalog; native wheel checks covered downward and
reverse scrolling in both review layouts. The client remains open for visual inspection.

Duplicate-dialog dismissal restores badge focus only when the badge held focus before opening.
The real photo-tile regression reproduced an unwanted focus and persistent selection control after
mouse dismissal. Both mouse and keyboard cases now pass, alongside ten existing dialog/workspace
checks. Keyboard path-layout checks now open with Tab/Enter and retain Escape focus restoration.
The updated Debug verifies mouse dismissal with the pointer outside the card: the unselected
selection circle disappears. Keyboard restoration is covered by the focused widget cases.

Location rows now use one ListTile action instead of repeating reveal in a trailing button.
`当前图片` occupies the current row's trailing slot; paths and exceptional availability remain in
the subtitle. Twelve focused dialog/workspace/scroll cases pass, covering single-action geometry,
unmarked other rows, long-path wrapping at normal/double text size, actual reveal dispatch,
keyboard focus return and shared wheel behavior. The row has 68 production lines, no inline tests;
the existing dialog/workspace test file has 562 lines. The updated Debug displays the right-side
current marker without duplicate buttons. Clicking the first row opens a new Explorer window,
selects `06705-3840x2160-0741.jpg` and reports one selected item; that owned window is then closed.

The first Explorer reveal was reproduced with a newly opened test-corpus folder: the folder was
correct, but the target file was not selected. The Windows adapter now resolves a Unicode shell
item and calls SHOpenFolderAndSelectItems on a worker isolate. Thirteen focused adapter, native
failure and dialog/workspace cases pass; invalid inputs fail before entering the Shell, and three
successive missing-item calls return the native HRESULT through the reveal error boundary.
Scoped Dart analysis, formatting and the Debug build pass. Native inspection of the rebuilt client
confirms the first reveal opens a new folder window, scrolls to `03325-6000x4000-0313.jpg`, highlights
that exact file and reports one selected item. The owned Explorer window was closed afterward;
the existing test catalog, client profile and source files remain in place. The adapter and Shell
owner have 63/101 production lines, with 29/47 dedicated test lines and no inline tests.

Review preview previously used the default Material page animation. Its route now shares the
gallery's fade component, 180-ms entry, 100-ms exit and immediate reduced-motion behavior. Pointer
input finishes entry immediately. Sixteen focused route, gallery-transition, review and viewer
checks pass, covering stationary content bounds during the fade, retained review choices, adjacent
catalog batches, source-read retirement and late completions. The shared motion owner has 55
production lines and 72 dedicated route-test lines, with no inline tests.
The updated Debug also opens a member image in the viewer and returns to the same member-list
position on Escape. Exact interpolation timing remains covered by the focused route checks.

Timeline decorations retain their keyed elements across layout changes and use Flutter's
AnimatedPositioned for 180-ms geometry transitions. The installed SDK's implicit-animation owner
retargets from its displayed position; its Material Slider remains the input primitive. The
[Material slider reference](https://m3.material.io/components/sliders/overview) requires JavaScript;
the installed Slider implementation and [AnimatedPositioned API](https://api.flutter.dev/flutter/widgets/AnimatedPositioned-class.html)
were inspected directly. The product-specific gap is motion of the surrounding date annotations,
not slider input. Geometry identity separates layout movement from immediate scroll-value updates.
Only slider identity resets on layout/revision changes, retiring old pointers while retaining paint.
Eight motion cases, nine existing rail cases and six publication cases pass, including reduced
motion, rapid reversal, scroll interruption, linear final placement and rejection of stale input.
The rail has 451 production lines, presentation 124 and motion owner 79; the dedicated motion file
has 138 test lines. None of these production owners has inline tests.
The updated Debug reused the retained 72-group catalog. Native collapse/expand checks preserved the
right rail and toolbar edge while the date distribution changed; per-frame interpolation and input
timing are established by the focused widget cases above.

The reported preference-switch flash was traced to the preference transaction advancing the catalog
revision, followed by the gallery refresh notifying a review listener that cleared member payloads.
Catalog and analysis publications now revalidate without removing the rendered range. Explicit
refresh, initial loading and continuation retain loading feedback; errors remain retryable.

Twenty-two focused checks pass: seven layouts at 400/1200/1280 pixels and normal/double text scaling,
seven controller/widget cases, seven bounded-window/lifecycle cases and the preference publication
case. That last case switches A to B to A, holds each follow-up catalog read pending, checks no
loading bar, preserves card elements and scroll offsets, and verifies icon semantic names.
The two viewer cases previously passed for navigation across member 63/64 and late completion after
disposal; their current-head execution belongs to Actions. The initial semantic-label check exposed
a test handle released after Flutter's end-of-test verification; a finally block now releases it
before the test returns, and the check passes with its assertions retained.

The 2026-10-17 Debug build reused the original 10,000-file catalog and displayed 72 groups. Native
inspection confirms three-column group cards, compact full-width member rows, centered action icons,
whole-row hover, stable heading/scroll placement while switching two preferred copies, and direct
card activation of the viewer. The existing preferred copy was preserved; a separate group supplied
the temporary preference-switch check. The active test client and profile-restoration backup remain
available for user inspection. No migration, source mutation or new source-read workflow is added.

Group totals and keyset results are read in one SQLite snapshot, including empty later pages.
Regressions cover 70 groups across the 64-group page boundary, ignored categories and stale evidence.
This adds a scalar to the generated bridge contract without a database migration, source enumeration
or media read. The query has a 121-line owner; the review persistence facade has 191 lines, with 366
review-test and 251 analysis-test lines. The page has 354 production lines, the grid 127, cards 153
and analysis actions 91 at that earlier count revision. The current review page has 273 production
lines, controller 163, bounded window 197, cards 213, grid 136, header 105, metrics 79, viewer 135,
reveal owner 56 and shared empty message 46. Dedicated review/layout/window/viewer/preference tests
have 268/288/192/113/90 lines respectively. These owners have no inline tests.

The earlier `3325f54` Actions run exposed one old menu test expecting focus to open a tooltip. Commit
`e7d2bef` updates that expectation to the new hover-only contract while retaining focus assertions;
the dedicated semantic/hover lifecycle checks remain. Its successor Actions run owns verification.
That run also identified one null-aware collection lint and an import-browsing fixture that clicked
the hidden sidebar before the new opening transition advanced. The collection uses the required
null-aware element; the fixture advances the transition before selecting the existing root, retaining
its scan-continuity assertions. The affected sidebar resize cases passed in that hosted run.

The `0ea6d9f` hosted Flutter job passed all 14 photo-tile cases and then exposed two preview-feedback
tests still expecting focus-triggered tooltip text. They now assert accessible reason text with
keyboard focus, hover-only visual feedback, removal on pointer exit, and the existing keyboard Retry
action. All five focused feedback cases pass locally. This changes test expectations to the accepted
tooltip behavior without reducing the interaction or accessibility checks.

The same head's synthetic 10,000-file case reported a 67,174,400-byte catalog, exceeding its existing
64-MiB ceiling by 65,536 bytes; the resumed catalog was 37,826,560 bytes. Its active/all-location,
asset and cancelled-location counts passed before the size assertion. The failure remains open;
the size ceiling is unchanged. Native scan, accessibility, unsigned Release and the other four
synthetic workloads passed on that head. `213f10c` carries the review UI/continuity changes and has
its own queued Actions run; those older results do not validate its changed inputs.

## Duplicate viewer route accessibility lifetime

The 2026-10-17 crash investigation reproduced an invalid native accessibility update on the first
frame of opening a duplicate member in the viewer. Later maximize/restore and sidebar changes could
terminate the process. The retained Windows dump resolves the access violation to
`flutter::FlutterPlatformNodeDelegate::ChildAtIndex`; later failures include the bridge's reparent
assertion. Bounded recording of the actual Flutter semantics updates identifies the first orphan
before those later actions: the parent lists it for hit testing but omits it from traversal.

The installed SDK's HeroController makes the incoming page offstage for its initial measurement,
while Slider creates its value-indicator OverlayPortal immediately. A traversal container around
the slider does not fix this lifecycle mismatch. The viewer route now owns `Overlay.wrap` so the
portal and its traversal parent become visible and retire together. No SDK modification, new
dependency, accessibility suppression, source-media operation or persistence change is involved.

The new retained-update regression fails with the same orphan shape before the repair and passes
after it. Normal and reduced-motion cases cover three viewer open/return cycles, reachable zoom
values, viewer information sheets, window-size changes, gallery location dialogs, context-menu
information sheets and 36 rapid sidebar reversals. Thirteen focused cases pass across this boundary,
the existing route fade timings, retained workspace semantics and duplicate review interactions.

The rebuilt normal Debug entrypoint reused the retained 10,000-file / 72-group test catalog.
Native interaction opened a member preview and its information sheet, returned to the gallery,
maximized/restored the window, rapidly clicked the sidebar toggle and verified both settled panel
states, then opened and dismissed gallery information through its context menu. The process remained
responsive with zero AXTree/native error lines and zero Dart exceptions. Windows accessibility still
exposes the zoom slider and its 100% value. The active client and profile-restoration backup remain
for inspection. The route/fade owner has 58 production lines and no inline tests; the new dedicated
regression has 213 lines. Applicable hosted checks belong to the submitted commit.

## Photo context-menu focus restoration

Mouse secondary activation no longer requests keyboard focus on the photo card. The menu therefore
restores only an existing focus target; dismissing it or its information sheet with the pointer
outside the card hides the selection control. Keyboard activation still retains card focus and its
visible selection control. This changes input focus ownership without changing selection state.

Twenty focused cases pass across photo-tile interactions and overlay focus restoration, including
mouse/keyboard dismissal of both the menu and information sheet. The rebuilt Debug client reused
the retained catalog; both mouse flows were verified with the pointer outside the card and neither
left a selection circle visible. No source or persistence operation is added.

## Duplicate location bottom sheet

The gallery badge now opens the standard modal bottom sheet with its drag handle. The list has
no close or pagination buttons; Escape and the dismissal barrier retain the existing focus and
gallery-position behavior. Filename/folder wrapping, the right-hand current-image marker and
whole-row Explorer action are preserved. The obsolete group/location pagination controller is
removed. The sheet binds the existing typed analysis service to LibraryDuplicateReviewWindow;
framework-built rows report demand without estimating variable path heights from scroll offsets.
Reads remain 64 locations per batch, with six cached payload pages and 64 cursor checkpoints.

Twenty-six focused cases pass: sheet continuation/retry and stale/disposed reads (2), bounded
window admission and caching (7), wheel motion with/without reduced motion (2), existing exact
duplicate interactions and long-path layout (7), mouse/keyboard focus restoration (6), and the
member-preview retained-semantics regression (2). The 400-pixel layout uses doubled text size;
filenames and paths remain complete and the current marker remains reachable. Obsolete paging
controller tests are replaced by the window lifecycle and sheet integration coverage.

The normal Debug build reuses the retained 10,000-file catalog. Native inspection opens the
600-location sheet, scrolls into subsequent batches, then dismisses it outside the list. The title,
paths and current marker render correctly, no close/pagination controls remain, and the gallery
retains its position without a selection-circle residue. No AXTree/native errors or Dart exceptions
were emitted during this check. The active client and restoration backup remain for inspection;
source media and the original catalog are unchanged. The sheet has 168 production lines and no
inline tests; new sheet, motion, exact-interaction and focus test owners have 143/84/489/177 lines.
Applicable hosted verification belongs to the submitted head.

## Photo-wall geometry motion

Sidebar movement now carries the photo content's left edge while the toolbar and timeline retain
their right-edge positions. A separate mounted-item motion owner interpolates cell position and
size across lazy rows for sidebar and window dimension changes. It consumes the geometry owner's
pending scroll correction before the sliver applies it, keeping the first painted frame aligned.
Query-wide geometry still rebuilds only for destination widths; motion does not enumerate the catalog.

Thirty-nine focused cases pass: layout motion and rapid retargeting (3), existing geometry/scroll
ownership (25), sidebar boundaries and reduced motion (3), right-edge/content-edge geometry at
three widths and two text scales (6), and retained viewer semantics (2). Final-size assertions for
height changes now wait for the 180-millisecond transition; their original anchor bounds remain.
The existing continuous sidebar-resize anchor check passes without relaxing its two-pixel bound.
The local analyzer and format gate pass. Native Debug inspection reused the retained 10,000-file
catalog and covered sidebar collapse/expand and maximize/restore. No Dart, AXTree or native errors
were emitted. The current client remains open for inspection; source media and catalog data are
unchanged. Production owners have 1,971 wall, 226 motion, 94 entry and 66 sidebar-content lines,
with no inline tests; the new motion regression has 132 lines. Hosted checks belong to the pushed head.

The title and count also consume the sidebar's leading offset. The six width/text-scale cases now
check their positions on every animation frame alongside the photo edge, toolbar and time rail.
The updated Debug client was checked with the retained gallery; its heading/count and photo wall
follow the same collapsing boundary. The toolbar remains fixed. No data operation is introduced.

Hosted verification exposed a layout-time interruption: recovered preview dimensions terminate
wheel activity while the viewport lays out, sending an idle user-scroll notification. The motion
owner now interrupts reflow only for an active user direction on its own scrollable. Idle activity
does not change animation values or schedule a build during layout. Four motion cases and all 43
screen cases pass, including both failing recovered-dimension paths and immediate interruption by
new user scrolling. No anchor assertions, time limits or scrolling behavior were weakened.

## Source and merged image counts

GalleryTimeline now returns the source count and displayed count from the same SQLite read
transaction. Root, folder, descendants and search constraints are shared; All images reuses its
existing total. The timeline query/aggregation responsibility moves into a 128-line owner instead
of adding counting policy to the snapshot coordinator. The bridge is regenerated without a schema
change. The header shows `10000 张图片 · 去重后 72 张` in the retained merged gallery and
`10000 张图片` in All images; both were observed in the rebuilt Debug client.

Eight focused Rust cases pass, including scoped counts and a concurrent catalog publication between
page and timeline reads. The existing changed-source fixture now uses a valid changed revision
token, retaining its original folding assertions. Four count-display cases and six sidebar-frame
cases pass across 800–1280-pixel widths and 1x/2x text. Scoped analysis, formatting and all 16 generated
asynchronous bridge contracts pass. Native filter switching also exposed an AXTree pending-node
error, retained as a separate semantics-lifetime investigation; the observed count result does not
close that client check. Original media and retained catalog are preserved.

Production owners have 498 gallery-query, 128 timeline, 274 snapshot plus 29 test-hook, 784 domain
plus six test-export, 556 catalog-mapping, 516 presentation-model, 2,017 screen and 521 header lines.
There are no inline test cases. Dedicated query, count-display and sidebar-frame tests have
366, 57 and 152 lines respectively. Applicable hosted verification belongs to the submitted head.

## Filter-switch investigation and hosted failures

The retained-semantics validator now models the pinned Windows bridge's removal of a moved node's
entire old subtree. It requires surviving descendants to be serialized again; paired fixtures
reject a missing descendant and accept the complete update. An application-level filter regression
covers delayed query publication followed by asynchronous layout-manifest takeover. Thirty-four
focused cases pass across this boundary, gallery/window transitions and workspace navigation.
These checks strengthen the test model; they do not establish a repair for the observed native
pending-node error. A temporary bounded native update recorder did not reproduce that error and
was removed from the delivered Debug entrypoint.
The test-only validator has 470 lines; its unit cases and the filter fixture have 189 and 179 lines.
No production semantics implementation or inline tests were added.

Actions run `38964168799` at `e2e2345` passed native accessibility, native scan, unsigned packaging
and four synthetic workloads. Static analysis reported three multiline control-flow bodies without
braces; those are corrected without changing behavior. Two recovered-dimension widget cases exposed
the layout-time animation interruption corrected above. The ten-thousand-image workload completed
its scan sequence but measured a 67,203,072-byte catalog, exceeding the unchanged 64 MiB ceiling by
94,208 bytes. This catalog-size failure remains open; no workload, limit or assertion was relaxed.

## Catalog storage bound

The unchanged scan workload reproduced locally at 67,145,728 bytes. Its exact-candidate cursor
index occupied 3,444,736 bytes, while the production candidate query already used
`asset_locations_location_id`. A read-only in-memory catalog comparison retained that cursor seek,
the covering same-size peer lookup and candidate results after removing the redundant index.
Schema 43 retires only that index, with the preservation contract in ADR 0028.

Fourteen focused query, migration and compatibility cases pass. They cover keyset ordering,
preferred/ignored decisions, fingerprints, unchanged catalog revision, malformed legacy rejection,
historical versions 40–42 and unknown-version rejection. The original scan workload then passes:
10,000 files, cold 30,157 ms, warm 19,249 ms, pause 12 ms, resume 26,831 ms, cancel 86 ms,
catalog 63,455,232 bytes and resumed catalog 35,975,168 bytes. All original source-preservation and
resource assertions remain enabled; the catalog ceiling remains 64 MiB.

The migration owner has 47 production lines and 17 test-fixture lines, with 112 dedicated test
lines; the production query owner has 147 lines and its index regression 50 lines. The existing
3,622-line catalog facade changes only its schema constant. The existing migration facade has
9,851 production/fixture lines and 5,267 inline-test lines; its changes only register the new owner
and route version 42 to it. Its retained decomposition obligation is unchanged.

The broader focused selection also reproduced two incremental-fingerprint failures and one module
declaration rejection in the pre-migration schema 42 test binary, whose SHA-256 matches the earlier
performance receipt. These are separate PR repair work, not a storage-bound pass. Actions run
`38967752272` on `c8ba056` subsequently reported sixteen Rust failures; final CI and native
filter-switch acceptance remain open.
