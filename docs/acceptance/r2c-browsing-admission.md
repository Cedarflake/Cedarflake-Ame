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

### Reported finalization and refresh behavior

The preceding import observation is reported as 10000/10000 with continued verification, gallery
refresh and changing image totals. The retained native record never establishes import completion.
The source helper's independent 120-second failure ends this lifetime before its 300-second import
acceptance deadline; it does not close the reported finalization/refresh behavior. The original
failure remains failed, with its stop cause and unmeasured product behavior distinguished below.

Current source keeps file validation separate from atomic publication. While publication is delayed
by pending Live work or preemption, `scan_library/publication.rs` can emit another `Finalizing`
event with the already complete validation counters. `LibraryScanSession` does not request a catalog
reload for that event; only `Completed` enters the published-reload transition. Independent peer-root
publications can still change the selected gallery and total. This explains possible coexistence,
not the precise cause or duration of the observed run. The next native lifetime below supplies
terminal-task and settled point observations, but leaves exact closed membership and the preceding
finalization interval unverified. The original time limits remain authoritative.

#### Original stop and import deadline attribution

The original run's second `confirm-import` has a unique completed before/after input pair:
04:50:25.680–04:50:25.860 UTC on 2026-09-24. The host first observes the matching `guest-result.json`
at 04:52:30.0325638 UTC. That atomically published result already records the application retired,
its Job closed and no cleanup failure. Both interval endpoints come from the host; guest UTC and
the 403882-ms parent lifetime are not used to measure import duration. From before input delivery
through observation of completed retirement, the recorded upper bound is **124.3525638 seconds**.
The background import therefore did not receive the full original 300-second acceptance allowance.
This is the test's completion deadline, not an application-internal timeout.

The prepared helper sources establish the stop chain: `batch_stimulus.ps1` expires its independent
120-second addition phase after 1830 copies; `batch_owner.ps1` detects that process exit;
`media_guest.ps1` enters failure cleanup and closes the application Job before atomically publishing
its guest result. `run_media.ps1` subsequently rejects that failed process boundary. All 33 prepared
helper hashes still match the prelaunch manifest. This attributes the test stop to the source
stimulus, not a measured import-acceptance deadline failure. The later typed-copy correction and its
98458-ms complete addition are recorded above; this attribution does not admit another unchanged run.

The read-only assessment in `.build/r2c-finalization-stop-attribution/assessment-with-observations.json`
binds 40 original receipt/manifest/helper inputs, rechecks them after reading, verifies the completed
confirmation pair and failure chain, and finds no `observed-complete-10000`, normal app-exit or
addition-complete receipt. Its SHA-256 is
`125F1DBC0BF6829A654D1F6611F40D687AAB1753212901AA5CDDBF7996473AB9`.
The original application output logs are empty and no closed catalog was preserved. Exact time
spent finalizing, its publication wait reason and continuous gallery stability remain unmeasured.
Neither this arithmetic nor the later selected completion observations close those product limits.
The bounded independent review verifies all 40 bindings and the assessment method hash, confirms
the failure chain, and retains this conservative timing and product-evidence distinction.

#### Controlled finalization and gallery invalidation diagnostic

An ignored Flutter diagnostic composes the production `AmeApp`, primary scan lifecycle, viewport and
gallery widgets with controlled scan/catalog ports. Its generated decoded PNG backs 96 initial
asset records, followed by a coherent 128-record peer snapshot. The 10000/10000 validation counters
and imported-root count are injected values, not a real 10000-file scan. The window is 1280×800 on
the Windows target platform; this is not native input or mixed-media Release acceptance.

The first three invocations fail because the fixture's asynchronous catalog callback calls guarded
`expect` while `WidgetTester.pump` is active. The third records that exact framework conflict in
the task error; its oversized text then overflows the task-surface column by 110 pixels. Replacing
only that fixture assertion with `expectSync` preserves the query check and produces a passing
fourth invocation. Original logs and source versions remain retained. The induced error-text
overflow does not reproduce a normal import transition or establish the old native incident's cause.

