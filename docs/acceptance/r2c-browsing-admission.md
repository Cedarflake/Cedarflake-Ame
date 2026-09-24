# Browsing admission during import and publication

Status: current Daily and unsigned Release pass; import-time root selection observed, deletion/date native proof remains open after source-cost and input-admission failures

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
final source. The fresh optimized package and same-frame import-time root observation are separate
requirements. No previous native result is relabeled as evidence for this change.

### Optimized build and uncompleted native admission

The canonical unsigned Windows gate passes on clean source `058d38b9651e069158bfb826be517e57b6e3d817`,
from 02:33:44 to 02:35:52 UTC on 2026-09-24. It includes the fresh optimized application and broker,
three engine-free runner cases, two actual Debug-engine retirements and catalog-free Release
bridge smoke. Its retained evidence SHA-256 is
`1A7BDFE3DDEF3E055D2721DEF3F0EA3B42B34B8B0C5A1F8C2FEED61492079354`.

Native run `5fb59ce2fb6246e9a9b1cc3f2374eeb0` fails before application launch. The guest prepares
the 12-file baseline and starts background source preparation, but the 120-second native-binding
admission expires during the control-session interruption. No `start-media.json`, import
confirmation, batch request or application-exit receipt exists. Consequently root switching,
automatic convergence, closed membership and application-close timing are all unreached.
This is a failed verification lifetime, not a product failure or a passing native result.

The observed normal Sandbox close is followed by its confirmation. That input returns before
capture reports no target for the same bound Sandbox process. The parent exits with failure after
284367 ms, with no remaining Sandbox processes, no guest cleanup failure and minimum host available
memory of 5113954304 bytes. Successful resource retirement does not change the failed parent result.
All 33 reviewed helper hashes match. Startup and post-exit checks each verify all 10516 host
generated sources; the final check completes in 56.133 seconds. No real source is involved.
The independent bounded result review confirms these structured limits and does not replace
the missing client observations. This single-use lifetime is consumed without an unchanged replay.

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

### Coherent date-intent correction

The subsequent correction preserves strict existing anchors. Only a still-current explicit date
request that receives `catalog_cursor_stale` may obtain one semantic time snapshot. Rust resolves
the month and bounded within-month offset against the current timeline, then reads its page in the
same SQLite transaction. Missing months choose the next surviving month in display order, or the
last surviving endpoint; empty and unknown-date results remain explicit. A concurrent root deletion
after resolution cannot mix a new page with an old timeline. No retry loop or source read is added.

The application result owner checks root membership, revision, query, cursors, timeline totals and
target containment before publishing. Cancellation, disposal and supersession reject late results
without clearing a newer request's loading state. A passive read retains its prior behavior; an
explicit request may promote the same pending read. Presentation retains semantic intent only
through its admitted publication and aligns against current geometry within the existing eight
frame bound. Query/layout/controller changes and later user input still cancel that authority.

Implementation review identifies a multi-column counterexample: an old row start becomes the middle
of a new row after deletion, while a page beginning exactly at the target omits that row's prefix.
The added Rust oracle first fails with window start 22 instead of zero. The corrected read keeps
the target anchor but begins at most half a page earlier, resolved through the same timeline.
The Dart request retains the target ordinal independently from this window start. Static incremental
review finds the identified boundary addressed; it does not establish native acceptance.

Focused verification passes all five new Rust tests, plus the existing clock case matched by the
filter. It includes exact full-order page comparison in both directions, missing/unknown dates,
endpoint clamping, concurrent deletion and unchanged strict/sort/month rejections. The final changed
Flutter files pass 24 cases: 13 application date cases, six presentation alignment cases and five
existing committed-update cases. The four-column case changes date groups from `[70, 10, 20]` to
`[20, 8, 22]`: target 80 becomes 30 in the row starting at 28 and aligns without a wheel event,
including geometry delayed three frames. These use real navigation/viewport owners with controlled
catalog and layout fixtures, not a native full-wall or media-decoding claim. Earlier controller and
navigation tests remain recorded; complete Daily must verify the final accumulated source.

The first lint invocation fails because one static update fixture lacks the new typed catalog
method; that fixture now rejects unexpected semantic reads explicitly, and its five cases pass.
Two canonical bridge-generation invocations generate output but fail at immediate Rust formatting
with Windows error 1224 on the mapped generated file. Separate formatting and Release-library
compilation then pass after generator retirement. The failed generator invocations remain failures;
generated output is not hand edited. Final-source gates retain hash and wire-mode verification.

Physical owner sizes at the focused checkpoint include blanks and declarations; inline tests are
zero for these files. Generated bridge output is excluded from handwritten maintenance findings.

