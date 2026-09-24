# Browsing admission during import and publication

Status: root-browsing correction passes focused and complete Daily checks; Release/native proof remains open

The [Release continuation](r2c-release-native.md#release-live-continuation-result) observes two root
clicks ignored during background import and an unsuccessful historical jump during deletion.
These are independent boundaries in UX-03A and UX-08A, under the
[current execution method](../plans/r2c-closeout.md#browsing-admission-during-import-and-publication).

## Root selection and catalog reads

At source `ae3d642`, the sidebar passes the general task busy state to browse actions. The viewport
also rejects query changes, paging and time reads during a primary scan. An existing published
root therefore becomes unavailable until an unrelated import finishes, despite the task snapshot
and catalog projection already having separate owners.

Four new application cases fail on the unchanged product: root query admission, passive refresh,
query-failure reporting and import completion following a pending user query. An actual wide-screen
sidebar tap also leaves the root query unchanged. The first compact-screen test incorrectly looks
for an expanded title; its corrected finder targets the existing compact IconButton. This fixture
error is separate from the product failure.

`LibraryBrowseAdmission` now owns read admission. A scan, pause or cancellation can coexist with
reads when a root has a published scan identity. A completely unpublished import retains its
previous restriction. Picker, query-loading, page-loading, explicit time-loading and removal
boundaries retain their respective exclusions. Query supersession remains distinct from passive
refresh and page admission. Publication reservations, root membership, revisions, generation
checks and actual result publication remain with their existing owners.

The screen and navigation compose that policy; scan and root-removal commands continue to use
their unchanged task/lease admission. Successful, failed and late query reads retain the current
primary snapshot. Independent review identifies a connected omission: query-failure Retry still
uses the general task busy flag. The added real-button test fails with that flag, then passes with
the shared browse policy. It requires the selected root, idle query state, exactly one additional
catalog read, the same primary snapshot and exactly one scan start. The incremental static recheck
closes that finding without claiming another test or native run.

The final focused checkpoint passes 92 cases: six admission cases, seven new application cases,
three connected sidebar/Retry cases, 56 existing controller cases, three committed-primary-refresh cases,
four folder-position cases and 13 navigation/semantics cases. One original lost-terminal case
expected no read while scanning; its intentional policy update now requires exactly one latest
time request with the original revision/month and offset 19. All 20 obsolete jumps still fail,
and its complete catalog, task identity, counts and no-error assertions remain. No result guard
or test deadline is removed.

The new admission owner has 50 production lines and 80 dedicated test lines. The affected viewport,
screen and navigation owners have 1342, 2189 and 456 production lines respectively, with no inline
tests. The new connected application and presentation tests have 154 and 110 lines. The existing
large facades retain their broader decomposition debt; the new behavior belongs to the extracted
read policy rather than another scan-state flag.

No real library, original-media write, cloud hydration, schema, bridge or dependency change is
involved. Focused tests use controlled catalog/scanner fixtures and the actual Flutter composition.
They do not establish optimized-client or full R2c acceptance.

### Complete local quality checkpoint

The complete canonical `quality_verify_daily.ps1` exits zero on the frozen ten source/test files,
from 01:57:16 to 02:30:26 UTC on 2026-09-24. It includes warning-free lint, the complete Rust suite
with its 19 existing explicit performance/external/helper exclusions, all 91 Flutter test files,
controlled Windows scanning, all ten native accessibility phases, the 15 asynchronous bridge
contracts and tracked whitespace validation. Native accessibility reports both tests passed,
normal primary exit and Job closure, with no run or cleanup failure. The controlled scan likewise
reports exit zero and no cleanup failure. The original mixed P0/P1/P2 assertion passes with all
25 foreground samples, 2048 P1 candidates and 10000 P2 entries; visible P95 is 86 ms and maximum
100 ms. Its connection-lifetime control is separate evidence, not a substitute workload.

The ignored `.build/r2c-publication-browsing/daily-summary.json` retains the original base commit,
start/end times and all ten unchanged hashes; SHA-256 is
`F0D8CB2CEC81B2A4E2652F2CBDCD75D309F71B39576442B945D325CC88E4B97B`.
The standalone lint checkpoint precedes the final Retry line; complete Daily reruns lint on that
final source. The fresh optimized package and same-frame import-time root observation remain
separate pending requirements. No previous native result is relabeled as evidence for this change.

## Separate date-navigation diagnosis

The controlled `rail_revision_probe_test.dart` under ignored `.build/r2c-publication-browsing`
reproduces a different failure: a requested `2012-03` offset 10 belongs to revision 1; a deletion
publishes revision 2 before its read. The old anchor is correctly rejected, but the viewport then
loads the first page, publishes offset zero, clears the time anchor and returns false without an
error. The diagnostic records `readRevisions=[1]`, `publishedRevision=2`, `accepted=false` and
`publishedMonth=null`. Its passing assertions describe the unresolved defect, not successful
navigation. This causal boundary is consistent with, but does not prove the internal path of,
the preceding Release point observation.

A correction must keep the explicit date intent independent from obsolete catalog/layout
authority, obtain coherent current timeline/page evidence, and align the current result while
rejecting replaced-query, removed-root and later-user-intent results. Merely permitting stale
cursors, replaying clicks, adding a notification or loading the first page does not meet that exit.
No date-navigation product change is included in the root-browsing correction.