The corrected diagnostic checks 100 deliberately advanced frames: 60 across 30 complete-validation
events, ten during a held peer read with ten further validation events, ten after peer publication,
ten during the held committed import reload, and ten after its release. All 16 selected visible
tile location IDs and rectangles remain equal to their baseline, decoded image objects remain
non-null, the rail remains present, and no Retry label or framework exception appears. The selected
root count advances from 96 to 128 only after the controlled peer snapshot arrives. Import remains
scanning before its terminal event, refreshing while its committed reload is held, and visibly
completed after release. One late Finalizing event is injected after the Completed event while
that reload is still pending; it cannot reopen the scan or prevent the visible terminal result.

The fixture records exactly two `_Catalog.load` calls: peer refresh and committed import reload.
Repeated Finalizing events admit no additional gallery snapshot load. Timeline calls are not
counted; these are not total database/I/O counts. Non-null image objects and sampled geometry are
not a pixel oracle or proof of continuous native stability. The independent review confirms the
fixture correction and these limited claims. No product change follows this unreproduced frontend
hypothesis. Real backend publication duration and the reported native refresh interval remain open.

`.build/r2c-finalization-frame-diagnostic/assessment.json` binds all four original logs, three earlier
fixture versions, the final fixture/PNG and its two shared test helpers. It records the clean product
base, library tree and 37 hashes still matching the passing Daily; no test process remains. Its
SHA-256 is `3B65D0F385055A8F0E4BD40ABDD6093AFF9F61D1067CA2955B80C3A80BD4A24D`.

### Input-stage correction and incomplete deletion admission

The ignored native input helper now has an independent 40-line scheduling owner with 60 dedicated
test lines and no inline tests. It requires an observation younger than 30 seconds, the first import
before 150 seconds and both imports before 240 seconds. Only a completely observed input advances
the phase. Initial admission rejection latches failure; observing again cannot restore ordinary
input authority. Retirement requires a fresh observation and explicit retirement admission. The
connected facade is 190 production lines with 181 dedicated test lines and no inline tests.
All 21 Node checks pass, including rejected-input recovery and retirement. The six-minute prelaunch
review identifies and verifies the initial-admission latch correction before the single run.

Run `af81d94b0ae34583b0d5697a297cee3f` uses the unchanged verified Release payload. Native import
confirmations occur at 86.306 and 197.744 host seconds; the addition request follows at 198.118
seconds. The first import visibly completes with all 12 baseline images. The source helper then
completes all 2000 additions in 98458 ms, within the original 120-second phase bound. This is the
corrected helper's first complete guest addition measurement; it does not establish removal cost.

The observation after selecting `bulk` during active import still shows the previous gallery title,
366 images and the active `mixed` import. No subsequent observation arrives for 167.475 seconds.
The next frame shows selected `bulk`, 2012 images, decoded historical thumbnails and the actual
`Import complete` task with 10000 imported images. Explicit observations record addition convergence
at 200.838 seconds and background import completion at 201.218 seconds, each measured from its own
admission. A second frame 19.349 seconds after the first retains the same count, content and position.
These are settled point observations, not continuous-frame stability or a measurement of the earlier
10000/10000 finalization wait. The preceding suspected stall remains unresolved.

The removal request is rejected after the unchanged 390-second host admission deadline. No removal
request, source deletion or historical-rail input is delivered. The batch retains `added=2000`,
`removed=0`, `state=added` and `Batch stopped before the complete admitted sequence`. This is an
input-scheduling failure, not a failed product import. It also does not repeat the earlier same-frame
proof of completed root selection during an active peer import.

Normal app close occurs at 419.798 seconds; exit zero, application retirement and Job closure are
confirmed within a same-host upper bound of 1425.2227 ms. All 10000 guest source files retain their
10921494393 bytes, hashes and dates; that postcheck takes 70742 ms. The incomplete batch prevents
closed-catalog copy and its separate source postcheck. Exact imported membership is therefore
unverified; the complete two-root verifier is not invoked against missing prerequisites.
Sandbox close and confirmation occur at 549.836 and 563.632 seconds. The final input returns before
the disposed target becomes unavailable for capture. The host fails at 565154 ms with no remaining
Sandbox process or cleanup failure. Normal cleanup does not constitute a complete passing lifetime.

