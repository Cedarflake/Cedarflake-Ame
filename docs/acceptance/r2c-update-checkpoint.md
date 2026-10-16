# Retained update checkpoint and pending-read observation

Updated: 2026-10-10. Scope: atomic scan progress, update task projection, identity-query preparation,
and bounded update/browsing observations. This record does not accept R2c or combined-source recovery.
The execution owner is [R2c closeout](../plans/r2c-closeout.md).

## Causal changes

The previous catalog boundary committed buffered locations before committing their checkpoint or
directory completion. Three focused regressions reproduced two durable commits, a cleared batch
after rejected progress, and prematurely accepted resume membership. `scan_checkpoint.rs` now
composes staging and progress in one existing Recovery-lane transaction; only a successful commit
retires the batch. No schema, source-read, generation, writer-priority or cancellation rule changes.
Four cases, each exercising both boundaries, pass: one-commit persistence, SQL failure, retained
inspection/membership rollback, and refused commit followed by a single successful retry. Three
existing resumption cases also pass.

The per-root update controller previously emitted 104 task notifications for 100 assets and four
progress checkpoints. Its extracted immutable task projection now emits those four checkpoints;
terminal counts still include the three trailing assets in completion and cancellation cases.
Individual issues remain observable, and primary-import asset delivery is unchanged. Two new
projection cases and 30 existing controller/refresh/cancellation/update-flow cases pass. Full
repository lint, warnings-denied Clippy and Dart analysis pass before native execution.

Physical reviewability: the catalog facade has 4462 lines including test support, with no inline
test bodies; `scan_checkpoint.rs` has 123 production lines and its dedicated tests have 259 lines.
`scan_staging.rs` has 107 production lines. The update controller has 602 production lines, the
task owner 171, and the dedicated projection regression 140; neither production owner contains
inline tests. These extractions identify the changed responsibilities, not a whole-facade cleanup.

## Cost evidence and limits

One pre-change headless warm scan used a copy of the completed 64490-location publication,
120000 item/entry bounds and a 300-second cancellation deadline. It completed in 192488 ms with
70115 visited entries, 64490 images and 678 unsupported-format issues. Existing test-only timers
recorded source discovery 40584 ms, prior selection 9662 ms, inspection 478 ms, location staging
36218 ms, directory persistence 33900 ms and checkpoint persistence 16953 ms. Directory/checkpoint
categories include nested buffer flushes. Their full cost is not standalone progress SQL, and the
absence of the desktop/synchronization runtime prevents attributing its separate cost.

Disposable 2048-row SQLite probes measured 1657 ms for the existing cache settings, 1336 ms for
a 16-MiB cache, and 2749 ms with spill disabled. They used Python SQLite, not the bundled Rust
engine or a complete scan. No speculative cache tuning was made. Six protected catalog/WAL
components remained unchanged after profiling; the audit opened no source media.

Ignored diagnostic provenance is under `.build/r2c-update-cost-20261009/`, including
`warm-profile.log`, `staging-result.json` and `profile-protected.json`. The temporary Rust profile
module was removed before lint/build. Its focused timing result is diagnostic evidence only.

## Native method

The native client copies the completed publication into isolated derived storage and admits one
ordinary update, no import or resume, at most 120000 items/entries and a 300-second cancellation
deadline. The complete lifetime is bounded at 1200 seconds, the client at 2 GiB and available host
memory above 2 GiB. Source media and the original/predecessor catalogs remain protected. The
test entry permits one original-image acquisition and no placeholder hydration.

One middle-timeline query holds delivery of its actual database result until a native wheel input,
with a 20-second deadline. This proves application behavior while a result is pending; it does not
claim that the database naturally took that long. Later queries run without the delivery hold.
Native pointer records, query dispatch/completion and visible item/position samples share the
same run and epoch. Normal close requires a receipt written immediately before the real input.

The first lifetime, `b45934ca4ad04d0fb434f26acbff6941`, failed in its scope observer after update
admission. The observer read the old admission receipt before a newer database snapshot and
rejected the legitimate new scan. Original catalogs remained unchanged, cleanup had no failure,
and its unpublished partial scan remains preserved. A generated interleaving reproduced this
failure. The corrected observer reads the durable admission witness after capturing the database
snapshot and checks time at that boundary; all 25 generated scope cases pass. It keeps every
identity, work bound and deadline check. The earlier failed lifetime is not relabeled as a pass.

The corrected lifetime is `c738af31a9a245afa26da6e3db9b1303`, epoch
`7402441d2e594c099d57e631be695707`, with source/helper/payload bindings under
`.build/r2c-ready-update-corrected-20261009/`. Its 719 source records and 773 total bound records
include the diagnostic entry; this is a Debug observation of the dirty candidate over
`ffd15dd5d596852613325c79e46bc67b6682e124`, not a packaged Release claim.

## Native result

The complete lifetime passes its prepared bounds and closes normally. The ordinary update takes
211836 ms from admission to terminal delivery; traversal reaches finalization at 176656 ms and
final source validation completes at 189115 ms. The preceding native update took 234882 ms. This
single comparison is about 9.8% shorter, without isolating cache/runtime variability or establishing
a general performance threshold. A 212-second warm update remains a material latency limitation.

The middle query's actual database result is ready after about 65 ms. Its delivery is held for
7122 ms until a native upward wheel arrives with request 1 still pending. Request 1 then completes;
a new query for the later visible offset 482 follows, rather than restoring offset 503. The
visible window settles at item 31117 with 160 retained items. These observations connect native
input with pending delivery and later demand. Two empty-viewport samples during the deliberate hold
have no position projection, so they are not continuous position evidence for that interval.

| Observed action | Stable pixel position | Visible previews ready | First all-ready sample | Retry samples |
| --- | ---: | ---: | ---: | ---: |
| Upward input during pending result | 1111102.667 | 31 | 9231 ms after input | 0 |
| Further upward input, -536 pixels | 1110566.667 | 36 | 9001 ms after input | 0 |
| Reverse input, +536 pixels | 1111102.667 | 31 | 286 ms after input | 0 |

The maximum extent remains 2035797.2 pixels. Further upward and reverse movement exactly match
their native deltas; the viewport does not stop at a false end. After reversal, 99.785 seconds of
samples retain the same position and ready previews without Retry. First uncached viewport
completion still takes about nine seconds; this is preserved as a performance limitation.

The Debug queue log identifies 22 slow completions among the first viewport's 31 locations, all
ready, with 15006 ms of accumulated active time. Its longest recorded active request takes
1960 ms; the latest recorded completion takes 8647 ms, including 7404 ms queued and 1242 ms
active. The largest recorded queue wait is 7467 ms. The logger omits ordinary active requests
below 250 ms, so these are not complete per-item timings. This establishes accumulated processing
and queueing in the observed wait, without distinguishing native decoding from catalog work.

Exactly one original acquisition and one completed buffer read are recorded, followed by lease
release and viewer return. This happened before middle navigation: a transient notice disappeared
before its dismissal click, exposing the underlying image. No second acquisition was attempted.
This proves the selected read/return path, not a middle-item viewer sequence.

The verified close input retires the application in 531 ms, with exit code zero and no cleanup
failures. The complete lifetime is 564295 ms. Peak working set is 525713408 bytes, sampled private
memory 489328640 bytes and kernel peak commit 490790912 bytes; minimum available system memory is
4638183424 bytes. Closed integrity and foreign-key checks pass, one root publishes 64490 locations,
and no pending/leased/retry-wait path work remains. The six protected original/predecessor
catalog/WAL components remain unchanged.

The post-close comparison checks all 64490 rows against the completed predecessor. Asset,
location and root identities, source paths, file size/times, dimensions, inspection-engine/capture
metadata, physical identity, source revision and generation all match. The headless profile's
publication independently matches the same fields. Their ordered payload hash is
`2aeebf1731ce31e21c92de245550074a88c1522ba6d928a492ef4426a1118a95`.
This is a derived-record comparison, not a whole-library source-content hash. The comparison
opens no source media; the prepared runtime source adapters retain read-only/no-hydration rules.

Raw receipts remain in `build/integration-storage-c738af31a9a245afa26da6e3db9b1303/`; concise
derived evidence is in `.build/r2c-update-cost-20261009/native-summary.json` and
`publication-comparison.json`. The preceding failed scope-observer and missing-close lifetimes
remain separate. Current candidate verification follows below.

## Candidate verification

The complete candidate Daily now passes: warnings-denied lint, 1683 Rust library tests and three
broker-binary integration tests, all 107 Flutter test files, controlled Windows scan, both native
accessibility tests with all ten required UIA phases, 16 asynchronous bridge contracts and diff
whitespace. The 23 ignored Rust cases remain owned by their explicit platform/performance/real-source
wrappers. Windows scan run `9d5eb1bb04fd46a187e4ac0381d459c9` exits normally in 51308 ms with no
cleanup failure. Native accessibility also closes its primary process and owned Job without cleanup
failure. The transcript is `.build/r2c-update-cost-20261009/checkpoint-progress-daily.log`.
The exact optimized 10000-image synthetic scan also passes, with no ignored selected test:
fixture preparation 11038 ms, cold scan 27640 ms, warm scan 16570 ms, pause 8 ms, resume 27490 ms
and cancellation 115 ms. The published catalog uses 56090624 bytes and the resumed-first-import
catalog 32161792 bytes. Its owned workload completes in 91.42 seconds. The warm measurement
attributes 9498 ms to location staging and 1705 ms to publication, while metadata inspection is
reused. This controlled result does not replace the retained-library latency obligation. Evidence
is `.dart_tool/performance_synthetic/5decb60dca13469298592630a72fec05/` and
`.build/r2c-update-cost-20261009/checkpoint-progress-synthetic.log`.
Unsigned Windows verification also passes: the fresh Release application and broker payload match,
all three native window cases and both real-engine retirement cases pass, and the isolated bridge
smoke loads the Release DLL without catalog access. Engine retirement exits with code zero and no
cleanup failure. The owner is `build/quality-unsigned-windows/evidence.json`, with transcript
`.build/r2c-update-cost-20261009/checkpoint-progress-unsigned.log`. This verifies the dirty candidate
over the stated commit; no product source changed between these gates. It does not establish signed
installation, populated Release restart or the remaining source-backed performance boundaries.

The packaged correction at `146490b36482e27ef9cd87b817f1c1a217c325a2` also completes its hosted
checks: eleven successful results, including the required Daily, synthetic and unsigned Windows
jobs, with three signing-flow jobs skipped by their configured admission. No required job fails
or remains running. This does not provide signing or installed-service evidence.

