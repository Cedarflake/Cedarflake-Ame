# R7 implementation evidence

Date: 2026-11-07
Scope: filters, shared features, semantic retrieval and resource-aware scheduling on
`codex/r7-semantic-discovery`.
The [roadmap](../roadmap.md#r7-implementation-sequence) owns disposition and the
[plan](../plans/r7-semantic-discovery.md) owns remaining slices.

## Composed filter implementation

The existing filter menu retains its four quick choices and adds `更多筛选`. A typed presentation
draft validates extensions, exact byte conversions, dimensions, explicit calendar dates and camera
text. Applying publishes one existing metadata query; cancelling preserves it. A retired opening
query cannot receive the editor result. Invalid custom fields are revealed and brought into view.
Filter-menu summaries remove one condition or clear metadata without changing scope,
literal search or duplicate display.

The revised editor supports multiple formats through the existing shared native predicate. Schema
77 and archive version 7 retain old single-format scopes through the typed compatibility owner.
No model package or source-write behavior changes; query generation and gallery position retention
remain the existing owners.

## Component admission

Inspected the [Material component catalog](https://m3.material.io/components) and the installed
Flutter 3.47.8 / Dart 3.13.7 implementation; the hosted Windows workflows pin the same Flutter version.

| Purpose | Framework capability and choice |
| --- | --- |
| Draft editing | `showModalBottomSheet` uses the album picker's standard centered placement and theme width. Multi-select `FilterChip`, `Form` and `TextFormField` retain the application theme. `Scrollbar` exposes scrolling; `validateGranularly` reveals invalid fields through framework scrolling. |
| Date field, ranges and size unit | The shared `AmeChoice`, extracted from the settings selector, opens date and unit choices through `AmePopupMenuButton` and its 200-ms transition. `DropdownMenu` does not expose that route transition and is excluded under ADR 0009. Direct minimum/maximum fields replace range presets. |
| Calendar | Read-only `TextFormField` opens `showDatePicker`, a compact desktop dialog. Two independent bounds support an open-ended interval and individual clearing. |
| Camera and control shapes | Outlined `TextFormField` under its section title. Fields use the framework's standard outline; chips and menus retain the application theme without local shape overrides. |
| Applied conditions | The shared popup-menu owner exposes removable conditions and Clear; the existing tooltip summarizes active conditions. No custom input, menu, picker, scrolling or chip primitive was introduced. |

## Verification

Initial single-format editor evidence, before the compact multi-format revision:

- Focused Flutter checks: 16 passed across the filter editor, draft validation and existing position
  regressions. Coverage includes cancel, clear, individual removal, retired query authority, exact
  values above the floating-point integer range, invalid bounds/dates, and original-position recovery
  in both gallery layouts. Existing position cases now include metadata summary insertion/removal.
- Changed-file Dart analysis passed with no issues after the field-reveal change. Dart dependency
  boundaries passed for 529 modules with zero frozen exceptions.
- Built and ran the Windows Debug client with the production entrypoint and isolated preferences.
  Rust used the repository's optimized Debug profile with test-storage admission; the guarded build
  receipt bound source-input and native DLL hashes before launch.
- Reused 51 retained fixture images in a leased isolated catalog. The native client showed the
  standard filter menu, calendar and keyboard-editable fields. The following counts matched an
  independent file inventory and PNG-header dimension reads:

| Native interaction | Observed result |
| --- | --- |
| Apply PNG and minimum width 2000 px | 9 images; both condition summaries visible |
| Open result, move right, return with Esc | Viewer moved from 4/9 to 5/9; returned to the same filtered gallery position |
| Remove width only | 17 PNG images; extension condition retained |
| Clear remaining condition | 51 source images / 50 exact-group representatives; original gallery position restored |

The client exited normally, all 51 source SHA-256 hashes remained unchanged, and the scratch profile
was removed by its lease owner. No client overflow or exception was observed in the run logs.

### Compact multi-format revision

Replaced single-format selection with a normalized multi-format set shared by predicates, query
identity, the generated bridge and saved scopes. The editor puts section titles above controls, uses
one row of compact format chips and direct numeric ranges in a centered bottom sheet. The toolbar
menu replaces the header's format chips and permits individual removal without shifting gallery content.

The 18 affected Flutter cases passed across draft editing, query coordination and both gallery
layouts. Rapid consecutive format taps initially exposed an overwrite from a stale child snapshot;
selection toggles now use the draft owner's current set. Changed-file Dart analysis passed.
The 40 focused Rust cases passed: OR formats with other AND conditions, count/page/layout/selection
agreement, identity, saved continuation, transactional migration rollback, legacy upgrade and archive
staging/restoration. Initial fixture creation failed because the system temporary directory denied
access; the same compiled tests passed with `TEMP`/`TMP` redirected into the leased repository scratch.
The scratch owner removed the temporary catalogs after completion.

The final centered-sheet revision passed the four existing editor cases and changed-file analysis.
The ten position and four duplicate-display regressions also passed during this revision; no additional
cosmetic test suite was added. The native Windows client used 368 retained JPG previews in an isolated
catalog: JPG plus PNG returned 368, removing JPG returned zero, Escape discarded a draft change while
preserving PNG, and Clear restored 368. The sheet used the album picker's centered Material placement;
scrolling reached the file-size and camera fields while its title and action row remained visible.
The client exited with code 0, all 368 preview hashes remained unchanged, and the scratch lease was
removed. No model worker ran. Maximum client private memory was 477.1 MiB; minimum available system
memory was 4174 MiB during the client check and 4192 MiB during the guarded, single-job build.
Subsequent rebuilt-client checks confirmed that Clear follows More filters before the removable
conditions, and the sheet's right-side content inset separates fields from the scrollbar.
The shared-choice extraction passed the 19 existing filter/settings cases and changed-file analysis;
the retained settings case covers selected values, keyboard activation, disabled state, semantics
and the shared opening/closing transition durations.
The user confirmed matching unit-selector and numeric-field heights in the rebuilt Windows preview.

## Reviewability and remaining evidence

Original filter-slice production / inline-test lines: screen assembly 1919 / 0, filter draft 188 / 0,
dialog 292 / 0, summary 95 / 0. Dedicated affected tests contained 244, 61 and 256 lines respectively.
Current filter production / inline-test lines: screen assembly 1970 / 0, draft 198 / 0, sheet 380 / 0,
range fields 50 / 0, format choices 75 / 0, menu 207 / 0 and summary 84 / 0. The editor regression has
268 dedicated test lines. Editing, parsing and applied-condition summaries remain separate owners;
the gallery page only connects the existing query transition.
The promoted shared choice contains 144 production / 0 inline-test lines; settings section, page,
storage and cleanup consumers contain 98, 271, 738 and 155 production lines with no inline tests.
The existing settings suite retains its assertions in 1231 dedicated test lines.

Branch pushes alone do not trigger the CI workflow; its PR, main-push or manual-dispatch execution
remains the hosted verification owner.
Semantic indexing, text search and related-image search have separate evidence below. The stage's
real-library search quality and performance acceptance remains in R7-05.

## CI reduction — 2026-11-07

Run [42348844763](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42348844763) at `ee63a1c8`
passed Static, Windows build/integration and the five synthetic jobs. Flutter failed on an obsolete
CPU sentence; Rust Remaining failed on a fixed settings-version assertion and two unrelated domain
module-roster assertions. Its 1,228 passing Rust tests took 1,556 seconds; the Flutter wrapper started
about 240 separate commands before reaching its failure.

Removed the incidental version/text assertions and unrelated domain roster/visibility mutations.
Migration data, download authority, device switching, the availability call closure, protected DTO
loading and dangerous alternate-loader checks remain. Local focused results: one migration and
11 availability cases passed; seven Flutter cases across three suites passed in one invocation
(runner-reported test duration: three seconds).

Quick/full/extended selection and aggregate failure propagation passed their focused checks;
actionlint and PowerShell syntax checks passed. Presentation changes select two main jobs, critical
or unknown changes select nine, and nightly/extended runs select fourteen. Release workloads bind
the candidate source and no longer duplicate the scan benchmark inside the runtime verifier.
Changed large owner counts: `local_files.rs` 2,603 production / 3,351 inline-test lines; dedicated
topology tests 458 lines. No source-reading production behavior changed.

Run [42356744377](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42356744377) at `2c039388`
completed all 1,783 Flutter cases in four batches / 260 files; invocation-to-completion time was
13 minutes 27 seconds. The earlier run spent approximately 24 minutes before its stale assertion
failed. Rust Remaining passed 1,232 cases in 1,160 seconds; runner variation prevents attributing the
entire timing difference to pruning. All four Rust partitions, Windows build, scan and accessibility passed.
Static exposed a duplicate assertion tying a scan benchmark to the old verification job; `d2dd220d`
removes it while the independent release workload remains required. Its existing guardrail passed
under PowerShell 7 and Windows PowerShell 5.1. Remaining hosted results retain that run's revision.

The quick path additionally avoids unrelated native compilation/packaging checks and selects Flutter
suites through the existing Dart parser. Bounded selection probes covered direct/transitive imports,
conditional exports, parts, removed targets, cycles and runner configuration. The actual R7 filter
comparison selected 203 of 260 suites and included its filter/draft cases; that broad change includes
a shared domain type and therefore still selects full CI. No new permanent test matrix or changes to
`testing.md` were added for this refinement. Its hosted quick-path duration remains unmeasured.

## Shared-feature foundation

[ADR 0040](../architecture/0040-durable-discovery-features.md) owns common feature identity and the
independent SQLite store. Classification now reuses compatible durable features and promotes LRU
hits; clearing its LRU retains discovery features. Newly encoded features publish after source and
catalog revalidation, before category evidence. Original F32 values and existing key hashes remain
unchanged. No new model, inference worker or production retrieval dependency was added.

Rustfmt and all-feature library Clippy passed. Five real-SQLite store cases cover reopening, model
separation, budget retention, atomic rollback, corruption and foreign-database rejection. One shared
repository case covers cache promotion and LRU cleanup. The six existing analysis, ten domain and
eleven cache regressions also passed. The fixed dependency inventory check passed with updated
symbol-scoped composition registrations; no legacy exception was added.

Production / inline-test lines: store facade 193 / 0, storage admission 107 / 0, shared repository
92 / 42, common features 171 / 0, native executor 212 / 0. Dedicated store tests contain 154 lines.
These checks establish the persistence foundation. The connected search and scheduling checks below
retain their separate scope.

## Retrieval evaluation

An ignored Windows x64 release probe used USearch 2.26.6 with default features disabled and the
retained 368 image / 48 text-query F32 embeddings (768 dimensions). No model download or source
image read was needed. Exact F64 cosine supplied the comparison; scopes selected all candidates,
every third candidate and every seventeenth candidate before ranking.

Search expansion 128 recovered 479/480 exact top-ten results in the full scope. Expansion 512
recovered 480/480 in each scope. Full-scope median/p95 retrieval was 0.098/0.150 ms; build time was
19.2 ms. Serialized size was 1,232,948 bytes and engine-reported memory 16,783,168 bytes, excluding
process overhead. Save/load, mapped-view agreement, removal and a Unicode path passed. Native
search has no observed cancellation callback, so integration would need bounded work windows.
These small-corpus measurements do not establish full-library latency, recall or resident memory.

## Parallel-worker evaluation — 2026-11-07

The local-primary comparison used the production Profile worker, protocol 2, the unchanged admitted
SigLIP 2 artifacts, ORT CPU 1.31.1 and DirectML 1.24.4 / 1.15.4. Worker SHA-256:
`3f06854bd9d5e218f22b88256a036691e09eec5cc879178532f4edd9f6ee3f63`.
The host has an i7-13620H, 10 physical / 16 logical processors, 15.80 GiB usable RAM and an RTX 4050
Laptop GPU. CPU-only results below disable GPU inference on this host; they do not characterize a
low-core or low-memory machine.

GPU runs used all 368 retained previews and 48 bilingual descriptions. CPU-only runs used 64 evenly
spaced previews from that corpus and the same 48 descriptions. The harness prepared bounded image
tensors and drove the actual native protocol, with one below-normal-priority worker per encoder,
one inference thread per worker and the existing one-process / 2-GiB Windows Job limits. It opened
no originals or catalogs. The application scheduler, decoder and semantic retrieval were outside
this experiment. Each paired configuration ran sequential, parallel, parallel, sequential batches;
sequential batches retained both workers to distinguish concurrent execution from model reloads.

| Paired configuration | Sequential image median | Parallel image median | Sequential text median | Parallel text median | Peak combined private commit |
| --- | --- | --- | --- | --- | --- |
| GPU image + CPU text | 13.07 ms | 13.11 ms | 63.50 ms | 52.53 ms | 1,375 MiB |
| CPU image + CPU text | 363.37 ms | 368.80 ms | 37.40 ms | 42.60 ms | 1,644 MiB |

Medians pool both runs of each mode. GPU-run text timing varied substantially: per-run medians were
36–136 ms sequential and 38–150 ms parallel, so the pooled difference does not establish faster text
encoding. The two GPU image batches took 5.27 / 4.81 seconds sequential and 5.28 / 5.06 parallel.
CPU image batches took 23.42 / 23.31 seconds sequential and 25.72 / 23.33 parallel. Every produced
embedding matched its retained same-backend reference exactly, including concurrent execution.

The isolated image workers used approximately 1,006 MiB private commit on GPU and 1,277 MiB on CPU;
the additional text worker used approximately 368 MiB during steady execution. A separate cold-load
measurement observed a 702-MiB text private-commit peak and a 1,339-MiB GPU-image peak. Their sum is an
upper bound from individual process peaks, not a measured simultaneous peak. Working-set trimming
occurred during the runs; private commit and physical resident memory are distinct measurements.

Starting text while the GPU image worker continued required 1.79 seconds through the first query.
Serial image retirement plus text startup/query took 1.62–1.71 seconds; retiring text and recreating
the GPU worker through its first image took another 1.70–1.86 seconds. Parallelism therefore avoids
background interruption and image-worker replacement, while first text use retains cold-start cost.
Terminating the owned text worker during a separate 64-image GPU pass left all image outputs valid;
this establishes process isolation, not application cancellation/restart acceptance.

The measured benefit of parallel workers is background continuity and avoided image-worker reloads,
rather than faster individual inference. [ADR 0041](../architecture/0041-resource-aware-analysis-scheduling.md)
admits bounded concurrency when device resources permit and serial execution otherwise. These process
measurements do not yet verify that scheduler's foreground latency or mode transitions in Ame.
All owned processes retired, leased scratch directories were removed and
all 368 retained preview hashes remained unchanged. Raw timings and exploratory harnesses remain
under ignored `build/r7-semantic-eval`; no new permanent CI workload was added.

## Connected scheduling and search

The application now owns independent foreground search and background indexing behind the same
two-encoder scheduler. Cancellation remains request-specific. Low capacity uses serial admission;
memory pressure closes parallel admission, with delayed recovery. Cached text queries do not extend
the serial model-retention interval. Source-image preparation retains its file lease through final
reference revalidation. Retrieval reads a fixed catalog snapshot and feature generation in bounded
windows, so background publication cannot invalidate or reorder an active result manifest.

Eleven focused Rust cases passed across scheduler concurrency/retirement, capacity hysteresis and
dispatch cancellation. A real SQLite two-connection case additionally passed: a background catalog
publication commits while the reader finishes its original 257-item snapshot; a fresh reader sees
the replacement. All-feature library Clippy and the fixed dependency inventory passed. Dart analysis,
two search-coordinator cases, three retained search-focus cases and six search-availability cases
passed. The coordinator cases cover stale replies/errors, IME composition, Enter submission and
scope/mode continuity; they do not establish native desktop acceptance.

The native text worker also completed 48 bilingual descriptions and the maximum 512-character input
under the new 1-GiB Windows Job limit while a GPU worker processed 32 retained previews. Text private
commit peaked at 702.2 MiB, with a 33.5-ms median warm query. Both encoders matched their retained
reference vectors exactly. Both processes retired and all 368 retained preview hashes were unchanged.
This run validates the worker limit; connected desktop scheduling is verified separately.

An isolated optimized native desktop run indexed all 368 retained previews while servicing searches.
Repeated cached queries took 10–41 ms as coverage advanced; the first query, including model startup,
took 4.54 seconds. The connected 48-query bilingual run retained the earlier fixed-target results:
Chinese top-one/top-five 15/20 of 24, English 19/23 of 24. Median query latency was 72.40 ms and p95
105.18 ms; one cold text-worker restart took 2.41 seconds. A root-restricted query counted 128 eligible
images, and reference-image retrieval excluded its source and returned the other 367 images.
These measurements describe the retained preview corpus, not whole-library latency. This initial
run ranked all eligible candidates; the subsequent relevance admission below changes returned counts.

The process sampler observed at most two model workers, peaking at 1,603 MiB combined private commit.
Idle workers retired, normal window close exited successfully, all owned workers joined, disposable
storage was removed and all 368 source hashes remained unchanged. Native search input and relevance
results were exercised in the window. This pass exposed inherited gallery positioning and rejected
position seeks on relevance results; the affected regression now verifies ordered pages and seeks
for capture/name/modification settings against a fixed shuffled SQLite fixture.

The rebuilt desktop verified the revised Material format/range chips and date picker. Applying PNG
returned zero results in the all-JPEG corpus; removing that condition restored 368. The header showed
only the count and the search bar omitted trailing scope text. A new `fox` query started at position
0%; dragging to 56% loaded the selected result window and dragging back restored the first result.
Opening that image, moving to the next and returning retained relevance order and the gallery
position. `查找相关图片` switched to 367 results at position 0%, excluding its reference. Normal close
returned exit code zero, all owned workers retired, scratch storage was removed and all 368 preview
hashes remained unchanged. This pass does not cover restart recovery or large-library latency.

### Relevance admission

The initial connected search ranked every indexed candidate, so small libraries returned the entire
library. The relevance rule in [ADR 0040](../architecture/0040-durable-discovery-features.md#relevance-admission)
removes the weak tail and permits an empty result. Reusing the frozen F32 vectors, it retains the
fixed target for 21/24 Chinese and 24/24 English descriptions. Their existing top-five target counts
remain 20/24 and 23/24. Median returned counts are 5.5 and 1; these fixed-target measurements do not
label every returned image as relevant.

Six short-term probes and ten additional descriptions used the same admitted native text worker.
`fox` selects two of 368 images; the Chinese equivalent selects ten. Eight of the ten additional
descriptions produce no matches, including both language versions of a red helicopter in snow,
microscope bacteria and a coral reef with a diver. Chinese rocket-launch and circuit-board queries
still return candidates. The heuristic reduces irrelevant tails but retains the model's semantic
errors; no probability or universal accuracy claim follows from these cosine cutoffs.

The rebuilt native client returned only `B142.jpg` and `B143.jpg` for `fox`. The complete description
`a red helicopter parked in snow` returned zero with `没有找到匹配的图片`; clearing restored all 368
images. Normal close succeeded and all retained preview hashes were unchanged. Two focused policy
regressions, changed-file Dart analysis and all-feature library Clippy passed.

Production / inline-test lines: scheduler 184 / 0, admission 245 / 0, search execution 119 / 0,
index pass 174 / 0, retrieval 154 / 0, image preparation 126 / 0, presentation coordinator 338 / 0,
screen assembly 1977 / 0, filter dialog 316 / 0 and range choices 125 / 0. Dedicated scheduler tests
contain 214 lines, coordinator tests 164 lines, relevance tests 46 lines and filter interaction tests
267 lines. New search
widgets reuse the existing Material search bar, menus, notification controller, gallery and viewer.

## Album icon follow-up — 2026-11-08

The album changes based on `26fe6d14` add schema 78 icon metadata and archive version 8 without
adding icon changes to undo history. Nine focused Rust cases passed for migration rollback,
create/replay/edit/reopen, history isolation, archive compatibility and retained album operations.
Initial compiler hosts failed under experimental memory caps, and system temporary storage rejected
fixture creation; the same checks completed under the existing system-headroom monitor and leased
repository scratch storage. Thirteen existing album Flutter cases, changed-file analysis, Dart
dependency boundaries and bridge contracts passed. Layout follow-ups reused six album cases.

The isolated native Debug client verified icon selection before name entry, saved sidebar icons,
context-menu editing, retained selection when reopening and feedback without an icon Undo action.
The user verified the album-selection surface and requested its rounded interaction shape and
`更改 12 的图标` title format. The selection row retains its single-album behavior with the icon/name
on the left and checkbox on the right. A completed isolated run exited normally with all 368
retained preview hashes unchanged; later preview sessions remain independently scoped.

Production / inline-test lines: migration dispatcher 4,132 / 2,349; album persistence 259 / 0;
album picker 435 / 0; album editor 169 / 0. Dedicated icon migration and persistence tests contain
41 and 81 lines. The dispatcher only registers the new typed migration owner.

## Failed related-search acknowledgement — 2026-11-08

The follow-up to `29af2ebe` connects acknowledgement to the current failed reference search.
It clears the reference and restores browsing with the existing scope and filters. Notification
identity and failure admission reject an old acknowledgement during or after a newer retry;
partial-result index failures retain their results. Retry keeps its existing search target.
Three search-coordinator cases passed, including the failed-reference and stale-notification
regression. Changed-file analysis and the rebuilt native Debug client passed; the user confirmed
the remaining desktop interactions. Production / inline-test lines: search coordinator 349 / 0,
screen composition 1,975 / 0; the dedicated coordinator suite contains 229 lines.

## Search alignment follow-up — 2026-11-08

The follow-up to `fe501efb` aligns the search field with the gallery heading using the existing
sidebar transition and live resize width. The native Debug client retained alignment when collapsing,
expanding, resizing from 260 to 340 pixels and resetting the sidebar. Changed-file analysis and twenty
existing shell, sidebar-motion and search-focus cases passed for that alignment change.

The spacing follow-up to `35377206` reserves 32 pixels beside the expanded search field and lets the
collapsed field stretch with a 16-pixel gap. Interpolating fixed endpoint widths removes the reversing
right edge caused by deriving the width cap from the animated inset. The follow-up passed analysis
and Debug compilation; the native client verified both sidebar states and remains open for inspection.
No new tests were added. Production / inline-test lines: screen composition 1,975 / 0, workbench
325 / 0, sidebar transition 156 / 0 and global bar 174 / 0; the existing shell suite has 356 lines.

The shared viewer-notice follow-up to `4cd0a94c` gives actionless messages symmetric 14-pixel
horizontal padding. Eleven existing feedback-template and organization-feedback cases, changed-file
analysis and Debug compilation passed. The user verified the rebuilt native client's appearance.
The optional-action layout and its existing dismissal and undo behavior remain unchanged.

## Derived-index recovery — 2026-11-08

The follow-up based on `529eac58` replaces the index retry counter with joined analysis retirement,
owned-store recovery and reopening under [ADR 0040](../architecture/0040-durable-discovery-features.md).
Actual corrupted storage terminated analysis after opening; the disconnected search path incorrectly
reported an absent model. Search now preserves startup and runtime causes. Acknowledgement clears
the current failed text or reference query; stale notifications and partial-result index failures
cannot clear another request.

Eight native API scenarios covered missing and healthy stores, damaged vectors and headers,
interrupted reset, exclusive locking, foreign substitution and recreation after removal. Healthy
features, the catalog sentinel and the foreign substitute remained unchanged; locking and substitution
were rejected. These checks reused the optimized Debug DLL with isolated-storage assertions.

The actual production Dart bindings and native runtime recovered a damaged header in 117 ms and
indexed all 368 retained previews within 34.3 seconds. `fox` returned the two known matches;
warm queries during recovery took 6–19 ms. The normal desktop separately showed the storage-damage
message, cleared text on `知道了`, and recovered through `重试搜索` and `更新搜索结果`. Both runs
closed successfully, joined their workers, cleaned temporary profiles and preserved all source hashes.

Twenty-six lifecycle cases and four search-coordinator cases passed, along with affected-file Dart
analysis. Earlier all-feature Rust checking and the fixed dependency inventory cover the unchanged
native recovery boundary. Local Rust unit-test compilation hit the system memory safeguard;
execution of those cases remains assigned to hosted CI. This is not large-library latency evidence.

Production / inline-test lines: recovery facade 47 / 0, lifecycle 76 / 0, session lifetime 143 / 0,
search coordinator 365 / 0, feature-store facade 216 / 0, ownership receipt 77 / 0 and storage recovery
139 / 0. Dedicated lifecycle, search and storage-recovery suites contain 884, 250 and 118 lines.

The request-order follow-up to `dd70397c` serializes native handle preparation while allowing a new
query to cancel active retrieval. The bridge dispatches preparation through a thread pool; a delayed
older arrival could otherwise replace the newer native request. Five search-coordinator cases passed,
including delayed preparation, stale replies, IME/Enter, scope changes and failure acknowledgement.
Affected-file analysis passed. The session owner has 75 production lines and no inline tests; the
coordinator suite has 292 lines. Native desktop continuation retains its separate verification scope.

## Large-library retrieval — 2026-11-08

The follow-up to `3244947a` keeps a single SQLite candidate cursor for each search and feeds bounded
256-item batches to exact scoring. Location-ordered access avoids repeatedly sorting the remaining
library; the single cursor computes scoped duplicate representatives once. Background indexing keeps
its bounded keyset pages. Scope predicates, preferred representatives, feature-generation admission,
the fixed catalog snapshot and the 500-result cap remain unchanged.

The production Dart/native path used a leased online backup of local-primary: 63,121 active locations,
a 482.8-MiB catalog and a 90.4-MiB classification cache. The baseline spent 25.7–27.9 seconds on warm
unfiltered queries after twelve minutes of indexing. Fixing location order alone reduced those
queries to about one second, but merged mode exceeded the 120-second deadline. The complete cursor
change produced these results with the same source population and a 90-second indexing budget:

| Operation | Time | Eligible / indexed locations |
| --- | --- | --- |
| First Chinese query, cold text worker | 2,989 ms | 63,121 / 0 |
| Warm unfiltered queries | 894–1,079 ms | 63,121 / 891–944 |
| Chinese query with merged duplicates | 2,741 ms | 47,877 / 655 |
| PNG-restricted flower query | 649 ms | 16,526 / 382 |
| Cancellation acknowledgement | 7.3 ms | Active request retired |
| Joined runtime shutdown / healthy-store reopen | 352 / 26 ms | Existing features retained |
| Cached query after reopen | 805 ms | 63,121 / 985 |

Reference search excluded its own location and returned 165 results from 63,120 eligible locations.
The first English query after worker inactivity took 2,731 ms. The baseline reached 8,515 indexed
locations; the shorter final run reached 985 and did not finish indexing. These are partial-coverage
latency measurements, not full-index throughput or semantic-quality comparisons.

The complete run exited successfully in 135 seconds, with at most two model workers. Peak client
private memory was 124.7 MiB; combined worker private memory was 1,375.5 MiB. Physical/commit headroom
stayed above 3,574 / 3,981 MiB. The semantic database reached 2.75 MiB and the entire derived profile
1,078.7 MiB. All workers joined; all 63,121 source metadata records, 64 sampled source hashes and the
original catalog/cache hashes were unchanged. The leased snapshot was removed afterward.

The optimized Debug DLL retained debug assertions and isolated-storage admission; its SHA-256 was
`EFC1F39B23EBADD04B5FC160F088925CD7B277213075D173D6227A708F63B358`.
The existing snapshot regression now checks streaming across a concurrent publication; a focused
instruction-budget regression rejects whole-library sorting for a 256-item page. Execution of these
Rust cases remains assigned to hosted CI. Native compilation, affected formatting and diff checks
passed. Production / inline-test lines: filter owner 161 / 0, snapshot 161 / 0, retrieval 146 / 0;
the dedicated semantic catalog suite contains 214 lines.

## Desktop continuation — 2026-11-08

The isolated desktop at `8141d463` completed two normal launches using the real Windows preference
serializer redirected into the leased profile. Content search initially showed partial coverage;
`fox` returned two matches after indexing. Selecting the 128-image folder narrowed the effective
scope; `整个图库` restored the 368-image scope and its two matches. Rapid character input, replacement,
Enter submission and clearing kept the current query's result. Relevance results exposed generic
scroll position rather than date seeking. File-name mode returned only `B142.jpg` for `b142`.

After clearing and closing, the second launch restored the selected file-name mode with an empty
field. Switching back to content search reused all 368 indexed locations and returned the same two
`fox` matches. Both launches exited with code zero, joined their workers and preserved all 368
preview hashes; the leased profile was removed afterward. Earlier viewer, filter and reference-image
interaction evidence remains applicable to these unchanged surfaces.

The existing composition/Enter regression passed, but actual Windows IME observation remains open:
the available native automation did not deliver composition or Chinese text to the field. That
input limitation is not recorded as a product failure or a completed native IME check. The
[`60778542` full run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42414123566) passed
Static, Flutter, Windows accessibility, Windows scan, production-bundle build, Rust Remaining and
Rust Synchronization and Scan. The shared-choice cases now locate visible
Material controls instead of a removed component type and menu key; existing behavior and native
accessibility assertions remain. Seven affected widget cases also passed locally.

Rust Remaining exposed the same roster coupling for unrelated adapter modules and a paused-startup
assertion that equated an absent cache directory with no model work. The boundary guard now admits
ordinary sibling declarations while retaining protected modules, exact call closure and alternate-
loader rejection. The paused-session case retains its absent-model fixture and joined lifecycle,
and checks zero cached features. Rust Remaining passed with these corrections; production behavior
is unchanged.

Schema and Queue passed 403 cases and failed seven migration fixtures. Six called current album
queries on deliberately old schemas before the icon table existed; they now verify historical
state directly where necessary and reopen through normal upgrades before using current APIs.
The v33 preservation case no longer enumerates every later migration's new tables: it still compares
all original rows and sequence values across two reopenings, while dedicated migrations own their
new defaults. Rollback, receipts and organization-history checks remain. Formatting and diff checks
passed; execution of the corrected fixtures remains assigned to hosted CI. Dedicated test lines:
camera metadata 130, compact search 224, search 201, admission clock 101, history 118, queue index 203;
this correction changes no production or inline-test code.

Catalog passed 928 cases, failed two and retained seven explicitly manual/profile cases as ignored.
Idle maintenance had the same directory-existence assumption; both maintenance lifecycle cases now
check zero cached features while retaining absent-model/source fixtures. The export fixture now
assigns an album icon before requiring that record family in the archive. The existing icon
export/restore/retained-choice case passed in this run. These test corrections passed formatting and
diff checks and await hosted execution; their dedicated suites contain 304 and 217 lines, with no
production or inline-test changes.
