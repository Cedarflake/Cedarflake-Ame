# R5 desktop classification integration

Manual confirmation/correction, review filters, deferral, continuation, and classification undo were
withdrawn on 2026-11-01. Earlier checks below retain their historical scope; current behavior follows
[the revised R5 requirements](../product/r5-classification.md). Legacy records and archives remain compatible.
The all-results menu entry and optional definition editor are also retired; their earlier checks
below describe the former workflow.

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
Failed-analysis retry and batch review are verified below; integrated quality and constrained-device
measurements retain their R5-07 evidence requirements.

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
Profile build is updated. The initial real-window check was interrupted; the later isolated
first-use workflow above supplies the connected configuration and category-navigation evidence.

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

Real-client first use, contextual and batch review, continuation, and failed-analysis retry have the
observations recorded in this document. Integrated quality, scale and the paired cloud-primary
comparison remain in R5-07. These checks do not close the stage.

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
notification controller and clears after a successful retry. Real-window confirmation, correction
and undo are recorded below.

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
was updated with the added annotation entrypoint. The later isolated first-use, retry and batch
workflows verify the real card states and their committed transitions.

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
sidebar. The explicit typed fallback is committed in `1f39b270`; focused analysis and the later
`58d68688` hosted Static job pass. The isolated workflow above verifies real-window continuation;
failed-analysis retry is covered below. Integrated quality retains its roadmap scope.

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

### Batch review and manual organization independence — 2026-11-01

The isolated Profile client built from `58d68688` selects two current suggestions and adds them to
Favorites, then creates a manual album through the existing destination tray and assigns both
groups. The tray contains only the manual album. Both collections display the same two groups.

Batch confirmation retains both selected groups and removes their attention labels. The all-results
pending count decreases from 31 to 29. Batch correction to another category and the bottom notice's
Undo both commit successfully. Read-only snapshots show the original confirmed choices restored,
with identical album, Favorite, organization revision/history and classification bookmark records.

The confirmed filter displays three groups: the earlier individual correction and these two batch
confirmations. The pending filter displays 29 groups. A confirmed image's context menu omits
confirmation and retains correction; its correction and operation-specific Undo both complete.
Confirming another image through the pending view's context menu removes that card only after
commit and changes both displayed counts to 28. The same two Favorite and manual-album groups
remain present. Classification success and undo do not appear in the notification history.

A full process restart retains the manual album, both Favorites and the two confirmed categories.
Startup returns to ordinary browsing rather than opening a review viewer. Original-source safety
retains the 51-file length/hash comparison above; these additional actions only change virtual
decisions. Fixture predictions are used to exercise state transitions, not measure model accuracy.

An injected Ctrl+A did not activate selection in this desktop session. Temporary logging in the
isolated launcher records Control released before the A event, with `isControlPressed == false`
when A arrives. The application receives no held-Control combination in that attempt. A focused
framework regression selects the complete 79,013-item query with Ctrl+A, and all four selection
tests pass. In the native client, Down followed by Return selects all 50 folded groups through More.
Viewer arrow navigation is covered above. No production input remapping is introduced; the temporary
launcher logging is removed afterward.

## Manual-classification workflow removal — 2026-11-01

The change on top of `3255772c` removes confirmation, correction, deferral, review filters/counts,
continuation, and classification undo from gallery selection, context menus, and the preview. The
generated desktop bridge no longer exposes their mutation/session commands. Unreachable native
batch/session execution and its dedicated fixtures are removed. Read-only item/annotation queries,
automatic analysis, failed-image retry, and R4 actions remain connected. No schema migration, model
change, or durable-record purge accompanies the removal.

