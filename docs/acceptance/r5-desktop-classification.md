# R5 desktop classification integration

This record covers smart-album browsing, configuration and guarded review within R5-06. The
[roadmap](../roadmap.md#current-execution-queue) owns stage disposition; the
[desktop contract](../product/gallery-ui.md#smart-album-workflow) owns interaction requirements.

## Connected boundaries

Smart albums appear after manual albums in the existing Albums sidebar. Their compact heading is
plain text, and its trailing arrow expands/collapses category rows without navigating. Category
rows read the active scheme; an unconfigured library has no sample-category fallback. Category
rows enter the shared gallery with exact duplicates folded. The existing filter
menu selects all, pending or confirmed results and retains structured metadata conditions. Category
names are retained during asynchronous refresh and ordinary primary-workspace navigation.

Classification summaries are read in the same SQLite transaction as the page and timeline. Their
query/revision identity prevents a previous scope's numbers from appearing after navigation.
Manual-album destinations, Favorite controls and organization-mode authority remain independent.

The gallery More menu and Settings share the setup and optional category editor. First use requests
model download and enablement without directory selection or category authoring. Category names and
descriptions retain their identity; saving does not implicitly enable analysis. Managed installation
and configuration saving join the existing worker-retirement boundary. Explicit enable, pause,
continue, stop and disable use the existing native configuration and dispatch contracts. Feedback
uses the shared transient bottom template.

Contextual selection connects confirmation and an anchored category menu to a native frozen group
selection. All-matching selections retain exclusions on disk and process bounded windows. Confirmation
accepts each eligible group's displayed prediction; an explicit correction retains the existing
field-authority rules when model evidence changes. Favorite and manual-album writes do not invalidate
classification authority. The viewer uses the same selection and validates its retained source version.
Its label and command eligibility come from the same prepared observation.

Committed classification operations expose operation-specific undo through the existing bottom and
viewer-top feedback hosts. A failed gallery refresh retains the committed receipt and undo while
offering refresh separately. Classification success does not enter notification history or R4's
chronological organization history.

## Focused checks

### All-results navigation

The category rows previously provided no route into the existing unscoped smart-album query.
The shared More menu now offers `全部分类结果` from a category or a review-filtered scope. It clears
those two predicates through the existing query transition, retaining other conditions and sorting.
The sidebar heading stays passive; returning through a category row restores that category's title
and selected state. No persistence, bridge, model or source-media behavior changes.

The three smart-album widget cases and three existing classification-progress widget cases pass.
They cover an empty category returning to an unclassified image, clearing category/review scope,
coherent heading/sidebar selection, unchanged empty headings, and explicit saved continuation.
Scoped analysis and owned formatting pass. The isolated Profile client subsequently verifies the
empty-category route into all results, as recorded below.

The affected menu contains 232 production lines and no inline tests; the screen composition contains
1,930 production lines, with three callback-routing lines added. The dedicated smart-album and
progress-widget suites contain 339 and 225 lines respectively. Navigation lifetime remains in the
existing query-transition owner.

### Isolated first use and review continuation — 2026-11-01

The Profile client uses the production entrypoint with in-memory preferences and the existing
native isolated-storage switch. Its catalog, previews and managed model cache stay under
`build/test/derived/r5-smart-album-client/profile`. The existing three-file metadata fixture is
imported through the Windows folder picker. Verified retained model artifacts are reused; this
check does not measure a fresh network download.

First-use Settings offers download/enablement without artifact-directory selection or category
authoring. Successful installation publishes automatic dispatch and two real predictions for the
three files, with the duplicate pair sharing one content result. The empty photo category reaches
all results through More, displaying two folded groups and coherent counts.

The viewer confirms and corrects a category while retaining the current image. Keyboard navigation
opens the duplicate group. Deferring that group displays the shared top notice; its Undo restores
the prior review state and reports success. Repeating deferral produces one pending group in the
gallery. After a first-image correction and full process restart, ordinary startup does not open a
viewer. The corrected category remains present. Explicit continuation from another category restores
the saved all-results scope and opens the saved first image with its human category intact.

The preview menu originally measured icon-bearing commands as text-only choices and clipped
`确认分类` and `稍后确认`. Its composition now includes the command icon width, while batch category
choices retain their narrower selection-indicator measurement. The real client displays the full
confirmation label. A rendered-text regression fails before the change and passes afterward;
the two control cases and three progress-widget cases pass, as does scoped fatal-info analysis.
The affected control has 294 production lines, no inline tests, and 143 dedicated test lines.
Failed-analysis retry is verified below; batch review and the remaining integrated desktop cases
retain their own evidence requirements.

The hosted Flutter run for `83f7f69f` also found that a settings typography assertion matched both
the page title and the new settings action. Commit `d60469a8` scopes that assertion to the title row;
all 15 tests in that settings suite pass locally. Its push-triggered hosted gate remains pending.

### Retained checks

- The rewritten compact sidebar passed both targeted navigation tests. In the rebuilt isolated
  Profile client, clicking the plain section heading leaves the selected category unchanged.
  Comic, meme, photo, screenshot and collapse icons render correctly after rebuilding the stale
  font-subset asset. Comparing constants in the current kernel with the packaged fonts found all
  127 font-family glyph references present; no icon-library or runtime rendering workaround was added.
- Generated bridges and the native Release library build completed after the snapshot extension.
- Scoped Dart analysis passed for the affected library/settings owners and new UI tests.
- The generated classification-review bridge and native Release library build pass. All five native
  batch regressions now pass in isolated temporary catalogs. They cover frozen predictions, whole-group
  confirmation, exclusions, human-choice races, cancellation, operation replay and retained viewer targets.
- Four focused application tests pass: bounded all-matching exclusions, cancellation during open,
  receipt-before-feedback publication and operation-specific undo, and retained undo after a failed
  gallery refresh. Two menu cases and two existing viewer-feedback cases also pass locally.
- Native batch regressions cover mixed predictions, exact-group folding, Favorite independence,
  atomic rejection of stale predictions/human choices, operation replay, cancellation and retained
  viewer versions. The UI interactions still require the rebuilt isolated client.
- Classification count identity, smart-category navigation/filter/title continuity and the existing
  manual-album title regression passed locally.
- The sidebar reuses the accepted sketch's compact section heading and collapse action. Earlier cases
  covered initial category availability, collapse without query changes, the existing gallery
  More entry and smart-album preview lifecycle; all eight cases passed locally.
- The earlier isolated Profile client exposed initial definitions and the collapse control before
  saving configuration. The sample-definition fallback is subsequently removed to match the
  application-managed category contract. The updated empty-state regression and existing configured
  category navigation/title regression both pass; scoped analysis reports no issues. This establishes
  the provider/widget boundary. The rebuilt isolated Profile client also opens Albums with the
  existing manual album, plain smart-album heading and collapse control, without any sample-category
  rows. No source images were imported or analyzed. The category-icon adjustment passed scoped
  analysis. This empty-state check does not establish automatic category discovery or first-use setup.
- Settings editing preserves category identity; saving keeps analysis disabled; first use prepares
  managed installation; cancellation does not enable analysis; failed saving retains edits and reuses
  the operation identity on retry. The current four widget cases and lifecycle checks are recorded in
  [managed setup verification](r5-runtime-admission.md#managed-setup-bridge-and-desktop-controls).
- Early settings-test attempts stalled during asynchronous teardown after their assertions. Closing
  the inactive lifecycle while the widget test still advances its clock resolves the fixture issue;
  product shutdown behavior was not changed.
- The native grouped-count regression passes across review filters, physical root/search scope and
  a whole-group correction. Five module-topology cases pass with the new classification selection
  domain owner explicitly admitted; alternate loading and visibility continue to be rejected.
- All 38 asynchronous bridge contracts and the method-boundary rejection fixtures pass. The ten
  new review contracts cover selection opening, reads, bounded collection, preparation, writes,
  undo and joined close. Parameterized opaque methods retain their exact lease and forward the
  declared named values; substitutions and duplicate lease forwarding are rejected.
- The previous `323c5ba2` hosted Flutter/static failures exposed missing smart-album branches in the
  screen title and viewer-test scope label. Both owning switches now handle that collection; the
  viewer lifecycle suite also includes it.
- Retained Windows Scan diagnostics for that commit report the same non-exhaustive title switch
  before the integration test starts. All four hosted native partitions passed for `323c5ba2`;
  pending desktop changes and new native batch regressions require their own commit's hosted results.

## Physical ownership

The empty-sidebar follow-up uses the folder list's static `ListTile` pattern for both manual and
smart albums: an existing neutral album icon and one short line, `还没有创建相册` or
`还没有分类结果`. The manual row requires a loaded zero album total; it does not describe an empty
page of a larger collection. Smart-album collapse also hides its empty row, and configured categories
replace it without a sample-definition fallback. The two updated smart-sidebar cases and two existing
album creation/undo cases pass, including the empty row's disappearance and return. The isolated
Profile build is updated. The user stopped Computer Use with Escape during the window update; the
final rows have not received a new real-window interaction check.

Line counts exclude generated bridges. UI tests are dedicated suites, not inline production code.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Gallery screen / viewport controller | 1917 / 1306 | 0 | Existing shared suites |
| Gallery header / album navigation | 537 / 245 | 0 | Existing shared suites |
| Settings editor / category editor / gallery menu | 285 / 114 / 131 | 0 | 198 |
| Smart navigation / filter / title owner | 158 / 86 / 61 | 0 | 221 |
| Classification configuration facade / settings providers | 126 / 20 | 0 | Existing lifecycle suite |
| Native gallery snapshot / classification counts | 315 / 39 | 0 | 74 |
| Dart query-count authority | 28 | 0 | 39 |
| Classification controls / action publication / selection lifetime | 229 / 232 / 85 | 0 | 136 / 161 |
| Native selection / preparation / observation codec | 230 / 112 / 162 | 0 | 203 |
| Shared gallery selection staging | 206 | 0 | Existing organization batch suites |
| Native selection session / bridge facade / port | 150 / 84 / 74 | 0 | Adapter and client suites |

The existing local-files owner has 2,629 lines before its inline test module (including retained
test-only helpers), 3,476 inline test-module lines and 540 dedicated topology-test lines. This change
adds only the exact classification-selection roster entry and its rejecting topology case there;
no source-access behavior or additional responsibility is introduced.

## Remaining connected workflow

Application-managed discovery and guarded category maintenance now join the desktop session
retirement/publication boundary, with pinned vocabulary and runtime resources in the build payload.
First-use setup supplies managed model installation without manual directory or category entry.
The [category evidence](r5-category-discovery.md#joined-desktop-maintenance-publication) records the
native/client lifecycle checks; the isolated first-use interaction is recorded above. The roadmap
owns the remaining context coverage and integrated quality acceptance.

Real-client acceptance of contextual review retains its R5-06 scope. Classification continuation
and single-image failed-analysis retry are connected through the workflows below. Integrated quality, scale and the paired
cloud-primary comparison remain in R5-07. These focused checks do not close the stage.

## Smart-album image context menus

The existing image context menu now prepares the clicked source version and offers applicable
confirmation/correction in smart-album scopes. Confirmed or otherwise ineligible items omit
confirmation. Correction opens the same anchored category-choice entries as selection and viewer
controls, with the shared dot marker, neutral icons and framework focus/navigation. No dialog or new
menu primitive is introduced; `showAmePopupMenu` retains the installed SDK's `showMenu` ownership.
The original Favorite/manual-album and information actions remain available.

Five new widget cases pass: target-bound confirmation with shared undo, confirmed-item correction,
late inspection after navigation, an already-open command after navigation, and failed inspection
with successful reopening. The two existing classification-control and five original photo-menu
cases also pass. Focused analysis reports no issues, owned formatting and whitespace checks pass.
Successful classification stays outside notification history; read failure uses the existing
notification controller and clears after a successful retry. Real-window interaction remains pending.

The photo-menu composition has 275 production lines, its classification scope/lifetime owner 85,
shared category entries 34 and existing classification controls 227. No inline tests are added;
the new dedicated menu fixture has 230 lines. There are no schema, dependency or media-access changes.

## Card classification annotations

The photo wall reads classification reasons in batches of at most 256 current source versions,
only in smart-album scopes. One SQLite snapshot checks the displayed revision and every requested
location; the shared effective-category projection preserves human precedence and retained display
membership. A replaced window retires the remaining requests and cannot publish late results.
Partial batches are not presented as complete annotations.

Uncertain, uncalibrated, deferred and conflicting items use the compact `待确认` label. Failed
analysis has a distinct `分析失败` label; ordinary suggestions remain unmarked. Hover descriptions
explain these states, and the viewer's existing category tooltip receives the corresponding review
reason. Human confirmation removes the attention label. Terminal failures, exhausted interrupted
work and explicit retry invalidate the catalog projection; automatic transient retries do not
publish a terminal failure. No schema or dependency changes, model execution or media reads are added.

`LibraryPhotoStatusLabel` shares the former album-conflict surface between both labels. The photo
tile lays them out without overlap and retains the existing selection and duplicate controls.
The installed Flutter `Tooltip` owns hover timing and semantic descriptions; this composition adds
no gesture recognizer or custom focus behavior. Clicking a status still opens the image.

The 17 affected projection checks, two failure/retry annotation checks, one interrupted-session
check and one retained-display command-boundary regression pass. Twenty Flutter checks pass,
including 500-item batching, retired-window work, failed
batch recovery, coexisting labels, hover/open behavior, quiet routine suggestions, original album
conflict feedback and card focus. Focused analysis reports no issues. The generated bridge passes
all 39 asynchronous contracts and the rejecting contract fixtures. The contract fixture's roster
was updated with the added annotation entrypoint. Real-window interaction remains pending under
the existing Computer Use restriction; focused checks do not establish that acceptance.

Production ownership is 130 lines for native annotations, 97 for the shared projection, 167/274 for
work completion/control, 158 for the native review facade, 71 for the Dart window reader, 44 for
the shared label, 26 for its classification binding and 441 for the existing photo tile. No inline
tests are added. Dedicated annotation tests contain 143/128 native and 143/117 Dart lines.

## Connected classification continuation

Single-image commands in the viewer and smart-album context menu now start a classification session
with the observed category, review filter, physical scope, search and ordering. Preparation is passive.
The existing frozen selection reaches the native review transaction through a single-group adapter,
so its field changes and continuation position commit together. Ordinary batch writes retain their
separate bounded command owner. No schema, dependency, model, media-read or source-write changes are
required.

Refreshing a retained image's controls preserves its review entry. Later commands use the latest
committed sequence without repeating the checkpoint; after-current placement affects resumption,
while the active session, exact group and field guards continue to authorize the displayed image.
Old sessions, positions, evidence and cancelled selections cannot publish a new decision. Context
menu retirement awaits a pending session opening and ends only that matching session.

`稍后确认` is available for unfinished, non-deferred images through the category and context menus.
`更多` offers `继续确认分类` after saved activity. Explicit continuation reloads the saved target,
uses the existing bounded gallery transition and opens that image. Replaced or disposed views retire
late reads; ordinary selections are not restored. Unavailable and exhausted scopes use shared bottom
feedback. R4 organization actions and its bookmark remain independent.

Seventeen native checks pass: nine frozen-selection/review checks, five existing review/recovery
checks, two bounded-continuation checks and one smart-album scope-normalization check. They cover grouped deferral, committed retries, repeated
retained-image correction, stale evidence/session rejection, cancellation, control refresh, restart
and R4 preservation. The initial two new fixture failures were missing display publication and a
configuration operation counted as a review write; both fixtures now establish the actual boundary.

Thirty-nine Flutter checks pass across classification sessions, explicit continuation, viewer and
context-menu controls, shared feedback and the existing R4 viewer controls. The affected session and
menu checks were repeated after connecting subsequent retained-image commands. Focused analysis
reports no issues. All 46 asynchronous bridge contracts and their rejection fixtures pass; generation
adds seven entrypoints and removes none. The shared reverse gallery-query mapper retains the exact
former R4 mapping body. An initial generator pass exposed a misplaced Dart import; the corrected
generation completes without the failed build step. The final targeted checks also preserve prepared
controls across ordinary viewer rebuilds and report failed continuation publication without opening
an image. Smart-album progress normalizes the retained duplicate preference to its actual folded
query while preserving the other filters and ordering.

The previous head's hosted Static job found an untyped empty category-list fallback in the smart-album
sidebar. The explicit typed fallback is separately committed in `1f39b270`; focused analysis passes
and its hosted gate is running. Real-window review remains pending under the existing Computer Use
restriction. Failed-analysis retry and integrated quality acceptance retain their roadmap scope.

The native batch facade has 168 production lines, its batch-command owner 84 and its single-review
adapter 40. Application progress/selection owners have 54/170 lines and their bridge facades 72/139.
The Dart review session and continuation navigation contain 109/51 lines; classification controls,
viewer binding, More menu and photo-menu owner contain 247/103/210/120. Existing photo-menu composition
is 289 lines. The 1,921-line gallery screen gains four net lines of dependency wiring and no new
workflow responsibility. No inline tests are added. Dedicated native batch-review/scope fixtures
contain 254/32 lines; Dart session/navigation/widget fixtures contain 161/147/221 lines.

## Single-image analysis retry

Failed images expose `重新分析` in the existing preview category and photo context menus. A typed,
asynchronous request revalidates the observed dispatch revision and exact source version before
requeuing only its failed source/content tasks. Duplicate copies share the content task. The request
preserves pause, current human decisions and both review histories; it changes no schema, dependency,
model installation or original media. Disabled/stopped analysis rejects the command. Already queued
or completed work returns unchanged rather than restarting successful analysis.

The existing bottom and viewer-top hosts display the same transient outcome. Success adds no
notification-history or undo entry. Refresh failure retains a separate refresh action, whose retry
never repeats the analysis mutation. Menu eligibility comes from the observed failure and analysis
mode. The original category, Favorite, album and continuation controls remain available.

Four focused native tests pass for scoped/repeated retry, shared duplicate work, stale source/settings,
human-choice preservation and pause/stop/disable. Twenty-two Flutter tests pass, covering both menus,
the shared top/bottom notice providers, double-click exclusion, failure recovery, refresh-only retry,
retired callbacks and existing classification/organization controls. The first widget run exposed
a fixture using Column for the top notice's Positioned child; the corrected fixture uses the actual
Stack host. Native compilation also required the existing checked unsigned-SQL reader. Focused Dart
analysis reports no issues; all 47 asynchronous bridge contracts, their rejecting fixtures and
matching generated hashes pass.

The native retry owner has 99 production lines with 17 application and 10 bridge lines. Dart retry
service/actions have 27/138 lines; existing classification controls, context scope and context menu
have 293/135/301. The 1,927-line gallery screen adds only six lines of notice routing. No inline tests
are added. Dedicated native retry tests contain 199 lines; Dart action/widget tests 142/143 and the
affected context-menu suite 309. The subsequent real-client check below covers these recovery
entrypoints and the missing native failure-state connection.

### Failure status in real menu preparation — 2026-11-01

The isolated client imports the retained 48-file measurement fixture alongside the three-file
metadata fixture. It publishes 35 content predictions and 15 terminal `classification_image_limit`
failures. Failed cards remain browseable. Pausing through More persists `paused` without changing
the saved human category or deferred review.

Opening a failed image exposes a missing native connection: card annotations include terminal work
failures, while direct item reads and frozen single-group preparation previously omitted them.
The shared annotation owner now supplies its existing configuration/source/content failure predicate
to both single-item paths in their current read transaction. Human decisions and compatible
predictions retain precedence; failure status grants no classification or retry mutation authority.
The retry command still revalidates dispatch and source versions independently.

Both focused native regressions fail before this repair and pass afterward. They compare the card,
direct-item and frozen-selection results for failed source work, shared duplicate content, retry
clearing the failure, successful publication and a human correction. The annotation, single-item and
batch-preparation owners contain 156/87/113 production lines, with no inline tests; their dedicated
work-annotation fixture contains 181 lines. Formatting and whitespace checks pass.

The rebuilt isolated Profile client retains manual pause across restart. Its preview category menu
now exposes `重新分析` for the failed image. Activating it displays the shared top notice
`已安排重试，继续分析后处理`; only one content task becomes queued, and the action disappears for
that queued image. The context menu on a second failed image gives the same result through the
shared bottom host. Read-only before/after snapshots show exactly one changed work row per command,
with identical dispatch, human classification and continuation records throughout both requests.

Explicit Continue consumes both queued tasks. Their unchanged oversized inputs produce the same
bounded `classification_image_limit` outcome, and both card labels return to `分析失败` without
losing browsing or human choices. All 51 original fixture file lengths and SHA-256 hashes remain
unchanged. These observations establish retry scheduling, pause persistence and truthful failure
publication; they do not raise the admitted image-size limit.
