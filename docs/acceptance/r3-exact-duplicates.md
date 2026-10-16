# R3 exact-content verification

Status: R3-01 client verification recorded; automated checks pending.

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
resizing was not established by this check. Automatic analysis, gallery folding/badges and durable
preferred-copy/ignore decisions remain the next scoped deliveries in the execution plan.

Large existing owners were extended only at registration/dispatch boundaries: `sqlite_catalog.rs`
has 3,621 production lines, no inline tests and 8,009 lines in its existing dedicated test file;
`migrations.rs` has 9,842 production lines and 5,267 inline-test lines. New analysis, query,
publication and migration behavior has dedicated owners. The existing gallery shell remains the viewer/query owner; navigation and sidebar preference
ownership moved into `LibraryWorkbench`, and hidden-gallery preview retirement has its own widget.
The changed shell has 2,005 production lines and 4,122 dedicated-test lines; the gallery wall has
2,002 production lines and 1,738 lines in the affected resize/visibility test file. Neither owner
has inline tests. The workbench, analysis page and activity widget own their new responsibilities.
