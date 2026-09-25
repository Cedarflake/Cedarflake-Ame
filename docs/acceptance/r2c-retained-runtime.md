# Retained-library runtime observations

Observed runtime baseline: `dcce10e3ce7f2f62a61a82adcdede64c1608f501`, 2026-09-25.
Correction: `0677ba67020142d50e4b28cf7f3ce07e44fec924`, verified 2026-09-25.
Status: startup recovery completed; paging correction passes focused, lint, complete local Daily,
fresh unsigned Windows and hosted checks. Recovery cost, the exact retained-client incident and
remaining R2c client acceptance remain open.
The subsequent documentation-only head's first-import Pause failure is retained below. Its owning
correction `724a83e` passes focused checks, complete local Daily and connected native Pause, plus
current-source hosted verification, including every required job and the aggregate;
earlier passing gates do not change the original failed result.

## Existing recovery after startup

The existing Debug process started at 08:14:13.866 UTC. Read-only catalog evidence binds the
`cloud-primary` generation-4 recovery to a watcher coverage gap authorized on 2026-09-08 at
23:48:01.191 UTC. It completed and retired its authority on 2026-09-25 at 08:32:22.466 UTC,
1088.599 seconds after this process started. The retained operation was not newly authorized by
this diagnostic session. Its cumulative inventory contains 50515 entries and 49685 candidates.

This process completed 12211 metadata-inventory path candidates across 193 persisted publication
timestamps. The median inter-publication interval was 5.999 seconds; P95 was 8.538 seconds.
These intervals include unseparated work and scheduling costs, not measured source-read durations.
Later terminal snapshots report synchronized state, zero pending/retry work and zero gaps.
The persisted journal capability remains `live_only`; completed recovery does not prove persistent
coverage across a future closed-process interval.

All diagnostic SQLite connections used `mode=ro` and `query_only`; summary queries had 350 ms
progress deadlines. No scan, source enumeration, source content read or source mutation was
initiated. A 64-item production path-prior SQL sample took 2.057 ms in total (63 records present),
which does not account for the observed batch intervals. On the exact retained root/generation,
the current explicit-recovery claim-count subquery took 16.278 ms; a read-only claim-driven
comparison returned the same zero in 0.045 ms. This is a query-cost finding, not proof of the
18-minute recovery cause or authorization to weaken root/generation ownership. An earlier count
probe selected an unrelated state row, and a subsequent single-registered-root assertion failed;
both are excluded from target performance evidence. The final samples bind the previously
verified root identity and generation explicitly.

The finite recovery now completes, but its cost remains unaccepted. No unchanged real-library
replay or broad optimization is implied by these observations.

## Timeline navigation, upward scrolling and Retry

Severity: S1 browsing-position continuity; source or durable-data harm is not evidenced.

### Observed unsolicited return to the beginning

Run `24b37f47caa941b9994ed8051edfeeaf` uses the corrected source-reconciliation result boundary,
649 bound product files and the same isolated 79281-item settled baseline. Actual middle-rail
input at 19:09:36.554 UTC is followed by upward input at 19:09:46.381. The observer records window
52700, then an anchored revision refresh at window 52461. After upward input it records pixels
4555.333; at 19:09:50.866, without another navigation input, the window becomes zero with pixels
58. This is a reproduced unsolicited jump, but its destination differs from the original reported
earlier, non-top position. Captured tile states contain no failed preview or Retry feedback.

The observer subsequently stops on `Converting object to an encodable object failed: NaN`.
The wrapper fails the run and closes its owned processes; elapsed lifetime is 58981 ms, cleanup
has no failures and the prior catalog copy is unchanged. Normal-close timing and subsequent
reverse/viewer outcomes are unavailable. The ignored `visible-observations.jsonl`, original tool
frames, observer error and `completion.json` preserve both failures. Code inspection identifies
an unanchored stale time-prefetch fallback; a causal regression is required before attributing
this native movement to it. Non-finite render geometry must be classified explicitly before a
subsequent observation; an omitted sample cannot establish visible correctness.

The dedicated passive-prefetch regression reproduces an unanchored return from offset 52500 to
zero. Six of its initial twelve cases fail, including later-position and cancellation boundaries.
The correction uses the existing anchored query projection and coherent snapshot reader while
retaining independent publication, position and time-request authority. Current read failures
remain visible; retired reads and errors cannot replace the gallery. No extra retry, scan or source
operation is introduced. Independent review then reproduces a same-target promotion omission:
an explicit click during passive recovery was lost. The time snapshot owner now retires that
passive result and resolves the still-current explicit intent once, preserving shared completion
and loading ownership. Both success/error promotion regressions retain their causal failures.

A proposed extra microtask case does **not** establish a read-to-publication race: its diagnostic
listener already sees revision two before injecting position input. Preserve its failed attempts
and ordering log, but exclude the invalid before-publication assertion from regression evidence.
The publication boundary still rechecks the same authority immediately before committing state.
The genuine held-read cases retain their original assertions. The current focused checkpoint
passes 14 stale-prefetch, 13 semantic date, ten bidirectional-page and eight manual-update cases.
Fatal-info analysis also passes. The viewport has 1408 production lines, the coherent snapshot
reader 55 and the time snapshot owner 141, with no inline tests; the new dedicated test has 309
lines. The ignored observer's corrected classification passes eight geometry and six metric
boundary examples, including NaN, infinities, clipping and zero-area cases. Native correction and
complete batch gates remain pending at this implementation checkpoint.

The corrected artifact in run `d24ae94c81f3403181fd90a4b58dfe54` **still fails native position
continuity**. Middle-rail input at 19:39:05.349 UTC and upward input at 19:39:16.076 are followed
by window 52461/pixels 4555.333 at 19:39:18.703, then window zero/pixels 58 at 19:39:20.202.
No further navigation input intervenes. This disproves that the passive-prefetch correction alone
resolves the captured native jump. No captured Retry appears, and the observer completes without
its prior serialization failure. Full lifetime is 119508 ms, normal exit zero, close-to-exit
620.5582 ms and no cleanup failures. Peak working set is 467812352 bytes, kernel peak commit
425848832 and minimum host availability 6433083392. The prior catalog/WAL are unchanged, count
remains 79281 and the same 110 persisted unsupported/invalid classifications remain. Membership
metadata changes; a twelve-location logged sample finds three cloud-primary source-token/generation
changes and does not establish their complete population. All files and native events remain in
the ignored run directory. Next diagnosis records the actual publication origin and rendered
anchor availability instead of attributing the movement to another unanchored call by inspection.

### Passive synchronization publication origin

Run `f84f9bc22a264d448e32f76207a6c648` adds a bounded read-only provider observer to the same
product candidate. Middle-rail input occurs at 19:51:32.348 UTC, upward input at 19:51:42.921.
The synchronous publication call stack identifies `refreshFromSynchronization` through
`_updateQueryWithOutcome`. A read started before scrolling publishes revision 1083 at
19:51:46.266 while the wall still renders revision 1075 at pixel 4555.333. The next refresh starts
at 19:51:46.268, before that publication renders, with no valid anchor for revision 1083 and no
pending restoration. At 19:51:47.526 its unanchored snapshot replaces offset 52461 with zero.
The next native frame shows the beginning without intervening navigation. This establishes
the captured first-page jump's publication cause, not every original Retry/non-top/false-bottom
variant. The provider observer performs no writes, catalog requests or forced frames.

The full 249031-ms lifetime exits normally with code zero, 622.7517 ms close-to-exit and no cleanup
failure. Peak working set is 466501632 bytes, kernel peak commit 421232640 and minimum host
availability 3667558400. The prior catalog/WAL remain unchanged. A separate post-run failed-preview
comparison is interrupted by its existing 350-ms SQLite deadline, so this run does not claim a
complete post-run classification or membership comparison. Its native failure and complete process
evidence remain valid independently of that unavailable comparison.

