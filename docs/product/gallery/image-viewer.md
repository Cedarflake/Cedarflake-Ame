# Single-image loading and zoom

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Single-image loading and zoom

Original-image sampling follows the rendered physical-pixel scale, including viewport fitting,
zoom and display density: mipmapped filtering for reduction, bilinear at one-to-one pixels, and
bicubic for enlargement. Zoom geometry and source resolution remain independent of this choice.

While an original image is loading, its progress indicator stays at the center of the image
viewport. Zoom, pan and wheel input transform the image or retained preview, never the indicator.
Loading completion retires the indicator through the current source image's frame lifecycle.

Opening a photo moves and scales its existing image from the activated thumbnail to the preview.
The image preserves its aspect ratio while the thumbnail crop opens into the full image. Window
chrome and toolbars keep their positions. Shared Standard spatial motion drives the image, which
remains opaque through its return to the card; backgrounds and controls fade independently.
Its gallery stays mounted at the existing scroll position.
Card badges and selection overlays fade out during entry, stay hidden while the image returns,
and fade in after it reaches the card. They retain their existing positions and reduced-motion behavior.
Closing immediately retires viewer input, accessibility focus and
source-read authority; at most one non-interactive closing surface remains for the visual exit.
Reopening replaces that surface, and further pointer input finishes entry immediately. Disabled
animations use an immediate transition. Original pixels reveal over the same image's small preview
in 100 ms after entry finishes, without resetting the zoom transform. A return freezes any active
quality blend; decoded source arrival cannot replace the travelling preview. The existing preview element stays mounted across
the original's first frame, including after cache eviction, and is released when the reveal finishes.
The viewer captures its activated thumbnail and tracks geometry suppliers only for mounted lazy
cards. Returning to a still-visible matching card reverses the image transition, including after
adjacent navigation. A removed/offscreen card or rotated viewing direction uses the shared fade
instead. Missing launch geometry also falls back to the fade.
The image clip follows the card's full bounds and corner radius in screen coordinates, including
under viewer zoom; the visible viewport clips partial cards throughout the return.
These transitions reuse the current image element and source lifecycle, without another decode,
image buffer, layout interpolation or queue of outgoing full-resolution images. Duplicate review
uses the same image transition from its member thumbnail; comparison retains the surface fade.
Returning retains the underlying review list and its position.

An open gallery viewer retains its displayed source version after a confirmed replacement and shows
`图片已更新，重新加载以查看最新内容。` with `重新加载`. Reload adopts the latest confirmed catalog
version through the existing guarded source reader. A confirmed removal keeps the viewer open and
shows `这张图片已不在图库中。` with `返回图库`; its obsolete ordinal and adjacent navigation are
unavailable. Same-version renames update the name without a content warning. Query revisions,
selection changes and disposal retire delayed lookup results; a failed lookup cannot claim absence.
Gallery-page rebuilds cannot silently replace the retained viewer version before the user reloads.
These notices share the existing viewer fallback notice, neutral notification surface, rounded
corners and action styling. They float within the image viewport without moving its contents. A
confirmed source notice takes precedence over retry feedback for the old source version.

Favorite and organization results reuse this same top-of-viewport notice component, including its
geometry, colors and action styling. Show undo or retry when applicable, without an additional close
button. Successful transient results use the shared notification lifetime. Favorite, unfavorite and
their successful undo provide transient feedback only; they do not enter notification history or
increase unread state. Keep failed saves actionable. Source changes and original-image failures
retain priority. An organization result cannot replace their recovery action or add a second surface
at the bottom of the viewer. Detailed internal errors remain in diagnostics.

The zoom percentage keeps its source-pixel reference: actual size (`1:1`) is 100%, enlargement by
one quarter is 125%, and reduction by half is 50%. The default opening preference is Automatic:
images exceeding the available width or height shrink to fit the viewport, while fitting images
stay at 100% actual size. This mode never enlarges a small image to fill the window. Automatic,
fit-to-window and actual-size preferences persist across restart; an existing saved choice is
preserved. One toolbar button switches between fit and actual size; its icon and tooltip describe
the next action. Automatic remains the opening preference in Settings, so manual zoom and the
toggle affect only the current image. Explicit fit preserves
the whole image within the available width and height; its percentage follows the resulting pixel
scale rather than being relabeled as 100%. Resizing the image area preserves actual size or a manual
pixel scale, while automatic and fit continue to follow the available area.

