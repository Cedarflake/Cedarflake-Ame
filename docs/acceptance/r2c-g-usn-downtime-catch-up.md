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
| Rename continuity | Pair bounded same-file-reference old/new names before durable planning |
| Candidate authority | Recheck every candidate through the existing final-state reconciler |
| Enqueue ordering | Commit all root plans before advancing the exclusive volume watermark |
| Crash replay | Re-read and idempotently coalesce an uncheckpointed journal range |
| Queue evidence | Preserve catch-up source and watermark through coalescing and supersession |
| Explicit fallback | Enqueue `FreshnessUnknown` for every affected root on uncertain evidence |
| Standard token | Fall back on `usn_volume_open_failed` without requesting elevation |
| Schema v19 | Migrate v18 atomically, reject an unverifiable marker, and repair only a marker-complete missing derived index |
| Query bound | Use an exact root-and-relative-path index during authoritative reconciliation |
| Shutdown | Cancel and retain catch-up worker ownership until its thread is joined |
| Source safety | Perform no source-media mutation or cloud-placeholder hydration |

## Focused verification evidence

```text
cargo test application::library_change_catch_up::tests --all-features -- --nocapture
4 passed; 0 failed

cargo test adapters::windows_usn_catch_up::tests --all-features -- --nocapture
14 passed; 0 failed

cargo test adapters::sqlite_catalog::migrations::tests --all-features -- --nocapture
9 passed; 0 failed

cargo test active_relative_path_lookup_uses_its_complete_lookup_index --all-features -- --nocapture
1 passed; 0 failed
```

The deterministic adapter covers create, modify, rename, remove, invalid continuity, corrupt
records, candidate and record overflow, deleted-parent reconstruction, root filtering, and one
journal read shared by multiple roots. The controlled real Win32 temporary-root test was executed
with both the normal sandboxed token and the same standard token outside the workspace sandbox.
Both reached the documented `usn_volume_open_failed` permission fallback. Ame did not elevate or
self-elevate. Direct Win32 journal candidates therefore remain unverified on this workstation;
the production adapter path is covered by deterministic backend and parser fixtures.

## Complete repository evidence

```text
./tool/quality_format.ps1 -Check
140 files checked; 0 changed

./tool/quality_verify_daily.ps1
Rust: 346 total; 341 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled picker and scan integration: 2 passed
Windows native accessibility integration: 2 passed
bridge compatibility, release guardrails, and whitespace: passed

./tool/release_verify_windows.ps1
Windows x64 Release build: passed
release bridge and system accent smoke integration: 2 passed

./tool/performance_benchmark_synthetic_library.ps1
files=10000
fixture_ms=7577
cold_ms=16920
warm_ms=18199
pause_ms=27
resume_ms=16692
cancel_ms=170
catalog_bytes=52346880
peak_working_set_bytes=17125376
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
final migration repairs only its missing derived index atomically, rejects a malformed same-name
index, and continues to reject any v19 catalog without the catch-up marker. The final Daily and
Windows Release runs above include those regressions and passed.

## Independent audit

Pending the dedicated PR audit. R2c-G is not marked complete until the committed head has no
remaining Critical, High, Medium, or Low findings.

## Remaining boundary

R2c-G does not claim that the current standard desktop token can read the USN journal directly.
Permission-denied and unsupported volumes remain correct through durable authoritative fallback,
but may not receive the startup-cost optimization. R2c-H owns authorized, serial, read-only
target-library measurement of journal hit rate, startup catch-up, fallback cost, event-to-visible
latency, queue and database growth, memory, cancellation, recovery, source preservation, and
cloud-placeholder preservation.
