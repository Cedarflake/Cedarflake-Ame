# ADR 0003: One unified gallery with contextual actions

- Status: Superseded
- Date: 2026-08-07

## Supersession notice

[ADR 0009](0009-accepted-unified-gallery-ui-contract.md) replaces this record. Later user decisions
retained the unified-gallery foundation but rejected several presentation details in this record.
The following statements are no longer authoritative:

- a permanent task-activity action or surface in the application shell;
- Settings as a sidebar destination;
- a standalone Duplicates action in the gallery toolbar;
- duplicate review outside the grouped gallery filter menu;
- permanent scan-status or engineering surfaces in ordinary navigation.

The accepted contract uses a source-only sidebar, global search and import or settings actions, temporary
action-specific import progress, duplicate modes inside Filter, a contextual selection toolbar, and
a plain-language Simplified Chinese settings page. This notice prevents the obsolete sections below
from being used to restore rejected UI behavior while preserving their historical context.

## Context

Earlier prototypes separated All, Folders, Timeline, Categories, and duplicate behavior into peer
tabs or navigation entries. That structure duplicated the same library concept and conflicted with
the intended Microsoft Photos-like workflow.

The user confirmed that folder, time, category, search, and duplicate behavior operate on one gallery.
The user also confirmed that duplicate review belongs in the upper-right action area, not the sidebar,
and that item selection replaces the browsing toolbar with additional contextual operations.

## Decision

Use one unified library canvas.

### Global shell

- application identity at the upper left;
- centered library search;
- import, task activity, settings, and window controls at the upper right.

### Sidebar

The sidebar owns navigation and source scope only:

- Library;
- Favorites when implemented;
- configured local and OneDrive-backed roots;
- expandable folder trees;
- albums when implemented;
- Settings.

Do not place Timeline, Categories, Search, or Duplicate Review in the sidebar.

### Gallery toolbar

Normal browsing state:

```text
Library · result count            Select | Duplicates | Sort | Filter | Layout | More
```

Selection state:

```text
Selected N items                 Cancel | View | Favorite | Album | Compare | Duplicate info | More
```

Selection replaces the normal action set. It does not create a new page. Actions that have not been
implemented safely are absent rather than displayed as dead placeholders.

### Gallery and time rail

- default presentation is a dense, aspect-preserving justified photo wall;
- date headings are part of the continuous content stream;
- the UI renders a bounded visible region and overscan area;
- the right scrollbar is also a year/month time navigator for the complete filtered result set;
- folder, search, category, duplicate state, sorting, and layout operate on the same gallery query;
- no page numbers or user-visible page transitions are introduced.

### Duplicate action

The upper-right duplicate action controls:

- all physical file instances;
- merged byte-identical groups;
- only exact duplicate groups;
- contextual duplicate-group review in the same main canvas.

A merged representative selects a logical group. Future physical file operations require expansion
and explicit `AssetLocation` selection.

## Consequences

- The UI does not create separate All, Folders, Timeline, Categories, or duplicate gallery stacks.
- New filters compose with the same query, selection, scroll anchor, and time distribution.
- Background scan status appears in a collapsible task surface instead of a permanent status bar.
- Classification and perceptual similarity can be added without changing primary navigation.

## Validation evidence

This information architecture is directly confirmed by the user's review of Microsoft Photos in the
current project conversation.

- The visible shell contains one Library entry and source scopes only; the earlier permanent
  read-only-validation tile is absent.
- Import and Settings remain compact global actions at the upper right, matching the reference
  layout rather than the earlier validation prototype's dominant controls.
- Pause, cancel, resume, progress, and retry are contained in the task-activity surface instead of
  displacing the global or gallery toolbar.
- A normal user import has no validation-only item or entry cap. Explicitly bounded scans remain
  available only through test and acceptance contracts.
- Widget and controller tests prevent the validation tile, exposed scan controls, and default
  500-image or 2,000-entry limits from returning.
- R2 continues to validate full timeline navigation and bounded lazy gallery behavior.