The correction carries the existing typed projection-read authority through passive synchronization
and rejects obsolete success/error before publication. Two connected held-read/upward-scroll
cases first fail with `applied`/`failed` instead of `superseded`; both now retain the middle window,
release loading and preserve the new tile rectangles through a subsequent refresh before drawing
the previous publication. Independent review also finds that an explicit date read retires the
publication generation but leaves the old query baseline/loading. Two additional cases reproduce
that terminal-state failure. The extracted application query-transition owner preserves the
original baseline across consecutive query replacements and retires it on date/external transfer;
old completion cannot clean up a newer token. Subsequent visible-range reads regain admission.

The current focused checkpoint passes 16 stale-prefetch/date-transfer, nine coherent-snapshot,
two connected passive-position and 43 screen cases. The preceding adjacent checkpoint also passes
four projection, six refresh, ten presentation-transition and one stale-page-position cases.
The reviewer confirms the identified transfer defect is resolved. This remains focused evidence
until the changed native replay. The viewport has 1414 production lines; the extracted query
transition has 44 and snapshot reader 60, all with zero inline tests. Dedicated connected-position
and stale-prefetch/transfer files have 279 and 347 lines. Preserved red/green logs are under
`.build/r2c-batch-native-verified-20260926`; the original run retains publication/visible events,
native input timestamps and complete process receipts.

### Corrected passive publication native checkpoint

Run `b0e732489ecd441a915dd9e7ed0c3493` binds 650 product files and 686 total source/helper/payload
files to the corrected candidate. The same native input at rail coordinate 1244/510 and upward
wheel -1030 no longer publishes the middle window as zero. The accepted refresh instead anchors
the newly scrolled item 52679 and moves the bounded window from 52461 to 52429. Following revision
changes preserve that anchor; window-local pixels change when the full-query manifest arrives,
so raw offset equality across those coordinate spaces is not a stability assertion.

Actual reversed downward scrolling continues into subsequent images. A second middle-rail target
at 1244/420, upward scrolling while the loading line is visible, and downward reversal also render
subsequent content without a false bottom in this lifetime. The second input overlaps a loading
presentation, but the exact time-read future had completed just before that wheel event; this is
not proof of the original database-pending variant. The bounded observer records zero middle-to-zero
publications, zero failed visible tile samples and no Retry feedback. Captured native frames settle
with loaded previews in both regions. The previously reproduced first-page jump is corrected for
this exact native sequence; original non-top Retry, false-bottom and continuous-frame variants
retain their remaining obligations.

Normal exit is zero after 239956 ms, close-to-exit 761.5328 ms and no cleanup failures. Peak working
set is 570114048 bytes, kernel peak commit 555819008 and minimum host availability 4792213504.
Post-run read-only comparison uses the existing root/scan/relative-path/location index in pages
of at most 512 under the original 350-ms per-query bound and a 30-second overall bound. It verifies
all 79281 identities, asset identities and relative locations; the original catalog/WAL are unchanged.
Only 152 cloud-primary source tokens/generations differ among the compared metadata fields.
The same 110 preexisting unsupported/invalid classifications remain, with no new failed location.
No source media is opened by that comparison. The earlier aggregate comparison's timeout remains
recorded; this indexed complete comparison changes the diagnostic method rather than relaxing its
deadline. Native events, publication/visible observations, post-run comparison and lifecycle
receipts remain in the ignored run directory. Full combined Daily/Windows checks follow this
functional checkpoint and do not replace it.

The first combined Daily stops in Dart analysis on five `prefer_initializing_formals` findings
in the private prepend-compensation constructor. The preceding guardrails, format and Clippy
steps pass; later Daily components are unexecuted. Replacing the explicit field assignments with
the repository's named initializing-formal syntax preserves argument names, types and behavior.
The failed `combined-daily.log` is retained beside the focused/native evidence; this syntax-only
correction does not invalidate the native causal observation or justify another real-root run.

The reported sequence is a successful middle-timeline seek, upward scrolling, partial Retry
preview feedback, then an unsolicited jump toward an earlier timeline position, specifically
not the top. The report explicitly confirms synchronization had already finished. Treat this as
an unresolved C05 functional browsing incident. The retained terminal tail
contains successful preview materializations and synchronized roots, but lacks the error and
navigation frames needed to establish the precise cause. Its lack of a Retry event is not a
negative reproduction result.

A subsequent 2026-09-26 rerun reports that the earlier problem still occurs. The executing binary
has not yet been bound to the current source for that report. Retain the recurrence as negative
experience evidence; neither an assumed old build nor the separate generated update-completion
correction establishes resolution.

The next/previous expired-cursor boundary is independently reproduced: both directions originally
called an unanchored first-page reload. In a loaded middle window, both regressions failed because
no visible identity was supplied. Recovery now uses the existing query projection and refresh
coordinator to retain the current identity, resolved ordinal and within-row fractions. Continued
scrolling retires the old read and captures the later position after the gesture ends. This proves
that boundary, not that it caused every part of the retained-client report.

Independent review exposed two integration risks in the first correction. A page recovery must
retain its original publication authority rather than inherit a committed scan's obligation to
follow a later query. A held recovery followed by an actual failed user query reproduced the loss
of `LibraryQueryFailed`; the correction preserves that failure without another read. Also, passive
synchronization must obtain query admission before entering the shared gallery projection. The
screen no longer captures a position before a request that may be rejected as busy. Viewer
reconciliation remains inside the mounted projection. A gesture-delayed passive request retains
its original publication authority; the red/green supersession test proves it cannot start a late
read after that authority is retired.

The final focused file has 11 passing cases: both stale-page directions, scroll-before-read,
scroll-during-read, disposal, query replacement, failed newer query, admitted passive capture,
gesture-wait supersession, busy passive coexistence and ordinary read failure. These use three
in-memory assets in a 10000-item timeline, real viewport/refresh/projection owners and the actual
gallery transition. They are not a 10000-image native-client workload. Three primary-refresh and
three actual-widget preview-synchronization cases also passed after the admission composition
change. The final five-file batch passes 83 cases, including the existing stable-asset and coherent
snapshot assertions using an attached fixed projection, four committed-position widget cases and
three preview-synchronization widget cases. A separate actual `AmeApp` widget regression passes
with 120 loaded records in the middle of a 10000-item timeline: it holds the stale-page recovery,
delivers a real synchronization-stream notification through the screen, then verifies the same
visible image identities and rectangles after recovery. It uses one generated one-pixel preview,
not native decoding or real-library media. The earlier seven-file pass remains historical to the
final admission composition.

No source-media, schema, bridge or decoder behavior changed. Existing owners retain the lifecycle:
the refresh coordinator admits work, the projection captures position, and the viewport rejects
retired reads. The affected handwritten production sizes are controller 351, viewport 1362 and
screen 2117 lines, with zero inline test lines. The new dedicated application and widget regression
files are 423 and 263 lines; their shared fixed-projection fixture is 14 lines. Existing controller
and coherent-snapshot test files are 3640 and 454 lines after adapting their anchor setup, with
their original identity and atomicity assertions preserved.
The reused refresh, projection and gallery-transition owners remain 136, 77 and 213 lines.
The larger physical decomposition debt remains with the roadmap; no new state flags were added.

Read-only inspection of current registered published roots found 110 persisted failed previews:
102 nonempty `.jpg` entries with `image_format_unsupported`, six nonempty `.jpg` entries with
`image_decode_invalid`, and two zero-byte `.png` entries with `image_format_unsupported`. These are
stored classifications, not a fresh source inspection or proof that those entries were visible
during the report. The 256-row-capped query returned 110 rows in 86.192 ms. No paths or filenames
are included in the evidence. This finding does not establish the transient Retry cause or justify
suppressing feedback, changing format support, or opening originals.

