# R5 metadata and search evidence

The [roadmap](../roadmap.md#r5-implementation-sequence) owns R5 delivery status. This record covers
indexed text search under [ADR 0032](../architecture/0032-indexed-catalog-text-search.md) and camera
metadata under [ADR 0006](../architecture/0006-exif-capture-time-adapter.md). Structured predicates
and integrated R5-01 acceptance retain their separate remaining scope.

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

## Camera metadata - 2026-10-30

Scope: optional camera make/model, schema 61 to 62, compatible metadata reuse and invalidation,
generated desktop mapping, and the existing information sheet. The admitted parser retains the
128-byte field bound and independent failure handling. No new dependency or source-writing path is
introduced; the engine revision is `0.6.1+ame-camera-1`.

Focused local checks passed:

- Three parser cases cover Unicode, padding, missing fields, malformed type/encoding, multiple
  values, control characters, field bounds and independent preservation of valid capture time.
- Four migration cases cover retained catalog/history, unknown old values, rollback/retry across
  all three payload tables, bounded column admission and Unicode round-tripping.
- Six persistence/application checks cover row writing, hard-link evidence propagation, scan
  handoff reuse, same-identity metadata reuse and invalidation without opening source content.
- Two Dart mapping tests and eleven viewer tests pass, including camera fields in the existing
  information sheet and preservation through preview updates.
- The canonical bridge check verifies 17 asynchronous contracts and matching generated hashes.

The invalidation check initially reported `root_publication_namespace_guard_unsupported` from the
restricted local process before metadata publication. The same unchanged check passed with normal
workstation permissions using repository-contained temporary storage. No namespace guard or test
assertion was relaxed.

The Windows Debug client used a separate profile and two controlled images derived from the reusable
test corpus. Import completed with two visible images. The JPEG's preview information sheet showed
`Fixture Camera`, `Model 2`, `2026-10-30 12:34:56` and its image-time provenance. The PNG's card
information sheet showed unknown camera and capture fields. Both sheets retained aligned, selectable
information rows and fit the settled 1089-by-694 client surface. Searching the literal filename
suffix `jpg` returned one image, whose viewer showed `1 / 1`; clearing search restored both images.
The resulting catalog retained both values under schema 62. Both source SHA-256 values matched the
pre-run hashes after the owned client exited.

Hosted verification remains required for the committed camera change. The earlier search head's
10,000-image workload reported a 79,667,200-byte catalog against the retained 64-MiB limit; that
index-storage repair remains part of R5-01 and is not waived by these metadata checks.

### Camera ownership and reviewability

Physical line counts include blank lines; test-only counts include conditional test helpers and
module declarations. Metadata assembly, row decoding and physical-identity projection now have
dedicated owners. The existing transaction, generation and source-revalidation boundaries remain.

| Affected owner | Production | Inline test-only | Dedicated tests |
| --- | ---: | ---: | --- |
| `sqlite_catalog.rs` | 2,889 | 442 | Existing test tree: 11,299 lines |
| `migrations.rs` | 4,084 | 2,349 | Existing migration modules retained; camera migration below |
| `scan_publication.rs` | 643 | 65 | Existing test and pagination files: 1,012 lines |
| `incremental_library_changes.rs` | 1,340 | 2 | Existing dedicated test tree: 5,914 lines |
| `exif_metadata.rs` | 250 | 136 | New camera parser: 91 lines |
| `exif_metadata/camera.rs` | 51 | 2 | 91 lines |
| `migrations/camera_metadata.rs` | 66 | 18 | 129 lines |
| `location_row_read.rs` | 120 | 0 | Existing query and row-persistence tests |
| `scan_publication/identity_projection.rs` | 261 | 0 | Existing publication/identity tests |
| `incremental_library_changes/location_evidence.rs` | 142 | 0 | Existing reuse/invalidation tests |
| `library_asset.dart` / `library_asset_mapping.dart` | 145 / 65 | 0 | Existing mapping and viewer tests |

The Dart facades retain compatible exports: `library_models.dart` is 425 lines and
`library_catalog.dart` is 551 lines. Their new camera behavior lives in the asset value/mapping
owners, without extending unrelated scan or workspace policies.

## Compact text index - 2026-10-30

Scope: schema 62 to 63 retains each search document's integer identity directly on its catalog
location and retires the duplicate composite-key mapping. A unique integer index and transactional
triggers own allocation, path updates and deletion. The positionless term index stores no phrase
positions; at most 64 quoted trigrams reduce candidates before the existing literal predicate.
Source identity, camera evidence, organization decisions and history retain their authority.
The migration reads catalog rows only, rolls back incomplete identity backfill, and adds no VACUUM.

### Diagnosis and retained limits

The first positionless-only candidate retained the mapping table. Its optimized 10,000-image
cold/warm workload used 73,822,208 catalog bytes, exceeding the unchanged 64-MiB assertion.
A focused unoptimized run reported 73,035,776 allocated bytes, including 18,751,488 free bytes;
active search objects occupied about 3.2 MiB. Mapping rows duplicated long scan/location keys during
scan replacement, while existing gallery indexes retained partially occupied pages after retirement.
No existing gallery index, storage assertion or source check was removed.

After replacing the mapping with an indexed integer column, the same long temporary-base setup
used 69,480,448 bytes, including 17,002,496 free bytes. The terms occupied 1,269,760 bytes, document
sizes 143,360 bytes and the integer index 122,880 bytes. That run still failed the 64-MiB assertion:
fixture 10,201 ms, cold 26,769 ms, warm 19,418 ms, pause 20 ms, resume 24,455 ms and cancel 112 ms.
The resumed catalog occupied 38,940,672 bytes. Source-preservation assertions after the size check
were not reached in either failing workload.

The long diagnostic temporary base was explicitly redirected into workspace storage. Reusing the
same compiled test with the normal 37-character Windows temporary base passed every assertion:
66,310,144 catalog bytes (63.24 MiB), 37,404,672 resumed bytes, fixture 10,347 ms, cold 42,431 ms,
warm 23,813 ms, pause 17 ms, resume 39,991 ms and cancel 87 ms. Final active/all location and asset
counts were 10,000, cancelled rows were absent, both sampled source bytes matched and all 10,000
source entries remained. The retained absolute paths and their tokens explain part of the difference:
location pages were 5,881,856 bytes and term pages 696,320 bytes in this setup, versus 6,856,704 and
1,269,760 in the long-base run. No assertion or workload changed between these runs. The longer-base
failure remains recorded; the fixed 64-MiB synthetic assertion is not a universal limit for arbitrary
path lengths. These unoptimized diagnostics do not replace the optimized hosted resource gate.

### Boundary verification

Focused local diagnosis passed five compact-index migration checks and seven shared text-query
checks. Coverage includes stable keys, rejected reassignment, delete/reuse without old tokens,
upsert, transaction rollback, VACUUM, missing-identity rollback, retry/reopen, retained camera and
organization evidence, literal ordering/repetition, cross-path false positives, long-query suffixes,
Unicode/punctuation, scan publication and coherent page/count/layout reads.

Four original search migration checks and four camera migration checks also passed. The selective
fixture's current-schema result used 113 VM steps versus 110,021 for the literal count query, with
one hit in each; updating 10,000 paths took 185 ms. Its 140-ms upgrade and 2,146,304-byte increase
measure only the historical version-61 backfill, not the current compact schema.

Hosted run 41009578733 on `a30f1e37` also exposed two old version-36 fixtures that retained new camera
columns and two exact-module checks missing the admitted `media_metadata` declaration. The fixtures
now remove later columns through the existing downgrade chain, and the source guard lists the exact
new public module. Two version-36, four foreground-index migration and three module-contract checks
pass, including rejection of alternate/broader module loading. Production admission is unchanged.
The version-33 retained-row comparison accounts for retirement of the derived mapping table.

The rebuilt Windows Debug client upgraded the retained two-image isolated catalog from schema 62
to 63. Searching `jpg` displayed the one matching JPEG, and opening it retained a `1 / 1` viewer
scope. Returning and clearing search restored both images. The source completed synchronization,
camera make/model remained present, and both original fixture SHA-256 values matched after closure.
No user profile or original library was used. Exact-commit hosted verification remains outstanding.

The previous camera head's completed Catalog partition exposed eight duplicate-review failures:
its extra review fields still used offsets from the pre-camera projection. That separate decoder
repair remains required after this compact-index delivery; it is not covered by the search checks.

### Compact-index ownership and reviewability

Physical counts include blank lines. Existing facades register and validate the migration; integer
identity/index rebuilding and literal-candidate construction have separate owners.

| Affected owner | Production | Inline test-only | Dedicated tests |
| --- | ---: | ---: | --- |
| `sqlite_catalog.rs` | 2,889 | 442 | Existing catalog test tree retained |
| `migrations.rs` | 4,087 | 2,349 | Existing migration modules retained |
| `migrations/current_schema.rs` | 336 | 29 | Existing schema tests retained |
| `migrations/gallery_search.rs` | 95 | 19 | 200 lines |
| `migrations/compact_gallery_search.rs` | 138 | 27 | 224 lines |
| `gallery/search.rs` | 54 | 0 | Shared text-query tests: 274 lines |
| `scan_library/tests.rs` | 0 | 5,527 | New page/object accounting: 32 lines |

`local_files.rs` retains its 2,629-line runtime section, including pre-existing conditional test
hooks, and its 3,398-line inline test module. This change adds only six module-contract test lines;
its dedicated module-loading suite is 461 lines. No filesystem behavior or admission guard changes.
