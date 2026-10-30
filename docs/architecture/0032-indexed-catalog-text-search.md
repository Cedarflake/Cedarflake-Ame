# ADR 0032: Preserve literal gallery search with an FTS5 candidate index

Status: Accepted for validation
Date: 2026-10-30

## Context

The gallery searches absolute and relative paths with a case-insensitive literal substring
predicate. Pages, counts, layout, selection, duplicate-group membership and preview anchors share
that predicate. R5 requires indexed text search within these existing bounded query contracts.
Changing it to word matching, interpreting punctuation as operators, or ordering by relevance
would change existing behavior.

The catalog stores multiple scan versions of a location under the composite primary key
`(scan_id, location_id)`. Its implicit SQLite rowid is not a durable index identity: table rebuilds
or VACUUM can renumber rows without an integer primary-key alias.

## Decision

Use the bundled SQLite FTS5 trigram tokenizer to reduce candidates for literal searches of at least
three Unicode characters. Quote and bind the complete search string as an FTS phrase, then apply
the existing substring predicate to those candidates. This preserves SQLite's existing case rules
and the gallery's selected order. One- and two-character searches, and strings containing NUL,
retain the literal predicate because the tokenizer cannot faithfully represent them.

Schema version 61 adds a rebuildable `gallery_search_documents` mapping with an integer primary key
and a unique `(scan_id, location_id)` association. `gallery_search_terms` is a contentless-delete
FTS5 table with full token positions. The catalog remains the only stored path-text authority;
the index does not duplicate path columns as stored FTS content. The mapping key survives VACUUM.

Location insert, path/identity update and delete triggers maintain both projections in the same
catalog transaction. Unchanged path updates do not rewrite tokens. Queries match both scan and
location identities, while the existing source joins retain publication and availability authority.
An uncommitted or retired scan cannot become visible merely because it has index terms.

The migration streams existing catalog rows through SQL in one transaction under the existing
migration backup and admission policy. It reads no source media and preserves user decisions.
Table/trigger definitions participate in schema validation. Routine connections do not run a full
FTS integrity check or reconstruct the index.

Migration and query construction have separate owners under ADR 0025. The catalog facade registers
the migration; the shared gallery location filter delegates only its text predicate. No UI, bridge,
application or domain type exposes SQLite FTS syntax, rowids or shadow tables.

## Dependency and alternatives

The locked `rusqlite` 0.40.1 uses bundled `libsqlite3-sys` 0.38.1 / SQLite 3.53.2, compiled with
`SQLITE_ENABLE_FTS5`. No dependency or license changes are required. Contentless-delete tables require
SQLite 3.43 or later; using the bundled engine keeps packaging consistent. See SQLite's
[FTS5 documentation](https://sqlite.org/fts5.html) for trigram and contentless-delete behavior.

Word tokenization would lose substrings and short Chinese queries. LIKE/GLOB indexing would require
additional literal-escaping rules. An external-content index over implicit rowids would be unstable;
duplicating full path text in another ordinary table would increase storage without a second authority
being necessary. Continuing only the existing predicate would retain whole-scope text scans.

## Consequences and validation

Index construction adds catalog-only upgrade time and disk usage. Inserts and path changes incur
token-index writes; token positions are necessary for arbitrary-length phrase matching. Broad and
short searches can still inspect many catalog rows. SQLite performs candidate handling, ordering
and bounded result windows; Flutter never collects all hits to filter or sort.

Validation must cover literal punctuation and quotes, Chinese short/long searches, unchanged case
rules, composite identities, scan publication, rename/delete, transaction rollback, VACUUM, migration
rollback/reopen and preserved organization history. Query-plan evidence must show the FTS candidate
lookup. Existing group/count/layout/selection/anchor regressions remain applicable. Measure upgrade
and mutation cost on a synthetic catalog before accepting the storage/performance tradeoff; exact-head
CI and the affected desktop search workflow own integration acceptance.

## Replacement and rollback

The index contains only derived catalog text. A future forward migration can drop its triggers,
terms and mapping, then rebuild them from `asset_locations` without rescanning media or discarding
organization decisions. During replacement the application can use the literal predicate. Do not
downgrade a published database in place; a failed version-61 migration rolls back atomically and can
be retried, while application rollback uses the existing pre-migration catalog backup.

Replace this design if the measured indexing/storage cost interferes with scan publication or if
query compatibility cannot be preserved. Later metadata/category predicates remain typed filters
and do not add machine-result authority to this index.
