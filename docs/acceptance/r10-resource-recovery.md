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
ten-second migration timing assertion at 10.118 seconds; that failure remains separate and open.

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
Hosted verification still owns the previously failing runner result.

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

The two initially unindexed locations obtained compatible features, leaving 1,595 distinct
contents beyond the existing input-dimension limit and no unexplained eligible gap. The store
accounted for 166,276,096 bytes and did not reach its budget. A read-only checkpoint taken about
18 minutes into the process matched all 17 durable decision/history tables after shutdown and
restart, including 24 album bindings, 20 exact-duplicate decisions and 33 file-operation items.
This checkpoint covers the remainder of the session and restart. All 63,070 source-entry checks
and 16 sampled content hashes matched the pre-run state. The production native DLL was restored
by hash and the leased catalog/cache copy was retired.

## Remaining diagnostics

The first formal mixed session stopped after approximately 24 minutes: cycle 26's uncached text
query reached the worker initialization deadline and returned `classification_worker_timeout`.
The client exited normally and left no owned analysis worker. All 63,070 mapped source locations
and 16 bounded content hashes matched the pre-run state. Three isolated starts of the same text
worker completed in 1.76–2.51 seconds; the mixed-session cause remains under investigation.
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
Live-change/event-storm and watcher-overflow recovery hit their existing wait deadlines; these
two failures remain open. This native matrix execution does not establish a successful run of the
host's guarded R2c-R wrapper.
