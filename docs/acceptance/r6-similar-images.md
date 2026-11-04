# R6 similarity integration evidence

Status: scoped evidence; [roadmap](../roadmap.md) owns delivery disposition.

The [engine evaluation](r6-engine-evaluation.md) records selection and resource probes.
[ADR 0038](../architecture/0038-bounded-perceptual-similarity.md) owns the admitted algorithm and
group identities. This record covers their production integration.

## Persistent identities and choices — 2026-11-04

Schema 74 separates immutable analysis runs, content evidence, bounded candidate indexes and
resumable matching from durable group references, representative choices, ignores and exclusions.
The migration runs atomically after schema 73 and preserves existing catalog and exact-group choices.

Two focused migration cases passed against the final DDL and row validation. They cover retained
exact-group choices, malformed reference-projection rejection, preservation of similarity choices
after derived evidence removal, complete rollback on a rejected version update, and successful retry.
The command used the repository tool lock and `Invoke-AmeAcceptanceScratch`, with `TEMP` and `TMP`
bound to that disposable fixture directory:

```powershell
cargo test --locked --offline --manifest-path rust/Cargo.toml --lib adapters::sqlite_catalog::migrations::similar_images -j 1 -- --test-threads=1
```

The new migration facade/schema/validation have 51/130/162 source lines, including conditional
fixture helpers, no inline test cases and 153 dedicated test lines. The domain identity owner has
58 source lines. Existing migration coordination changes only dispatch: 4,181 source lines including
fixture helpers and 2,290 inline-test lines; current-schema routing has 416 source and 36 dedicated
test lines. The SQLite facade changes only the schema constant and retains 3,167 source lines with
no inline cases. The retained dedicated catalog suite is unchanged.

This checkpoint covers persistence admission. Automatic analysis, query/decision commands and the
desktop interaction are not connected yet; their acceptance remains outstanding.