Thirty-four focused Flutter checks pass across annotations, labels, retry actions/widgets, shared
feedback, context menus, and smart-album navigation. They cover all six legacy review states, stale
query callbacks, failed reads, and disabled/stopped retry. The context-menu fixture retains settings
across remount after the first run exposed its auto-disposal; final Dart analysis reports no issues.
The bridge roster and rejecting fixtures pass for 31 asynchronous contracts with matching generated
Rust/Dart hashes. Final format and whitespace checks pass. Native compilation exposed obsolete
batch/session code and an unused trait import; those remaining references are removed without warning
suppression. Full native and hosted gates follow the submitted commit.

The existing isolated Profile client uses its in-memory preference entrypoint and the native
debug-assertion storage override. An initial normal-entry launch exposed no window and was retired;
verification then used that established isolation configuration. Actual interactions show:

- Category results display their image count without pending/confirmed controls or review badges.
  The filter menu contains metadata conditions, and More retains analysis controls and all-results.
- Previewing a saved category displays its name without a classification menu. Favorite and manual
  album actions remain available. Selecting all 50 groups offers no manual-classification action.
- A failed image retries from its context menu through the existing bottom notice. A second retries
  directly from the preview through the existing top notice. Both report queued work while paused,
  remove the retry action for that queued image, and create no classification undo entry.
- The existing manual album and Favorites each retain their two groups. Restart opens ordinary
  Library browsing with 51 files / 50 folded groups rather than resuming classification review.

Read-only comparisons before the update, after both retries, and after restart preserve row counts
and SHA-256 digests for all nine sampled durable tables: five saved classification decisions, their
operation/target/change history, the classification bookmark and review operations, albums,
organization records, and the organization bookmark. All 51 fixture file lengths and SHA-256 hashes
remain unchanged. This check exercises retained data and current retry/browsing behavior; historical
archive, model-quality, and resource results above retain their original scope.

Production line counts are 1,904 for the gallery facade, 532 for its header, 258 for the photo context
menu, 79 for preview classification, 58 for context classification, and 36/98 for the Dart read port
and adapter. Native read-only application/bridge facades contain 16/16 lines. No inline tests are
added. The affected native annotation fixture has 159 lines; dedicated retry-widget, context-menu,
label, and smart-album fixtures have 155/231/121/341 lines. Large existing owners only lose wiring;
the replacement read/retry presentation remains in its existing bounded owners.

## Managed setup and category navigation — 2026-11-01

The change on top of `1242c102` removes the separate `全部分类结果` menu entry and the optional
category-definition editor. The settings surface exposes managed enablement and installation,
without new/edit/remove category commands or a save-only action. Enablement passes the retained
definitions, or application-owned initial definitions when absent, through the existing guarded
configuration boundary. Legacy definitions, protection, native maintenance and archives are preserved.
Ordinary Library navigation keeps uncategorized images reachable; the existing preview retry remains.

Thirty-two focused checks pass: 21 classification lifecycle cases, eight setup cases and three
smart-album navigation cases. They cover first-use managed installation, cancellation/retirement,
enablement failure and idempotent retry, retained definitions, closing all four mode states without
a write, passive category headings and ordinary Library access from an empty category. The navigation
fixture initially retained the retired smart-album folding expectation; it now checks the restored
ordinary Library query, including its existing display preference. Owned formatting and whitespace
checks pass. Hosted gates follow the submitted head.

The isolated Profile client is rebuilt with its in-memory preferences entrypoint and the unchanged,
verified native isolation library. Its real paused settings dialog shows the current analysis state
and a Close action, with no category table or editor. The existing category gallery remains visible
behind it. Further automated input is left to the active local operator after two concurrent-input
guards. Read-only before/after checks preserve row counts and digests for 12 tables covering category
definitions/versions/protection, saved classification activity, manual albums and R4 organization.
No schema, model, runtime or original-media change accompanies this interface removal.

The gallery facade contains 1,898 production lines, the smart-album menu 130, settings 207 and the
configuration facade 117; no inline tests are added. Dedicated navigation/setup fixtures contain
324/319 lines. The existing 677-line lifecycle fixture is unchanged. The 114-line definition editor
is removed, and the gallery facade only loses callback wiring.

