# C05 browsing diagnosis

Status: preview-demand, capture-time publication and presentation corrections have focused evidence;
selected native browsing and read-only presentation checks pass. Overall workload replay failures,
transient deletion errors and final acceptance remain open.

## First diagnosis: source and outcome

The 2026-09-10 diagnosis starts from `507db08f897839aeafb2dc011ef7332ad50fadfa` on
`codex/r2c`, after the approved 120-minute supplement. Product source is unchanged at that checkpoint.
The [execution plan](../plans/r2c-closeout.md#approved-browsing-continuation) owns the scope and
30-active-minute unknown-cause stop. Earlier [native observations](r2c-live-gap-recovery.md#c05-browsing-findings--unresolved)
remain valid observations; the controlled widget results below do not disprove them.

| Observation | Current evidence | Disposition |
| --- | --- | --- |
| Entire wall blank after a timeline click until another scroll | Preserved original capture; controlled clicks and reconstructed native top/interior/bottom checks materialize current pixels without wheel input | Native functional checks pass for selected variants; complete replay and original-screenshot causality remain distinct |
| Persistent gray cards without retry after deletion/jump | Preview-demand and recovered-date publication each have boundary regressions; the latter reproduces the native ten-item mismatch and gray failure | Causal correction has native addition/navigation and removal/navigation evidence below; no claim that all screenshots share one cause |
| Transient whole-wall thumbnail errors after deletion | Reported native observation; retained logs do not identify all visible requests at that instant | Unresolved; do not infer harmlessness or decode failure |
| Time rail disappears during publication | Deterministic first-frame revision-gap regression passes after correction; native refresh retains the rail in all checked frames | Focused and native presentation checks pass; native refresh is not a forced revision-gap proof; no proved connection to persistent blank/gray |
| Layout flashes after closing the viewer | Hidden gallery width changes from 940 to 1020 logical pixels at a fixed 1280 by 800 viewport; fixed-width reservation and frame regressions below | Focused resize checks and 96 native open/return frames pass; Release acceptance remains separate; no proved connection to persistent blank/gray |

## Controlled boundary evidence

The fixture uses the actual screen, viewport, layout and navigation owners with a controlled
2012-item catalog, twelve mixed aspect ratios and two date groups in 2026 and 2012. It is an
interleaving/geometry fixture, not a replacement for the frozen 10000-image native workload.
Its preview adapter deliberately never completes: it cannot verify pixels, preview recovery,
decoder behavior, actionable failures, or cold/warm preview convergence.

The final four-test run has two passing navigation cases and two failing presentation cases.
Direct navigation covers both ends and two interior positions without a wheel event. The
interleaving case publishes revision two, delays its manifest, requests ordinal 1730 and verifies
that the target is visible after manifest completion. Both cases also require visible location
slots to be real photo tiles rather than unloaded detail placeholders.

Earlier assertions in the interleaving fixture were invalid and remain in the raw evidence:
the inner rotated Material Slider reverses its value, so 0.86 selected near the top. Correcting
that input to 0.14 issued the expected ordinal-1730 request. A subsequent assertion incorrectly
required the retained window to begin after 1500; normal previous-page prefetch moved its start
to 1230. The final assertion inspects the visible target itself and passes. Neither earlier
failure establishes lost navigation intent, blank browsing or a product regression.

The viewer case catches the width change on the first hidden frame. The screen removes its
80-pixel timeline child while the gallery remains laid out inside an IndexedStack. This proves
unnecessary hidden reflow; it does not capture the exact user-observed return frame or establish
a source-size change. The revision case catches an absent slider on the first publication frame:
the screen rejects the previous revision's layout snapshot, while the time-navigation owner clears
its stable metrics and renders no rail until replacement geometry arrives. Keeping stale geometry
without identity proof is not an admitted correction.

Read-only preview inspection also finds possible demand re-admission questions after superseded
completion and explicit root-authority restoration. Neither was reproduced in the bulk-delete
workflow. The restoration caller belongs to explicit root scan completion, not ordinary live
removal. No speculative queue retry, source-guard relaxation or cache invalidation is implemented.

## Retained artifacts and verification limits

Final raw output is in ignored
`build/diagnostics/r2c-p2-raw-f9b3cda5730642229d12537ad8a8898d/`:

- `stdout.bin`: SHA-256 `53F166B9D2E98B0F485CC1408AD38ED3C73401EDA61B2356DFC476D47CEFF509`;
- `outcome.json`: SHA-256 `EA7AAE201B4D0BF583365CDCA749D95E6731EB9F1219F0B75BCB46A077B70080`;
- owned capture duration 16151 ms, exit one, process retired, Job closed, no cleanup failure.

Earlier outputs remain in the same diagnostics directory under raw-run suffixes
`b3dcf8aa99364cd3902968f3b8abd861`, `28e4257427614b6392a5432a18cac30c`,
`32f3def9d37741c6b15e65ff96f8c1bf`, `f73ce817ad0d4d8389f48525a2b0b3ee` and
`9559df8828fc4dec8169b913f8e4f683`. They retain evolving assertions and failures rather than
forming five independent product reproductions.

The exact diagnostic source is preserved as ignored
`build/diagnostics/r2c_c05_browsing_continuity_test.dart`, SHA-256
`BE32704F7BBD0A9E5F28BC24138F2571E7EAF06460C93CBA74CBF4EE7F1D6C83`.
It originally ran at `test/features/library/presentation/library_browsing_continuity_test.dart`;
its relative support import requires that original location. It is retained outside the regular
test inventory because its two unresolved assertions intentionally fail, not disabled or marked
passing. There are no C05 production changes or committed test exclusions.

At this first checkpoint no C05 native client, source mutation, decoder run, Debug rebuild, lint or full Daily is claimed.
Only synthetic in-memory assets were used in the widget runs. Frozen source files and all retained
C04 catalogs remain untouched. The stopped C04 client lifetimes are not reused or extended.

## Stopping decision

Conservatively charge the full 30-minute diagnosis allowance, including five delegated inspection
minutes, setup corrections, source inspection and result analysis. The persistent native blank/gray
cause remains unknown at that boundary. Stop expansion before product changes and retain the two
lower-impact presentation defects separately. Counts and passing controlled navigation do not
establish usable native browsing. A replacement native-evidence method is proposed in the execution
plan; it does not become authorized merely because repair time remains.

The first independent evidence review uses two active minutes and confirms this limited conclusion.
The width assertion stops at hidden frame zero before the return assertions execute; the rail
assertion also stops at its first failing frame. Neither test proves the duration of the reported
native symptom. A final scoped documentation check is recorded in the cycle accounting.

## Renewed native observation and preview-demand correction

The renewed functional instruction admits the execution plan's 76-minute pivot from `4b02218`.
The owned Debug entry copies the production bootstrap and delegates to real Rust catalog,
manifest and preview ports, adding bounded request and visible-tile observations. It is an
instrumented entry, not an unchanged performance baseline. Build capture
`r2c-p2-raw-8403e029003c4a3880f0b605b701b765` passes in 39340 ms. A read-only SQLite backup of
the closed C04 catalog creates a separate C05 catalog; previous catalogs and caches are preserved.
A separate generated twelve-image stimulus is prepared but never imported.

The one admitted session runs from 04:27:26.907 to its fixed 04:57:26.907 UTC deadline.
Computer Use can inspect the window but rejects every click as simultaneous input, including
fresh screenshot, fresh accessibility element and one runtime reset. The user confirms no window
operation. The tool's underlying cause is unproved; no user interference or product-input failure
is inferred. No root removal, stimulus import, timeline jump or bulk change executes. Startup
records 50 real preview completions and twenty visible tiles with image pixels; this does not
exercise the incident sequence.

Run `c59c2edaf6a64bdfaa32443f440829bf` ends at the parent's 1800-second deadline, after 1800518 ms,
with its process retired, Job closed and no cleanup failure. This is a blocked observation,
not a failed repaired replay. Memory monitoring retains 6782 samples, sampled peak private bytes
333885440, kernel peak commit 350994432 and minimum available system memory 5483483136; its
monitor reports no resource failure. The second lifetime cannot be started after the shared
deadline. Client receipts, trace, preparation provenance and session lock remain in the ignored
C05 fixture identified by `build/diagnostics/r2c-c05-client-root.txt`.

Remaining diagnostic work uses finite preview completions at the application boundary. A real
timeline publication first trims pending preview work and replaces `state.assets`; visible demand
arrives after layout. A second navigation or query can restore the same visible assets before
that demand is published. A pending request may already have been cancelled, or an active result
may have been correctly rejected while its asset was absent. The coordinator nevertheless skips
an identical demand description, so the current tile remains pending without a queued request.
These production call sites establish reachability, not causality for the captured native frame.

Two focused tests fail on the prior source: cancelled pending work never starts again, and a
rejected completion prevents unchanged visible demand from loading again. Five existing tests
pass. Capture `r2c-p2-raw-3941f3f13c724ad7903af29fc55207c9` preserves both failures, exit one,
9322 ms and successful owned retirement. The correction removes coordinator-level description
deduplication. Each actual demand publication reaches the existing queue, which alone knows
whether compatible pending/active work or a stored result satisfies it. No timer, forced retry,
cache reset, concurrency increase or source/publication-guard weakening is added.

The final focused capture `r2c-p2-raw-cdc61fc7e61f450282bbb4d49051a6c2` passes all 100 tests:
controller 56, coordinator 10, queue 29 and store five, in 29534 ms, with empty stderr and clean
retirement. New negative cases submit unchanged demand twenty times and retain one active/pending
decode, no repeated ready decode, and one notification for decode/root-unavailable failure.
Explicit Retry still reaches ready. Existing queue cases preserve source-generation rejection
and concurrency bounds. Independent causal inspection and code review each take two active
minutes and find no blocking issue in this narrow correction.

Full lint capture `r2c-p2-raw-1f14669dc83949f899784020e4cf3327` passes in 152668 ms, including
format checking, repository guardrails, Clippy and Dart analysis. Its process retires and Job
closes with no cleanup failure; stderr contains only Cargo's successful completion message.
The corrected normal `lib/main.dart` Debug entry also builds successfully in 35196 ms, capture
`r2c-p2-raw-a319ba37ec8140508e6b785fe64e17bb`, with empty stderr and clean retirement. Its Dart
kernel SHA-256 is `42F3BE3E9B45E0E54D9FA9674E2A5438CF753EB97F9A1D85F6EC2324F424CF92`.
This replaces the scratch diagnostic executable payload; no new application lifetime is launched.

After native retirement, a full read-only source check verifies all 10000 frozen files against
their exact path roster, SHA-256, file identity, size, creation/modification times and date
distribution. The retained C04 512 files and newly prepared C05 twelve files also match their
identity/hash/size/modification-time ledgers. The C05 copied catalog still publishes exactly
10000 and 512, has SQLite integrity `ok` and no foreign-key violation. Evidence is the new
fixture's `c05-final-integrity.json`; no old evidence is overwritten. This proves fixture
preservation and structural integrity, not an application FULL reopen or successful navigation.

The changed production coordinator shrinks from 277 to 254 lines, with zero inline-test lines;
its dedicated tests grow from 231 to 385 lines. Queue (700 production/1061 dedicated-test lines)
and store (248/173) retain their existing ownership and are not extended. There is no dependency,
schema, bridge, source-media or licensing change. The instrumented Debug artifact predates the
repair. A repaired native replay, full Daily and complete browsing acceptance remain unverified;
the original blank wall, transient whole-wall failures, rail and viewer observations remain open.

## Final hosted source checkpoint

Run [34443329392](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34443329392)
targets product commit `8cf120ffca611b8d96d10322f3daf8a7c8286f2d` and finishes with failure.
Nine verification jobs pass and Windows Accessibility fails; the aggregate gate also fails.
Three signing/release jobs are conditionally skipped, not accepted external evidence.

The complete Flutter log proves 71 files and 570 passing tests. The Rust main suite reports
1503 passed, zero failed and 19 ignored, followed by three passing documentation tests; nested
child-process test summaries are not counted again. The production mixed-load case passes and
reports 25 P0 samples with P95 442 ms. Its separate connection-lifetime control reports PerPoll
P95 1230 ms and PerEpoch P95 477 ms; the diagnostic PerPoll arm is not the production latency gate.
These hosted results do not explain or erase the earlier failed complete local Daily.

All five synthetic jobs execute exactly one selected test with zero failures and zero ignored:
high-resolution JPEG, 10000-image scan, seven formats, million journal records and 50000-identity
publication. Windows Scan and the unsigned Windows Release job pass. These workloads are distinct
from the frozen mixed-size, historical-date client and do not prove its native browsing path.

Accessibility repeats C02's `application-ready` parent deadline failure. The probe records 126
elements, internal completion in 7570 ms and evidence writing in 71 ms, but the parent records
8018 ms against its unchanged 8000 ms deadline. Loading the UIA client accounts for 4928 ms of the
probe's stage timing. Native process exit and owned Job closure are verified with no cleanup
failure. This remains a failed required gate, not a successful probe with an excused deadline,
proof of ordinary input failure, or proof that the local Computer Use refusal shares its cause.
No workflow rerun, deadline change or new C01/C02 investigation follows.

Complete raw logs remain under ignored `build/diagnostics/r2c-c05-hosted-*-34443329392.log`.
The final run JSON SHA-256 is `919F431CFC3A54BA60CA26EAEAD138003CCCCE270D5A72774E542E8BD264FEF4`;
the Rust log is `E5741F891898EC8ED7F3304B7566001379D8F9C8DDBAE51ADD936F8D937711DA`, and the
accessibility log is `118563E0F646ADD8CA45E1A45F9B29D4B0D27CF695CE0DA7023833B9F07EF51B`.
The CLI initially refuses a completed job's log while its overall run is active; direct job-log
retrieval preserves the completed jobs. The running Rust log is unavailable until terminal state.
Waiting uses the same live run and existing 60-minute hosted limit, not a replacement execution.

The single final evidence recheck finds no actionable discrepancy and takes 2.3 active minutes.
Source safety, build/test evidence and the incomplete native replay remain accurately separated.
Current source has focused verification and partial hosted verification; candidate readiness,
usable native browsing and complete R2c acceptance remain blocked.

## Reconstructed native observation — 2026-09-22

The resumed checkout has lost the prior ignored fixture and raw logs. Original task outputs retain
the old `e86c97ad23904a85ba5d20e97306b2cb` failure: bottom navigation after addition leaves ten visible
pending images without preview requests, with start 1508 and 514 assets against 2012 total. Its full
source-integrity evidence is unavailable. This is retained failure evidence, not a current rerun.

Fresh generated sources reproduce the documented distribution with new physical identities:
10000 files, 9000 JPEG/1000 PNG, 12 dimensions, 72 textured templates, 201 historical months from
2010-01 through 2026-09, and 10,921,494,393 source bytes. Repeated-content limitations remain.
The generator's first launch used a wrong target-directory assumption and started no child;
the corrected existing executable passes in 133.752 seconds with 212,131,840 peak working-set bytes.
No real root or retained personal catalog is used. The Rust catalog boundary checks add two passing
tests for time-anchor/reverse-page and restored-query ordinal agreement; neither changes production
nor reproduces the old failure. The traced Debug client builds and its scoped analyzer passes.

Observation `2a84b240c90241eea26ed778acdc3b30` passes mixed-library jumps at 0, 0.5, 1 and 0.15,
then the 12-item baseline, bulk addition to 2012, and added-library jumps at 0.5 and 1. Pixel checks
require current preview identity, decoded render images and no visible unloaded slots. At the added
bottom, the catalog returns four items and a 500-item previous page: start 1508 plus 504 equals 2012,
with no observed disagreement against the current layout manifest. This does not explain the old
514-item window or prove intermittent behavior repaired.

The next top-endpoint click produces no catalog seek and times out at the unchanged 30-second
position deadline. Deletion is not executed. A focused test on the actual vertical Material control
confirms that the harness's exact rotated padding-edge point misses its render box; one logical
pixel inside delivers input. Correct the harness coordinate and verify the hit target before the
single admitted continuation. Do not change product navigation or call this a product correction.

The observation exits one after 218197 ms, with process exit, Job closure, synchronization shutdown,
and no cleanup failure. Its 814 memory samples report peak working set 713,465,856 bytes, sampled
private 890,515,456, kernel commit peak 1,003,577,344, and minimum available system memory
6,585,790,464. Resource limits pass. After exit, a full 53.815-second read verifies all 10000 frozen
files' roster, SHA-256, identities, sizes and creation/modification times, plus all 2012 generated
stimulus files. Local raw evidence is retained under ignored `.build/c05-fixture` and the isolated
build storage identified by its run/pointer files. The old failed run remains failed; C05 stays open.

## Causal continuation and transaction correction

The harness-corrected continuation `acd055d0c98a4b2cb92687bdd62cf447` reopens the generated-only
2012-item catalog and verifies delivered slider input. At 10982 ms the detail window first disagrees
with its same-revision manifest by ten positions. Its later bottom window starts at 1503 and contains
519 items, exceeding the 2012-item total by ten. Top navigation then fails the original 30-second
pixel deadline with ten visible gray items. Deletion is not reached. The process exits one after
43592 ms with clean owned retirement. All 158 memory samples remain within the client/system bounds:
peak working set 657,993,728 bytes, sampled private 482,611,200, kernel commit peak 666,210,304, and
minimum available system memory 6,707,712,000. A full source check passes in 56.011 seconds.

Read-only catalog and trace comparison identifies the owning invariant. Preview publication fills
recovered capture dates, changing `COALESCE(capture_local_time, file_local_time)` and thus gallery
order, while `catalog_state.revision` remains unchanged. The detail query, timeline and layout
manifest can therefore disagree while claiming one catalog revision. SQLite integrity is intact;
the defect is publication invalidation, not missing media or an identified decoder error. This
reproduces the ten-position discrepancy and persistent-gray workflow; it does not establish that
every earlier screenshot or transient error has this sole cause.

`recovered_preview_capture_time_retires_the_old_timeline` fails on the original publication method:
expected revision two remains one. The correction advances the revision only when the effective
chronological key changes, in the same transaction as guarded metadata/artifact publication.
Existing stale-cursor handling can then reject the old timeline. Repeating compatible metadata or
changing dimensions alone does not retire the catalog revision. No schema, bridge, dependency,
source-media mutation, viewport compensation or retry policy is added.

Preview publication now has a typed owner under `sqlite_catalog/preview_publication.rs`. It retains
the original request/source/root authority checks, artifact ownership and cleanup, sibling revision
adoption, transaction admission, and rollback. Independent comparison finds all eight original SQL
statements and seven binding groups preserved, with the effective-time check and atomic revision
update added. The application publication guard still spans the complete catalog commit. ADRs
[0014](../architecture/0014-query-wide-gallery-layout-manifest.md) and
[0025](../architecture/0025-invariant-owned-workflow-modules.md) record the invariant and boundary.

All three time-anchor/query-window regressions pass after the correction. The preview-filtered Rust
suite passes 189 tests, with zero failures and three existing manual performance tests ignored
(73.87 seconds test execution). Added publication cases prove dimension-only stability, missing-date
recovery and reordering, and stale-source rejection without metadata/revision changes. The independent
read-only review reports no new blocker and charges six active minutes; it runs no tests. Continuous
revision changes and native convergence remain the repaired replay's responsibility.

## Repaired native workflow and retained harness failures

Repaired run `879ce45925e74d2d9f3a1bdd7861a647` uses a fresh client and 12-item stimulus with the same
frozen mixed corpus. Import of 10000, four mixed-library jumps, addition from 12 to 2012, and all four
added-library jumps pass. Its 299 recorded windows have zero same-revision manifest disagreements;
the formerly failing top jump displays current pixels in 3.816 seconds. The complete lifetime still
fails: a generated-file deletion receives Windows sharing violation 32 after deleting 489 of the
planned 1500 files. It exits one after 216757 ms with clean owned retirement. The source adapter's
publication guard intentionally excludes delete sharing; the exact holder at failure is unknown.
No source guard is weakened and no full deletion or complete replay pass is inferred.

The failed intent is retained without advancing its ledger. A 47.519-second full integrity check
proves that exactly its first 489 intended files are absent, all 1523 survivors retain their
identities/content, and all 10000 frozen sources retain their identities, bytes and dates. The
810 memory samples stay within bounds: peak working set 1,190,363,136 bytes, sampled private
1,316,446,208, kernel commit peak 1,320,656,896, and minimum system available 6,198,861,824.

The separately admitted removal continuation starts from the earlier intact generated-only 2012-item
root. Its stimulus handles only sharing violation 32, with root/path/identity/hash revalidation on
each attempt, a two-second per-file bound and one 300-second batch deadline. Other failures propagate.
Six generated-file guard cases pass, including real lock release/timeout, changed-source rejection,
expired batch, and the review-found deadline edges before/after revalidation. The single scoped
recheck charges five minutes, finds no new S0/S1, and confirms the deadline correction in code.
This helper is local test infrastructure, not a product retry or physical-operations capability.

Run `251298f9605f49e1b65fddbba4276e9a` completes the entire 1500-file deletion, 512-item convergence,
post-removal current pixels, three direct jumps, and exact source/catalog verification. Deletion
starts at 18731 ms, stimulus completion is 42915 ms, current pixels are ready at 78636 ms, and the
idle catalog oracle passes at 91116 ms: 72.385 seconds from mutation start, within the unchanged
300-second bound. Post-removal bottom/middle/top pixel checks take 2.022, 12.105 and 5.222 seconds.
All 183 windows have zero observed same-revision mismatch. Ten files encounter bounded sharing
waits; none exceeds the per-file allowance. The full post-exit source check passes in 46.951 seconds
for 10000 frozen files plus 512 surviving stimulus files (704,936,782 bytes).

Its functional body completes, but the overall result remains failed: the installed Flutter binding
unmounts a successful test's providers before `tearDownAll`; the harness subsequently reads an
already disposed container to stop synchronization. Earlier failing bodies were not unmounted by
that binding path. The recorded result has `complete: true`, no functional failures, and this explicit
shutdown failure. It exits one after 115744 ms with Job closure and no cleanup failures. All 430
memory samples pass: peak working set 824,422,400 bytes, sampled private 882,233,344, kernel commit
peak 890,040,320, and minimum system available 5,763,837,952. This is not a passing complete replay.

The harness correction awaits synchronization stop in the test body's `finally`, before framework
unmount, with its ten-second timeout intact. Cleanup-only probe `e8df361af68a45e99792a1b8d5d42313`
opens the retained 512-item generated catalog, verifies current pixels and read-only oracles, and
passes shutdown with no test failures. It exits zero after 12935 ms, with clean Job/process retirement.
It cannot upgrade either earlier failed lifetime or substitute for the original whole-window
UIA/Release/full-Daily gates. All these native runs use framework pointer input in the real Windows
Debug runner and actual Rust/decoding; they do not establish operating-system UIA or Release input.

The cleanup probe's stop completes in 17 ms before framework unmount. Its 43 memory samples pass:
401,334,272 peak working-set bytes, 391,389,184 sampled private, 405,798,912 kernel commit peak, and
6,253,215,744 minimum system available. The final 49.232-second post-probe integrity check passes
for all 10000 frozen files and all 512 survivors. The repaired run, removal continuation and cleanup
probe use the same Rust DLL SHA-256:
`43B5AB01FC2A8AF58F8A0B48384B8ADA589F741219B61D2361F6B01E2A0A8AEF`.
Their per-run artifacts, receipts, logs, pointer files and final source hashes remain in ignored local
evidence. None of the original failed runs or their raw records is replaced.

## Capture-time correction verification and scope

`quality_lint.ps1` passes on the corrected product, including formatting, Clippy with warnings denied,
Dart analysis and repository script/bridge/policy guardrails. The native Debug build and scoped
harness analysis pass. No Flutter presentation code, generated bridge, schema or dependency changes.
The earlier C01/C02 complete-Daily failures and consumed retry allowances remain; no fresh complete
Daily, hosted final-source, Release input/decoding, real-library or R2c acceptance is claimed here.

Physical size is recorded rather than hidden by extraction: the catalog facade changes from 5127
to 4765 lines (including existing test-only support, with zero inline test cases). The publication
owner has 469 production lines and zero inline tests; its dedicated publication suite has 92 lines,
and the time/query-window suite has 207. The existing aggregate catalog test file remains 8002 lines
with 112 test cases and adds only module registrations. Its broader decomposition remains debt.
No further unrelated behavior is added to that aggregate or facade in this repair.

## Rail publication and viewer geometry correction

This bounded continuation starts from `0fa23ff`; it changes Flutter presentation only. The
[complete scope map](../plans/r2c-closeout.md#complete-scope-and-retained-readability-findings)
retains the other readability findings and all 24 R2c variants. These two presentation corrections
do not close the transient post-deletion thumbnail observation or the complete acceptance cycle.

The new publication regression fails before the fix because the rail is absent in the first frame
of revision two without replacement metrics. The viewer regression fails on its first hidden frame:
the wall's width changes from 940 to 1020 at a 1280-wide window. The correction keeps the rail's
80-pixel allocation while removing its controls during viewer display, preserving existing Windows
semantics retirement. At initial widths 1280 and 1000, repeated open/return and a real 20-pixel
resize while open preserve the gallery's resulting width, center-card identity/rectangle and scroll
offset on every checked return frame. The resize stays within one sidebar mode.

`LibraryTimeRailPresentation` owns only a retained painted projection/value and input lifetime.
An incompatible query, layout shape, controller or empty timeline clears it. Across a newer revision,
the prior frame is passive until geometry arrives; it never supplies old metrics to navigation.
Same-revision navigation retains the original stable metrics/virtual geometry/window tuple and its
active target semantics. An initial implementation accidentally froze that target display while a
seek hid metrics; two existing tests detected it. Restoring the same-revision tuple corrects that
regression without weakening the cross-revision boundary.

Independent review identifies an additional interaction boundary within this correction: Flutter
retains an active Slider drag across disabled/enabled updates. A real pointer regression presses in
revision one, disables during revision two, restores geometry and releases the original pointer.
Before input retirement, the gallery jumps from the required 1000 offset to 3250. A `KeyedSubtree`
bound to revision/query/layout/controller now retires that gesture; the original release preserves
1000 and a new pointer still navigates. The retained frame continues painting through the change.

The [official Material Slider catalog](https://m3.material.io/components/sliders/overview) remains
the component choice. The installed Flutter source confirms `Slider.onChanged: null` disables its
framework input/semantics, while the render object's disable setter does not itself end a drag.
The existing first-party Slider/IconButton and subtree lifecycle solve those boundaries; no custom
gesture implementation, dependency, source operation, schema or bridge change is introduced.

Focused evidence includes six publication/gesture cases, eleven existing time-navigation cases,
two viewer geometry/resize cases, twelve gallery resize cases, nine rail cases, the retained gallery
semantics case, three viewer navigation lifecycle cases and 42 unified-screen cases. The original
failure logs and intermediate failures remain under ignored `.build/c05-presentation/`; the first
resize-test attempt incorrectly assumed fixed sidebar width across a compact-mode threshold and
selected the first visible card instead of the center anchor. Its corrected test geometry is not a
product workaround. Independent review consumes eight active minutes and reports no remaining
actionable blocker in the corrected scope; native/client and final gates retain separate authority.

`quality_lint.ps1` passes, including warnings-denied Clippy, full Dart analysis and formatting.
An earlier redirected Windows PowerShell 5.1 invocation stopped on normal Clippy stderr despite
the command succeeding; the complete unredirected gate passes. Native Windows accessibility passes
both integration cases and all original ten whole-window UIA phases, with zero exit, owned process
retirement, Job closure and no cleanup failure. The stored transcript is
`.build/c05-presentation/windows-accessibility.log`. This is current native semantics evidence;
real mixed-media presentation is recorded below; Release input and complete Daily remain separate
obligations.
The settings-menu-open probe succeeds on its existing second attempt and retains the first
attempt's evidence-file replacement error in `lastMismatch`. This passing gate does not erase that
observation or establish a causal correction for the historical C02 failure.

Physical size, including blank lines, remains visible: screen 2321 to 2326; navigation 703 to 699;
annotated rail 487 to 506; vertical Material adapter 86 to 94; new presentation owner 127 production
lines. These owners contain zero inline tests. Dedicated new suites have 198 publication and 193
viewer lines. The screen only preserves a composition constraint; it does not acquire another
workflow. Broader controller/screen decomposition remains outside this correction.

### Read-only mixed-media presentation session

The one admitted Presentation lifetime `db57ec939e7943caaec6360587f381da` passes in the actual
Windows Debug client with real Rust/catalog/preview/source adapters and framework-pointer input.
It uses the retained generated 10000-file corpus and the exact 512-item post-removal stimulus;
it performs no additions, removals or source rewrites. The parent lifetime is 30591 ms, exit zero,
with confirmed process retirement, Job closure and no cleanup failure. Synchronization stops in
16 ms before framework unmount. Original failed workload lifetimes remain failed.

Each root is selected through the source control, then jumped to normalized positions 0.5 and 0.9
without wheel input. Each position has two viewer open/return cycles, checking six frames per
transition: all 96 frames preserve the wall rectangle, center-card identity/rectangle and exact
scroll offset. All eight viewer opens show decoded original-source pixels with the expected
location identity. Sixteen visible-gallery observations contain 12 to 20 current decoded tiles,
with no unresolved visible slot. Each root's passive refresh retains the rail for six checked frames.
Those refresh observations use revisions 256 and 258 respectively; they are not a deliberately
delayed cross-revision publication. The deterministic publication/old-pointer tests own that proof.
Actual resize is covered by the focused widget tests, not this fixed-size native session.

The 109 resource samples report 650809344 peak working-set bytes, 980844544 sampled private bytes,
1207881728 kernel peak commitment and 5545291776 minimum system-available bytes. These satisfy the
unchanged 2 GiB client ceiling and 2 GiB system reserve. Full pre/post integrity passes in 48.641
and 48.406 seconds for all 10000 frozen files (10921494393 bytes) and 512 stimulus survivors
(704936782 bytes). Both checks cover exact membership, hashes, sizes and file identities. Frozen
files also match their baseline creation/modification timestamps. Stimulus modification time is
checked for stability during each read, not against its baseline; baseline timestamp preservation
for those survivors is not established by this oracle.

Artifact SHA-256 identities are executable
`2A24C91530C05D88B29ACC4E6FCF73199C237DEB04159B232A9FD7E43BD98DCE`, Dart kernel
`8EC3E64243F25382A1A5BB480DAC332091BDCEA36859D29BAE2621858EF0F33B` and unchanged Rust DLL
`43B5AB01FC2A8AF58F8A0B48384B8ADA589F741219B61D2361F6B01E2A0A8AEF`.
The admission, source hashes, raw output, result, process and memory receipts remain under ignored
`.build/c05-fixture/` and its GUID-owned `build/integration-storage-*` directory. This session
supports the selected UX-03/04/05/08 presentation paths; it does not replace the 24-variant roster,
a complete bulk replay, post-deletion transient-error diagnosis, Release or final-source gates.
The scoped evidence recheck charges three further review minutes (eleven total) and narrows the
stimulus timestamp statement to the oracle's actual proof. It finds no remaining material mismatch;
it does not repeat product review or admit another native run.
