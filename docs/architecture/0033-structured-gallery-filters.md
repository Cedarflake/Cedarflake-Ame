# ADR 0033: Share typed metadata filters across gallery workflows

Status: Accepted for validation
Date: 2026-10-30

## Context and decision

R5-01 needs structured search without changing literal text search or introducing a second gallery.
Add an Ame-owned metadata filter to the existing query: filename extensions, inclusive byte/dimension
ranges, a calendar-date range with an explicit capture/created/modified field, and literal camera
make/model text. Existing source, folder and manual-album scopes compose with these conditions.
An extension describes the filename, not a verified encoding. Unknown metadata does not match a
condition on that field. Capture-date filtering uses capture evidence alone, without filesystem
fallback; filesystem dates use the same local calendar convention as their gallery ordering.
Format selections are a normalized, sorted set of at most 16 literal extensions. Selected formats
combine with OR; the resulting predicate combines with the other conditions using AND. Empty means
unrestricted. Each extension contains at most 16 ASCII alphanumeric characters.

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
Schema 77 migrates the single extension in payload version 1 to the format list in version 2,
preserving every other condition in one transaction. Archive version 7 uses this list; readers
continue to accept versions 1–6, converting their empty or single-format payload while staging.
Invalid payloads remain errors, and inspection never rewrites the input archive.

The original desktop form was withdrawn on 2026-11-02 because its custom presentation did not fit
the shared controls. R7 restores composed conditions through a standard Material bottom sheet, retaining
the descendant and duplicate-display shortcuts. A presentation-owned draft validates input without
changing the applied query; Apply publishes all conditions together, Cancel preserves them, and
the filter menu's summaries permit individual removal. Clear affects metadata conditions only. The
[gallery controls contract](../product/gallery/header-and-controls.md#filter-and-exact-duplicate-behavior)
owns the interaction.

When continuation supplies a current query, its metadata conditions take precedence before choosing
the saved or next image, alongside its existing sort precedence. Reads without a current query
retain the saved payload for compatibility. Editing does not rewrite organization decisions;
the format-list migration preserves active and bookmark scope state.

## Alternatives and consequences

Embedding operators in the search string would reinterpret literal filenames and require a second
escaping language. Filtering in Flutter would break bounded paging and counts. Adding predicates
only to ordinary gallery reads would change the meaning of full-result selections and continuation.
The shared typed query avoids these mismatches while adding bridge and saved-scope compatibility work.
Existing SQLite and serde dependencies suffice; licensing and runtime packaging remain unchanged.

## Validation and replacement

Verify compound/null/boundary predicates, invalid dates and ranges, literal special characters,
group representative selection, page/count/layout/selection agreement, query identity, migration
rollback, restart continuation with current conditions, and the shared filter menu/editor. Preserve source
hashes. Native persistence and query modules own separate focused tests under ADR 0025.

A later forward migration can replace the bounded scope payload while preserving its versioned
meaning. Failed upgrades roll back; application rollback uses the existing pre-migration backup.
New predicates must join the same typed identity, saved scope and common query path.
