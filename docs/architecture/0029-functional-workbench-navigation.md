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