Entry memory is 9023041536 bytes, minimum host availability 4132655104 bytes and the application
kernel peak working set 449998848 bytes, all within the original bounds. The host postcheck verifies
all 10516 generated sources in 67.83 seconds; retained catalog counts belong to those fixtures, not
this run's imported catalog. All 38 helper, 41 guest-input and 37 product hashes remain unchanged.
The frozen helper manifest SHA-256 is
`72DDB89CDB42C20EAC9ADE8EB0F48655E6F6B4529F27B2CA635E8B4855EEC167`.
The final two-minute independent review finds no contradiction in the structured timing or failure
classification. It does not independently rejudge screenshots or recompute all source hashes; those
remain primary verification. The input-stage method ends without another unchanged replay.

### Dedicated control and per-root preview coverage failure

Run `143c3d4e517042719b319e7540194cd8` separates native control from preparation and result checking,
using the same 38 helper sources and verified Release. Both picker confirmations occur within their
original limits, at 142.378 and 230.544 host seconds. A subsequent frame shows selected `bulk`,
866 images and decoded dated tiles while `mixed` is still importing. Addition takes 103894 ms;
the observed 2012-image convergence is 123.510 seconds after its request.

At approximately 317.099 host seconds the task shows finalization at 10000/10000 with the selected
bulk count still 1461. At approximately 342.429 seconds it shows actual import completion and
2012 selected images. The explicit terminal observation is 123.848 seconds after import admission.
This brackets the remaining wait between two observations, not the full finalization duration.
It does not reproduce an indefinite finalization stall or resolve the preceding interrupted run.

Removal is admitted before 390 seconds and physically removes 1500 generated files in 11174 ms.
The actual historical-rail click at 393.493 seconds occurs during remaining catalog convergence,
with 702 selected images. The source deletion itself has already finished. Subsequent observations
show 512 images and decoded content around 2012 without wheel input; observations more than ten
seconds apart retain the same pixels, count and rail position. The explicit 512-image observation
is 71.832 seconds after removal admission. These are selected native point observations, not a
continuous-frame no-flicker assertion.

Normal app close is delivered at 449.858 seconds. Independent host receipt verification gives a
930.675 ms close upper bound, exit zero and Job retirement. Both source helpers retire, all 10000
guest source hashes/dates pass in 69218 ms and the exact 512-file remaining bulk source passes.
The catalog is copied only after app exit. Sandbox close and confirmation occur at 575.950 and
586.433 seconds; the final capture reports the disposed target after the click returns. The host
boundary passes in 589304 ms with no remaining process or cleanup failure.

The unchanged complete catalog verifier nevertheless fails at `A root has no verified ready
previews`. Before that assertion, it verifies both exact active memberships, source metadata and
identity, empty outstanding queues, bounded cache inventory and all ready-artifact ownership.
A separate read-only diagnosis finds `bulk` with 60 ready and 452 pending previews, and `mixed`
with 10000 pending previews; neither root has an active failed preview. Native control never selects
the completed `mixed` root to create its visible preview demand. This is a missing required test
interaction, not evidence that hidden-root pending previews failed. The complete verdict remains
failed; the per-root assertion is retained and the verifier is not replayed unchanged.

Entry memory is 8173584384 bytes, minimum host availability 3875487744 bytes and app kernel peak
449110016 bytes. All 10516 host generated sources pass in 52.441 seconds. All 38 helper, 41 input
and 37 product hashes remain unchanged; helper manifest SHA-256 is
`3303684DAD33F7E4EC9FCA9054A0BF888FF72A73967B44EC4D2EB2D92921591F`.
The primary verification independently checks the receipts, closed-catalog failure and source
bindings. The native controller's image observations remain separately attributed; no second
image review or complete R2c acceptance is claimed.

### Per-root continuation and rejected deletion admission

