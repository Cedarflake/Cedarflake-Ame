# ADR 0011: Bidirectional gallery keyset around time anchors

- Status: Accepted
- Date: 2026-08-08

## Context

ADR 0008 introduced deterministic forward keyset pagination and later time-anchor queries replaced
the loaded Flutter asset set with a bounded page beginning at the selected month. The presentation
then moved its local `CustomScrollView` to offset zero. Although the complete-result timeline still
contained newer items, Flutter correctly treated that anchored page as the local top because the
catalog contract exposed only a next cursor.

This made upward mouse-wheel navigation stop after any noninitial timeline jump. Forwarding wheel
events or drawing another scrollbar would not restore the missing records. Loading from the first
catalog row through the selected month would make arbitrary jumps proportional to library size and
would violate the large-library query boundary.

## Considered options

### Forward wheel events to the time rail

Rejected. The event already reaches the gallery when the pointer is over an image. The unavailable
records, not pointer routing, are the failure.

### Reload every row preceding the selected month

Rejected. It turns a bounded time jump into an unbounded walk and retains unnecessary assets and
previews.

### Use offset pagination before the anchor

Rejected. Deep offsets do not preserve ADR 0008's indexed, revision-safe keyset behavior.

### Add a reverse keyset boundary

Accepted. The same Ame-owned ordering keys can query the records immediately preceding a page
boundary by reversing every ordering term for the database read and restoring canonical order
before publishing the page.

## Decision

- `CatalogSnapshot` carries independent previous and next cursors.
- A catalog request accepts at most one of an after cursor, a before cursor, or a time anchor.
- Before-cursor queries invert the missing-rank, primary, root, and location ordering terms, fetch
  one bounded page plus lookahead, then reverse the returned records into canonical gallery order.
- Flutter keeps previous-page loading separate from next-page loading and prepends results without
  duplicating stable location identities.
- A mouse-wheel signal directed upward while the anchored gallery is at its local top starts the
  previous query. Ordinary Material and `Scrollable` pointer behavior remains framework-owned.
- The justified gallery's already-computed row extents determine the exact prepend displacement.
  After a page is inserted, the scroll position advances by that displacement so the asset the user
  was viewing stays at the same screen position.
- Query revision and query identity checks apply in both directions. A stale cursor reloads a
  trustworthy first window rather than merging incompatible results.

## Consequences and risks

- The Rust/Dart bridge changes atomically to carry the before cursor and previous page boundary.
- A time-anchor page may perform one empty reverse query when the selected anchor is already the
  absolute first result; that response clears the speculative previous boundary.
- Individual database reads remain bounded. The current presentation still retains pages visited
  during one continuous browsing session; later retention eviction must preserve both boundary
  cursors and the visible asset before it can replace that behavior safely.
- Source media remains read-only. The feature reads only catalog rows and rebuildable previews.

## Validation evidence

- SQLite fixtures walk backward across a capture-time anchor without gaps or reversed display
  order.
- Flutter controller tests verify prepend order, cursor preservation, and stable query identity.
- A desktop widget test sends an upward `PointerScrollEvent` at the local top, verifies that the
  previous page is requested, and confirms the original anchor remains at the same screen position.
- The complete Rust suite passes 60 tests with 3 explicitly gated acceptance tests ignored, and
  Clippy passes with warnings denied.
- The complete Flutter suite passes 56 tests, including the desktop pointer regression and the
  packaged Release bridge smoke test.
- The Windows Release runner loads the Rust DLL beside its executable even when launched from the
  repository working directory, then opens the configured gallery without a bridge mismatch.

## Replacement strategy

The before-cursor API is additive to the canonical ordering contract. Replacing it requires a new
cursor version or query identity, bridge regeneration, forward-and-reverse gap tests, and a visual
scroll-anchor regression. Catalog rows and user data do not require migration.
