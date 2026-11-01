# R5 smart-album query evidence

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
