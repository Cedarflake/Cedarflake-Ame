# ADR 0029: Separate workspaces from gallery scope

Status: Accepted.

Date: 2026-10-16.

Supersedes ADR 0009 only for shell navigation, duplicate entry and duplicate review placement.
Its gallery, viewer, source safety and Material interaction contracts remain in force.

## Context and decision

Ame needs stable destinations for browsing and duplicate analysis as its organizing capabilities
grow. A permanent narrow function rail selects Library, Albums and Duplicates in that order;
Settings is pinned below.
The Library workspace contains a collapsible source/folder sidebar and the existing photo wall.
Favorites is accessible only from Albums; Library contains source and folder navigation.
Albums owns a separate flat album sidebar and uses the same photo wall.
The other workspaces own their contextual controls without duplicating the catalog or viewer.
The accepted interaction blueprint is in [the gallery contract](../product/gallery-ui.md#workbench-blueprint).

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
The heading's trailing plus imports a root or creates an album. Flutter 3.47.8's
`ReorderableListView.builder`, `ReorderableDragStartListener` and `onReorderItem` provide lazy rows,
drag geometry, scrolling and reorder semantics. Only the imported root row starts a folder drag;
its expanded descendants remain ordinary navigation. The presentation order owner serializes saves,
retains current item identities through metadata updates, appends new items and restores the prior
order on failure through the shared feedback surface. It does not reorder gallery images or mutate
physical folders.

Use the existing Flutter Material 3 design system. The installed Flutter 3.44.9 SDK provides
`NavigationRail` with destination selection, labels, focus, semantics and a bottom trailing action;
`Visibility.maintainState`, `TickerMode` and `ExcludeFocus` retain inactive workspace state while
retiring input and animation. Existing source navigation and resize controls remain reusable.
The product-specific responsibility is destination lifetime, folder-panel preference and composing
the existing gallery with a dedicated duplicate workspace. No new UI package is needed.

The secondary-sidebar toggle lives in the function rail above Settings while Library or Albums is active,
outside the collapsible sidebar.
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

Compose its 180-millisecond transition from the pinned SDK's `AnimationController`, `AnimatedBuilder`,
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
from the displayed rectangle, retire item records on unmount, and clear motion on query changes,
direct scrolling or reduced motion. The existing geometry owner supplies its pending scroll
correction so the first animated frame accounts for the sliver's later layout correction.
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
The group-list heading/count block, centered category switch and right-aligned analysis actions
share one row. Equal Expanded regions on both sides keep the switch at the content-area center.
LayoutBuilder uses a text-scaled compact-width boundary to stack these blocks when necessary,
retaining the switch's centered alignment. The count line reserves space during loading.
The existing Material SegmentedButton retains selection, focus and semantics. Material
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
