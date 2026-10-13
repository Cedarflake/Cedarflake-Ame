# R2c-F recovery and consistency validation

- Date: 2026-08-18
- Last amended: 2026-08-21
- Scope: bounded authoritative recovery, evidence-driven full-scan escalation, and legacy audit retirement
- Source-media access: controlled temporary fixtures only, read-only except fixture setup
- Real-library access: none

## Contract under test

ADR 0021 completes the recovery ladder behind the R2c-E production lifecycle. Subtree, root, and
freshness-gap work must either publish one authoritative catalog revision or remain durable and
degraded. A bounded recovery cannot publish a partial absence set, and a full scan cannot consume
queue evidence newer than the generation and high watermark captured at scan start.

The controlled fixtures prove:

| Boundary | Required result |
| --- | --- |
| Bounded subtree | Reconcile source additions and authoritative removals at one revision |
| Directory rename | Preserve asset identity while replacing every old descendant path |
| Enumeration bound | Defer the original row and request a full scan before any partial publication |
| Source failure | Retry without mass false removals or source mutation |
| Full-scan authority | Capture root generation and unresolved queue watermark transactionally |
| Later evidence | Preserve queue work created after the full-scan watermark |
| Scan abandonment | Immediately release only rows frozen by that scan |
| Existing worker lease | Leave independent pre-scan leases under their original owner |
| Slow authoritative worker | Foreground path polling cannot reclaim its lease after nominal expiry |
| Process owner | Reject another same-user production process before it can reclaim a live lease |
| Owner loss | Recover an expired authoritative lease through a new SQLite connection |
| Concurrent catalog writer | Wait before reading mutable scan or queue state; do not fail a later deferred write upgrade |
| Prolonged writer contention | Keep recovery updating and retryable for 30 seconds before surfacing one blocked condition |
| Single scan owner | Reject a second running or paused authoritative scan for the same root |
| Restart safety | Persist the requirement to preserve the previous snapshot and reject publication |
| Corrupt rescan | Keep the last trustworthy active location and catalog revision |
| Authoritative media failure | Publish independent trustworthy evidence and atomically enqueue exact path retries |
| Authoritative finalization race | Publish stable evidence, preserve or omit the exact changed paths, and retry only those paths |
| Media-only restart | Convert a prerelease previous-snapshot checkpoint into the same durable retry contract |
| New full-scan issue | Keep a published root stale and do not advance freshness without prior evidence |
| Migrated placeholder | Preserve a normalized v17 path with its legacy location identifier |
| Migrated healthy file | Preserve a normalized v17 location and asset without identity evidence |
| Migrated identity backfill | Reuse the legacy v17 location identifier without duplicating the location |
| Limited rescan | Never replace an existing root with a partial snapshot |
| Relative paths | Store slash-separated catalog and checkpoint paths across Windows and migration |
| Scan lifecycle owner | Prevent production and Flutter from concurrently resuming one scan ID |
| Multi-root recovery | Rotate a bounded cursor across every authoritative recoverable scan |
| Bounded root fairness | Rotate due bounded authoritative roots even when the first stays ready |
| Schema v18 | Migrate v17 atomically, repair unambiguous draft ownership, and fail closed on conflicts |
| Recovery retry | Preserve bounded exponential retry across re-escalation without blocking another root |
| Retry scheduling | Start workers only for currently due authoritative work |
| Elapsed time | Never enqueue a periodic root scan merely because time passed |
| Legacy audit | Retire prerelease root-audit rows and audit-only scans without dropping historical path retries |
| Active recovery status | Project an in-progress authoritative scan as updating, not as an idle reconciliation request |
| Watcher restart | Keep the continuity gap unresolved until a restarted observer is healthy |
| Background worker | Do not enumerate authoritative scopes inside the production polling mutex |
| Shutdown | Suspend full scans at a durable checkpoint; cancel non-scan recovery and retain timed-out ownership until join |
| Source safety | Retry new and existing placeholders without hydration, removal, or false freshness |

## Focused verification evidence