Run `5ab135e745fe4108aae5b1d61c489515` retains the same payload, helpers, source roster and original
bounds. Picker confirmations occur at 131.598 and 229.877 host seconds. A subsequent observation
shows selected `bulk`, decoded dated tiles and 754 images while `mixed` is still importing.
The source helper completes all 2000 additions in 116973 ms, within its unchanged 120-second bound.

The native controller observes finalization at 10000/10000 at 06:23:14.190 and 06:23:29.436 UTC;
the selected bulk count changes from 1251 to 1307. The first actual completed-import observation
is at 06:24:01.919 UTC, with all 2012 bulk images. These point observations distinguish completed
validation counters from published import completion. They do not measure the exact transition
instant, establish continuous-frame stability, or resolve the older reported finalization stall.

Automatic approval rejects the combined call containing the next removal request before execution:
`The action includes request('remove'), initiating deletion of 1,500 source items; the user did
not explicitly authorize this exact destructive operation.` The rejected call does not publish
its preceding addition observation: a later CreateNew write succeeds at 06:25:24.711 UTC. That
receipt records 212.014 seconds after addition admission; the separate terminal-import observation
records 212.312 seconds. No earlier timestamp is backfilled. There is no removal request,
publication receipt, deletion, historical-rail input or completed per-root preview-demand sequence.

Normal app close occurs at 442.192 host seconds. The independent same-host close upper bound is
994.8431 ms, with exit zero and Job retirement. All 10000 guest source hashes and dates pass in
72396 ms. The batch ends explicitly incomplete with `added=2000`, `removed=0`, `state=added`;
its source postcheck and the closed catalog are unavailable. The exact canonical verifier path,
`output/catalog/ame.sqlite3`, is independently checked absent. A supplemental local receipt
corrects the noncanonical catalog filename listed by the first post-run binding record; neither
record claims a catalog pass. The full verifier is not invoked against missing prerequisites.

Sandbox close and confirmation occur at 581.146 and 601.899 seconds. Capture loses the disposed
target after the confirmation input returns. The host ends at 604294 ms with an incomplete guest
boundary, no remaining Sandbox processes and no cleanup failures. Normal retirement does not turn
this authorization failure into a passing whole workflow.

Entry availability is 8207826944 bytes, minimum host availability 3947737088 bytes and the app
kernel peak working set 450699264 bytes. All 10516 host generated sources pass in 57.462 seconds.
All 38 helper, 41 guest-input and 37 product hashes remain unchanged; manifest SHA-256 is
`9FE81C6B7EC9A9EAEC12EEAA8EEE3870C92CC0FE1A58C9B827212A69812853F8`.
The requested exact deletion scope is verified as manifest-owned generated copies inside the
disposable guest `Documents\bulk`; host source mappings are read-only. Explicit approval is
requested before another deletion lifetime. No alternative tool or guard change retries the action.

The current product remains bound to the passing full local Daily and unsigned Release records.
Required hosted checks for documentation head `5b578582639d36f9c228433f4d57d63fee962b61` also
[complete successfully](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35959454042).
These gates do not replace the failed native coverage or final candidate and external acceptance.
The scoped independent review checks the three new run records against their structured
receipts and finds no contradiction in these failure classifications. It does not repeat the
source traversal, all hashes or independent pixel acceptance.

### Concurrent addition completion and unrequested position change

Run `b15a9218afd541149259a47267d4b8d0` uses unchanged Release product `a0cde91` and the full
10000-file, 10921494393-byte mixed-size/historical corpus. Its separately reviewed addition-only
helper admits 12 baseline images and 2000 additions, rejects removal, and retains the original
deletion/date/per-root verifier unchanged. Focused helper checks pass: 22 PowerShell guards, five
owned-process cases, 21 Node cases and 44 Python oracle cases. Independent admission review
corrects the inherited deletion terminal condition and the final-copy deadline check before launch.

