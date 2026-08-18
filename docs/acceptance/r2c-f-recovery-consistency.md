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
| Restart safety | Persist the requirement to preserve the previous snapshot and reject publication |
| Corrupt rescan | Keep the last trustworthy active location and catalog revision |
| Limited rescan | Never replace an existing root with a partial snapshot |
| Relative paths | Store slash-separated catalog and checkpoint paths across Windows and migration |
| Schema v18 | Migrate v17 atomically and reject a prerelease v18 missing the recovery contract |
| Recovery retry | Apply bounded exponential retry per root without blocking another root |
| Consistency audit | Persist completion time and never project synchronized before publication |
| Shutdown | Request cancellation without running recovery inside watcher callbacks |
| Source safety | Do not hydrate placeholders or modify original media |

## Focused verification evidence

```text
cargo test application::authoritative_library_changes::tests --all-features -- --nocapture
4 passed; 0 failed

cargo test authoritative_scan --all-features -- --nocapture
2 passed; 0 failed

cargo test adapters::sqlite_catalog::migrations::tests --all-features -- --nocapture
2 passed; 0 failed

cargo test application::library_synchronization::tests --all-features -- --nocapture
9 passed; 0 failed

cargo test corrupt_rescan_preserves_the_last_trustworthy_published_location --all-features -- --nocapture
1 passed; 0 failed

cargo test --all-targets --all-features
300 total; 295 passed; 0 failed; 5 explicit ignores
```

No authorization-bound source root is required or accessed by these fixtures.

## Complete repository evidence

```text
./tool/quality_format.ps1 -Check
140 files checked; 0 changed

./tool/quality_lint.ps1
release guardrails, format, Clippy with warnings denied, and Dart analysis: passed

./tool/quality_verify_daily.ps1
Rust: 300 total; 295 passed; 0 failed; 5 existing explicit ignores
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

The first Daily invocation reached the documented workspace-only Flutter SDK lock before creating a
Dart or `flutter_tester` child. It was interrupted through its own terminal session and the same
repository command passed with scoped sandbox approval. No SDK lock was deleted and no unrelated
Dart or Flutter process was terminated.

## Independent audit

Pending the dedicated R2c-F pull request and read-only subagent audit.

## Remaining boundary

R2c-F does not add a Windows USN Journal adapter or claim downtime catch-up performance. R2c-G
remains conditional under the roadmap's fallback and measured-budget criteria. Target-library
reliability, event-to-visible latency, idle overhead, persistent queue growth, database growth, and
recovery timing remain R2c-H evidence.
