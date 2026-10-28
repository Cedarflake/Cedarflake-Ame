# Notification template verification

Date: 2026-10-28.

The [gallery contract](../product/gallery-ui.md#feedback-templates) owns stable presentation rules.
The [verification map](quality-gates.md#ui-workflow-verification) owns affected interaction checks.
This maintenance covers bottom feedback and image-preview top notices within the existing R4 branch.

## Scope and ownership

Ordinary results and record recovery already shared a card, while import and multi-root updates
duplicated its shell. Seven feature call sites constructed default SnackBars. They now use the typed
bottom card or the shared transient transport. The viewer's existing top-notice entry remains intact
and shares the surface and action owners. Organization mode no longer adds the obsolete 80-pixel
bottom offset left by its former bottom toolbar.

`ame_feedback_style.dart` owns surface tokens, severity icons and framework text buttons.
`ame_feedback_card.dart` owns title/message/detail, progress and bounded task-list layout.
`ame_transient_feedback.dart` supplies that card through ScaffoldMessenger; it does not publish
notification history. `LibraryViewerNotice` retains compact viewport positioning and one optional
action. Existing source-recovery priority, persistence, cancellation and receipt lifetimes remain
with their original owners. No schema, bridge, media engine or dependency changes are involved.

The pinned Flutter 3.47.8 Material, TextButton and SnackBar implementations supply the visual surface,
button interaction and transient transport. SnackBar owns its queue and route animation; the card
owns the visible shell. The transparent transport does not clip the card's shadow.

## Focused checks

The canonical serial Flutter test wrapper passed 44 checks across the template, bootstrap failure,
notification, task completion, unified task feedback, viewer organization result, archive settings
and cleanup settings suites. These cover both themes, enlarged text, width/insets, wrapping, connected
actions, acknowledgement, progress, task priority, cancellation, retry, stale completion and existing
hover/focus/accessibility retention. The template test also rejects direct SnackBar construction
outside the shared transport. An initial typography assertion compared an unresolved ThemeData style;
it now checks the resolved MaterialApp theme without changing product typography.

The alignment follow-up corrects the retained top-aligned bottom row. Its nine template checks pass,
including explicit icon/text/action center comparisons for single-line results and multi-line text
blocks in both themes. This adds the geometric assertion missing from the initial template checks.

Scoped Dart analysis reports no issues. The final Windows Debug build succeeds. Hosted verification
is owned by the exact delivery commit's Quality / CI run; these local results do not replace it.

## Native interaction

The rebuilt Debug client reuses the established local-primary isolated test profile containing
10,000 files and 72 groups. Gallery Favorite publication displays the shared 680-pixel bottom card,
24 pixels above the window edge, then dismisses. Detailed viewing displays the compact top
unfavorite result without moving the image; its Undo action restores the favorite. A final unfavorite
restores the test group's initial state. The top result has no additional close button.
Organization-mode Favorite feedback uses the same bottom edge as ordinary mode. Its test change is
also reverted, and the client returns to the ordinary gallery.

An explicit metadata update of the same test source displays the multi-root task template, its
current counts, progress and cancellation control, then the completed result for 10,000 pictures.
The source returns to its synchronized state and the result dismisses. Source media and the original
catalog are not modified. This check adds no test corpus or second library copy.

The alignment follow-up rebuild reproduces the single-line Favorite result in the same client:
its icon, title, Undo and acknowledgement now share one vertical center. The test favorite is then
removed through the viewer, restoring the group's previous value.

## Physical ownership

All affected Dart production files have zero inline tests. The shared style/card/transport contain
59/113/36 production lines; the top notice contains 62. The large composition owners remain narrow:
`unified_library_screen.dart` has 1,850 production lines with one obsolete argument removed;
`library_image_viewer.dart` has 425 with two transient-message calls migrated. No new lifecycle is
added to either. Dedicated template and task-surface regressions contain 202 and 364 lines.
