# R2c controlled closeout cycle

Status: discovery complete; 24-variant roster retained; two-family repair batch frozen.

Current implementation checkpoints: `d91580d` adds the connected root-interleaving regressions;
`dd3f556` contains the queue-readiness correction, work bounds and observation measurements.
Their focused checks pass. Applicable lint is interrupted at the separate native probe evidence
owner; hosted and final-source verification remain open. The same-folder count follow-up below
does not change this disposition.

This record implements the [bounded execution plan](../plans/r2c-closeout.md). The canonical
[roadmap](../roadmap.md) retains stage and queue authority. Findings belong in the existing
[interleaving ledger](r2c-interleaving-remediation.md); this record maps the fixed journeys to
evidence and records execution budgets rather than introducing another issue backlog.

## Baseline

- Started: 2026-09-09 05:07 UTC. Branch: `codex/r2c`.
- Source: `228403f1af9c7568c52aaa3f602cc28f21f230d3`; initial working tree clean.
- Product source is unchanged from `49f648aca482f53e0358acea13bf3636096a985d`.
  The intervening commits change only repository instructions and documentation.
- Windows x64 build 26340; Windows PowerShell 5.1.26100.9233; Rust/Cargo 1.97.1;
  Flutter 3.44.9 (`6b182d2c7585eba26d4edce0f97630effd256c33`), Dart 3.12.2,
  engine `5a2a6a42cce67f965cf540fcecf616faca624aa1`.
- Existing unsigned Release executable SHA-256:
  `F7C233C29A24C74A55EF9C9DB62D2D4005100BFB3A0FC7478BDFC7D686B28372`.
  Packaged Rust DLL SHA-256:
  `240CB46DA81C072023EDC3F1956856EDE7ED56F622BC3591D391B982D0A7E922`.
  Their retained unsigned evidence identifies dirty source at `74688ec`; these hashes identify
  the starting artifacts, not a fresh build from the baseline commit.
- No application, Cargo compiler, or Flutter tester was active at the initial process check.
  Four pre-existing Dart process records remain untouched. Heavy commands run serially under the
  repository tooling lock, with one Cargo compiler job to avoid the recorded commit-memory failure.
- Controlled source changes use generated fixtures and separate derived storage only. Real roots,
  retained catalogs, installed services, signing, and cloud hydration are not discovery inputs.
  Private logs and paths remain outside tracked evidence.

## Starting evidence and unresolved obligations

The [browsing-recovery record](synchronization-browsing-recovery.md) remains the exact starting
checkpoint. Its complete first Daily failed; later focused and partition results cannot replace it.
The original mixed-load run had P0 P95 288 ms and complete P1, but only 2,944 completed P2 owners
at its 300-second creation-to-reopen deadline. The unchanged isolated pass took 274.12 seconds.
Both results remain evidence. No additional unchanged replay has been consumed by this cycle.

The mixed-load workload remains 25 P0 samples with P95 at most one second, 2,048 P1 candidates,
10,000 P2 source entries, a 4,095-entry logical page, complete publication and authority retirement,
and FULL reopen inside the existing 300-second deadline. Earlier per-stage deadlines remain intact.
Historical hosted latency attribution and the recorded hosted `application-ready` timeout remain
open in the interleaving ledger; later local success cannot erase either failure.

Real EXE exit/relaunch, actual Release decoding/input, and simultaneous multi-root update/removal
require explicit coverage mapping. Widget remounts, static preview accessibility fixtures, and
catalog-free bridge smoke do not establish those results. Missing evidence is not a diagnosed bug.

## Frozen discovery roster

Frozen on 2026-09-09 at the baseline source above. Each row names one normal, interleaving, or
failure/restart variant; supporting boundary tests do not multiply the roster. `Mapped` means the
test and owner were inspected, not that the complete client path passed. Required missing paths
remain open until recorded evidence closes them. Existing tests may be reused only after matching
their actual passing output and unchanged source; the final complete Daily remains separate.

