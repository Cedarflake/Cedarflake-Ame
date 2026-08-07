# ADR 0008: Capture-time keyset for the unified gallery

- Status: Accepted
- Date: 2026-08-07

## Context

R1 loads active catalog locations in root-creation and relative-path order. That order is stable for
catalog validation but cannot support the R2 continuous timeline: adding another root creates a
separate block, date headings are not contiguous across roots, and a time rail cannot address the
complete result set.

Capture metadata is intentionally evidence-preserving. Many images have a normalized local capture
time but no trustworthy timezone offset. File modification time exists for every accepted location,
but it is not capture-time evidence and must not silently become one.

The real two-root acceptance catalog contains 79,013 active locations. A gallery query must remain
bounded, deterministic across equal timestamps, revision-safe during publication, and independent
of root import order.

## Considered options

### Keep root and relative-path ordering

Rejected. It cannot produce one cross-root timeline or a meaningful date distribution.

### Convert every capture value to a UTC instant

Rejected. Offsets are absent for a substantial class of personal images. Inventing an offset would
turn uncertain metadata into false chronology and make date headings surprising.

### Substitute file modification time when capture time is absent

Rejected as a display meaning. Modification time remains useful only as a deterministic ordering
key inside an explicitly unknown-capture-time section.

### Order by normalized local capture time with an explicit unknown section

Accepted. A personal gallery primarily presents the local date recorded by the camera or source.
Known local capture values sort newest first across every active root. Unknown values follow in a
separate section and sort by modification time without being relabeled as captures.

## Decision

The default unified-gallery key is:

1. capture-time-missing rank, ascending (`known` before `unknown`);
2. normalized local capture time, descending;
3. file modification time, descending;
4. root identity, ascending;
5. location identity, ascending.

The final two keys provide deterministic ordering when timestamps are equal. The cursor carries the
catalog revision and every ordering key. A publication invalidates an older cursor before any page
is returned.

SQLite schema v11 adds an index matching the gallery sort expressions. The migration is additive:
it does not rewrite media evidence or durable user data. Query-plan and multi-page contract tests
must prove that the index is usable and that known and unknown timestamps cross page boundaries
without duplicates or gaps.

Flutter consumes the server order. It does not sort a partial page locally. Contiguous known dates
form localized date headings; all missing capture values form an explicit unknown-capture-time
section. Automatic near-bottom loading remains a continuous-scroll behavior. A control is visible
only for retry after a page error, not as ordinary pagination.

This decision defines the default unfiltered order. Folder scope, search, alternate sorting, date
distribution, and arbitrary time jumps must compose through later Ame-owned query contracts rather
than bypassing this keyset.

## Consequences and risks

- Local wall-clock order is honest about missing offsets but is not a globally normalized travel
  chronology.
- Modified time changes the order inside the unknown section without changing its semantic label.
- Cursor bridge types change and must be regenerated as one compatibility boundary.
- A future alternate sort requires a sort identity in the query and cursor; it must not reinterpret
  this cursor silently.
- The first R2 slice can still retain more loaded records than the final viewport-window design.
  Date distribution and arbitrary jumps must introduce bounded window replacement before R2 closes.

## Validation evidence

- Schema v10 migrates to v11 without rewriting a stored capture value, and a query-plan test uses
  `asset_locations_gallery_time` for the ordering expressions.
- Cross-root fixtures prove descending known capture time, modified-time tie breaking, explicit
  unknown ordering, 1,025-location keyset traversal, and stale-revision rejection.
- The generated Rust/Dart bridge carries every new cursor key. Flutter analysis and tests verify
  localized date grouping, the unknown heading, stable tile identity, automatic continuation, and
  retry-only page controls.
- All 52 non-ignored Rust tests and 18 Flutter tests pass with formatting, Clippy, analysis, bridge
  hash, and diff checks.
- The retained real catalog migrated to v11 and loaded 79,013 locations through 155 bounded windows
  at revision 2. Every key was globally monotonic with no duplicate or gap. It contains 7,715 known
  capture times from `2026-08-01T16:30:17` through `2006-11-29T17:15:41` and 71,298 explicitly
  unknown capture times.

The real distribution confirms that the unknown section is a material product constraint rather
than an edge case. Later date distribution and jump behavior must represent that population without
silently relabeling modification time as capture time.

## Replacement and rollback strategy

The index can be dropped without discarding catalog rows. Replacing the ordering contract requires a
new cursor shape or explicit sort identity, focused migration and query tests, regenerated bridge
types, and an atomic Flutter adapter update. Old cursor values are ephemeral and may be rejected;
catalog and user data remain preserved.