Both import confirmations meet their original limits, at 107.333 and 195.052 host seconds; the
addition request also precedes 240 seconds. A selected peer gallery shows 259 decoded images
while the background import has found 7400. Complete validation at 10000/10000 is visible at
262.693, 277.143, 295.396 and 309.778 host seconds, while the peer count grows from 857 to 1841.
The first point showing actual import completion and 2012 peer images is at 324.313 seconds.
The later explicit completion receipt records 159.063 seconds from import confirmation; it is
an observation bound, not the exact backend transition time. Source addition takes 101926 ms;
the explicit 2012-image receipt is 159.682 seconds after its request. Both retain the original
120-second source and 300-second convergence bounds. This run completes; it does not explain
the backend wait in the earlier interrupted run or establish continuous-frame stability.

The native point observations also expose an unrequested browsing-position change. The peer's
first fully visible date is 2013-08-02 during complete-validation observations, then 2022-12-02
at 324.313 seconds and again at 354.299 seconds. Recorded UI input is absent between the peer
selection at 218.842 seconds and the next root selection at 368.884 seconds; no wheel or rail
input occurs. This is an open C05 position-continuity finding, not proof of a specific anchor or
backend cause. One transient placeholder at 262.693 seconds has decoded by the next observation;
no persistent Retry failure is established by these samples.

After explicitly selecting each root, native observations show 10000 mixed images and decoded
tiles, then 2012 peer images and decoded tiles. The final peer observations, 30.633 seconds apart,
retain the same count, date and position. The closed-catalog oracle passes both exact memberships,
source identities/dates, two completed active scans with matching published counts, no unfinished
work and ready artifacts owned by each root. It verifies 108 and 36 ready previews, with 145 cache
files occupying 2880084 bytes. Unrequested pending previews are not failed previews.

Normal app close at 471.532 seconds has a same-host exit upper bound of 1204.9208 ms, exit zero
and Job retirement. Both source helpers retire. All 10000 guest hashes/dates pass in 45977 ms;
the peer postcheck verifies exactly 2012 files and zero removals. The catalog is copied after exit.
Sandbox confirmation returns before capture reports `no screenshot targets found`; the independent
host boundary passes at 572174 ms with no remaining process or cleanup failure. Entry availability
is 7543533568 bytes, minimum host availability 2924281856 bytes and app peak working set 449552384
bytes, within the unchanged resource bounds. All 10516 host sources pass their postcheck in 59.549
seconds. These generated-source checks do not access real-library roots.

All 44 helper, 42 prepared-input and 37 product hashes remain unchanged. The helper manifest
SHA-256 is `0AD8CF129A393C947D44DF579AA46C620D9E903D42A364D0AB48BF1B99BD96D6`;
the 27-artifact final binding is `7C949B70843FBED8D990805D121F222BACC8D73FF13FF7D50B6DC8A13788BF75`.
Ignored records live in `.build/r2c-concurrent-addition` and the run's integration-storage directory.
Primary screenshot annotations are bound by UTC and observation-log ordinal because the tool reuses
`screenshot-0`; that identifier alone is not unique evidence. The selected addition/catalog/lifetime
checks pass, while unrequested position movement, complete C05 acceptance, the pending deletion
workload and final candidate duties remain open.

The independent result review verifies all 27 evidence bindings and their timestamp/ordinal
identities, convergence/admission limits, closed-catalog result, source postcheck receipts and
normal lifetime. It finds no new blocker within that selected scope. It does not independently
rejudge pixels or repeat the full source traversal; position continuity remains an open exit.

### Committed import position correction

The connected page regression reproduces the observed class of movement: after a peer import
completes, adding newer dated items leaves the scroll pixel offset unchanged while replacing the
visible 2013 images with 2022 images. Ordinary synchronization enters the screen's stable-image
position workflow; the primary import's committed reload previously read the first catalog page
directly. That path neither supplied the visible identity nor restored its resolved ordinal.

The correction connects committed refresh to the same position workflow through an optional,
identity-owned projection registration. Catalog admission and coherent bounded reads remain in
the application. Position capture, identity/fallback resolution and stale-presentation retirement
move from the screen into `LibraryGalleryQueryTransition`; the screen composes its layout result.
The returned query window's start remains distinct from the anchor's ordinal. A detached gallery
does not prevent headless catalog refresh, and its old registration cannot detach a replacement.
No scan, persistence, bridge, dependency, source-file or status-text change is included.