| Variant | Fixed action and expected result | Owning boundary and evidence path |
| --- | --- | --- |
| UX-01A | Warm existing catalog starts with usable content; no-change continuity enumerates/opens no source and creates no inventory | Startup/lifecycle; `library_synchronization_lifecycle_owner_test.dart`; Rust `one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning`; mapped, combined client observation pending |
| UX-01B | Close while start/poll is pending, then launch the same fixture catalog; old PID exits within the existing bound and old epoch cannot publish | Window shutdown and synchronization epoch; `window_manager_actions_test.dart`, `library_synchronization_test.dart`; owned EXE relaunch pending |
| UX-01C | Launch with interrupted first import; display paused checkpoint, require explicit Continue, preserve truthful portable capability | Scan restoration; `library_controller_test.dart`, `library_viewer_position_test.dart`; process evidence shares UX-01B/02B |
| UX-02A | Import through the actual picker with Chinese paths, wrong-extension PNG and damaged input; exact count/issues and unchanged source | Primary scan, media admission; `integration_test/scan_workflow_test.dart` controlled picker case; prior Debug native evidence mapped |
| UX-02B | Pause then cancel before native Started/registration; cancel wins, late Started does not regress feedback, execution retires | Scan control; `library_primary_scan_control_test.dart`, retained-import workflow; deterministic boundary mapped, real interruption/restart pending |
| UX-02C | Cancel replacement before commit or fail display after commit; preserve previous baseline or retry display exactly once without rescan | Publication control, committed refresh; Rust `accepted_cancel_before_projection_commit_preserves_the_published_baseline`, `library_primary_scan_workflow_test.dart`; mapped |
| UX-03A | Switch root/search/sort and cross a page while publication proceeds; latest query owns coherent page/count/timeline | Query snapshot and viewport; `library_query_snapshot_reader_test.dart`, `library_controller_test.dart`; mapped, Release input pending |
| UX-03B | Expand a folder, change revision, load another folder page; replace obsolete window and append only at the same revision | Folder paging; `library_folder_controller_test.dart`; mapped, Release input pending |
| UX-03C | Fail display after committed removal/update, replace query and explicitly retry; retain page, settle loading and never repeat source action | Query refresh/removal; `library_query_snapshot_reader_test.dart`, `library_update_refresh_test.dart`; mapped |
| UX-04A | Materialize cold then warm actual media previews; verify pixels, source version, cache ownership and unchanged source | Preview store/media adapters; `media_format_tests.rs`, existing seven-format acceptance; mapped, Release decoding pending |
| UX-04B | Rewrite fixture with identical size/ID/mtime while the old request is pending; reconcile one path and show new pixels, reject old publication | Source reconciliation/lease; `preview/tests/source_reconciliation.rs`, `preview/tests/failure.rs`; mapped, interleaving evidence to verify |
| UX-04C | Hold source exclusively or present corrupt bytes, then recover; preserve precise failure, valid source retries and no stale ready publication | Preview failure; `source_open_failure_keeps_its_cause_and_does_not_enqueue_reconciliation`, `explicit_retry_of_same_version_corrupt_source_retires_legacy_ready_ownership`; mapped |
| UX-05A | Open original, navigate both ways and return; actual decode, correct anchor and released source slots | Viewer/source reader; `integration_test/support/viewer_source_workflow.dart`, `library_viewer_position_test.dart`; Debug decode mapped, Release pending |
| UX-05B | Close/reopen while paging or buffer copy is pending; old completion/errors remain retired and new navigation works | Viewer session/source lifetime; `library_viewer_navigation_lifecycle_test.dart`, `library_source_read_lifecycle_test.dart`; mapped |
| UX-05C | Same-path source rewrite, then authoritative rename/removal; newest pixels and stable asset until authoritative removal | Viewer source generation; `library_viewer_image_test.dart`, `library_viewer_position_test.dart`; mapped, Release input pending |
| UX-06A | A/B update while queued C is cancelled; independent progress and only C is cancelled | Multi-root update; `library_update_controller_test.dart`; mapped |
| UX-06B | Remove C while A publishes and B continues, then register C while old cleanup remains; no repeated unregister, stale root or cleanup authority | Root removal, publication and cleanup; `library_update_controller_test.dart`, queue registration/retention regressions; exact connected overlap pending |
| UX-06C | Make fixture A unavailable then restore it while B updates; preserve A's catalog and B's progress | Root availability/scheduling; production offline/available and R2c-R isolation cases; mapped, connected overlap pending |
| UX-07A | Original 25-sample P0/P1/P2 workload through complete P2 publication, authority retirement, synchronized state and FULL reopen | `production/tests/priority.rs` and `priority/recovery_completion.rs`; original 300-second failure remains S1, discovery observation timing added |
| UX-07B | Same workload with per-poll versus per-epoch connection lifetime; preserve proof and production P95 bound | `priority/connection_lifetime_control.rs`; existing two-arm evidence mapped, no full-recovery substitution |
| UX-07C | Interrupt/reopen exact leases and exhaust recovery retry; retain durable failure/lineage and keep other roots eligible | Production restart/stop and queue retry owners; `production_restart_recovers_an_expired_live_gap_lease_and_retains_its_consumer_lineage`, `exhausted_recovery_candidate_prevents_authority_completion`; mapped |
| UX-08A | Jump by scrollbar/time rail then reverse before completion; immediate visible demand and no old seek rollback | Gallery visible range/time navigation; `library_time_navigation_test.dart`; mapped, Release input pending |
| UX-08B | Original ten-phase populated whole-window UIA sequence and native process exit; no invalid AXTree | `integration_test/windows_accessibility_bridge_test.dart` and existing public runner; prior local pass and hosted timeout both retained |
| UX-08C | Keyboard menus and task Retry/Cancel; correct focus return, immediate feedback and one committed action | Shared menu/task surfaces; `ame_menu_test.dart`, `library_retained_scan_interaction_test.dart`; mapped, actual client keyboard pending |