Partial, sanitized terminal buffers are retained under
`.build/r2c-retained-runtime-20260925/terminal-captures.json`. They do not contain complete process
stdout or wall-clock event timestamps. The existing generated-data and hosted passes do not
certify this new retained-library sequence.

Focused red/green logs and the sanitized failure summary are in the same ignored evidence directory:
`stale-page-before.log`, `stale-page-after.log`, `query-failure-before.log`,
`query-failure-after.log`, `passive-projection-before.log`, `passive-projection-after.log`,
`passive-wait-before.log`, `passive-wait-after.log`, and `preview-failure-groups.json`.
Final focused results are `final-focused-corrected-import.log` and `middle-gallery.log`;
`source-manifest.json` binds the affected Dart sources and fixtures by SHA-256.
The first lint wrapper stopped on Cargo's normal stderr because PowerShell merged it into its
terminating error stream; it did not reach the analyzer. The corrected wrapper runs the unchanged
canonical entrypoint in a separate process with separate stdout/stderr. Its first analyzer pass
found two old test call sites still supplying removed anchor parameters and one redundant import.
The tests now attach an explicit projection without weakening their original assertions. The new
shared fixture initially imported the anchor from the wrong module; that failed compilation is
retained in `final-focused.log`, followed by the corrected passing run. Final lint passes formatting,
Clippy with denied warnings, all tool guardrails and Dart analysis with no issues; its logs are
`lint-final.log` and `lint-final.log.stderr`. Daily's static component passes: 1570 Rust cases,
19 explicitly ignored cases, three broker binary cases and all 16 asynchronous bridge contracts.
The Rust suite took 1142.11 seconds; its existing mixed-load P0 latency assertion also passes.
Those ignored cases retain their
separate explicit gates, and this pass does not explain historical C01/C02 failures. Daily's Flutter
component also passes all 98 test files with no failed files; `daily-flutter.log` and its separate
stderr retain the full run. Both components ran serially on the source bound by the manifest.

Native gates initially waited for the retained process to release their shared Debug output.
After that process exited, the unchanged correction passed both remaining Daily components:
the controlled scan integration completed three cases in a 51.160-second parent lifetime, and
native accessibility completed all ten ordered UIA phases. Both owned process lifetimes and cleanup
completed without failure. Together with the earlier static and Flutter components, this establishes
complete serial local Daily on the same source, without replaying its completed components.
Evidence is retained in `daily-windows-scan.log`, `daily-windows-accessibility.log` and their separate
stderr files under the ignored evidence directory.

Fresh unsigned Windows verification also passes on the clean correction commit: three native
window-lifecycle cases, two actual Debug-engine retirement cases, the optimized application/broker
payload identity and the Release-DLL/native-channel bridge smoke without catalog access. The
canonical receipt is copied to `unsigned-evidence.json`; `unsigned-windows.log` and its stderr retain
the run. The parent PowerShell wrapper labels Cargo's normal compilation stderr as `NativeCommandError`,
but the child exits zero and the canonical receipt records every required boundary as passed.
These generated/virtual and catalog-free gates do not constitute a new retained-library run.

[Hosted CI for the correction](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/36122608963)
also passes, with 11 successful jobs including the final required-gate summary. It covers complete
Daily, the independent unsigned Release job, high-resolution JPEG, all seven supported formats,
10000 images, one million journal records and 50000-identity concurrent publication. Three
conditional release-workflow jobs were skipped, including signed verification and protected
signing; signed installed-service acceptance remains open. `hosted-result.json` binds the result
to the exact correction commit. Subsequent changes to this record do not change the tested product
source or renew these completed gates. Neither these results nor the independent expired-page
reproduction establishes the exact Retry cause in the retained-client report.

## False bottom after scrolling during a timeline load

Severity: S1 blocked browsing; source or durable-data harm is not evidenced. Reported 2026-09-26.
The sequence is a click at an arbitrary timeline position, upward scrolling before the photo wall
has finished loading, then an inability to scroll downward although the timeline is not at its end.
The supplied frame shows 79281 catalog items, visible final rows followed by blank space, and the
rail thumb around its middle. It establishes the reported visible contradiction, not its cause.

Keep this incident distinct from Retry feedback, unsolicited position movement and the generated
manual-update completion failure. Investigation must track the timeline request, intervening user
input, retained window, next/previous cursors and rendered scroll extent together. Native acceptance
must include input during the pending load and downward continuation beyond the first returned
window; waiting for a fully loaded wall before scrolling does not exercise the trigger.

### Delayed-layout prepend correction

A connected 2400-item, 2026-to-2003 fixture now proves a related owning defect. The first date
window starts at ordinal 840. A second time read is deliberately held; upward input retires it
and requests earlier rows. When the full layout manifest arrives before the old prepend's rendered
frame, the old screen calculation subtracts a window-local extent from the full-query extent and
adds the result to the scroll position. The cursor log then reaches ordinal 2378 with 22 final
items; pixels equal the complete maximum 59602. The timeline also moves to the end in this fixture,
so this is not yet proof of the reported native frame's still-middle rail discrepancy.

`LibraryGalleryPrependCompensation` now owns that coordinate boundary and retirement. It adjusts
only compatible window-local layouts using their shared first-item offset, preserving later wheel
input and trailing-window trimming. A full manifest receives no such adjustment. New date/query/
layout intents, query publication and disposal retire the prior operation; an old cleanup cannot
clear its successor. The cause is removed from the position-restoration owner, with no cursor,
extent clamp, retry or source-read workaround.

The original compile-only fixture failure and subsequent causal/geometry/read-order failures are
retained separately under the ignored batch evidence. Four ownership tests and five connected
pending-navigation cases pass, together with all 43 existing connected screen regressions. The
old anchored-page fixture now states its existing 30-item prefix explicitly; its original visible
location and pixel assertions remain. The new connected case distinguishes publication from frame
completion: it proves the previous page has published at ordinal 340 before starting a newer date
target at 744. Native and full-batch acceptance remain open. Current owner sizes are 83 production /
0 inline tests / 156 dedicated-test lines, with 419 connected-test lines at the first reviewed
checkpoint; the screen remains 2148 production lines and retains its separately tracked debt.

A deliberate negative check removed only the new-date retirement hook. The connected test still
passed: it observes publication before the new intent but does not force the old restoration to
resolve after that intent. Preserve this non-detection in `pending-navigation-retirement-mutation.log`;
the original source was restored. The owner retirement unit case and static wiring review remain
evidence, not a connected proof of that exact final-frame race.

### Corrected retained browsing observation

Run `c05ae733efa942208dfc667c987baa7e` binds the dirty candidate's 648 product source files and the
compiled Debug payload. It uses a fresh isolated derivative of the previously settled 79281-item
catalog. The native lifetime lasts 124759 ms and exits normally 654 ms after Alt+F4; app, parent and
monitor retire. Peak working set is 448.15 MiB, kernel peak commit 444.23 MiB, and minimum available
host memory 5.87 GiB. The prior derivative's main/WAL files remain unchanged. Analysis opens no
source media. This is not a recovery run against the user's interrupted original catalog.

Both roots start synchronized. The first middle-rail click, upward input, downward reversal and
longer upward traversal continue through different visible rows without the false bottom. Click
to first upward input is about 9.5 seconds; the displayed loading frame does not establish that the
database request remained pending at input time. The second middle-rail click is followed by upward
input about 11.4 seconds later. That frame contains Retry tiles and a root updating state. The run
is closed to retain the failure evidence; second reversal and viewer return are not performed.

