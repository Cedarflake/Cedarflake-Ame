# ADR 0008: Capture-time keyset for the unified gallery

- Status: Accepted
- Date: 2026-08-08

## Context

R1 loads active catalog locations in root-creation and relative-path order. That order is stable for
catalog validation but cannot support the R2 continuous timeline: adding another root creates a
separate block, date headings are not contiguous across roots, and a time rail cannot address the
complete result set.

Capture metadata is intentionally evidence-preserving. Many images have a normalized local capture
time but no trustworthy timezone offset. File creation and modification timestamps are filesystem
evidence, not capture-time evidence, but they can provide a deterministic display date without
overwriting or relabeling the original metadata.

The real two-root acceptance catalog contains 79,013 active locations. A gallery query must remain
bounded, deterministic across equal timestamps, revision-safe during publication, and independent
of root import order.

## Considered options

### Keep root and relative-path ordering

Rejected. It cannot produce one cross-root timeline or a meaningful date distribution.

### Convert every capture value to a UTC instant

Rejected. Offsets are absent for a substantial class of personal images. Inventing an offset would
turn uncertain metadata into false chronology and make date headings surprising.

### Overwrite missing capture evidence with a filesystem timestamp

Rejected. Filesystem timestamps must not be stored or exposed as EXIF capture evidence.

### Derive a separate gallery date with deterministic fallback

Accepted. A personal gallery primarily presents the local date recorded by the camera or source.
When that value is absent or malformed, the gallery uses file creation time, then file modification
time. The derived value controls ordering, headings, timeline buckets, and seeking while the stored
capture evidence remains unchanged.

## Decision

The effective gallery date is selected in this order:

1. normalized local capture time;
2. file creation time converted to a local display key;
3. file modification time converted to a local display key.

The default unified-gallery key is the effective gallery date, file modification time as a stable
tie-breaker, root identity, then location identity. Creation-time sorting uses creation time with
modification time as its fallback. Modification-time sorting continues to use modification time
directly. An unknown date remains possible only when no timestamp can be represented.

The final two keys provide deterministic ordering when timestamps are equal. The cursor carries the
catalog revision and every ordering key. A publication invalidates an older cursor before any page
is returned.

SQLite schema v13 adds a rebuildable `file_local_time` key derived from creation time or modification
time. Capture sorting uses `COALESCE(capture_local_time, file_local_time)`. The migration is additive:
it does not rewrite media evidence or durable user data. Query-plan and multi-page contract tests
must prove that the indexed effective key remains bounded and crosses page boundaries without
duplicates or gaps.

Flutter consumes the server order. It does not sort a partial page locally. It derives the same
capture, creation, then modification date for localized day headings. Automatic near-bottom loading
remains a continuous-scroll behavior. A control is visible only for retry after a page error, not as
ordinary pagination.

This decision defines the default unfiltered order. Folder scope, search, alternate sorting, date
distribution, and arbitrary time jumps must compose through later Ame-owned query contracts rather
than bypassing this keyset.

## Consequences and risks

- Local wall-clock order is honest about missing offsets but is not a globally normalized travel
  chronology.
- Filesystem fallback changes gallery placement without changing capture-time provenance.
- The derived local file key is rebuildable catalog data and may be regenerated if timezone policy
  changes later.
- The cursor bridge shape remains stable; cursor key values are read from the same SQLite
  expressions that own ordering, avoiding a second timestamp formatter in Rust.
- A future alternate sort requires a sort identity in the query and cursor; it must not reinterpret
  this cursor silently.
- The first R2 slice can still retain more loaded records than the final viewport-window design.
  Date distribution and arbitrary jumps must introduce bounded window replacement before R2 closes.

## Validation evidence

- Schema v12 migrates to v13 without rewriting a stored capture value, and a query-plan test uses
  `asset_locations_gallery_time` for the effective-date ordering expression.
- Cross-root fixtures prove capture, creation, then modification fallback, indexed timeline
  distribution, bidirectional month seeking, and gap-free keyset traversal.
- Flutter tests verify localized capture, creation, and modification headings while preserving
  stable tile identity, automatic continuation, and retry-only page controls.
- Before this decision, the retained real catalog loaded 79,013 locations through 155 bounded
  windows at revision 2. It contained 7,715 known capture times and 71,298 missing capture values.

The historical distribution confirms why a fallback is required: most files lack capture metadata.
Schema v13 gives those items a useful gallery date while preserving their capture field as unknown.

## Replacement and rollback strategy

The derived column and indexes can be rebuilt without discarding catalog rows. Replacing the
ordering contract requires focused migration and query tests plus an atomic Flutter update. Old
cursor values are ephemeral and may be rejected; catalog and user data remain preserved.