Flutter filenames above are under their existing `test/app` or `test/features/library` owners;
Rust test owners are under `rust/src`. This is one fixed cross-layer discovery pass, not a full
repository audit. No UI redesign, source operation feature, dependency or schema change is admitted.

### Observation preparation

The first mixed-load probe measures accumulated production poll wall time separately from the
existing test observer's evidence-query wall time. It prints bounded ten-second progress reports
and terminal totals. Hypothesis: the unresolved completion cost may lie in production work,
test observation, or both; no cause has been selected. Every original query, workload, assertion,
sleep and deadline remains in place. This diagnostic-only test change cannot close item 2 alone.

The installed Release storage path intentionally excludes `CEDARFLAKE_AME_TEST_STORAGE_ROOT`
through `cfg(debug_assertions)`. An ordinary Release launch would use the configured user catalog.
It is not a safe substitute for an isolated fixture run. A reviewable client-run description is
required before any retained-root authorization request; no configuration or real catalog has
been altered for this cycle.

### First controlled mixed-load observation

The current-source single-test inventory discovers 1,504 tests. The first discovery invocation
adds only the timing counters described above and runs the exact production P0 priority test with
all features, one compiler job and one test thread. It passes in 218.85 seconds; compilation and
invocation total 264.81 seconds. All 25 P0 samples progress both lower lanes; P0 P50/P95/maximum
are 88/110/111 ms. P1 completes 2,048 candidates; P2 stages and completes all 10,000, closes its
coverage, publishes `current`, retires its authority and passes FULL reopen plus source-byte checks.

The recovery tail takes 138.051 seconds over 3,068 polls: 99.110 seconds accumulate inside production
poll calls and 31.803 seconds inside the existing test evidence queries. The remaining tail includes
the unchanged sleeps and reporting. These concurrent wall-time observations do not assign worker
CPU or disk time, identify the historical timeout cause, or justify removing observer assertions.
The original failing Daily and 274.12-second isolated pass remain unchanged in the prior record.
Local output: `build/diagnostics/r2c-closeout-mixed-observation.log`.

The Debug client preparation uses the existing production `main` with only an in-memory
`SharedPreferencesAsyncPlatform` and the existing Debug storage override. Its entrypoint refuses
non-Windows, non-Debug or missing fixture-marker execution. It introduces no production option or
Release storage bypass. Its generated sources are 2,304 bounded 320-by-240 PNGs split across roots
A/B/C, with a source hash manifest and derived storage outside those roots. The scratch entrypoint,
fixture generator and owned-process wrapper live under `build/diagnostics`; they are discovery
equipment, not a second committed automation framework or current Release evidence.

### Bounded Debug desktop observations

Both allowed desktop sessions have now been consumed. The first owned run reached its unchanged
1,800-second deadline while waiting for desktop-tool access; no application input occurred. Its
receipt records confirmed process exit and Job closure with no cleanup failure. The extended
permission wait is tool/user wait, not active engineering or evidence of an application crash.

