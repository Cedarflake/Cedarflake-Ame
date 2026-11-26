# ADR 0029: Separate workspaces from gallery scope

Status: Accepted.

Date: 2026-10-16.

Supersedes ADR 0009 only for shell navigation, duplicate entry and duplicate review placement.
Its gallery, viewer, source safety and Material interaction contracts remain in force.

## Context and decision

### Material surface composition

The 2026-11-16 accepted layout uses distinct tonal roles for the primary rail, contextual chrome
and content. Resolve them from the existing Material color scheme in both brightness modes;
retain dynamic accent and theme preferences. Gallery and Albums keep one bounded gallery and
their existing contextual sidebars. Duplicates places exact/similar/ignored navigation in its own
sidebar while retaining degree and cleanup beside the result heading. Secondary sidebars omit a
redundant large destination title.

Follow Material's [bright/dim large-screen example](https://m3.material.io/styles/color/the-color-system/):
the navigation backing uses `surfaceDim` and the main content uses `surfaceBright`.
Contextual chrome uses `surfaceContainer` between them. Keep these roles unchanged across light
and dark appearances; the generated scheme owns their tones. This retains a darker navigation
backing and brighter content in both appearances without swapping lowest/high roles by brightness.
The earlier `surfaceContainer` / `surfaceContainerLow` / `surface` combination made the light-theme
regions too similar. The wider tonal range separates the existing three regions without new borders
or shadows.
The search field retains the installed `SearchBar` default `surfaceContainerHigh` background,
independently of the main content surface; this default was checked in `material_ui` 1.11.0.

