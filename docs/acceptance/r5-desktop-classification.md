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
native/client lifecycle checks; model-backed first-use interaction remains pending. The roadmap owns
the remaining context coverage and integrated quality acceptance.

Classification continuation and real-client
acceptance of contextual review retain their R5-06 scope. Integrated quality, scale and the paired
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
