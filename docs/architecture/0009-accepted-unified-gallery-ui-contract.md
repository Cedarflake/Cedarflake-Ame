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

- The global bar contains application identity and centered gallery search. App-drawn window controls
  may share this surface as defined by ADR 0012, but library import and settings do not.
- The global bar and sidebar share the Material `surfaceContainerLow` application backdrop. The
  gallery or settings canvas is one `surfaceContainerLowest` Material pane with a rounded leading
  top corner. Tonal surface hierarchy and spacing separate these regions; full-window header and
  navigation dividers do not define the shell.
- The sidebar contains Library, Favorites when functional, one aligned folder-source list, and
  albums when functional. The Library row owns Add folder, while Settings remains pinned to the
  sidebar bottom rather than scrolling with folder sources.
- On desktop, the expanded sidebar is user-resizable from 220 to 420 logical pixels by dragging its
  trailing resize boundary. Double activation restores the 260-pixel default, the chosen width
  persists as a presentation preference, and constrained windows still collapse to the fixed icon
  rail. The resize target remains visually transparent before, during, and after dragging; surface
  contrast, the resize cursor, keyboard focus, and semantics communicate the boundary without a
  persistent blue rule.
- Local, cloud-backed, unavailable, and removable sources remain folders in one list. Availability
  is row status, not a separate provider hierarchy.