## Observed viewport backend cost

The separate optimized diagnostic selects exactly the first observed viewport's 31 locations and
their published preview-size buckets, with two workers and a fresh empty cache. Their source-file
sizes total 155953835 bytes; no file exceeds 64 MiB. The owned process has a 180-second deadline
and 768-MiB working-set ceiling. It calls the production active-preview path with explicit isolated
storage; no full-library scan or original-image viewer acquisition occurs.

Initial preparation rejects the catalog's verbatim DOS path prefix before creating storage or
reading source media. Read-only inspection proves that removing that prefix yields exactly the
prepared path, including component case. The corrected guard admits only those two exact spellings;
it does not case-fold source components. The failed preparation remains in
`.build/r2c-preview-cost-20261009/prepare-prefix-check.json`.

All 31 cold-cache previews are generated in 1694 ms. Per-request total time has a 93-ms median,
185-ms P95 and 235-ms maximum. Accumulated stage times are catalog reads 961 ms, materialization
1679 ms, artifact commit 67 ms and catalog publication 361 ms; two-worker sums are not wall time.
Source-open timers round below one millisecond per call and do not mean zero source access.
Peak working set is 87121920 bytes. Closed integrity/foreign-key checks pass and all eight protected
original, predecessor and observed catalog/WAL components remain unchanged. The temporary test
module is restored byte-for-byte before execution, and the bound executable has one exact passed
diagnostic with no selected ignored result.

The exact Debug comparison uses the same locations, size buckets and two workers in another fresh
derived catalog with an empty cache. All 31 previews complete in 8194 ms; per-request median is
381 ms, P95 1690 ms and maximum 1887 ms. Accumulated catalog reads take 1009 ms, materialization
14167 ms, artifact commit 76 ms and catalog publication 425 ms. Peak working set is 112869376 bytes.
Closed integrity/foreign-key checks pass, all eight protected components remain unchanged, and the
temporary module is restored before the one exact diagnostic executes successfully.

This pair attributes most of the observed backend difference to Debug media materialization:
14167 versus 1679 accumulated milliseconds, while catalog reads remain similar. The optimized batch
is about 4.8 times faster in wall time. One ordered pair over this cohort is not a general benchmark
or a complete Release-window timing. The Debug native queue also includes bridge/UI execution and
has only thresholded slow-request logs. Its nine-second visible completion cannot establish
optimized-client latency. Helpers, executable bindings, per-item metrics and closed summaries
remain under `.build/r2c-preview-cost-20261009/` and `.build/r2c-preview-cost-debug-20261009/`.

## Identity-state query preparation

The subsequent optimized comparison keeps the current location-upsert cache in both arms and
changes only preparation of the two physical-identity aggregation queries. Each arm seeds 10000
generated catalog identities and stages 4096 unchanged locations through production admission and
128-item batches. No real catalog or source media is read. An authorizer counts query compilations;
per-statement counters separately measure SQLite execution. Both arms perform 696320 VM steps,
32 durable write batches and produce the same complete staged payload BLAKE3
`d8607603e3ee4583738b747bb4ddd8eff01176d3711285c5f8e59a4639095353`.

Actual preparations fall from 16384 to two. Accumulated preparation time falls from 180657 to
3244 microseconds; query execution is 90599 versus 81662 microseconds. Complete staging is 1689
versus 1505 ms. Published membership/revision remains unchanged, all staged payload fields match,
and closed integrity/foreign-key checks pass. This single ordered pair establishes the preparation
mechanism and this generated workload's result, not a whole retained-library latency improvement.
The owned process exits normally in 12491 ms with a 22081536-byte peak working set, within its
180-second/512-MiB bounds. All 687 bound inputs match after the temporary instrumentation is
removed. The optimized build completes without a compiler warning.

The production correction extracts `identity_group_state.rs` from the catalog facade and reuses
the connection's bounded prepared-statement cache for those two queries. Both SQL strings and all
remaining query/consistency bodies match the preceding source exactly after removing visibility
changes and the preparation call. Results are never cached. Source generation assignment,
captured-observation comparison, alias invalidation, transactions and source access are unchanged.

The compilation regression first fails on the extracted unchanged code with 512 preparations
instead of two; its three state regressions already pass. After the correction, all four cases pass:
distinct file/scan parameters and staged precedence, peer commits and transaction rollback with
snapshot release, empty/retired observations, and conflicting aliases with known/unknown revision
semantics. The catalog facade has 4263 lines including test support and no inline test bodies.
The extracted owner has 121 lines including its test-module declaration, no inline test bodies,
and 245 dedicated test lines containing four cases. Generation/publication policy remains explicit
decomposition debt rather than moving into the read owner.

Ignored evidence is under `.build/r2c-identity-query-profile-20261009/`: the bound comparison,
restored source bytes, complete process output, red/green regressions and `extraction-proof.json`.
Standalone lint passes in 237.45 seconds. The complete fresh-process Daily passes in 2079.15
seconds: 1690 Rust library cases and three broker lifecycle cases, all 107 Flutter test files,
the controlled Windows scan, all ten native UIA phases with normal process/Job closure, and 16
asynchronous bridge contracts with matching hashes. The Rust suite retains 23 separately admitted
opt-in cases as ignored. All 810 frozen source/test/tool inputs remain unchanged.

On 2026-10-10 the optimized 10000-image scan gate passes: cold import 26818 ms, warm update
16274 ms, pause acknowledgement 11 ms, resume 25607 ms and cancellation 127 ms. The explicit case
finishes in 89.91 seconds with no ignored test; the owned process exits normally in 90047 ms with
a 28459008-byte peak working set. The earlier candidate's 15910-ms warm result is a separate run,
not a controlled comparison. This complete-workflow observation does not demonstrate a whole-scan
speedup. The two-arm preparation result above retains its narrower causal evidence.

Unsigned Windows verification passes in 302.68 seconds, including a fresh Release application and
broker, three engine-free window cases, two actual engine-retirement cases with normal process/Job
closure, and the catalog-free Release-DLL/Windows-channel smoke. Both gates preserve all 810 frozen
inputs. The artifact is attributed to the dirty candidate over `892035b`; the retained
`candidate-*-fresh.json` records, transcripts and `release-evidence.json` bind these results. No
real-library update, service installation or release publication participates. Retained whole-library
latency, assembled-destination discovery and the remaining R2c acceptance duties stay open.

## Bundled staging cost

A generated-only diagnostic seeds 10000 identities and stages 4096 unchanged published locations
through the production port in 128-item batches. Bundled SQLite 3.53.2 records 2046 ms in total:
location insertion 1567 ms, initial identity capture 128 ms, transactional identity decisions 165 ms,
asset insertion 18 ms, retained-membership acceptance 6 ms and 32 commits totaling 153 ms. Writer
admission totals less than one millisecond. Nested identity/persistence timers are not additive.
All 26 payload fields beside the scan/location keys match the baseline, the active publication and revision remain
unchanged, and closed integrity/foreign-key checks pass. Generated setup takes 5127 ms and the whole
test 7.50 seconds; no real catalog or source media participates. The temporary instrumentation is
restored byte-for-byte before the bound executable runs. Evidence remains under
`.build/r2c-staging-profile-20261009/`.

The first diagnostic build reports an existing Release-test-only unused `Duration` import in the
observation timer. Its import condition is narrowed to the existing Debug logging branch without
changing runtime statements. The initial diagnostic is successful execution evidence, not a
warning-free quality gate. The insertion result selects the separately bounded preparation-reuse
comparison in the execution plan; neither elapsed staging time nor code inspection alone attributes
that time to SQL compilation.

The two-arm follow-up preserves the complete production staging path, insertion SQL and parameters.
Actual authorizer observations count 4096 versus one location-insert preparation, with identical
901120 SQLite VM steps and complete payload BLAKE3
`6281e89da97819a3e9448fb456accb3ab38a3c54eef68ad747e8b930183ff53a`. Preparation takes 299466 versus
2049 microseconds; statement execution takes 1236584 versus 1199034 microseconds. Full staging
takes 2045 versus 1703 ms, with 32 commits per arm and unchanged published baselines. Both closed
catalogs pass integrity/foreign-key checks. The whole owned test completes in 14.69 seconds, and
its optimized build has no warning. The earlier preparation attempt rejected a changed formatting
boundary before creating either helper output or starting a build; its exact marker was corrected.
Evidence remains under `.build/r2c-staging-cache-profile-20261009/`. The single ordered timing pair
does not control operating-system cache order or establish whole-library or native-client latency.

### Location row correction

`location_row_write.rs` now owns the typed row binding and both existing insertion statements.
It reuses only the location-upsert statement through the connection's existing bounded cache;
asset insertion, transaction ownership, 128-item staging batches and upstream source/generation
decisions remain unchanged. Both normalized SQL statements and their complete parameter-expression
lists match the preceding source. The pinned adapter clears bindings when a statement returns to
the cache, and the borrowed statement retires before the caller commits.

The preparation-count regression first fails on the extracted unchanged implementation with
256 actual compilations instead of one, while both original 128-item batches commit. The corrected
case and refused-commit/retry/optional-value replacement case pass in the complete SQLite adapter
group: 590 passed, zero failed and three existing opt-in cases ignored, in 325.26 seconds. The same
group retains identity, source revision, alias invalidation, migration, publication and rollback
coverage. This establishes the focused boundary; current candidate gates follow separately.

Physical reviewability: the catalog facade has 4369 lines including test support and no inline test
bodies. The row-writing owner has 138 lines including its test-module declaration, no inline test
bodies, and 169 dedicated test lines containing two cases. Remaining identity-policy responsibilities
stay explicit in the roadmap. Evidence is `red-regression.log`, `focused-catalog.log` and
`extraction-proof.json` under `.build/r2c-staging-cache-profile-20261009/`.

The candidate's standalone lint passes, including warnings-denied Clippy and Dart analysis. The
initial local wrapper then starts Daily in that same PowerShell process and fails its compiler
ownership fault check before product tests. The first guardrail invocation has already loaded all
four native types; `Initialize-AmeR2cRNativeTypes` returns for that complete type set before reaching
the bootstrap fault point. The guardrail expects a fresh type scope, so its second invocation cannot
observe the requested fault. `candidate-quality.json` retains the passed lint and failed 12.32-second
Daily attempt. The corrected ignored wrapper admits one canonical gate per fresh PowerShell process,
rejects an already loaded native type scope, and checks all 807 frozen source/test/tool files before
and after each gate. Product and quality-tool source remain unchanged during this correction.