There are no inventory/recovery markers or stderr output, and both roots end synchronized. Exact
asset count remains 79281, but membership-plus-metadata hashing changes: 57 cloud-primary records
advance their Windows ChangeTime revision token and source generation, with the other compared
metadata fields unchanged. Their 57 new consistency-audit paths complete: 27 in one attempt and
30 in two. The local-primary projection is unchanged. The 110 pre-existing cloud-primary failed
preview records retain identical identities/classifications: 104 unsupported formats and six invalid
decodes, with no new persisted preview failure. These aggregates cannot identify the on-screen Retry
tiles or rule out transient in-memory failures. The browse incident remains open. The changed
projection also prevents claiming an unchanged-catalog browsing pass.

Retained evidence includes `completion.json`, `result-analysis.json`, `revision-comparison.json`,
memory and process logs in the ignored run directory. A first offline comparison incorrectly parsed
the revision token as JSON; the corrected analysis uses the repository's scheme/value encoding.
No product change or source read was made for that parser correction.

### Preview reconciliation publication conflict

A deterministic generated-source database fixture now reproduces a result-classification defect.
An injected peer revision commit occurs during source-reconciliation admission. The old preview
attempt then records `retry_wait`, `incremental_catalog_revision_changed`, attempt one and a retry
deadline on its existing durable path. The application nevertheless returns `source_changed_during_scan`
to the preview caller, which can display Retry for that cold thumbnail. The causal RED is retained
in `reconciliation-publication-red.log`. This establishes the boundary defect; the earlier native
frame did not identify every visible failed tile and remains separately qualified.

`SourceReconciliationOutcome` now distinguishes retained durable path work from unresolved admission.
Acknowledged retry/deferral retires that stale preview request, as do completion or supersession.
Unavailable admission and persistence errors retain their failure behavior. No new retry, attempt
reset, lease change, source mutation, or weakened catalog-revision check is introduced. The same
fixture then lets the original path finish on attempt two and materializes the current preview,
with exact unchanged source bytes and one queue record. An injected retry-persistence abort proves
that storage failure remains an error and rolls back the claimed transfer.

All 14 source-reconciliation cases pass, including locked/unavailable source, stale admission,
existing path ownership, source removal/recreation and current pixels. The preceding format-only
rejection remains in `reconciliation-publication-focused.log`; the completed result is in
`reconciliation-publication-corrected.log`. The owner is 165 production lines with no inline tests;
the new dedicated publication-race module is 147 lines. Native and full candidate gates remain open.

## Screenshot arrival and manual update completion

Reported 2026-09-26: a screenshot is saved into a configured library directory and appears at the
top of the gallery, while that root reports blocked updating. The current terminal repeatedly
records `catalog_terminal_media_evidence_path_mismatch`; one root remains `needsReconciliation`
with one pending/retrying item while the other is synchronized. Individual visible publication
does not establish complete root convergence. The bounded captured terminal tail is retained in
the ignored `user-terminal-sync-blocked.txt` receipt; earlier overwritten output is unavailable.

A subsequent manual-update frame shows 30658/30658 images verified and 35117 files inspected, with
the task and sidebar still updating. This remains an unresolved terminal-transition observation,
separate from the earlier deliberately bounded validation workload. Verify actual progress,
publication and lease state before classifying the wait or joining it to the background error.
The second available terminal tail is retained as `user-terminal-finalization.txt`; it still shows
the repeated background evidence mismatch and does not contain the manual task's complete history.

Two read-only derived-catalog observations, about six minutes apart, show the same running manual
scan and one unchanged P0 Live subtree. At the later observation the scan has lasted about
19 minutes 44 seconds. The task has exhausted eight attempts with `change_lease_expired`, no lease
or next retry, while the scan retains 35117 visited and 30658 accepted. This establishes a blocked
publication dependency; it is not evidence of a progressing slow commit. The observations preserve
logical root names and hashed relative-path identity in ignored `live-catalog-state*.json` files.

Generated authoritative-subtree cases reproduce the terminal-media mismatch, and multiple bad
children also reproduce the old evidence-count rejection. The scope owner now admits child paths
under their completed directory lease, retaining the 256-mutation bound and normalized literal
path containment. Cleanup covers terminal-only deleted children and rename predecessors within
the same transaction. Overflow takes existing bounded inventory recovery. Neighbor isolation,
rollback, repaired media, previous-path restrictions and FULL reopen retain focused evidence.
The catalog-delta namespace passes 25 cases after the review correction preserving legal
Path/Reconcile requests with previous cleanup paths. Earlier failures remain in
`subtree-terminal-red.log`, `sync-publication-focused.log` and `terminal-scope-review-focused.log`.

Publication now distinguishes retryable work, retained work eligible for the existing one-time
P2 transfer, and exhausted uncovered work. Retained transfer additionally admits the exact old
`change_lease_expired` shape under the same LiveOnly, generation, namespace, subtree and ownership
guards; the original failure and attempt count survive. The reviewer identified a capacity
dependency: P2 cannot drain while the foreground scan runs. Publication therefore shares actual
transfer capacity, ending the foreground attempt on an exhausted blocker without replacing the
old snapshot when that capacity is unavailable. This grants neither false freshness nor a new
retry allowance. Focused results and fixture failures remain in `retained-recovery-focused.log`;
seven focused cases pass: three admission boundaries, two complete production recovery paths,
and two actual foreground scans ending within their three-second bound while preserving the old
catalog. The full-capacity case retains all 3072 pending P2 entries. Intermediate fixture failures
are preserved: a synthetic P2 transition initially retained its completed revision, and a seeded
lane insert duplicated the schema-owned insert trigger. Corrected fixtures retain both negative
assertions. The final independent static recheck closes the capacity dependency finding; current
native recovery and full batch gates remain open.

New owner production/inline/dedicated-test sizes are 179/0/110 for terminal scope and 96/0/188 for
publication admission. Additional transaction/application coverage has 63 and 234 lines;
the retained queue owner has 99 production lines and its production-runtime test has 185 lines.
These extractions do not complete the larger physical facade decomposition retained in the roadmap.

## Generated middle-timeline native observation

The 2026-09-25 single Debug lifetime uses source `ae5180c`, whose product is unchanged from
`0677ba6`. A freshly built guarded entry calls production startup with isolated derived storage
and in-memory preferences. Run `ef6f3687ee9247c68ab4bc0759d5bcae` imports the frozen generated
10000-image, 10921494393-byte corpus with 12 dimensions and 201 historical months. All 10000
source identities, dates and content hashes match the oracle before and after, in 57.399 and
47.576 seconds. No real-library source or retained profile was opened.

Actual Computer Use observations cover:

- a native picker import, observed complete after 87.872 seconds, with 10000 images displayed;
- a middle-rail click reaching 2016-05-02, followed by source-backed pixels without more input;
- decoded `6000x4000` viewer content at ordinal 4248/10000, then Escape retaining the same visible
  date, tile arrangement and rail position;
- upward wheel deltas of -1320, -3960 and -12000, reaching 2016-07-02 then 2016-12-02;
- reverse +12000 returning the earlier 2016-07-02 arrangement, also unchanged at the later idle
  observation.

The immediate cold-region screenshots contain loading slots. Later observations show all visible
slots filled, with no observed Retry feedback or unsolicited position jump. This samples frames;
it cannot prove every intermediate frame, the entire 10000-image preview set, a forced stale-cursor
race or the original real-library incident. The 107 emitted preview-queue diagnostic records all
report `ready`; the logger omits some fast requests, so this is not an all-request trace. Maximum
logged total/queue/active times are 13565/11620/6224 ms. Those Debug cold-preview costs remain
performance observations, not a responsiveness acceptance pass. Engine stderr is empty.