```text
cargo test application::authoritative_library_changes::tests --all-features -- --nocapture
7 passed; 0 failed

cargo test authoritative_scan --all-features -- --nocapture
3 passed; 0 failed

cargo test adapters::sqlite_catalog::migrations::tests --all-features -- --nocapture
5 passed; 0 failed

cargo test application::library_synchronization::tests --all-features -- --nocapture
11 passed; 0 failed

cargo test migrated_v17_placeholder_preserves_the_normalized_legacy_location --all-features -- --nocapture
1 passed; 0 failed

cargo test migrated_v17_healthy_file_preserves_legacy_location_without_identity_evidence --all-features -- --nocapture
1 passed; 0 failed

cargo test authoritative_full_scan_with_new_placeholder_remains_stale_without_advancing_audit --all-features -- --nocapture
1 passed; 0 failed

cargo test application::library_synchronization::production::tests --all-features -- --nocapture
7 passed; 0 failed

cargo test --all-targets --all-features
318 total; 313 passed; 0 failed; 5 explicit ignores
```

Post-integration hardening added deterministic coverage for the later full-range findings:

```text
cargo test adapters::sqlite_catalog::change_queue::tests --all-features -- --nocapture
50 passed; 0 failed

cargo test application::incremental_library_changes::tests --all-features -- --nocapture
26 passed; 0 failed

cargo test application::library_synchronization::production::tests --all-features -- --nocapture
7 passed; 0 failed

cargo test adapters::sqlite_catalog::migrations::tests --all-features -- --nocapture
24 passed; 0 failed

cargo test --locked --manifest-path rust/Cargo.toml --all-targets --all-features
402 total; 395 passed; 0 failed; 7 explicit ignores
```

These fixtures prove foreground path polling cannot reclaim a live authoritative lease after its
nominal deadline, v17-to-v19 reopen plus live identity backfill retains one legacy location, and
bounded authoritative selection rotates across continuously ready roots. They also prove an
expired final authoritative attempt is normalized through an independent connection after worker
loss and a lower retry policy clears obsolete authoritative deadlines without allowing path
leasing.

The 2026-08-20 media-recovery hardening added the atomic path-retry publication and active-recovery
status fixtures:

```text
cargo test --locked --manifest-path rust/Cargo.toml -j 1 application::scan_library::tests -- --nocapture
30 passed; 0 failed; 2 explicit ignores

cargo test --locked --manifest-path rust/Cargo.toml -j 1 application::library_synchronization::tests -- --nocapture
16 passed; 0 failed

cargo test --locked --manifest-path rust/Cargo.toml -j 1 adapters::sqlite_catalog::change_queue::tests -- --nocapture
50 passed; 0 failed

cargo test --locked --manifest-path rust/Cargo.toml --all-targets --all-features -j 1
405 total; 398 passed; 0 failed; 7 explicit ignores

./tool/quality_lint.ps1
release guardrails, format, Clippy with warnings denied, and Dart analysis: passed
```

The authoritative fixtures cover an existing undecodable location, a newly discovered undecodable
path, the prerelease persisted media-only checkpoint, exact retry insertion after the captured root
gap is completed, preservation of controlled source bytes, and the running-scan status projection.

The 2026-08-21 finalization-race correction extends that exact-path contract to source changes found
after enumeration. A controlled authoritative scan changes one previously published image and
removes one newly discovered image during final validation. The scan completes, publishes the
independent stable snapshot, retains the prior trustworthy location, omits the unverified new
location, and leaves exactly two durable path retries instead of abandoning and restarting the
whole root:

```text
cargo test --locked --manifest-path rust/Cargo.toml authoritative_finalization_races_publish_stable_evidence_and_retry_exact_paths -- --nocapture
1 passed; 0 failed
```

The same 2026-08-20 closeout head passed the complete Daily: 405 Rust tests total with 398 passing
and seven existing explicit ignores, all Flutter test files, Windows Scan 2/2, Windows
Accessibility 2/2, bridge compatibility, release guardrails, formatting across 145 files, Dart
analysis, and whitespace validation.

No authorization-bound source root is required or accessed by these fixtures.

## Complete repository evidence

```text
./tool/quality_format.ps1 -Check
140 files checked; 0 changed

./tool/quality_lint.ps1
release guardrails, format, Clippy with warnings denied, and Dart analysis: passed

./tool/quality_verify_daily.ps1
Rust: 318 total; 313 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled picker and scan integration: 2 passed
Windows native accessibility integration: 2 passed
bridge compatibility, release guardrails, and whitespace: passed

./tool/release_verify_windows.ps1
Windows x64 Release build: passed
same-user duplicate process rejection and replacement startup: passed
release bridge and system accent smoke integration: 2 passed

git diff --check
passed
```

