# Unified gallery UI contract

Status: accepted presentation contract under [ADR 0009](../architecture/0009-accepted-unified-gallery-ui-contract.md)
and [ADR 0029](../architecture/0029-functional-workbench-navigation.md).

The [roadmap](../roadmap.md) owns delivery order and current availability. Future controls described
here remain absent until their owning use cases are implemented. This document preserves the
accepted user workflow; it does not supersede technical or accessibility decisions in the ADRs.

## Scope and authority

The current UI source of truth is the user-confirmed Microsoft Photos-like structure recorded in
this document. Flutter Material 3 supplies components, tokens, focus behavior, and accessibility; it
does not redefine the information architecture.

The UX reference application is **Microsoft Photos Legacy**, as clarified on 2026-10-10. Compare
new interaction decisions with that application and the accepted behavior below. Observations of
the newer Microsoft Photos application retain their recorded version and evidence scope; they do
not establish Photos Legacy behavior or supersede this contract.

The [component implementation requirements](#component-implementation) own reuse order, capability
evidence and framework interaction boundaries for every affected surface.

On 2026-08-07 the user accepted this structure after interactive review. ADR 0009 records the
current authority and fully supersedes ADR 0003. Later work may refine spacing, responsive behavior,
component details, and wording, but it must not silently reverse these accepted
information-architecture choices:

- one reusable gallery for folder, timeline, category and search scopes;
- a permanent function rail selects `图库`, `相册`, and `去重`, in that order;
- the Library workspace owns the collapsible source/folder sidebar and Add folder;
- the Albums workspace owns its flat album list and shares the gallery and viewer;
- Settings remains pinned to the bottom of the function rail;
- the gallery filter changes duplicate display; the Duplicates workspace owns group review;
- temporary action-specific import progress rather than permanent task navigation;
- a plain-language settings canvas rather than a dialog or engineering dashboard;
- Simplified Chinese as the initial application language.

ADR 0003 remains historical evidence only. Its standalone duplicate action, task activity, provider
grouping, and other superseded surfaces are not current alternatives.

Simplified Chinese (`zh-CN`) is the only initial application language. All user-visible titles,
actions, menus, tooltips, accessible labels, progress text, empty states, confirmations, and error
explanations use concise Simplified Chinese. File names, paths, metadata values, and quoted operating-
system details retain their original content. Internal code identifiers and stable error codes remain
English and must be translated into understandable Chinese at the presentation boundary rather than
leaking raw implementation messages.

R2 does not add a language selector or a complete localization runtime. User-facing copy is kept in
a presentation-owned string catalog instead of being scattered through widgets, so formal i18n can
be introduced later without rewriting the UI structure.

The completed R2a prototype remains acceptance evidence, not a second application entry point.
Unavailable controls must not ship as dead production actions, and a fixture or screenshot is not
feature-completion evidence.

## Design principles and rationale

These principles explain the accepted surfaces below and guide new choices within their scope.
Keep the reason and applicable exception with a changed decision. New capabilities or reversals of
accepted navigation require a product decision; routine spacing, component composition and responsive
adjustments follow the established design without another approval step.

### Design as a desktop application

Ame is a desktop workbench. Start from desktop interaction conventions and its existing application
surfaces, not from webpage design habits. Window management, navigation, action placement, mouse and
keyboard input, scrolling and image manipulation must form one consistent desktop experience.
Reuse established window chrome, toolbars and interaction owners before introducing another pattern.
Do not import website navigation, mobile-style stacking or viewport breakpoints merely because they
are customary in web development. An alternate layout needs an actual desktop use case within the
[supported window constraints](../../lib/app/window/ame_window_placement.dart), not a hypothetical
phone-sized viewport or a test created to justify it. Preserve desktop DPI scaling, enlarged text,
keyboard accessibility and normal/maximized window behavior within those constraints.

| Principle | Reason and application | Adaptation boundary |
| --- | --- | --- |
| Give each visible element a useful role | Photos, names and paths are the work. A title identifies context, supporting text adds needed information, and controls offer distinct actions. Omit duplicated explanations, decorative icons beside sufficient previews, and buttons that repeat the same row action. | Preserve actionable errors, exceptional availability, accessible names and discoverability. A familiar auxiliary icon can save space; an unfamiliar action may need visible wording. |
| Preserve spatial relationships | Stable controls and retained image identity let people keep their place. The title and photo wall follow the sidebar's leading boundary; window-edge controls stay anchored. Counts reserve space, and data updates preserve the current item and browsing position. | Window constraints and enlarged text may require intentional reflow. Background updates must respect interaction and publication ownership rather than hiding jumps with animation. |
| Choose density for the information | Group cards support visual comparison; member rows expose long names and paths. Height follows actual text, preview and target sizes. Shared alignment and spacing make neighboring views predictable. | Cards, rows and text/icon actions are contextual choices, not universal formats. Check wrapping and reachability at compact widths and enlarged text; keep accessible targets. |
| Match feedback to the affected action | Saving a preference updates that decision while the reviewed content remains usable. Initial loads, continuation reads, explicit refresh and failures retain their appropriate feedback. Counts and wording describe the actual scope and state. | A local update must still expose in-flight admission and failure. Retained content cannot imply freshness or success that the application has not established. |
| Use motion to explain continuity | Sidebar, window and viewer transitions show where content moves and preserve the user's visual reference. Scrolling shares the established inertia behavior across surfaces. | Retarget interrupted motion from what is displayed, respect reduced motion and preserve direct input. Animation must not delay actions, mask state defects or require unbounded layout/preview work. |
| Share behavior with appearance | Common loading, empty, scrolling, hover, focus and preview owners keep the same operation recognizable across features. Repair an invariant at its owner so adjacent callers inherit the correction. | Reuse cohesive primitives and composition. Do not force different tasks into one flag-heavy component or change established behavior merely to make shapes match. |

For a changed workflow, establish its entry, primary action, resulting state and return path before
expanding its implementation. Compare it with the adjacent accepted surface and run the affected
interaction while the change is still small. The
[UI verification map](../acceptance/quality/ui-verification.md#ui-workflow-verification) links these decisions
to shared owners, existing regressions and client checks. It is the discovery point for implementation
and evidence; this product contract does not duplicate that inventory or record test results.

## Component implementation

- Prefer installed framework/design-system components, shared repository components, mature
  packages, then minimal custom code. The [dependency policy](../../AGENTS.md#8-dependency-and-open-source-policy) also applies to UI packages; appearance cannot
  justify stale, low-adoption, poorly documented or difficult-to-replace dependencies.
- Before a new/substantially redesigned Flutter control, inspect `https://m3.material.io/components`
  and the pinned SDK API/implementation. Record the official component, verified installed
  capability, evaluated alternatives and product gap in its decision/evidence. Memory, visual
  similarity or design availability does not establish SDK support.
- Before custom controls, record evaluated components and missing behavior. Compose around framework
  scrolling, selection, menus, dialogs, focus, input and accessibility;
  do not replace those primitives. Reuse tokens, components and state mechanisms instead of isolated
  visual constants or copied behavior. Organize behavior, structure, then presentation.
- Use virtualization/lazy slivers, bounded thumbnail decoding/caches and stable item identity/scroll
  position across incremental updates. Keep business/persistence state out of views; never send
  full-resolution images or unbounded result sets across the desktop bridge.
- Represent loading, empty, partial, cancelled, failed and stale states explicitly. Meet keyboard,
  focus, contrast, text scaling and screen-reader accessibility expectations.
- Apply the [content and action rules](#content-and-action-wording) to
  affected text and adjacent controls. Copy-only work preserves layout, navigation and behavior;
  authorized interaction changes must implement and verify the wording's promised action/result.

## Content and action wording

- Before editing, verify where the text appears, what triggers it, its adjacent titles and buttons,
  and the states before and after the action. When a change affects interpretation or presentation,
  check it in the actual interface for meaning, wrapping, truncation, and its relationship to the
  available actions.
- Titles describe what is happening; body text adds information needed in that situation; buttons
  name the action they perform. Include instructions in body text when the context needs them.
  An already clear button does not need a repeated instruction pointing to it, and a description
  can be omitted when there is no useful information to add.
- Organize copy by interface location and usage scenario. Document the affected locations of shared
  text. When the same words serve different purposes in different places, separate definitions may
  be appropriate to avoid changing another scenario unintentionally. Comments explain purpose and
  state conditions so maintainers can locate the interface and understand when the text appears.
- Preserve user-approved terms and wording, and edit existing text within the current authorization.
  Copy-only work preserves layout, navigation, and behavior. When the user explicitly requests
  interaction changes, implement the corresponding behavior and verify that the wording matches
  the actual action and result.

## Read by surface

The shared rules above apply to every UI change. Read only the affected surface contracts below,
together with the verification map. Each linked contract retains its accepted behavior, rationale
and exceptions; stage availability still comes from the roadmap.

| Task or section | Read |
| --- | --- |
| <a id="workbench-blueprint"></a> <a id="global-shell"></a> <a id="left-sidebar"></a>Workbench, shell and navigation | [Workbench, shell and navigation](gallery/workbench-and-navigation.md) |
| <a id="unified-gallery-header"></a> <a id="sort-behavior"></a> <a id="filter-and-exact-duplicate-behavior"></a> <a id="layout-behavior"></a>Gallery header, sorting, filters and layout | [Gallery header, sorting, filters and layout](gallery/header-and-controls.md) |
| <a id="gallery-canvas-and-timeline"></a>Gallery canvas and timeline | [Gallery canvas and timeline](gallery/canvas-and-timeline.md) |
| <a id="single-image-loading-and-zoom"></a>Single-image loading and zoom | [Single-image loading and zoom](gallery/image-viewer.md) |
| <a id="temporary-task-and-notification-feedback"></a> <a id="feedback-templates"></a> <a id="source-status-and-notification-ownership"></a>Task, notification and source-status feedback | [Task, notification and source-status feedback](gallery/feedback.md) |
| <a id="settings-page"></a>Settings page | [Settings page](gallery/settings.md) |
| <a id="review-workflow"></a>Organization workflow | [Organization workflow](gallery/organization.md) |
| <a id="smart-album-workflow"></a> <a id="browsing-and-result-state"></a> <a id="selection-and-single-image-actions"></a> <a id="feedback-and-compatibility"></a>Smart-album workflow | [Smart-album workflow](gallery/smart-albums.md) |

## Accepted UI regression states

Deterministic presentation fixtures and focused regressions retain coverage for these accepted
states without becoming production data paths:

- empty library;
- active import and import failure;
- populated unified gallery;
- source tree, unavailable source, and source overflow menu;
- selection and cancellation;
- filter menu with subfolder and exact-duplicate groups, merged representatives and location badges;
- function-rail navigation, retained collapsible source sidebar and duplicate review in Duplicates;
- layout menu with both shape modes and all three thumbnail sizes;
- search results and empty results;
- preview failure and retry;
- single-image view and return-to-scroll-position behavior;
- sort menu for all confirmed keys and both directions;
- time rail including unknown capture time;
- settings page with every initial row and its plain-language help text.

## Explicit current UI exclusions

Until each owning stage connects its complete workflow, the production UI does not show:

- classification, category filters, category pages, classification review, or model status;
- semantic similarity;
- people, faces, or identity features;
- editing tools;
- delete, move, copy, rename, recycle-bin, quarantine, or dry-run execution controls;
- a permanent task center, read-only-validation entry, cache diagnostics, or engineering limits.

Classification belongs to R5 and appears as `智能相册`, scoping the same gallery through a derived
effective-category query. Category predicates compose with existing search and metadata filters;
the visible collection entry remains in Albums. Category membership and definitions are application-
managed, with no manual classification controls. The [R5 requirements](r5-classification.md) own
that behavior and legacy-record compatibility.

Perceptual similarity is scoped to Duplicates under the
[workbench contract](gallery/workbench-and-navigation.md#workbench-blueprint); it does not add a
Library filter or similarity-driven folding.
