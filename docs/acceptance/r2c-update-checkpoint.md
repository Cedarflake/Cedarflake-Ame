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
