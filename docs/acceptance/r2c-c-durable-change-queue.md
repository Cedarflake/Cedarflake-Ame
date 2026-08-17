# R2c-C durable change queue validation

- Date: 2026-08-17
- Scope: normalized intent persistence, coalescing, leasing, retry, recovery, metrics, and retention
- Source-media access: none
- Real-library access: none

## Contract under test

ADR 0018 stores ADR 0016 intents in schema v17 through the Ame-owned `LibraryChangeQueue` port.
The validation ends before R2c-D filesystem reconciliation, catalog delta publication, or Flutter
freshness presentation.

The controlled queue fixtures prove:

| Boundary | Required result |
| --- | --- |
| v16 migration | Existing rows remain intact; queue tables and initial root-generation authority are added |
| Historical migrations | Every committed schema fixture reaches schema v17 |
| Repeated notification burst | Four observations across two plans become one path reconciliation |
| Create then remove | One final-state reconciliation remains; the pair is not discarded |
| Process exit after enqueue | Reopening the same catalog leases the same stable work after debounce |
| Source restart | Later ingress keeps its tuple through sequence reset, equal timestamps, origin change, and clock rollback |
| Paired rename | Old and new relative paths survive restart in one atomic intent |
| Conflicting rename | Shared old/new paths, directory descendants, and nested old subtrees invalidate or degrade safely |
| Subtree supersession | One parent subtree replaces an unleased child path and retains both evidence counts |
| Capacity overflow | Distinct excess work degrades; an absorbing subtree/root fits a lowered normalized bound |
| Later same-path evidence | The earlier lease is superseded and cannot acknowledge completion |
| Root lifecycle | Registration advances durable authority before ingress, including zero-work roots; missing authority fails closed |
| Crash during lease | Expiry produces structured retry-wait state and bounded backoff after reopen |
| Retry exhaustion | Work remains degraded under a lowered policy; new evidence reopens a bounded budget |
| Metrics | State counts, freshness gaps, ready count, expiry/exhaustion, and oldest delay are non-mutating |
| Retention | Terminal queue rows are removed in bounded passes; root-generation tombstones are retained |

The 500 ms initial stabilization default is exercised with controlled events spaced 50-100 ms
apart. Leasing before the final 500 ms deadline returns no work; leasing at the deadline returns one
row with first sequence 1, most-recent sequence 4, and coalesced count 4.

## Verification evidence

Focused queue tests:

```text
cargo test --manifest-path rust/Cargo.toml change_queue
35 passed; 0 failed
```

Complete Rust suite after schema v17:

```text
cargo test --manifest-path rust/Cargo.toml --all-features
243 tests; 238 passed; 0 failed; 5 ignored
```

The five ignored tests remain the existing explicit real-library or manual-performance gates. No
ignored test belongs to R2c-C.

Clippy with warnings denied:

```text
cargo clippy --manifest-path rust/Cargo.toml --all-targets --all-features -- -D warnings
passed
```

Complete lock-aware repository Daily:

```text
./tool/quality_verify_daily.ps1
Rust: 243 total; 238 passed; 0 failed; 5 existing explicit ignores
Flutter: all test files passed
Windows controlled scan integration: 2 passed
Windows native accessibility integration: 2 passed
format, Clippy, Dart analysis, bridge compatibility, release guardrails, and whitespace: passed
```

The initial workspace-only Daily reached the documented Flutter SDK lock path without creating a
Dart child. It was stopped, and the identical repository command passed with the scoped sandbox
approval required by `AGENTS.md`; no SDK lock file was deleted and no second Flutter process was
started concurrently.

## Remaining boundary

The queue intentionally does not read source metadata, open media, decide file identity, update
active locations, increment a catalog revision, emit Flutter state, or run catch-up discovery.
Those are R2c-D, R2c-E, and later R2c slice responsibilities.
