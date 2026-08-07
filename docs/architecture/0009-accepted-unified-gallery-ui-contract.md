# ADR 0009: Accepted unified gallery UI contract

- Status: Accepted
- Date: 2026-08-07
- Supersedes: ADR 0003

## Context

The earlier production shell and intermediate prototypes drifted between separate gallery pages,
engineering surfaces, a duplicate-review destination, provider-grouped sources, and several custom
timeline attempts. The user reviewed an isolated interactive prototype against Microsoft Photos and
accepted a corrected unified-gallery structure after iterative Windows visual inspection.

This record freezes the accepted presentation and interaction boundaries before production catalog
behavior is connected. It does not declare fixture-backed behavior production-complete.

## Decision drivers

- one coherent image-library workflow rather than peer feature pages;
- familiar Windows photo-library information architecture;
- first-party Flutter Material 3 interaction, focus, semantics, and state layers;
- bounded rendering suitable for large local libraries;
- exact-duplicate behavior that remains a gallery view state;
- a stable presentation contract independent of Rust and engine DTOs;
- Simplified Chinese copy that does not expose engineering vocabulary.

## Decision

Use one unified gallery with these presentation rules.

### Shell and navigation

- The global bar contains application identity, centered gallery search, Import, and Settings.
- The sidebar contains Library, Favorites when functional, one aligned folder-source list, and
  albums when functional.
- Local, cloud-backed, unavailable, and removable sources remain folders in one list. Availability
  is row status, not a separate provider hierarchy.
- Timeline, classification, search, sorting, settings, task activity, and duplicate review are not
  sidebar destinations.

### Contextual gallery header

- The title and result count remain left aligned.
- Browsing actions are right aligned in this order: Select, Sort, Filter, Layout, More.
- Selection replaces the browsing action set with Cancel and selection-specific actions. It does
  not create or nest another page.
- A tile reveals its upper-right checkbox on pointer hover or keyboard focus. Selection mode shows
  checkboxes on all visible tiles; selected tiles keep their check mark and primary outline. Tile
  activation opens the viewer while checkbox activation only changes selection. Touch and assistive
  technology expose selection without depending on hover.
- Exact duplicate display modes and review belong to the grouped Filter menu. There is no separate
  duplicate action.
- Unimplemented destructive operations are absent rather than disabled placeholders.

### Context menus

- A gallery item opens a Material context menu through secondary click or the platform keyboard
  context-menu gesture. The menu targets the invoked item and does not silently discard an existing
  multi-selection.
- R2b exposes only connected read-only actions: Open, View information, Copy path, and Open in File
  Explorer. Duplicate-location and favorite actions appear only when their owning stages exist.
- A source row's overflow action, secondary click, and keyboard gesture open one shared menu for
  Rescan, Open in File Explorer, and Remove from Ame. Removing a root unregisters catalog state and
  never deletes its directory or files.
- Menu placement, focus, dismissal, keyboard navigation, and semantics use Flutter Material menu
  primitives. Source-file edit, print, share, move, copy, rename, and delete actions remain absent
  until separately accepted workflows own them.
- The browsing toolbar's More menu contains Select all and Deselect all with `Ctrl+A` and
  `Esc`/`Ctrl+D` shortcuts. Select all means the complete current query, not only loaded widgets.
- Complete-query selection is represented by query identity plus explicit exclusions so it remains
  bounded. A query change clears selection rather than silently changing what the selection means.

### Gallery and time navigation

- The default `等高` layout is an aspect-preserving justified photo wall. Every complete row has
  one height and fills the available width; sparse rows have a bounded enlargement policy.
- The `方形` layout is a uniform square grid. Small, medium, and large density choices remain
  independent from shape.
- Date headings and explicit unknown capture time are part of one continuous gallery.
- The right-side annotated time rail is the only visible scroll-position control. It represents the
  complete filtered result rather than only loaded widgets.
- Flutter's Material Slider owns pointer, keyboard, focus, hover, track, handle, and semantics. A
  thin rotation adapter provides vertical orientation. Ame adds only nonuniform timeline annotations
  and a background extension needed to keep endpoint nodes visually enclosed.
- Timeline arrows use first-party Material IconButton and Icon components. Arrows, Slider axis,
  handle, and month nodes share one geometric axis.

### Feedback and settings

- Import and update work uses temporary action-specific bottom progress with cancellation; there is
  no permanent task destination.
- Settings is opened from the global gear and uses shallow, plain-language Material rows.
- Simplified Chinese is the initial user-facing language. Paths and source metadata preserve their
  original text.
- Classification, perceptual or semantic similarity, people, editing, and source-file mutation
  controls remain absent until their separately accepted capabilities exist.

## Component policy

Presentation follows this admission order:

1. first-party Flutter Material and framework widgets;
2. repository-owned shared components;
3. mature external packages with recorded admission evidence;
4. the smallest necessary custom layout or annotation layer.

A custom layer must not recreate framework-owned pointer, focus, keyboard, semantics, or platform
behavior. The justified photo-wall calculation is an Ame-owned layout policy because Flutter's
built-in Wrap and regular SliverGrid delegates cannot solve aspect-ratio rows to a shared width.

## Consequences

- Production integration must adapt catalog state into this UI instead of restoring the legacy
  engineering shell.
- All gallery query states share selection, scroll restoration, and one canvas.
- Exact duplicates, later classification, and future search extensions compose as filters rather
  than parallel gallery applications.
- Large-library production rendering still requires bounded slivers or equivalent lazy row windows;
  accepting the fixture prototype does not permit eager construction of a complete catalog.

## Validation evidence

- The user accepted the Windows interactive prototype after reviewing the unified shell, flattened
  source list, right-aligned contextual actions, grouped Sort, Filter, and Layout menus, selection
  exit, settings presentation, and single gallery canvas.
- DPI-aware Windows inspection confirmed that the annotated timeline is the sole visible scrollbar,
  Material arrow controls align with the Slider axis, and endpoint nodes remain enclosed.
- Widget tests cover desktop and constrained widths, selection replacement, source alignment,
  Windows hover behavior, bidirectional gallery and Slider synchronization, nonuniform time marks,
  duplicate filtering, settings, and temporary import progress.
- Pure layout and widget tests confirm balanced justified rows fill one gallery width and sparse rows
  do not exceed their enlargement limit.
- Flutter analysis, the full Flutter test suite, and a Windows Debug build passed for the accepted
  prototype.

## Replacement strategy

A later presentation change must preserve the unified-gallery workflow unless new user evidence
explicitly supersedes it. Framework replacement or a different navigation model requires a new ADR
with measured accessibility, performance, migration, and maintainability consequences.
