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
| Cross-root moves | Preserve asset and compatible preview identity for path and bidirectional authoritative moves without dependency cycles |
| Candidate authority | Recheck every candidate through the existing final-state reconciler |
| Enqueue ordering | Commit all root plans before advancing the exclusive volume watermark |
| Crash replay | Re-read and idempotently coalesce an uncheckpointed journal range |
| Queue evidence | Preserve catch-up source and watermark through coalescing and supersession |
| Explicit fallback | Enqueue `FreshnessUnknown` for every affected root on uncertain evidence |
| Standard token | Fall back on `usn_volume_open_failed` without requesting elevation |
| Schema v19 | Migrate v18 atomically, reject an unverifiable marker, and repair only marker-complete handoff authority or derived indexes |
| Query bound | Use an exact root-and-relative-path index during authoritative reconciliation |
| Root isolation | Allow a ready root to recover while another root still lacks healthy continuity evidence |
| Checkpoint retention | Delete at most 128 obsolete checkpoints after seven days and never while a freshness gap is unresolved |
| Exact subtree scope | Keep `Album` and `album` sibling locations outside each other's authoritative capacity window |
| Durable handoff | Snapshot identity and compatible evidence before destructive catch-up publication, then clean it after the watermark is terminal |
| Shutdown | Cancel and retain catch-up worker ownership until its thread is joined |
| Source safety | Perform no source-media mutation or cloud-placeholder hydration |

## Focused verification evidence

```text
cargo test application::library_change_catch_up::tests --all-features -- --nocapture
4 passed; 0 failed

cargo test adapters::windows_usn_catch_up::tests --all-features -- --nocapture
19 passed; 0 failed

cargo test adapters::sqlite_catalog::migrations::tests --all-features -- --nocapture
10 passed; 0 failed

cargo test application::incremental_library_changes::tests --all-features -- --nocapture
22 passed; 0 failed

cargo test application::library_synchronization::tests --all-features -- --nocapture
13 passed; 0 failed

cargo test adapters::sqlite_catalog::catch_up::tests --all-features -- --nocapture
5 passed; 0 failed

cargo test adapters::sqlite_catalog::catalog_delta::tests --all-features -- --nocapture
11 passed; 0 failed

cargo test active_relative_path_lookup_uses_its_complete_lookup_index --all-features -- --nocapture
1 passed; 0 failed
```

The deterministic adapter covers create, modify, rename, remove, invalid continuity, corrupt
records, candidate and record overflow, child-before-parent reconstruction, case-sensitive root
filtering, and one journal read shared by multiple roots. Incremental fixtures cover both path move
orders, bidirectional authoritative moves, compatible preview transfer, exact-case subtree
capacity, and unrelated-removal progress. The controlled real Win32 temporary-root test was executed
with both the normal sandboxed token and the same standard token outside the workspace sandbox.
Both reached the documented `usn_volume_open_failed` permission fallback. Ame did not elevate or
self-elevate. Direct Win32 journal candidates therefore remain unverified on this workstation;
the production adapter path is covered by deterministic backend and parser fixtures.

## Complete repository evidence

```text
./tool/quality_format.ps1 -Check
140 files checked; 0 changed

./tool/quality_verify_daily.ps1
Rust: 361 total; 356 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled picker and scan integration: 2 passed
Windows native accessibility integration: 2 passed
bridge compatibility, release guardrails, and whitespace: passed

./tool/release_verify_windows.ps1
Windows x64 Release build: passed
release bridge and system accent smoke integration: 2 passed

./tool/performance_benchmark_synthetic_library.ps1
files=10000
fixture_ms=12476
cold_ms=35115
warm_ms=34676
pause_ms=26
resume_ms=32196
cancel_ms=172
catalog_bytes=52318208
peak_working_set_bytes=17612800
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
final migration may add the empty durable handoff contract or repair a missing derived index
atomically, removes the obsolete prerelease peer index, rejects a malformed same-name object, and
continues to reject any v19 catalog without the catch-up marker. The final Daily and Windows Release
runs above include those regressions and passed.

## Independent audit

The first PR audit returned Critical 0 / High 2 / Medium 2 / Low 1. Its exact-case candidate,
cross-root path handoff, per-root readiness, deleted-parent reconstruction, and checkpoint-retention
findings were corrected. The second audit of `85a4556` returned Critical 0 / High 1 / Medium 1 /
Low 1: authoritative peers could form a wait cycle, subtree catalog containment was still
case-insensitive, and the canonical active slice was stale. The current head replaces wait edges
with durable pre-removal snapshots, adds bidirectional authoritative and exact-case subtree
regressions, and updates the roadmap. Final independent re-audit remains pending; R2c-G is not marked
complete until that committed head has no remaining Critical, High, Medium, or Low findings.

## Remaining boundary

R2c-G does not claim that the current standard desktop token can read the USN journal directly.
Permission-denied and unsupported volumes remain correct through durable authoritative fallback,
but may not receive the startup-cost optimization. R2c-H owns authorized, serial, read-only
target-library measurement of journal hit rate, startup catch-up, fallback cost, event-to-visible
latency, queue and database growth, memory, cancellation, recovery, source preservation, and
cloud-placeholder preservation.
