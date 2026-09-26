# ADR 0009: Accepted unified gallery UI contract

- Status: Accepted
- Date: 2026-08-07
- Last amended: 2026-09-06
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

- The global bar contains application identity and centered gallery search. One icon-only notification
  history control sits immediately before the app-drawn window controls defined by ADR 0012; library
  import and settings do not appear there. The control uses the ordinary notification icon when the
  current-session history is read and the notification-with-dot icon when unread entries exist. It
  never adds a text label or numeric count to the bar.
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
- A source-row subtitle is reserved for compact actionable state: synchronized, updating, blocked,
  or unavailable. A healthy `LiveOnly` capability explanation is not repeated or truncated in each
  row; it is exposed through the main notification/history surface and accessible compact-row
  description. A blocked explicit recovery remains visible and actionable.
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
- After Remove from Ame is confirmed, the dialog closes and the controller publishes a dedicated
  removing state before native catalog work begins. The temporary task surface names the affected
  source, states that source files are unchanged, and remains visibly indeterminate while the
  catalog transaction and bounded post-removal gallery reload run. The first removing frame is
  rasterized before the bridge call is admitted, duplicate removal and update commands are disabled,
  and a failure retains the exact root identity for retry instead of falling into import behavior.
  Native success and an already-absent result both establish the same idempotent committed boundary.
  At that boundary the controller immediately removes the root and its loaded locations from the
  in-memory roots, assets, query, and selection projections and clears the old timeline, so no ghost
  source or stale navigation data can remain interactive while display reload continues. Completion
  waits only for the bounded first catalog page and root list. The full timeline is loaded separately
  in the background and may publish only for the same request sequence, query, revision, and query
  identity. If the committed bounded reload fails, the task remains a dedicated, non-dismissible
  `已移除，正在刷新显示` recovery state; Retry repeats only that reload and never resubmits
  unregistration. Removal has no misleading mid-transaction cancel action because catalog cleanup
  commits atomically.
- Update Library opens one Material multi-selection dialog over the configured source roots. The
  invoked root is selected initially, and the user may select additional roots before one
  confirmation starts the work. Already active roots remain visible but unavailable for duplicate
  selection; missing, inaccessible, offline, and not-yet-proven roots are also visible but disabled
  because an explicit update cannot make an unavailable source readable. Ame runs at most two root
  updates concurrently and queues any remaining selection;
  the temporary progress surface preserves one status, progress, failure, retry, and cancellation
  control per root. First import remains `添加文件夹`; an update of an already configured root is
  always `更新图库` / `正在更新图库` and never inherits import wording merely because both operations
  use the same native scan adapter. A native scan completion enters `正在刷新显示`; the row reaches
  `更新完成` only after the current catalog view reloads. Reload failure remains visible on that row,
  and Retry repeats only the reload rather than scanning the source again. Terminal actions become
  interactive only after the owning scan stream has closed and left the active-run table, so an
  immediate retry or next update cannot be silently rejected by the previous run.
- A retained first-import task does not hide independent update rows or reserve browsing. Existing
  published roots remain navigable while they update; only conflicting update/removal commands are
  disabled. Continue rejected by another active scan keeps the retained checkpoint and reports the
  conflict on that task. Replacement updates do not expose Pause because they have no durable
  continuation contract. Their stop action is Cancel; the prior published baseline remains usable.
- Query activity is independent of import activity. A failed explicit query retains the previous
  gallery and shows an inline, non-modal Material banner with a retry of that exact query. Its
  failure cannot replace or disappear behind a paused import. A scan that has committed but failed
  to refresh the view also retries only publication of the current view, never source enumeration.
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
  Material `Slider`, `IconButton`, root-navigator popup-menu routes, and modal bottom-sheet
  primitives own zoom input, navigation, read-only actions, and information presentation. The
  product-specific layer only calculates fit-to-window versus actual-pixel scale and connects
  gallery navigation.
- The source image is loaded for viewing. A derived preview may be shown while it loads or as an
  explicitly labelled fallback when the source is unavailable; a preview is never presented as the
  original without that state being visible.
- Original-image access uses a typed application source-reader port, not an unchecked path-based
  `FileImage`. Rust validates the exact active root, scan, location, source generation, revision,
  identity, and metadata before and after acquiring a held native source lease. The Windows adapter
  pins the complete namespace with metadata-only no-follow directory opens and a no-follow/no-recall
  final file open, with source data-write/delete sharing denied. Windows share modes do not prohibit
  attribute-only access and are not proof that every metadata bit is immutable, per
  [CreateFileW sharing semantics](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew).
  The configured-root
  contract remains unchanged. Below it, only ordinary directories and
  official Cloud-family directory objects whose own content is locally available are admitted;
  partial, unverified, unknown-reparse, junction, and volume-transition states are rejected. An
  available parent never implies that the selected file is available, and offline sibling files
  are neither enumerated nor opened by this read. This policy does not claim live Cloud Files
  acceptance for every provider or partially populated directory state.
