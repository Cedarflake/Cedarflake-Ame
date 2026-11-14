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
catalog. Subsequent cloud acceptance must use that scope and preserve placeholder availability.

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

## Open sustained-use qualification

The first formal mixed session stopped after approximately 24 minutes: cycle 26's uncached text
query reached the worker initialization deadline and returned `classification_worker_timeout`.
The client exited normally and left no owned analysis worker. All 63,070 mapped source locations
and 16 bounded content hashes matched the pre-run state. Three isolated starts of the same text
worker completed in 1.76–2.51 seconds; the mixed-session cause remains under investigation.
This partial run does not supply the required 60-minute or restart result.

The current R2c-R entrypoint stopped before its first production-coordinator fixture because native
child-directory creation under its guarded KnownFolder root returned Windows error 5. The same
owned directory admitted .NET and command-interpreter creation. Releasing its root handle, changing
normal/extended path spelling and removing the process-local compatibility layer did not resolve
the failure. Existing safeguards remain unchanged; current R2c-R execution evidence is pending.