- Truncated source and folder labels expose their complete user-readable path through a tooltip.
  Windows device prefixes such as `\\?\` remain an adapter detail and are never shown as part of a
  path presented to the user.
- Timeline, classification, search, sorting, task activity, and duplicate review are not sidebar
  destinations.

### Contextual gallery header

- The title and result count remain left aligned.
- Browsing actions are right aligned in this order: Select, Sort, Filter, Layout, More.
- Selection replaces the browsing action set with Cancel and selection-specific actions. It does
  not create or nest another page. Deselecting the final selected asset exits selection mode and
  restores the browsing action set automatically.
- When favorites become functional, Favorite or Unfavorite is a selection-specific command in this
  same upper-right contextual area. It changes the favorite state of the selected assets. The
  Favorites row in the sidebar has a separate navigation responsibility: it scopes the unified
  gallery to assets already marked as favorites and does not modify favorite state. Neither control
  is shown as an inert placeholder before its owning use case exists.
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
- Open in File Explorer reveals the physical file or configured folder selected in its parent
  directory. Windows device-path translation stays inside the platform adapter and never becomes a
  presentation concern.
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

### Image viewer

- The viewer remains a state of the unified gallery rather than a separate library destination.
  While active it replaces browsing chrome with one image-focused toolbar. That toolbar also remains
  the visible application window chrome: its path region owns native window dragging and its trailing
  edge reuses the caption controls accepted by ADR 0012. Returning to the gallery preserves the
  existing gallery widget, query, selection, and scroll position.
- Flutter `InteractiveViewer` and `TransformationController` own image panning and zoom transforms.
  Material `Slider`, `IconButton`, `MenuAnchor`, and modal bottom-sheet primitives own zoom input,
  navigation, read-only actions, and information presentation. The product-specific layer only
  calculates fit-to-window versus actual-pixel scale and connects gallery navigation.
- The source image is loaded for viewing. A derived preview may be shown while it loads or as an
  explicitly labelled fallback when the source is unavailable; a preview is never presented as the
  original without that state being visible.
- Thumbnail and viewer loading indicators remain square and shrink against the shortest available
  edge instead of accepting non-square constraints. Viewer zoom actions remain first-party Material
  Slider, IconButton, TextButton, and Divider components, with explicit padding between the zoom and
  display-mode groups.
- The viewer toolbar identifies the current physical file by filename. The filename remains
  single-line and uses trailing ellipsis when space is constrained; hovering it always exposes the
  user-readable absolute path in a tooltip. Windows device prefixes remain hidden.
- Fit-to-window and actual size are distinct commands. The displayed percentage represents image
  pixels relative to their actual size, not merely the `InteractiveViewer` transform. Pointer,
  trackpad, Material controls, double activation, and keyboard shortcuts share one transformation
  state.
- Programmatic zoom, fit, actual-size commands, and discrete mouse-wheel zoom animate one uniform
  scale-and-translation state through Flutter `AnimationController`. Direct slider manipulation,
  image dragging, and touch or trackpad gestures remain immediate and cancel an active command
  animation. Mouse-wheel zoom retains the focal-point transform calculated by `InteractiveViewer`
  and eases from the pre-event transform to that target. Flutter 3.44.9's `Matrix4Tween` supports
  translation only, so Ame interpolates the existing controller's uniform scale and translation
  without replacing `InteractiveViewer` gesture ownership.
- Viewer controls occupy a reserved full-width bottom command surface rather than floating over the
  image. Fit and actual-size actions align to the leading edge, zoom actions align to the trailing
  edge, and the center remains clear at supported window widths.
- Opening the viewer explicitly moves focus into its shortcut scope even when the previously
  focused gallery tile remains mounted offstage. `Esc`, Backspace, and the browser Back key return
  to the gallery; Left and Right navigate; Plus and Minus zoom; `0` fits the image; and `1` shows
  actual pixels. The Ctrl-modified zoom, fit, and actual-size forms remain available. Gallery-only
  selection shortcuts are inactive while the viewer is open.
- Previous and next navigation follow the active gallery query and may extend the bounded loaded
  window through the existing catalog paging contract. Viewer actions remain read-only: view
  information, copy path, and open in File Explorer.

### Gallery and time navigation

- The default `等高` layout uses one density-selected height for every photo row. Complete rows
  distribute their aspect-weighted cell widths across the available width, while the final sparse
  row keeps natural widths and remains left aligned. Existing `BoxFit.cover` rendering handles any
  necessary crop inside a cell.
- Equal-height tiles retain the minimum width required by their Material selection affordance.
  Extremely narrow source images may be center-cropped in the derived thumbnail so selection,
  focus, and pointer targets remain fully visible; source media is never changed.
- The `方形` layout is a uniform square grid. Small, medium, and large density choices remain
  independent from shape.
- Date headings use capture time, then file creation time, then file modification time as defined by
  ADR 0008. An unrepresentable date remains an explicit unknown section in the same continuous
  gallery.
- The right-side annotated time rail is the only visible scroll-position control. Its controlled
  Material Slider derives its stable full range from the complete catalog timeline and its current
  value from the gallery's sole `ScrollController` plus the materialized window's global start
  offset; it never owns an independent committed month or timeline position. Loading another page
  must not extend the rail. Exact day offsets are used where rows are materialized, while bounded
  off-window seeking uses the catalog time-anchor contract.
- Flutter's Material Slider owns pointer, keyboard, focus, and semantics for the rail through a thin
  rotation adapter. Its visual track and handle are hidden. Ame draws a passive current-position
  line, a hover preview line, exact-offset annotations, and the background needed to keep endpoint
  nodes visually enclosed.
- Colliding annotations are hidden deterministically without being moved or merged. The timeline
  does not create clusters, badges, hover menus, or a second selection surface.
- Timeline arrows use first-party Material IconButton and Icon components. Arrows, Slider axis,
  handle, and month nodes share one geometric axis.

### Feedback and settings

- Import and update work uses temporary action-specific bottom progress with cancellation; there is
  no permanent task destination.
- Settings is a sidebar-selected destination rendered in the existing main canvas. It keeps the
  global bar and source sidebar visible and never opens an application-settings dialog.
- The settings canvas uses shallow, plain-language Material rows grouped as Personalization,
  Browsing, Storage, and About. Each row has an icon, a user-facing name, one short explanation,
  and a right-side Material control where the setting is actionable.
- Flutter `Card` and `ListTile` own settings grouping and row semantics. `DropdownMenu`,
  `OutlinedButton`, and progress indicators own choices, actions, and storage feedback; custom code
  only supplies the responsive page composition and Ame-owned state connections.
- Internal database versions, task queues, raw worker counts, hash engines, and analysis parameters
  are not user settings. The preview loading preference is the narrow exception: it exposes only
  `small`, `medium`, and `large` resource policies while ADR 0005 retains ownership of the internal
  concurrency limits. Account, cloud-service, and media-editing controls remain absent until their
  workflows exist end to end.
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

Flutter 3.44.9 was verified to provide Material shape clipping, `ClipRRect`, and the
`surfaceContainerLow` and `surfaceContainerLowest` color roles. The shell therefore composes
first-party `Material`, `Row`, and `Column` primitives; the only product-specific gap is the
resizable sidebar hit target, whose interaction remains built from Flutter focus, pointer,
gesture, cursor, and semantics primitives.

The official Material 3 Slider and Icon button components remain the basis of viewer zoom input.
Flutter 3.44.9 was verified to provide `Slider`, `IconButton`, `TextButton`, `VerticalDivider`, and
`CircularProgressIndicator`; Ame adds only responsive square constraints and group spacing around
those primitives.

The official Material 3 menu catalog remains the basis of settings choices. Flutter 3.44.9 was
verified to provide controlled `DropdownMenu` selection, selection callbacks, disabled search, and
select-only behavior. Preview loading speed therefore reuses the repository-owned `SettingsChoice`
composition and adds no custom pointer, focus, keyboard, or semantics layer.

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
  exit, main-canvas settings presentation, and single gallery canvas.
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
