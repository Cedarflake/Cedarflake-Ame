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

## O6 Presentation organization — 2026-11-03

Gallery, duplicate review and Settings entries now live under their feature's `presentation/pages`.
Fourteen complete view-state owners and ten corresponding test files move to `presentation/coordination`;
their bodies are identical. Settings and duplicate page bodies are identical, and the gallery body
changes only its Settings destination call. Existing application and source-lifetime authority remains.

Required presentation bindings connect the Settings destination, record feedback and default-album
picker without cross-feature page/widget imports. Cancellation and clearing retain distinct results.
Preview construction moves to a platform factory that preserves `FileImage` and resized cache keys;
tiles, filmstrip and fallback rendering keep their repair and reveal owners. The read-only host fact
moves to its named desktop contract; direct access to its private implementation remains rejected.
All seven remaining Dart migration entries are retired. The fixed baseline is unchanged; final
analysis and dependency checks pass for 492 modules, zero migration exceptions and 48 named public
cross-feature edges. Boundary tests cover both the admitted host fact and rejected private access.

All 96 focused tests across 12 files pass, including omitted bindings, cache identity, repair,
source-generation and lease retirement, fallback pixels, Settings, album selection, feedback and
retained semantics. Canonical Windows run `65d93f57823f4be386e94720377b033a` passes all four cases in
140,294 ms, exits zero, has empty stderr and no cleanup failures. The new desktop path selects an
existing native album, cancels without clearing it, clears the selection and returns to Gallery.
Existing source viewing, organization, archive, duplicate and source-byte assertions also pass.
The later coordinator relocation changes imports only; exact body comparisons and final analysis
cover those paths while retaining the behavioral evidence. Hosted verification runs independently.

| Large affected owner | Production / inline-test lines | Dedicated test lines |
| --- | ---: | ---: |
| Gallery / duplicate / Settings pages | 1,902 / 0; 290 / 0; 268 / 0 | 356; 495; 1,234 |
| Dimension publication / gallery query transition | 260 / 0; 273 / 0 | 244; 397 |
| Organization mode / viewer session | 408 / 0; 354 / 0 | 490; 595 |
| Photo tile / filmstrip thumbnail / viewer image | 432 / 0; 259 / 0; 248 / 0 | 1,041; 314; 451 (source-lifetime suite: 634) |