| Responsibility | Production lines | Dedicated test lines |
| --- | ---: | ---: |
| Rust semantic date policy | 126 | 150 |
| SQLite coherent date read | 63 | 180 |
| Rust application composition | 20 | Covered through adapter and connected caller boundaries |
| Dart typed result / read and projection | 40 / 134 | 434 |
| Dart time-request lifecycle / viewport facade | 312 / 1327 | Existing controller/request suites plus the above cases |
| Presentation semantic alignment / navigation | 52 / 760 | 278 new and existing navigation tests |

No schema, dependency, real-library, cloud, installed-service or source-media mutation is involved.
The complete Daily, fresh unsigned package and new native lifetime remain required for this source.

### First complete-gate failure and exact topology correction

The first date-candidate Daily runs from 03:35:12 to 03:55:21 UTC on 2026-09-24, with all 35 frozen
source/test hashes unchanged. Lint passes; the Rust library finishes with 1537 passing, two failing
and 19 previously explicit ignored cases. The original mixed P0/P1/P2 gate passes, including full
convergence, with visible P95 85 ms and maximum 88 ms. Flutter and subsequent native/bridge phases
are not reached because the Rust phase fails.

Both failures report the exact cause: the domain topology roster rejects the newly introduced
`gallery_time_snapshot` module as unexpected. The correction adds only that existing public,
attribute-free, out-of-line declaration to the typed roster. No root availability implementation,
enumeration restriction or checker algorithm changes. Positive topology checks now include it;
the same ten mutation cases reject missing/duplicate declarations, changed visibility/attributes,
alternate paths, inline/generated loading and an added unknown module. The original failing run
remains in `.build/r2c-date-publication`; the one corrected full invocation gets separate evidence.
Its `daily-result.json` SHA-256 is
`84F0ABF7970A4D7E7F23A92E9EA971825A4BE4A3384D5BCDDFC543B1888E3AB4`.
All eight focused availability cases then pass, including exact topology, macro/module-redirection
and enumeration rejection. A one-minute incremental review confirms the declaration remains exact;
it introduces no runtime or AST-policy relaxation. The touched local-files owner retains 3991
production and 3432 inline-test lines; its topology test module has 253 dedicated lines. The change
only updates existing typed test data and its independent positive/negative verification.

The following standalone lint attempt stops in the unchanged UIA guardrail's sequential progress
round-trip: `File.Replace` reports that it cannot remove the destination. No native client starts,
and the guardrail's finally block retires its scratch. One permitted isolated invocation of that
same compiler-free guardrail then exits zero, including blocked-child cleanup. The original I/O
failure has no proven causal attribution and remains recorded; neither product code nor evidence
publication policy is changed to conceal it. The corrected full Daily must execute this guardrail
again as part of its complete lint phase, alongside the causal topology correction.

### Corrected complete local quality checkpoint

The corrected canonical Daily exits zero from 04:02:27 to 04:36:58 UTC on 2026-09-24. All 37 frozen
source/test/tool hashes remain unchanged. Warning-free lint, the complete Rust library (1539 passing
and the same 19 explicit exclusions), three broker integration cases and all 93 Flutter test files
pass. The original mixed workload retains all 25 foreground samples, 2048 P1 candidates and 10000
P2 entries with complete convergence; visible P95 is 79 ms and maximum 90 ms.

Controlled Windows scanning exits zero without run or cleanup failure. Both whole-window
accessibility cases and all ten phases pass, with normal primary-process exit, owned Job closure
and no cleanup failure. All 16 asynchronous bridge contracts and matching content hashes pass.
The previously failing unchanged UIA guardrail also passes within this complete lint invocation.
Neither earlier failed invocation is replaced or reclassified.

The immutable `.build/r2c-date-corrected/daily-result.json` retains times, base commit and exact
candidate hashes; SHA-256 is
`CE3596CE1B1E8E9A7B0AA7A706E8044123EB0B7E93C9EC55EF7E8DFC5B67C7C5`.
Fresh optimized packaging and the single admitted native lifetime remain separate requirements.

### Current Release result and source-stimulus failure

Clean commit `a0cde9141b514f92d2a6415f633def5f9bb5d221` passes fresh unsigned Windows verification
from 04:40:35 to 04:43:59 UTC on 2026-09-24, including the optimized payload, three runner cases,
both real-engine retirement cases and Release bridge smoke. The retained evidence SHA-256 is
`223FBCC9CDFFF33A02B0BEFE045DE786860F4A22BE68D7C25810492B29EB8FCD`.

Native run `20eac6bdf85346319d15abd637a8de3c` binds successfully and displays all 12 baseline images.
Both native picker confirmations precede 240 seconds. Addition is admitted at 230.797 seconds;
the subsequent root-selection input occurs at 239.418 seconds. A single observed frame then shows
`bulk` selected, 346 images and decoded historical thumbnails while `mixed` still reports an
active import with 3575 found images. This directly observes the selected root-admission behavior;
it does not establish complete native acceptance or transient-frame stability.

