# ADR 0029: Separate workspaces from gallery scope

Status: Accepted.

Date: 2026-10-16.

Supersedes ADR 0009 only for shell navigation, duplicate entry and duplicate review placement.
Its gallery, viewer, source safety and Material interaction contracts remain in force.

## Context and decision

Ame needs stable destinations for browsing and duplicate analysis as its organizing capabilities
grow. A permanent narrow function rail selects Library or Duplicates; Settings is pinned below.
The Library workspace contains a collapsible source/folder sidebar and the existing photo wall.
The other workspaces own their contextual controls without duplicating the catalog or viewer.
The accepted interaction blueprint is in [the gallery contract](../product/gallery-ui.md#workbench-blueprint).

Use the existing Flutter Material 3 design system. The installed Flutter 3.44.9 SDK provides
`NavigationRail` with destination selection, labels, focus, semantics and a bottom trailing action;
`Visibility.maintainState`, `TickerMode` and `ExcludeFocus` retain inactive workspace state while
retiring input and animation. Existing source navigation and resize controls remain reusable.
The product-specific responsibility is destination lifetime, folder-panel preference and composing
the existing gallery with a dedicated duplicate workspace. No new UI package is needed.

The secondary-sidebar toggle lives before the gallery title, outside the collapsible sidebar.
Store its visibility with existing presentation preferences. Preserve source scope, expanded
folders, scroll anchor and selection on workspace switches. Narrow windows may initially hide the
secondary sidebar; explicitly opening it restores the same panel and its retained state.

The gallery filter owns All images and Merge duplicates. Duplicate-only browsing, analysis control,
group inspection and durable decisions belong to the Duplicates workspace. Both consume one typed
analysis service. Exact evidence and later perceptual candidates remain distinct.

The gallery uses the existing `AmePopupMenuButton` around Flutter's `showMenu` and
`CheckedPopupMenuItem` for its two display choices. The duplicate badge opens a centered `Dialog`
through `showDialog`, with a lazy `ListView` and bounded paging. Long filenames and folder paths
wrap completely within the available width. The previous anchored menu constrained this richer
content and clipped paths and controls; reading multiple locations needs a stable dialog surface.
The pinned Flutter 3.44.9 SDK provides dialog constraints, safe-area/inset handling, modal dismissal
and closed-loop focus traversal. Use a maximum width of 560 logical pixels and fit the height to
the window; keep the title, close button and paging controls outside the scrolling list. Restore
focus to the badge when it remains mounted in the visible workspace. `AlertDialog` was considered;
`Dialog` permits the existing lazy list and paging footer without intrinsic list sizing.
Material's
[menu](https://m3.material.io/components/menus/guidelines) and
[dialog](https://m3.material.io/components/dialogs/guidelines) contracts supply focus, dismissal and
choice behavior. Ame supplies query scope, persisted display preference, duplicate badges,
filename/folder presentation, current-location marking and 64-location paging. The dialog retains
the gallery's browsing position; no second positioning, focus or scrolling implementation is added.

## Alternatives and consequences

Keeping all analysis inside the filter makes a growing workflow difficult to discover. A separate
bottom sheet also competes with the photo wall and temporary task feedback. A function rail provides
a stable destination while local badges retain fast access to individual duplicate locations.

Workspace visibility must retire background preview demand, focus and hidden animations without
recreating the photo wall. Background analysis remains application-owned and resource bounded;
changing destinations cannot restart hashing or dispose a running analysis.

## Validation and rollback

Verify destination and folder-panel changes preserve browsing state, keyboard focus is restricted
to the visible workspace, the toggle remains reachable, and location activation reveals the exact
file through the existing platform adapter. Exercise narrow and wide windows in the client.
Automated boundary checks run in Actions. Rollback can restore the prior shell without discarding
versioned fingerprints, source identities, catalogs or user data.

References: [Material components](https://m3.material.io/components),
[Flutter NavigationRail](https://api.flutter.dev/flutter/material/NavigationRail-class.html),
[Windows navigation](https://learn.microsoft.com/en-us/windows/apps/design/basics/navigation-basics).