The corrected fresh-process Daily passes in 2057.99 seconds: 1685 Rust library cases and three
broker lifecycle cases, all 107 Flutter test files, controlled Windows scan integration, all ten
native UIA phases with normal owned-process exit, and 16 asynchronous bridge contracts with matching
hashes. The Rust suite retains 23 opt-in cases as ignored; explicit performance and external-input
acceptance remain separate obligations. All 807 candidate source/test/tool files retain their frozen
hashes. This result is recorded in `candidate-daily-fresh.json` and its transcript.

The optimized 10000-image scan gate also passes: cold import 25469 ms, warm update 15910 ms,
pause acknowledgement eight milliseconds, resume 24150 ms and cancellation 123 ms. The single
explicit case completes in 82.34 seconds with no ignored test. Warm staging totals 8967 ms,
validation 2531 ms and publication 1799 ms; compatible inspection reuse totals about two
milliseconds. Its fresh optimized build has no warning and the 807 frozen inputs remain unchanged.
This is generated-fixture performance evidence, not retained-library or Release-window acceptance.
Evidence is `candidate-synthetic-scan-fresh.json` and its transcript, with raw owned-process receipts
under `.dart_tool/performance_synthetic/8e4645006ce74858b2cf35b8e2101b4a/`.

The unsigned Windows gate passes in 284.07 seconds: fresh Release application and broker payloads,
three engine-free native window cases, two actual Debug-engine retirement cases with clean process
and Job closure, and the isolated Release-DLL/Windows-channel smoke without catalog access. The
807 frozen inputs remain unchanged. Its source attribution is the dirty candidate over
`146490b36482e27ef9cd87b817f1c1a217c325a2`; the source-hash binding preserves that distinction from
an older committed tree. `candidate-unsigned-windows-fresh.json` and `build/quality-unsigned-windows/evidence.json`
own these results. They do not establish populated Release-window or signed installed-service acceptance.

### Optimized retained update

After those gates, one optimized headless ordinary update uses a fresh copy of the normally closed
64490-location publication. Admission requires that exact source, one completed root and no running
or paused scan. The source/test/tool manifest, executable, helper, root scope and initial derived
catalog are hash-bound. Temporary test attachment is restored byte-for-byte before execution, and
the 807 candidate inputs remain unchanged before and after the run. Bounds are 120000 items and
entries, cancellation after 300 seconds, a 400-second owned-process lifetime, 768 MiB working set
and a two-GiB host memory reserve. This is no new import or unfinished-scan continuation.

The update completes in 147916 ms with all 64490 images and no limit reached. Its 678
`image_format_unsupported` issues have the same classification/count as the baseline. Finalization
begins at 122127 ms, all source validation is reported at 130944 ms, and terminal delivery follows
at 147916 ms. The single test passes in 147.97 seconds; the complete owned process takes 148093 ms,
exits zero and records a 25321472-byte peak working set.

| Operation timer | Calls | Accumulated milliseconds |
| --- | ---: | ---: |
| Source discovery | 146998 | 39356.881 |
| Prior record selection | 65168 | 5122.056 |
| Media inspection or reuse | 65168 | 412.940 |
| Location staging | 64490 | 24709.012 |
| Directory persistence | 27038 | 25941.596 |
| Checkpoint persistence | 548 | 12159.905 |

Accounted operations total 107702392 microseconds; the remaining 40240971 microseconds include work
outside these six timers. Directory/checkpoint timers include their buffered writes. Timer calls
are not a file-count projection, and these categories do not isolate SQL from filesystem cost.

The closed comparison matches all 64490 rows across all 21 identity/source fields, retaining ordered
payload SHA256 `2aeebf1731ce31e21c92de245550074a88c1522ba6d928a492ef4426a1118a95` on both sides.
Integrity and foreign-key checks pass. All eight original, predecessor and baseline catalog/WAL
components retain their hashes. Postchecks open no source media; the runtime uses the existing
read-only/no-hydration adapters. No whole-library content hash or Release-window timing is inferred.

Evidence is `.build/r2c-warm-release-20261009/`, including `prepared.json`, `executable.json`,
`measure.json`, `summary.json` and `issue-summary.json`. The earlier Debug headless/native timings
differ in build mode and runtime participation, so this is not an isolated before/after speedup.
An explicit whole-directory update still takes about two and a half minutes on this cohort.
Retained idle-browsing variants, Release-window timing and combined-source disposition remain open.

### Retained update after identity-query reuse

The 2026-10-10 optimized update uses the same bounded method and a new copy of the completed
64490-location publication. All 810 candidate source/test/tool inputs are bound and unchanged.
It completes in 159127 ms with 64488 locations, the same 678 unsupported-format issues and no
limit reached. Finalization begins at 131017 ms, full source validation is reported at 141829 ms,
and terminal delivery follows 17298 ms later. The owned process takes 159321 ms and records a
25423872-byte peak working set. This does not establish a whole-update speedup.

The diagnostic's old 64490-location assertion fails and remains a failed result. A bounded read-only
comparison subsequently finds exactly two omitted baseline paths and no added location. Attribute
lookups confirm both paths are absent; their deletion was confirmed by the source owner. All 64488
retained locations preserve all 21 identity/source fields. Closed integrity and foreign-key checks
pass, and all eight protected original/predecessor catalog components retain their hashes. This
disposes of the count discrepancy; it does not change the original assertion verdict or require
another unchanged scan. The next current-source cohort is 64488, subject to subsequent source changes.

| Operation timer | Calls | Accumulated milliseconds |
| --- | ---: | ---: |
| Source discovery | 146994 | 43641.608 |
| Prior record selection | 65166 | 5388.646 |
| Media inspection or reuse | 65166 | 428.858 |
| Location staging | 64488 | 23480.035 |
| Directory persistence | 27038 | 29900.725 |
| Checkpoint persistence | 548 | 12833.927 |

Accounted operations total 115673801 microseconds; unaccounted work totals 43483715 microseconds.
Directory/checkpoint timers include their buffered writes. Evidence remains under ignored
`.build/r2c-warm-release-20261010/`, including `prepared.json`, `executable.json`, `measure.json`,
the failed test output and `membership-result.json`. Temporary diagnostic source is restored before
execution. No source content is read by the membership comparison and no source mutation is performed.

### Native publication phase attribution

A catalog-only diagnostic uses a new 443531264-byte copy of the completed 64488-location derivative.
It admits a new scan in the copied catalog and stages the same existing observations without opening
source paths or inventing a filesystem-validation proof. Temporary timers surround the existing
publication phases; the final transaction boundary executes ROLLBACK. Instrumentation is restored
before execution and all 810 source/test/tool inputs retain their verified hashes.

| Native phase | Milliseconds |
| --- | ---: |
| Identity reconciliation | 3195.668 |
| Staged count | 48.337 |
| Retained handoffs | 0.701 |
| Projection replacement, total | 12967.126 |
| — Preview reference replacement, nested | 975.801 |
| — Previous projection DELETE, nested | 11322.452 |
| — Orphan asset cleanup, nested | 668.183 |
| Final rollback, excluding commit | 1.592 |

The complete publication call takes 16214 ms; nested costs are already included in replacement.
The owned process passes its one explicit test in 75330 ms with a 23265280-byte peak working set.
All 64488 active rows match the closed baseline across every location column. Native full integrity,
closed quick-check and foreign-key checks pass; all 11 protected catalog components are unchanged.
No publication is committed and no source path/media is opened. This identifies previous-projection
retirement as the largest measured phase in this catalog workload, not the cause of all 159127 ms
in the actual source update. Directory traversal state, filesystem proof and commit are not recreated.

The first diagnostic build rejects an attempted consuming rollback through the priority transaction
wrapper; the corrected diagnostic issues explicit ROLLBACK through that wrapper's connection. Product
source is restored after both builds. The first postcheck's full-row EXCEPT exceeds its 45-second
budget. Indexed full-field comparison succeeds, but repeating full integrity afterward exceeds the
same budget. The final postcheck retains the passing native full-integrity assertion, verifies the
closed payload through its primary key, and completes closed quick-check/foreign keys in the original
bound. These verifier failures remain distinct from the passing measured transaction. Receipts,
timers, source/executable bindings and `verified.json` remain in ignored
`.build/r2c-publication-phases-20261010/`.

The follow-up rollback-only comparison changes only the connection's page-cache suggestion. SQLite
3.50.4 in the Python probe reports 21296.747/16974.588 ms for default/16 MiB, with WAL extents of
295589432/292482952 bytes. The same query is then compiled against the exact rusqlite 0.40.1 and
bundled SQLite 3.53.2 libraries from the successful native diagnostic build. Four fresh-connection
arms run in default/16-MiB/16-MiB/default order: **9077.588, 14984.668, 14537.144, 9256.069 ms**.
Every arm deletes and rolls back exactly 64488 rows with WAL/FULL, foreign keys, enabled spilling,
4096-byte pages and secure-delete disabled. Both 64488-row projections, the copied database bytes,
bound input files and all 810 product inputs are unchanged. Closed quick-check and foreign keys pass.
The first wrapper's empty argument list is refused before measurement; the corrected invocation
supplies and validates its diagnostic scope argument. No product cache policy is changed: the pinned
engine rejects the larger-cache hypothesis. Dependency/helper/binary bindings, owned-process receipts
and all four outputs remain under the same ignored directory. This query experiment cannot establish
retained-update speedup or close the wider latency and client-acceptance duties.

### Generated directory-persistence attribution

The subsequent generated-only diagnostic isolates directory operations from buffered image writes.
It creates one empty namespace and a fresh catalog, enqueues 2048 directory names, and persists and
reads back four names per directory through the current catalog methods. No source enumeration,
media read or real catalog participates. The optimized build temporarily adds a test-only timer
around the existing transaction commit and attaches the generated test. Before execution the
catalog source is restored byte-for-byte; all 702 bound product/tool inputs remain unchanged after
execution. The 390-second build has no warning.

| Operation | Calls | Total milliseconds | Nested commit milliseconds |
| --- | ---: | ---: | ---: |
| Enqueue directory | 2048 | 1104.792 | 1010.557 |
| Claim directory | 2048 | 1189.342 | 1047.242 |
| Read enumeration state | 2048 | 67.620 | — |
| Stage four names | 2048 | 995.310 | 940.677 |
| Complete enumeration | 2048 | 986.744 | 896.474 |
| Read bounded name window | 2048 | 121.617 | — |
| Complete directory | 2048 | 1076.868 | 995.042 |

