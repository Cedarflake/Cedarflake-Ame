# R2c-G Windows USN downtime catch-up validation

- Date: 2026-08-19
- Scope: watcher-first Windows downtime catch-up, durable per-volume checkpoints, and explicit fallback
- Source-media access: controlled temporary fixtures only, read-only after fixture setup
- Real-library access: none

## Contract under test

ADR 0022 adds the Windows change journal as a bounded candidate source above the R2c-F
authoritative recovery ladder. Journal records never become catalog truth. The application starts
live observation first, validates continuity once per volume, durably enqueues root-relative
reconciliation work, and advances the per-volume checkpoint only after every affected root commits.
Any permission, support, continuity, parsing, reconstruction, containment, or capacity uncertainty
becomes root-level freshness-gap work instead of a synchronized claim.

The controlled fixtures prove:

| Boundary | Required result |
| --- | --- |
| Watcher-first startup | Establish live observation before downtime catch-up or fallback recovery |
| Shared volume read | Query and read one journal range for multiple roots on the same volume |
| Continuity | Bind journal identity, next USN, volume GUID, root set, generations, and catalog revision |
| Bounded records | Read at most 65,536 V2/V3 records for one volume |
| Bounded evidence memory | Retain at most 64 MiB of parsed records and reconstructed paths for one volume |
| Bounded candidates | Retain at most 4,096 normalized observations for one root |
| Root filtering | Convert only contained paths to root-relative path or subtree reconciliation |
| Case-sensitive paths | Keep case-distinct NTFS roots and sibling paths as separate candidates |
| Deleted parent chains | Reconstruct child-first delete and rename-old histories without accepting a later new name |
| Rename continuity | Pair bounded same-file-reference old/new names before durable planning |
| Cross-root moves | Preserve asset and compatible preview identity for path, bounded authoritative, and full-scan moves without dependency cycles |
| Candidate authority | Recheck every candidate through the existing final-state reconciler |
| Enqueue ordering | Commit every root plan in one all-or-nothing transaction before advancing the exclusive volume watermark |
| Crash replay | Expose either no root enrollment or the complete batch, then idempotently replay an uncheckpointed range |
| Queue evidence | Preserve a bounded lineage of unconsumed catch-up watermarks through coalescing and supersession |
| Explicit fallback | Enqueue `FreshnessUnknown` for every affected root on uncertain evidence |
| Standard token | Fall back on `usn_volume_open_failed` without requesting elevation |
| Schema v19 | Migrate v18 atomically, validate exact checkpoint, legacy and normalized handoff, lineage, and preview-repair markers, reject orphan or unverifiable authority, and serialize one-time repair across concurrent opens |
| Query bound | Use an exact root-and-relative-path index during authoritative reconciliation |
| Root isolation | Allow a ready root to recover while another root still lacks healthy continuity evidence |
| Checkpoint retention | Delete at most 128 obsolete checkpoints after seven days and never while a freshness gap is unresolved |
| Exact subtree scope | Keep `Album` and `album` sibling locations outside each other's authoritative capacity window |
| Durable handoff | Store full-scan snapshots as `N` deduplicated identity items plus `L` lineage owners, retain them across later watermarks, protect ready preview artifacts from budget reclamation, atomically downgrade missing or explicitly cleaned preview expectations, then clean after the last owner is terminal |
| Shutdown | Cancel and retain catch-up worker ownership until its thread is joined |
| Source safety | Perform no source-media mutation or cloud-placeholder hydration |

## Focused verification evidence

```text
cargo test application::library_change_catch_up::tests --all-features -- --nocapture
5 passed; 0 failed

cargo test adapters::windows_usn_catch_up::tests --all-features -- --nocapture
19 passed; 0 failed

cargo test adapters::sqlite_catalog::migrations::tests --all-features -- --nocapture
24 passed; 0 failed

cargo test application::incremental_library_changes::tests --all-features -- --nocapture
25 passed; 0 failed

cargo test adapters::sqlite_catalog::change_queue::tests --all-features -- --nocapture
47 passed; 0 failed

cargo test application::library_synchronization::tests --all-features -- --nocapture
13 passed; 0 failed

cargo test adapters::sqlite_catalog::catch_up::tests --all-features -- --nocapture
5 passed; 0 failed

cargo test adapters::sqlite_catalog::catalog_delta::tests --all-features -- --nocapture
13 passed; 0 failed

cargo test application::scan_library::tests --all-features -- --nocapture
28 passed; 0 failed; 2 existing explicit ignores

cargo test active_relative_path_lookup_uses_its_complete_lookup_index --all-features -- --nocapture
1 passed; 0 failed
```