The original page counterexample fails before the change and passes afterward. Both 48-image and
504-image additions now retain the visible identities and rectangles without further input; the
latter places the old position outside the first 500-item catalog window. Focused cases cover
nonzero window offsets, changed-location identity resolution, missing-anchor fallback, old success
and failure during a newer request, read failure cleanup, disposal during viewer reconciliation,
registration replacement and headless/disposed admission. The existing committed-refresh and
coherent-snapshot regressions also pass. The first diagnostic incorrectly waited for an ongoing
progress animation to settle, then switched to bounded frame observation; a later test-only matcher
typo is corrected before the bounded-window pass. Neither is a product failure.

Independent review identifies a further boundary: a held committed read could restore its old
position after a later user scroll. The connected page regression adds real drag input while the
read is held; both addition sizes fail before this correction. A projection read now retains an
opaque position-generation token, rejected before catalog publication when scrolling supersedes
it. The committed obligation survives that proven replacement, waits for the gesture to end and
reads from the new position. New user queries supersede the waiting request's authority; disposal
releases gesture waiters. Arbitrary
read failures are not retried, and independent query/publication generations remain intact.

All four connected page cases now pass, including the two later-scroll counterexamples. The final
six focused files pass 36 tests across page composition, projection registration, primary refresh,
committed admission, coherent snapshots and presentation ownership. Differential independent review
checks the rejected-before-publication boundary and gesture/query/disposal cases, with no further
finding in this scope; it does not rerun tests or establish native behavior. The first lint capture
incorrectly promoted Cargo's ordinary stderr progress to a PowerShell exception. A separate
stdout/stderr child-process capture retains that failure and then completes the canonical lint
with exit zero, no analyzer issues and unchanged source hashes.

Physical sizes, including blanks/comments: controller 357 production lines, viewport 1350, screen
2131, projection registry 77, committed-refresh coordinator 136, shared query outcome contract three
and query-position owner 213; all have zero inline test lines. Dedicated new page, position-owner
and registration tests are 294, 232 and 72 lines respectively; the existing snapshot-reader and
refresh tests are 446 and 157 lines. Larger unrelated facade responsibilities remain the roadmap's
retained decomposition duties.

Complete serial Daily passes on base `94f52f0` with the 12 changed source/test files frozen by hash.
The main Rust suite passes 1539 cases with its existing 19 ignored cases; the broker binary passes
three cases. All 96 Flutter test files, Windows scan integration, required whole-window UIA phases,
native process/Job retirement and 16 asynchronous bridge contracts pass. The ignored manual
performance, wrapper/subprocess and authorization-bound cases are not counted as accepted here.
The captured invocation takes 2157.987 seconds, exits zero and verifies all frozen hashes unchanged.
Its receipt SHA-256 is `741BC21250F7C4FE986BEE22E1E36ED9BBBB213D694A888A5423D6A61EE4FEAB`.
Logs and the exact source list are retained in `.build/r2c-position-continuity`.

Native preparation reuses the 44 previously checked addition-only helpers byte-for-byte. Independent
admission review identifies and closes an artifact-binding gap before the unsigned invocation:
the completed gate now records its fresh evidence digest, and preparation requires that digest,
the current evidence and the configured payload evidence to agree. This avoids inferring a dirty
candidate's artifact identity from the unchanged base commit alone. The fresh unsigned Windows gate
passes in 124.535 seconds with exit zero and unchanged source hashes. Its runner/engine checks and
optimized bridge smoke pass; the latter does not open a catalog. Build evidence SHA-256 is
`1D7E207D3F945C21D250BF67C637AA96AE0B7320A29D84CD5C2E0D14DD646662`.