- The bridge transfers an opaque read lease and its approved path, never full-resolution image
  bytes. Admission bounds encoded source size to 256 MiB, path length to 32,767 UTF-16 units and
  256 components, and process-held source leases to two. Exceeding a bound is an explicit source
  failure with the existing labelled preview fallback, not an unprotected read retry. The Dart
  scheduler retains one active read and only the latest pending selection. Cancellation discards
  stale intent without closing a lease still needed by the native buffer copy. The lease is closed
  on success, failure, supersession, and disposal after that copy settles. Source image-cache
  identity includes the active scan and source lease plus explicit retry generation; a preview
  update alone does not invalidate it.
- Flutter still owns codec creation, image streams, painting, panning, and zoom. The pinned SDK's
  `ImmutableBuffer.fromFilePath` implementation copies mapped bytes into owned `SkData` before
  completing its Dart future (`immutable_buffer.cc`, `initFromFile`), establishing the lease-release
  boundary. Windows read-sharing semantics are checked against Microsoft's
  [Cloud Files open flags](https://learn.microsoft.com/en-us/windows/win32/api/cfapi/ne-cfapi-cf_open_file_flags)
  and [placeholder state definitions](https://learn.microsoft.com/en-us/windows/win32/api/cfapi/ne-cfapi-cf_placeholder_state).
  Reusing unchecked `FileImage` was rejected because its ordinary pathname open can follow replaced
  state or recall a placeholder; transferring original bytes through the desktop bridge was rejected
  because it duplicates large buffers and bypasses the existing decoder boundary.
- A small source-stream resource owner composes `MultiFrameImageStreamCompleter`: errors from an
  actually retired intent have no presentation authority, while current decode errors still reach
  Flutter's error handling. Its codec proxy only releases frames arriving after retirement before
  handing them to Flutter, and releases its inner codec exactly once. Neither error suppression nor
  file-lease release is inferred merely from codec creation completing; late first-frame success and
  failure are separate regression cases.
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
- Layout shape, density, and refinements within the current source scope (including sort field,
  direction, filters, and search) preserve one logical viewport anchor identified by the actual
  card nearest the anchor point, stable location ID, and card and viewport fractions. Explicitly
  selecting Library, another source root, or a child folder starts that newly selected scope at its
  first result instead of carrying over the previous scope's time or viewport anchor.
  Unknown-dimension recovery uses the same
  center-card contract, freezes the current logical range until later native movement, and expands
  preview demand from the center toward both sides. A new query obtains the anchor's new ordinal
  from the application layer; only a confirmed absence falls back intentionally to the first
  result. The replacement sliver applies the new geometry through its first layout correction,
  before painting, rather than painting one position and jumping afterward.
- Date headings use capture time, then file creation time, then file modification time as defined by
  ADR 0008. An unrepresentable date remains an explicit unknown section in the same continuous
  gallery.
- The right-side annotated time rail is the only visible scroll-position control. Its controlled
  Material Slider derives its stable full range from the complete catalog timeline and its current
  value from the gallery's sole `ScrollController` plus the materialized window's global start
  offset; it never owns an independent committed month or timeline position. Loading another page
  must not extend the rail. Exact day offsets are used where rows are materialized, while bounded
  off-window seeking uses the catalog time-anchor contract.
- A newly selected time target owns navigation until it fails, is replaced by a newer time target,
  or the user performs native gallery movement. Visible-range synchronization, initial scroll
  metrics, preview publication, recovered dimensions, and late layout or manifest callbacks are
  passive observations and cannot cancel or replace that target. Native movement cancels the
  application request explicitly; cancellation is not inferred merely because an older visible
  range does not yet contain the target. Beginning the explicit interaction also retires the prior
  dimension-recovery range; the target's first usable viewport metrics establish its replacement,
  without requiring a manual scroll.
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
  no permanent task destination. A multi-root update remains one temporary surface with
  independently labelled root rows rather than collapsing failures, progress, or cancellation into
  one synthetic cross-root task. Closing Ame cancels every admitted update and waits for each scan
  stream to finish; queued roots are cancelled before they start. Batch progress is not presented as
  resumable after restart, while the last trustworthy published catalog remains available.
- A retained unfinished first import is restored as a paused task awaiting explicit Continue after
  window closure or an abnormal exit. Startup must not execute its checkpoint automatically.
  Cancelled tasks remain terminal. Source registration without a completed baseline is not displayed
  as active synchronization; an existing completed catalog remains usable during later updates.
- A retained task is not a gallery-wide busy state. Folder navigation, paging, synchronization
  refresh, Settings return, and work on other roots remain available while it awaits Continue.
  Its task identity, progress, and actions survive gallery-query changes. Continue and Cancel are
  both explicit actions; cancelling a retained task persists its terminal state without starting
  enumeration, and a failed cancellation keeps the checkpoint and retry feedback. Removing its
  root clears the retained task only after catalog unregistration commits. A new primary import
  cannot silently replace the retained one; this does not introduce multiple primary imports.
- Non-task status changes and failures use one bounded bottom notification queue. Source rows retain
  only their compact availability or freshness label; reconciliation cause, affected counts, source
  path, stable technical code, and retry or reconcile action belong to the notification detail
  surface and current-session history. Persistent failures remain until acknowledged or resolved,
  transient success notices dismiss automatically, and repeated active conditions update one
  deduplicated history entry instead of generating notification spam.
- Active, paused, cancelling, failed, cancelled, and completed scan feedback retains priority over
  the notification surface. Acknowledging a notification removes it from the bottom queue without
  falsifying source freshness or deleting its bounded history entry.
- Settings is a sidebar-selected destination rendered in the existing main canvas. It keeps the
  global bar and source sidebar visible and never opens an application-settings dialog.
- The settings canvas uses shallow, plain-language Material rows grouped as Personalization,
  Browsing, Storage, and About. Each row has an icon, a user-facing name, one short explanation,
  and a right-side Material control where the setting is actionable.
- Flutter `Card` and `ListTile` own settings grouping and row semantics. `OutlinedButton`,
  `CheckedPopupMenuItem`, and progress indicators own choices, actions, and storage feedback; custom
  code only supplies the responsive page composition and Ame-owned state connections.
- Internal database versions, task queues, raw worker counts, hash engines, and analysis parameters
  are not user settings. The preview loading preference is the narrow exception: it exposes only
  `small`, `medium`, and `large` resource policies while ADR 0005 retains ownership of the internal
  concurrency limits. Account, cloud-service, and media-editing controls remain absent until their
  workflows exist end to end.
- Simplified Chinese is the initial user-facing language. Paths and source metadata preserve their
  original text.
- Simplified Chinese text resolves through Windows `Segoe UI`, `Microsoft YaHei UI`, and
  `Microsoft YaHei` in that order. Body copy remains regular, controls and compact labels use
  medium weight, and page or content headings use semibold so hierarchy is semantic rather than
  assigned independently by each widget.
- The theme preference controls brightness. `system` follows Windows brightness, while explicit
  light and dark choices lock brightness. All three modes derive their Material color scheme from
  the current Windows accent color and update when that accent changes; the accepted Ame blue is
  the fallback when Windows cannot provide a palette.
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
verified to provide `OutlinedButton`, `CheckedPopupMenuItem`, root-navigator `showMenu`, initial-value
positioning, selection callbacks, disabled items, focus traversal, and configurable
`AnimationStyle`. Its `DropdownMenu` was also evaluated, but it constructs an internal
`MenuAnchor`/`OverlayPortal` and does not expose the route-level transition contract used elsewhere
in Ame. It is therefore not retained as a settings exception on Windows 11 x64. The
repository-owned `SettingsChoice` composes the first-party button and checked popup items through
`AmePopupMenuButton`; preview loading speed and preview-cache budget reuse that control with their
existing widths, current-value rendering, disabled state, keyboard behavior, and semantics.

The official Material 3 Dialog and Checkbox catalogs remain the basis of multi-root library update
selection. The repository-pinned Flutter 3.44.9 SDK provides `showDialog`, `AlertDialog`, and
`CheckboxListTile`, including keyboard, focus, dismissal, checked-state semantics, and the dialog
route. Ame adds only configured-root availability and active-task eligibility plus the selected-root
set; it does not implement a parallel custom selection control.

For explicit query failures, the [official Material component catalog](https://m3.material.io/components)
was evaluated alongside the pinned SDK's
[MaterialBanner](https://api.flutter.dev/flutter/material/MaterialBanner-class.html).
Flutter 3.44.9 `material/banner.dart` provides static non-modal content and framework action buttons;
Ame composes it with `TextButton` rather than adding an overlay, snackbar timer, or custom focus
control. The product-specific layer supplies the failed query and its read-only retry. Existing
linear loading feedback and menu motion are unchanged.

The official Material 3 Icon button, Menu, and Badge catalogs were evaluated for notification
history and unread state. Flutter 3.44.9 provides `IconButton`, `MenuAnchor`, constrained
`MenuStyle`, root-navigator `showMenu`, configurable `AnimationStyle`, and small `Badge`; the pinned
Material Symbols package also provides distinct
`notifications_rounded` and `notifications_unread_rounded` glyphs. The accepted bar uses the two
glyph states because the product requires an icon swap with a dot and no count. The bounded history
reuses `AmePopupMenuButton` so pointer, keyboard, focus, dismissal, and accessibility behavior remain
framework-owned; the product layer owns only queue state, text, severity, and action routing.

Flutter 3.44.9 can publish retained `OverlayPortal` semantics updates that violate Windows
`AccessibilityBridge` transaction preconditions. Real retained-catalog interaction produced
repeated orphan-node rejection after the earlier dedicated `Semantics` container mitigation, so
that wrapper is not accepted as sufficient on Windows 11 x64. The first-party `Tooltip` was
evaluated but cannot be retained on this pinned Windows engine because its visual surface is owned
by that failing portal path. `AmeTooltip` instead keeps one stable framework `Semantics.tooltip` on
the target and creates a visual `OverlayEntry` only after pointer hover. The visual entry is
`IgnorePointer`, excluded from semantics, bounded to the screen, fades through a lazily allocated
framework `AnimationController`, and is removed after exit or target disposal. It neither retains
an idle route nor replaces focus, keyboard, activation, or screen-reader ownership. Other platforms
retain the first-party tooltip. Updating a visible tooltip from a target rebuild schedules its
overlay invalidation after that frame: the root entry is a sibling, not a build descendant. The
callback must still own the same live entry, and removal must dispose it. Target replacement or
disposal cannot update a retired overlay. The pinned SDK's animated
`MenuAnchor` can also paint a follower menu at its final visual position while its pointer hit-test
geometry still resolves to content behind that menu. Ame therefore does not retain `MenuAnchor`
portals for application dropdown or command menus: settings choices, preview-cache budget, source,
folder, photo, toolbar, notification, and viewer menus all create the same root-navigator
`PopupMenuRoute` only when requested. The shared route owns one visible Material transition
contract: 200 milliseconds, ease-out cubic when opening, and ease-in cubic when closing.
This prevents motion from changing with the call site while retaining framework-owned pointer,
keyboard, focus-return, dismissal, and accessibility behavior. Changing menu motion is a single
application-wide decision and requires the native accessibility, visual placement, pointer hit-test,
and keyboard canaries to pass. A later pinned SDK upgrade may return Windows to the first-party
tooltip only after that same native accessibility sequence passes. Accessible names, keyboard
activation, focus return, visual hover, and complete-path discoverability remain required. No Ame
menu retains an idle route. This matters most for virtualized photo tiles, where retaining one `MenuAnchor`
portal per visible tile multiplies the number of subtrees detached and introduced during sliver
recycling. Photo tiles therefore keep their Material focus, pointer, and keyboard entry points but
create one root-navigator popup route only when a menu is requested. Dynamically expanded folder
rows use the same on-demand route ownership rule.

Material `Slider` also retains a value-indicator portal. When the gallery's timeline Slider becomes
offstage inside the viewer `IndexedStack`, Flutter can prune that portal node and then serialize its
old child identifier when the retained Slider returns. Ame therefore recreates only the timeline
navigation subtree when the viewer closes; the gallery `Scrollable`, its `ScrollController`, and
the logical viewport anchor remain intact.

The regression harness forwards real incremental Flutter semantics updates into a retained
child-graph model. It commits a batch only after validating the initial root, child existence,
single traversal ownership, cycle freedom, reachability, and the Windows requirement that a moved
existing child receive an update in the same reparent transaction. This is an AXTree-precondition
model rather than an implementation of Chromium AXTree ordering. The native Windows accessibility
integration remains the authoritative canary for `AccessibilityBridge` stderr failures.

Windows accent-color integration uses the operating system's documented
`DwmGetColorizationColor` API and `WM_DWMCOLORIZATIONCOLORCHANGED` notification through an Ame-owned
Runner platform channel. `AmeSystemThemeBuilder` exposes only a Flutter `Color` to the presentation
theme, ignores stale startup reads after a newer change notification, and falls back to the accepted
Ame blue when Windows does not provide a color. `system_theme` 3.3.0 was evaluated and rejected
because its Windows listener had avoidable null-message and cancellation-lifetime hazards. Ame owns
the narrow adapter, `ColorScheme.fromSeed`, brightness policy, and fallback, with no third-party
theme type crossing into widgets or application contracts.

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
- Retained semantics tests exercise application-level tooltip, stable menus, timeline Slider,
  viewer Slider, viewer menu, viewer-return sequences, and deep virtual-gallery jumps with repeated
  on-demand photo menus against the retained Flutter semantics child-graph model.
- The native Windows accessibility integration runs both a virtual-gallery stress sequence and a
  populated partial-detail application sequence covering adjacent source controls, stable toolbar
  and source menus, menu dismissal during movement, deep final-geometry unloaded-slot replacement,
  real pointer-driven timeline and viewer Sliders, viewer menus, repeated viewer return, and a
  settings choice opened through the shared popup route. A test-only, nonce-bound checkpoint
  first activates the target process's `FLUTTERVIEW` through the documented
  [AccessibleObjectFromWindow](https://learn.microsoft.com/en-us/windows/win32/api/oleacc/nf-oleacc-accessibleobjectfromwindow)
  `OBJID_CLIENT` request. The isolated MTA probe finds windows without caching an empty UIA provider,
  revalidates process and class immediately before the request, and releases the returned interface.
  This happens in suite setup before `testWidgets` records its semantics-handle baseline; the suite
  also requires the real platform dispatcher to report enabled semantics. Flutter 3.44.9's Windows
  embedding creates its native bridge on the OS accessibility request, not on a test-only
  `ensureSemantics` handle, and drops updates sent before that bridge exists. Activation alone is
  not evidence of a valid tree: all nine subsequent UIA phases and engine stderr checks remain
  mandatory. No production runner setting, global screen-reader setting, or SDK patch is involved.
  The same nonce-bound checkpoint
  protocol holds the application at deterministic open and dismissed states while an independent
  PowerShell UI Automation client locates the runner's direct desktop child window by process ID
  and traverses its native descendant tree. The gate requires phase-specific exact accessible names
  and control types on visible reachable controls, requires popup names to be absent after
  dismissal, rejects an incomplete or out-of-order checkpoint transcript, and still rejects every
  `AccessibilityBridge` `ui::AXTree` error captured from the engine. The Flutter command is admitted
  to a kill-on-close Windows Job Object before execution and has a 15-minute parent wall-clock
  deadline. Native UIA reads run in a separate single-purpose probe process, not on the deadline-
  owning parent thread. Each probe has its own kill-on-close Job Object and a parent-enforced
  deadline of at most eight seconds, further capped by the complete gate's remaining time. A blocked
  provider therefore cannot prevent the parent from terminating the probe and test process trees.
  That parent is the only deadline owner. The probe atomically publishes bounded, nonce-, phase-,
  and process-bound progress at native-call boundaries, including the last assertion mismatch.
  After termination the parent validates and retains the last complete evidence record; progress
  is never sufficient for acknowledgement or success. This distinguishes loading, window lookup,
  tree traversal, and property reads without extending the deadline or weakening the UIA contract.
  Each traversal uses a fresh
  [UIA CacheRequest](https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/use-caching-in-ui-automation)
  to obtain the same six properties in bulk. Its raw-view, element-only cache scope applies to
  every result of the unchanged full-subtree search; no snapshot survives a retry or checkpoint.
  The dedicated PowerShell probe starts explicitly in MTA and checks that apartment before making
  any UIA call, following Microsoft's
  [UIA threading contract](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-threading).
  This affects neither the Flutter thread model nor the parent's process deadline.
  A controlled blocked-probe fixture verifies deadline rejection and descendant cleanup without
  traversing a real application or library. The process-completion evidence boundary replaces the
  unified output log with the current Flutter output, validated phases, exit code, and run/cleanup
  failure metadata before returning a run or cleanup exception. A failed or timed-out run must not
  retain a previous successful transcript. This native evidence complements rather than weakens the
  model's deterministic invariant coverage.
- Pure layout and widget tests confirm balanced justified rows fill one gallery width and sparse rows
  do not exceed their enlargement limit.
- Flutter analysis, the full Flutter test suite, and a Windows Debug build passed for the accepted
  prototype.

## Replacement strategy

A later presentation change must preserve the unified-gallery workflow unless new user evidence
explicitly supersedes it. Framework replacement or a different navigation model requires a new ADR
with measured accessibility, performance, migration, and maintainability consequences.
