# R10 resource and recovery qualification

This record owns measurements and provenance for R10-02. The
[roadmap](../roadmap.md#r10--beta-readiness) owns its disposition; the
[execution plan](../plans/r10-beta-readiness.md#r10-02--resources-interruption-and-sustained-use)
owns the workload and exit criteria.

## Scope and measurement setup — 2026-11-15

The baseline is `b943677c4884b9ad94a668b3eefbbfcf3917030c`, with the documentation-only
R10b activation at `3ec4708a`. The retained catalog contains 63,170 active locations across
`local-primary` and the existing demonstration root. Its semantic store initially contains
46,392 compatible feature records, and its classification cache contains 69,166 records.

Local verification uses the installed SDK, a Release Flutter entry and an optimized native library
with debug assertions enabled. A receipt binds the source commit, native hash, storage override and
measurement entry. The existing production native library is retained and restored by hash after
the owned client exits. SQLite online backups place the catalog, installation settings and derived
feature databases inside one leased acceptance scratch directory. Model files are reused read-only;
models are not downloaded, and source-media mutation is outside this workload.

The mixed session uses the existing gallery, folder, smart-album, favorite, album, organization and
duplicate workspaces, both gallery layouts, distant forward/reverse movement, viewer opening and
closing, cached semantic queries and occasional new descriptions. Two minutes of index warmup
precede the 60-minute interaction period. Client working set, private commit and handles are sampled
separately from owned analysis processes. Device-wide GPU allocation is recorded as shared-device
evidence. Application samples include retained details, compact geometry, decoded-image cache and
live-image counts, foreground read activity, indexing state and synchronization state.

Raw measurements and the scoped entry/launcher remain under ignored `build/performance/r10b-*`.
The resource sampler runs every ten seconds and records observed occupancy, not unsampled peaks.
The host guard stops the owned workload if available physical memory falls below 512 MiB.

## Preflight and source scope

The initial preflight used already-cached search descriptions and was closed normally before the
formal sustained-use measurement. Its source comparison covered 63,070 catalogued `local-primary`
locations and 16 bounded content-hash samples; none changed. The decoder cache approached its
existing 100 MiB ceiling. The formal workload adds uncached descriptions so native model loading is
also observed.

The old `cloud-primary` mapping was unavailable. The current location supplied for this stage resolves
to its image directory; the private mapping has been updated without changing the configured user
catalog. Cloud acceptance uses that scope and preserves placeholder availability.

## Current-root scanning and combined reads

The canonical read-only scan entrypoint ran against both mapped roots in one leased, initially empty
catalog. Native temporary storage was redirected into that lease: the host default temporary path
failed before scanning with `catalog_database_error`. No product code or source checks changed.

| Root and pass | Visited / accepted | Elapsed | Peak working set | Source hash samples |
| --- | --- | --- | --- | --- |
| `local-primary`, cold | 68,690 / 63,072 | 236.61 s | 84.88 MiB | 59 |
| `local-primary`, warm | 68,690 / 63,072 | 230.69 s | 77.50 MiB | 59 |
| `cloud-primary`, cold | 25 / 24 | 2.34 s | 56.04 MiB | 1 |
| `cloud-primary`, warm | 25 / 24 | 1.03 s | 55.98 MiB | 1 |

All source-hash samples remained unchanged. The first local pass recorded 138 camera-metadata,
170 capture-time, 86 metadata-parse and 678 unsupported-format issues without aborting the scan;
the repeated pass retained only the 678 unsupported-format issues. The first cloud pass deferred
24 changed source revisions; the repeated pass completed with no issues. The production catalog
loader then read 63,096 active locations across exactly two roots in 124 pages, preserving gallery
date order. This full-scan measurement is distinct from change-driven catch-up.

## Compact manifest measurements

The existing `tool/benchmark_gallery_layout_manifest.dart` ran serially with the installed Dart SDK.
It constructs 4,096-item chunks using synthetic location identities and geometry.

| Items | Existing storage | Build time | Primitive bytes | Bytes per item |
| --- | --- | --- | --- | --- |
| 79,013 | Flat | 48 ms | 5,925,999 | 75.00 |
| 250,000 | Flat | 85 ms | 18,750,024 | 75.00 |
| 1,000,000 | Hierarchical | 253 ms | 73,006,860 | 73.01 |

These are manifest construction measurements, not per-frame layout or media-memory measurements.
RSS differences across sequential cases include garbage collection, so they are not attributed to
retained manifest storage. The million-item case uses the existing hierarchical representation;
this measurement introduces no new layout or scroll authority.

## Preview storage and source safety

The canonical Release preview workload selected 512 locally readable locations from `local-primary`
and used a separate online catalog backup with a 64 MiB preview budget. All 72 compatible warm
requests reused their artifact across the 128/256/512 buckets. Cold P95 was 517/472/503 ms;
warm P95 was 472/461/486 ms. The first 128-pixel request was a 3,607 ms outlier.

Pressure population reached 57,229,504 bytes across 333 locations. Explicit reclamation took
194 ms and reduced usage to 53,668,191 bytes, below its 53,687,091-byte target. Regeneration took
133 ms; its immediate warm repeat took 134 ms and caused no boundary churn. No automatic
reclamation was observed during population. The workload finished in 172.94 seconds with
430.71 MiB peak working set, below its 1 GiB process limit. All 512 source-entry checks and
16 content-hash samples were unchanged; no preview request failed.

## Native interruption and recovery

Eight current native cases passed in disposable Windows Sandbox: interrupted move/copy receipt
recovery, original-content verification after a lost receipt, album/year movement and membership,
report-only retry, exclusive file locks, unavailable roots, interrupted deep-scan continuation, and
handoff of temporarily locked media from authoritative scanning to incremental synchronization.
The native test executable SHA-256 was
`3307c80cff79b6310c55bb3200abc6d031216d89ccaa17ebefb3930cae011801`.
Each case selected exactly one test and reported zero failures and zero ignored cases.

The lost-receipt fixture originally assumed that rewriting bytes while restoring modification time
must change Windows ChangeTime. This Sandbox retained the same ChangeTime. The fixture now checks
the required behavior independently of that timestamp: unchanged content can be rebound; changed
or unproven content remains uncertain, cannot be published, and cannot be replayed. Identity,
length, modification-time, persisted state and final-byte assertions remain. Production recovery
still hashes the destination against its durable original-content proof. The affected dedicated
fixture contains 196 lines; the unchanged production recovery owner contains 198 lines.

Inputs were mapped read-only and copied to guest-local storage. Networking and clipboard redirection
were disabled. A separate evidence-only output mapping was added after all fixture processes exited
and their executable was removed. The owned Sandbox then stopped and leased scratch storage retired.

## Compatible index reuse and process interruption

The isolated catalog's 63,172 active locations contained 46,331 distinct compatible indexed
contents, 1,595 distinct contents beyond the existing image-input dimension limit, and two
locations without a current published fingerprint. The audit found no unexplained missing
compatible feature. The feature store contained 46,392 records, one compatible 768-dimensional
model identity and 166,268,928 accounted bytes; its storage budget was not reached.

The Release client was interrupted at two actual pending searches. Terminating its owned text
worker returned `classification_worker_exited` in 779 ms; an explicit retry completed in 3,366 ms
using 46,331 indexed contents. Terminating the client during a separate worker initialization
left no owned analysis process. A new client reopened the same isolated catalog and feature store,
reached usable gallery content in 4,624 ms, and completed all 20 search requests without an error.
The first three scope reads took 1,397–1,833 ms; the complete series had median 758 ms and
P95 1,712 ms. Final client working set was 299 MiB, retained details were 500 and live images were
five. No reindex was required. All 63,070 mapped source entries and 16 sampled content hashes
remained unchanged, and the production native library was restored after the owned processes exited.

## Capacity admission coverage

At `262c8d06`, [Quality / CI](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43629299659)'s
Remaining lane passed both capacity-policy cases and both
application-scheduler cases. They exercise CPU/GPU eligibility, unavailable capacity, physical and
commit headroom, immediate pressure downgrade, the continuous 30-second recovery requirement,
foreground work alongside image work, cancellation, and confirmed retirement before serial
replacement. This uses the existing controlled capacity boundary; no workstation-wide memory
exhaustion or threshold change was introduced. The same run's Catalog lane failed its existing
ten-second migration timing assertion at 10.118 seconds; the schema-proof follow-up below records
its repair and subsequent passing full run.

## Schema proof cost

Current catalogs usually retain the exact admitted DDL text. `schema_object_sql_matches` now
accepts byte-identical definitions before performing its existing SQL canonicalization; differing
definitions still use the same quoted-token-preserving comparison. All structure and authority-row
proofs remain, with no persisted proof cache or altered migration order.

The same local Debug migration fixture measured 6.009 seconds before and 4.983 seconds after,
with exactly one full validation in each run. One hundred validated reopens took 991/994 ms total,
with maximum single reopen 11.55/11.76 ms. The original ten-second validation and one-second
individual-reopen assertions passed unchanged. All six schema-shape cases passed, covering
missing objects, literal values, quoted tokens, foreign keys, column order and plain-index shape.
At `763f8e6b`, the complete [Quality / CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43641282085)
passed, including the previously failing Catalog lane. This run predates the live-move repair.

## Sustained-use results

The completed session ran 118 mixed-operation cycles over 60.4 minutes after the two-minute
warmup. All 94 cached descriptions and 24 new descriptions succeeded, with no Flutter or analysis
error. Cached-description P95 was 1,646 ms; new-description P95 was 5,542 ms and maximum was
5,847 ms. These requests followed workspace switches and include result publication/layout.

| Resource | Median | P95 | Observed maximum |
| --- | --- | --- | --- |
| Client working set | 560.72 MiB | 598.15 MiB | 619.14 MiB |
| Client private commit | 736.54 MiB | 799.56 MiB | 832.14 MiB |
| Sampled worker working set | 378.33 MiB | 378.87 MiB | 628.48 MiB |
| Sampled worker private commit | 367.08 MiB | 420.98 MiB | 1,007.70 MiB |
| Client dedicated GPU memory | 192.86 MiB | 199.65 MiB | 212.96 MiB |
| Client handles | 976 | 1,045 | 1,089 |

After the initial warmup, successive ten-minute private-memory medians were approximately
713, 744, 738, 755 and 758 MiB. The decoded-image cache stayed below its 100 MiB limit;
retained detail counts stayed within 160–500. Preview files reached 13,991,606 bytes and
compact geometry remained 3,604,944 bytes. Twenty-five owned worker process identities were
observed; they retired between searches rather than accumulating. Device-wide GPU usage peaked
at 1,685 MiB, including other applications. Minimum host physical/commit headroom was
4,676/5,194 MiB. These samples show a bounded working range for this workload.

Clearing a pending search took 13 ms and left no semantic result attached. Normal window close
left no owned analysis worker. The same isolated catalog restarted successfully and reused all
46,333 compatible indexed contents; 20 search requests had median 767 ms, P95 1,256 ms and
maximum 1,349 ms. This restart's usable-gallery time was 8,965 ms while synchronization resumed,
above R10-01's five-second fresh-process target; the earlier interruption restart was 4,624 ms.
The slower restart remains a timing observation to locate, not a changed startup target.

At `1d54c6fd`, the optimized acceptance DLL
`1dc87e8518c17b6bb74c1bca63169859ccafe2427c75a493b039d5b8727afa26`
ran five fresh processes against one new retained-catalog copy. Process-to-decoded-gallery times
were 5,657 / 4,891 / 4,620 / 4,582 / 4,559 ms. The first launch generated previews and consumed
14 journal observations with two catalog mutations; those observations do not isolate the reason
for its 657 ms target overrun. This series did not reproduce the earlier 8,965 ms restart, but the
first result still exceeds five seconds. All five clients closed normally, all 63,070 source
entries and 16 sampled hashes matched, and the production DLL was restored by hash.

A second new scratch copy, without recompiling, measured 5,448 / 4,730 / 4,578 / 4,562 / 4,640 ms.
Both series began without copied preview files; each first launch generated previews and reconciled
the two pending locations. The eight subsequent fresh processes reused those previews and all
reached decoded gallery content within five seconds. The 5.45–5.66-second empty-preview-store
launches remain distinct measurements; neither compilation nor preview generation alone has been
established as the cause of the difference. This comparison changes no startup target. The second
series also preserved all source-state/hash samples and restored the production native library.

The two initially unindexed locations obtained compatible features, leaving 1,595 distinct
contents beyond the existing input-dimension limit and no unexplained eligible gap. The store
accounted for 166,276,096 bytes and did not reach its budget. A read-only checkpoint taken about
18 minutes into the process matched all 17 durable decision/history tables after shutdown and
restart, including 24 album bindings, 20 exact-duplicate decisions and 33 file-operation items.
This checkpoint covers the remainder of the session and restart. All 63,070 source-entry checks
and 16 sampled content hashes matched the pre-run state. The production native DLL was restored
by hash and the leased catalog/cache copy was retired.

## Startup and retained-root recovery

At `1d54c6fd`, retained-root R2c-M preparation measured 3,181 ms opening the isolated catalog and
352 ms reading the two root-scoped pages and initial manifests, for 3,533 ms combined.
The original one-second combined assertion reported five passing cases and one timing failure
after inventory and source-safety checks. On 2026-11-15, the accepted R10 alignment replaced that
combined limit with five seconds for startup including initial reads and one second for those
cached reads after catalog validation. The revised entrypoint records both windows and catalog
opening separately; the historical failed measurement remains unchanged.

At the same `1d54c6fd` revision, Release executable
`b14b0084ce3926942c1c5627547cd18c9043a5579d30c1e7fb97c0712550dd48`
completed both retained-root inventories below their unchanged 45-second limits:

| Root | Entries | Candidates / unchanged | Total | Source pages | Repository work |
| --- | --- | --- | --- | --- | --- |
| `local-primary` | 68,690 | 2 / 68,688 | 17,870 ms | 11,039 ms | 6,831 ms |
| `cloud-primary` | 25 | 0 / 25 | 93 ms | 6 ms | 87 ms |

Neither inventory was cancelled or backpressured. The separately enclosed repeated pass preserved
all directory-entry metadata, placeholder state and scan rows. Peak Job Object memory was
119,713,792 bytes, below its 2 GiB limit. Small reports were exported before the leased catalog was
retired, including after the retained timing assertion failed. The controlled Release phase passed
25 live changes with P50/P95 351/356 ms, 577 image observations coalesced into 96 rows, 26 ms maximum
foreground poll, 3,553 ms independent-process recovery and 2 ms shutdown. No full-scan row was added.

The revised entrypoint at `2da11054` used Release executable
`5122f19e8910f28de87dd76ec9b3a8a4e6b4b905ea2d0f790be4e852cdbdf9ba`.
Its first run exceeded the local inventory limit at 62,896 ms. A subsequent run completed
`local-primary` in 40,548 ms and `cloud-primary` in 136 ms, with the same entry/candidate counts
as above, no cancellation or backpressure, and unchanged source metadata, placeholders and scan
rows after the repeated pass. Peak Job Object memory was 118,214,656 bytes. Its controlled phase
passed 25 live changes at P95 437 ms, 578 image observations coalesced into 96 rows, 39 ms maximum
poll, 5,896 ms recovery and 4 ms shutdown.

That second run failed the five-second startup assertion at 8,821 ms, including 7,873 ms opening
the catalog. The four individual page/manifest reads totalled 945 ms; the final cached-window
assertion and success report were not reached. The complete entrypoint reported five passes and
one failure. Both retained-catalog seeds had 1,036,095,488 bytes, matching the earlier preparation.

A separate temporary timing build based on `2da11054`, native SHA-256
`a296b773c765e21315e22e09b2dd34eec40ea55025902fb5b0fefc7ac1cb3379`,
measured 7,767 ms for full schema/row validation: 24 ms for structure, another 2,217 ms for the
journal/relationship proof, 2,623 ms for classification and 2,903 ms for remaining checks.
Classification included 187,012 historical predictions, with 2,614 ms reading and validating
them and 1,435 ms inside per-prediction validation. This empty-preview scratch launch reached
usable content in 15,162 ms. All 63,070 source entries and 16 sampled hashes remained unchanged.
The temporary timing edits were removed and the production native library was restored by hash.

The workstation retained a running game throughout these repeats; the second run began at 18%
sampled total CPU load. Separate read-only query samples predominantly ran on efficiency cores.
These failures remain recorded separately from the quiet-host comparison below; no startup,
cached-read or recovery limit changed.

The quiet-host recheck at `8aa1fd5f` began at 9% sampled total CPU load with no game or Ame client
running. Native inputs were unchanged from `2da11054`; the recorded clean-source receipt and
SHA-256 above verified reuse of the same Release test executable. The canonical R2c-M
`-ValidationOnly` admission checked the roots, cloud acknowledgement, physical separation and
fresh storage. The six native selectors then ran under the same Job Object ownership, ten-minute
deadline and 2 GiB bound, without invoking Cargo or rebuilding. All six passed.

| Measurement | Result | Existing limit |
| --- | --- | --- |
| Catalog opening and initial gallery reads | 3,634 ms, including 3,273 ms opening | 5,000 ms |
| Cached root pages and initial manifests | 360 ms | 1,000 ms |
| `local-primary` metadata inventory | 19,844 ms: 12,948 ms source pages, 6,896 ms repository work | 45,000 ms |
| `cloud-primary` metadata inventory | 98 ms: 6 ms source pages, 92 ms repository work | 45,000 ms |
| Peak Job Object memory | 99,938,304 bytes | 2,147,483,648 bytes |

The initial reads returned 524 details and 4,120 manifest items from 63,094 root-scoped items.
Local inventory retained 68,690 entries, two candidates and 68,688 unchanged entries; cloud
inventory retained 25 entries, 24 candidates and one unchanged entry. Both completed without
cancellation or backpressure. The isolated seed catalog contained 1,036,103,680 bytes; queue growth
was two rows. The separately enclosed repeated pass preserved directory-entry metadata, placeholder
attributes and scan rows. Controlled recovery retained 25 live changes at P95 354 ms, 578 image
observations coalesced into 96 rows, a 24 ms maximum poll, 3,564 ms independent-process recovery and
1 ms shutdown. Owned processes exited and leased scratch storage was removed. The production DLL
retained SHA-256 `5b41a66008e2610e076cd68a5821bef6f711bac9ce5d411be9ecdceebc83d04f`.

## Retained diagnostic observations

The first formal mixed session stopped after approximately 24 minutes: cycle 26's uncached text
query reached the worker initialization deadline and returned `classification_worker_timeout`.
The client exited normally and left no owned analysis worker. All 63,070 mapped source locations
and 16 bounded content hashes matched the pre-run state. Three isolated starts of the same text
worker completed in 1.76–2.51 seconds; the mixed-session cause was not isolated.
The subsequent complete session above did not reproduce it; the cause of the isolated timeout
has not been attributed to a repaired defect.

The current R2c-R entrypoint stopped before its first production-coordinator fixture because native
child-directory creation under its guarded KnownFolder root returned Windows error 5. The same
owned directory admitted .NET and command-interpreter creation. Releasing its root handle, changing
normal/extended path spelling and removing the process-local compatibility layer did not resolve
the failure. Existing safeguards remain unchanged.

The same 19 selectors from the canonical R2c-R matrix were then run with the current native
executable inside disposable Sandbox storage. Seventeen passed, including 100 unchanged startups,
million-record bounded parsing, closed-process catch-up, concurrent lane scheduling, shutdown,
blocked I/O cancellation, root replacement, same-volume root isolation and placeholder handling.
Closed-process create/modify/rename/move/replace/delete had P50/P95/maximum 557/583/583 ms,
six durable source ranges, zero inventory runs, zero source enumeration and unchanged scan rows.
The initial live-change and overflow failures are followed up below. This native matrix execution
does not establish a successful run of the host's guarded R2c-R wrapper.

## Live move continuity and overflow follow-up

Windows reported an across-directory move as separate removal and creation observations. When
removal published first, the later creation no longer had the original asset identity. The live
removal owner now resolves that exact Windows identity inside the pinned root and publishes the
relocated file through the existing guarded delta. The inverse ordering also removes the old
location when the destination is already current. Genuine deletion and movement outside the root
retain ordinary removal; no source is moved by this lookup.

Sandbox native executable `9fa9ed2a442745239a47bd5cc6347348540d6d2812a0dcbff6deeb96749d3b5f`
passed both notification-order regressions, real deletion, native containment and unsupported-
identity cases. The shared lookup extraction also retained all four directory-recovery cases in
the preceding build. Actual create/modify/rename/move/replace/delete observations retained their
expected identity and source state, with median 776 ms and maximum/P95 983 ms.

The subsequent Release executable
`65e4b1a7d6a5a0c58a719ba7e09a07548c5c2971813dc3fde514c51b73bb3aca`
passed the complete live/event-storm case. Six live changes had median 494 ms and P95/maximum
634 ms. The 96-path storm coalesced 708 observations into 97 rows, including the new parent
directory's subtree intent, and became visible in 8,807 ms. Each storm image was opened once;
the preceding six operations used nine content opens across their independently admitted
notifications. The fixture now measures those workloads separately instead of assuming three
content opens for six different operations. Its live reads remain bounded by admitted work;
storm reads retain the exact one-open-per-image assertion. There were 111 affected-path entry
reads against the existing 192 limit, zero root enumeration, zero inventory/P2 work and no new
scan rows. All affected and unrelated source snapshots remained unchanged.

The overflow fixture now uses a genuine LiveOnly baseline and 8,192 interleaved writes across
256 paths, exceeding the 4,096-observation ingress bound. A fixed empty connected-journal fixture
had incorrectly claimed coverage; the former 768 writes also did not reliably overflow. Recovery
ran once while all 25 foreground changes completed under the same active authority, with median
301 ms, P95 322 ms and maximum 332 ms. Convergence took 9.998 seconds. All source snapshots and scan
rows remained unchanged. Exact inventory counts include the five retained baseline files, one
foreground file and two directories; independent metadata rechecks are counted separately from
inventory-owned candidates. Existing one-second P0, 4,095-entry read-page and 3,072-queue-row bounds
remain unchanged.

The affected incremental facade has 1,353 non-test lines and its existing dedicated test owner
has 3,860 lines; the new relocation owner has 86 production lines and 62 dedicated test lines.
The local-files facade retains 2,547 lines before its 3,376-line inline test module, including
existing test instrumentation. Production changes are module declarations; the inline test registry
also admits their exact private Windows-only declarations. Native identity lookup
has 69 lines; file containment has 75 production lines plus test-module wiring and 51 dedicated
test lines. Directory recovery now has 123 production lines plus test-module wiring. No broad
facade decomposition or schema migration is part of this repair.

## Replacement continuity fixture

R2c-M now establishes its root and identity through production scanning. Its LiveOnly process
interruption retains a typed uncovered watcher gap before exiting, so the next process has the
recovery authority required by ADR 0024. Native gap admission and journal-backed catch-up remain
covered by the R2c-R results above. The former fixture expected ordinary startup to authorize
inventory without retained gap evidence, a behavior superseded by that contract.

Storm coalescing counts the 96 affected image paths, excluding independently completed parent-
directory notifications. Diagnosis found two completed subtree records for the parent and exactly
one record per image. Final-state checks now require the last written dimensions, including the
offline modification; latency and queue bounds remain unchanged.

Sandbox executable `6d1b8957e54acc863f65ed9850099970489aa0d703f3c6b35ac2e4b91202902c`
passed this focused Debug run: 25 live changes at P95 886 ms, 603 image observations coalesced into
96 rows, independent-process restart convergence in 4,359 ms, shutdown in 11 ms and unchanged
full-scan rows. The dedicated fixture now has 1,311 test lines and no production code. The Release
retained-root measurements and passing revised timing windows are recorded above.

## Source-loading verification

At `1d54c6fd`, [Quality / CI](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43653112176)
passed Static, Flutter, Scan, Accessibility, Windows production build and all Rust partitions except
Remaining. That partition passed 1,255 cases and failed two source-loading checks: the private
Windows identity and relocation modules were absent from the exact module registry.

The registry now admits both declarations without changing the validator. Its dedicated 479-line
test owner covers the current declarations and rejects missing, duplicated, public, differently
conditioned, alternate-path, generated and inline replacements. All seven module-topology cases and
the owning-function enumeration check passed locally; no source enumeration was admitted.
At `c06396d4`, the complete [Quality / CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43657241957)
passed, including Remaining. The subsequent `4be864ba` change updated documentation only.
At `2da11054`, the complete [Quality / CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43687989878)
also passed, including all Rust partitions, Flutter tests, Static,
Accessibility and the Windows production build. The separate real-root recheck above used that
revision's unchanged native inputs; subsequent changes update documentation only.

## R10b closeout — 2026-11-15

R10-02 delivered the 60-minute resource workload, compatible-index restart reuse, preview-budget
reclamation, interruption/recovery coverage, both retained-root inventories and the three compact-
manifest scales. The bounded native recheck above closes the revised startup/cached-read comparison.
Production changes preserve identity across unpaired move notifications and avoid redundant schema
SQL canonicalization; recovery fixtures now exercise retained gap authority and native source-loading
boundaries. Applicable full CI passed at `2da11054`.

The host's guarded R2c-R wrapper still encounters Windows error 5; the same native matrix was
completed in disposable Sandbox. Empty-preview-store starts measured 5.45–5.66 seconds, some
mixed-workload search requests exceeded one second, and one earlier worker initialization timed
out without recurrence in the completed session. Those observations retain their measurements and
scope above. Performance thresholds and accepted UI behavior remain unchanged.

## R10b integration — 2026-11-16

[PR 66](https://github.com/Cedarflake/Cedarflake-Ame/pull/66) was squash-merged as
`121786ebabfa96a5a7297f8cd2e9b153ea0b3428`. Review covered all 24 changed files at
`cc4f866e84e3c4d30cc4e2b3e86df706ec776a96`, their admission/publication call chains and the retained
native evidence, with no new blocking finding. The review checked notification ordering, actual
deletion, root containment, file-state revalidation, schema comparisons and recovery assertions.
The native inputs remain unchanged from the validated `2da11054` revision.

The PR's [Quality / CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/43798952656)
passed all four Rust partitions, Flutter tests, Static, Windows Scan, Windows Accessibility,
the production build and the aggregate Windows Gate. Release and synthetic workload jobs were
not selected by this run. The closeout observations above retain their original scope.

All 24 reviewed paths matched the merged tree before the local and remote
`codex/r10b-resource-recovery` branches were deleted. The independent security documentation from
`c1f84b70` remains in `main`. The roadmap owns the next slice; this integration starts no later stage.
