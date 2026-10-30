# ADR 0033: Share typed metadata filters across gallery workflows

Status: Accepted for validation
Date: 2026-10-30

## Context and decision

R5-01 needs structured search without changing literal text search or introducing a second gallery.
Add an Ame-owned metadata filter to the existing query: filename extension, inclusive byte/dimension
ranges, a calendar-date range with an explicit capture/created/modified field, and literal camera
make/model text. Existing source, folder and manual-album scopes compose with these conditions.
An extension describes the filename, not a verified encoding. Unknown metadata does not match a
condition on that field. Capture-date filtering uses capture evidence alone, without filesystem
fallback; filesystem dates use the same local calendar convention as their gallery ordering.

The domain owns bounds and valid calendar dates. The SQLite predicate owner uses bound parameters
before representative selection, shared by pages, counts, layout, selection and viewer anchors.
The application includes every condition in query identity. Filtering remains catalog-only and
bounded at result windows; broad predicates may inspect many catalog rows. No new media reads,
model dependencies, source mutations or per-asset queries are admitted.

Schema 64 adds one versioned, bounded filter payload per saved organization scope, with cascading
scope retirement. Migration supplies empty filters to existing scopes in one transaction. Active
and bookmark copies publish together with their existing scope/cursor transaction; a missing or
invalid payload fails rather than silently widening a saved query. This table is navigation state,
not classification or album decisions. Portable archive version 4 appends the payload to the saved
scope; readers of versions 1 through 3 supply empty metadata conditions. Restoration preserves an
existing local bookmark and imports the archived conditions atomically with a newly restored one.

The current Filter menu opens a standard desktop form for these conditions. Applying changes uses
the existing query transition; cancellation leaves the query intact. Clearing metadata conditions
retains source, album, text, duplicate and sorting choices. The form composes Material text fields,
dropdown and dialog primitives; no additional primary navigation or custom selection control is
needed. The native filter remains authoritative when requests bypass the form.

## Alternatives and consequences

Embedding operators in the search string would reinterpret literal filenames and require a second
escaping language. Filtering in Flutter would break bounded paging and counts. Adding predicates
only to ordinary gallery reads would change the meaning of full-result selections and continuation.
The shared typed query avoids these mismatches while adding bridge and saved-scope compatibility work.
Existing SQLite and serde dependencies suffice; licensing and runtime packaging remain unchanged.

## Validation and replacement

Verify compound/null/boundary predicates, invalid dates and ranges, literal special characters,
group representative selection, page/count/layout/selection agreement, query identity, migration
rollback, restart continuation, and the actual desktop apply/cancel/clear workflow. Preserve source
hashes. Native persistence and query modules own separate focused tests under ADR 0025.

A later forward migration can replace the bounded scope payload while preserving its versioned
meaning. Failed upgrades roll back; application rollback uses the existing pre-migration backup.
New predicates must join the same typed identity, saved scope and common query path.
