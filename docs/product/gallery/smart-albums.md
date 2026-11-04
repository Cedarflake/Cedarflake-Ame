# Smart-album workflow

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Smart album workflow

R5 uses the existing desktop shell, Material components, photo wall, viewer, and feedback templates.
The visible section name is `智能相册`. Keep the primary `图库`, `相册`, `去重` order and place the
section below manual albums in Albums' secondary sidebar. Use the same compact section-heading
alignment as manual albums, with a trailing expand/collapse arrow. The heading is plain text with
no click target, hover treatment or navigation action; only the arrow expands or collapses the list.
Category rows open the corresponding derived results.
Category rows show application-managed categories with current results, using one coherent bounded
presence snapshot with the gallery's membership rules. An empty category remains configured but has
no sidebar row; an already-open category retains its title and browsing scope. First use relies on
the existing setup and empty-state surfaces without a category-definition form. The Settings canvas has no smart-album
section or status-only dialog. Missing-model setup uses a download icon with the `下载模型` tooltip
in the smart-album empty row. Keep the adjacent empty-state text on one line; the confirmation states
the download size and starts the existing installation feedback.
Restored categories without a local model retain the same download action below their rows.
Installed models remove that action and add no status explanation. The sidebar heading's trailing
action remains collapse. Smart albums reuse the ordinary gallery More menu, with no settings,
pause, continue, stop or disable-analysis actions. There are no category creation, editing, removal,
or comparison-description forms.
Model/runtime directory selection stays outside normal setup. Existing definitions remain preserved
and retain their authority independently of the application's curated topic catalog.
When expanded without categories, use the same static icon-and-text empty row as the Library's
folder list, with `还没有分类结果`. Hide it when collapsed or when category rows become available;
keep loading and failure separate. Do not substitute evaluation categories or add a setup paragraph
to this row.
Model preparation uses one task in the existing top-right notification history. Its bottom surface
can be hidden without cancelling the operation; subsequent progress updates keep it hidden. The
notification details remain live and retain the applicable cancellation or retry action. Downloading
shows a progress bar, percentage and transferred/total bytes; verification has its own phase label.
Completion updates the same history entry. The sidebar does not provide a second progress entrypoint.
Favorites remains independent above the manual albums. This adds no primary workspace, dashboard,
mobile layout, or competing gallery.

### Browsing and result state

Selecting a category immediately displays its known name and result count in the existing header.
Retain the title during asynchronous loading, using the same
ownership rule as manual album names. Keep Select, Sort, Filter, Layout, and More in their existing
upper-right positions. Filter uses the same four choices as Library; no pending/confirmed classification
filters, counts, or badges are shown.

Per-image analysis failures recover silently under the failure-specific lifecycle. Cards and
previews show no failure label, icon, tooltip, or notification; image context menus contain no
analysis command. Album conflicts retain their independent label. Ordinary suggestions and legacy
review states add no classification badge, confidence percentage, or repeated model information.
Installed analysis runs automatically. Legacy paused, stopped or disabled setup transitions to
automatic scheduling at desktop startup; missing models retain explicit download setup.
Runtime startup or execution failure uses the existing bottom and viewer-top error feedback;
successful recovery resolves that notification without producing per-image retry notices.

Suggestions are usable immediately. Machine-only disagreement uses the current analysis and bounded
automatic re-screening; remaining uncertainty retains the best usable suggestion. The ordinary Library
keeps images with missing/failed results and unresolved legacy conflicts reachable without a guessed
category. Smart albums have no separate all-results
entry. The heading remains noninteractive; category rows provide navigation.

Use existing loading, empty, unavailable, retry, and task-feedback surfaces for setup and analysis.
Short waits do not insert transient wording or shift controls. Background results preserve the
current image and viewport anchor; a complete replacement query publishes coherently.
Smart-album results always fold confirmed exact duplicates. Sort, Layout, the right scroll rail,
selection, and preview use the shared bounded gallery behavior. Date sorts retain year/month
annotations and name sort retains letter ranges. Organization-only `新加入优先` and R4 queue controls
remain in the organizing Library.

### Selection and single-image actions

Selection uses the existing Favorite, manual-album, cancellation, and applicable More actions.
There are no single-image, contextual, or batch confirmation/correction commands. Smart-album views
do not expose classification deferral, review continuation, or classification undo.

The viewer shows an available category as read-only text with its existing category icon beside
Favorite and Album. It has no category-selection menu, drop-down arrow, or manual analysis retry.
A failed category read offers `重新读取分类`; stale reads
cannot replace the state of a newly opened image. Supported metadata stays in the viewer information
area without another permanent panel.

Manual-album actions reuse the bottom tray, its deselectable single destination, and state-aware
add/move/remove behavior. Smart albums and Favorites never become tray destinations. Classification
views omit R4 ignore/defer actions. Manual-album assignments update R4 progress independently of model
results. The organizing Library retains its mode, selection, and position when visiting Albums.

### Feedback and compatibility

Analysis errors reuse the [feedback templates](feedback.md#feedback-templates), including aligned
actions, the existing bottom host, and the preview top notice. No classification confirmation,
correction, or undo notices are created. R4's chronological `撤销操作` / `重做操作` controls remain
exclusive to organization mode.

Existing category decisions, review bookmarks, and operation history remain readable/restorable
under the compatibility contract, without controls to create new manual classification decisions
or resume those bookmarks. Opening a picture or menu creates no manual classification activity.

The accepted sketch retains section placement, the shared canvas, the top viewer area, and reuse of
the album tray. Manual classification actions were withdrawn on 2026-11-01. Sample media/counts and
substitute icons are not production assets or requirements; use the existing icon library and controls.
