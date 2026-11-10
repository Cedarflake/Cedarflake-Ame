# Material motion adoption

The [roadmap](../roadmap.md#material-motion-adoption) owns delivery status. This plan defines the
ordered implementation and rollback units for `codex/material-motion`, branched from
`codex/r8-file-organization` at `fd57185f60cd47365e94af735b7e44cafabb09c6`.

## Outcome and boundaries

Use the official `material_ui` 1.11.0 spring tokens for deliberate state transitions while
preserving Ame's accepted controls, desktop geometry and immediate input. Keep motion mechanics in
small shared presentation owners; feature widgets supply state and retain their resource lifetimes.
No community motion package or independent physics implementation is required.

The [design principles](../product/gallery-ui.md#design-principles-and-rationale),
[framework decision](../architecture/0001-flutter-rust-desktop-stack.md#standalone-material-library)
and [workbench geometry](../architecture/0029-functional-workbench-navigation.md) remain binding.
R8 file-operation behavior and its merge disposition are unchanged by this branch.

## Implementation sequence

| Slice | Change and owner | Required result |
| --- | --- | --- |
| M1 | Pin `material_ui` 1.11.0 and its lock entry; retain Flutter/Dart pins, renderer, language mode and existing button styles | Installed public spring API and package/license compatibility are verified; relevant settings, fields, menus and sheets retain their behavior |
| M2 | Shared state-transition driving under `lib/app/presentation`; folder/root icons and arrows, `AmeChoice` arrows, `LibraryGalleryHeaderActions` | Icons crossfade, arrows rotate, review controls expand/fade; rapid reversals continue from the displayed state, and reduced motion finishes immediately |
| M3 | `LibrarySidebarTransition`, `LibraryViewerTransition` and `LibraryViewerRoute` | Sidebar/search geometry remains synchronized with anchored right controls; viewer entry/return share effects motion and preserve immediate input/source-read retirement |
| M3a | Root and nested folder children, through a dedicated retained-content presentation owner | Children expand/collapse and fade as one block; closing immediately retires input and focus, and large branches retain their existing loading bounds |
| M3b | Duplicate-display changes through the bounded gallery motion owner | Both All images/Merge duplicates switching and background duplicate publication fade departing cards and reposition retained cards without moving the user's viewport or replaying unrelated query changes |
| M4 | Focused interaction verification, acceptance record, scoped review and branch delivery | Relevant checks and observed desktop behavior support the changed surfaces; commits are pushed and exact-head hosted results are inspected |

Each independently usable slice is committed and pushed before the next one. The dependency update
and its compatibility record are one rollback unit; local state motion and the geometry/viewer
integration remain separately reversible. Do not merge either this branch or its R8 parent as part
of this plan.

## Motion mapping

- Use the official Standard spatial springs for small rotations and bounded panel geometry;
  use effects springs for opacity. Expose only roles needed by the admitted consumers.
- Flutter owns simulation, ticking, focus, input, route dismissal and semantics. Preserve velocity
  on interrupted transitions and distinguish opening/completed from closing/dismissed lifecycle.
- Constrain opacity and layout progress to their valid ranges. Spring settling must not cause
  negative sizes, a second gallery reflow or removal of the closing surface before its exit ends.
- Preserve the sidebar's single shared progress for the search bar, heading and clipped viewport;
  the gallery receives destination geometry, not a new layout for each animation frame.
- Reduced motion and inactive retained surfaces finish or retire through the existing lifecycle
  owner. Starting an operation never waits for decorative animation.
- Folder children appear as one block, without per-item delays. Retain only the closing branch
  until its transition finishes; reopening uses the current progress and current branch data.
- Duplicate transitions operate on visible/mounted cards, not the entire query. Keep a bounded
  visual exit for removed cards, with no input, focus or source-operation authority. Continuous
  background results retarget/coalesce visual work instead of queuing animations or catalog reads.
- Query snapshots, deduplication decisions and scroll anchors keep their existing owners. Unrelated
  scope/filter/sort changes, direct scrolling and reduced motion retire merge animation immediately.

## Exclusions

Keep scrolling, timeline dragging, image zoom/pan, image rotation and asynchronous query navigation
unchanged. Only the admitted duplicate-display changes extend card reflow; unrelated gallery updates
retain their existing behavior. Keep Material-owned menu, dialog, bottom-sheet and notification
transitions. Tooltip and decoded-image fades are lower-priority follow-up candidates, not exit
requirements. Do not activate Expressive button styling or change accepted shapes and spacing.

## Verification and delivery

Use the existing [UI verification map](../acceptance/quality/ui-verification.md). Reuse the menu,
folder navigation, header, sidebar alignment and viewer lifecycle regressions. Add only missing
interruption/reduced-motion coverage at the shared driving boundary; do not assert exact spring
frames or replace behavior assertions with longer arbitrary waits.

In the Windows client, check root/child expansion, choice-menu reversal, organization controls,
sidebar/search alignment, viewer open/return and interrupted transitions. Check root/nested-child
expansion, both duplicate-display directions and continuous background duplicate publication while
scrolling. Recheck settings switches,
filter fields and sheets affected by the package update. Use the existing isolated verification
profile; original media and the normal catalog remain untouched. Record observed results and
remaining gaps in `docs/acceptance`, linked by the roadmap, rather than in this plan.

Formatting and focused diagnosis follow the canonical lock-aware commands. Hosted CI owns complete
automated checks; inspect the actual branch head and fix relevant failures without extra unrelated
test workloads. A package revert restores the pin and lock together; dependent motion changes must
be reverted in the same rollback if the official API is removed.