The second session ran from 08:10 to 08:38 UTC and included two actual process lifetimes within
the same 30-minute limit. The first lifetime ended normally after 663,947 ms; the second ended
normally after 959,746 ms. Both receipts record exit code zero, process exit, Job closure and no
cleanup failures. A fresh PID and start time distinguish the relaunch from a widget remount.
The Debug executable/DLL SHA-256 identities are respectively
`DEE7E6CD4E0A621D310ED75FE7165C366556855ED473D357DE30A89EB6F510FA` and
`50CCD3C6568CCB3E538ECD34A615963B3A1D5BB5ADA02C2E1372EAB746FED2F1`.

| Observed transition | Result and evidence limit |
| --- | --- |
| Native picker imports A with four Chinese subfolders | 2,052 entries checked and 2,048 images published; actual generated PNG thumbnails appear. Wrong-extension/corrupt admission remains covered by the existing controlled scan integration, not this all-PNG fixture. |
| Attempt to pause A after visible scan progress | The scan completed before the next action took effect. This is not a successful pause, interruption or checkpoint-recovery case. |
| Open image, press Right, press Escape | Actual source view changes from `image-2047.png`, 1/2048, to `image-2046.png`, 2/2048, then returns to the gallery. Pending-read retirement and Release decoding are not inferred. |
| Close EXE, relaunch same derived storage | All 2,048 catalog images return. Expanding A shows four Chinese folders; selecting `图片-0` reports 512 images. This proves ordinary completed-catalog relaunch, not close-during-poll or interrupted-import recovery. |
| Search input | Focus is confirmed, but two literal-text tool injections produce no visible text; a single key produces `a`, a later key exposes an IME composition popup, and clear-search works. Ordinary user input failure is unproved; full search/input verification remains open. No product patch is based on this ambiguous tool/IME observation. |
| Ascending sort and time-rail drag | Query keeps 512 images, replaces the visible order, and materializes the new viewport after transient placeholders. A single-date fixture does not prove multi-date geometry or a reversal while a seek remains pending. |
| Import B while browsing A's subfolder | B publishes 128 images; A's selected subfolder remains at 512, and the all-library count becomes 2,176. |
| Start an A/B update batch and select A's subfolder | Both roots are observed updating; the next state records both complete with counts 2,048/128 and the selected subfolder at 512. This does not establish C removal during publication. |
| Remove B, then import the same directory again | B disappears from the sidebar, A remains usable, and reimported B has 128 actual thumbnails. Raw-cleanup overlap and generation isolation still require the connected persistence fixture. |

After both process lifetimes, all 2,304 fixture files match the original byte lengths and SHA-256
manifest, totaling 1,797,833 bytes. No stimulus altered source files. Both application stderr logs
are empty. The owned storage retains process receipts, application logs, the initial manifest and
`source-integrity-after.json`; machine-specific locations are intentionally omitted here.

The existing unchanged-source evidence was rechecked against the retained logs: the controlled
scan completes with exit zero and no cleanup failure; whole-window UIA completes all ten exact
phases (including 126 elements at `application-ready`) and confirms native exit. The focused
preview/source-reconciliation, query/folder, explicit-retry and source-loading suites retain their
recorded passing results. The initial Daily's four failures remain recorded; reused passing
boundaries do not transform that invocation into a pass or replace the final complete Daily.

### Remaining connected discovery evidence

The cumulative timing probe retains the exact original mixed-load test and reports a pass in
243.83 seconds, including full publication and FULL reopen. Its 7,336 production polls total
151.923 seconds. Observation includes 49.314 seconds in 14,672 root queue-metric reads and
13.590 seconds in root-availability checks. Scheduling totals 60.734 seconds. Catalog revalidation
totals 16.793 seconds. The recovery tail separately records 152.565 seconds, including 109.401
seconds in polls and 35.840 seconds in unchanged evidence queries. Nested totals are inclusive;
per-stage poll counters round each call down to milliseconds and cannot be summed as exact wall
time. These measurements identify current investigation owners, not the cause of every historical
latency outlier. Output: `build/diagnostics/r2c-closeout-mixed-stage-totals.log`.