Material's [canonical layouts](https://m3.material.io/foundations/layout/canonical-examples/overview)
and the [Android layout guidance](https://developer.android.com/develop/adaptive-apps/guides/canonical-layouts)
inform the existing adaptive gallery feed and viewer supporting content. Navigation remains
separate from those content regions; this does not introduce another pane or navigation model.
Use `ColorScheme.fromSeed` with `DynamicSchemeVariant.tonalSpot` to retain the system accent hue
with restrained surface and selection chroma. The earlier fidelity palette made primary actions
and selected rows similarly prominent. Navigation uses a shared selection pair from
`AmeWorkbenchSurfaces`: blend 40% `primaryFixedDim` in light appearance or `primaryContainer` in
dark appearance over `secondaryContainer`, with `onPrimaryFixed` or `onPrimaryContainer` text
respectively. The default secondary container appeared too pale against the workbench backing,
while the full-strength primary containers were too prominent. This intermediate selection gives
navigation clearer accent without competing with primary actions. The primary rail, Settings destination,
collection/source/album rows and duplicate navigation share the pair; other controls retain their
existing color roles. Hover and focus continue to use the framework state layers.
Regular workspace headings and ordinary navigation text keep the content hierarchy legible;
selected labels retain emphasis. Calendar and dialog typography stays owned
by those surfaces. The installed color-scheme and text-style APIs provide this composition without
new fonts, palette code or dependencies.

One backing surface joins the top bar and contextual sidebar, with rounded outer left corners.
Sidebars do not introduce another rounded cutout below the top bar. The main content has its own
rounded surface and a small outer inset. The frame owns this geometry; sidebar transitions retain
their existing destination-width rendering and clipping.
The frame measures the three-destination rail and hamburger as one bounded `IntrinsicWidth` column.
Its adjacent chrome column contains both the top bar and workspace, so translated labels and enlarged
text cannot separate their leading edges. The rail's 72-pixel minimum is not a fixed background offset.
Sidebar materials remain transparent so they cannot paint over the shared backing's outer corners.
Collapsed sidebars and Settings retain the same eight-pixel content inset. The workspace-width
owner carries the visible boundary through destination changes, including reversals, while each
destination lays out at its final width. Reduced motion completes the boundary change immediately.
Each active pane reports its displayed inset to the workspace-motion owner. A destination change
starts from that inset, including an interrupted sidebar transition; the sidebar then settles its
local extent so the workspace transition alone carries the handoff. Hidden panes cannot replace
the active pane's reported boundary.

The shared search field has a leading search symbol and a trailing options arrow. Existing mode,
scope, clear, progress and retry semantics remain intact. Settings uses a circular selected surface.
The top-left hamburger controls contextual-sidebar visibility; the plain application mark and `Ame`
wordmark share the adjacent brand area. Search uses the configured sidebar width independently of
its visibility animation, retaining the placement rules in the
[global shell contract](../product/gallery/workbench-and-navigation.md#global-shell).
The brand's drag region starts after the hamburger's rail-width target.
Primary navigation symbols use the standard 24-pixel Material size without reducing their targets.
The workbench brand pairs a 36-pixel asset box with a 20-pixel wordmark. Navigation destinations
use the component's built-in spacing; the earlier additional 12 pixels made the group too spread
out. Selected symbols use the filled variant and unselected symbols use the outline variant,
retaining their existing weight, size and framework-owned selection motion.
The image viewer keeps straight canvas edges and its existing operation placements. Its rounded,
elevated thumbnail strip remains an overlay, with no change to image geometry when toggled.
Repaint boundaries isolate the strip's moving clip from the main image and retain the static strip
display list; movement retains its shared motion driver, shadow, clipping and preview lifetimes.

Use the installed `material_ui` 1.11.0 SearchBar, NavigationRail, ListTile, Material/Card and popup
components, plus the existing window and transition owners. No new navigation stack, widget
framework, dependency, catalog state or source read is introduced. The owning
[plan](../plans/material-workbench.md) identifies affected workflow verification; rollback restores
composition without a data migration.

### Retained workspace behavior

Ame needs stable destinations for browsing and duplicate analysis as its organizing capabilities
grow. A permanent narrow function rail selects Library, Albums and Duplicates in that order;
Settings is pinned below.
The Library workspace contains a collapsible source/folder sidebar and the existing photo wall.
Favorites is accessible only from Albums; Library contains source and folder navigation.
Albums owns a separate flat album sidebar and uses the same photo wall.
The other workspaces own their contextual controls without duplicating the catalog or viewer.
The accepted interaction blueprint is in [the gallery contract](../product/gallery/workbench-and-navigation.md#workbench-blueprint).

Sidebar order belongs to a separate typed application/repository boundary. Schema 58 stores one
position per imported root and album, preserving the previous visible order on upgrade. Creation
appends, rename and catalog publication retain positions, and explicit removal retires the matching
position. Album retirement retains its slot for undo. Relative moves revalidate both current items
and update the affected interval in one prioritized user transaction, without reading source media
or changing image-query revisions. Album keyset pages follow the saved positions.

Navigation order is local presentation state. Normal reindexing retains the configured roots and
album definitions and therefore their order. Catalog snapshots include it; portable organization
archives retain their decision-recovery contract and append newly restored definitions to the local
sidebar without overwriting an existing arrangement. A forward migration and verified pre-upgrade
snapshot provide the existing upgrade/rollback boundary.

Library and Albums use one section-heading component below their Library and Favorites entries.
The 2026-11-17 layout revision restores `添加文件夹` and `新建相册` as compact plus actions
beside those headings. Large filled top actions gave creation disproportionate space and emphasis.
The installed `material_ui` 1.11.0 `IconButton` supplies nullable activation, disabled styling,
keyboard focus and semantics; existing descriptive tooltips name both actions. Album disclosure
remains separate beside creation. Library/Favorites begin the sidebars, without a reserved action
block. No custom input handling is introduced. Flutter 3.47.8's
`ReorderableListView.builder`, `ReorderableDragStartListener` and `onReorderItem` provide lazy rows,
drag geometry, scrolling and reorder semantics. Only the imported root row starts a folder drag;
its expanded descendants remain ordinary navigation. `LibraryFolderReorderItem` collapses the branch
when the framework recognizer accepts a drag, then hands the drag to the list after the next layout.
This prevents the expanded subtree from becoming an oversized drag target. A pointer released or
canceled during that handoff retires it; dropping leaves the branch collapsed. The reorder list's
local `Overlay.wrap` owns proxy coordinates and clipping within the sidebar, below route-only
captured themes. The presentation order owner serializes saves,
retains current item identities through metadata updates, appends new items and restores the prior
order on failure through the shared feedback surface. It does not reorder gallery images or mutate
physical folders. Sidebar navigation rows and their drag proxies share a framework
RoundedRectangleBorder with eight-pixel corners. Their 16-pixel side insets align the background's
leading edge with the brand mark. Single-line rows and source rows with status share a 48-pixel
minimum height; text scaling and embedded controls can increase that minimum. This replaces the
40/56-pixel split so the selection surface does not change proportion between navigation types.
Navigation uses the theme's body-large labels and 24-pixel icons. Source and child-folder rows
omit ListTile's legacy dense setting, which overrides the title with a 13-pixel size even when
the theme supplies body-large text. Source status uses body-small typography; trailing controls
take their intrinsic width instead of reserving space for an absent expansion button.
Source names and statuses compose a minimum-height Column in ListTile's title slot, allowing the
framework to center them together rather than applying fixed two-line baselines to a compact row.

Manual and smart albums use shared section headings and framework icon buttons for disclosure.
Manual rows share Ame spatial/effects controllers and native SizeTransition/FadeTransition widgets inside
the existing lazy reorder list. The completed collapsed state has no row children; the bounded page,
scroll owner, order, selection and active collection remain retained. Smart albums reuse the existing
folder disclosure owner; empty, loading and failed manual-album states use that same owner and timing.
Closing retires row input, focus and semantics immediately. A nested
shrink-wrapped scrolling list is unnecessary and would disconnect native drag auto-scrolling.

Sidebar selection is a separate presentation owner. The installed Flutter 3.47.8 `IconButton`
provides focus, toggled semantics and the original/selected icons; a 24-pixel filled circle with the
existing Material Symbols check identifies a selected row inside its 40-pixel target. The section
heading keeps `已选择 (N)` (`已选 (N)` at compact widths), Select all, removal and disclosure in one row. Creation
is shown outside selection. This replaces the two-row arrangement that shifted the list on selection;
the full selected-count label remains available to accessibility. The shared framework tristate
`Checkbox` uses the shared circular shape with an 18-pixel neutral outline without fill; the other actions use 20-pixel
symbols, all within 36-pixel targets. Material's checkbox and icon-button component documentation was inspected;
no replacement gesture, checkbox painter or package is needed. Selection disables reordering, and
Escape clears it. A shared selection region owns pointer focus and the Escape shortcut outside row
decoration; selection icons use framework keyboard focus without requesting it after mouse clicks.
Primary-workspace departures clear secondary-sidebar selection. Child folders never enter this selection set. Album selection remains on its
bounded 64-entry page, with an explicit current-page accessible label when more albums exist.

A typed removal queue serializes the existing guarded commands and records each committed item.
One framework confirmation dialog lists the frozen selection before execution. Stopping or disposal
prevents the next command; an in-flight commit remains valid. Failures retain the completed subset,
and retries submit only remaining items, retaining unknown album-operation identities and renewing
confirmed stale authority through the existing command owner. Album retirement uses its existing
history, undo and publication; source removal retains its path admission, committed display projection
and source-record cleanup. No new schema, source mutation or all-or-nothing batch guarantee is added.

Album More, secondary-click and keyboard menus share one Material popup owner and capture the album
identity and callback when opened. A separate photo context-menu owner admits one bounded group
inspection, freezes image identity/callbacks, rejects stale or disposed completions and reuses guarded
Favorite and membership commands. The shared album sheet owns selection, errors and confirmation;
direct membership removal uses the existing notice/retry path. Popup placement, keyboard navigation,
focus return and dismissal remain in the installed SDK's existing Material menu route. No new
overlay, database access, source operation or menu package is introduced.

Use the existing Flutter Material 3 design system. The installed Flutter 3.44.9 SDK provides
`NavigationRail` with destination selection, labels, focus, semantics and a bottom trailing action;
`Visibility.maintainState`, `TickerMode` and `ExcludeFocus` retain inactive workspace state while
retiring input and animation. Existing source navigation and resize controls remain reusable.
The product-specific responsibility is destination lifetime, folder-panel preference and composing
the existing gallery with a dedicated duplicate workspace. No new UI package is needed.

The secondary-sidebar toggle is the global bar's hamburger above the function rail, outside the
collapsible sidebar. Library, Albums and wide Duplicates share the visibility preference and motion;
the compact Duplicates navigation bar and Settings have no collapsible contextual panel.
Store its visibility with existing presentation preferences. Preserve source scope, expanded
folders on workspace switches. Primary-sidebar switches to Albums, Duplicates or Settings clear
ordinary selected images, exit selection mode and reset the scroll position on return. An organizing
Library retains its mode, source/queue query, logical position, selection and ordinary return context
across those visits. The collection workspace itself never enters organization mode.
Narrow windows may initially hide the
secondary sidebar; explicitly opening it restores the same panel and its retained state.

`LibraryBrowseWorkspaces` owns the two bounded browsing snapshots and serial destination changes.
It retains query snapshots with empty ordinary selection and position; an organization-mode departure
keeps its complete active snapshot. A departing active review session retires before the other
collection can use the shared gallery. Returning restores the query before reactivating organization
mode and its selection. The owner distinguishes the visible primary destination from the retained
gallery. Returning from Settings or Duplicates resets an ordinary gallery through the same query
transition, while an active organization session remains intact. `LibraryReviewQueryTransition`
owns the requested target and reload decision. An unchanged query whose first window remains loaded
can reset its position without another catalog read. A later window, saved review anchor, pending
window request or query failure retains the existing refresh and admission path. The application's
query owner still admits changed collection/source queries and rejects stale results.
A failed query keeps the previous
destination; the existing query retry
replays the whole navigation intent. Album retirement invalidates both saved and in-flight album
restoration before it can publish a destination. Disposal retires pending publication.
The shell retains both sidebars and one gallery subtree through the existing visibility/focus
owners; the unselected Albums destination uses the shared empty-state component. No additional
gallery controller, database, package or navigation primitive is introduced.

Compose its bounded spatial transition through the shared official Material motion driver and the
pinned SDK's `AnimationController`, `AnimatedBuilder`,
`ClipRect`/`ClipRRect` and `OverflowBox`. The gallery transition clips its backing surface to the
same rounded boundary as `LibraryMainSurface`. The sidebar boundary and gallery's left clip move
together; the destination-width gallery remains right-aligned inside that clip. Left alignment
would translate its right-edge controls while the clip and child have different widths. The toolbar
also remains right-aligned below the title in narrow layouts. The gallery receives only its final
width for each toggle; its query-wide geometry must not reflow at every animation frame.
The moving boundary owns the gallery's only rounded clip through `LibraryMainSurface`. This surface
paints its rounded tonal background outside the content clip and supplies a transparent Material for
ink and text. The workbench backdrop does not clip the photo wall again; the sidebar clips only its
own bounds. Standalone Duplicates and Settings reuse the same surface and own their content clipping.
The photo viewport separately follows the moving left boundary through a left-aligned OverflowBox
inside a right-anchored clip. Its toolbar and time rail remain outside that content motion.
The title/count block consumes the same leading offset through Transform.translate, without
changing the toolbar's layout or right alignment.
`LibrarySidebarContent` also accepts centered alignment for the gallery's empty-state and floating dock
slots and the Albums landing surface. Its outer width remains the destination width; the clipped
visible width follows the same sidebar progress, and the inner destination-width content centers
within it. `OverflowBoxFit.deferToChild` preserves a footer's intrinsic height. The pinned SDK owns
layout and hit testing, with no second animation clock or per-frame catalog reflow. The viewport
overlays the dock at the bottom without shrinking the photo viewport. Only its controls capture
input; transparent space remains part of the gallery. The existing trailing scroll padding clears
the final row above the compact dock.
`LibraryGalleryLayoutMotion` owns a bounded registry of mounted sliver items and interpolates their
rectangles for sidebar and window-size changes. The installed SDK's AnimatedBuilder, Transform and
OverflowBox retain the existing scroll, input and semantic primitives; AnimatedSize alone cannot
carry an item across lazy rows, and a whole-wall scale would distort icons and rounded corners.
Resize each tile with interpolated constraints rather than scaling its rendered contents. Retarget
from the displayed rectangle and retire item records on unmount. Duplicate-display switches and
same-scope background duplicate publication may reuse that bounded reflow; unrelated queries,
direct scrolling and reduced motion retire it. The existing geometry owner supplies its pending scroll
correction so the first animated frame accounts for the sliver's later layout correction.
Departing duplicate cards retain only painted pixels, never live providers, focus or actions.
The exit owner captures at most 64 visible cards and four million pixels, limits each image's
longest edge to 512 pixels, and releases them after the effects spring or interruption. Its narrow
`RenderRepaintBoundary` subclass only checks for an existing paint layer before the installed SDK's
`toImageSync`; Flutter continues to own rendering. Repeated publications coalesce these visuals.
Chronological duplicate publication keeps the current date vicinity when its browsed copy disappears
and the representative belongs to a different date. The presentation position owner uses the coherent
compact manifests, without catalog policy changes or extra source reads. Existing positions and
explicit viewer identity retain their normal resolution. Capture a scroll edge with its browse
position, so temporary filter clamping cannot replace the saved browsing intent. The time rail
continues to derive its position from the actual gallery viewport.
Viewport anchors capture start/end position from the published layout's scroll range as well as
the image fraction. Resizing or sidebar toggling retains that edge; an intervening user scroll
recaptures the current position through the existing pending-reflow owner.
A plain `AnimatedSize` around the sidebar would propagate intermediate
widths into the photo wall. Retain both widget subtrees, disable collapsed-sidebar input, finish
hidden or reduced-motion transitions immediately, and reverse rapid toggles from the current value.

The gallery filter owns All images and Merge duplicates. Duplicate-only browsing, analysis control,
group inspection and durable decisions belong to the Duplicates workspace. Both consume one typed
analysis service. Only merged gallery mode displays the duplicate icon, without a count; all-images
mode leaves the cards unmarked. Exact evidence and later perceptual candidates remain distinct.
GalleryTimeline carries the source-location count separately from the displayed count. The SQLite
timeline owner reads both in the existing query snapshot transaction, sharing root, folder and
search filters. All-images mode reuses its displayed count; merged mode counts source locations
without the representative filter. Presentation labels these as total images followed by the
merged count, without loading all items or summing unfiltered root totals.

The gallery uses the existing `AmePopupMenuButton` around Flutter's `showMenu` and
`CheckedPopupMenuItem` for its two display choices. The duplicate badge opens a modal bottom sheet
through `showModalBottomSheet`, with a lazy `ListView` and continuous bounded reads. Long filenames
and folder paths wrap completely within the available width. The pinned Flutter 3.44.9 SDK's
`material/bottom_sheet.dart` provides the drag handle, safe-area handling, dismissal barrier,
Escape dismissal, route focus and Material 3 width/shape defaults. Match the existing information
sheet and size the list to 72 percent of the window height. Keep its heading outside the list;
there are no close or pagination buttons. Restore the badge's existing keyboard focus when it
remains mounted in the visible workspace; mouse dismissal does not create new focus. An anchored
menu lacks space for complete paths; a centered Dialog was the earlier surface but does not meet
the bottom-sheet interaction. DraggableScrollableSheet is unnecessary for a fixed-height list;
the framework bottom sheet already supplies handle dismissal and scrolling remains in ListView.
Material's
[menu](https://m3.material.io/components/menus/guidelines) and
[bottom sheet](https://m3.material.io/components/bottom-sheets/guidelines) contracts supply focus,
dismissal and choice behavior. The M3 component index and bottom-sheet pages require JavaScript;
the [Flutter API](https://api.flutter.dev/flutter/material/showModalBottomSheet.html) and installed
implementation establish the available behavior. Ame supplies query scope, persisted display
preference, duplicate badges, filename/folder presentation and current-location marking. The lazy
list reports the built index range to the shared LibraryDuplicateReviewWindow, retaining its
64-location query batches, six cached pages and 64 keyset checkpoints. This replaces the obsolete
previous/next controller. Catalog changes revalidate the visible range and stale responses lose
publication authority. The sheet retains
the gallery's browsing position; no second positioning, focus or scrolling implementation is added.

The duplicate workspace composes Material `Card`, `Wrap` and the pinned SDK's lazy
`GridView.builder` with `SliverGridDelegateWithFixedCrossAxisCount.mainAxisExtent`. These provide
card surfaces, wrapping actions, bounded widget creation and framework scrolling. Ame chooses
column count from the available width and text scale and owns visible preview demand for those
columns. Groups retain responsive columns; members use one column to expose long paths. Both levels
share a thumbnail/text area. Group cards retain a lower-right text action; members place two
48-pixel IconButton targets at the right, vertically centered, without a footer. The pinned SDK
provides IconButton.isSelected/selectedIcon and focus; each icon supplies its semantic action label
and AmeTooltip supplies the existing Windows-safe hover text. TextPainter measures installed typography, availability wrapping
and TextButton padding/target dimensions, so height follows actual content and controls.
A Wrap of every result would eagerly build the entire list and lose viewport-based preview admission.
No custom scrolling or additional package is introduced.

LibraryDuplicateReviewWindow owns continuous keyset-backed reads, six retained payload batches,
64 cursor checkpoints and generation-based response admission. The framework viewport reports the
visible ordinal range; unloaded items retain their final row geometry. Returning to an evicted range
reads from its nearest retained checkpoint, replaying bounded catalog batches when needed. Source
publication invalidates checkpoints. A background replacement owns a separate bounded window and
publishes only after the visible range is complete, retaining the prior rendered payload in the meantime.
This temporarily retains at most two six-batch payload windows; a newer publication or disposal revokes
the older response authority. Explicit refresh clears the window and exposes loading; failed reads
remain reported with retry. Late reads cannot clear a newer request or overwrite a saved decision.
Group and member windows are separate; neither retains every asset or thumbnail. The
viewer requests adjacent members through the same bounded window rather than stopping at a batch edge.

The page heading is `重复图片查找`, followed by the current review category's total group count.
Normal analysis status, scope and explanatory paragraphs are omitted from this surface. Refresh,
pause/resume and the selected group's ignore/restore operation share one wrapping action row.
The group-list heading/count block and right-aligned actions share one row and stack at a
text-scaled compact-width boundary. The count line reserves space during loading. Exact, similar
and ignored destinations use contextual Material ListTiles, or the installed NavigationBar when
the supporting pane cannot fit. The existing workspace owns destination changes. Material
`TextButton.icon` and `FilledButton.icon`, available in the pinned SDK, pair the existing action
labels with leading icons and retain framework padding, focus, disabled state and semantics.
The selected group's back/title block and right-aligned actions share a centered Row above the
text-scaled compact-width boundary. Below it, a Column keeps actions beneath the heading; their
Wrap aligns every run to the right. LibraryPathText retains filename truncation and its full-path
tooltip. This composes existing controls without a custom button or layout primitive.
A card-wide `InkWell`
supplies pointer, keyboard and hover behavior, while
the text/icon buttons keep their separate actions. The pinned SDK's `Card.clipBehavior` clips the ink to
the shared card shape; an embedded clickable `ListTile` would restrict feedback to the path region.
Group cards place a square representative image beside details, with a shared bounded preview demand.
The preview and detail slots align at the top, with the group action retained at the lower right.
The image slot selects its group without replacing the preview; a circular check and thin card
outline show selection. All member reads remain demand-driven.
The selected group retains its initial representative filename as the heading while scrolling;
content aligns beneath that heading. Member cards omit a separate preferred-copy label and the
decorative folder icon beside their existing thumbnail. Preference and reveal actions remain.
The page uses the gallery's existing two-pixel loading overlay around the result region below
the toolbar. Preference writes retain disabled-action and failure feedback without a page loader.
Sheet and member rows share one reveal owner for availability, in-flight admission and errors.
In the location sheet, ListTile.onTap is the single reveal action; no trailing IconButton repeats
it. The trailing slot displays `当前图片` only for the current location. The subtitle retains wrapped
paths and exceptional availability, and the tile's enabled state follows reveal admission.
The Windows platform adapter resolves a Unicode shell item with SHParseDisplayName and uses
SHOpenFolderAndSelectItems to open its parent and select it. Detached Explorer command startup only
reports process creation and does not establish selection in a newly opened folder. Dart's FFI
binds these OS APIs without an additional package; a worker isolate performs the synchronous COM
sequence off the Flutter thread. Every successful COM initialization is paired with uninitialization,
and every allocated path, output pointer and PIDL is freed on success and failure. Native HRESULT
failures return through the existing reveal error boundary. No source-content read or mutation is
performed. The existing application port keeps this Windows implementation replaceable.
Empty/error feedback reuses LibraryGalleryMessage with the gallery's centered icon, title typography,
spacing and optional detail/action rather than a separate plain-text presentation.

The member viewer's page route owns a framework `Overlay.wrap`, available in the pinned Flutter
3.44.9 SDK. The zoom slider's always-mounted value-indicator portal must share the route's offstage,
transition and disposal lifetime. The Navigator briefly lays out an incoming page offstage for Hero
measurement; targeting its outer overlay would expose the portal while its traversal parent is
absent. Keeping both inside the route preserves accessibility without replacing Slider, suppressing
semantics or changing the shared viewer fade, focus and dismissal behavior.

## Alternatives and consequences

Keeping all analysis inside the filter makes a growing workflow difficult to discover. A separate
bottom sheet also competes with the photo wall and temporary task feedback. A function rail provides
a stable destination while local badges retain fast access to individual duplicate locations.

Workspace visibility must retire background preview demand, focus and hidden animations without
recreating the photo wall. Background analysis remains application-owned and resource bounded;
changing destinations cannot restart hashing or dispose a running analysis.

## Validation and rollback

Verify primary-destination changes reset ordinary position and selection while preserving an
organizing Library; folder-panel changes retain browsing state. Keyboard focus stays restricted
to the visible workspace, the toggle remains reachable, and location activation reveals the exact
file through the existing platform adapter. Exercise narrow and wide windows in the client.
Automated boundary checks run in Actions. Rollback can restore the prior shell without discarding
versioned fingerprints, source identities, catalogs or user data.

References: [Material components](https://m3.material.io/components),
[Flutter IconButton](https://api.flutter.dev/flutter/material/IconButton-class.html),
[Flutter NavigationRail](https://api.flutter.dev/flutter/material/NavigationRail-class.html),
[Windows navigation](https://learn.microsoft.com/en-us/windows/apps/design/basics/navigation-basics).
