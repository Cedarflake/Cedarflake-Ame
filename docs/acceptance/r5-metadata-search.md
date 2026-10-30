# R5 metadata and search evidence

The [roadmap](../roadmap.md#r5-implementation-sequence) owns R5 delivery status. This record covers
indexed text search under [ADR 0032](../architecture/0032-indexed-catalog-text-search.md) and camera
metadata under [ADR 0006](../architecture/0006-exif-capture-time-adapter.md), plus shared structured
filters under [ADR 0033](../architecture/0033-structured-gallery-filters.md). Integrated R5-01
acceptance retains exact-head hosted storage and integration checks after the local reclamation repair.

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

The previous camera head's completed Catalog partition exposed eight duplicate-review failures.
Their separate decoder repair is recorded below; the compact-index checks do not cover that path.

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

## Duplicate-review camera projection - 2026-10-30

Hosted Catalog job 123912239183 reported eight failures after camera columns extended the shared
candidate projection. Duplicate review appended group/decision fields but still read them at the
old fixed offsets, interpreting namespace identity text as group byte length. The existing group
query now aliases these appended fields and reads those names, retaining unsigned-value checks,
source admission, count/page snapshot consistency, preference and ignore semantics.

Focused diagnosis passed nine review/seek checks and the group/member boundary-page check. The
preferred-copy restart test now carries non-empty camera make/model and verifies the representative,
group digest/length, physical/location counts and saved decision together. This covers all eight
reported failures and retains stale-decision, no-op revision, invalidated-evidence, ignored-list,
empty-list and direct-position behavior. Rust formatting and diff checks pass.

The affected production owner `exact_duplicate_review/groups.rs` is 154 lines with no inline tests;
the existing dedicated review and seek suites are 389 and 139 lines. The query/decoder remains in
its existing cohesive owner. No schema, bridge, model dependency or source mutation changes.
The rebuilt Windows Debug client used the existing isolated metadata profile plus one exact copy
of its controlled JPEG fixture. The Library showed three images and two merged items. Duplicate
review displayed one group with two locations and opened both members. Setting the original JPEG
as preferred changed its control to `取消首选`; returning to the group list displayed that JPEG as
the representative with unchanged group/location counts. Both original fixture hashes and the
copy's hash remained unchanged. No original library or user profile was used. Exact-commit hosted
checks remain pending.

## Structured metadata conditions - 2026-10-30

The shared gallery query now carries filename extension, byte/dimension ranges, a selected calendar
date field, and literal camera text. Conditions apply before duplicate representative selection and
compose with source, folder, album, text and sorting scope. Unknown dimensions and capture dates do
not match conditions on those fields. Schema 64 retains the same conditions in active organization
sessions and bookmarks. Portable archive version 4 preserves them; versions 1 through 3 expand to
empty conditions without changing their existing scope. No model or additional source reads are used.

### Focused verification

Fifteen affected native checks passed: five compound/group/query/continuation checks, two migration
and rollback checks, two domain validation checks, one query-identity check, and five archive checks.
The archive checks cover versions 1–3, rejection of invalid version-4 payloads, restored selected
sorting/filter conditions, and preservation of an existing local bookmark/session. The final query
run includes the maximum-only width case, so unknown dimensions cannot satisfy that condition.

The first broad identity name filter also selected an unrelated source-admission test, which failed
at the sandbox's namespace ACL boundary. Its affected query-identity test passed. Archive diagnosis
initially failed when sandbox permissions denied access to temporary backup files and was stopped.
The five relevant archive cases then passed with normal local file permissions under the repository
lock. No production guards or assertions were changed for either execution limitation.

Two editor widget checks and the existing asynchronous filter-menu semantics check passed. They
cover invalid calendar dates and inverted ranges, exact byte conversion, typed bridge round-trip,
scope-preserving apply/cancel/clear, and the filtered-empty message. Rust and Dart formatting pass;
the generated bridge and Windows Debug client were rebuilt from the changed source.

The actual Windows client reused the three-image isolated profile and upgraded schema 63 to 64.
Its menu opened the complete form at the retained desktop window size. Applying `png` reduced the
gallery to one image and the viewer to `1 / 1`. Editing maximum width to `1` and cancelling retained
that PNG; reopening showed the unsaved width absent. Clearing conditions restored three physical
images and two merged items. Closing the owned window preserved all three fixture SHA-256 values.
The client log contained no matching exception, panic or failed-root entry. Original libraries and
user profiles were not used.

### Component admission and ownership

The form composes Flutter 3.47.8 `AlertDialog(scrollable: true)`, `TextFormField`,
`DropdownButtonFormField(initialValue: ...)`, and `showDatePicker`, inspected in the installed SDK.
The [Material dialog guidance](https://m3.material.io/components/dialogs/overview),
[dropdown API](https://api.flutter.dev/flutter/material/DropdownButtonFormField-class.html), and
[date-picker API](https://api.flutter.dev/flutter/material/showDatePicker.html) identify the framework
owners. The current menu remains the entry; a separate page would split the shared gallery workflow.
A standard form supplies validation, focus, scrolling and calendar selection without custom input
primitives. Existing menu dots, theme, buttons and empty-state composition are reused.

Physical line counts include blanks. New query predicates, saved-filter persistence, archive
validation and editor state have separate cohesive owners; existing facades only compose them.

| Owner | Production | Inline test-only | Dedicated tests |
| --- | ---: | ---: | --- |
| `domain/gallery_metadata_filter.rs` | 114 | 2 | 82 lines |
| `gallery/metadata_filters.rs` | 64 | 0 | Catalog 209; grouped selection 49 lines |
| `migrations/organization_review_filters.rs` | 82 | 14 | 72 lines |
| `organization/review/filters.rs` | 54 | 0 | Restart/continuation 41 lines |
| `organization_archive/staging/metadata_filter.rs` | 22 | 0 | Existing staging suite 470 lines |
| `application/load_catalog/query.rs` | 107 | 202 | Existing inline identity cases |
| `library_metadata_filter.dart` / mapping | 84 / 52 | 0 | Editor/bridge suite 118 lines |
| `library_metadata_filter_editor.dart` | 304 | 0 | Editor/bridge suite 118 lines |

The catalog facade remains 2,889 production / 442 inline test-only lines; migrations has 4,090 /
2,349 and current-schema validation 342 / 29. The filesystem facade retains 2,629 production /
3,404 inline test-only lines, with only its exact-module contract updated. The 1,301-line viewport
controller adds one normalization call; the 433-line gallery model composes the typed filter value.
No unrelated controller or filesystem behavior was added.

### Hosted storage failure before reclamation repair

Run 41015378674 for branch head `17270938` (PR merge revision `b5ff9a7b`) still fails the unchanged
10,000-image 64-MiB catalog assertion. Its isolated optimized workload reports 69,484,544 catalog
bytes, 16,969,728 free bytes and 39,129,088 resumed-catalog bytes. Fixture creation took 1,421 ms,
cold scan 9,197 ms, warm scan 11,958 ms, pause 11 ms, resume 7,919 ms and cancellation 89 ms.
The retained artifact is `synthetic-scan-41015378674-1`, ID 12960146812. Source checks after the size
assertion were not reached. This repeats the long-base storage regression and remains an R5-01
obligation at that head; structured-filter verification does not resolve it or complete the hosted gate.

## Post-scan storage reclamation - 2026-10-30

The compact index's live term data is only part of allocated database size. The failed hosted
catalog retained 16,969,728 free bytes after old scan versions were deleted. The application already
schedules background reclamation after scan retirement, but its previous 64-MiB/25-percent start
threshold left this slack untouched. The standalone scan fixture also disabled the application
worker and drove only retired-row deletion. Its allocated-file measurement therefore omitted the
desktop's post-scan maintenance lifecycle.

Catalog-only comparisons used SQLite 3.47.1 and two 10,000-row generations with the actual location
table/index definitions. They were diagnostic comparisons, not bundled-engine acceptance. An
external-content index saved 270,336 bytes; delayed publication indexing saved 765,952 bytes;
preassigning integer identities saved no space. None was adopted. Their 311,533,568 bytes of disposable
databases were removed after process retirement; the small scripts/results remain ignored.

The [storage policy](../architecture/0005-storage-governance-and-budget.md#catalog-database-page-reclamation)
now admits bounded incremental reclamation at 16 MiB and 20 percent free space. Legacy full-rewrite
admission remains at 64 MiB and 25 percent. The existing 256-page windows, foreground preemption,
validated-session preservation, 8-MiB/five-percent stop conditions and final WAL checkpoint remain.
The policy has its own owner; no migration, index replacement, source read or synchronous full VACUUM
was added. The synthetic fixture invokes the real background worker after retired-row cleanup,
requires completion within ten seconds, and reports pre/post allocation without concealing the peak.

Twenty-seven focused native reclamation checks passed under the repository lock. They cover the
new byte/ratio boundaries and hysteresis, actual small-catalog reclamation with retained indexed
search and album history, unchanged source bytes, no smaller legacy rewrite, cancellation,
foreground preemption, interleaved reads, session validation and worker-generation ownership.

The changed 10,000-image workload then passed all assertions using a 112-character workspace
temporary base, longer than the failed hosted base. This was an unoptimized diagnostic build,
not the optimized hosted performance result.

| Measurement | Result |
| --- | ---: |
| Allocated catalog before reclamation | 69,722,112 bytes (66.49 MiB) |
| Final catalog after reclamation | 60,272,640 bytes (57.48 MiB) |
| Reclaimed free pages | 9,437,184 bytes |
| Remaining free pages | 7,774,208 bytes |
| Reclamation | 442 ms |
| Fixture / cold / warm | 8,246 / 27,166 / 19,855 ms |
| Pause / resume / cancel | 24 / 24,891 / 88 ms |
| Resumed catalog | 39,215,104 bytes |

The final active/all location and asset counts were 10,000, cancelled rows were absent, both
sampled source contents matched and all 10,000 source entries remained. The original 64-MiB final
storage and scan/control timing assertions remain. The allocation before reclamation still exceeds
64 MiB; no peak-allocation pass is claimed. Exact-head hosted verification remains outstanding.

Physical ownership: `catalog_reclamation.rs` is 427 production lines plus two test-module lines;
`catalog_reclamation/policy.rs` has 31 production and 44 inline-test lines. The dedicated parent
suite is 347 lines, the new small-catalog suite 127, and the scan maintenance helper 62 test-only
lines. The scan test facade adds one production-maintenance invocation. No UI, bridge, source-safety,
publication or logical-cleanup behavior changed.