One refinement splits the existing scheduling call without changing execution order or admission.
The original mixed load passes in 260.60 seconds, with complete P2 publication and FULL reopen.
Across 7,465 polls, live scheduling totals 32.658 seconds, journal scheduling 27.193 seconds,
recovery selection 5.236 seconds and first-import completion 3.524 seconds. Root metrics total
54.612 seconds. The 169.800-second recovery tail contains 120.891 seconds in production polls
and 41.443 seconds in unchanged evidence queries. The three discovery invocations have distinct
timing probes; none is an unchanged replay or a causal correction. Output:
`build/diagnostics/r2c-closeout-mixed-scheduling-totals.log`. Further whole-load discovery replays
are not planned; targeted query work measurements finish this observation.

UX-06B's actual admission permits C removal during unrelated A/B updates, but C reimport waits for
all update leases, including their display refreshes, to retire. The connected fixture must follow
that order and then overlap C's new generation with bounded cleanup of its old retired raw spool.
Existing tests separately prove removal scope, generation advancement and cleanup retirement;
they do not yet connect this full sequence. A focused test-only fixture is being added at the
existing metadata-inventory spool lifecycle owner without changing product behavior.

The first fixture compilation used an incorrect generation field and was corrected to the intent's
generation. Its first executable run reports 22 passing existing cases and one new-case failure:
the helper had already committed the complete inventory page, transitioning the run to `Comparing`,
where source enumeration authority is intentionally invalid. This is not evidence that FULL reopen
or old-spool cleanup revoked the new root. The corrected fixture observes the new run while raw
storage is ready and the run is still `Running`, then separately commits the complete page and
checks normal source-authority retirement. Both failed outputs remain retained.

The corrected fixture and all 22 existing spool-lifecycle cases pass together in 26.39 seconds.
The connected case proves real A publication while B still owns its scan, C removal and partial
old cleanup, C's next generation after A/B finish, current raw reads across old cleanup and FULL
reopen, and normal `Running` to `Comparing` source-authority retirement. Five generated source
files retain their relative paths, bytes and modification times. A helper mutability compile
correction is retained as a separate failed attempt. Output:
`build/diagnostics/r2c-closeout-spool-connected-4.log`. This closes the connected persistence
part of UX-06B; it does not substitute for Flutter admission, automatic cleanup scheduling,
physical database reclamation or Release interaction evidence.

UX-06C has no exact connected passing case yet. The existing single-root offline/reconnect test uses
an empty baseline; the two-root reliability test injects journal failure rather than actual root
loss and stops before recovered publication. Their passing outputs establish only those boundaries.
The missing oracle is a populated A/B catalog where A's baseline survives actual fixture loss,
B publishes new content during A's recovery, and A then converges through FULL reopen and source
checks. This remains one frozen variant, not an invitation to multiply availability combinations.

The missing production connection is now exercised by
`production_offline_root_recovery_preserves_catalog_while_peer_changes_publish` and passes in
1.56 seconds. It uses real scans for A's two-image and B's one-image baselines, renames only the
generated A source offline and back, and requires B to publish one P0 file while A is unavailable
and another while A's P2 source enumeration is held by the existing test gate. A's complete cached
page stays usable; after release both roots synchronize, exact A recovery and all queue/lineage
authority retire, FULL reopen succeeds, no automatic scan row appears, and all five resulting
source files retain the expected bytes and modification times. The two intentional B additions
are recorded separately from integrity assertions. The queued observer fixture does not establish
real Windows watcher delivery or client input. Output:
`build/diagnostics/r2c-closeout-root-availability.log`.

Release decoding/input, actual interrupted-import and pending-poll relaunch, and the named
multi-root unavailable/cleanup overlaps retain their explicit client or connected-test gaps.
No additional desktop session, broader fixture audit or new product repair is authorized by these
observations. The two original S1 obligations retain their IDs in the interleaving ledger.

## Post-discovery queue repair evidence

The terminal-history work regressions pass after extracting queue readiness and narrowing the
active-state metric projection. With 256/4096 retained completed rows, exact metrics now use
2058/28938 VM steps; live and journal path readiness each use 247/247, authoritative live readiness
248/248, and legacy recovery readiness 375/375. The 113 queue tests pass together, including due
retry, lease expiry, exhausted attempts, superseded unresolved candidates, optional-index absence,
claim ownership and rollback. Output: `build/diagnostics/r2c-c01-queue-after.log`.