The complete measured loop takes 5558 ms, with exactly 10240 commits under unchanged WAL/FULL
durability. Nested commit time totals 4889.992 ms and is already included in the operation totals.
Closed verification finds all 8192 names accounted for in the checkpoint, no pending directory or
roster rows, no assets/locations, and passing integrity and foreign-key checks. One explicitly
selected test passes with no ignored execution; its entire test takes 5.72 seconds, within the
90-second measured and 120-second owned-process bounds.

This identifies commit cost within the generated directory-only workload. It does not assign the
retained update's 25941.596-ms directory timer, which also contains buffered image writes, to those
commits. In particular, the separate enumeration-completion boundary accounts for 896.474 ms of
commit time across 2048 directories; this result alone does not justify expanding traversal ports
or changing durability to address the whole 147916-ms retained update. No production policy or
checkpoint bound is changed. Preserve this single-run and workload-size limitation; the retained
warm-update cost remains open rather than being closed by an unrelated microbenchmark.

Evidence remains under ignored `.build/r2c-directory-cost-20261009/`, including source/executable
bindings, the restored-source snapshot, owned build/test receipts and per-operation output.

## Idle journal opening admission on 2026-10-14

A retained-library Release update reached 64,451/64,451 finalization without publishing before its
300-second process bound. A subsequent Debug diagnosis recorded 125 publication attempts entering
the payload proof and zero completed publications. Ordinary journal pages supplied opening boundaries
even when no recovery window needed writing. Their attachment method requested Journal writer
priority before checking that condition, repeatedly preempting the Recovery publication transaction.

`c7737c6` moves this responsibility into `persistent_journal/recovery_opening.rs`: a deferred read
snapshot returns unnecessary or already-attached outcomes without writer admission. A missing window
still requests Journal priority and rechecks root generation, capability, checkpoint identity,
authority and existing window inside the write transaction. No schema, durability, publication-proof
or source-access policy changes. Temporary diagnostic instrumentation was removed before commit.
The owning module has 158 production lines and no inline tests; its dedicated regression file has
209 lines. The facade retains 3,440 production and 4,740 inline-test lines; this slice adds no other
responsibility to that retained decomposition debt.

The same retained environment's corrected unsigned Release completed its update in 141,770 ms,
checking 70,078 entries and publishing 64,451 locations with 678 inspection issues. Its interface
displayed completion and synchronized state, and the catalog held supported/current journal state
without a failure code. A fresh metadata-only enumeration of the two reported October subtrees found
51 files, all present in the published catalog. The process exited normally with code zero; its
208,798-ms lifetime is not the update latency. Original profile/catalog hashes were restored.

Four regressions cover no-op and idempotent reads during a held publication, real attachment priority,
and a conflicting attachment arriving after the read hint. The first hosted run exposed two missing
test imports; `9355b87` corrects those imports without changing production behavior. Hosted verification
for that revision remains pending at this checkpoint. No local CI or unit suite was run.

This comparison diagnoses the retained failure; it is not fresh-database acceptance. After normal
retirement and restoration checks, cleanup removed the obsolete Debug output, downloaded failure
logs and this environment's catalog/preview cache, totaling 687,092,599 bytes. The current Release,
reusable source corpus, original profile backups and small result records remain. Subsequent
independent acceptance cases reuse this environment with a fresh database and empty preview cache.

## Fresh real-library import on 2026-10-14

The same isolated environment started without a catalog or preview-cache directory. The ordinary
unsigned Release retains the `c7737c6` production inputs; the subsequent test-import and documentation
commits do not change that runtime. Actual picker input imported the combined destination for
`local-primary` and `cloud-primary`, under a 120,000-entry, 600-second process and 2-GiB client bound.
No source copy, service installation or local CI run was required.

The single scan completed in **228,654 ms**, checking 70,078 entries and publishing 64,451 locations.
There were zero metadata-inventory runs. Its 955 inspection issues comprise 678 unsupported formats,
181 invalid capture times and 96 metadata-parse failures; issue count is not a missing-image count.
The interface displayed completion, current October previews and synchronized state. Closed catalog
inspection confirmed supported/current journal state with no failure code. A fresh attribute-only
enumeration of the two reported October subtrees found 51 files, all present in the active scan.

The process exited normally with code zero after 592,939 ms, including picker interaction and
observation time; that lifetime is not import latency. The launcher restored and hash-verified the
original profile and catalog. After process retirement, exact-path and reparse checks, cleanup removed
224,510,555 bytes of disposable catalog/cache data while retaining the reusable source corpus,
current Release and compact result records. Source-media byte integrity and instrumented no-hydration
claims are not inferred from the catalog checks. This case establishes fresh import and initial
preview display, not the remaining independent update, viewer or browsing variants.

Hosted run [38552243492](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38552243492) passes
the unsigned Release, Flutter tests, Windows scan, identity-publication and supported-format jobs at
this checkpoint, while other jobs are still running. Its Windows Accessibility job fails the existing
eight-second `application-ready` probe deadline at 8,019 ms. Both assembly-validation marker sets are
complete; the last persisted probe record remains at `loading-uia-client`, 3,112 ms, without a recorded
window traversal. This does not attribute the timeout to application readiness or to assembly loading.
Keep the failure open without weakening its deadline or native assertions.

## Fresh update and cancellation continuation on 2026-10-14

The same isolated environment started with an absent database and preview cache, then imported the
actual combined library before updating it. The unchanged `c7737c6` Release checked 70,078 entries
and retained 64,451 published locations. The baseline's catalog interval was 226,287 ms; two complete
updates recorded 169,153 ms and 158,673 ms. These timestamps precede transaction commit and are not
exact UI completion latency. The first update preserved the selected October scroll position and
visible previews through completion. The second completed before the intended Cancel input, so it
does not prove cancellation. This client exited normally with code zero and restored the original
profile/catalog; its 817,598-ms lifetime includes all three scans and observation.

A continuous reopened client then ran a separate update. Native Cancel at final validation was
accepted immediately; a 200-ms read-only observer saw committed `cancelled` state 11,688 ms after
the input. Existing photos stayed visible. The catalog's terminal timestamp was assigned before
the cleanup transaction committed, so that timestamp alone understates cancellation cost. The
client was not closed promptly after observation and reached its 300-second test guard; that
lifetime is failed, not a normal-retirement pass.

The following continuous reopen displayed the populated gallery, previews and synchronized state
within 2,694 ms of process start. Closed inspection retained the same four scan records, zero
metadata inventories, 64,451 active locations, zero cancelled staging locations and supported/current
journal state without a failure. It exited normally after 12,240 ms with code zero. Original settings,
preferences and catalog components matched their saved hashes. Exact-path and reparse checks then
allowed cleanup of 458,150,326 bytes of isolated database/cache data. The source corpus, `build/test`,
current Release and compact results remain. This continuation does not prove instrumented zero
source calls, every browsing transition, or acceptable cancellation latency.

Source inspection finds a second, unscoped orphan sweep after the scan-staging owner already
retires its affected assets transactionally. Explicit cancellation and interrupted-update retirement
now rely on that scoped cleanup, avoiding another traversal of unrelated library assets. Active
locations and both durable handoff protections remain unchanged; physical staging deletion still
finishes in the same terminal transaction. Two regressions compare actual retirement VM work with
128 versus 4,096 unrelated assets, require their preservation and verify a full catalog reopen.
The catalog facade has 3,774 source lines with no inline tests; the lifecycle test owner has 497
lines. Owned Rust formatting and diff checks pass; Actions owns test execution. The observed
11,688-ms cancellation cost is not considered resolved until the changed client is measured.

Hosted run [38552243492](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38552243492) finished
with 1,869 Rust tests passing and one failure: the 4,096-file coordinator test counted two journal
reads while requiring one empty closing replay. Its `AdvancingJournalSession` supplied two `20`
boundaries, then silently returned `21` on later queries, admitting ordinary catch-up into the
same counter. The test now reuses `NoChangeJournalSession`, which stays at `20` and rejects nonempty
reads. The exact-one-read assertion, 4,096 files, 4,095-entry page, 300-second deadline and retirement
checks remain. Additional FULL-reopen assertions require both checkpoint cursors to remain at the
opening boundary with current continuity. Production behavior is unchanged; the owner retains
3,957 production and 10,490 inline-test lines, with no added dedicated test file. Formatting and
diff checks run locally; GitHub Actions executes the regression and full CI. The separate native
Accessibility timeout remains open.

## Fresh real-library browsing on 2026-10-14

The reusable isolated environment again started without a database or preview cache. Its unchanged
`c7737c6` Release imported the actual combined library under the 120,000-entry, 900-second and 2-GiB
client bounds. The single scan checked 70,078 entries and published 64,451 locations; its catalog
interval was 220,059 ms, assigned before commit. There were no metadata inventories, journal state
was supported/current without a failure code, and all 51 previously enumerated October files were
present. This membership comparison does not represent another current source enumeration.

Native middle-rail navigation and an upward 1,000-pixel scroll initially showed unpainted slots with
the shared top loading line. Both populated automatically without additional input. The next
populated screenshots were 6,265 and 8,458 ms after input; these are discrete observation upper
bounds, not measured page latency. Reversing by 1,000 pixels restored the same visible rows. Opening
the small original at position 27,933 of 64,451 showed 100-percent scale, and Escape returned to the
same gallery position. No Retry feedback appeared in the sampled frames; continuous transient-error
coverage and the exact historical Retry incident remain unverified.

The unpainted pending slots agree with ADR 0014's existing loading treatment. The product contract's
older painted-placeholder wording is aligned with that decision; no rendering or query behavior
changes. The process exited normally with code zero after 418,580 ms. Original settings, preferences,
their recovery backups and original catalog components matched their saved hashes. After process,
exact-path and reparse checks, cleanup removed 225,435,919 bytes of isolated catalog/cache data.
The reusable corpus, current Release and compact results remain; source-byte integrity and
instrumented no-hydration claims are not inferred from these checks.

Following hosted run [38554959576](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38554959576)
passes the unchanged Accessibility gate. This preserves the earlier timeout without claiming its
cause was fixed. At this checkpoint its Rust job still runs, while the `8a671a6` empty-journal fixture
correction waits for its main-branch CI run. No local CI or unit suite was run.