## Failure-aware automatic recovery — 2026-11-01

Commit `e5f45b56` replaces obsolete integration-test configuration calls while preserving enablement,
pause, restart and cache-reuse coverage. Its hosted Static job passes. The following repair changes
single-image recovery from a desktop command to the durable failure policy: transient failures wait
5 seconds and then 30 seconds, unavailable sources wait for availability or a verified peer, and
permanent/exhausted inputs wait for a changed source/content or model/rule compatibility. Compatible
new runs, ordinary refresh, pause/continue and restart do not reset the three-attempt budget.
Per-image failures are silent in cards and previews. The desktop removes their annotations, retry
actions and notices; retained native compatibility commands are not desktop recovery paths.

Session startup errors propagate through enablement instead of reporting success. Saved settings
remain readable, while runtime-wide interruption uses the shared bottom/preview-top feedback and
settings state. Successful reopening clears the runtime error before attaching signals, preserving
any new attachment failure. No schema, generated bridge, model, source-media or durable-decision
change accompanies this repair.

All 65 focused native classification-work tests pass. They cover retry deadlines/exhaustion, restart,
same-input admission, changed source/model recovery, verified peers, pause and unrelated runnable
work. The new failure persistence owner initially passed an unsigned value directly to SQLite;
the corrected implementation uses the existing checked integer conversion. Fifty-seven retained
focused Flutter checks pass: lifecycle (24), runtime notifications (2), source status (2), setup (9),
context menus (8), preview (11) and card status (1). Failure injection reaches the real lifecycle,
activity listener and shared feedback host. The card check preserves album-conflict opening without
requesting analysis status. Owned Dart analysis and formatting pass, as does whitespace validation.

The rebuilt isolated Profile client opens the existing 51-file / 50-group test library. Actual
ordinary-Library context menus and failed-image preview retain Favorite/manual-album operations
without `重新分析`; the 6,000 × 4,000 fixture opens normally. Before/after read-only comparisons
preserve all nine sampled durable-table digests and all 51 fixture-media hashes. The subsequent
silent-indicator removal passes the card/preview checks above. Its resumed client verification is
recorded below. The integrated implementation through `e801841b` passes all 15 applicable jobs in
[run 41331763486](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41331763486);
the three conditional signing jobs are skipped.

Production line counts are 1,892 for the gallery facade, 432 for the photo tile, 202 for its context
menu, 49 for preview classification, 72/145 for lifecycle/session ownership, and 150 for foreground
activity. The native domain failure policy owner has 142 lines; failure persistence/completion have
61/115. No inline tests are added. Dedicated lifecycle/notification/setup/context/preview/card
fixtures contain 744/103/346/147/141/59 lines; native work/automatic-recovery fixtures contain
488/189. The gallery facade loses wiring; classification annotation/retry presentation is removed.

## Archive version rejection fixture — 2026-11-01

The Catalog job for `63e2997b` passes 902 tests and fails the archive malformed-input test. Its
future-version mutation still replaces version 5 after the writer advanced to version 6, leaving
the valid archive unchanged. The fixture now derives the unsupported version from the codec's
current version and requires each malformed sample to differ from its input. All six focused codec
tests pass, including truncation, digest/count corruption, unsupported fields/kinds and cancellation.
No archive reader, format, stored record or restoration policy changes. Hosted CI follows the fix.

## Resumed isolated desktop acceptance — 2026-11-02

The existing 51-file / 50-group Profile client is rebuilt from `e801841b` with the preview repair
below. Its receipt verifies the optimized native library's test-storage override before launch.
The existing synthetic corpus, catalog and model are reused. Each process uses a leased temporary
directory, including `TEMP` and `TMP`; no original library or catalog is opened or changed.

