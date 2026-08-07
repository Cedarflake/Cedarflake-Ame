# ADR 0010: Dual-level gallery time navigation

- Status: Superseded
- Date: 2026-08-07
- Superseded by: rollback to ADR 0009, Gallery and time navigation

## Context

The single annotated time rail in ADR 0009 had to represent both the complete filtered library and
fine movement inside one month. Trying to stretch it into a fine-grained control made a large month
dominate the rail, while replacing its accepted nonuniform annotations with uniform month positions
lost the previously approved time geometry. Removing the ordinary scrollbar also left no precise
local control. This was especially visible with tens of thousands of images in one month.

The user initially accepted a two-level navigation experiment. Runtime and visual validation then
showed that it did not match the earlier approved single-rail design, and the synthetic local
scroll surface caused a Windows Release crash while the gallery was scrolling. The user explicitly
requested a rollback to the previously approved screenshot state.

## Decision

Restore ADR 0009 without amendment:

- the right annotated Material time rail is the only visible scroll-position control;
- its year and month annotations retain the approved nonuniform bucket placement;
- normal gallery scrolling selects the node for the currently visible month instead of inventing
  intermediate density markers;
- activating a timeline annotation uses the existing bounded catalog-window query;
- no adjacent native scrollbar, current-month scrollbar, hidden `Scrollable`, percentage-range
  adapter, or month-transition label is rendered.

## Consequences

- The dual-level experiment is not part of the current UI contract.
- Exact full-month fine seeking remains unresolved and must not be represented by a fake scroll
  range.
- ADR 0009 remains the binding presentation decision.

## Validation evidence

- Pure tests verify the accepted nonuniform bucket-node placement.
- Widget tests verify that no second scrollbar exists and ordinary gallery movement advances the
  single global time rail to the visible month's node without issuing a catalog jump.
- Flutter analysis, the complete Flutter test suite, and a Windows Release scrolling stress test
  are required before delivery.

## Replacement strategy

Any future second navigation control requires a new user-approved decision and Release-mode
interaction evidence. It must not amend ADR 0009 implicitly.