## Scoped cancellation cache correction on 2026-10-15

The `99888cc` Release reused the isolated environment for a fresh actual-library baseline and update.
Late Cancel committed after 15,761 ms; the 64,451 published locations remained and cancelled staging
was empty. The client exited normally with code zero and restored the original profile/catalog.
Hosted run [38581201784](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38581201784) passes
Rust, including scoped retirement, and unsigned Release. Flutter viewer fixtures fail separately.

A bounded, rollback-only SQL comparison on the retired isolated catalog attributes most cleanup
time to deleting 64,451 staged locations: 7,862 ms with a 2-MiB pager suggestion and 2,562 ms with
64 MiB applied only during cleanup. Collection took 897 and 501 ms respectively. These measurements
exclude commit and client feedback. Each diagnostic transaction rolled back and preserved row counts.

The retirement owner now scopes that cache suggestion to its atomic deletion and restores the prior
value on success or SQL failure. SQLite's [cache-size contract](https://www.sqlite.org/pragma.html#pragma_cache_size)
allocates on demand; the suggestion is connection-local, not a total-process memory cap. Regressions
cover byte/page-unit restoration, injected deletion failure, terminal rollback, retry and FULL reopen.
Staging and retirement have 62 and 77 production lines with no inline tests; lifecycle tests have
575 lines. Owned formatting and diff checks are local; Actions runs CI.

The changed ordinary Debug client reused the retained baseline in `build/test/derived`. Its update
reached 69,142 visited entries and 63,669 accepted locations. Native Cancel immediately displayed
Stopping; a read-only observer saw the committed cancelled state 15,567 ms after input. All 64,452
published locations remained and cancelled staging was empty. The process exited normally after
314,101 ms and restored the original settings, preferences and catalog hashes. This Debug result
still exposes slow cancellation; the earlier 15,761-ms measurement used Release and is not a
same-build speed comparison.

A same-storage diagnostic reconstructed 63,669 staged locations and 3,300 directory identities
without reading images. The native interrupted-update recovery call took 26,084 ms. An experimental
optimization of the SQLite dependency took 28,982 ms with the same row counts and was removed.
Those calls include catalog opening and retirement and do not isolate deletion. The separate SQL
measurement records 3,680 ms for staged-location deletion and 3,461 ms for commit. Continue from
native per-phase attribution of retirement and commit; do not repeat the compiler experiment or
infer client completion from the pre-commit terminal timestamp.

After rebuilding Debug without that experiment, process retirement and original-profile hashes
were verified. Exact-path and reparse checks preceded removal of 50 disposable catalog/cache,
profile-backup, log and diagnostic-script files totaling 429,824,941 bytes. The reusable `build/test`
sources, templates, fixture and current builds remain.

## Bounded cancellation retirement on 2026-10-15

Native phase timing attributes the remaining Debug delay to deleting staged locations and
committing their index changes. On the retained 63,669-location fixture, deletion reached 12,402 ms,
orphan cleanup 13,682 ms and commit 17,352 ms, excluding 2,491 ms of opening validation. Increasing
the temporary cache to 256 MiB still leaves a 13,971-ms complete native call; that experiment was
removed. The production suggestion remains 64 MiB.

Foreground cancellation now retires the task and clears its continuation authority in one
transaction, deleting at most 128 staged locations. The existing reclamation worker drains the
remaining terminal staging in bounded transactions and resumes after reopening. Published locations,
running or paused tasks, other asset references and handoffs remain protected. Six adapter
regressions cover FULL reopen, immediate next-task admission, retained-first-import cancellation,
interrupted update retirement, batch rollback/retry, contention/preemption and bounded foreground
SQL work. CI runs these regressions; no local unit suite was run.

The first changed native run returns after 2,681 ms including full catalog validation and commits
exactly 128 deletions. Reopening clears the remaining rows but exceeds the 180-second deadline
during file reclamation. The old SQLite call used `execute_batch` for `incremental_vacuum`, stepping
only the first result row and reclaiming one page instead of the requested batch. The dedicated
adapter now consumes all rows with the existing progress handler. Its regression requires exactly
32 reclaimed pages when more than 32 are free. The subsequent 113-ms continuation had already
fallen below the reclamation threshold and is not a valid whole-cleanup timing comparison.

A fresh reconstruction of the same 63,669 staged locations and 3,300 directory identities verifies
the corrected native path. Foreground retirement returns in 2,879 ms, cancellation of the background
worker completes in 116 ms, and 63,541 staged rows remain durably terminal. A new process opens and
validates in 2,426 ms, then completes staging cleanup and file reclamation in 100,780 ms. Both
processes exit with code zero. All 64,451 published locations remain and terminal staging is empty;
peak Dart VM RSS is 254.93 MiB. This diagnostic only changes the isolated catalog and reads no source
images. Earlier command-launcher working-set samples do not measure the Dart VM and are excluded.

Affected owner sizes (production / inline tests / dedicated tests): reclamation adapter 403 / 334 / 0,
incremental-vacuum owner 10 / 0 / 0, staged-cleanup adapter 39 / 0 / 0, scan lifecycle 187 / 0 / 936,
staging facade 64 / 0 / 0 and retirement owner 134 / 0 / 0. The application reclamation facade has
458 production lines and its new staging loop 24; scan application facade has 749, retained
cancellation 28 and terminal retirement 52, with no inline test modules. The shared ports file has
1,155 lines; this change adds only the bounded staging-cleanup contract. Existing application tests
stay with their current owners. Owned Rust formatting, whitespace checks and the native Debug build
pass; Actions owns lint and test execution.

The actual Debug window reuses the same isolated environment after resetting its derived catalog
and preview cache and restoring the retained update baseline. Existing incremental continuity adds
one current location at startup, leaving 64,452 published locations before explicit update. At
68,266 visited entries and 63,482 accepted locations, Cancel commits after 344 ms. The window
shows Stopping followed by Cancelled, retains visible images and returns the folder to synchronized
state. Clicking Update again starts a new scan after 288 ms; the old cancelled scan still has
47,098 staged locations while the new task progresses. Cancelling the new task at 8,158 visited
entries and 7,848 accepted locations commits after 455 ms. These are input-to-committed-observation
measurements with a 100-ms read-only observer, not sub-frame rendering timings.

The client exits normally with code zero after 256,415 ms, including preparation and interaction,
while background cleanup is still permitted to resume on a later opening. The original settings,
preferences and catalog components match their pre-run hashes. The known actual root is read-only,
with a 120,000-entry, ten-minute and 2-GiB process bound; only visible previews are decoded and cloud
placeholders remain unhydrated. No source files, user copy or layout are changed by this correction.

After verifying process exit and restored hashes, explicit contained-path and reparse checks precede
removal of 54 disposable files totaling 455,630,955 bytes. The reusable sources, templates, manifest,
oracle and fixture remain in `build/test`, alongside the current Debug and Release build outputs.

### Ordinary Release late cancellation and reopening on 2026-10-15

The current unsigned Release repeats this boundary against the known combined destination of
local-primary/cloud-primary, using fresh derived storage in `build/test`. The read-only run is
bounded to 120,000 entries, fifteen minutes and 2 GiB; only requested previews read media content.
Its baseline imports 70,078 entries and 64,451 locations in 282,951 ms. The following update reaches
the same entry/location counts before Cancel. Input to the transaction's cancellation timestamp
is 233 ms; the 100-ms read-only observer first sees the committed state after 762 ms. The window
shows Stopping, then Cancelled, with the existing gallery preserved.

The next Update starts 218 ms after confirmation while 39,875 old staged locations remain.
Cancelling at 19,250 visited entries and 18,668 accepted locations commits in 248 ms; the first
observer confirmation is at 717 ms. These transaction and observation timings have distinct
boundaries and do not measure a rendered frame. The first process exits normally, and the second
opens the same populated catalog with Current coverage and no additional scan or inventory.
Background staging reaches zero 88,654 ms after the second cancellation, including the closed
interval. All 1,844 observations during update, cancellation and reopening retain exactly 64,451
published locations and Current journal coverage. Ordered path, asset, source-revision and file-
identity tuples match the baseline; all 128 sampled source metadata/identity records are unchanged.

Both processes exit with code zero, at cumulative runner times of 780,117 and 870,418 ms. Original
preferences, settings and catalog hashes match after restoration. Cleanup removes 44 disposable
data files (321,283,596 bytes) and eleven diagnostics/helper files (17,855,407 bytes), leaving only
the derived fixture marker. The ordinary user Debug is restored at 65,130 images and Synchronized.

## Native picker discovery in hosted verification

Run [38647442080](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38647442080) completes
three Windows scan scenarios but fails the picker-cancellation test's final semantics-handle check.
The helper traversed the Flutter window through UI Automation to find the native dialog. The pinned
Flutter binding retains a platform-owned semantics handle when that traversal enables accessibility;
the test then observes a handle created after its starting baseline. Native dialog discovery now uses
[EnumWindows](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumwindows),
filtered to the target process, visible `#32770` class and a single match. Existing control IDs,
button invocation, action deadlines and picker-result assertions remain. A new boundary assertion
requires the cancellation helper to preserve the actual platform semantics-enabled state. Dedicated
accessibility tests keep their existing real UI Automation coverage. PowerShell parsing, embedded
C# compilation, Dart formatting and diff checks pass; the native workflow runs in Actions.

## Preview replacement sharing conflict on 2026-10-15

The ordinary Debug client reuses `build/test` and its 10,000 varied images. After a completed import
and normal exit, three derived previews are replaced with invalid bytes. Two recover automatically;
the 2160-by-3840 source's preview reports `preview_publish_failed` with Windows error 32. Its new
encoding already exists; the atomic installation fails. Manual Retry then restores it. All three
source hashes and modification times remain unchanged. The blocking reader's process is unidentified.

The Windows installation owner now retries only sharing violations, using the same staged encoding
and exclusive backup. Three waits total at most 140 ms. Terminal and partial-replacement errors keep
their existing rollback and accounting paths. Five native regressions cover target/staging readers,
persistent contention, bounded attempts and unrelated errors; Actions executes them.

The changed client reuses the same catalog and the same three corrupted previews. A read-only handle
blocks one cache replacement until the Debug log observes the actual sharing conflict, then retires.
Two installations take one 20-ms retry each. All three preview hashes match their original encodings,
and source hashes/times remain unchanged. The window still retains a failed image stream on one
already repaired preview; that presentation defect remains open. The client exits normally after
72,909 ms and restores the original profile/catalog hashes. The native build, owned formatting and
diff checks pass; this result establishes cache recovery, with visible recovery still to correct.

