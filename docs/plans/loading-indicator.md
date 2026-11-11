# Loading indicator adoption

The [roadmap](../roadmap.md#loading-feedback) owns delivery status. This plan defines the
loading-feedback changes and deletion-latency follow-up on `codex/loading-indicator`.

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
The thumbnail strip opens and closes with shared spatial motion as an overlay above the viewer
toolbar; it preserves the main image's viewport, position and zoom.
The dialog review also aligns failure content, spacing and error colors across dialogs and trays,
preserves the weekday in compact date input, and matches sidebar drag surfaces to their rows.
The deletion follow-up bounds explicit selection and duplicate evidence reads to the chosen files,
reuses the ordered snapshot when preserving the viewport, and coalesces rapid operation publications.
Confirmation, native completion and gallery refresh form one operation with one final result.
Explicit file selection retains its mode during same-scope updates and resolves its current
state before confirmation under [ADR 0042](../architecture/0042-windows-file-operations.md), with
focused native selection verification in addition to the presentation checks below.

The response-time follow-up reduces smart-album navigation/category reads and duplicate-review
switching cost. Foreground navigation must not wait for background classification. Preserve automatic
curated-category expansion, current source validation and cached image features while reducing
redundant update work. Measure category-list, collection-switch and review reads on the established
library, including active classification, rather than using loading feedback as completion evidence.

The selection follow-up narrows album/Favorite admission to selected membership, source state and
the affected decision field, preserving the initial all-matching guard and frozen file-operation
authority. Background publications cannot cancel an unchanged explicit selection. Reuse bottom
task/notification feedback instead of putting status text or progress under toolbar actions.
Organization-mode entry and continuation reads must publish usable results despite repeated
background updates, without holding the mode transition open for continuation metadata.
The same follow-up retains state-based Favorite actions and a menu only for mixed selections,
keeps exit available during mode reads, retains known single-image actions during refresh, and lets
completed file feedback yield to new organization notices. Selection changes clear their old failure;
mode failures name the operation that actually failed.

Use existing progress, gallery and dialog regressions, adding only missing lifecycle/coordination
coverage. The deletion follow-up also needs bounded native query and anchor checks, with read-only
timings from the established library. Verify the affected Windows surfaces and inspect normal branch
CI without adding a full release workload. Evidence belongs in the
[motion acceptance record](../acceptance/material-motion.md).

Keep the installed SDK and package pins. When an official stable `LoadingIndicator` is available,
replace the shared component's internal renderer and remove the temporary port and its notice.
Feature loading state and the visibility contract stay with their current owners. Rollback restores
these presentation changes together without changing catalogs or source files.
