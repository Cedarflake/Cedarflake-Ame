# R5 metadata and search evidence

The [roadmap](../roadmap.md#r5-implementation-sequence) owns R5 delivery status. This record covers
the first indexed-text-search change under [ADR 0032](../architecture/0032-indexed-catalog-text-search.md),
not the complete R5-01 metadata and structured-query workflow.

## Catalog text index - 2026-10-30

Scope: schema 60 to 61, catalog-only path indexing, and the existing shared gallery text predicate.
The index consumes catalog rows, never original media, and does not require model inference.
Catalog, query, organization and UI contracts keep their existing public shapes.

Focused local diagnosis used the locked Rust dependencies and the repository tool lock. Incremental
compilation was disabled for this process after the local cache reported a hard-link fallback;
the completed diagnostic build emitted no warning. Initial test-fixture visibility and organization
revision errors were corrected before the passing runs below.

| Filter | Result | Boundary |
| --- | --- | --- |
| `text_search` | 5 passed | Literal punctuation/quotes, Unicode and short queries, case compatibility, transactional rename/delete, VACUUM, scan identity, paging/count/layout and FTS query plan |
| `migrations::gallery_search` | 4 passed | Existing-path backfill, rollback/retry, retained album/history, reopen, damaged schema rejection and selective-query cost |
| `migrations::organization_admission_clock` | 3 passed | Previous latest migration remains valid through the extended downgrade/upgrade test chain |
| `organization::tests::queries::collection_groups` | 3 passed | Grouped album/Favorite search retains physical scope and canonical content authority |

The exact-head hosted checks and the affected desktop search interaction remain required before
integrated acceptance. These adapter checks do not establish metadata extraction, structured filter
controls, real-library search latency or complete R5 acceptance.

### Retained migration compatibility

Hosted run 41005834851 on `76653f32` exposed nine failures in the schema/queue partition.
Recovery admission still capped the unchanged inventory/live-gap contracts at schema 60, an old
version-16 fixture omitted the source path column, and the version-33 preservation comparison
did not account for the new derived FTS tables. The repair binds the compatibility horizon to the
current admitted schema while retaining the exact DDL/marker checks and rejection of future
versions. The fixtures retain complete path structure and separately validate the empty FTS index
before comparing all pre-existing rows.

Focused working-tree diagnosis passed 17 `current_schema_` checks, the version-16 upgrade, and the
version-33 retained-row upgrade/reopen check. These checks ran alongside the in-progress camera
metadata extension; hosted verification of the committed repair remains separate.

## Synthetic cost sample

One local unoptimized Rust diagnostic used 10,001 synthetic catalog locations in temporary storage.
The comparison selected one filename; timings are a single observation, not a device-independent
latency threshold. There were no source files or filesystem reads in this measurement.

| Measurement | Observed value |
| --- | ---: |
| Version-61 migration and backfill | 158 ms |
| Additional SQLite allocated pages | 2,146,304 bytes |
| Indexed gallery result-query VM steps | 115 |
| Previous literal count-query VM steps | 110,021 |
| Rename 10,000 paths, including index maintenance | 356 ms |

Both queries returned one match. The indexed query also selected the gallery's result columns and
applied its ordering; the literal comparison only counted matches, making it a conservative work
comparison rather than an identical end-to-end timing comparison. The regression requires at least
a tenfold reduction in VM work for this selective fixture. Broad or fewer-than-three-character
queries retain different costs, and source import/decoding costs are outside this measurement.

## Ownership and reviewability

Physical line counts include blank lines. Production counts exclude `cfg(test)` item ranges;
test-only owner ranges include helpers and module declarations. Existing large owners receive only
schema-version/module registration or validation delegation; index policy and SQL live together in
the new migration owner, with query policy in the small gallery search owner.

| Affected owner | Production | Test-only owner lines | Dedicated test files |
| --- | ---: | ---: | --- |
| `sqlite_catalog.rs` | 2,994 | 437 | Existing `sqlite_catalog/tests` tree: 11,292 lines, including the new 205-line text-search test |
| `migrations.rs` | 4,080 | 2,349 | New search migration tests: 199 lines; existing migration test modules retained |
| `migrations/current_schema.rs` | 328 | 29 | Existing dedicated schema tests: 36 lines |
| `migrations/gallery_search.rs` | 94 | 19 | 199 lines |
| `gallery/search.rs` | 31 | 0 | 205 lines |

The large pre-existing facades remain tracked architecture debt. This change adds no search policy,
tokenization, index SQL or branch-specific behavior to those facades.