The remaining display failure comes from reusing the same `Image` state after same-path repair.
The pinned Flutter implementation only resolves another stream when its image provider changes;
a repair can complete before a progress frame removes the old failed Image. The tile now keys the
image by its existing retry generation and evicts the path's decoded cache again after successful
repair. Existing generation checks retire stale completions. A widget regression replaces corrupt
bytes at the same path and completes repair immediately, then requires decoded pixels, one repair
request, no remaining failure/progress control and unchanged tile dimensions.

The final ordinary Debug comparison reuses the same catalog and three preview paths, including the
controlled native reader. The three automatic repairs finish in 302, 311 and 705 ms including queue
wait; the visible window shows all three images with no Reload Preview control. No manual Retry is
used. All selected preview hashes match their pre-corruption encodings, and source hashes and times
remain unchanged. Normal exit takes place after 62,679 ms; original profile/catalog hashes match.
After process retirement and reparse/containment checks, cleanup removes 53 disposable files totaling
33,677,610 bytes. The corpus, templates, manifest, oracle, fixture and current builds remain.

Affected sizes: the installation owner has 311 non-inline-test lines including test hooks, 174 inline
test lines and 127 dedicated sharing-conflict test lines. The photo tile has 452 production lines,
no inline tests and a 921-line existing dedicated test file. Owned formatting, diff inspection and
the Debug build pass; new Rust and widget regressions run in Actions.

## Retained cold-preview and publication observation on 2026-10-15

The ordinary Debug build containing `a9e995b` and `029462e` opens a byte-verified copy of the
retained catalog in `build/test/derived`, with preview references reset only in that copy and an
empty 256-MiB cache. The original catalog and media remain untouched. This targets cold middle
navigation and idle position during publication; it does not repeat the completed fresh import.
The run is bounded to 15 minutes and a 2-GiB client working set.

Two middle destinations, downward scrolling and reversal display preview pixels without a Retry
control. The 48 logged slow native preview completions all return ready. Two logged visible comic
previews take 834 and 841 ms including 577 and 578 ms of queue wait. Later off-screen prefetch work
takes up to 10,063 ms; these logs do not time every visible image's first decoded frame.

The retained baseline-required inventory also completes during the same lifetime: 70,078 entries
and 2,446 candidates, with published membership increasing from 64,489 to 65,130. The existing scan
count stays at one. Eighty-eight status polls have a 7-ms median and 55-ms maximum. The final client
and persisted state report synchronized, with no pending work, retries or gaps. Both October
subtrees exactly match their current source sets, 9 and 42 entries. The selected idle image group
and visible position remain unchanged across publication; no Retry control appears in the sampled
states. The historical complete transient Retry/jump remains unattributed.

The client exits normally after 452,724 ms. All 128 sampled source sizes, modification times and
file attributes match, and original settings, preferences and catalog hashes are restored and
verified. Cleanup removes 232 disposable derived files totaling 460,870,655 bytes plus the temporary
preparation helper. Reusable sources, templates and the derived fixture marker remain.

## Catalog short-name reuse correction on 2026-10-15

Hosted run `38656143904` at `2484086` reports 1,898 Rust passes, one bounded-retirement failure
already corrected by `a9e995b`, and 24 ignored tests. Resumed-import identity, availability contracts,
the mixed-load connection-lifetime workload and the other applicable jobs pass. All 177 slow
catalog-identity diagnostics report a short-name namespace mismatch. That mismatch forces a
temporary metadata open and close for each proof; the longest close in this run is 1,273,056 us.
The earlier 9,209,153-us close remains the triggering failure, without a Windows-internal attribution.

The catalog namespace owner now reuses the existing Windows long-name expansion before comparing
with the pinned physical namespace. Expansion is fresh on every proof; the held file still supplies
its current final path and full File ID. Expansion failure and unresolved aliases retain the existing
fresh-open path. No case-folded identity, cached alias mapping or relaxed deadline is introduced.
The raw TEMP-path regression retains the caller spelling and requires 32 fresh identity reads,
zero extra temporary opens and one full schema validation. Existing parent-rename and junction-retarget
regressions cover replacement, shared hardlink identity and in-operation retargeting in hosted CI.

Affected sizes: namespace owner 149 production lines and 35 dedicated lexical-test lines; path-identity
tests 350 lines. The existing local-files adapter changes only the expansion helper's visibility
(3,695 pre-inline-test lines and 3,493 inline-test lines). Owned formatting and diff checks pass;
the updated hosted checks are pending. The ordinary Debug client opens its catalog through an
actual DOS short-directory spelling, imports the reusable 10,000-image corpus and reopens current
in 854 ms without another scan or inventory. The following update correction is verified through
the same catalog and spelling.

## Cursor-only journal writes during an update on 2026-10-15

The short-path Debug workflow reaches 10,000 of 10,000 validation results but cannot finish its
explicit update. The first attempt is cancelled after 260,858 ms with the previous 10,000 published
locations intact. A second diagnostic attempt logs 181 publication preemptions while journal pages
contain zero source observations and only advance their checkpoint. Cursor bookkeeping repeatedly
interrupts and rolls back the lower-priority atomic scan publication.

Commit `48dbe7f` admits cursor-only volume batches through the ordinary Recovery writer queue.
Image intents, directory changes, pending or consumed renames and cross-root lineage retain Journal
priority. Root, range and checkpoint validation still execute inside the same atomic transaction.
The new hosted regressions exercise actual SQLite admission, retained cursor state while waiting,
source-change preemption and directory-only priority.

The rebuilt ordinary Debug client reuses the same isolated catalog and cache in `build/test/derived`.
Its update completes in 27,083 ms with zero publication-preemption diagnostics. All 10,000 locations
remain present; the task disappears and the source displays Synchronized. The frozen source corpus
retains all 10,000 recorded sizes and modification times.

After normal close, three temporary copies of existing textured fixtures are added to the same test
root: 640 by 480, 800 by 1,200 and 1,024 square, with distinct sizes and modification dates. Reopening
automatically publishes exactly 10,003 locations and three ready previews. Upward scrolling shows
all three new pictures while preserving the preceding position until that input. Scan count stays
at four, including the two earlier cancelled attempts, and inventory count stays at zero. The final
sync state is current with no pending work, retries or gaps. Original and copied fixture hashes match.

Both sessions exit normally, and original preferences, settings and catalog hashes are restored.
Cleanup removes 45 derived files totaling 64,863,717 bytes, nine diagnostic/handshake files totaling
36,979 bytes, three temporary source copies and three preparation helpers. The reusable corpus,
templates, fixture marker and current Debug build remain. CI runs in GitHub Actions; no local test
suite is repeated.

## Recovery dispatch waiting behind real publication on 2026-10-15

The ordinary-token Debug run copies the retained catalog into the same `build/test/derived` storage,
clears preview references only in the copy and uses a fresh 256-MiB cache. The source scope is the
known combined local-primary directory: metadata and requested previews, at most 120,000 entries,
300 seconds for the explicit update, 900 seconds for the client and a 2-GiB working-set bound.

The explicit update completes 70,079 entries, 64,452 accepted images and 678 issues in 221,570 ms.
There are no publication-preemption diagnostics. During the update, middle navigation, upward input
and an 8,000-pixel reversal display previews; the 29 sampled image identities remain unchanged across
publication, with no sampled Retry. The existing baseline inventory continues afterward and restores
65,130 published locations. Both current October sets match exactly, 9 and 42 files. All 128 sampled
source sizes, modification times and attributes remain unchanged. The client exits normally after
655,497 ms and restores original profile/catalog hashes; its derived state is retained for the correction.

The run records 121 status polls with an 11-ms median and a 25,529-ms maximum. Native timing places
25,521 ms in scheduling, outside catalog opening, observation, worker retirement and projection.
The recovery-control lease synchronously acquires the same writer used by atomic scan publication.
Its admission now tries once and leaves the durable task unchanged when another writer owns the
catalog. SQLite contention also returns immediately with the connection timeout restored; unrelated
errors retain rollback and error propagation. Focused hosted regressions cover both writer boundaries,
unchanged lease/attempt state, later successful admission and rollback after a non-contention error.
The same-input comparison completes the explicit update in 206,900 ms with zero publication
preemptions. Its 59 successful status polls have a 10-ms median and 66-ms maximum. The same 29
visible identities remain stable and both October sets still match. All 128 source samples and
original profile/catalog hashes remain unchanged; the session exits normally after 311,342 ms.
The log also exposes the independent recovery-ownership failure described below, so continued
automatic recovery is not established by those successful polls.

## Foreground capture of unfinished inventory work on 2026-10-15

Starting a foreground update while the retained inventory control is pending assigns that control
to the scan. Publication then marks it completed, although its authority and inventory remain active.
The native comparison records 165 `metadata_inventory_recovery_affinity_corrupt` errors afterward;
the inventory stays at its initial frontier. This explains the retained task's inability to continue.

`scan_change_capture` now owns queue capture and completion within the existing scan transactions.
Active recovery authorities and inventory-owned candidates retain their own workflow. Publication
also releases those tasks if a retained scan from an older client has already captured them. Ordinary
scan-covered queue changes retain their completion and catch-up lineage. The existing authority
validator is unchanged. Hosted regressions cover a pending control and candidate across publication
and reopen, an already captured control, ordinary completion, and rollback of a failed publication.

Affected production / inline-test / dedicated-test sizes: capture owner 196 / 0 / 203; scan admission
235 / 0 / existing tests; scan publication 948 / 0 / existing tests. The 3,619-line catalog facade and
6,150-line dedicated queue-test facade only add module wiring. Owned formatting and diff checks pass;
the Debug build succeeds in 43.8 seconds.

The ordinary Debug run at `180917d` uses the same original snapshot and source scope. Its foreground
update completes 70,079 entries and 64,452 accepted images in 205,976 ms. The pending recovery
control retains its own authority, continues after publication, and completes 70,078 inventory
entries and 2,242 candidates. The final catalog contains 65,130 locations with Current continuity;
the sidebar displays Synchronized. The two October sets match all 9 and 42 current source files.
All 128 source metadata samples remain unchanged. There are no affinity errors, native sync errors
or publication preemptions. The 430 status polls have a 15-ms median and 80-ms maximum.

