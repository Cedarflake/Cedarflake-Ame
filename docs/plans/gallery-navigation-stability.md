# Gallery navigation stability

Status: retained repair plan; the [roadmap](../roadmap.md#current-execution-queue) owns disposition

## Outcome

Restore continuous dragging in both gallery layouts while classification publishes new results.
The user's current navigation owns the viewport. Background reads may supply data and compatible
geometry; they cannot restore an obsolete position. Preserve the single Flutter scroll position,
Material Slider interaction and original-media safety required by
[ADR 0014](../architecture/0014-query-wide-gallery-layout-manifest.md).

This work adds no navigation features and does not start R8. Keep the repair on an independent
branch; do not integrate it into `main` before real-library interaction acceptance.

## Implementation boundaries

1. Read the bounded detail window, timeline and compact layout from one SQLite read snapshot.
   Classification may continue writing through WAL while that snapshot completes. Publish these
   values together through the existing query/publication owner; a changed catalog revision must
   not restart every chunk or replace the displayed layout with an empty estimated canvas.
2. Keep geometry independent of detail and preview readiness. Reuse unchanged row geometry across
   metadata-only publications and coalesce background refreshes through the existing query owner.
   Unloaded cards retain the same rectangles as loaded cards.
3. Keep continuous input separate from bounded detail loading. One admitted read may complete while
   the newest pending target replaces earlier pending demand. A later user gesture, query change
   or disposal retires its authority. Preserve the held Material pointer across same-query updates.
4. Revalidate passive demand before execution. A page already supplied by adjacent paging needs no
   replacement read, and passive completion cannot write the viewport. Retain category reset,
   native-wheel tracking, previous-page continuity and drag reversal.

Use existing query, snapshot, layout and navigation owners. New ownership must replace conflicting
responsibilities instead of adding an independent position model or another retry loop. Full media
records and previews remain bounded; the compact layout contains no paths or image bytes.

## Acceptance

Use the existing local-primary gallery and its active background classification, without changing
source files. For both equal-height and square layouts, verify held drag before mouse release,
cold-target placeholders, late completion without snapback, distant drag followed by reversal, and
manual wheel tracking. Check category switching from a non-top position and edge paging.

Focused regressions protect coherent reads, stale publication and gesture lifetime. Their counts
are not an acceptance result. Record real-client observations and relevant resource measurements
in the owning acceptance record; keep this plan free of the debugging transcript.