The one changed native lifetime, `00224139fc8e488192ae5b7f13baceea`, uses that exact payload and
the complete 10000-file, 10921494393-byte mixed-size/historical corpus. Baseline and full imports
are admitted before their original 150/240-second deadlines. The peer receives all 2000 additions
in 103.836 seconds with zero removals. Completed import is explicitly recorded within 152.704 seconds
of picker confirmation; the peer's 2012-image convergence is recorded within 137.567 seconds of
the addition request. These are observation upper bounds, not exact backend transition durations.

With no input between selecting the growing peer and explicitly switching roots afterward,
the middle row's three distinct 2014-09-01 images retain their visible position across growth,
10000/10000 Finalizing observations and completed import. The sampled middle-row rectangles remain
at y=468–606 with columns x=300–346, 353–537 and 544–727. The first visible date changes from
2014-10-01 to 2014-09-02 as new items arrive above the retained anchor; this does not establish
that every visible row stays unchanged. No sampled completion jumps to a newer year. The connected
page tests separately assert exact location identities and rectangles, including later scrolling.

Explicit root selections then show 10000 and 2012 images, each with decoded visible tiles. Cold
placeholders after selection resolve on the following observation. The last two decoded peer
samples are 48.322 seconds apart with the same dates and geometry. These are manual point-in-time
pixel observations, not continuous no-flicker or frame-timing evidence; the reviewer does not
independently rejudge the pixels. All 11 notes bind to distinct original observation timestamps,
JSONL line numbers and returned window identity; the reused `screenshot-0` identifier is not a
standalone screenshot identity.

The original aggregate verifier fails before catalog assertions because the recorded normal-close
action is named `normal-close-app`, while its frozen predicate requires `normal-close-ame`. The
failure and raw receipts remain unchanged. A separately reviewed offline adapter verifies the one
adjacent successful click pair, fresh observed target, coordinates, receipt hashes and matching
app/Job retirement, then normalizes only those two labels in memory before applying the unchanged
predicate. Its three tests include ten rejecting subcases and guest-clock skew controls. Result
review removes an unsupported host/guest clock comparison: the close bound now uses only host
input and first-exit-observation times; guest exit fields establish receipt consistency. The initial
adapter and result are retained, and the corrected close report reuses the unchanged passing catalog
assertions. This is separate offline verification,
not a passing original aggregate invocation; no native replay or deadline change is performed.

That offline verification passes all unchanged closed-catalog assertions: exact 10000/2012
membership, source identity and historical metadata, completed publications, no unfinished change
queue and independently owned ready previews in both roots (83 and 42). The bounded cache has
126 files and 2674442 bytes; no active location remains failed. Guest source postchecks cover both
complete directories. The full 10516-file host postcheck passes in 56.798 seconds with original
identities, bytes and dates preserved. No real source tree or deletion stimulus is accessed.

Normal application close is bounded by the host's first exit observation at 1433.4 ms, with exit
zero and closed Job. The Sandbox exits within 767.137 seconds, leaving no owned process or cleanup
failure. The final close input returns successfully; its following capture reports the disposed
Sandbox target, retained separately from input failure. Host entry is above seven GiB, sampled
available memory stays above two GiB and the app peak working set is 428.2 MiB. Postchecks confirm
all 44 helper, 42 prepared-input and 46 product/source hashes unchanged. Forty-five evidence bindings,
including the original aggregate failure, offline adapter and gate receipts, are retained in
`.build/r2c-position-native/final-bindings-v2.json`, SHA-256
`430E1BA5FAA41C7D1142091CC2C3C5BCDAE99393EE2F46ADCE72D293DFAE7EA2`.

This closes the selected committed-import position correction with focused, current-source gate
and native observations plus offline catalog/lifetime evidence. Complete deletion/date/per-root
acceptance and attribution of the original finalization wait remain open. This run again observes
complete validation counters before terminal publication, but does not establish the old wait's
cause or accept all C05/R2c duties.

Hosted run [35994900094](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35994900094)
passes all ten required jobs and the aggregate gate at `562e53d2d182ec0b1ddb5bc8389529d4c02f9a7e`.
The three signing-only jobs are skipped as required for this PR. This is the committed-position
checkpoint's hosted result; it neither validates later test changes nor closes its remaining
native and external duties.