The session exits normally after 677,119 ms. Reopening the same catalog reports Current in 952 ms
with 65,130 locations, two scans and one completed inventory. Seven subsequent polls take at most
25 ms, with zero recovery starts or sync errors. This session exits normally after 44,386 ms.
Both exits restore original preferences, settings and catalog hashes. Final cleanup removes 108
derived files totaling 378,368,161 bytes and 11 diagnostic/helper files totaling 53,729 bytes;
`build/test/derived` retains only its fixture marker. Reusable sources, templates and the current
Debug build remain.

The `ccc18df` Actions run passes 1,902 Rust tests, including the former mixed-load connection-lifetime
failure. Its only Rust failure is the new directory-only publication fixture, which omitted the
required subtree queue owner. `4c2980d` adds the rename intent without changing validation or the
Journal-priority assertion. All other applicable jobs in that run pass. The latest ownership,
contention and corrected-fixture checks remain with Actions.

### Reopening a recovery control completed by an older client

The old foreground-publication SQL can already have committed a completed control while its
inventory remains running. Replaying that actual SQL from `0773ef5` reproduces the state rejected
by `catalog_recovery_execution_contract_unverifiable` during normal opening. Preventing subsequent
capture does not repair these persisted records.

The current-schema recovery transaction now recognizes the completed foreground publication by
root generation, active scan, queue high-water mark, publication time and catalog revision. It
restores that control and candidates completed by the same publication, preserving inventory
progress, task identity, attempts and independently completed candidates. The unchanged full
catalog validation runs before commit. Any restoration or validation error rolls back every repair.

Replaying the actual old and new SQL in an in-memory diagnostic restores the captured control and
candidate, retains an earlier completed candidate and the running inventory, and leaves no match
for a second repair. Hosted regressions cover normal reopen and continued leasing, preserved
progress, idempotent reopening, unrelated completion rejection and transactional rollback.
Scoped formatting and whitespace checks pass; Actions remain pending. The actual retained schema
stores the revision witness in `catalog_state`; `abb4bfd` uses that owner in both repair and fixture.

The ordinary Debug client at `abb4bfd` opens an isolated retained snapshot with the older terminal
state reconstructed using that publication SQL. The application repairs the control, retains the
same inventory and finishes 70,078 entries and 2,446 candidates. It publishes 65,130 locations and
shows Synchronized with no pending work, retries or coverage gaps. Both current October source sets
match exactly (9 and 42 files), and all 128 source metadata samples remain unchanged. Across 103
status polls, the median is 6 ms and maximum 63 ms; no affinity or synchronization error is recorded.
Normal reopening reports Current in 1,380 ms, retains one scan and one completed inventory, starts
no recovery and has an eight-poll maximum of 13 ms. Both processes exit normally; original profile
and catalog hashes match after restoration. Cleanup removes 108 disposable data files (342,884,513
bytes) and 11 diagnostics/helper files (213,190 bytes). The reusable derived directory retains only
its fixture marker. The later bounded-admission change is covered separately below.

Affected production / inline-test / dedicated-test sizes: current recovery coordinator 79 / 0 / 0,
capture-state repair 87 / 0 / 0, capture regressions 0 / 0 / 323. The migration facade is
9,845 / 5,267 / existing tests; it only wires the new owners and removes the former coordinator.

### Bounded recovery admission during repeated writes

The `5bef30f` Actions run passes 1,904 Rust tests and fails five. Three capture fixtures attempted
to alter an immutable authority reason; `d44f592` creates the authority with its intended reason.
Two mixed-load checks stop when the next P2 source page does not start within five seconds. The
dispatcher's one-shot admission has no queue position, so the continuously repeating cleanup
writer can reacquire the same-priority lane before each dispatch attempt.

Recovery dispatch now uses the existing FIFO admission with a five-millisecond wait bound. It
retains its position behind the current writer, preserves higher-priority work and yields promptly
when a longer publication still owns the lane. SQLite lock acquisition remains non-waiting and the
original busy timeout is restored afterward. Regressions cover a competing writer at the exact
registration boundary and timeout retirement without removing another waiter's position; the
existing publication-lock, external-lock and rollback cases remain. The full mixed-load workloads,
five-second page deadlines, P0 latency assertions and source counts are unchanged.

The same Actions run passes the 100-startup zero-source-enumeration regression and all nine other
applicable jobs. The subsequent `0309ae4` run passes 1,914 Rust tests, including both mixed-load
checks and the new admission regressions, plus all nine other applicable jobs. Its sole failure
is the capture fixture's independent path: `ConsistencyAudit` merges it into the pending root
recovery control, so no separate row exists. The fixture now enqueues a `StartupCatchUp` path and
asserts its independent insertion and foreground capture before checking completion, preserved
recovery ownership and reopening. This is a test correction; runtime behavior is unchanged.
The corrected `6d4a288` [Actions run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38681407733)
passes all ten applicable jobs and the aggregate gate. Rust reports 1,915 passed, zero failed and
24 ignored, plus three passing documentation tests. The complete run includes the corrected capture
fixture, both mixed-load checks, paused-import restart with changed membership, bounded retirement,
and the real-journal unchanged restart that asserts zero source enumeration and media opens. No
workload, deadline or assertion is relaxed. The three signing-path jobs remain outside this run's
required gates.

Affected production / inline / dedicated-test sizes: admission 329 / 0 / 354 plus its existing
reservation tests; lease owner 75 / 0 / 198; capture regressions 0 / 0 / 344. Scoped formatting and
whitespace checks pass.

### Completed-task retention with file issues on 2026-10-15

The retained update exposed a completed task card that stayed over the gallery because its
nonzero file-issue count disabled automatic dismissal. The card had no pending recovery action.
`c62fd7b` lets completed import and update feedback use the existing five-second acknowledgement;
paused, failed, limited and unresolved-publication states retain their existing controls. Hover,
keyboard focus and accessible-navigation retention remain unchanged. Copy and layout are unchanged.
The hosted regressions cover completion with file issues, an independent paused import and a
limited import that must remain visible.

The ordinary Debug walkthrough reuses the 10,000 varied images in `build/test`, with one temporary
invalid-image fixture. Import completes in 48,451 ms and update in 26,995 ms; each records one
`image_format_unsupported` issue. Both completion cards disappear without a dismissal click, and
the gallery remains at 10,000 images and continues scrolling. Both issue records remain in the
catalog after dismissal. All 10,000 original fixture sizes, modification times and attributes match.
The process exits normally after 258,901 ms, restoring the original profile and catalog hashes.
Cleanup removes 61 derived files (65,255,043 bytes) and five diagnostic/fixture files (777,983 bytes),
retaining only the derived fixture marker, reusable sources/templates and current Debug build.
The scoped formatter and whitespace checks pass; automated tests remain assigned to Actions.

### Ordinary Release closed-process additions on 2026-10-15

The ordinary unsigned Release containing `3177de1` reuses the 10,000 varied images in `build/test`
with fresh derived storage. Initial import completes in 35,798 ms with 10,000 active locations,
Current journal coverage, one scan and no inventory. After normal exit, three temporary copies
from the existing landscape, portrait and square templates are added while the process is closed.
Reopening reaches exactly 10,003 locations, Current and zero pending/retry work in 1,404 ms from
the restart signal, including process launch. The first completed observation is 1,191 ms after
Windows process creation. Scan and inventory counts remain one and zero. Searching the added
filenames shows all three images with decoded previews and no retry controls.

Both processes exit normally. The first session retires at 306,200 ms and the second at a
cumulative 489,008 ms, including the initial window-activation delay and manual foreground recovery.
All 10,000 reusable source sizes, modification times and attributes match before and after cleanup;
the three additions match their recorded hashes. Original preferences, storage settings and catalog
components match their saved hashes after restoration. Cleanup removes the three additions and
all disposable catalog, cache, diagnostics and helper output, totaling 33.12 MiB. The derived
directory retains only its fixture marker. The restored user Debug opens the original gallery at
65,130 images and Synchronized. The subsequent `6d4a288` changes only a test fixture and records;
the latest hosted Rust job remains in progress while the other nine applicable jobs have passed.

The subsequent requested bulk case reuses the same 10,000-image corpus and adds 1,000 temporary
copies while Release is closed. The selection contains 900 JPEGs and 100 PNGs, all 12 existing
dimension groups, modification dates spanning 2010–2026, and encoded sizes from 31,164 to
13,858,639 bytes. It includes landscape, portrait, square, 12,000×1,500 and 1,500×12,000 panoramas,
and 8,000×8,000 images, with non-uniform content. The temporary additions total 1,092,114,678 bytes.

The baseline import completes in 30,870 ms. After exit and fixture additions, 117 read-only
observations track incremental publication to exactly 11,000 locations. The final observation
has Current coverage, zero pending/leased/retry work, one scan and zero inventory runs at
15,666 ms from the restart signal, or 15,622 ms from process creation. At 10,557 images the native
window displays Updating and keeps existing decoded images visible. Subsequent downward/upward
scrolling shows the added images; 50 added previews have been materialized on demand. Opening an
added 12,000×1,500 JPEG finishes full-image loading, and Escape returns to the same gallery position.
Search-text injection did not populate the field after refocus, so this run uses direct scrolling
for added-image inspection and makes no search-input acceptance claim.

Both processes exit normally, with the first at 161,484 ms and the full two-session run at
409,224 ms. All original fixture sizes, modification times and attributes match; every temporary
copy matches its recorded hash. Original preferences, settings and catalog hashes are restored.
Cleanup removes all 1,000 copies, 116 disposable derived files (39,695,374 bytes), and ten
diagnostic/helper files (1,009,275 bytes). Only the fixture marker remains in the derived directory;
the reusable corpus is retained and the user's ordinary Debug client is reopened.

### Paused import with bulk directory changes on 2026-10-15

The existing ordinary-journal regression performs actual fixture deletion and addition between
paused-process exit and continuation. It checks retained progress, unchanged inspection reuse,
renamed hardlink identity, exact final membership and a fully validated reopen. The separate
membership regression deletes an already inspected file, adds files to both an inspected directory
and a new directory, changes another file, then pauses and reopens again before completion. Both
pass in the `6d4a288` Actions run; the first also covers removal of an unvisited file.

The requested larger native Debug case reuses the established 10,000-image corpus with fresh
derived storage. The window pauses after 5,900 accepted images. While it stays paused, 250 inspected
and 250 unvisited files are moved outside the watched source into a contained restoration directory;
500 temporary copies are added, half under completely inspected directories and half under
unvisited directories. The reversible removals exercise disappearance from the selected source;
literal file deletion remains covered by the production restart and membership regressions above.
The additions use 440 JPEGs and 60 PNGs, ten dimensions, dates spanning 2010–2026, encoded sizes
from 31,164 to 7,216,220 bytes and non-uniform content, totaling 543,328,144 bytes.