The deterministic adapter covers create, modify, rename, remove, invalid continuity, corrupt
records, candidate and record overflow, child-before-parent reconstruction, case-sensitive root
filtering, and one journal read shared by multiple roots. Incremental fixtures cover both path move
orders, bidirectional authoritative moves, compatible preview transfer, exact-case subtree
capacity, cross-watermark handoff, and unrelated-removal progress. Queue and migration fixtures
cover all-root transactional rollback, single-call root enrollment, bounded 64-watermark queue
lineage, bounded 4,096-watermark scan lineage, lineage transfer, multi-watermark asset and preview
ownership, exact checkpoint and handoff table shapes, exact foreign keys, owner and
reverse-provenance validation, bounded atomic terminal cleanup, orphan rejection, and fail-closed
prerelease repair. Full-scan fixtures move assets in both directions between two roots, retain one
identity item for two hard-link locations with 64 lineage owners rather than duplicate identity or
watermark copies, and prove stable asset and compatible preview continuity through source-first and
destination-first publication. Bounded and full-scan fixtures also clear ready preview expectations
before adoption when explicit preview cleanup or startup recovery removes a missing artifact;
reclamation fixtures prove that normal budget pressure cannot select or delete an artifact while
either handoff form owns it. Reopen fixtures restore prerelease v19 states with a stale legacy
artifact row, a missing normalized full-scan artifact row, or an invalid ready expectation already
published into the active snapshot after terminal handoff cleanup. They prove all three forms are
downgraded before bounded adoption, full-scan adoption, or later preview reuse, and that a stale
active owner is removed atomically. A dual-connection fixture gives both connections the same stale
marker-absent preflight, commits the repair through the first writer, and proves the second writer
revalidates and accepts the completed marker rather than racing its creation.
The controlled real Win32 temporary-root test was executed with both the normal sandboxed token and
the same standard token outside the workspace sandbox.
Both reached the documented `usn_volume_open_failed` permission fallback. Ame did not elevate or
self-elevate. Direct Win32 journal candidates therefore remain unverified on this workstation;
the production adapter path is covered by deterministic backend and parser fixtures.

## Complete repository evidence

```text
./tool/quality_format.ps1 -Check
140 files checked; 0 changed

./tool/quality_verify_daily.ps1
Rust: 391 total; 386 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled picker and scan integration: 2 passed
Windows native accessibility integration: 2 passed
bridge compatibility, release guardrails, and whitespace: passed

./tool/release_verify_windows.ps1
Windows x64 Release build: passed
release bridge and system accent smoke integration: 2 passed

./tool/performance_benchmark_synthetic_library.ps1
files=10000
fixture_ms=12163
cold_ms=39412
warm_ms=33973
pause_ms=25
resume_ms=31805
cancel_ms=161
catalog_bytes=52371456
peak_working_set_bytes=17764352
result: passed

git diff --check
passed
```

The first standard 10,000-file performance run exposed an existing root-relative lookup plan that
scanned every active location in a root for each path. At 5,000 rows, 5,000 direct lookups required
72,019 ms. Schema v19 adds the complete `(root_id, relative_path, scan_id, location_id)` index; the
same diagnostic workload then required 288 ms, and the standard benchmark passed with the evidence
above. A regression test checks SQLite's selected query plan so a future migration cannot silently
restore the quadratic behavior.

The first post-index Daily run exposed seven intentionally minimal historical migration fixtures
that did not reproduce their version's path columns or removed the new index incompletely; the
fixtures were corrected without weakening production validation. The first post-index Windows
Release run then opened a marker-complete prerelease v19 catalog created earlier in this slice. The
final migration may add the empty durable handoff and queue-lineage contracts, reconstruct a
bounded scan lineage from retained active queue rows, seed primary lineage only when no retained
handoff exists, or repair a missing derived index atomically. It removes the obsolete prerelease
peer index, validates exact lineage foreign keys and relational ownership, rejects malformed
same-name objects, orphan rows, active scans whose lineage cannot be proved, or handoff evidence
whose lineage cannot be proved, and continues to reject any v19 catalog without the catch-up
marker.
The final Daily and Windows Release runs above include those regressions and passed.

