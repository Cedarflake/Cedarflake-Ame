# Material desktop workbench

The [roadmap](../roadmap.md#material-desktop-workbench) owns execution status and order.
This plan implements the accepted wide-screen Gallery, Albums, Duplicates and image-viewer layouts.
The [presentation contract](../product/gallery-ui.md) and
[navigation decision](../architecture/0029-functional-workbench-navigation.md#material-surface-composition)
own the resulting behavior and component boundaries.

## Outcome and boundaries

Use three distinguishable tonal surfaces for primary navigation, contextual chrome and content.
Retain the system accent, light/dark appearance, thin Material Symbols, existing window controls,
keyboard behavior, data scopes and operation semantics. Secondary sidebars use contextual rows and
small section labels without a redundant page heading. Keep search options at the trailing end of
the search field and the settings selection circular.
Place compact `添加文件夹` and `新建相册` plus actions beside their section headings,
using the existing picker and album editor. Library and Favorites begin their respective sidebars.
Use the global hamburger for sidebar visibility, keep the plain mark beside `Ame`, and align search
with the configured expanded-sidebar boundary. Manual/smart album sections collapse independently
while retaining scope and selection. Primary-destination changes animate the content boundary
without per-frame image reflow. Retain the filmstrip appearance while isolating its motion from
unchanged image painting.

Apply Material canonical-layout guidance to the existing composition: the gallery is an adaptive
feed, contextual sidebars select its scope, and the viewer filmstrip supports the current image.
Refine the shared visual hierarchy with the system accent's tonal-spot palette,
quieter secondary-container selection and regular-weight workspace headings.
Keep the three surface layers and established control geometry. Validate the palette in both
appearances and retain the existing compact layout and text-scaling behavior.

Duplicates moves its exact/similar/ignored choices into its contextual sidebar. Degree, selection
and cleanup remain with the results. Group cards emphasize the existing representative preview;
member rows, comparison, preferred-file decisions, ignored groups and cleanup use the existing
controllers and bounded reads. Do not fetch all members just to decorate a group card.
Use compact group cards. Selecting their image retains the preview and adds a circular check and
thin card outline; opening the details still enters the group.

The viewer retains its current top and bottom actions, zoom, rotation, copy and source lifecycle.
Its image canvas has straight edges. The thumbnail strip overlays the canvas with rounded corners
and a subtle shadow; toggling it cannot resize or reposition the main image. Menus and notifications
keep their existing placement, feedback ownership and timing.

No framework, package, database, generated bridge, source file or analysis-policy change is required.
The existing retained widget trees, virtualized image demand and shared motion drivers remain the
performance boundary. Prototype sample content and browser-only behavior are not production inputs.

## Implementation order

| Slice | Owned result | Verification boundary |
| --- | --- | --- |
| MW1 | Shared tonal surfaces, rounded workbench content, global search and navigation; Gallery/Albums composition | Search mode/scope and keyboard use; sidebar resize/toggle; source/album switching; retained selection and position |
| MW2 | Duplicates contextual navigation and revised group-card composition | Exact/similar/ignored switching, degree, paging, selection, members, preferred decisions, comparison and return |
| MW3 | Shared viewer canvas/chrome and elevated overlay filmstrip | Open/return from Gallery and Duplicates, filmstrip show/hide, zoom/rotation, copy/menu, source/loading feedback and reduced motion |
| MW4 | Integrated desktop qualification and delivery | Supported compact/normal/maximized sizes, enlarged text, light/dark appearance, real interaction and affected hosted gates |

Keep each independently reviewable result with its focused regressions and owning documentation.
Commit and push a validated slice before starting the next independent slice. All slices stay on
the same implementation branch; this plan does not authorize merging or a release.

## Evidence and rollback

Reuse existing workbench, search, sidebar, duplicate-review, viewer and filmstrip regressions.
Add coverage only for changed navigation/geometry boundaries. CI owns the routine automated suites;
local diagnostic runs answer a specific unresolved question. Verify actual interaction in a
Windows client with the existing isolated `build/test` corpus, preserving originals and catalogs.
Record current-head results in the existing Material UI acceptance record. UI composition can be
reverted independently without changing durable data or admitted operation authority.