The unchanged priority group passes all ten tests in 438.38 seconds. Its connection control records
PerPoll P95 199 ms with 1786 opens/closes and PerEpoch P95 103 ms with one open across 5122 polls.
The complete mixed-load case records P95 109 ms, maximum 124 ms, and all 25 P0 observations while
P1/P2 progress. All 2048 P1 candidates complete. The 162.297-second recovery tail completes all
10000 P2 entries/candidates/owners, closes journal coverage, publishes current state, retires
authority and passes FULL reopen under the original creation-to-reopen deadline. This group total
is not the duration of the single full-recovery case. Output: `build/diagnostics/r2c-c01-mixed-after.log`.
The new source still requires applicable lint, hosted evidence and final-source gates; these local
measurements do not uniquely attribute historical outliers or close R2C-C01 alone.

Physical ownership remains explicit: the queue facade is 2444 lines (2670 before extraction,
including existing test-only helpers; no inline test suite), readiness is 338 production lines,
and metrics is 229 production lines. Their new dedicated tests are 268 readiness and 119 retained-
history lines; the existing 7040-line queue test module remains decomposition debt. Production
synchronization remains 4048 owner lines plus 10486 inline-test/module lines; this change adds only
the three-line test module declaration there and puts the connected availability case in its own
390-line suite. The root-reregistration case occupies 398 dedicated-test lines. These measurements
do not waive the roadmap's remaining production runtime/lane and test-suite decomposition.

Live hosted inspection also corrects the earlier evidence snapshot: baseline `228403f` has a passing
[CI run 34313274396](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34313274396), completed
2026-09-09 05:34:16 UTC, including all ten ordinary workers and their aggregate. Its Rust result is
1485 passed, zero failed and 19 ignored; the original full mixed-load P95 is 141 ms. The earlier
failed run 34294505152 belongs to `74688ec` and fails the two availability source-topology tests
already corrected by the browsing-recovery change. Neither result verifies the current dirty repair
or erases the separately recorded intermittent native-accessibility failure.

## Same-folder count discrepancy follow-up

A newly reported recurring count mismatch is checked against the already registered folder by
reading the existing catalog and enumerating names/attributes only. This diagnostic is separate from
retained-library client acceptance. It opens no image content, follows no reparse directory, launches
no client scan and changes neither catalog nor source. Its limits are 150000 directory entries and
60 seconds, with private path-level results retained only in Git-ignored local diagnostic storage.

The single census completes in 5.681 seconds: 50515 directory entries comprise 830 directories and
49685 files. The published catalog contains 48624 distinct locations/assets. No supported-extension
file is missing, no catalog path lacks a current file, and there are no skipped reparse directories
or enumeration errors. Root modification time is unchanged; this metadata check is not a byte-level
integrity or placeholder-hydration acceptance result.

The 1061 paths absent from the catalog comprise 960 videos (925 MP4, 34 MOV, one MKV), 42 other
image-format paths (23 SVG, nine AVIF, nine HEIC, one PSD), and 59 other or unidentified files.
Extensions establish classification candidates only; the diagnostic does not inspect their bytes.
All 48605 paths with the nine currently supported extensions are present. Nineteen additional
signature-admitted paths account for the remaining catalog entries. The unsupported-format gap is
therefore visible in this collection, while arbitrary supported-extension omission is not observed.
The comparison application's exact total remains unavailable, so parity with it is not established.

The retained completed foreground scan reports 48514 accepted items, 48624 published assets and
48899 issues. Its 104 unsupported-decoder and six invalid-decoder issues match active failure-code
totals of 104 and six. A subsequent read-only join confirms every one of those 110 exact issue paths
has the same failure code in the active catalog. Another 48605 historical issues report `source_revision_changed_during_scan`,
exactly the supported-extension population. That pattern requires investigation at discovery/source
revision admission; it does not prove that all those files changed or were omitted. The catalog also
retains one comparing inventory and 449 pending queue rows. No scan reset, automatic full rescan,
new codec dependency or count-only presentation workaround is admitted by this evidence.

A four-file generated first-read probe compares fresh and seven-day-aged PNG bytes under `.png`
and `.data` names. Identity, modification time, ChangeTime and bytes remain unchanged through
content open, header read, close and whole-file read; access time changes. This rejects ordinary
Win32 content reading as a sufficient explanation on the fixture volume. It does not exercise the
production root-relative NtCreateFile/inspector sequence or explain the retained historical events.
Output: `build/diagnostics/r2c-revision-first-read.log`; no real source content was accessed.