The source-stimulus helper copies 1830 of the planned 2000 added files before its unchanged
120-second phase limit expires. Its result records `Source stimulus exceeded 120 seconds`.
No addition-complete, removal, background-import-complete or closed-catalog receipt exists. The
last background observation is verification of 10000/10000, not a completed import. The guest
retires the application through failure cleanup. Subsequent normal Sandbox close and confirmation
retire the host; the parent fails after 403882 ms with no remaining Sandbox process or cleanup
failure. This is not normal client-close timing or a passing lifetime.

Entry availability is 8407019520 bytes, minimum observed host availability is 3512258560 bytes,
and application kernel peak working set is 448610304 bytes. The declared memory limits are met;
these measurements do not explain source-copy latency. All 33 helper hashes remain unchanged.
Independent result review confirms the structured failure boundaries; its three minutes consume
the remaining review allowance. All 10516 host generated sources pass the postcheck in 65.787
seconds. Guest source postchecks and exact closed membership remain unreached after failure.
The consumed lifetime is retained without an unchanged replay. Delete-time date proof remains open.

### Bounded source-copy cost correction

A fixed 96-file, 72962832-byte generated sample separates path validation, source verification,
copy/date assignment and destination verification. The first diagnostic fails only when projecting
ordered dictionaries into `Measure-Object`; its generated copies remain retained. Explicit typed
PowerShell records correct that reporter before the separately labeled measurements.

The original helper takes 2.0044891 seconds (634 ms paths, 483 ms source, 355 ms copy and 507 ms
destination). A typed .NET file-identity owner takes 1.029733 seconds (150, 241, 349 and 278 ms).
The subsequent original-helper control takes 1.965297 seconds. These host samples demonstrate
repeated helper-call overhead, not proof that the guest's 120-second failure has been eliminated.

The 93-line C# owner retains ordinary/reparse path checks, complete source and destination SHA-256,
creation/modification ticks, no-overwrite copy and deterministic hash-stream disposal. The 114-line
PowerShell facade retains run ownership, exact roster, cancellation, removal and phase limits.
Guest preparation includes the new owner. The original 17 guard checks and 24 additional checks
pass; the latter cover same-length content replacement, changed dates, locks, malformed identity,
rooted/traversing paths and reparse targets without changing the source or rejected target.
The new dedicated test file has 85 lines; these owners have no inline tests.

The seven-minute independent boundary review finds no new admission blocker and requires complete
measurement provenance. A separate `profile-source-binding.json` binds the C# owner, PowerShell
facade, profiler, new tests and all three original profile results without overwriting them.
The revised single native lifetime and final three-minute result review are recorded below.

### Corrected-helper lifetime without import admission

Run `827d7a8bcb2d43a3b61eb04a4a418ddb` uses the same verified product payload and the corrected
source helper. The actual picker opens at 85.013 seconds; its next input occurs at 263.813 seconds
after a control-session gap. The 240-second import/addition admission is already expired. No import
confirmation or batch request occurs. The picker is cancelled, and the batch result retains
`added=0`, `removed=0`, `state=seeded` and `Batch stopped before the complete admitted sequence`.
This input-scheduling failure supplies no measurement of the corrected helper's guest batch cost.
Deletion/date navigation, both import completions and exact closed membership remain unreached.

Normal application close is observed at 285.058 seconds. The matching exit receipt reports exit
zero, process retirement and Job closure; the same-host input-to-receipt upper bound is 1647.0173 ms.
All 10000 guest sources retain their full hashes and dates after exit. Normal Sandbox close and
confirmation occur at 396.480 and 404.779 seconds. The final input returns before capture reports
the disposed target. The parent finishes with failure at 407266 ms, no remaining Sandbox process
and no cleanup failure. Normal cleanup does not turn the incomplete sequence into a passed lifetime.

Entry available memory is 8222470144 bytes, minimum host availability is 3686486016 bytes and the
application kernel peak is 156971008 bytes. These meet the original resource bounds. The final
host check verifies all 10516 generated sources in 55.239 seconds. Its retained-catalog counts
belong to the source-preservation fixtures, not this run's unimported catalog.

All 36 helper/profiler/test hashes, all 41 guest-input hashes and the 37 product hashes from the
passing Daily remain unchanged. The frozen helper manifest SHA-256 is
`305F7827D2D4BD1408A0218220D1F9CC6E54F61F38DECA3DFFCF4BEB5AD1B162`;
the separate measurement binding SHA-256 is
`17674E65B214AA80CD55CC373D141C888E2FB51EF2BCBB13B87221EDB6032943`.
The final three-minute independent review confirms the failed admission and limited cleanup/source
evidence. It independently checks the helper/measurement binding and two typed-owner inputs; the
complete 41-input hash check is separate primary verification. The source-cost method is consumed
and ends here without an unchanged replay. Native date/publication acceptance remains open.
