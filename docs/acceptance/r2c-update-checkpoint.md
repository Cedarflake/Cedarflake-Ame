# Retained update checkpoint and pending-read observation

Date: 2026-10-09. Scope: atomic scan progress, update task projection, and one bounded native
update/browsing observation. This record does not accept R2c or combined-source recovery.
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
