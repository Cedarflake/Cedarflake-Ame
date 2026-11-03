# Project organization verification

The [roadmap](../roadmap.md#project-organization-after-r5) owns activation and exit decisions.
The [execution plan](../plans/project-organization.md) owns O1–O7 and
[ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md) owns the target rules.

## Entry verification — 2026-11-03

The implementation branch is `codex/project-organization`, created from merged R5 baseline
`f74146a4`. R5 product behavior, retained catalogs and original media remain unchanged during entry
verification. Product module migration has not begun.

The final R5 [workflow run 41602861678](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41602861678)
reports a Windows Scan failure. Its diagnostic artifact records two passing workflows, a native
picker cancellation failure (`Cancel button was not found`), and a subsequent controlled import
remaining in `choosingDirectory`. The original script did not distinguish an absent dialog from an
unready button, and acknowledged a delivered click without observing dialog closure. These logs do
not establish why the first dialog action timed out.

On local-primary, the first diagnostic invocation stopped before native automation because the
Windows PowerShell helper lacked a process-scoped execution policy. A second invocation, with the
policy scoped to that test process and restored afterward, passed all four existing scan workflows
in 67,054 ms, including preparation and owned process retirement. The CI timeout was not reproduced.

### Native picker verification boundary

`integration_test/support/NativeDirectoryPicker.cs` owns one observed dialog's action and retirement.
The helper waits for a visible, enabled button, bounds native message delivery, and acknowledges
success only after the same process-owned dialog is destroyed. Failure retains its original stage
and attempts bounded cancellation of that observed dialog. An absent or ambiguous dialog grants no
cleanup authority over another window. The operation retains its 12-second deadline and has a
separate two-second failure cleanup bound inside the existing 15-second caller deadline.

`control_native_directory.ps1` compiles this owner before readiness and reports the action stage,
elapsed time and cleanup result. Its Dart launcher scopes execution-policy admission to the helper
process, following the existing native accessibility helper convention. It does not change machine
policy. No UI Automation tree is opened, preserving the Flutter semantics-ownership assertion.

The eight focused cases cover delayed readiness and closure for both actions, an ignored
confirmation, failed delivery with failed cleanup, a missing dialog, an unready button, closure
before the requested action and original-error preservation. They pass in both PowerShell 7 and the
helper's actual Windows PowerShell host. The existing Static gate runs the same regression owner;
the real Windows Scan gate retains all existing assertions.

The changed real Windows Scan workflow passes all four tests on local-primary in 67,965 ms,
including preparation and owned process retirement. Run `933f611439a54ce89cfde25904b5a55e` reports
exit code zero and no cleanup failures. It covers native cancellation, controlled directory
selection, scanning, preview/source checks and the retained organization assertions. The helper now
uses its own process policy; this run required no caller environment adjustment.

The support action/native owner has 179 implementation lines, no inline tests and 101 dedicated
test lines. The PowerShell wrapper has 34 lines, its Dart process launcher 81 and the focused gate
14. These are test infrastructure, not additional product orchestration.

Submitted-commit CI remains pending. This verification repair does not establish O1–O7 completion
or a passing final R5 hosted result; the original CI timeout remains distinct from the reproduced
false-success protocol defect and local script-startup failure.

## O1 Dart dependency foundation — 2026-11-03

The checker inventories 417 Dart modules: 362 handwritten modules and 55 explicitly named FRB
outputs. Its baseline retains 124 exact migration entries (46 cross-feature, 69 layer, seven
external-library, one shared-feature and one cycle), with named responsibility, reason, regression
owners and O2–O6 removal conditions. The baseline Git blob is pinned independently of the mutable
active-ID policy; an arbitrary fresh violation list cannot replace it. The source/target and
physical-size inventory is in the [execution inventory](../plans/project-organization-inventory.md).

The 17 focused regressions pass through `quality_test_flutter.ps1`. They cover admitted layers,
multiline/raw/package URI resolution, all conditional branches, transitive exports, parts, complete
cycles, explicit cross-feature contracts, shared visuals, desktop contract admission, exact generated
outputs, invalid syntax, new violations, stale/duplicate exceptions and immutable baseline identity.
Scoped analysis reports no issues; the repository dependency command passes against the current
124 active exceptions. These checks add no product behavior, catalog/schema or runtime changes.

`analyzer` remains at its existing locked version 10.2.0 and hash; only its dependency classification
changes from transitive to direct development use. The existing Static/Flutter workflow owners run
the submitted checks. By current user direction, their pending results do not hold subsequent work.
The Rust portion of O1 is recorded below.

## O1 Rust dependency foundation — 2026-11-03

The Syn-based check covers the existing native domain, ports, application and bridge API modules.
It resolves explicit references through grouped imports, renames, re-exports, module globs and
block-local aliases, including macro paths. Test-only declarations are excluded without excluding
potential production platform/feature branches. Source, owning function/type, resolved target and
rule are reported together. Existing locked Syn/quote dependencies are reused without a Cargo
manifest or lockfile change.

The fixed R5 baseline has 279 entries: 26 named catalog-session, synchronization and classification
composition edges; the held source-handle contract in `PreviewStore::materialize`; and 252 individually
recorded application integration edges for O7 review. Domain/API contain no remaining rejected
explicit references. Existing API `StreamSink` use stays inside the bridge boundary. The
[execution inventory](../plans/project-organization-inventory.md#rust-composition-and-boundary-inventory)
names the retained construction/resource owners and their lifetimes.

The original inventory attempt exposed a parser recursion defect when a module and its re-exported
function shared a name. Resolution now distinguishes a module path prefix from a value export and
tracks alias cycles; the same-name regression passes. The repository scan then completed normally.
The gate pins the baseline blob and rejects new symbols even inside an admitted source file, missing
responsibility metadata, stale/duplicate/unknown IDs and directory-wide exceptions.

All 16 focused tests pass, including the current repository inventory; scoped Clippy for the test
and inventory example passes with warnings denied. The parser has 311 reference-walking lines,
246 name-resolution lines and 184 source-loading lines; policy comparison has 106 lines. These
owners have no inline tests and share 391 dedicated parser, policy and repository regression lines.
The check is part of the existing Rust `other` partition; hosted execution continues independently
under user direction.
This slice changes verification infrastructure and its ownership inventory, with no product,
schema, generated bridge or original-media changes.

## O2 Platform and storage — 2026-11-03

Directory selection, Explorer/Clipboard actions, storage settings, archive pickers and preference
persistence now have separate ports, adapters and feature bootstrap modules. Preference values are
pure domain models. Provider declarations retain their identities and require production bindings;
the previous in-memory preference fallback is test-only. Storage mapping, native Explorer behavior
and shared-preference implementation bodies are unchanged. Reclamation, snapshot scheduling and
configuration projection move together into storage application without changing their bodies.

One root Provider container is validated before publication. Its lifetime owner registers before
background resources, so shutdown joins background retirement before disposing the container.
Acquisition failure joins already registered resources; failed provider validation disposes its
unpublished container. The failure surface retains its window controls, and failed acquisition
does not start the abandoned background owners. Cleanup errors retain the original acquisition cause.

All 37 O2 migration entries are retired, together with the shared-theme entry closed by placing
preference-to-theme translation in Settings presentation. There are 86 active legacy Dart entries
for subsequent slices and 28 exact public cross-feature contracts. The immutable baseline is unchanged.
The dependency command passes for 435 Dart modules. Source, test and integration-import analysis passes.

There are 121 passing focused Dart tests across ten files: six production-assembly/lifetime cases,
56 library-controller cases, five preference-persistence cases, 15 Settings cases, one view-preference
case, one Explorer boundary case, two storage-picker cases, 23 reclamation cases, seven snapshot
coordination cases and five application-entry/theme cases. Production binding tests omit each required
port in turn, verify one shared preference store, reject duplicate/closed assembly, and cover joined
failure cleanup and background-before-container retirement. Existing test scopes and containers use
the production platform binding functions with an explicit test-only preference store; scenario
overrides retain their original precedence.

The canonical Windows Scan workflow passes all four cases on local-primary. Run
`4a4d9c788d434930bcca394f5b227f89` takes 96,504 ms including preparation and process retirement,
reports exit code zero and no cleanup failures, and has empty stderr. It covers native cancellation,
controlled selection, scan/catalog/preview/source handling and the retained organization interactions.
The isolated native evidence is retained by its existing execution owner. No real-library source,
user catalog, schema, generated bridge, package version or model changes are included.

The affected large owners have these implementation / inline-test / dedicated-test line counts:
shared preferences 243 / 0 / 193; reclamation controller 385 / 0 / 905; snapshot coordinator
124 / 0 / 146. The storage bridge adapter has 296 implementation lines. Provider assembly has
71 implementation lines and 237 dedicated regression lines; `main.dart` has 189 lines. Existing
assertions and asynchronous authority remain with their original owners. Hosted checks continue
independently; O3 does not wait for them.

## O3 Catalog and image reads — 2026-11-03

Catalog/folder/query contracts, layout-manifest contracts, previews and source-reader leases are
separate from their native implementations and mappings. Startup and subsequent catalog/folder
reads share the same selected catalog. The shared asset/query mapping closure includes native scan,
duplicate-review, organization, batch and review implementations; their existing bodies and ports
move together, with required bindings in their named capability modules. Their remaining runtime
work stays in O4/O5. Native bodies match the prior implementation after import/format changes; the
preview/source constructors additionally receive the foreground activity port instead of selecting
the application counter themselves. Native request/result/error and prepared-handle behavior remains
unchanged. No compatibility exports expose bridge mappings from application files.

The source-scheduler Provider owns one scheduler per root container, shared by child scopes.
Container retirement cancels pending intent and rejects further requests. An acquired buffer lease
stays with its image-copy owner until the copy finishes; retirement during native admission waits
for the resulting lease to close before settling. Widgets resolve the application dependency or an
explicit inherited override. The source image has no concrete native fallback. Preview/source reads
share the existing foreground activity counter through its port.

All 19 O3 legacy entries and 16 dependencies in the shared-mapper closure are retired. The immutable
Dart inventory is unchanged. The dependency gate passes for 458 modules with 51 remaining migration
entries and 28 public cross-feature contracts. Source, test and integration-entry analysis is clean.

All 147 focused tests across 16 files pass: two production-roster tests, eight native-mapping fixture
tests, 56 controller cases, three layout-manifest cases, 17 preview-coordinator cases, nine scheduler
cases and 52 viewer/query/timeline/preview integration-widget cases. Required-binding tests omit each
of the ten migrated ports in turn. Scheduler regressions verify parent/child identity, container
retirement, pending cancellation and lease retention both during acquisition and during buffer use.
Existing assertions for stale results, source-generation identity, preview recovery and bounded
navigation remain in place. Four mapper test files now mirror their adapter owners; test roots use
the same production binding modules before applying explicit scenario overrides.

The canonical Windows Scan workflow passes all four cases on local-primary. Run
`bda6b24015f54a958bf8f24902da42b6` takes 70,568 ms, reports exit code zero, no cleanup failures and empty
stderr. Its controlled workflow exercises native folder selection, scan/query/preview publication,
held source leases, original-image fallback/retry and organization interactions. The canonical
execution owner retains the isolated evidence. No original-media, user-catalog, schema, generated
bridge, dependency-version or model changes are included. Hosted execution proceeds independently.

Large native owners have implementation / inline-test line counts of 400 / 0 for catalog,
291 / 0 for scanning and 294 / 0 for organization; the shared native workflow entry has 511 test lines.
The application manifest loader has 111 / 0 / 199 implementation / inline-test / dedicated-test lines.
The source scheduler has 140 / 0 / 269; source-image composition has 136 implementation lines, with
630 source-lifecycle and 449 viewer-image test lines. The production-roster regression has 116 lines.
Existing request, publication and copy-completion owners remain intact.

## O4 recovery-status repair — 2026-11-03

Failure retention reconstructed a root status without its current `recoveryBlocked` value. Native
status can report an active full scan while exhausted retries still block recovery; the projection
then changed that flag to its default `false`. It now preserves the incoming native authority while
retaining the prior error presentation. A later native `false` clears the flag, rather than retaining
an obsolete blocked value. The focused regression failed with expected `true` / actual `false`
before the repair. All four retention tests pass afterward, including first-import recovery and
root-generation isolation. This independent repair precedes the O4 structural migration.

## O4 Scanning and synchronization — 2026-11-03

The scanner port and native mapping moved with the O3 shared-mapper closure. The synchronization
service now separates its Ame lifecycle contract, native gateway, snapshot mapping, phase-age
projection and diagnostics. `LibrarySynchronizationController` retains the existing timer, retry,
ticket/fence, generation, stop/dispose and publication flow. The lifecycle method bodies match the
prior implementation under their explicit gateway, projection, error-type and diagnostic substitutions.
Native DTOs and bridge failures stay inside the adapter; Ame failures preserve native codes/messages
or the established unknown-error code. Phase-age retention preserves current native recovery authority.

One production binding factory creates the selected gateway, application controller and existing
bootstrap lifecycle owner. The same service enters the Provider and background start/close path.
Missing bindings fail before application publication; the inactive fixture lives in test support.
The generated 250 ms cadence is unchanged, with tuning available only through test-only constructors.
No Rust, bridge, schema, dependency-version or original-media changes are included.

All 102 focused tests across ten files pass: two startup-orchestrator, two binding, three lifecycle,
two native-gateway, 15 primary-scan control, ten scan-restoration, four failure-retention, 32
synchronization, 23 feedback and nine revision-refresh cases. Coverage includes actual timer cadence,
one shared startup service, omitted binding, native ticket/fence forwarding, typed failure mapping,
non-overlapping polls, stop during startup/retry, stale completions, repeat close, retained phase age,
native recovery-flag changes and existing scan restoration. Source, test and integration-entry
analysis is clean. The dependency command passes for 465 modules; all five remaining O4 entries are
retired, leaving 46 later-slice entries and 28 public cross-feature contracts. The baseline is unchanged.

The canonical Windows Scan workflow passes all four cases on local-primary with the production
synchronization factory. Run `5ae02af2499f40fda43b7eeabea52d62` takes 93,137 ms, exits zero and reports
no cleanup failures; stderr is empty. Native startup, active scanning, subsequent synchronization,
preview/source/organization interactions and process retirement pass. Its isolated evidence remains
with the canonical execution owner. Hosted execution continues independently under user direction.

Implementation / inline-test line counts are 461 / 0 for the polling controller, 97 / 0 for
diagnostics, 42 / 0 for phase projection, 118 / 0 for the gateway and 132 / 0 for native mapping.
The retained application regression has 1,349 lines; gateway regressions have 134, production binding
regressions 73 and bootstrap-lifecycle regressions 161. The bootstrap lifetime implementation remains
97 lines, with only its port import changed.

## O5.1 Exact analysis and duplicates — 2026-11-03

Exact-analysis/query and gallery-annotation contracts now live in application ports; native calls
and DTO conversion live with duplicate review under Rust adapters. Native failures are translated
to Ame code/message values, including synchronous admission and stream errors. Annotation batching,
retired-window rejection, analysis cancellation/publication and foreground preemption remain with
their existing application owners. Controller and scheduler class bodies are unchanged; query
request/mapping bodies match after their explicit failure-translation wrapper.

The duplicate binding module shares one exact service between queries and the production analysis
controller/scheduler pair. Its idempotent close joins existing shutdown and disposes both notifiers.
Missing required ports or runtime bindings fail during assembly. Test-only assembly explicitly
selects inactive scheduling and permits scenario overrides through the same Provider identities.
The immutable inventory is unchanged; D072, D073 and D076 are retired. The dependency command passes
for 470 modules with 43 remaining migration exceptions and 28 public cross-feature contracts.

All 47 focused tests across nine files pass: four duplicate-binding, two shared read-binding, two
failure-conversion, seven publication, ten scheduler, three location-sheet, eight review, seven
exact-workspace and four annotation cases. New coverage proves parent/child runtime identity,
missing-binding failure, foreground-listener retirement, joined native completion, repeated close
and partial-startup cleanup. Existing stale-result, bounded-batch, retained-choice and cancellation
assertions remain. Source, test and integration-entry analysis is clean.

The canonical Windows Scan workflow passes all four cases on local-primary. Run
`11770b139ff4460fac99d07b6172dd95` takes 78,971 ms, exits zero and reports no cleanup failures. Its
added duplicate workflow uses production ports and runtime bindings, analyzes the two controlled
copies, verifies group/location/annotation results, opens the duplicate workspace and persists a
preferred physical location through the desktop control. The existing source-byte checks, native
picker, scan, source viewing, organization and removal workflows remain passing. Isolated evidence
is retained by the canonical execution owner; hosted verification continues independently.

Implementation / inline-test counts are 149 / 0 for the analysis controller, 190 / 0 for its scheduler,
70 / 0 for annotation batching, 117 / 0 for the exact adapter, 38 / 0 for annotations, 152 / 0 for
review and 75 / 0 for bindings. Dedicated scheduler and publication tests contain 305 and 199 lines;
the retained exact/review workflow tests contain 496 and 312. Binding regressions contain 219 lines,
failure-conversion tests 52 and the native duplicate helper 94. No Rust, generated bridge, schema,
dependency-version, model, original-media or user-catalog changes are included.

## O5.2 Organization records and ordering — 2026-11-03

Archive, cleanup, conflicts and sidebar ordering now have application ports and native adapters.
Sidebar move values live in domain. The organization binding module selects all seven organization
capabilities, including the command, batch and review adapters moved in O3. Settings consumes 20
explicitly registered controller/state/value edges; no broad cross-feature exemption is added.
Required ports fail when omitted, while inactive conflict reads are explicitly selected in test support.

Archive/cleanup task methods and event/error conversions are unchanged after relocation. Their
application controllers and optimistic sidebar-ordering owner retain identical lifecycle/policy
bodies. Conflict and sidebar request/mapping bodies are unchanged apart from native error translation
to the existing Ame organization failure. The immutable baseline remains unchanged. Ten native-layer
exceptions and 20 declared cross-feature entries are retired; the dependency command passes for 480
modules with 13 later-slice exceptions and 48 explicit public contracts. Source, test and integration
analysis is clean.

All 36 focused tests across eight files pass: two production-roster, seven archive-controller, four
cleanup-controller, three ordering, seven conflict-workflow, three sidebar-drag, five archive-Settings
and five cleanup-Settings cases. These retain cancellation versus terminal publication, failed and
stale confirmation, operation identity on retry, bounded/coalesced reads, conflict provenance,
optimistic rollback, native-picker cancellation and empty-inspection behavior. Required-binding
coverage includes each new organization port.

The first expanded Windows workflow, run `f5a7ec8ae986438f8869dabf8cecaaf0`, failed at its new Settings
navigation step because the test used a nonexistent button key. Existing native organization and
duplicate workflows had passed; the run retired without cleanup failures. The helper now uses the
actual `library-sidebar-settings` entry, with its export/inspection/restore and cleanup assertions
retained. A second run, `a03d012aea134c3696cafcc012a1d00d`, timed out in the earlier retained-import
pause check before reaching the new workflow. Repeated native poll-busy responses accompanied the
timeout; subsequent root-count failures followed the incomplete fixture retirement. That run took
177,462 ms, exited one and reported no runner cleanup failures. Its cause is unresolved.

A scoped diagnostic run, `5d398762db4a4675ae5520328bef5ca0`, recorded an accepted pause and stream
completion in 144 ms. All four Windows cases passed, including production archive export,
inspection, confirmed restoration, unchanged backup bytes, conflict reads, typed sidebar rejection
and disabled empty cleanup. The run took 249,844 ms and reported no cleanup failures. Temporary
native phase probes and test timing messages were subsequently removed; the original Rust and
retained-import helper bodies match HEAD. No scan timeout or assertion was weakened, and no causal
scan repair is claimed from this non-reproduction. Run `f1955f7ea5714c66a73172a018c24a43` reused the
instrumented native build before the restoration took effect: all four cases passed in 202,626 ms,
with no cleanup failures and retained-scan completion in 115 ms. It is additional diagnostic evidence,
not a separate uninstrumented native result.
The O5.2 ports and desktop flows passed with those probes enabled; the normal hosted run owns the
submitted source's full gate. Revisit the retained-import timeout if it recurs, without repeating
unchanged acceptance work solely for this unresolved observation.

Implementation / inline-test counts are 167 / 0 for the archive controller, 156 / 0 for cleanup,
103 / 0 for conflict projections, 77 / 0 for ordering, 108 / 0 for the archive adapter, 102 / 0 for
cleanup, 69 / 0 for conflicts and 27 / 0 for sidebar order. Dedicated archive/cleanup tests contain
185 and 109 lines, ordering tests 74 and conflict workflow tests 371. Production-roster tests contain
140 lines and the native records helper 131. No Rust, generated bridge, schema, archive format,
dependency-version, model, original-media or user-catalog changes are included.

## O5.3 Classification and model installation — 2026-11-03

Classification/session, configuration, review and installation contracts live in application ports;
configuration and progress values live in domain. Native implementations and synchronous/asynchronous
failure translation live under `adapters/rust`. The classification binding module selects those
ports and composes the existing runtime. Missing production bindings fail during acquisition;
explicit inactive bindings remain test-only. The review-service application/adapter cycle is removed.

Six application class bodies, configuration/review DTO mappings and native request arguments retain
their behavior. Existing startup, authorized recovery and reverse-order shutdown remain in place.
The immutable inventories are unchanged; D047, D055–D058 and D067 are retired. The dependency check
passes for 487 modules with seven presentation migration exceptions and 48 public cross-feature edges.
Analysis of source, tests and integration entries is clean. All 82 focused tests across 11 files pass,
covering production identity, each omitted binding, partial acquisition, no-installation startup,
error code/message/stack retention, cancellation/recovery, notifications and retained viewer semantics.

The first added native check (`9cc2ce5f90844dc6a31ef754681e0b2c`) incorrectly expected an unclassified
item before any taxonomy existed. The unchanged selection reader explicitly returns an empty list
in that state; the fixture now asserts absent categories and empty results. The stale-revision
failure assertion remains. That failed run retired without cleanup failures.

Canonical Windows run `e4cc8e790deb4bb3945afe18726f4dfd` passes all four cases in 157,909 ms, exits zero
and reports no cleanup failures. Production bindings share their instances with the desktop scope;
real configuration/navigation reads, disabled startup, empty classification reads and translated
`catalog_revision_changed` errors pass. Existing scan, source-byte, viewer, duplicate and organization
record checks also pass. No temporary native probes are present. The prior O5.2 retained-import
timeout did not recur; its cause remains unresolved. Hosted execution continues independently.

Implementation / inline-test counts are 83 / 0 for bindings, 76 / 0 for lifecycle, 145 / 0 for session
lifetime, 149 / 0 for environment, 85 / 0 for configuration and 181 / 0 for installation. Native
classification/configuration/review/installation adapters contain 79/111/91/54 lines, with no inline
tests. Dedicated binding, lifecycle, automatic-startup and installation tests contain 226/748/180/207
lines; the native classification helper contains 59. Unchanged model installation and analysis
protocols retain their [R5 evidence](r5-closeout.md); this slice adds no model download, evaluation,
Rust, generated bridge, schema, archive-format, dependency-version or source-media change.