The bottom toolbar groups the thumbnail-strip toggle, information button and more-actions menu
first on the left, followed by compact file type/dimensions/size and the current image count.
The count includes the image unit and is separated from file metadata by a thin vertical divider.
The bottom More menu opens upward from its fixed lower edge above the button and closes toward
that same edge. Its placement and animation leave the triggering button unobscured.
The top bar retains the filename, back action and window controls. Favorite, album and organization
actions form one icon-only group on the right, separated from the window controls by a short vertical
divider. The current image's classification appears in both ordinary viewing and organization mode.
The flexible filename region remains draggable, and long filenames elide without moving these
actions. Action buttons remain outside drag regions. Hover tooltips and accessible names describe
each action and its current state;
defer and ignore use labeled entries with icons inside the organization More menu. Album-conflict
actions also use icons. Full screen keeps these actions at the upper right while hiding the filename
and window controls, preserving any in-flight action across the placement change. Both sides of the
bottom toolbar use the same icon size and text style.
Clockwise rotation, zoom mode, percentage, zoom slider and full-screen
action are on the right; full screen is the last action. The strip starts hidden and remembers its
visibility. It uses bounded catalog paging and lazily creates visible thumbnails; selecting one opens
that image in the current query order. Its ordinary mouse wheel moves horizontally through the native
scroll position, alongside native horizontal-wheel, touchpad and scrollbar input.

Rotation changes the current viewing direction by 90 degrees without changing the original file,
EXIF, preview cache or catalog dimensions. It reuses the existing image interaction and sampling:
fit and automatic modes follow the oriented image bounds; actual size and manual pixel scale are
preserved. Loading and error notices remain upright. Navigating to another image or closing the
viewer resets the viewing rotation.
Small triangle buttons at the scrollbar ends move the strip by one thumbnail without changing
the selected image. Each disables at its corresponding end. They use Material `IconButton`;
the installed `ScrollbarThemeData.mainAxisMargin` reserves their space outside the native track.
New thumbnail pixels retain the placeholder until decoding finishes; a larger preview of the same
source retains the already decoded image, and scrolling skips the first-frame fade.

The strip is first created when expanded. Collapsing retains its lazy visible elements until the
viewer closes, while removing its preview demand, focus and animation activity. Every expansion
centers the current image within the natural scroll bounds while reusing compatible previews.
Near the start or end, the strip aligns to that boundary without adding empty space to force
centering. Selection changes while the strip remains open only scroll
when needed to bring the thumbnail fully into view. Manual strip scrolling leaves the main image
unchanged.

Paging preserves the visible thumbnails before the next frame is painted and keeps a held scrollbar
drag active. Layout correction must not restart the gesture or briefly expose a different page.
Loading occupies a separate thumbnail-sized card at the requested edge: left for earlier images,
right for later images. It does not overlay an image. An attempt that leaves the same unfinished
boundary stops automatic requests and exposes retry there. Background publication retains the
independently browsed strip position without changing the main image.

Completed preview metadata and verified sizes each keep at most 256 recently used entries beyond
their immediate request lifetime. They reuse compatible results when returning to an image; source
generation changes and root/cache invalidation retire incompatible evidence. Decoded pixels remain
in Flutter's existing bounded image cache, with only the current lazy strip window retained while
collapsed. Closing the viewer releases the strip elements and requests; disk previews retain their
normal storage-budget lifetime. A missing or damaged displayed preview admits one automatic repair
for that source while visible, without loading the rest of the library. Failure leaves the image
selectable, and a late result cannot revive a hidden or replaced thumbnail's repair.

This retention composes the installed Flutter `Visibility.maintainState` and `TickerMode` behavior;
it does not introduce another pixel cache or persist strip scroll offsets across application restarts.

The bottom more-actions menu opens above its button with a small gap, keeping the toolbar visible.

F11 or the rightmost button toggles native full screen. Escape exits full screen first, otherwise
returns to the gallery. Full screen preserves normal window placement and maximized state and is
not restored on startup. Images change through manual navigation; there is no automatic playback.

The controls compose Material `IconButton`, `Slider`, `ListView.builder` and `Scrollbar`, whose
toggle, keyboard, lazy-item and scroll-position APIs are available in the pinned Flutter SDK.
The product-specific strip adds selection, catalog-page continuity and bounded preview demand;
scroll physics, focus and input dispatch remain framework-owned.