The 2026-08-19 post-integration `quality_lint`, complete Daily, and Windows Release gates passed.
Daily repeated the 402-test Rust result, every Flutter test, Windows Scan 2/2, Accessibility 2/2,
bridge compatibility, guardrails, formatting, and whitespace. Windows Release built the x64
application, rejected a concurrent same-user process before runtime initialization, started a
replacement after the owner exited, and passed both packaged bridge smoke tests. No
authorization-bound real-library acceptance workload was run.

An unprivileged Daily invocation could not write Dart's user-level telemetry configuration; the
same repository command passed with scoped sandbox approval. An unprivileged Windows Release
invocation later remained before child-process creation while holding the repository tool lock.
Terminal interruption did not unwind its PowerShell host, so only the two process IDs proven to
belong to this turn's Release invocations were stopped before the same script passed with scoped
approval. No SDK or repository lock file was deleted, and no unrelated Dart, Flutter, or PowerShell
process was terminated.

## Independent audit

The first dedicated PR audit returned `REQUEST CHANGES` with four High, one Medium, and one Low
finding. Its first re-audit confirmed those paths closed, then returned two High, three Medium, and
one Low follow-up finding. The next re-audit confirmed those paths closed and found one Medium retry
state issue. The final independent re-audit of committed head `91c1933` returned `APPROVE` with zero
Critical, High, Medium, or Low findings. It confirmed that per-root full-scan failure history survives
bounded re-escalation, repeated failures continue toward the five-minute ceiling, another root
remains independent, and successful bounded or full-scan recovery clears the retry state.

A later full-range PR review identified live authoritative lease expiry under foreground polling,
migrated incremental identity duplication, target-evidence overstatement, and bounded-root
starvation. Its first re-audit confirmed those four paths closed and found one Medium follow-up in
final-attempt and policy-lowering retry normalization. The final independent full-range re-audit of
committed head `b1436fc` returned `APPROVE` with zero Critical, High, Medium, or Low findings. It
confirmed live-worker lease isolation, worker-loss exhaustion, migrated identity continuity,
bounded-root fairness, and the narrowed target-evidence boundary without finding a new regression.

## Remaining boundary

R2c-F does not add a Windows USN Journal adapter or claim downtime catch-up performance. R2c-G
remains conditional under the roadmap's fallback and measured-budget criteria. R2c-H owns
target-library catch-up ingress, queue, storage, memory, and source-safety evidence, but its
authorization-bound phase intentionally does not measure target-scale authoritative recovery and
publication timing; that extended measurement remains R10 evidence.

## 2026-08-21 policy correction

The current working tree removes seven-day scheduling, retires only legacy root-reconcile audit
rows, preserves historical path retries that reused the same origin, keeps disconnected full scans
recoverable, and suspends production full scans during shutdown. Five focused Rust regressions passed
for elapsed-time scheduling, legacy audit retirement, disconnected scan retention, shutdown scan
suspension, and production full-scan stop ownership. The complete Daily passed 412 Rust tests total
with 405 passing and seven existing explicit ignores, all Flutter files, both Windows integration
suites, bridge compatibility, formatting, analysis, and whitespace validation. The Windows Release
gate built the x64 application and passed both packaged bridge smoke tests. No authorization-bound
real-library workload was run.

The same-day SQLite writer-coordination amendment makes queue and scan read-modify-write paths acquire
the writer before reading mutable state, leaves an empty path poll read-only, and gives background
recovery the same 30-second transient-contention window as foreground synchronization. Deterministic
two-connection fixtures prove that path leasing and directory claiming wait for an existing writer,
while an empty queue probe returns without competing for the writer. Runtime fixtures prove transient
recovery contention remains non-blocking until the bound and other database failures block
immediately. The Daily static partition passed 418 Rust tests total with 411 passing and seven existing
explicit ignores, formatting across 145 files, Clippy with warnings denied, Dart analysis, bridge-hash
compatibility, and whitespace validation. No real library was accessed.
