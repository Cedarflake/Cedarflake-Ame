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
| Rename then removal | Remove the obsolete old location even when the destination is also gone before processing |
| Same-path replacement | Create a new asset and inherit no prior derived evidence |
| Authoritative absence | Remove the active location and make its orphaned evidence reclaimable |
| Related valid paths | Publish every mutation at one shared catalog revision |
| Malformed sibling | Retry only the unreadable path while an independent valid path publishes |
| Stale lease | Publish no catalog row or revision after newer overlapping evidence supersedes the lease |
| Revision race | Reject the complete batch after another catalog publication changes the revision |
| Full scan | Leave pending work unleased and reject a transaction that races a running scan |
| Root lifecycle | Reject publication after the leased root generation is retired |
| Database failure | Roll back location changes, revision, preview state, and queue completion together |
| Evidence contract | Reject inconsistent outcome and retain-or-invalidate combinations |
| Source safety | Preserve every controlled fixture byte during incremental processing |

## Focused verification evidence

```text
cargo test --manifest-path rust/Cargo.toml catalog_delta -- --nocapture
8 passed; 0 failed

cargo test --manifest-path rust/Cargo.toml incremental_library_changes -- --nocapture
11 passed; 0 failed

cargo clippy --manifest-path rust/Cargo.toml --all-targets --all-features -- -D warnings
passed
```

## Complete repository evidence

```text
cargo test --manifest-path rust/Cargo.toml --all-features
263 tests; 258 passed; 0 failed; 5 existing explicit ignores

./tool/quality_verify_daily.ps1
Rust: 263 total; 258 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled scan integration: 2 passed
Windows native accessibility integration: 2 passed
format, Clippy, Dart analysis, bridge compatibility, release guardrails, and whitespace: passed
```

The initial workspace-only Daily reached the documented Flutter SDK lock before creating a Dart
child. It was stopped, and the identical repository command passed with the scoped sandbox approval
required by `AGENTS.md`; no SDK lock was deleted and no unrelated process was terminated.

## Remaining boundary

R2c-D does not start the production observer lifecycle, render Flutter freshness state, perform
authoritative subtree/root recovery, read downtime journal evidence, or run a real-library event
acceptance. Subtree, root, and freshness-gap rows remain durable rather than being partially
acknowledged. Those responsibilities remain R2c-E through R2c-H.