The first resumed run reproduces a Windows crash twice: add a smart-album image to Favorites,
open it from Favorites, then cancel Favorite in the preview. Flutter reports an invalid AXTree
update with pending nodes before exiting in `flutter_windows.dll`. Preview classification used a
direct `Tooltip` and an implicit `IconButton.tooltip`, bypassing the accepted Windows boundary in
[ADR 0009](../architecture/0009-accepted-unified-gallery-ui-contract.md). Both now use the existing
`AmeTooltip`; the catalog-read retry retains its accessible name. Per-image analysis failures remain
silent, and no runtime, schema, model or classification policy changes.

The retained-semantics regression embeds classification in the actual `LibraryImageViewer`, whose
slider also owns a portal. A row-only fixture misses the failure; the real-viewer fixture rejects
a missing traversal child before the repair. It now passes category hover, loading, empty results,
catalog-read failure, recovery and disposal on Windows. This regression and the 11 existing preview
classification checks pass, as do owned formatting, Dart analysis and whitespace checks.

The resumed isolated-client interactions, including replay after the preview repair, verify:

- The original Favorite cancellation path completes without a crash or AXTree error. A second
  preview cancellation after process restart also completes; both repaired processes exit normally.
- Continue Analysis resumes the saved paused setup and updates the Screenshot category from one
  to six groups. Pausing again survives restart, and the menu offers Continue Analysis afterward.
- An empty Illustration category shows the existing empty state; returning to Screenshot restores
  its six groups. The previously classified duplicate remains one card with two locations.
- A 7,680 × 4,320 image recorded as `classification_image_limit` opens normally. Its card, context
  menu, preview and preview menu expose no analysis-failure indicator or manual retry command.
- A smart-album image can be favorited, opened and unfavorited, and can join the ordinary `r5` album.
  The bottom result and preview-top result use the shared feedback surfaces. The destination tray
  contains only the ordinary album. The album gains that image and retains it after restart.

Automated Undo inputs produce no saved undo, and the next observations show the notices gone; those
attempts do not establish a new desktop Undo result. The retained native organization/undo coverage
is unchanged. Final fixture state retains the newly added album member without a Favorite mark;
the original two album members and two Favorites remain unchanged.

Before/after comparisons preserve all 51 source-file SHA-256 hashes, eight complete durable-table
digests and all five pre-existing organization records. The sole new organization record belongs
to the exercised image. All nine sampled tables match exactly across the process restart. The 15
permanent failed work items retain failure count one across Continue Analysis and restart; no retry
budget is reset. SQLite quick-check returns `ok` and foreign-key checking returns no rows. Both
sources complete their journal-loss recovery and display synchronized status. Both leased runtime
directories are removed after normal exit; no complete profile copy is created for this replay.

The preview classification owner contains 52 production lines and no inline tests. Its new dedicated
semantics fixture contains 115 lines; the existing 141-line behavior fixture is unchanged. The
submitted tooltip repair `0d5801af` passes all 15 applicable jobs in
[run 41400808080](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41400808080);
the three conditional signing jobs are skipped. The roadmap retains the measured large-library
latency as follow-up work after accepting the current performance on 2026-11-02.

## Shared filter menu — 2026-11-02

Smart albums now use the existing gallery Filter menu, including its source, duplicate-display and
metadata groups. The separate one-entry menu is removed. Fixed-group scopes show Merge duplicates
selected and disable both duplicate choices; manual albums previously offered All images even
though their query always remained folded. Library and Favorites keep their supported choices.

The integrated smart-album fixture fails on the missing shared entries before the repair. All six
affected filter, metadata-editor and smart-album checks pass afterward, including Windows retained
semantics, disabled actions and metadata apply/cancel/clear. Owned Dart analysis has no warnings.
The rebuilt isolated Profile client shows matching Library and Screenshot filter layouts, with the
smart-album duplicate choices disabled and the shared metadata action available. No source, model,
schema or native query behavior changes. The query model/filter owners contain 437/111 production
lines and no inline tests; the amended metadata/smart-album fixtures contain 138/331 lines.
