# R5 smart-album query evidence

Manual confirmation/correction, review filters, deferral, continuation, and classification undo were
withdrawn on 2026-11-01. Earlier checks below retain their historical scope; current behavior follows
[the revised R5 requirements](../product/r5-classification.md). Legacy records and archives remain compatible.

The [roadmap](../roadmap.md#r5-implementation-sequence) owns delivery status and order.
[ADR 0035](../architecture/0035-classification-decision-authority.md#gallery-projection) owns the
effective-category and query contract. This record covers the R5-05 query integration; the sidebar
and connected review controls retain their R5-06 verification.

## Query boundary

The typed gallery collection carries an optional category and an independent review filter.
It shares physical scope, search, sorting, bounded pages, counts, layout and viewer anchors with
the existing gallery. Exact duplicates remain one group. All-results retains unclassified groups;
pending review includes uncertainty, deferral and human conflicts. A routine machine suggestion
is not a human confirmation. Manual albums, Favorites and organization queues remain independent.

Schema 70 introduces two read-only views over existing decisions and immutable predictions.
They select local human intent before restored intent and compatible machine evidence, preserving
conflicts without choosing a winner. The migration adds no source reads, copied decisions, model
files or portable-archive records. Current and retained display evidence follow the existing
gallery publication boundary; commands still require fresh source and field authority.

Classification writes already advance the catalog revision. The native synchronization root read
includes that revision, and the existing desktop refresh coordinator coalesces it through the
shared query-position owner. No additional polling loop or classification-specific scroll owner
is introduced. Classification continuation resolves the saved logical group and sort boundary
through the same membership query, without wrapping at the end.

## Verification on 2026-10-31

Added native regressions cover:

- category/review query identity, invalid scope rejection and mandatory duplicate folding;
- page/count/layout agreement, forward/backward navigation, physical/search scope and hidden-copy
  anchors;
- whole-group human correction and undo, later predictions, local/restored conflicts, changed
  content, source removal and semantic-rule replacement;
- refined rankings, deterministic ties, optional review and indexed human-lineage lookup;
- saved continuation across confirmation, deferral and restart;
- concurrent prediction publication, stale cursors, synchronization revision visibility and
  retention of the current image when a suggestion arrives before it;
- schema-69 upgrade rollback and rejection of altered projection definitions.

The dedicated native query suite contains 13 tests, the migration suite 2, and the query-identity
owner adds 1 test. These have compiled; their execution belongs to the exact-commit hosted Catalog,
Schema/Queue and Remaining jobs. They are not recorded as runtime passes here.

Owned Rust/Dart formatting and scoped Dart analysis passed. The focused Dart collection-mapping
test passed all nine category/review combinations and the three existing collection variants.
Canonical bridge generation completed; the following whitespace step initially encountered a
temporary mapped-file lock. Once the generator retired, its exact cleanup operation completed and
Rust formatting passed. The generator's trailing native Release build did not run in that attempt;
hosted bridge and Release checks retain that requirement.

Test-enabled Clippy completed compilation with warnings denied and no source diagnostics. The
last incremental pass reported a cache hard-link fallback to file copying. That workstation
invocation is not a clean warning-free gate; fresh hosted lint remains required. The native test
binary was not linked or executed locally after the earlier R5 memory failure.

A focused in-memory SQLite 3.50.4 diagnostic used the actual projection definitions and membership
template with isolated point bindings. It passed machine-only, human-override, human-conflict and
local-before-restored cases. The human plan uses the covering group-membership index and indexed
decision lookup. Increasing compatible prediction history from 1 to 1,001 rows changed the measured
statement steps from 188 to 177. This is a bounded SQL diagnostic, not a full gallery, migration or
large-library benchmark. Integrated scale and cloud-primary browsing measurements remain in R5-07.

## Ownership and physical size

Counts exclude generated bridges; dedicated suites are listed once rather than repeated for every
owner they exercise. The existing catalog/migration/domain facades only register the typed owner
or schema step; classification SQL and continuation policy live in their dedicated modules.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Classification membership / projection migration | 92 / 101 | 0 | 68 migration |
| Classification review facade / continuation / domain | 106 / 86 / 113 | 0 | Shared query suite: 757 |
| Gallery facade / filters / organization membership | 406 / 122 / 145 | 0 | Included in shared query suite |
| Query identity | 118 | 226 | 0 |
| Existing catalog / migration facade / current-schema validator | 3332 / 4167 / 404 | 0 / 2290 / 0 | Existing migration suites |
| Existing domain module | 846 | 0 | Existing domain suites |
| Dart catalog / review facade / collection mapping | 548 / 198 / 45 | 0 | 32 |

Actual desktop category entry, contextual commands, confirmation feedback and saved-position
interaction are verified with the connected controls in R5-06. This query record does not establish
those user-interface results or whole-stage acceptance.

## Retained catalog scale — 2026-11-01

At `c625135d`, an isolated copy of the retained local-primary catalog contains 63,077 locations and
47,833 folded gallery groups. The current native library migrates its schema from 45 to 73. Only
database timeline and layout-manifest APIs are invoked; no scanner, original-image reader, preview
recovery or classification worker runs. The retained baseline is preserved separately from this
disposable copy.

The scale fixture adds 43,644 synthetic immutable predictions across 14,548 distinct indexed
contents, with three history rows per content and alternating photograph/other suggestions. Every
fifth content receives a pending-review result. The fixture uses the current admitted definition/run
shape and passes foreign-key validation. It contains no human decisions, so the empty confirmed
scope tests coherent emptiness rather than human-override scale. Synthetic predictions measure
query cost, not classification quality.

The production bridge returns these results with merged duplicates and ascending filename order:

| Scope | Groups / locations | Timeline | First 128-item window | Next window |
| --- | ---: | ---: | ---: | ---: |
| Library | 47,833 / 63,077 | 3,502.497 ms | 2,291.454 ms | 1,255.459 ms |
| All results | 47,833 / 63,077 | 923.631 ms | 1,281.244 ms | 739.455 ms |
| Pending review | 2,884 / 5,906 | 2,695.066 ms | 2,879.571 ms | 1,605.068 ms |
| Confirmed | 0 / 0 | 2,286.365 ms | 2,481.815 ms | Not applicable |
| Other | 7,216 / 14,865 | 2,842.372 ms | 3,019.723 ms | 1,472.896 ms |

All count/window assertions pass: at most 128 identifiers per chunk, matching total counts, a second
start ordinal of 128 and no overlap. Process RSS increases by 5.94–6.65 MB from the initialized
bridge. These are sequential single-pass observations, with the library query first; they are not
repeated cold-cache comparisons. Migration preparation takes 53,062.590 ms and is excluded from the
table because it includes the schema upgrade.

The windows and memory are bounded, while filtered timeline/first-window calls still take about
2.3–3.0 seconds on local-primary. This leaves a measurable interactive query-cost issue to investigate;
the observations do not establish low-latency large-library browsing on cloud-primary. Diagnostic
sources, fixture metadata and the complete result lines remain in the ignored
`build/test/derived/r5-smart-album-scale` directory.

After the query process retires, exclusive file-open checks and contained/reparse-free path checks
precede removal of the disposable profile and its migration backup, reclaiming 870,670,372 bytes.
The retained baseline's SHA-256 remains
`93db288e598b601fb478d39ff598777a1d5f1615f152c15889bddc6fa7aecbfd`.

## Unused review-count queries — 2026-11-01

The complete category-snapshot path at `f0c3b1bd` still executes three additional grouped queries
for total, pending and confirmed classification counts after reading the page and timeline. The
withdrawn review interface has no consumer for these totals. Removing this calculation, its bridge
field and Dart state leaves visible-scope timeline counts, legacy category precedence, query
membership and the snapshot transaction unchanged. No schema or stored records change.

The boundary regression uses 2,051 physical locations in two classified groups. Before the repair,
the combined read uses 3,965,100 SQLite instructions, compared with 759,100 for its page and 1,213,600
for its timeline. After removal, it passes the page-plus-timeline instruction budget. All 19 focused
classification query tests pass, including legacy choices/conflicts, physical scope, retained display,
publication coherence, bounded pages and anchors. The generated bridge retains 31 asynchronous
contracts with matching content hashes; the repository format check passes.

The same prepared retained-catalog copy and synthetic predictions described above are reused for
complete production-bridge snapshots of `Other`, in ascending filename order with a 128-item page:

| Sequential call | Before | After |
| --- | ---: | ---: |
| First | 7,709.141 ms | 8,903.180 ms |
| Second | 5,852.290 ms | 3,923.616 ms |
| Third | 5,839.953 ms | 4,024.622 ms |

All six results contain 7,216 groups, 14,865 physical locations and the same ordered 128 identifiers.
The latter two calls average 5,846.122 ms before and 3,974.119 ms after, a 32.02% reduction in this
sequential comparison. First-call time did not improve; the runs do not isolate native startup or
disk-cache effects. The remaining complete-read latency still needs optimization. These results
establish neither instant category switching nor a cloud-primary latency bound. Observed process RSS
growth is 6.31–7.77 MB before and 7.31–8.07 MB after.

The rebuilt isolated Profile client opens its ordinary Library with 51 files / 50 folded groups and
loads `Other` with 28 images through the new bridge. Further preview input is blocked by another
application's Windows location-permission prompt; no action is taken on that prompt. Nine durable
classification/R4 table counts and digests match the pre-run baseline after the connected read.
Existing preview evidence covers unchanged preview actions; this run adds no new preview acceptance.

The scale process invokes only database query APIs, with no scanner, source-image reader or model
worker. After process retirement, exclusive-open and contained/reparse-free checks precede removal
of its disposable profile and migration snapshot, reclaiming 870,637,604 bytes. The retained baseline
SHA-256 remains unchanged. Small diagnostic sources and before/after result lines remain in the
ignored scale directory. Submitted-head hosted checks remain required.

Affected large-owner sizes are 285 production / 27 test-hook lines for the native snapshot facade,
548 / 0 for the Dart catalog adapter, 1,301 / 0 for the viewport controller, and 406 / 0 for its state.
The dedicated count/cost test module has 135 lines. These owners only lose unused state and work;
no new lifecycle or query-policy responsibility is added.

## Integrated retained-library acceptance — 2026-11-01

The retained local-primary baseline is copied into disposable storage and migrated from schema 45
to 73 with the native library at `7089ae0e`. The copy contains 63,077 locations and 47,833 merged
groups, with the same 43,644 synthetic predictions over 14,548 contents described above. Migration
preparation takes 54.9 seconds and is excluded from ordinary query measurements. No source scan,
image read, model worker, cloud transfer or desktop-window interaction runs in this acceptance.

Eleven production-bridge scopes pass count, 128-item window, page/manifest order, forward/backward
paging and revision-coherence checks. They cover merged/all-file browsing, ascending/descending
names, capture/modified time, two categories, PNG filtering, empty/literal search and folder scope.
The viewer anchor remains present and a cursor from another query is rejected.

| Complete snapshot scope | Groups / locations | Elapsed |
| --- | ---: | ---: |
| Merged Library, filename order | 47,833 / 63,077 | 2.10–3.84 s |
| All files | 63,077 / 63,077 | 0.71 s |
| Other, filename/modified-time order | 7,216 / 14,865 | 3.26–4.43 s |
| Photograph, capture-time order | 7,216 / 14,811 | 5.30 s |
| Other, PNG filter | 2,202 / 5,013 | 3.08 s |
| Empty search / literal path / exact folder | 0 / 1 / 1 groups | 1.06 / 1.18 / 1.13 s |

Selecting all matching results through the native batch draft changes exactly 7,216 groups for
both Favorite and album assignment. Favorite preparation/application takes 3.17/5.86 seconds;
album preparation/application takes 4.15/3.40 seconds. Undo empties the album, redo restores it, and
subsequent album/Favorite queries agree with the category's 7,216 groups and 14,865 locations.

The exported archive contains 283,514 records and is 55,896,748 bytes. Inspection matches all 7,216
groups. Restoration into the same isolated catalog preserves existing decisions; a separate process
then verifies album, Favorite and category counts, page order and backward navigation. This exercises
large-scale replay and preservation, while missing-record import retains its existing focused coverage.

The run exposes per-record autocommit in the private staging database. A single staging transaction
now covers streamed insertion and validation, keeping the bounded page cache, cancellation points,
input lock, validation and separate live-catalog transaction. The same archive gives:

| Operation | Before | After staging repair |
| --- | ---: | ---: |
| Export | 8.36 s | Unchanged implementation; not repeated |
| Inspect | 191.97 s | 11.41 s |
| Restore | 204.08 s | 16.84 s |

After the repair, inspection reaches matching at 5.22 seconds and restoration reaches its write
phase at 4.82 seconds. Restoring under a new operation identifier rejects the old catalog cursor
and preserves all three queried collections. These sequential observations use the same archive
and retained copy; they do not establish cold-cache or cloud-primary latency bounds. The repaired
native library SHA-256 is `117858644f0260bb56bd801816f09ef4ebec7bdc03799264bbf4397cc0123752`.

All 14 focused staging tests pass, including cancellation during streamed insertion and after the
last record, scratch retirement, subsequent successful inspection, legacy formats and malformed
relationships. The pre-repair large-file cancellation probe returns 130 ms after its request.
Direct Dart VM sampling during restart/cancellation/repaired recovery peaks at 340.8 MiB working set
and 323.1 MiB private memory. Earlier launcher-only memory samples are excluded.

Final SQLite quick-check and foreign-key validation pass. All 63,077 location identity tuples match
the retained baseline, and all 43,644 predictions and 7,216 organization records remain present.
The original baseline retains SHA-256
`93db288e598b601fb478d39ff598777a1d5f1615f152c15889bddc6fa7aecbfd`.
After owned processes retire, contained/reparse-free path validation and exclusive-open checks
precede deletion of the disposable catalog, migration snapshot and exported archive, reclaiming
1,032,895,184 bytes. Small diagnostic sources and results remain in the ignored scale directory.

Complete category snapshots after organization/recovery still take 7.40–7.95 seconds; the data checks
pass, but this interactive query cost remains open. Synthetic predictions provide no new model
accuracy evidence. The previously requested pause on window control remains respected, so this run
adds no scrolling, preview-rendering or final silent-indicator visual acceptance.

The staging owner has 179 production lines, no inline tests and 566 dedicated test lines. The
change adds no schema, archive-format, source-file or durable-decision policy change. Hosted checks
must cover the submitted repair; prior-head results cover unchanged inputs only.

## Local acceptance storage retirement — 2026-11-01

The storage inventory after integrated acceptance finds 53,184,983,683 bytes in Rust debug
incremental compilation directories and 18,017,695,788 bytes in obsolete R2c/R4 experiment catalogs,
migration copies and exported archives. One retired R4 diagnostic archives the complete profile
after each failed attempt, multiplying copies of the same large catalog. Small logs do not account
for the growth. These are development and acceptance artifacts, separate from application cache.

Cleanup removes 15,936 files, totaling 71,202,679,471 logical bytes. The measured increase in free
disk space is 65,908,563,968 bytes; logical file sizes and reclaimed filesystem allocation are
reported separately. Each target is workspace-contained, reparse-free, inactive and exclusively
openable before removal. Cargo and repository-owned application processes are absent. File
attributes are refreshed before comparing inventory metadata with the deletion preflight.

The reusable image corpus, models/calibration artifacts, retained baselines, current R5 client
profile, diagnostic scripts and small reports remain. The retained catalog still has SHA-256
`93db288e598b601fb478d39ff598777a1d5f1615f152c15889bddc6fa7aecbfd`.
No source media, application window, live catalog or unresolved recovery backup is modified.

The repository now disables Cargo incremental compilation while retaining ordinary compiled-artifact
reuse. The [Cargo configuration contract](https://doc.rust-lang.org/cargo/reference/config.html#buildincremental)
defines this switch; edited-crate rebuilds may take longer. The local retained-library diagnostic
runner now leases one named scratch directory and removes its disposable profile, migration copies
and archives on success or failure. Logs and reusable inputs remain outside that directory.
The shared owner and its usage requirements live in
[execution and storage safeguards](quality-gates.md#execution-scope-and-storage).

Focused checks cover success, action failure, invalid names, occupied files, interrupted-run
reclamation, concurrent ownership, junction rejection and preservation of unowned directories.
A dependency-free offline Cargo compilation produces no incremental cache and leaves no scratch
outputs. The retained-library Python storage integration also passes without copying the catalog
or opening source images. The modified diagnostic Dart files pass analysis. Full large-library
acceptance is not repeated for this storage-only change; its prior data-flow evidence remains valid.

The cleanup owner has 105 production lines and no inline tests; dedicated checks have 135 lines.
The 75-line lint facade adds only the test registration. Hosted checks must cover the submitted
configuration and tooling changes.

## Shared prediction reads and scoped counts — 2026-11-01

The category predicate previously decoded the same compatible prediction for each physical copy.
Collection reads now materialize the latest compatible prediction per content once per statement,
using the existing authoritative view. Point reads and collections restricted by root, folder or
search retain indexed prediction lookup. Human/legacy precedence, compatible retained membership,
current configuration, publication order and source-authority checks remain unchanged.

Filename sections also reuse the grouped query's physical count, as date buckets already do.
This avoids a separate category scan for the source total. Empty results, preferred copies,
cross-root location identifiers and scoped source counts retain their existing semantics. There is
no schema, bridge, model, archive-format or source-mutation change.

The same retained fixture has 63,077 locations, 47,833 merged groups and 43,644 synthetic historical
predictions over 14,548 contents. Two sequential SQL rounds reduce the category predicate from
1.118–1.164 seconds to 0.739–0.744 seconds, with the same 14,865 physical members. Selecting the
latest result independently per location did not improve this query and was not adopted.

The connected optimized native library, including isolated-storage support, has SHA-256
`9e80c25e5b8a8d23de9d5c232c5bdae06394ec9cfef435f369cf84eeeeabcb02` for the broad-query comparison.

| Native read | Previous observation | Updated observation |
| --- | ---: | ---: |
| Category snapshot after first bulk Favorite | 4.119 / 3.953 s | 3.869 / 3.699 s |
| Category snapshot after organization and archive replay | 7.40–7.95 s | 3.384 s |
| First snapshot after process restart, opening the album | Existing admission cost remains | 9.688 s |

The first 128 ordered locations and the category's 7,216 groups / 14,865 physical locations match.
Eleven gallery scopes retain bounded windows, count/revision coherence, layout order and forward /
backward navigation. Viewer anchoring and foreign-cursor rejection pass. Bulk Favorite and album
assignment each affect the expected 7,216 groups; undo, redo, archive replay and a subsequent process
preserve album, Favorite and category membership. This run includes one additional no-op Favorite
operation before archive export; it is not an identical archive timing comparison.

Bulk latency remains open: first Favorite preparation/application takes 3.785/8.992 seconds, and
album preparation/application takes 6.514/6.461 seconds. These are sequential workstation observations,
without a controlled CPU-load or cold-cache comparison. They do not establish a cloud latency bound
or responsive first opening. Category-query improvement does not close the remaining admission and
bulk-action performance work.

Actual Dart VM sampling peaks at 343.1 MiB working set and 347.5 MiB private memory. SQLite quick-check,
foreign keys, all retained source identity tuples and the baseline digest pass. The 43,644 predictions
and 7,216 organization records remain present. Owned processes retire before the leased disposable
catalog, migration backup and archive are removed. No source images are opened in the successful run.

Twenty classification query regressions and five organization count/section regressions pass.
The cost regression adds 65 historical predictions behind 256 duplicate copies and checks collection,
unrelated single-image and root/search reads. A local rerun initially failed before fixture creation
because the system temporary directory denied access; the unchanged tests pass in a leased workspace
temporary directory, which is cleaned afterward.

Two earlier attempts used an ordinary Release DLL, which ignores the debug-only test-storage
override. They opened the configured catalog and returned `catalog_change_catch_up_contract_unverifiable`
before gallery results or batch actions. That catalog's modification time changed during admission;
there is no pre-run logical snapshot for a before/after comparison. These attempts are excluded from
acceptance. The diagnostic runner now requires an isolated-storage build receipt matching the DLL's
hash before starting its child. The successful run above uses the previously established optimized
build command with `-C debug-assertions=yes` and its bound receipt. Window control remains paused.

The final restricted-scope probe uses native SHA-256
`58c31047f3000b49c82c4cdc046c06b59565770c284f336301149d0cb6d25fcd`.
Returning root/folder/search queries to indexed prediction reads reduces the sampled literal search
from 1.986 to 0.898 seconds and exact-folder snapshot from 2.060 to 0.884 seconds. The empty query
returns zero groups and files in 0.865 seconds. The unrestricted category remains coherent at
3.963 seconds, preserving the same ordered first page, counts and forward/backward navigation.
Source identity, foreign keys, quick-check, baseline digest and scratch retirement pass again.
The final focused probe peaks at 304.3 MiB private memory. The broad-query/recovery results above
remain applicable because this last change only selects indexed reads for restricted scopes.

The prediction-query owner has 133 production lines, filename/arrival sections 142, and timeline
reads 196; all have no inline tests. The two modified dedicated test owners contain 207 and 137 lines.
Full hosted gates still apply to the submitted commit. The configured-catalog handoff-validation
refusal observed in the excluded attempts remains a separate diagnostic finding; it was not repaired
or bypassed during this query optimization.

## Confirmed-group command lookup — 2026-11-02

SQL profiling of the retained 7,216-group Favorite operation separates member validation,
binding publication, undo-history writes and commit. The Debug trace takes 5.52 seconds to prepare
and 12.38 seconds to apply; final commit accounts for 0.26 seconds. These instrumented measurements
locate work and are not desktop or optimized-build latency claims. Statement reuse fits within the
existing 64-entry cache, and the inspected plans retain indexed access to durable decision history.

The confirmed-member lookup nevertheless starts with the file-size index before resolving the
content fingerprint. Adding 2,048 unrelated same-size locations increases one unchanged group's
read from 200 to 55,600 SQLite VM steps. Changing join order alone still visits those locations,
using 25,000 steps. The final query starts with the requested fingerprint's generations and pins
the existing location-generation index. Current-location authority joins remain shared with ordinary
queries; identity/revision compatibility, representative preference and pending-evidence rejection
are unchanged. Provisional singleton queries retain their existing lookup. There is no migration,
bridge, cache-budget or source-media change.

The same-size regression now bounds member-read and complete Favorite-write cost while checking the
same two members and unchanged unrelated records. Fifteen batch regressions and the retained-display
write-rejection case pass after the final index correction, including cancellation, whole-operation
rollback, stale proposals, frozen-selection refresh, conflict resolution and undo. The preceding
query-order variant also passes 161 organization checks; its one failing cost regression is the
reason for the explicit index correction, and is not reported as a passing full final-head suite.
Full hosted gates remain required for the submitted change.

The optimized native build with isolated-storage support has SHA-256
`4efcfc47956a5d0d19e570a267410542d7a3cc718a0a66aa8efde136d3293653`.
It completes the same retained 63,077-location fixture, with 43,644 synthetic prediction-history
rows. Observed command times are:

| Command over 7,216 groups | Preparation | Application | Changed groups |
| --- | ---: | ---: | ---: |
| First Favorite | 1.625 s | 5.437 s | 7,216 |
| Repeated Favorite | 3.206 s | 2.055 s | 0 |
| Album assignment | 3.299 s | 3.382 s | 7,216 |

Category snapshots take 2.298/2.423 seconds and retain the same first 128 ordered locations and
7,216-group / 14,865-location counts. Album/Favorite paging, undo/redo, archive replay and restart
remain coherent. The post-restart album snapshot takes 3.072 seconds and category snapshot 3.574
seconds. Unchanged migration/query work also runs faster in this session, so the wall-clock values
remain sequential observations rather than an isolated speedup percentage or cloud guarantee.
The deterministic same-size regression establishes the repaired query-cost growth.

The action/restart phases peak at 317.1 MiB private memory. SQLite quick-check, foreign keys, all
63,077 source identity tuples and the retained baseline digest pass. The archive round trip retains
7,216 organization records, one album and 43,644 predictions. Original media is not read or changed;
the leased catalog, migration copy and archive are removed after all owned processes retire.
First-admission, complete-snapshot and remaining per-group write costs stay in the roadmap's
performance step; the configured-catalog handoff refusal and paused window checks remain open.

The temporary SQL tracer has been removed from product/test compilation. Its three diagnostic-only
compiled artifacts totaling 632,763,931 bytes are removed after process retirement; small logs and
the diagnostic source remain in ignored storage. Leased catalog copies retire after each run.

The shared exact-query owner has 181 production lines and the selection owner 260, with no inline
tests. The focused dedicated regression has 103 lines; existing organization test ownership remains
unchanged.

## Terminal scope-recovery handoff — 2026-11-02

Read-only inspection of the configured local-primary catalog identifies 147 retained handoff rows
under one journal range. All have terminal queue evidence and a completed ordered scope-recovery
parent; no active queue, frozen scan, pending rename or pending cross-root owner remains. The scoped
recovery completion path omitted the handoff release performed by ordinary queue completion. The
retained rows therefore fail the next catalog admission's ownership check.

Parent completion now releases handoffs atomically. A dedicated retirement owner preserves the
existing shared-owner and derived-data reclamation rules. Compatibility recovery requires the
completed parent and correctly bound ordered children, retains active shared owners, and validates
the entire catalog before commit. Unproven handoffs and contradictory children remain rejected;
another validation failure rolls back the known repair as well. Recovery also precedes older-schema
upgrade validation and has no schema-version, bridge, dependency or source-media change.

Eight focused completion/recovery regressions pass, covering ordinary completion, cleanup failure,
shared ownership, idempotent reopen, schema-72 upgrade, contradictory children and rollback with an
unproven handoff. Three existing regressions pass for retained other-watermark ownership, unknown
normalized batches and incorrect foreign-key targets. Full hosted gates apply to the submitted head.

The optimized isolated-storage native library has SHA-256
`eb7c9fd187541b07ab66e378baf55ca7f911f624223b19847a81d9d4bb9161ef`.
A leased backup of the affected current-schema catalog opens successfully through the native
gallery API, then opens again in a fresh process. Both runs report 63,084 physical files and 47,840
merged groups, with two non-overlapping 128-item windows. The complete headless processes take
5.87/5.64 seconds; these include initialization and three gallery reads, not desktop frame timings.
Peak private memory is 312.8 MiB.

The repair removes the 147 terminal snapshots and 20 otherwise unreferenced derived asset rows.
Protected table digests preserve every image location, root, scan, queue/lineage, journal range,
organization record and classification record. SQLite quick-check and foreign keys pass. The original
catalog's file hash is unchanged, original media is not opened, both child processes retire, and the
leased catalog copy is removed. Existing window-control restrictions remain respected.

The extracted retirement owner has 135 production lines plus 56 test-only helper lines; scope recovery
has 283 production lines, compatibility recovery 82, and its current-schema coordinator 86. These
owners contain no inline test assertions. The new dedicated tests and fixture contain 123 and 187
lines respectively; the delta facade loses the retired responsibility instead of gaining another
recovery branch.