The complete native sequence and closed-catalog verifier finish in 440394 ms under the original
900-second bound. Normal close takes 551.4179 ms; application and parent exit zero, the owned Job
closes, and handles/monitor retire without cleanup failure. Peak working set is 709394432 bytes,
kernel peak commit 778416128 bytes and minimum system availability 4844118016 bytes. The closed
catalog has exactly 10000 active members and 160 owned ready preview artifacts totaling 3408886
bytes, with no active failed-preview or unsettled-change rows.

Preparation passed 68 admission, read-only, identity, capture, verifier-deadline and lifetime checks.
The native API and screenshot-coordinate inputs work. Early recording attempted to combine two
different REPL module instances; the original capture guard rejected that mismatched registration.
Using the same evidence module for capture and observation preserves that guard. An indexed click
also failed because its cached element was unavailable; a refreshed screenshot-coordinate click
then opened the picker. One later transient notice expired before its coordinate click, so the
click opened the underlying viewer; the actual viewer state and Escape result are recorded rather
than claiming the intended notice dismissal. Pre-input records are attempted inputs, not delivery
acknowledgements. No failed action or missing observation is counted as a successful workflow step.

The ignored helper owner is `.build/r2c-middle-timeline-20260925`; the GUID storage above retains
`started.json`, `native-inputs.ndjson`, observation receipts/screenshots, stdout/stderr, memory,
closed-catalog and source-before/after evidence. Thirty payload/helper hashes remain unchanged.
Independent method/result review checked those hashes, 28 observation identities and 31 screenshot
references, exact membership and resource/retirement receipts. It did not independently re-observe
the pixels. No product changes or repeated Daily were needed for this selected native observation.

## Subsequent hosted first-import pause failure

[Run 36126310364](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/36126310364) on `ae5180c`
fails the Windows Scan component after a successful build. In the retained-import test, the
Started listener requests Pause. The stream instead reports
`persistent_journal_first_import_inactive: The first-import journal probe lost its executing scan owner`
at `integration_test/support/retained_import_workflow.dart:75`. A later picker test then observes
two roots where it expects one; this follows the earlier failed retained-import flow. The middle
picker-cancellation case passes. The parent exits one after 253757 ms; cleanup has no failures.

The initial code trace identifies a control-versus-probe interval: the scan checks pending control
before polling first-import capture, while journal opening rechecks the executing capture lease.
A Pause arriving inside that poll can invalidate its lease. This is a concrete lifecycle race to
reproduce at the owning boundary, not evidence that the journal's stale-publication guard should
be removed or that the failure is only hosted-machine speed. No unchanged retry was requested.
The downloaded three-file diagnostic artifact and job log are retained in the helper evidence
directory. The earlier correction's passing hosted/local checks remain historical evidence.

## First-import opening retirement correction

R2C-C14's deterministic regression reproduces the hosted error after selecting first-import opening
work and accepting Pause before its LiveOnly capability write. The original test fails with the
same `persistent_journal_first_import_inactive` probe error. The execution lease is correctly
revoked; returning that normal retirement as an error from the selected opening aborts the
foreground pre-enumeration poll. The cause is opening-result ownership, not a missing UI delay.

`journal_baseline/opening.rs` now owns probe admission, capability/baseline publication and a typed
`Recorded`/`Retired` result. A revoked first-import lease exits before source/broker access or
after an already admitted probe. Write admission still acquires the existing SQLite transaction,
then the exact execution permit and root/generation/scan checks. Only this work's own inactive
publication error with its retained lease already revoked becomes `Retired`. Unrelated storage,
root-authority and broker errors remain errors. A successful commit remains `Recorded` even if
control arrived after permit admission. The scan facade and terminal/checkpoint owner are unchanged;
the next existing control check settles Pause, Cancel or Suspend.

The final focused journal-baseline namespace passes 18 tests in 6.58 seconds. Its 11 new boundary
tests cover Pause/Cancel/Suspend before capture and during supported/LiveOnly probes, no access
after retirement, rejected and already admitted SQLite writes, current supported/LiveOnly capture,
catalog-authority mismatch with a current execution, original error preservation and same-ID
execution replacement. The existing first-import filter passes 43 tests in 24.70 seconds, including
publication control, observer retirement, checkpoint restoration and explicit continuation. The
two filters overlap; their counts are not distinct test totals. An intermediate extraction compile
failed because supported publication returns a baseline receipt; the explicit unit-result mapping
corrected that type mismatch before these passing runs.

Physical ownership counts include blank lines. `journal_baseline.rs` changes from 546 production /
321 inline-test / 0 dedicated-test lines to 408 / 325 / 0. The new opening owner has 185 production,
0 inline-test and 457 dedicated-test lines. The existing production coordinator retains 3968
production and 10490 inline-test lines; only its opening-result mapping changes. This extraction
leaves its child owners' 699 dedicated-test lines unchanged and does not close the separately
tracked production-runtime decomposition debt.

Independent review checked the original hosted failure, deterministic red test, final opening
tests and the implementation delta without finding a blocking issue. It specifically retained the
need for production native validation: Rust test builds bypass the actual first-import preflight
poll, so focused tests alone cannot accept the connected Windows path. Source/format evidence and
original failed attempts remain under `.build/r2c-first-import-control-20260925`. All source access
in these tests uses generated temporary fixtures. No retained media, schema, bridge, dependency,
deadline or workload changed. The following accumulated verification completes the connected
native and local quality duties that were pending at the focused checkpoint.

### Connected verification on the corrected source

`724a83ecf6ceb45fb27fcea29d6c0ab8dc8b8d34` contains the correction. The four product/test SHA-256
records in `product-source.json` remain unchanged throughout verification and match the commit.
Formatting changes no Dart files; lint passes in 158021 ms with warnings denied. One complete
local Daily passes in 2035622 ms, from 11:56:08 to 12:30:03 UTC on 2026-09-25:

- Rust: 1581 passed and 19 explicitly ignored; the broker binary adds three passing tests.
- Flutter: all 98 test files pass.
- Windows Scan: all three connected cases pass, using the production Rust DLL. Run
  `d72fa1a99a6643f0af948c1c79a9ca03` exits zero in 80852 ms, with no cleanup failure. Its isolated
  storage remains retained because the runner has no owned recursive-cleanup proof.
- Windows accessibility: both test cases and all ten native UIA phases pass within the original
  deadlines, with the owned process and Job retired.
- The 16 asynchronous bridge contracts, matching bridge hashes and whitespace check pass.

The retained-import case generates 1024 PNG sources, accepts Pause from the Started listener,
observes the paused checkpoint without an error, restores that state in the widget tree, retains
usable navigation, then executes Cancel. A further widget remount proves Cancel remains durable;
source bytes remain unchanged through root removal. This case does not execute Continue or restart
the EXE. Those distinct client duties cannot be inferred from this pass.

[Hosted run 36133513545](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/36133513545)
belongs to head `724a83e`. Its Windows Scan, Flutter, accessibility, unsigned Windows and all five
synthetic workloads pass. Static/Rust and the aggregate subsequently complete successfully;
all ten required jobs and the aggregate pass. The three conditional release jobs remain skipped,
including protected signing and signed verification.
The unsigned artifact records merge commit `7987f5b917fcc7bc216b2fa8beea7fa696239e70`, whose tree
`178c3d4c99919ffbefb70f8feb571dfa7539c3bf` exactly equals the repair's tree. Its parents include
the repair head; no differing source is being accepted through an earlier build.

The canonical hosted unsigned command passes its three engine-free window cases, two actual
Debug-engine retirement cases, Release payload verification and catalog-free bridge smoke. The
payload has 19 files / 79564708 bytes; the engine process exits zero in 5068 ms and closes its Job
without cleanup failures. Only the text evidence artifact was downloaded. This current-source
hosted evidence satisfies that ADR 0026 gate without duplicating its build locally; it does not
prove Windows 11 Release interaction, signing, installed-service or retained-root acceptance.

