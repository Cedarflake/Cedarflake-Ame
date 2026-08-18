# R2c-F recovery and consistency validation

- Date: 2026-08-18
- Scope: bounded authoritative recovery, full-scan escalation, and low-frequency consistency audit
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
| Single scan owner | Reject a second running or paused authoritative scan for the same root |
| Restart safety | Persist the requirement to preserve the previous snapshot and reject publication |
| Corrupt rescan | Keep the last trustworthy active location and catalog revision |
| New full-scan issue | Keep a published root stale and do not advance its audit without prior evidence |
| Migrated placeholder | Preserve a normalized v17 path with its legacy location identifier |
| Migrated healthy file | Preserve a normalized v17 location and asset without identity evidence |
| Limited rescan | Never replace an existing root with a partial snapshot |
| Relative paths | Store slash-separated catalog and checkpoint paths across Windows and migration |
| Scan lifecycle owner | Prevent production and Flutter from concurrently resuming one scan ID |
| Multi-root recovery | Rotate a bounded cursor across every authoritative recoverable scan |
| Schema v18 | Migrate v17 atomically, repair unambiguous draft ownership, and fail closed on conflicts |
| Recovery retry | Preserve bounded exponential retry across re-escalation without blocking another root |
| Retry scheduling | Start workers only for currently due authoritative work |
| Consistency audit | Persist completion time and never project synchronized before publication |
| Watcher restart | Keep the continuity gap unresolved until a restarted observer is healthy |
| Background worker | Do not enumerate authoritative scopes inside the production polling mutex |
| Shutdown | Retain timed-out worker ownership and block restart until a later join |
| Source safety | Retry new and existing placeholders without hydration, removal, or audit success |

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
6 passed; 0 failed

cargo test --all-targets --all-features
318 total; 313 passed; 0 failed; 5 explicit ignores
```

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
release bridge and system accent smoke integration: 2 passed

git diff --check
passed
```

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

## Remaining boundary

R2c-F does not add a Windows USN Journal adapter or claim downtime catch-up performance. R2c-G
remains conditional under the roadmap's fallback and measured-budget criteria. Target-library
reliability, event-to-visible latency, idle overhead, persistent queue growth, database growth, and
recovery timing remain R2c-H evidence.