The view-only zoom owner has 210 production lines and no inline tests; retained viewer interaction
checks cover its behavior. The workbench binding has 50 production lines, its admission tests 45 and
the native destination helper 78. No Rust, generated bridge, dependency version, model, schema, source-media or user-catalog
change is included. [Scoped ownership](../architecture/workflow-ownership/dart-presentation-and-composition.md#workbench-destinations)
and the [verification map](quality/ui-verification.md) retain current paths and contracts.

## O7.1 Duplicate and organization native boundaries — 2026-11-03

Duplicate availability policy consumes the `SourceStateProbe` port; its native adapter retains the
existing metadata and identity checks. The analysis factory selects the same guarded reader and
shared background admission. Root-state precedence, file-failure classification and the existing
regression body compare identically after the port substitutions. Catalog transactions, bounded
reads, fingerprint publication and source/namespace lease retirement are unchanged.

R068–R071 and R165–R166 leave the active migration roster. Five exact composition registrations
follow the [native contract](../architecture/workflow-ownership/native-composition.md#duplicate-and-organization-composition),
with lifetime, replacement and regression metadata. The immutable baseline is unchanged; 273 legacy
entries remain for later O7 groups. All 20 boundary checks pass, including neighboring-function,
stale-entry, duplicate-admission and missing-decision rejection.

The first focused Rust run passes ten cases but fails 35 during system temporary-directory creation
with `PermissionDenied`, before catalog setup. With process-local temporary paths inside the existing
leased acceptance scratch, the same 45 duplicate checks pass. Source-retirement/admission and batch
close/capacity checks also pass, for 47 cases in total; scratch cleanup succeeds. This changes only
the diagnostic command's storage location, without changing assertions or product code.

Canonical Windows run `de7891ada6744a1c9c60a677ece3ca27` passes all four cases in 237,624 ms, exits zero,
has empty stderr and no cleanup failures. Real duplicate analysis, local location availability,
gallery annotations and persisted preferred-copy selection pass through the new factories. Existing
scan, viewer, organization, archive, classification and source-byte assertions also pass.

Production / inline-test lines are 181 / 0 for the exact facade, 48 / 0 for availability, 13 / 0 for
its factories, 14 / 0 for the native probe and 7 / 0 for its port. Availability and execution tests
contain 186 and 250 lines. The dependency-policy owner has 164 production lines and 170 dedicated
test lines; the shared 1,174-line ports facade gains only its module declaration, retaining existing
test-support declarations. No schema, generated bridge, dependency version or original-media change
is included. Hosted checks continue independently under the user's CI direction.

## O7 Catalog-session composition — 2026-11-03

R011–R025 are retained as the exact session-host, validation-flight and connection-factory boundaries
defined in the [native contract](../architecture/workflow-ownership/native-composition.md#validated-catalog-session).
The roster contains 258 active legacy entries and 20 retained technical edges; the fixed baseline
is unchanged. No Rust implementation or test body changes in this delivery.

All 20 dependency checks and 13 existing session/maintenance regressions pass. They cover shared
failure and retry, delayed waiters, invalidation during validation/maintenance, panic retirement,
independent catalog paths and incremental-reclamation proof retention. The leased scratch is retired.
The unchanged runtime retains O7.1's native client evidence. Storage and bounded-read integration
remain in the roadmap queue; this registration does not admit neighboring use cases.

## O7 Catalog reclamation ports — 2026-11-03

The complete reclamation sequence now consumes `CatalogSpaceRepository` and `CatalogCapacity` in
`catalog_reclamation/execution.rs`. Four retry helpers use the existing repository port; native
maintenance and capacity selection live in two named factories. The Windows capacity call moves to
its adapter with the same unsafe boundary, errors and platform branches. The execution sequence,
seven helper bodies and all three capacity bodies compare identically after their typed substitutions.
Operation initialization, registry ownership, cancellation, maintenance reservations and checkpoint
completion retain their order. ADR 0005's interruption wording now agrees with the existing scoped
contract and implementation; normal incremental interruption preserves proof, while errors, unwind
and independent invalidation retire it.

All 31 reclamation regressions, two native Windows capacity fixtures and 20 dependency checks pass.
New injected-port cases cover insufficient capacity, probe failure, cancellation before work and
`FULL` conversion without a legacy capacity query. Real SQLite fixtures cover bounded release,
foreground preemption, interleaved reads and retained proof. The controlled scan/reclamation fixture
preserves search results, organization history, its album and exact source bytes. Scratch cleanup
succeeds. R004–R010 are retired; 251 legacy entries and 22 exact technical edges remain, with the
immutable baseline unchanged. Dart and bridge routing retain their prior client evidence.

The facade has 312 production lines and no inline tests; execution has 88, factories 12 and the
capacity port seven. Dedicated entry and execution tests contain 348 and 183 lines. The native
capacity adapter has 62 implementation lines and 20 inline fixture lines. No catalog schema, runtime policy,
source authority, dependency version or public bridge change is included.

## O7 Bounded catalog reads — 2026-11-03

The validated-session factory returns `CatalogReader`, with the same owned lifetime and ten named
read operations. Their bodies, generic retry loop and factory acquisition compare identically;
the redundant loading facade is removed. Root availability consumes `SourceStateProbe` through one
exact native factory after the unchanged preview reconciliation/recovery sequence. Preview and
viewer callers gain only the port import; their lifetimes and source guards are unchanged.

The reader regression run passes 32 cases. Its remaining terminal-reparse case fails at fixture
`symlink_file` with Windows error 1314 before the reader executes. This local privilege limitation
remains open; the assertion and hosted workload are unchanged. Successful cases retain structured
errors, bounded retries, read-only connections, identity guards, retirement before backoff and bridge
serialization. The failed run's leased scratch is retired.

All 14 catalog-load cases and 20 dependency checks pass. The load suite executes its otherwise
ignored child for 256 public reads during real WAL writes; retained-library acceptance remains
unrequested. The native root fixture distinguishes available and missing directories without
creating the missing path or changing source bytes. The subsequent test builds emit no warnings,
and scratch cleanup succeeds. R160–R161 are retired; 249 legacy entries and 23 exact technical edges
remain. The immutable baseline is unchanged, and hosted checks continue independently.

Production / inline-test-support lines are 230 / 524 for catalog loading, 405 / 1,368 for the existing
retry owner and 346 / 26 for the session host. The read port has 86 lines; root projection has 11
plus two test-module declaration lines and 94 dedicated test lines, and its factory has five. No schema, dependency, generated bridge, source
authority or product-policy change is included.

## O7 Storage resources and cache layout — 2026-11-03

Storage selects settings persistence and preview-cache layout/accounting through ports. Three exact
factories select the settings connection, stateless cache adapter and OS default directories. Budget,
source-overlap validation, save admission and restart activation remain application policy. All 28
affected storage function bodies, activation and default composition compare identically after the
typed substitutions; the complete cache-accounting body is unchanged in its adapter.

The pure path-display function and its existing test move to domain ownership. All consumers use
that same function, and the old adapter export is removed. This shared import closure also retires
six scan/preview/continuity edges; those workflow bodies are unchanged. Along with the ten storage
entries and activation's settings construction, 17 migration entries retire. The fixed inventory is
unchanged; 232 legacy entries and 26 exact technical edges remain.

All 32 storage regressions, the relocated path-display regression and 20 dependency checks pass.
Real SQLite fixtures cover save/scan admission races, restart isolation, both activation commit
boundaries, source-overlap rejection, current/legacy cache accounting and access/display separation.
Assertions are unchanged and leased scratch cleanup succeeds. The cache-version export is now
test-only because production layout stays inside the cache adapter; the final production library
check passes without warnings.

Storage has 445 production lines plus six test-import/declaration lines, with 571 lines in its main
dedicated fixture file. Configuration update remains 72 lines; factories have 44, the cache adapter
49 and its port eight. Path display has 18 production and 20 inline-test lines. Other affected large
workflow owners change only imports; source access, schema, bridge and cache formats are unchanged.

## O7 Classification execution composition — 2026-11-03

R026, R055, R061–R067 become nine exact source-entrypoint and runtime-resource registrations under
the [native composition contract](../architecture/workflow-ownership/native-composition.md#classification-execution-resources).
The native host owns its catalog, feature cache and encoder factory; application use cases retain
publication and cancellation through existing ports. Shared admission, lazy initialization, source
retirement and blocked replacement after unproven shutdown retain their original owners.

Every registered dependency matches its original source/symbol/target. Implementation and regression
files compare unchanged to `cc060176`. The roster now has 223 legacy entries and 35 exact
technical edges; the fixed baseline is unchanged. Discovery, configuration and installation remain
separate migration work, with no neighboring-file exemption.

All 49 focused checks pass: seven runtime/ownership cases, nine encoder-residency cases, six analysis
and six source-verification cases, one admission/retirement case and 20 dependency checks. They
retain source-release ordering, cancelled/stale-result rejection, checkpoint rollback, cache reuse
after failed publication and blocked replacement after incomplete shutdown. Both builds emit no
warnings and leased scratch cleanup succeeds; no model download or user-library read is involved.

## O7 Classification discovery contracts — 2026-11-03

Discovery consumes the expansion, merge and retirement repository ports and a managed-vocabulary
contract. One exact factory selects the bundled implementation. The notification-scoped pass keeps
its original reconciliation, sampling, reduction and completion order; six complete state/resource
and metadata bodies compare identically after the factory substitution against `7bf11299`.

All 40 focused checks pass: five curated-vocabulary, six discovery-preparation, three discovery-policy,
six dispatcher-maintenance and 20 dependency cases. They cover malformed/cancelled loading, bounded
cached preparation, stale publication, pause/cancellation and executor retirement before handoff.
Builds emit no warnings and leased scratch cleanup succeeds. R056–R060 retire; 218 legacy entries
and 36 exact technical edges remain, with both frozen inventories unchanged.

The discovery owner has 219 production lines; its runtime host has 101 plus three test declarations,
the vocabulary port 34, its factory ten, and the curated adapter 152 plus three test declarations.
No lifecycle policy, model data, schema, source authority or bridge change is included.

## O7 Classification configuration and installation — 2026-11-03

Configuration, desktop startup and model installation consume installation-persistence and
model-package contracts. Paths and versioned download intents move to Ame domain values; three exact
factories select persistence, the pinned package and OS storage. The prepared-resource contract keeps
the existing file/namespace guards and staging lease until publication or disposal. Its application
owner and runtime-retirement admission retain their order. Against `3eff021e`, 52 complete persistence,
application, transfer and resource bodies compare identically after typed substitutions.

All 53 focused checks pass: seven installation-storage, 17 native package/transfer, six application
installation, one configuration, two desktop and 20 dependency cases. The existing managed/bundled
fixtures now hold their native guards through the prepared contract and still reject overwrite,
deletion and namespace replacement. The new terminal-outcome regression observes durable selection
at resource release for success, cancellation, stale selection, active runtime and disposal; each
releases installation ownership afterward. Migration, restart intent, bounded transfer and atomic
publication assertions remain intact. Both builds have no warnings; leased scratch cleanup succeeds.
No model payload download or user-library read is involved. R027–R054 retire, leaving 190 legacy
entries and 39 exact technical edges; both frozen inventories are unchanged.

The installation facade has 166 production lines plus three test declarations; its operation owner
is unchanged at 151 plus three. Their dedicated fixtures have 132 and 110 lines. Native preparation
has 269 production lines plus five test declarations and 348 main/bundled fixture lines. Persistence
has 147 plus three, its intent owner 84 plus three, and their fixtures 271. New factories, repository
port, package port and package adapter have 29, 44, 37 and 73 lines. Schema, pinned artifacts, transfer
protocol, public bridge and classification policy are unchanged. The roadmap advances to scan/preview.

## O7 Preview and viewer resource composition — 2026-11-03

R173–R184 and R276–R278 become 15 exact resource-host, construction and borrow registrations under
the [native composition contract](../architecture/workflow-ownership/native-composition.md#preview-store-resources).
Store selection retains access-first admission, operation-scoped namespace identity, current-budget
reacquisition and short registry locks. Viewer acquisition retains catalog revalidation and releases
its native guard before lease capacity. Workflow policy and the remaining native interfaces are not
covered by these registrations. Source and test files are byte-identical to `d2a1bfdd`.

All 34 focused checks pass: four production-store reacquisition, eight viewer-source, two Windows
viewer-guard and 20 dependency cases. Real fixtures retain source bytes, block file/ancestor mutation
while held, recover capacity under the current accounting owner and reject post-acquisition catalog
changes. Assertions are unchanged, builds have no warnings and leased scratch cleanup succeeds.
The roster has 175 legacy entries and 54 exact technical edges; the frozen baseline is unchanged.

## O7 Classification rules and reverse dependencies — 2026-11-04

The five SQLite-to-application dependencies present at `3f0a4567` now consume domain-owned decision,
merge and retirement rules. Transaction-local revalidation, cancellation, replay and publication
remain in their existing owners. Application preparation retains repository and vocabulary access;
the public classification facade retains its signature. All 92 affected original function bodies,
including tests, compare identically after relocation and the eligibility-path substitution.

All 98 focused checks pass: 26 domain, four contextual-selection, 44 SQLite workflow and 24 dependency
cases. They retain stale-plan rejection, human-intent protection, cancellation, rollback, undo and
restart/replay. A redundant unused helper found during the first build was removed; final library
and test builds emit no warnings, and leased scratch cleanup succeeds. No source-library read,
schema change, model download or generated bridge change is included.

The gate now checks adapters as dependency sources and loads handwritten root facades for alias
resolution. Four new cases cover admitted engine/port/domain dependencies, forbidden application/API
and bridge dependencies, conditional code and a root-facade bypass. A redundant combination of
already-covered forward and reverse edges was removed. Both frozen inventories remain unchanged,
with 175 active legacy entries and 54 retained technical edges. Broker direction and complete
cross-layer cycle coverage remain in O7.

Decision and merge-rule owners have 104 and 210 production lines plus three test declarations each;
their dedicated tests have 514 and 169 lines, with one 130-line shared fixture. Dependency collection
has 250 lines, the extracted rule owner 68 and parser tests 325. The final client/real-library check
remains required after the complete migration under the execution plan.

## O7 Scan discovery boundary — 2026-11-04

`ScanFileDiscovery` and its borrowed `ScanDirectoryEntries` expose source discovery without native
handles. The two factories select the original discovery and inspector; file-visit values move into
domain and their adapter exports are removed. Relative admission, cursor completion/iteration,
entry processing and traversal bodies compare unchanged to `43a1beb6`; the scan facade differs only
by the two factory substitutions. Final publication retains its separate namespace capability.
R002, R073, R210, R212, R214, R215 and R260 retire, leaving 168 legacy entries and 56 exact retained
edges. Both frozen inventories and generated bridges are unchanged.

All 48 focused checks pass: three directory-cursor, four file-admission, 12 entry-processing/resumption,
five full scan workflows and 24 dependency cases. They cover complete/partial/replaced directories,
held-source evidence, no placeholder reads, metadata/preview publication, cancellation, pause/resume,
deep-directory recovery and 1,025-entry bounded traversal. Existing cursor cases now exercise the port.
One duplicate magic-header-only test is removed; the retained real-PNG admission matrix and complete
scan/preview case cover wrong extensions with stronger result and source-preservation assertions.

The first build found a missing moved-type test import, corrected before these results. The sandboxed
full scan rejected an unpinnable parent namespace; the same controlled fixture passes outside the
sandbox with unchanged safety checks. Dependency checks use their integration target. Final builds
emit no warnings, the format check reports 794 files unchanged, and leased scratch directories retire.

The scan facade has 752 source lines, no inline test block and 5,511 lines in its main test file;
traversal has 357 source lines. The native file facade retains 2,614 lines before its 3,460-line inline
test block, including existing conditional instrumentation. File admission has 245 source/203 dedicated
test lines; enumeration has 132/63; entry processing has 211/327, with separate resumption fixtures.
The reconciliation facades change only imports: authoritative and incremental owners have 560 and
1,342 source lines, no inline test blocks, and 1,408 and 3,839 lines in their main test files.
The new domain, port, adapter binding and factory files contain 22, 22, 30 and 11 lines respectively.
No schema, source authority, engine version or client behavior changes; final real-library client
verification remains scheduled after the remaining migration.

## Cached preview dimension cursor repair — 2026-11-04

At `d3bd7366`, a valid oriented JPEG with an existing preview and unknown catalog dimensions fails
with `image_dimensions_failed` when its supplied file cursor has already reached EOF. The cloned
handle retains that position. The existing orientation/header case reproduces this failure without
changing the source bytes or cache fixture.

`inspect_source_display_dimensions` now seeks to the header before format detection, retaining
`image_open_failed` for seek errors. Both existing cache-hit cases pass: oriented dimensions recover
to 60×80 with unchanged cache/source bytes, and known catalog dimensions bypass source decoding.
No test is added; the original case now covers the previously missing cursor boundary. The owner
has 911 source lines and 901 inline-test lines. Format checks report 794 files unchanged; scratch
cleanup succeeds. Source admission, publication, schema and generated contracts remain unchanged.

## O7 Preview source contract — 2026-11-04

`PreviewStore` now consumes `PreviewSource`, whose associated reader exposes only standard read/seek
operations. The native adapter duplicates the already admitted handle and resets it to byte zero;
the three preview read paths reuse that owner. Source opening, revalidation and publication stay
in their existing owners. JPEG decoding accepts a reader with the same algorithm, errors and limits.
Application preview orchestration and real-format fixtures are byte-identical to `4ec40f5c`.

All 43 focused checks pass: two cache-hit, 12 real-format, four JPEG decoder, one complete scan/preview
and 24 dependency cases. Coverage retains wrong extensions, corrupt/truncated input, CMYK fallback,
orientation, animation, cache accounting, unchanged source bytes and the repaired cursor boundary.
The four decoder cases now use encoded bytes through `Cursor`, removing eight fixture lines and
unnecessary temporary files; real-file safety remains covered by the adapter and complete workflow.
No cases are added. Final test/production builds emit no warnings, the format check reports 794
files unchanged and leased scratch cleanup succeeds.

R279 retires without a replacement exception: 167 active legacy entries and 56 retained technical
edges remain, with both frozen inventories unchanged. The new port has 23 lines, its native source
owner 101, and the preview store 904 source/901 inline-test lines. JPEG decoding has 141 source,
112 inline benchmark and 79 dedicated contract-test lines. No package, cache-format, engine
version, schema or generated bridge change is included.
