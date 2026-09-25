# Retained-library runtime observations

Observed runtime baseline: `dcce10e3ce7f2f62a61a82adcdede64c1608f501`, 2026-09-25.
Correction: `0677ba67020142d50e4b28cf7f3ce07e44fec924`, verified 2026-09-25.
Status: startup recovery completed; paging correction passes focused, lint, complete local Daily,
fresh unsigned Windows and hosted checks. Recovery cost, the exact retained-client incident and
remaining R2c client acceptance remain open.
The subsequent documentation-only head has a new hosted first-import pause failure, retained below;
the earlier passing correction gates do not make that later run green.

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

The reported sequence is a successful middle-timeline seek, upward scrolling, partial Retry
preview feedback, then an unsolicited jump toward an earlier timeline position, specifically
not the top. The report explicitly confirms synchronization had already finished. Treat this as
an unresolved C05 functional browsing incident. The retained terminal tail
contains successful preview materializations and synchronized roots, but lacks the error and
navigation frames needed to establish the precise cause. Its lack of a Retry event is not a
negative reproduction result.

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
deadline or workload changed. Native, complete Daily, unsigned and new hosted results remain pending
at this focused checkpoint.