No additional complete local Daily is required by the commit or these evidence-only updates.
The original hosted failure, deterministic red regression and intermediate compile error remain
preserved. The reported retained-library Retry/jump and recovery cost remain separate open duties.

## Prepared retained middle-timeline observation

The current product remains `724a83e`; documentation head `08c0c86` changes no product tree.
The isolated preparation uses the existing catalog's 30657 `local-primary` and 48624
`cloud-primary` published records, 79281 in total. A read-only SQLite transaction supplies one
backup snapshot. The copy independently passes the exact registered-root allowlist, published
baseline, active-generation and no-pending/leased/retry/foreground-scan checks. Published membership
and metadata retain SHA-256 `8cc43a5f7fc602cee5ba32d01828aba7efed079a00196325fe8c669d3f57110a`.
The original main database and WAL have unchanged presence, size and SHA-256 after preparation.
No source media was enumerated or opened, and no client started.

The copied catalog resets only external preview-cache references under the existing storage
isolation policy; empty-path failed classifications remain. This creates a cold isolated cache
without altering original catalog/cache or published membership. It is a current-source
observation, not a reconstruction of the original warm-cache session or its missing error frames.

Six focused Python checks and nine Dart admission checks pass; Dart analysis reports no issues.
The guarded Debug entry builds in 24.6 seconds and calls production startup with isolated storage
and in-memory preferences. The runtime consists of 18 files / 175624124 bytes; its source trees,
artifact hashes and eight helper files are bound in the ignored `payload.json`. PowerShell parsing
and explicit missing-approval rejection pass without creating an admission or process. Native
window inventory is available; no Ame window exists. These helper checks do not rerun Daily or
establish a native browsing pass.

The first independent method review found seven launch blockers in snapshot revalidation, stale
pre-lock checks, final close/deadline checks, monitor terminal evidence, cleanup, recovery-start
rejection and partial entry receipts. Their corrections pass the single scoped static recheck.
The launch now checks the original database/WAL after both successful and failed lifetimes;
product/artifact/resource admission happens after acquiring the repository lock. Native input,
matching monitor results, original-catalog preservation and normal exit still need actual evidence.

Preparation is retained under `.build/r2c-retained-browse-20260925`, run
`639b186bcf1d45d8a75d4730e9a8143a`. No launch receipt exists. The pending approval covers only one
600-second Debug lifetime, selecting `cloud-primary`, a middle-date jump, upward scrolling,
reversal and settled observation. Its combined catalog registers both logical roots, so incidental
viewport/startup reads from either must be covered. No import, update command, viewer, settings,
source mutation or placeholder hydration is admitted. The original 4 GiB launch floor, 2 GiB
client ceiling/system reserve and six-second normal-close bound remain. Recovery, resource failure
or loss of window identity stops the run. The user-reported issue stays open pending this evidence.

### Approved observation and retained result

The approved lifetime runs from 13:34:07 UTC on 2026-09-25 and lasts 385651 ms, including cleanup.
Initial window capture showed another foreground surface; reselecting and activating the exact
returned Ame window restored the correct capture before any positional input. The isolated catalog
opens with 79281 records, and selecting `cloud-primary` shows 48624. A middle-rail click at 13:36:02
initially shows loading; the 13:36:23 observation shows decoded images without intervening input.
That frame also has a background-update indicator; it does not reproduce the report's already-idle
precondition for the entire first scroll.

Upward input of -1320 shows two Retry controls in the immediate frame. The later no-input
observation shows pixels at those positions and the update indication cleared. A further -6000
input then shows Retry across visible rows; 18 controls remain in the observation 43.4 seconds
later. The +7320 reversal returns the original middle-region arrangement, still stable 44.3
seconds later. No unsolicited position jump is observed in these sampled frames. This is direct
Retry feedback evidence, not proof that every such control shares one new functional defect, and
does not establish continuous frame stability or reproduction of the original jump.

The complete stdout contains 162 logged preview outcomes, all ready; stderr is empty. The existing
queue logger omits fast implicit terminal requests and does not attach a returned failed asset's
issue code, so those 162 lines cannot rule out transient failure. Read-only, row-capped comparison
finds the same 110 persisted failed identities and classifications before and after: 104
`image_format_unsupported` and six `image_decode_invalid`, all in `cloud-primary`. There are no new
or cleared persisted failures. The exact visible Retry controls were not bound to location IDs,
so this does not identify which of them corresponded to those existing classifications.

Published count remains 79281, but the combined membership/metadata digest changes. Among the 162
logged identities, 121 `cloud-primary` rows change only source revision and source generation; the
sample has no capture-date, dimension, size or modification-time change. This explains neither the
full historical recovery cost nor every unsampled metadata difference. The original full-root
comparison reached its unchanged 350 ms deadline; the replacement selects logged identities in
batches of at most 64 under the same deadline. Both roots finish synchronized with zero pending,
retry or gap counters, and no recovery-start/finish marker occurs. Original database/WAL presence,
size and SHA-256 remain unchanged. This comparison reads derived catalogs, not original media.

Alt-F4 closes the application normally in 602.6654 ms; application and parent exit zero and all
owned processes retire. The memory observer records 1444 samples, peak working set 414916608
bytes, peak commit 569671680 bytes and minimum system availability 7387635712 bytes, with no
failure or cleanup issue. The aggregate launcher nevertheless fails when reading the already-exited
parent's `ExitTime` without having retained that `Process` object's handle. Its original failure
receipt remains. A two-process, media-free comparison reproduces the same null-value error without
the handle and obtains a valid exit timestamp with it. The ignored launcher now retains the parent
handle before waiting; the executed version and matching hash are preserved as `launch-used.ps1`.
No Ame replay follows this observer correction. The completed stopwatch bound supports parent
retirement within 600 seconds, but does not rewrite the failed aggregate as a passing invocation.

The ignored run retains `input.json`, full logs, original completion and memory receipts and
`result-analysis.json`. It advances the observed symptom and its evidence; it does not close C05,
the historical incident, Release acceptance or the complete R2c goal. Further diagnosis must separate
existing failed media from transient admission/reconciliation or image-rendering feedback before
changing product behavior. No extra full quality run is warranted by this observation alone.

## Changed-source preview admission during existing path work

A generated 32-by-24 PNG establishes a separate, deterministic command-boundary defect. After a
same-size rewrite that restores modification time, an existing path reconciliation entry retains
its full row and authority, but the old preview request returns
`source_revision_changed_during_scan` instead of retiring. The original preview and catalog row
remain intact. This outcome can project Retry for a pending thumbnail; a retained ready artifact
already has independent frontend protection. The red test does not identify the locations behind
the earlier native Retry frames or reproduce the reported position jump.

The source-reconciliation port now separates superseded request, existing path work, granted lease
and unavailable lease outcomes. The application retires obsolete work and changed-source requests
already owned by path reconciliation. The adapter preserves its exact root, active scan, source
generation/revision, path, capacity and guarded lease checks, using the same bounded reads and
transaction. An unavailable claim still rolls back its insert and preserves the changed-source
error; the prior missing-source retirement rule is unchanged. Database, source-open and
root-unavailability failures are not converted to success. No schema,
bridge, dependency, UI retry policy or source-media behavior changes.

The 12-test source-reconciliation module passes. New cases retain both pending and executing path
work without duplication, let the original owner publish the current generation, and verify the
replacement pixels and unchanged generated source bytes. A pending-preview case first invalidates
derived work through ordinary reconciliation, then exercises the same changed-source boundary.
The rejected-claim case proves rollback and original-error preservation; existing stale-admission,
missing/recreated source, locked-source and unavailable-root cases remain passing. Both existing
frontend queue and failure-projection test files pass, including superseded-result retirement and
actionable real source failure. The earlier red result and an intermediate missing test import
compile failure remain under `.build/r2c-preview-admission-20260925` alongside the focused results.

