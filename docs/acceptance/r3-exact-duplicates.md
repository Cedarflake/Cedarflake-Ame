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
transition moves the sidebar boundary and photo-wall origin together; the gallery receives the
destination width once per toggle. Three focused transition cases pass: final-width layout counts,
rapid reversal and immediate reduced-motion placement. The reduced-motion fixture flushes the
SDK's focus-change layout callback without advancing time before checking that no ticker remains.

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
pass after it. Other existing application scroll views retain their shared motion wiring.
The updated Debug reused the retained test catalog; native wheel checks covered downward and
reverse scrolling in both review layouts. The client remains open for visual inspection.

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