## Applicable lint interruption

The first current repair lint reaches formatting and rejects two line wraps in the new spool test.
The repository formatter corrects those exact lines. The second invocation then stops earlier in
the existing native-accessibility guardrail: `Write-AmeWindowsUiaProbeRecord` throws from
`File.Replace` while sequentially replacing the progress fixture, before any live-window probe.
Both outputs are preserved (`r2c-c01-lint.log` and `r2c-c01-lint-after-format.log`). The latter is a
real failed gate, distinct from that guardrail's deliberately injected failures in preceding output.

This proven active-item lint blocker admits a narrow interruption for R2C-C02's existing probe
evidence owner: inspect atomic replacement/read lifetime and reproduce the exact failure with
owned scratch only. No desktop rerun, deadline change or retry loop is admitted. C01's controlled
tests remain passing, while its applicable lint and hosted exit remain open. The interruption does
not attribute the older application-ready timeout to this newly observed file failure without
evidence. R2C-C02 diagnostic pass 1 has a 60-minute active ceiling; a bounded 20-minute evidence
investigation is the next action. Final complete Daily invocations remain zero.

Clippy for all targets/features passes with warnings denied, and Dart analysis passes with no
issues. A first direct invocation was interrupted by PowerShell treating ordinary native stderr
as a terminating error; no Cargo/rustc process remained before the corrected invocation checked
both real exit codes. These scoped checks do not replace the incomplete applicable lint gate.

## Execution accounting

Discovery closes with the two connected persistence/production cases above and explicit unresolved
EXE/Release interaction evidence. It does not declare all 24 variants accepted. The last query-cost
fixture passes FULL validation and confirms exact idle results while measuring avoidable work:
256/4096 retained completed rows require 4495/69775 VM steps for each live/journal path readiness
check, 1936/28816 for authoritative-live readiness, 4615/69895 for legacy-recovery readiness, and
14230/225430 for exact metrics. Its first setup attempted to duplicate the schema trigger's lane
row; the corrected fixture asserts that existing ownership instead. Both outputs remain retained;
the successful output is `build/diagnostics/r2c-closeout-retained-query-work-2.log`.

The single triage admits R2C-C01 and R2C-C02 from the existing ledger, in that order. No new source
safety defect or confirmed search-input defect was found. Missing client evidence retains its
original blocked variants and does not become an invented third defect family. R2C-C01 diagnostic
pass 1 begins at 09:17 UTC with a 60-minute active ceiling: verify the measured terminal-history
query work at its adapter owner, add a failing work-bound regression, establish the typed readiness
boundary, and compare the unchanged full workload after a causal correction. Its second pass is
unused. No further whole-load discovery run is admitted; production queries were unchanged at
pass admission. The subsequent work-bound regressions fail on that source before repair:
256 terminal rows require 14,230 metric VM steps against a 7,168 counting-work ceiling and 4,495
live-readiness steps against the fixed 1,024 idle-work ceiling. Output:
`build/diagnostics/r2c-c01-work-bound-before.log`. Counts remain exact; the readiness bound applies
to a FULL-valid catalog with no unresolved rows and its ordinary schema-owned access paths.

All active engineering, including delegated inspection, preparation, failed attempts, and result
analysis, counts against the plan's phase ceilings. Tool execution/wait time is recorded separately;
concurrent inspection during a tool run still counts as active work. A handoff does not reset usage.

| Phase | Ceiling | Used at last checkpoint |
| --- | --- | --- |
| Baseline and discovery | 4 h | 150 min charged; complete, including delegated work, failed fixture attempts and final query-cost measurement |
| Triage | 1 h | 35 min charged, including the new same-folder report, code inspection and bounded read-only comparison; two repair families remain admitted |
| Diagnosis and repair | 8 h | 45 min charged to R2C-C01 pass 1; query correction, focused queue/mixed-load tests and static analysis pass; lint/hosted exit pending. R2C-C02 pass 1 began 09:55 UTC for the newly observed active-gate interruption |
| Independent review and scoped recheck | 2 h | Not started |
| Closeout recording | 1 h | Not started |

Final complete Daily invocations: 0. Unchanged diagnostic replays: 0. Independent final reviews: 0.
The controlled cycle and the separately authorized external/client acceptance remain open.