## Independent audit

The first PR audit returned Critical 0 / High 2 / Medium 2 / Low 1. Its exact-case candidate,
cross-root path handoff, per-root readiness, deleted-parent reconstruction, and checkpoint-retention
findings were corrected. The second audit of `85a4556` returned Critical 0 / High 1 / Medium 1 /
Low 1: authoritative peers could form a wait cycle, subtree catalog containment was still
case-insensitive, and the canonical active slice was stale. Head `a4aad5f` replaced wait edges with
durable pre-removal snapshots, added bidirectional authoritative and exact-case subtree regressions,
and updated the roadmap. The third audit of `a4aad5f` returned Critical 0 / High 1 / Medium 0 / Low 0
because root enrollment was still split across transactions and a later watermark could overwrite
the only snapshot lookup key. The current work atomically enrolls all roots, retains at most 64
watermark lineage entries per unresolved row, transfers lineage through coalescing and supersession,
and keeps asset and preview ownership until every lineage owner is terminal. The fourth audit of
`6fae32b` returned Critical 0 / High 1 / Medium 1 / Low 0 because full-scan escalation bypassed
handoff lineage and current-v19 validation accepted wrong-target foreign keys or orphan lineage.
Head `79f8418` froze bounded scan lineage, applied handoff before full-scan replacement, and fixed
the exact lineage foreign keys. The fifth audit returned Critical 0 / High 1 / Medium 1 / Low 0:
full-scan publication still materialized the Cartesian product of identities and watermarks, while
terminal queue retention could detach handoff evidence from its authority. The current work stores
one normalized batch with `N` identity items and `L` lineage edges, preserves terminal provenance
while an active scan owns it, releases the final owner atomically, and validates every forward and
reverse ownership relation on open. The sixth audit of `9e85877` then found that hard-link locations
could collide in the identity-keyed batch, checkpoint and legacy handoff tables were not validated
to exact shape, and preview cleanup or reclamation could invalidate a handoff-owned ready artifact.
The current work selects one deterministic representative per physical identity, validates the
checkpoint and legacy handoff table shapes, treats ready handoffs as reclamation owners, and
atomically downgrades their preview expectations during explicit cleanup. The seventh audit of
`f79f198` found that missing-artifact recovery incorrectly reused the owner-protected budget-delete
path, while successful preview publication scanned every unreferenced artifact. The current work
separates physical recovery invalidation from budget reclamation, atomically downgrades both
handoff forms before deleting a missing artifact, and restores algorithm/orientation/size-bounded
stale selection with handoff exclusions. The eighth audit of `d52c6aa` found that prerelease v19
could already contain a ready handoff whose artifact row was stale or absent, outside the startup
recovery enumeration. Head `ed03bfe` added an artifact-path-indexed preflight on catalog open and
atomically downgraded affected legacy or normalized handoff expectations before adoption. The ninth
audit of `ed03bfe` found that an older prerelease could already have published the invalid ready
expectation into the active snapshot and removed its terminal handoff. The current work extends the
same transaction to current active-scan locations and removes any stale owner, with missing-row and
stale-row reopen regressions. A separate exact-shape repair marker makes that full scan one-time for
prerelease v19 catalogs; fresh and v18-migrated catalogs start complete, later opens skip the scan,
and malformed markers fail closed. The tenth audit of `35b5521` found that two concurrent first
opens could both observe the marker absent before either obtained the migration writer lock, so the
second writer would race the first marker creation. The current work revalidates the exact marker
inside `BEGIN IMMEDIATE`, accepts another writer's completed repair, and retains fail-closed handling
for malformed named objects; a dual-connection regression replays both stale preflight results.
The final independent re-audit of committed head `de6ffad` returned `APPROVE` with no Critical,
High, Medium, or Low findings. R2c-G is complete and audit-hardened.

## Remaining boundary

R2c-G does not claim that the current standard desktop token can read the USN journal directly.
Permission-denied and unsupported volumes remain correct through durable authoritative fallback,
but may not receive the startup-cost optimization. R2c-H owns authorized, serial, read-only
target-library measurement of journal hit rate, startup catch-up, fallback cost, event-to-visible
latency, queue and database growth, memory, cancellation, recovery, source preservation, and
cloud-placeholder preservation.