Independent review identifies one test-evidence gap: the executing-lease case initially uses a
historical synthetic timestamp while preview reconciliation uses current time. The corrected
case uses current timestamps for enqueue, lease and completion, and asserts unexpired ownership
before materialization and before original-owner completion. Its default 30-second lease is not
extended. All three affected cases pass, and the targeted static recheck closes that finding;
no production change follows the review.

At this focused checkpoint the existing port owner has 21 production lines, application owner 144
and SQLite owner 128; none has inline tests. Their dedicated test file has 723 lines. The 1080-line
ports facade changes only its typed export, adding no workflow responsibility. Frozen-source
lint passes in 132252 ms, with zero formatting changes and no Clippy/analyzer warnings.
The complete Daily starts at 14:20:04 UTC on 2026-09-25 and is intentionally stopped at 14:38:02
following the direction to group all three reported symptoms before full regression. At interruption,
1269 Rust cases have reported passing with no assertion failure; the Rust suite is incomplete,
and Flutter/full Windows integrations have not begun. The explicit cancellation produces test
exit `0xffffffff` and parent exit one; this is an interrupted gate, not a passing or spontaneously
failing invocation. Exact PID/creation/ancestry checks precede termination of only its Rust test
process. Cargo and the Daily parent retire; no owned descendant remains and the repository lock
can be acquired and released. Original logs, source hashes, cancellation and retirement receipts
remain in the same ignored evidence directory. No unchanged complete replay follows.

Required complete gates remain pending at the combined batch boundary. This functional correction does not accept C05,
retained-library recovery performance, Release interaction or the complete R2c stage. The one
authorized retained-root lifetime has been consumed; no further real media is accessed here.

## Deferred geometry must retain later scrolling

Generated 4000-item manifest cases expose a second owning defect. A deferred geometry replacement
captures the center-card anchor, then an actual 480-pixel wheel moves the viewport before its next
frame. The original callback restores `location-3003` instead of retaining the later `location-2988`.
The same failure occurs in either direction when an explicit transition is created before the
wheel but has not reached its first build. A connected AmeApp case also proves that the first
correct reflow is insufficient: a second dimension batch can reuse the old screen recovery anchor
and move the newly visible `location-32` outside the viewport.

The typed reflow owner captures the native scroll object and offset. Both scheduling and delayed
application validate this origin; movement causes recapture from the displayed layout snapshot.
The screen likewise rejects an obsolete transition when freezing the next recovery epoch and
records the actual position on completion. Query/revision/location checks, newer explicit intent,
framework scrolling, atomic sliver corrections, initial attachment and the frozen recovery range
remain independent. This does not reset recovery epochs after every reflow or promote newly
exposed prefetch rows.

The final targeted run passes 18 resize/reflow cases and 43 connected-screen cases, including
wheel input before/after scheduling, both directions, two dimension batches, sidebar resize,
layout/density changes, initial attachment and offscreen recovery deferral. The independent review
identified both missing consumer boundaries; their red cases and passing corrections are retained
under `.build/r2c-preview-admission-20260925` as `reflow-red.log`, `reflow-epoch-red.log`,
`reflow-origin-red.log` and `reflow-origin-focused.log`. The last scoped recheck closes those
findings. This proves the generated race correction, not continuous native-frame stability or
attribution of the historical retained-client jump.

Production/inline-test/dedicated-test sizes are 238/0/0 lines for the new reflow owner,
2027/0/1491 for wall/resize coverage and 2136/0/4108 for screen/connected coverage. The geometry
algorithms moved into the owner; the wall remains a facade around existing layout and sliver
responsibilities. Remaining physical decomposition is not represented as completed by this repair.

## JPEG inspection reads only header evidence

The pinned image 0.25.10 JPEG constructor reads the entire compressed source before decoding
headers. A generated valid 1024-by-1024 JPEG with small headers contains 1,957,816 bytes; the
pre-correction adapter reads 1,966,008 bytes including format detection. This is a reproduced
per-file I/O cost, not proof that it accounts for the whole 1088.599-second retained-root recovery.

The narrow JPEG adapter now uses the same locked zune-jpeg 0.5.15 and zune-core 0.5.1 through their
buffered-reader API. Only the root dependency edges change in Cargo.lock. It consumes headers
through the first scan marker, preserves EXIF that appears after the frame header, and retains
the same orientation conversion, metadata extractor and engine identities. A bounded reader
preserves the original I/O error even when a decoder helper suppresses it, and refuses publication
on resource exhaustion. Other formats, source opening/revalidation and preview pixel decoding
retain their existing owners. No real source was read or modified by these generated checks.

The first correction reduces the same JPEG to 24,576 bytes, but independent review finds its
budget outside the buffer. A generated 8,947-byte JPEG with 2048 empty APP3 segments reproduces
9,873,275 bytes of reads because repeated seeks discard prefetch. The correction places the actual
source reader inside the budget, subtracts the format probe's possible prefetch, and uses buffered
relative seeks. The final large-JPEG result is 16,384 bytes, about 99.17% fewer than the original
adapter. Eleven adapter cases pass, covering existing orientation/source preservation, late EXIF
with a 1925 capture date, CMYK, all seven admitted formats, truncated headers, retryable access
failures, repeated skipped segments and a budget that reads exactly 64 bytes from the source.
The scoped recheck closes this finding. This proves a read bound, not total process memory usage.
The connected generated application checks pass eight scan/media cases, three metadata-inventory
terminal-media cases and 12 source-reconciliation cases. The original red and focused results are
retained as `jpeg-header-red.log`, `jpeg-header-focused.log`, `jpeg-budget-red.log`,
`jpeg-budget-focused.log` and `jpeg-application-focused.log` in
the same ignored evidence directory. No elapsed whole-root speedup is claimed.

The inspection facade has 237 production lines and 134 inline test lines; the JPEG owner has
195 production lines, with 301 dedicated test lines. Applicable complete gates remain deferred
to this combined batch's frozen-source boundary. These focused results do not accept Release,
retained-library performance, C05 or the remaining R2c variants.

The combined Daily's first invocation stops in Dart analysis after 163003 ms: four initializing-
formal suggestions and one null-aware-expression suggestion in the new reflow code are fatal
under repository policy. Its guardrails, formatting and Rust Clippy have passed; full Rust,
Flutter and Windows suites have not started. The five syntax-only corrections pass targeted
fatal-info/fatal-warning analysis. Preserve the original source receipt and failed invocation;
the revised source preserves behavior and tests. The second invocation passes lint and analysis
and enters Rust tests, then is intentionally interrupted after 432540 ms to perform actual native
client operation first. The verified owned test child is stopped; its cargo and Daily ancestors
retire, and the tool lock is released. This is not a complete gate pass. Its source, result and
interruption receipts are retained as `combined-product-source-v2.json`,
`combined-daily-v2-result.json` and `combined-daily-v2-cancellation.json`. Complete testing stays
deferred until the planned native experience checkpoint has an explicit outcome.

## Manual update loses the native browsing position

The frozen three-correction Debug client is operated through Computer Use before complete gates.
Run `ed24825fed214b429b46511b9112f5a3` uses a fresh isolated catalog and the unchanged generated
10000-image, 10921494393-byte mixed-size/historical corpus. Actual picker import reaches 10000;
a middle-rail jump loads without a subsequent wheel event. Upward and reversed scrolling settle
without Retry or uncommanded movement in the observed 2015 and 2011 regions. Opening a 3840-by-2160
image and returning preserves the visible tile arrangement in the first observed return frame.
These point observations are not continuous-frame proof that no transient flash occurred.

