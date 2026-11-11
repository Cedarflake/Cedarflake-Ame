# Loading indicator adoption

The [roadmap](../roadmap.md#loading-feedback) owns delivery status. This plan defines one
presentation change on `codex/loading-indicator`.

## Outcome

Use a shared Material Expressive loading indicator for content waiting and short dialog reads.
Preserve useful content, scroll geometry, operation admission and existing progress semantics.
The existing delay suppresses brief loading feedback; completed work never waits for animation.

## Implementation sequence

| Slice | Scope | Verification |
| --- | --- | --- |
| L1 | Adapt the indeterminate animation from `flutter/packages` PR 13429 at `9eaa17df4e0ba4b067a7203e857953e3d4bc04bd`, using installed Material shapes; retain attribution and notices behind an Ame component | Fixed geometry, reduced motion, hidden/disposed ticker retirement and the existing quiet interval |
| L2 | Replace preparation bars in file-action and cleanup confirmations with a centered content indicator; replace album, location and history tray read bars with content/header feedback | Delayed reads, completion, failure/retry and cancellation retain their actions and layout |
| L3 | Coordinate gallery/review loading, remove the review's empty-card wall and remove the timeline divider | First load/scope change, retained content, continuation demand and mutual exclusion of content/timeline indicators |

## Surface decisions

- Confirmation dialogs center the loading icon in the content area, replacing the pending content.
  Existing actions stay below; selection preparation still gates confirmation. Preparation failures
  replace the pending form with a readable explanation and a return action.
- Loading indicators have no visible loading captions. Keep accessible status labels and existing
  surface titles, consequences and recovery text.
- List trays show the indicator in empty content while reading, and beside the title when retaining
  an existing list during refresh (or beside the result-history paging actions). Result and retry
  controls remain interactive.
- An empty gallery or review page shows a centered indicator together with the top loading line.
  One visibility owner admits and dismisses both. Already displayed content remains during paging
  and background updates; those reads retain the thin top line.
- Review lists no longer paint blank cards for unknown entries. Virtual extent and visible-range
  requests remain functional, including continuation into unloaded rows. Gallery timeline layout
  placeholders retain their final geometry under ADR 0014.
- The timeline keeps its compact circular indicator. Content loading takes precedence before the
  quiet interval starts; the timeline indicator appears only when content is available and the rail
  alone is waiting. Removing its left divider does not change width or hit targets.
- Compact controls, filmstrip loading cards, task progress and measured storage gauges keep their
  current components. This is not a global replacement of every circular/linear indicator.

## Delivery and replacement

The follow-up scope also covers viewer quality/corner continuity, partial-card return clipping,
the filter-menu affordance, compact confirmation states and short-gallery rail endpoints.
The dialog review also aligns failure content, spacing and error colors across dialogs and trays,
preserves the weekday in compact date input, and matches sidebar drag surfaces to their rows.
Explicit file selection retains its mode during same-scope updates and resolves its current
state before confirmation under [ADR 0042](../architecture/0042-windows-file-operations.md), with
focused native selection verification in addition to the presentation checks below.

Use existing progress, gallery and dialog regressions, adding only missing lifecycle/coordination
coverage. Verify the affected Windows surfaces and inspect normal branch CI; do not run native
performance or full release workloads for this presentation change. Evidence belongs in the
[motion acceptance record](../acceptance/material-motion.md).

Keep the installed SDK and package pins. When an official stable `LoadingIndicator` is available,
replace the shared component's internal renderer and remove the temporary port and its notice.
Feature loading state and the visibility contract stay with their current owners. Rollback restores
these presentation changes together without changing catalogs or source files.
