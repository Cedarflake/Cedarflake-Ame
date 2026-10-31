# R5 incremental analysis evidence

The [roadmap](../roadmap.md#r5-implementation-sequence) owns stage status and order.
[ADR 0036](../architecture/0036-durable-classification-dispatch.md) owns durable dispatch authority;
the [runtime record](r5-runtime-admission.md#demand-loaded-encoder-residency) retains native inference
and encoder-lifetime measurements.

## Durable work records — 2026-10-30

Schema 66 adds source-verification and exact-content tasks, dispatch intent, stopped-run identities,
and process-session leases. Source admission reads at most 64 committed catalog entries per backlog
page or 256 explicitly selected entries. Compatible content shares one task, including copies
verified before the complete duplicate scan finishes. One transaction publishes a prediction and
its completed checkpoint; manual controls and process replacement invalidate earlier claims.

Manual pause survives process restart. Abandoned and transient work consumes a three-attempt budget;
unavailable sources wait without consuming that budget. Explicit retry resets failed/waiting work.
Requested, new-content and backlog lanes receive weighted turns, while only one task holds a claim.
Stop retains the run identity; a new explicit run may reuse compatible unfinished work and evidence.
Execution records remain outside portable human-decision archives. No source file reads, media
mutation, hydration or downloads are added by these repository operations.

Sixteen new regression cases cover duplicate sharing, per-source exact publication, manual pause,
restart and abandoned work, stopped/disabled runs, surviving copies after removal, failure budgets,
fair admission, bounded backlog, atomic result/checkpoint rollback, and migration preservation and
rollback. The historical version-33 snapshot now checks the two empty work tables and exact initial
dispatch singleton before comparing all retained older rows.

Local library compilation with `classification-worker` passed in 20.37 seconds. Production Clippy
with that feature and warnings denied passed in 27.97 seconds. A debug-info override caused a
dependency compilation to exhaust memory; reusing the canonical default-profile cache completed
the library check. The full root test compile also exhausted compiler memory and supplies no test
execution result. Hosted exact-commit checks own the new runtime and migration regressions; no
workload or assertion was removed. Formatting and whitespace checks passed.

This storage implementation is not yet connected to the desktop dispatcher. Foreground admission,
model setup and shared progress remain part of the same R5-04 workflow. The guarded verification
and feature-storage connections are recorded below. No background classification is enabled by
these internal operations.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Work domain / port | 77 / 71 | 0 | Repository scenarios below |
| Repository facade | 145 | 0 | 469 admission/publication + 252 recovery |
| Dispatch / admission / leases / completion | 248 / 241 / 245 / 144 | 0 | Shared repository scenarios |
| Migration / schema | 108 / 59 | 14 migration fixture helper | 116 migration |

## Guarded per-source verification

The classification workflow now selects its leased catalog source without requiring a same-size
duplicate peer. It uses the existing root-relative no-recall reader and 256-MiB encoded-source bound.
The held file and namespace lease spans fingerprint revalidation and the atomic fingerprint,
gallery/organization membership and task-checkpoint transaction. Existing compatible fingerprints
retain their original provenance and can complete verification without another media read.

The ordinary exact scanner keeps its peer pruning, run authority, bounded batches and publication
order. Shared membership publication now accepts the already-validated bounded generation list;
classification first validates its current task/session/epoch and exact source identity. Its native
entrypoint reuses the validated catalog-session registry and background write lane. Classification
verification does not emit or finish the complete duplicate scan's progress.

Six additional database scenarios exercise singleton admission, zero-read evidence reuse, source
lease retention through commit, rollback of fingerprint/membership/run/checkpoint, pause during
validation, hash cancellation and changed-source rejection. Existing R3 batch tests retain their
run-isolation, terminal-authority, bounded-publication and revision/rollback checks.

Production Clippy with `classification-worker` and warnings denied passed in 18.92 seconds outside
the restricted executor after the native entrypoint was corrected to use the catalog-session owner.
The restricted attempt exhausted compiler memory. A targeted database-test build then failed to map
Rust test metadata because Windows reported its pagefile limit (OS error 1455), and was stopped
without running tests. This is not evidence of an executed regression pass. Exact-commit hosted
checks retain those scenarios. Formatting and `git diff --check` passed; no originals were opened.

The hosted `dbdf2365` run found one compile-time mismatch in an existing stale-source test: it still
treated the shared publication result as a boolean. The assertion now requires `Superseded` for each
changed generation, revision and path, retaining the same rejection cases with the typed outcome.
The failed run did not execute the database regressions; their results require the corrected head.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Source verification application / port | 86 / 28 | 0 | 285 database/application integration |
| Source verification transaction | 132 | 0 | Shared verification scenarios |
| Exact fingerprint publication | 122 | 0 | Existing source-identity and publication suites |
| Shared gallery membership / organization publication | 129 / 71 | 0 | Existing batch, grouping and recovery suites |

## Compatible feature reuse

The native classification facade now exposes a separate SQLite feature cache and the description
preparation use case. Independent image/text identities and exact content/description keys allow
rule-only rematching without another image inference. Text preparation retains completed entries
after cancellation, releases its text encoder on success or failure, and preserves a warm image
encoder when all descriptions are already cached. No model is loaded by opening or cleaning the cache.

The cache uses the existing pinned-directory capability and held file identity. Initialization
publishes a complete temporary database without replacing another creator's file. Raw header
identity/version checks precede SQLite write access to an existing file. Integrity checks cover
the key and payload; normalized values are restored without a second normalization. Recency and
accounting update atomically, with indexed eviction and bounded cleanup under ADR 0034's limits.

Twelve added Rust scenarios cover restart and exact-vector preservation, independent encoder
invalidation, content/description changes, recency eviction, rollback after eviction, corrupt values,
bounded cleanup, unrelated/newer database preservation, Windows namespace lifetime, rule-only
rematching, interrupted description reuse, failed-description retirement and warm-image retention.
Their execution belongs to the exact-commit hosted gate. A local library Clippy attempt identified
integer-decoding and facade-export errors, which were corrected, but compilation then exhausted
memory; it supplies no passing Clippy result. No further full-root test build was attempted.

A focused SQLite check executed the production schema with `trusted_schema=OFF`: 260 insertions
accounted for 68,640 bytes, bounded cleanup left four entries, rollback restored all 260, and the
eviction query used `feature_recency`. Formatting and whitespace checks pass. This check covers SQL
accounting and query shape, not the Rust adapter or Windows file capability. The
[`cb3f7d53` hosted job](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41089127551/job/124178269076)
subsequently passed all twelve cache/description scenarios. That job still failed two exact module
loading assertions because the adapter roster omitted the new feature-cache module. The roster now
admits only its exact declaration and the new native-input declaration, retaining alternate-path,
generated-module and broader-visibility rejection. The corrected complete gate remains pending.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Feature-cache adapter / storage / schema | 272 / 179 / 26 | 0 | 322 |
| Description preparation / repository port | 76 / 41 | 0 | 307 |

## Guarded content execution

The feature-gated native entrypoint connects a leased content task to source admission, compatible
feature lookup, image preparation, encoder residency, bounded matching and transactional prediction
publication. It checks the stored run before source access, holds the file/namespace capability
through commit, and rechecks source/task authority after inference. A cache hit skips decode and
encoding. A failed catalog commit can retain the completed compatible feature without completing the
task or changing human decisions. Cancellation during prediction insertion rolls back the checkpoint.

Six database/application scenarios cover committed publication and lost acknowledgements, cache-only
matching with a held source, incompatible pipeline rejection before I/O, cancellation at decode and
inference, concurrent pause/source replacement, failed source validation, checkpoint rollback/reuse
and cancellation during the final transaction. Two real Windows source-capability cases cover Chinese
paths, wrong extensions, read-only file/namespace retention and exact-hash fallback when the revision
is uncertain. These cases await the new head's hosted execution; no local database pass is claimed.
The first integrated build exposed a missing test-type import, corrected in `3bb51c89`. That head's
Static job then rejected an unchecked `update_hook` result in the cancellation fixture. Registration
now must succeed before the fixture proceeds; the injected cancellation and rollback assertions remain.

Five decoder tests passed in a focused single-process probe in 0.13 seconds, with no compiler warnings
in the corrected probe. Its decoder and test files match the production files byte-for-byte. It covers
seven admitted formats, first-frame GIF behavior, RGB channel/alpha handling, EXIF rotation, invalid or
oversized inputs, and cancellation before/during I/O. It does not stand in for the Windows capability,
catalog transaction or actual model path. The initial restricted compile hit OS error 1455; a separate
serial probe used the repository lockfile offline after correcting its module/dependency setup.

Six synthetic PNG inputs compare native preprocessing with Pillow 12.3.0 bilinear resizing. The
224-by-224 input matches exactly; the other five differ by at most one 8-bit channel level, with mean
absolute differences from 0.07446 to 0.19791 levels. The image-pipeline revision now binds the native
RGB8/Triangle implementation, leaving text compatibility unchanged. This establishes a numerical
difference, not classification equivalence. The subsequent
[frozen-policy native-input comparison](r5-calibration.md#native-image-input-comparison) records
the retained cohort's result and decoder-coverage limits. The six synthetic probes read no original
media, download no dependencies or models, and modify no source media.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Content execution / analysis port | 163 / 48 | 0 | Database/application scenarios below |
| Native source capability / pixel preparation | 100 / 96 | 0 | 126 / 147 |
| Analysis publication transaction | 84 | 0 | 547 database/application integration |
| Existing local-file owner | 2,629 unchanged | 3,452; twelve exact-roster lines added | Existing availability suites plus adapter loading rejection |

## Shared background admission

Native exact verification and classification content execution now share one FIFO slot per file or
content task. Admission is released after the held source, including failed publication/revalidation.
A waiting classification task therefore precedes the duplicate scanner's next source; it does not
wait for the whole scan. Cancellation removes only the waiting request. The bounded queue rejects a
ninth waiter and never retires an active owner's source on another request's behalf.

The six focused checks pass in 0.07 seconds: FIFO handoff, cancellation while another task owns the
source, queue saturation/drain, pre-cancellation/error cleanup, source-before-slot retirement and an
additional reader failure probe. Five tests reside with the production owners. The diagnostic entry
uses byte-identical owner/test files with minimal domain/port fixture declarations; compiler warnings
are denied. It establishes these concurrency and lifetime rules, while the whole-repository types,
application entrypoints and retained R3 behavior remain covered by the current-head hosted gate.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Shared background admission / admitted exact reader | 94 / 58 | 0 | 116 / 52 |
| Content execution / source verification | 166 / 84 | 0 | Existing database/application suites |
| Existing exact-analysis facade | 220 | 0 | Existing exact-analysis suites |

The native dispatch connection is recorded below. Desktop enablement remains in R5-04; these native
entrypoints do not enable automatic analysis in the desktop application.

## Native dispatch lifecycle

The native runtime now composes the persisted work queue, guarded verification/content execution,
feature cache, compatible matcher preparation and encoder residency. Its signal owner distinguishes
cached browsing from actual foreground source reads, import, suspend and pressure. Cached browsing
defers initial admission without repeatedly cancelling an admitted queue. Manual pause remains a
catalog decision. Source-only or empty queues start no model. Cancellation returns unfinished work
without consuming the failure budget; model setup failure returns the active claim to the queue and
ends the loop. Runtime ownership prevents concurrent replacements within the application process.

An empty queue sleeps until a signal, retry or idle-retirement deadline. The ready-time query uses
six indexed lane/kind seeks. Retry timing starts after a failed attempt, including time spent in the
worker. Separate I/O and codec failures preserve unavailable-versus-invalid input handling; numerical
image preparation is unchanged.

Six signal tests and two runtime-ownership tests pass in a focused probe, together with two diagnostic
clock/admission checks, in under 0.01 seconds. Production signal/test and ownership files are
byte-identical to the probe inputs; its minimal error
type replaces only the unrelated domain fixture. Seven pixel-input tests pass in 0.13 seconds using
the existing offline probe, including the newly disconnected-source case. These checks read no source
media and download no models or dependencies.

The library passed feature-enabled Clippy with warnings denied. The initial restricted check emitted
an incremental-cache hard-link warning; the same repository-cache invocation outside that restriction
completed without it. The new database tests compiled, and a subsequent retry-clock assertion exposed
an unsupported unsigned SQLite fixture read. It now reads the actual signed SQLite integer type;
test-enabled Clippy subsequently passed, including the final ownership change in 20.86 seconds.
Seven database/dispatcher cases cover shared
prediction completion, completed-work reuse after restart, checkpointed pause/continue, foreground
yielding, retry exhaustion/deadlines, setup failure and empty-queue close. Their execution remains owned
by the current-head hosted gate, alongside native setup/ownership error handling and prior R5-03 suites.

The preceding `f703c445` run has passed Static, Rust Remaining, Windows Scan, Windows Accessibility
and Unsigned Release checks. Catalog, Schema and Queue, and other jobs were still running at the last
successful read; the complete R5-03 gate has not been declared passed. Initial status requests timed
out before the GitHub API resumed responding. No unchanged full-root test execution was repeated.

This delivery establishes the native loop and control boundary. The setup and arrival connection
is recorded below. Desktop lifecycle wiring, source-specific availability wakeups
and actual integrated model/foreground measurements retain their R5-04 scope. No UI acceptance,
automatic desktop classification or complete R5-04 result is claimed by these focused checks.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Dispatch loop / signal owner | 177 / 266 | 0 | 176 signal; 378 database/dispatcher |
| Native runtime / immutable setup / admitted text | 280 / 90 / 72 | 0 | 58 error-policy |
| Runtime ownership | 46 | 24 | 0 |
| Execution port / work repository / lease owner | 34 / 161 / 279 | 0 | Existing queue and new dispatcher suites |
| Pixel preparation | 101 | 0 | 199 |

## Stored setup and incremental admission

Schema 67 stores the complete immutable analysis setup and source-admission boundary. The native
runtime reconstructs both rule sets and the calibrated profile from the catalog before taking work;
missing legacy setup cannot silently select uncalibrated analysis. Activation commits its run,
profile and dispatch change atomically, with independent catalog/dispatch revisions. Retrying a
committed activation preserves a later pause, disable or stop. Existing decisions and portable
human archives retain their own ownership.

Arrival admission has its own durable cursor. New sources enter their priority lane before the old
catalog walk completes, while each turn still admits at most 64 sources and reserves capacity for
backlog. Pause/restart retain the boundary and both checkpoints. A replacement run captures a new
boundary. Availability wakeups and desktop publication signals remain separate integration work.

Focused offline checks pass for exact uniform/category-profile round trips, revision binding,
unknown-version rejection, payload bounds and missing refinement. They use the actual domain and
profile modules with bounded serialization/error fixture helpers; they do not substitute for the
complete catalog tests. Production migration SQL also passes isolated SQLite checks for retained
pause, arrival watermarks, invalid-watermark rejection and transactional DDL rollback. No source
media or models are read. The initial probe exposed an unused fixture dependency and its SQL reader's
handling of multiline literals; both probe issues are corrected. The cache hard-link restriction
was removed by using the established offline cache outside that restriction, without changing
compiler warning policy.

Test-enabled, classification-worker Clippy passes with warnings denied. Eight setup/runtime cases,
three arrival cases and three migration cases are included in the hosted catalog and schema suites.
They cover immutable restoration, explicit uncalibrated setup, replay/manual intent, replacement,
atomic failure, invalid profiles, paused native restart without a worker, early arrival admission,
fairness and schema preservation/rollback. Their full database execution remains pending the hosted
gate. Earlier `c96f7776` checks passed Static, Flutter, Synchronization and Scan, Windows Scan,
Windows Accessibility, Unsigned Release and the five synthetic workloads; Remaining, Catalog and
Schema and Queue were still running at the last read.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Immutable setup / setup port | 66 / 20 | 0 | Setup cases below |
| Setup persistence / profile codec | 152 / 126 | 0 | 379 setup/runtime |
| Source admission / dispatch control / queue lookup | 121 / 270 / 184 | 0 | 158 arrivals plus retained work suites |
| Setup migration | 83 | 14 downgrade-fixture support | 180 migration plus retained historical preservation |
| Native runtime / matcher preparation | 288 / 46 | 0 | Existing error policy and setup restart case |

## Unavailable-copy selection and scoped recovery

Schema 68 retains negative observations for individual physical copies. Claims try another current
copy without consuming the transient retry budget; a later verified copy wakes the existing content
task. Source and root recovery preserve manual intent and permanent failures. Root pages are limited
to 64 observations and a fixed failure-identity boundary; later failures survive an earlier recovery.
Removing a root retires its observations and schedules affected waiting work for membership
revalidation. The native control owner connects these transactions to the existing dispatch signals.

Eleven catalog/migration regressions compile with the classification-worker feature and warnings
denied. They cover alternate and late copies, hard-link aliases, paused restart, source/root guards,
transaction rollback, bounded recovery, post-boundary failures, stop/disable/permanent failure,
removal and legacy migration with retained Favorites. Full execution belongs to the current-head
hosted catalog/schema gates. A focused SQLite probe using the production SQL passes migration
rollback, retained pause/retry budget, copy selection, scoped recovery, monotonic failure identities,
source guards and removal cascades. It uses only in-memory fixture tables and does not establish
desktop or integrated model acceptance. The initial compile found two callers of the moved source
queries and two unsigned SQLite fixture parameters; both are corrected without suppressions.

The committed late-generation handoff and desktop lifecycle wiring are recorded below. These native
recovery methods do not themselves enable classification in the desktop UI.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Availability recovery / source selection | 220 / 219 | 0 | 299 source; 240 root |
| Queue admission / completion / leases | 147 / 156 / 280 | 0 | Retained work suites and availability cases |
| Native runtime / control | 250 / 90 | 0 | Retained runtime suites |
| Availability migration | 83 | 12 downgrade-fixture support | 159 |

## Committed scan-publication admission

Schema 69 connects scan publication to durable, coalesced root receipts. The background dispatcher
pages only source-generation ranges already passed by its ordinary admission cursors. Late first
imports and staged arrivals therefore enter the new-content lane after publication, within the
existing 64-source turn bound. The receipt and work checkpoint commit atomically; unchanged work
keeps its priority and failure state. Root removal, namespace replacement, pause and restart retain
their existing authority. A covering root/scan/source-generation index avoids a whole-catalog sort
or scan for each receipt page. Its integrated storage and scan costs remain covered by hosted workloads.

Nine new catalog/migration regressions compile with the classification-worker feature and warnings
denied. They cover late backlog and arrival publication, bounded paused restart, publication and
cursor rollback, source removal while stopped, unchanged-work priority, migration preservation and
rollback, and indexed paging. The historical all-row migration assertion also accounts for the new
empty table and sequence. Test-enabled Clippy passed in 13.32 seconds after the final assertion update;
full regression execution remains with the current-head hosted gate.

A focused in-memory SQLite probe using production SQL passes migration rollback, receipt/gallery
transaction rollback, bounded indexed pages, cursor/work rollback, retained pause, receipt replacement
and removal. This checks the changed SQL rather than establishing a desktop or real-model result.
No model artifacts or original media were read. The previously recorded root-test compilation memory
failure was not retried.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Published-source admission | 214 | 0 | 271 |
| Publication migration | 80 | 14 downgrade-fixture support | 85 |
| Source admission / dispatch control | 123 / 271 | 0 | Retained arrival and work suites |
| Scan publication transaction | 245 | 0 | Retained scan-publication tests plus new rollback case |

## Desktop lifecycle wiring

Desktop bootstrap now owns a classification lifecycle alongside exact analysis and synchronization.
The native session restores only automatic or manually paused dispatch, keeps initial admission
suspended until current desktop activity arrives, and runs on the bridge executor. Foreground reads,
import and committed catalog changes reuse the existing activity owners. Available-root generations
are coalesced and recovered serially. Shutdown signals cancellation, joins the running call and
availability recovery, then releases the session. A handle returned after closing began is retired
without entering the dispatcher.

Local installation selection verifies both pinned encoder artifacts and the runtime before saving
their directories in a separate schema-1 settings database. It does not enable dispatch. The default
Windows build includes the supervised worker executable; models and runtime DLLs remain separate.
Portable and installer checks now require that executable, and installer fixture checks retain file,
component and ACL ownership. No source-media files or real catalogs were used for these checks.

The generated bridge and Rust Release library build pass (4 minutes 26 seconds for the Cargo build).
Test-enabled classification-worker Clippy passes with warnings denied. Four added native cases compile
and await hosted execution: absent installation, settings persistence/version rejection, paused
startup, close-before-run and joined native shutdown. Six focused Flutter lifecycle cases pass,
covering late-open close, unconfigured startup, latest foreground state, task/recovery joining,
catalog/root coalescing and memory-pressure cooldown. Analysis of the five affected Dart entrypoints
passes after correcting one constructor-style diagnostic. The compiler-free installer payload check
and `git diff --check` pass. The unsigned payload and portable archive fixtures pass with the added
worker; the unsigned check initially required sandbox access to create its isolated rejection-case
junction. Bridge verification covers 21 asynchronous methods with matching generated hashes; its
roster assertion was updated for the four new classification contracts without changing rejection
rules.

These results establish the wiring and its boundary behavior, not the completed R5-04 client workflow.
Model-backed desktop foreground/resume measurements and operating-system power suspension remain
integration work. The existing Flutter paused/detached lifecycle signal is connected; focus loss and
minimization keep background classification eligible. A one-minute memory-pressure cooldown is an
integration candidate awaiting resource measurement. Model enablement and configuration controls retain
their R5-06 presentation scope.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Native desktop session / typed bridge facade | 181 / 144 | 0 | 137 |
| Local installation store | 91 | 0 | 47 |
| Desktop lifecycle owner | 192 | 0 | 218 |

## Classification configuration and activation

Typed application/bridge entrypoints now expose the saved configuration, editable initial categories,
explicit enablement and pause/continue/disable/stop. Selecting local artifacts still verifies them
without enabling classification. Saving categories while disabled needs no installed model. The
catalog transaction owns the definition edit, immutable replacement setup and dispatch transition;
stale revisions or failed activation roll back the whole edit. Save preserves paused/stopped intent,
while replay preserves any later user transition. A display rename keeps the existing run and cursor.
No catalog migration or new dependency is required.

The initial screening candidate contains the retained primary and purpose-aware refinement
descriptions, category thresholds and calibration identity. Its model identities are frozen to the
native-input candidate already measured in the calibration record. Exact definitions select that
profile at explicit enablement; semantic/category/model changes retain usable uncalibrated analysis.
Names can change without invalidation. This supplies an editable starting scheme without generating
additional categories or treating unsupported confidence bands as calibrated.

Desktop configuration joins opening, execution, availability recovery and handle retirement before
saving. The native runtime ownership gate independently fences mutation. A failed write restores the
prior runnable configuration. The replacement's opening settles before the controller reloads its
state, including the dispatch revision advanced by opening. Closing joins an already executing write
and prevents another session from starting.

Ten added native cases compile under test-enabled Clippy with warnings denied (34.63 seconds).
They cover saved-but-disabled definitions, explicit enablement, paused semantic replacement and
reopening, display-only preservation, replay/stop, stale control authority, transactional rollback,
model/description-specific calibration and missing installation. Complete execution remains with
hosted Catalog/Remaining gates. The generated bridge and Rust Release build pass (2 minutes
39 seconds); the 25 asynchronous contracts and their rejecting guardrails pass. Dart analysis of the
changed composition, configuration and lifecycle files passes. All 12 focused lifecycle tests pass,
including six new configuration/opening/closing races. Initial diagnostic runs exposed an unnecessary
timer turn and a test assertion inside a guarded asynchronous callback; the command now schedules a
microtask and the test observes callback state before asserting it outside the pump.

The preceding `25562604` Windows Accessibility job failed before its application-ready assertions:
the UIA probe exceeded its eight-second parent deadline while its last progress record identified
system assembly loading. Its cause was not established by that run; later gate results are recorded
below separately from classification-function verification.
Its Rust Remaining job completed 1,037 cases successfully, with two source-contract failures caused
by the missing exact declaration for the new classification installation module. The loading
contract now records that declaration and the configuration domain module, with rejection cases
retaining the visibility, platform and alternate-loading boundaries. Execution of the corrected
contract remains with the next hosted gate.
At this checkpoint, the real-model client run, operating-system suspend/resume and integrated
resource measurements remained outstanding. The following sections record the subsequent client
coverage. Controls retain their R5-06 presentation scope, and these focused checks change no originals.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Configuration application / catalog transaction | 149 / 137 | 0 | 55 / 234 |
| Version-bound screening candidate | 130 | 0 | 136 |
| Desktop lifecycle | 247 | 0 | 362 |
| Local-files availability contract | 2,629 | 3,464 | 532 (module topology) |

## Real-model Windows client verification

The configuration path now records the existing `configure` operation kind accepted by the durable
operation schema. The first implementation used a new unchecked string and would fail the database
constraint. The feature-cache composition now passes an explicitly named directory, keeping the
adapter's fixed SQLite filename inside it; the earlier composition named that directory as a file.
Neither repair changes catalog schema, source files or the admitted model.

The opt-in `integration_test_classification.ps1` runs the actual Debug desktop executable and its
packaged worker with the retained SigLIP 2 FP16-image/INT8-text installation. It starts the same
production synchronization owner required for first-import continuity, then imports three existing
small synthetic files with two distinct contents. Configuration goes through the production Dart
controller, generated bridge and native persistence/runtime. A bounded read-only observer inspects
only the GUID-isolated derived databases; all three source files and their bytes remain unchanged.

The complete client case passes in 62 seconds after a 53.7-second incremental build. It verifies:

- Saving categories before installation retains disabled mode; installation selection does not
  enable classification.
- Holding foreground source-read ownership for four seconds prevents both predictions and feature
  creation, exceeding the configured initial quiet interval.
- Manual pause survives joined session closure and reopening; releasing foreground ownership does
  not resume that pause. Explicit continuation publishes two predictions for the three files.
- Display-only renaming preserves the run, both predictions and all cached features. Changing one
  description creates a new immutable run and two compatible predictions, retaining the original
  50 feature values and adding one text feature. No work issues remain.
- Stop survives joined closure and reopening without producing another prediction. The complete
  test closes its synchronization/classification owners and the runner retires its owned Job Object.

The observed waits from continuation/new-run opening to complete publication are 24,652 ms and
11,539 ms. They include quiet admission, encoder startup, description preparation and observation
intervals; they are not per-image throughput or a low-resource benchmark. Feature retention is
observed directly; the existing encoder-call regressions own the assertion that rule-only rematching
does not invoke image inference. This case does not establish image classification accuracy.

The first harness run omitted synchronization startup and reached the first-import continuity
deadline; the second inspected the cache directory as a database file. Both were corrected before
the complete pass. Final test-enabled Clippy passes with warnings denied in 13.53 seconds, and Dart
analysis plus PowerShell/Python syntax checks pass. The separate `48e85d93` hosted Windows
Accessibility and Windows Scan jobs pass; this does not retroactively explain the earlier UIA timeout.
The following native power-event verification extends this client case. Integrated constrained-device
resource/foreground measurements and the visible classification controls retain their R5-04/R5-06 scope.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Native runtime / desktop composition | 258 / 183 | 0 | 58 / 137 |
| Atomic configuration | 132 | 0 | 234 |
| Local client runner / read-only observer | 76 / 54 | 0 | 196 + 98 (client and support) |

## Windows power events and pressure cooldown

The HWND adapter registers native suspend/resume events and supplies revisioned state to the desktop
environment. That environment owns combined framework/system inhibition and event admission;
`LibraryAnalysisActivity` retains only catalog and foreground-input composition. Existing native
dispatch retains cancellation, model retirement, source revalidation and durable manual intent.
There is no schema, model, source-media or third-party dependency change in this connection.

Six focused environment tests pass: a late initial snapshot, focus/minimize behavior, application
pause overlapping system resume, a missing suspend notification, disposal, and registration/protocol
failure. The existing 12 classification lifecycle tests also pass. Final Dart analysis and test-enabled
Clippy pass with warnings denied; Clippy takes 15.47 seconds. The actual Debug runner compiles with
the same adapter included in the engine-free and real-engine retirement targets.

The expanded real-model client case passes in 137 seconds after a 20.0-second incremental build.
It sends power events only to its own verified Debug HWND, without physically suspending the machine:

- Suspend/resume while manually paused retains pause and starts no worker or prediction.
- The loaded text worker has already published reusable features when interrupted. Its observed
  working set is 401,215,488 bytes, private memory 386,650,112 bytes and peak working set 715,886,592
  bytes. These are one worker observation on local-primary, not a full-run memory or cloud benchmark.
- The suspend message call takes 171 ms and the subsequent no-worker observation completes in
  172 ms. The combined check passes its two-second bound. Four seconds later, no worker exists and
  neither predictions nor features have advanced.
- Resume completes both distinct contents and preserves every feature committed before suspension.
  The 23,826-ms completion wait includes quiet admission, remaining preparation and model work.
- A framework memory-pressure callback holds new work across description configuration replacement.
  At 55 seconds, no worker or replacement prediction exists. The edited run completes
  71,072 ms after the pressure event, preserving the one-minute hold followed by quiet admission.
  Image-worker retirement under pressure is not separately measured by this case: display-only
  reconfiguration had already retired that resident process.
- The prior duplicate sharing, display-name preservation, rematching, stop/reopen and unchanged
  three-file source assertions remain in the complete passing case.

The first native event run timed out while observing workers, before producing observer diagnostics.
A bounded observer with owned-process retirement and delayed stack capture passed the same boundary;
that timeout's cause remains unestablished. A second pass covered interruption during process startup;
the final case requires feature production before interruption and adds pressure cooldown coverage.
The native-event path is verified; physical sleep and integrated constrained-device browsing/resource
behavior are not established by synthetic window messages.

The `67c0d52a` Schema and Queue gate passed 388 cases and failed one obsolete version assertion.
The availability migration correctly reached 68, and production reopen advanced it to 69 before
the assertion. Its test now checks 68 immediately after that historical migration and the canonical
current version after reopening, retaining pause, Favorite and queued-work assertions. Full native
execution of that adjusted assertion belongs to the next exact-head hosted gate.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Windows power channel, including header | 102 | 0 | Covered by native client and runner gates |
| Desktop environment | 138 | 0 | 153 |
| Classification lifecycle / activity composition | 250 / 138 | 0 | 362 (lifecycle) |
| Real client / client support / Windows probe | 0 | 0 | 236 / 165 / 163 |
| Availability migration regression | Unchanged | 0 | 164 |

## Integrated local foreground comparison

The standalone Profile client uses product head `3002ad1f`, the installed SDK, the optimized native
library with the existing isolated-storage assertion switch, and the production classification
worker. It composes the production synchronization owner, classification lifecycle, Windows power
adapter, foreground-read signals, `AmeApp`, photo wall and original-image viewer. It does not substitute
blank previews or a mocked catalog. The native storage path is checked before fixture admission.

The reusable measurement set contains 48 distinct files selected from the established synthetic
mixed corpus, totaling 47,786,849 bytes. Fifteen exceed the existing 16,777,216-pixel classification
input bound; the expected complete result is 33 predictions plus 15 `classification_image_limit`
outcomes. All 48 remain browsable. Those skipped images are an input-coverage limitation, not model
classification results. Original fixture hashes are unchanged after every attempt.

On local-primary, the final complete run lasts 51.664 seconds. First prediction follows enablement
after 9,109 ms, including quiet admission and initial text/image setup. Three scroll/viewer rounds
advance prediction count from one to five; pause retires the worker, and resume finishes the admitted
contents. Enabled wall time is 44,105 ms, including those interactions, deliberate quiet intervals,
pause and observation. This interval is not steady-state inference throughput.

| Measured interval | Scroll wall interval | Viewer wall interval | Raster P95, scroll / viewer |
| --- | ---: | ---: | ---: |
| Disabled, after warm-up | 713.086 ms | 304.525 ms | 1.165 / 1.062 ms |
| Automatic, round 1 | 711.575 ms | 295.391 ms | 0.961 / 0.873 ms |
| Automatic, round 2 | 714.328 ms | 300.869 ms | 1.305 / 1.032 ms |
| Automatic, round 3 | 711.293 ms | 300.763 ms | 1.010 / 0.862 ms |
| Paused, after automatic work | 712.659 ms | 294.953 ms | 0.763 / 0.843 ms |

Scroll intervals include two 350-ms settling waits; viewer intervals include 250 ms after the full
image becomes available. They are paired workflow intervals, not exact input-to-image latency.
No measured interval has a raster frame above 50 ms. Active-interval maximum raster duration is
6.926 ms and maximum build P95 is 2.780 ms. These are local-primary results at a 1,582 × 853 physical
surface with 1.25 scale; they do not establish cloud-primary rendering behavior or remove the
separately deferred first-use rendering work.

At 200-ms process sampling, peak combined client/worker working set is 1,121,148,928 bytes
(about 1.044 GiB). Separate client and worker peaks are 257,310,720 and 870,576,128 bytes; separate
peaks need not occur at the combined peak. Sampled CPU totals are 22.797 client seconds and 18.531
worker seconds. At most one model worker is resident, the gallery retains 48 assets for this bounded
fixture, and client retirement leaves no model worker. These are process observations, not a whole
machine memory limit or large-library query benchmark.

The first sampler attempt ended with an unlocated Windows access error. A second attempt preserved
a traceback showing replacement of a progress file while Dart held a reader. The sampler now appends
complete records and readers ignore an unfinished trailing record; native process handles are also
retained for sampling across retirement. A third attempt completed admitted classification but its
old all-files-success condition waited for the 15 explicitly rejected large inputs; that owned
client was stopped. The final assertions distinguish the frozen fixture's 33 supported and 15
limited inputs and pass, alongside real foreground interactions. No product guard was relaxed.

Diagnostic sources, the final `local-04` reports and earlier attempt records remain under ignored
`build/test/derived/r5-classification-measure`. The four retired attempt profiles are removed after
checking owned-process retirement and reparse-free contained paths, reclaiming 17,509,972 bytes;
the fixture, reports and a verified 36,020,546-byte transfer package remain. Cloud-primary capacity inspection confirms two logical
processors, 8,536,289,280 physical-memory bytes and 11,405,623,296 free-storage bytes at inspection.
Its existing model installation remains unchanged. File-transfer UI approval timed out before any
package was sent, so the paired cloud measurement remains open.

The exact-head hosted run [41132041240](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41132041240)
finishes with 12 passing and two failing applicable jobs. Schema and Queue now passes the adjusted
historical/current migration assertion. Catalog passes 795 cases, fails five archive cases and ignores
seven designated cases; failures concern stale validated-session authority and archive version/count/
cancellation expectations. Windows Accessibility reaches native-semantics readiness, then its
application-ready probe exceeds the eight-second parent deadline; final progress records 5,130 ms
loading UIAutomationClient and 1,395 ms locating the window, followed by element lookup. Cleanup also
fails to read the process log while it remains open. These failures retain their own repairs; this
performance comparison does not establish a passing complete hosted gate.

## Native verification process reuse and output retirement

The failed application-ready probe reloaded UIAutomationClient for 5,130 ms after a separate
preparation process had already exited. The archive-only head `efb3f7d2` passes the unchanged native
gate in [run 41156326620](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41156326620): its
application-ready phase takes 2,018 ms, including 752 ms loading UIAutomationClient. This establishes
variable loading cost rather than a consistently failing native assertion; the underlying cause of
the unusually slow binder call remains unestablished.

The canary now retains one owned MTA process after preparation. Each application phase keeps its
eight-second parent deadline, fresh sequenced request, invocation-return event and validated evidence.
An already-written complete record cannot acknowledge an invocation that is still blocked. Invalid
results and timeouts retire the session; whole-run success also requires session retirement. The
ten existing native assertions are unchanged. Output capture obtains exclusive log access after
closing the application job, waiting only within the remaining five-second cleanup budget for
asynchronous writer release. A persistent lock remains a failure with retained raw evidence.

The complete accessibility guardrail entrypoint passes locally. Its real retained-process loader
case measures 61/9 ms across two invocations, and the new cases reject wrong ownership, incomplete
results, assertion failures and a blocked invocation. Request validation rejects stale sequence,
unknown phase, invalid process identity and escaping result paths. Output checks verify delayed
writer completion, bounded lock failure and unchanged evidence. PowerShell syntax and diff whitespace
checks pass. At `1590e2be`, the native Windows job in
[run 41157554079](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41157554079) passes all
ten interaction phases using the same process as preparation. Application-ready takes 3,095 ms,
including 1,267 ms in its client-loading stage; subsequent assembly-loading stages take 1–3 ms.
All phases remain within their parent deadline. The complete run has no assertion or cleanup error,
confirms application exit/job closure, and removes its owned scratch storage. This result verifies
the new session in hosted native interaction without claiming that every first-use cost disappeared.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Probe session / protocol / worker | 145 / 53 / 38 | 0 | 132 |
| Output retirement | 29 | 0 | 58 |
| Existing cleanup / canary composition | 159 / 399 | 0 | Existing guardrail entrypoint: 503 |