Continue immediately displays the retained 5,900 count, also observed in the same scan's first
running database snapshot 222 ms after input. It reaches Current in 48,246 ms from Continue, after
finishing the remaining import and replaying the paused-period changes. During catch-up the window
displays Updating; published membership temporarily falls to 9,750 as removals precede additions.
The final exact set is 10,000: all 500 additions present, all 500 removed paths absent and the 9,500
retained source paths present. All 5,650 unchanged previously inspected assets keep their IDs.
There is one scan and zero inventory runs throughout 3,810 read-only observations. Normal reopen
retains the exact set, identities, Current coverage and scan/inventory counts.

Both client processes exit normally, at cumulative runner times of 363,257 and 414,406 ms. The
original profile and catalog hashes match after restoration. All 500 original fixtures are moved
back with matching hashes, and all 10,000 source sizes, modification times, attributes and file
identities match their baseline. Cleanup removes the 500 temporary additions, 92 derived files
(36,873,057 bytes) and twelve diagnostics/helper files (2,697,111 bytes). The derived directory again
contains only its fixture marker; the user's ordinary Debug is reopened with the original profile.

### Scoped closing review

An independent read-only review of `46d41e9..6d4a288` covers the 23 changed source/test files and
their owning contracts. It follows journal publication admission, bounded recovery lease admission,
foreground capture ownership, narrow retained-state repair and completed-task expiry through their
callers and regressions. No confirmed new defect is reported in that range. It performs no product
run or source access. Existing unstaged copy/layout edits remain outside its committed range. Its scope is this
closing repair batch, while earlier acceptance records retain their own coverage boundaries.

### Delivery verification on 2026-10-15

The final recorded candidate `712f8ea` passes all ten applicable jobs and the aggregate in
[Actions run 38688362613](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38688362613).
Rust reports 1,915 passes, zero failures, 24 opt-in ignored cases and three passing documentation tests.
The preceding `6d4a288` and `3109015` runs also pass. Product inputs are unchanged across these three
revisions; later edits only close this delivery status in documentation. Documentation uses scoped
encoding, link, whitespace and diff checks without another local product run.

The completed native cases above retain their original measurement boundaries. The reusable source
corpus and original profile are restored; `build/test/derived` contains only its 16-byte fixture marker.
The user's 21 pre-existing unstaged files remain unchanged by this closeout. No additional product,
installer, signing, Sandbox or R3 work is introduced by the status update.

### Retained interrupted-journal reopening on 2026-10-16

The retained 10,000-image catalog in `build/test` contains a completed baseline and later proven
journal coverage. A subsequent `identity_journal_hardlinks_not_supported` failure leaves its
checkpoint in `recovery_required`. The production catalog opener rejects this valid interruption
with `catalog_persistent_journal_baseline_contract_unverifiable` before browsing can begin.

The baseline validator now admits interrupted checkpoints with explicit failure details and the
same required identity and durable coverage proof. Unadvanced checkpoints must still match both
baseline boundaries; advanced checkpoints require their retained range and lifecycle records.
Recovery inventory admission also preserves partially consumed coverage when its captured end is
within the new opening boundary, matching the existing recovery writer. Journal replacement keeps
its separate identity rule. This correction changes no schema, checkpoint, source file or scan.

The unchanged retained catalog fails through the production Dart/Rust bridge in 355 ms before the
correction and returns all 10,000 items in 417 ms afterward. Its SHA-256 and checkpoint values remain
identical across those reads. Native Debug then opens the populated gallery, shows decoded previews,
and opens a 3840-by-2160 JPEG. Normal closure preserves the count and interrupted checkpoint. The
unsupported hardlink lookup remains recorded; these observations cover catalog usability and
retained progress, without claiming that the failed journal operation completed.

The original storage configuration is restored byte for byte and the normal 64,419-image gallery
reopens. Disposable probes and 343,378 bytes of test preview cache are removed; the reusable corpus
and retained catalog remain. A final Debug build including the recovery-opening boundary correction
passes in 50.6 seconds and opens the original gallery.

Six boundary regressions cover repeated FULL reopen, complete and partial journal coverage, pending
enrollment, failure persistence, recovery and journal-reset admission, and rejection of missing
failure details, identity changes, missing coverage and out-of-bound recovery openings. Actions owns
their execution. A focused local attempt stops during LLVM compilation with an out-of-memory error;
no tests execute in that attempt, and its disposable failed incremental output is removed. Owned-file
formatting and whitespace checks pass. The affected validator has 450 production lines and no inline
tests; its test facade has 431 lines and the new interruption module has 248 lines.

### Retained journal recovery continuation on 2026-10-16

The error-50 result had been classified as nonrecoverable, excluding its retained checkpoint from
normal journal scheduling. A successful manual update left that checkpoint unchanged. Older affected
roots now enter one existing journal worker probe when available, healthy, idle and without pending
recovery. The probe captures a current opening before admitting metadata recovery. New first/next
hardlink enumeration error-50 results use the same reconstruction path; partial results cannot
publish coverage. Native failures retain the operation, file identity, link count and attributes,
without source paths. The original failing object's identity was not retained and its journal range
has expired; its precise Windows trigger remains unknown.

The retained run exposed a second boundary failure: closing recovery rejected a partially consumed
old checkpoint and would replay from the older failed or trimmed cursor. The closing owner now
validates identity and that the retained captured end is within the opening. A reset, trimmed
journal or specifically unsupported hardlink page requires whole-root reconstruction and replay
from the inventory opening. Readable intervals retain their earlier cursor, including whole-root
translation recovery, so pending cross-root moves can finish with their original identities.
Journal replacement still requires a different journal identity and whole-root reconstruction.
Publication remains transactional and absence waits for replay and candidate completion.

The first hosted run exposed three failures: reset closing rejected retained old-journal ranges on
FULL reopen, and the two cross-root translation orders lost pre-inventory replay. The cursor policy
above corrects the latter. An extracted range-history validator admits old-journal coverage only
with a matching, closed whole-root reset authority and chronological ownership. Same-journal
coverage and volume checks remain strict. Terminal cleanup retains that reset proof until the old
ranges retire; repeated journal resets and cleanup/reopen boundaries have dedicated regressions.

The existing 10,000-image corpus and catalog are reused. Automatic startup enumerates 10,013 entries
into one inventory. After closing is corrected, the production Dart/Rust bridge resumes that same
inventory, drains 797 pending work items and reaches Current in 41,474 ms, with zero pending,
retry or gap counts. Its final report contains 10,000 candidates and zero catalog mutations. A fresh
bridge session reaches Current in 1,456 ms; the inventory count stays one and both baseline windows
remain completed. These timings measure the resumed tail and subsequent reopen, not the entire
first recovery. PowerShell reports native stderr as a command error; the bridge prints completion
and the persisted checkpoint and queue confirm the result.

Native Debug then opens all 10,000 images, displays Current, decodes previews, opens a 3840-by-2160
JPEG and closes normally. An earlier native recovery attempt terminated in `ucrtbase.dll` with
`0xc0000409` while its log also contained Flutter AXTree update errors. That crash's cause remains
open; core continuation and a successful later idle-gallery run do not attribute it. The original
profile and catalog components are restored and match their saved hashes. All 10,000 fixture sizes
and modification times match the existing manifest. No source write or placeholder hydration is
introduced. Cleanup removes 7,333,733 bytes of disposable previews, logs, profile-lease files and
the bridge probe. The retained catalog, reusable corpus and 114,550-byte unresolved crash log remain.

Boundary regressions cover first/next enumeration failures, partial-page rejection, fresh opening
without replaying an old failure, one-root admission, unavailable/stale/busy roots, partial checkpoint
closing and FULL reopen, subtree coverage and journal replacement. Hosted Actions owns full execution.
A local attempt to diagnose the three failed Rust cases stopped during test-binary compilation with
LLVM out of memory; no Rust test ran in that attempt. Direct execution of the extracted production
SQL passed four current/reset/legacy checks and ten rejecting mutations. Owned-file formatting and
whitespace checks also pass. Hosted verification subsequently passes for
`5ea62d05eb7f84a64a12967fdba20fda9008f371`, as recorded below.

| Changed owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Native identity reader / extracted hardlink lookup | 349 / 92 | 0 | 83 / 54 |
| Production synchronization / recovery admission | 3,883 / 67 | 10,508 / 0 | 122 for admission |
| Identity journal volume reader | 442 | 0 | 680 |
| Persistent journal facade / recovery closing policy | 3,429 / 51 | 4,742 / 0 | 87 for closing policy |
| Migration facade / extracted range-history validator | 9,839 / 81 | 5,267 / 0 | 185 for reset history |
| Terminal queue cleanup | 150 | 0 | Included in reset-history regressions |
| Migration interrupted-checkpoint regression module | 0 | 0 | 301 |

The synchronization and persistence facades retain the roadmap's decomposition debt. This change
adds narrow scheduling and policy calls; it does not introduce schema or bridge changes. The
migration facade's pre-test line count includes its existing test-counter and downgrade helpers.

### R2c freeze on 2026-10-16

Product baseline: `5ea62d05eb7f84a64a12967fdba20fda9008f371`.
[Actions run 38856645464](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38856645464)
completes successfully for that exact commit. All ten applicable jobs and the aggregate Windows
gate pass, covering static/Rust, Flutter, native Windows scan/accessibility, unsigned Release and
the five synthetic workloads. The three signing-path jobs are skipped under their configured
conditions. The three failures from the preceding recovery run are corrected and covered by this
passing run; the earlier local out-of-memory attempts remain recorded above.

The [roadmap stage decision](../roadmap.md#current-stage-and-acceptance-boundary) closes and freezes
R2c. The retained 10,000-image recovery and subsequent unchanged reopening keep the measurements
and source-preservation scope above. No additional product run is created for this documentation
transition.

The native Debug termination during recovery remains an unresolved, deferred incident. Its retained
stderr contains Flutter AXTree update errors, and the Windows report records `ucrtbase.dll` with
`0xc0000409`; these observations do not establish the crash's cause. Investigation resumes upon a
new occurrence. Existing logs and the Windows error report are retained. Earlier incomplete
platform or installer qualifications keep their recorded scope and are outside the active R3 queue.
