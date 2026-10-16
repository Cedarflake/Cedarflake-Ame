# R3 exact-content verification

Status: R3-01/02 client checks and R3-03 gallery-mode checks recorded; affected Actions verification remains open.

Date: 2026-10-16.

The [execution plan](../plans/r3-exact-duplicates.md) owns the live checkpoint. This record covers
the implementation on `codex/r3-exact-duplicates`, based on `aeaec3d`. It does not close R3.

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

## Gallery ordinal recovery

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
has 3,621 production lines, no inline tests and 8,009 lines in its existing dedicated test file;
`migrations.rs` has 9,845 production lines and 5,267 inline-test lines. New analysis, query,
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
