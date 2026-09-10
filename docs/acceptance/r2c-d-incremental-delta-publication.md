# R2c-D incremental delta publication validation

- Date: 2026-08-18
- Scope: path final-state reconciliation and atomic catalog-delta publication
- Source-media access: controlled temporary fixtures, read-only during reconciliation
- Real-library access: none

## Contract under test

ADR 0019 consumes leased ADR 0018 work, applies ADR 0007 and ADR 0016 identity rules, and publishes
catalog mutations, preview ownership, one catalog revision, and queue completion in one SQLite
transaction. Full scans retain their existing complete-snapshot boundary.

The controlled fixtures prove:

| Boundary | Required result |
| --- | --- |
| Unchanged path | Complete the lease, retain compatible evidence, and do not increment revision |
| New file | Add a new asset location without deriving logical identity from its path |
| In-place edit | Preserve the asset, replace dimensions and metadata atomically, invalidate preview state |
| Metadata-engine change | Reinspect unchanged source, preserve the asset, and invalidate incompatible derived state |
| Paired rename | Remove the old location and publish the new location with the same asset and compatible preview |
| Recreated old rename path | Publish the replacement at the old path and the original asset at the new path atomically |
| Case-only rename | Remove the obsolete Windows spelling without duplicating one physical location |
| Rename then removal | Remove the obsolete old location even when the destination is also gone before processing |
| Identity backfill | Persist identity, retain asset continuity, and reuse a migrated v17 location ID |
| Same-path replacement | Create a new asset and inherit no prior derived evidence |
| Authoritative absence | Remove the active location and make its orphaned evidence reclaimable |
| Related valid paths | Publish every mutation at one shared catalog revision |
| Malformed sibling | Retry only the unreadable path while an independent valid path publishes |
| New cloud placeholder | Create no catalog location, perform no hydration, and retain durable retry work |
| Existing cloud placeholder | Preserve the last trustworthy location and retain durable retry work |
| Stale lease | Publish no catalog row or revision after newer overlapping evidence supersedes the lease |
| Revision race | Reject the complete batch after another catalog publication changes the revision |
| Full scan | Leave pending work unleased and reject a transaction that races a running scan |
| Unsupported scope | Leave subtree, root, and freshness-gap work eligible for the authoritative worker |
| Normal deferral | Restore the attempt budget when a full-scan boundary races prepared path work |
| Root unavailable | Leave path work unleased instead of consuming retry attempts while offline |
| Root lifecycle | Reject publication after the leased root generation is retired |
| Preview lifecycle race | Reject prepared retain-compatible state after cleanup without relying on catalog revision |
| Failed preview | Preserve status, issue code, and issue message across a compatible rename |
| Root containment | Reject an intermediate symlink or junction before reading outside the configured root |
| Bounded maintenance | Do not scan, stale, or delete unrelated global preview and asset state |
| Database failure | Roll back location changes, revision, preview state, and queue completion together |
| Evidence contract | Reject inconsistent outcome and retain-or-invalidate combinations |
| Source safety | Preserve every controlled fixture byte during incremental processing |

## Focused verification evidence

```text
cargo test --manifest-path rust/Cargo.toml catalog_delta -- --nocapture
10 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml incremental_library_changes -- --nocapture
18 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml local_files::tests -- --nocapture
7 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml deferred_lease_restores_the_attempt_budget_for_normal_coordination -- --nocapture
1 passed; 0 failed

cargo clippy --manifest-path rust/Cargo.toml --all-targets --all-features -- -D warnings
passed
```

## Complete repository evidence

```text
cargo test --manifest-path rust/Cargo.toml --all-features
275 tests; 270 passed; 0 failed; 5 existing explicit ignores

./tool/quality_verify_daily.ps1
Rust: 275 total; 270 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled scan integration: 2 passed
Windows native accessibility integration: 2 passed
format, Clippy, Dart analysis, bridge compatibility, release guardrails, and whitespace: passed
```

The initial workspace-only Daily reached the documented Flutter SDK lock before creating a Dart
child. It was stopped, and the identical repository command passed with the scoped sandbox approval
required by `AGENTS.md`; no SDK lock was deleted and no unrelated process was terminated.

The 2026-08-19 final integration audit hardening added production-runtime fixtures for both cloud
placeholder cases. The focused synchronization suite passed 15 tests, and the complete Rust suite
passed 390 tests with seven existing explicit ignores. The fixtures use temporary files marked with
the Windows offline attribute, clear that attribute after inspection, and never open placeholder
content through the discovery adapter. The complete repository Daily subsequently passed with 397
Rust tests total, all Flutter test files, both Windows integrations, and the shared quality gates.

Post-integration migration hardening reopens a true v17 catalog whose backslash path and legacy
location identifier are normalized by v18, then executes the first live path identity backfill.
The focused incremental suite passed 26 tests and proved the mutation retains that identifier,
creates no duplicate location, and leaves the scan asset count unchanged. The current complete Rust
suite passed 402 tests total: 395 passed and seven existing explicit ignores.

## Remaining boundary

R2c-D does not start the production observer lifecycle, render Flutter freshness state, perform
authoritative subtree/root recovery, read downtime journal evidence, or run a real-library event
acceptance. Subtree, root, and freshness-gap rows remain durable rather than being partially
acknowledged. Those responsibilities remain R2c-E through R2c-H.