The same lifetime **fails** during a real menu-driven update. With 10000 images unchanged, the
viewport shows 2011-02-02 after upward scrolling during the update. Without another navigation
input, the update completes and the viewport instead shows 2025-02-01; the rail moves from near
its lower end toward its upper end. The displaced position persists after thumbnails settle.
This is a current generated-client defect, not attribution of the original retained-root incident.

The complete lifetime is 622853 ms with normal exit zero, 596.022 ms from close input to exit,
closed owned processes/Job/handles and no cleanup failure. Peak working set is 624.52 MiB, kernel
peak commit 633.98 MiB and minimum system availability 6.61 GiB. Before/after full source identity,
date and hash checks pass, as does exact closed membership of 10000. These safety and process
results do not change the failed experience verdict. Original returned native frames and
`native-observations-final.json`, application logs, `lifetime-1.json`, `closed-1.json` and both source
receipts retain the result under the run's ignored integration-storage directory. Preparation and
the 44-file source/payload/helper binding remain in `.build/r2c-batch-native-20260926`.

Two new controlled manual-update regressions fail with a null gallery anchor, while the five
existing refresh cases pass. The manual-update provider called an unanchored current-query refresh,
bypassing the registered gallery projection used by primary committed publication. The correction
connects that route to the existing projection and typed read authority. Replaced read success or
failure retires before publication; the committed obligation then follows the latest position.
Current query failure feedback and explicit display-only retry remain intact. No new retry policy,
presentation offset patch, source mutation, schema or bridge is introduced.

The focused result passes eight manual-update cases, three primary-publication cases, six
committed-refresh ownership cases and four connected position cases. The viewport has 1380
production lines and no inline tests; the dedicated manual-update file has 455 lines. Independent
review finds that the new route omits retirement of old page/window authority. A pending old page
can still publish into the replaced window or attach a stale error. Four held next/previous-page
success/error regressions reproduce this; the correction supersedes page/window authority before
the committed projection and retains the original generation across gesture waiting. The combined
focused checkpoint passes 22 cases across bidirectional paging, primary and manual refresh, and
the connected screen. The viewport now has 1394 production lines and no inline tests. The corrected
native lifetime and full testing remain open. `retired-page-red.log` and
`retired-page-focused.log` retain this boundary. The earlier red
and focused results are retained as `manual-update-red.log` and `manual-update-focused.log`.

### Corrected generated native checkpoint

Run `e907837b0dad4cab916f4787522d0f07` binds 36 changed product/test files and a 65-file
source/helper/built-payload closure to the same base head. Its actual Debug window imports the
unchanged 10000-image mixed-size/historical corpus through the native picker. The observed
completion interval is 60.008 seconds, within the original 300-second limit; this is a bound from
input to the observed completion frame, not an exact internal scan duration. An obsolete picker
accessibility index first fails; fresh screenshot-backed input then selects the verified directory.

Actual middle-rail clicks show an initially gray or loading wall. Upward wheel input and reversed
downward scrolling continue into subsequent dates in the observed 2016/2015 and 2012 regions.
Visible gray tiles finish loading without Retry. The selected 3840-by-2160 viewer opens and Escape
returns to the same tile group. These observations do not prove that input arrived while the
database future itself was pending, or rule out an unobserved transient frame; the retained-library
false-bottom and already-idle Retry/jump reports remain open.

The menu-driven update starts at 18:00:17.425 UTC, upward scrolling at 18:00:28.924 reaches
2012-04-02, and 10000/10000 verification is visible at 18:00:53.276. Without additional navigation,
the completed update is observed by 18:01:27.739 at the same date, tile group and rail region.
The previous large jump toward 2025 is absent. Compared point frames have about four pixels of
vertical adjustment; exact pixel/continuous-frame stability is not claimed. This closes the
selected generated large-jump recurrence check, not the original idle incident or all C05 exits.

The complete lifetime is 461418 ms, with normal exit zero and 738.437 ms from the recorded native
close-command boundary to exit. Owned processes, Job and handles close with no cleanup failure.
Peak working set is 663.777 MiB, kernel peak commit 691.067 MiB and minimum system availability is
6.441 GiB. Full before/after source identity, dates and hashes pass for all 10000 files and
10921494393 bytes. Closed catalog membership is exactly 10000; the cache contains 280 ready
previews using 6021596 bytes. The run directory retains all source, catalog, resource and lifetime
receipts; helper/binding records are in `.build/r2c-batch-native-verified-20260926`.

This native run contains no old exhausted retained-root task. Its successful import/update cannot
establish that the user's existing task has recovered. That old-task boundary currently has
production generated evidence only. Remaining functional diagnosis and complete batch gates stay
open; no new real-source operation or automatic library reset is inferred from this checkpoint.

## Combined repair batch verification

The corrected complete `quality_verify_daily.ps1` invocation exits zero on the frozen repair
batch over `847c1cb`. Its 46 changed handwritten-source/test/lock files match the SHA-256 manifest
before and after execution; the six accompanying documentation files are outside that code
binding. The private prepend constructor's syntax-only correction passes its four existing owner
cases and fatal-info analysis; its production file is now 79 lines with no inline tests.

| Check | Current result |
| --- | --- |
| Format, guardrails, Clippy and Dart analysis | Passed, including warnings-denied Clippy and fatal-info Dart analysis |
| Rust | 1608 library cases and three broker-binary cases passed; zero failed; 19 explicitly ignored library cases retain their separate invocation requirements |
| Flutter | All 102 test files and 793 cases passed |
| Windows scan integration | Three cases passed in run `27a3db2798d54c9486765f0287b78181`; process exit zero after 60711 ms, no run or cleanup failure |
| Windows accessibility | Both cases and the original ten native phases passed; primary process exited, owned Job closed and no cleanup failure |
| Bridge and whitespace | All 16 asynchronous contracts, matching generated content hashes and tracked-diff whitespace passed |
| Unsigned Windows | Fresh x64 Release application and broker, all three window lifecycle and both real-engine retirement cases, payload identity and the catalog-free Release bridge smoke passed |

The unchanged mixed-load connection controls and production case pass inside this complete Rust
run. Their respective visible P95 values are 158, 100 and 93 ms for the original 25 samples;
the complete tests also enforce the retained recovery and publication assertions. These current
passes do not supply missing attribution for historical C01/C02 failures. Their failed evidence,
separate performance/authorization-bound cases and final candidate obligations remain retained.

The ignored evidence directory `.build/r2c-batch-native-verified-20260926` retains the initial
lint failure, corrected transcript, partial captured native output, focused constructor result and
final source manifest. The native output capture is explicitly partial; the Windows scan's own
stdout/stderr and completion receipt provide its full process evidence. The selected generated
and retained-library manual native observations above remain the functional evidence; automation
does not close the exact original non-top Retry, database-pending false-bottom, old retained-task
recovery or complete retained-root update-cost obligations.

The unsigned gate exits zero at 21:10:12.775 UTC. Its 46 bound source files still match the same
manifest. The real-engine process exits zero, retires its owned Job in 3151 ms and reports no
cleanup failure. `build/quality-unsigned-windows/evidence.json` retains payload/engine identity
and exact native receipts; `combined-unsigned-windows.log` retains the canonical invocation.
The signing, installed-service and remaining Release interaction requirements are separate from
this unsigned build and catalog-free smoke result.

The verified product source is committed as `704c8ae` (JPEG inspection), `43a46c4` (terminal-scope
and exhausted publication admission), `35a2497` (preview reconciliation ownership) and `1704228`
(gallery position/publication ownership). Those commits contain the same 46 bound code/test/lock
files and the related accepted architecture amendments. Commit creation changes none of the
tested product bytes and does not require another complete gate. The remaining document commit
records evidence and open duties without declaring R2c or the original incidents accepted.
