# R2c picker and task controls

Status: **Selected Debug native picker, focused Enter Retry, pre-registration Pause-Cancel,
rendered feedback, preserved baseline and normal exit verified**.

This record covers the [actual picker and task-control method](../plans/r2c-closeout.md#actual-picker-and-task-control-method).
The initial 2026-09-22 source starts at `ef4eee27ca47c1c7e6d3b97e986f9d73da6ded7a`, with the C06
presentation correction and regression below. The [complete native checkpoint](#complete-native-input-and-rendered-feedback)
uses unchanged product source at `4787c84`. It closes the selected Debug input duties, not Release,
every UX-08C focus-return variant or all of R2c.

## C06: cancellation while pause is pending

The task surface previously displayed controls only for `scanning`. After Pause changed the
projection to `pausing`, Cancel disappeared although `library_scan_control.dart` still admitted
Cancel as the higher-priority intent. This directly blocked the selected pre-registration journey;
it is an S1 control-entry defect, not a new native cancellation policy.

The connected widget regression first fails on the original presentation because
`library-cancel-button` is absent. The correction keeps the existing Cancel button during `pausing`
and displays Pause only during `scanning`. The regression drives both buttons through the real
controller, observes cancelling feedback, then delivers late Started and verifies only Cancel is
replayed. Terminal cancellation preserves the previous roots. Native registration, stream-drain,
publication and cancellation ownership are unchanged.

The affected production file has 204 lines and no inline tests; its dedicated feedback suite has
122 lines. This change adjusts the existing task control rather than introducing a new control,
state owner, migration, dependency or polling loop.

Serial verification passes 15 scan-control, 2 feedback, 8 retained-interaction and 9 diagnostic
admission/input cases (34 total). After the diagnostic keyboard guard was tightened, its three
affected cases pass again. Repository formatting checks 228 files with no changes; complete Dart
analysis passes with informational and warning diagnostics fatal. The final isolated Debug build
passes in 22.9 seconds. Full local lint/Daily remains blocked by C02; earlier hosted results are
not final-source evidence for this presentation change.

Independent preparation review identifies three evidence weaknesses: Enter needed a focused task
button, cancellation needed current terminal-state pixels, and the issue needed its exact damaged
path and error code. The corrections retain those independent assertions. A scoped recheck finds
that an invalid retry could be hidden by a later valid one; rejection is now permanently recorded
and forwarded to the diagnostic failure owner. Negative tests reject a later correct Enter after
an invalid attempt. The final scoped recheck closes that finding. Active review totals about nine
minutes, including the C06 control admission; no native pass is inferred from review.

## Intended native evidence

The prepared generated source contains three valid images, including PNG bytes under a `.jpg`
name, and one damaged PNG in a Chinese-named directory. Exact paths, hashes, file IDs, sizes and
timestamps are frozen before launch. The second picker target is the existing 10000-image corpus
with twelve dimensions and historical dates, totaling 10921494393 bytes.

The diagnostic uses the production picker, bridge, catalog and synchronization lifecycle with
marked Debug-only storage and in-memory preferences. After the first real commit it would inject
one catalog display-read failure, then require actual focused Enter to retry only the display.
A second-command decorator would hold dispatch for at most 120 seconds so actual Pause then Cancel
could reach the pre-registration boundary; it forwards real events without substituting native
results. These disclosed controls prove ownership rather than production latency. Neither phase
was reached in the lifetime below.

## Retained failed native lifetime

Run `5c0b0f7ea3c14bfba8fd6a50d066d2e3`, PID 13912 under owned parent 37844, reveals the empty
application and opens the actual Windows folder picker. Computer Use subsequently returns
`coordinate input geometry is unavailable`, `call get_window_state before using this window`,
and `unknown screenshotId screenshot-1`. Re-observation and fresh selection do not resolve the
binding failure. Reported focus remains the search box; successful delivery to the intended path
field is not established. No directory is accepted and no source scan is dispatched.

The attempt stops without an unchanged replay. After verifying executable, PID and parent identity,
the owned client is deliberately retired. Exit -1 after 250839 ms is a failed execution, not normal
shutdown evidence. Parent receipt confirms process exit, Job closure and no cleanup failure. The
monitor retires with 938 samples, peak working set 440160256 bytes, sampled private bytes 313692160,
kernel peak commitment 359796736 and minimum system availability 5378469888 bytes. No resource
bound is exceeded. There is no successful input result or ready-to-close receipt.

Full pre/post source checks pass in 70.267/50.485 seconds for all four small files and all 10000
frozen files: exact membership, bytes, IDs, sizes and baseline timestamps remain unchanged. No real
library is selected. The failure identifies a missing automation path, not an Ame scan/preview
defect, and does not establish shared causality with C02's separate native probe or file replacement.

Debug SHA-256 identities are:

- EXE: `2A24C91530C05D88B29ACC4E6FCF73199C237DEB04159B232A9FD7E43BD98DCE`.
- Dart kernel: `2BDE941F13E38E2A0336AD47EF71D6ABB17A5E0764ADF8552EB36E6931B2D98E`.
- Rust DLL: `77B193EA7CA1B5CEEC033038E3806490890E5DBB3E818AAD72B062D0A46F50C2`.

Ignored `.build/r2c-input-controls/` retains the immutable admission, diagnostic source, failing
pre-fix regression, passing focused outputs and build logs. Its GUID-owned fixture retains the
input failure, process/memory receipts, raw logs, frozen source manifest and integrity results.
Conservatively charge the full 120-minute input reservation and 45-minute C06 supplement; these
ceilings are not measured active-time totals. Earlier C01/C02 and failed native allowances remain
consumed. Another client lifetime requires a recorded changed input method and a new bounded
checkpoint. It cannot replace this failure, loosen the input oracle or launch against the user catalog.

The [C05 bulk preview pass](r2c-browsing-diagnosis.md#accumulated-candidate-and-transient-preview-feedback) remains separate:
C06 changes task-button visibility, not preview materialization or retirement. Actual picker/task
input, complete focus assertions, Release paths and the final accumulated gates remain open.

## Native automation preflight checkpoint

The changed native-button method on production head `4a5987b` uses the same real picker with an
explicit initial fixture directory, as in the controlled Windows scan integration. Static review
took 5 minutes 30 seconds. A helper error-recording weakness was corrected before launch so receipt
or log IO cannot replace an earlier helper failure. Dart analysis, the 31.0-second Debug build,
PowerShell syntax, x64 keyboard input layout and rejection of an invalid target pass. Preparation
also retained an initial missing-import analyzer failure; that import was restored without changing
the application. The original keyboard and registration-barrier assertions remain unchanged.

Lifetime `6510b7c9d5e048e3894b13641a9f2763` starts successfully but its native automation tree exposes
only the correct `FLUTTER_RUNNER_WIN32_WINDOW` and one `FLUTTERVIEW` pane. No buttons or edit controls
appear after activation and refresh; the third read records the exact window identity and node
types. All three snapshot helpers exit zero with their Jobs closed. No control action, picker
selection or source scan is delivered. The main process is stopped after identity revalidation;
its 170669 ms receipt records exit -1, process exit and Job closure with no cleanup failure. This
is an incomplete input attempt, not a passing native workflow or a diagnosed scan defect.

All four generated inputs and 10000 frozen sources retain exact membership, bytes, IDs, sizes and
timestamps in before/after checks taking 61.847/58.847 seconds. Across 637 samples, peak working set
is 372527104 bytes, sampled private memory 306688000, kernel peak commit 324825088 and minimum system
availability 4691898368. EXE and Rust DLL hashes match the preceding failed-input lifetime; the new
Dart kernel hash is `DE3079BCCA2170CB8D15BBCF29F0A9B90ED18305B9D3340DD9D8D8F066FB2519`.
Ignored `.build/r2c-input-native-actions/` and its GUID-owned fixture retain the admission, helper
requests/results, native logs, memory/process receipts and source verification. The 90-minute
reservation is charged conservatively in full; no unchanged replay is admitted.

Inspection identifies two preparation differences from the established accessibility integration:
that fixture explicitly holds a Flutter semantics handle and its native probe uses a raw-view cache
request. This input harness did neither. The pinned SDK's `ensureSemantics` keeps semantics enabled
while its handle lives; Microsoft documents the cache filter's default as
[ControlViewCondition](https://learn.microsoft.com/en-us/dotnet/api/system.windows.automation.cacherequest.treefilter).
These differences justify a changed diagnostic method, but do not yet establish why native automatic
activation returned no controls. No product accessibility correction or native input pass is claimed.

Hosted [35710919997](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35710919997) on
`4a5987b4f955e8750ae30da1e1e3e5a365d1e114` subsequently passes all ten required jobs and the aggregate;
three signing-only jobs remain skipped. This is current hosted evidence for the C06 product change,
separate from both failed local input attempts and C02's unresolved local complete gate.

## Semantics-enabled negative diagnostic

Lifetime `b946720e7c794644840a75339e821eea` tests explicit semantics ownership plus raw-view querying.
The 34.0-second Debug build and Dart analysis pass; native lifecycle, fixture, picker and keyboard
owners remain unchanged. Two narrow reviews take one minute each. The second checks a discovered
helper-recording collision: the cleanup function's `$Action` parameter had shadowed the caller's
action name inside receipt serialization. The new helper uses `$NativeAction` with the same external
alias. The old malformed fields remain; their separate immutable requests identify all three old
actions as Snapshot, and their exit/Job/process fields remain valid. A direct scope check confirms
the distinct name survives that same cleanup owner.

The application reports framework semantics **true before and after** acquiring its diagnostic
handle, with platform semantics also true. Nevertheless the first raw-view snapshot exposes only
the main window and `FLUTTERVIEW`, with zero controls. Thus merely omitting an explicit semantics
handle does not explain this diagnostic's native visibility gap. The cause of the provider/input
gap remains unproved; the two changed prerequisites do not establish separate causal effects.

No input or scan is delivered. The exact owned process is retired after 141373 ms with exit -1,
confirmed process exit and Job closure, and no cleanup failure. The snapshot helper's corrected
receipt identifies Snapshot and confirms exit zero, process exit and Job closure. All 10004 sources
pass exact before/after checks in 81.861/48.740 seconds. Across 524 samples, peak working set is
381800448 bytes, sampled private and kernel peak commit are both 332316672, and minimum system
availability is 4411379712. EXE/Rust DLL hashes remain unchanged; the kernel is
`DD2653A63503CDDF2F3BD314B1CC3D8898988A5BC406AE35384B8B92605701E1`.

Ignored `.build/r2c-input-semantics/` and its GUID-owned fixture retain this separate admission,
negative framework/native observations, corrected helper receipt and source/process/memory evidence.
Charge the 60-minute reservation in full. Stop this native-provider input route; do not repeat it
under another unchanged accessibility flag. Actual task inputs remain open, as do automatic native
accessibility activation, Release and the separate full local gate. A later input method must use
an observed, available interface and retain the original application-level input assertions.

## Graphical input and read attribution

Lifetime `67c06061452b4b3ea55bf3fb70c22dd4` uses the same entry, native picker, fixture and process
owners byte-for-byte; only the build target changes. Preparation passes Dart analysis, PowerShell
syntax and a 30.3-second Debug build. A two-minute independent static review finds no blocking
preparation difference. The available Computer Use interface returns the native application tree.
Initial indexed input reports missing window state, and a recovery query reports an active helper
request. Fresh observation and screenshot input then open the real picker. Its observed initial
directory is confirmed through the owned modal screenshot; no path is typed or substituted.

The actual picker returns the exact prepared Chinese directory. The native scan completes with
three accepted items and one issue. At 235241 ms, before recorded application keyboard input, the
diagnostic rejects a query as lacking focused Enter. The retained stack enters through
`refreshFromSynchronization` and `runPassive`, not the task retry command. Its predicate checks only
`reloadPending`, which also remains true while the failed task coexists with passive refresh.
This establishes an attribution gap in the diagnostic; it does not prove a production Retry defect
or a passing keyboard path. The second source selection and pause/cancel sequence are not executed.

The attempted graphical close reports an unknown current screenshot ID. After exact PID, parent,
executable and start-time revalidation, only the owned application is terminated. The 378395 ms
receipt records exit -1, process exit and Job closure with no cleanup failure. Across 1422 samples,
peak working set is 349302784 bytes, sampled private memory 316522496, kernel peak commit 334422016,
and minimum system availability 4207874048. All 10004 sources pass exact before/after integrity in
52.450/48.970 seconds. EXE/Rust DLL hashes remain unchanged; the kernel hash is
`116D76835899090D276EA7C2077E90034522B3D95EA016B4D45CE6E0E0F1702C`.

Ignored `.build/r2c-input-graphical/` and its GUID-owned fixture retain the preparation comparison,
admission, raw trace, failed input and process/memory/source receipts. The 60-minute reservation is
charged in full. Narrow the read classifier only after proving passive-versus-task behavior with
the real controller; actual keyboard, pause/cancel and complete native acceptance remain open.

## Primary committed refresh admission

The attribution regressions reproduce a separate application defect, C07: primary first-publication
and Retry call the raw first-page reader without entering the existing query-refresh coordinator.
A passive synchronization read can replace their publication generation and produce
`catalog_publication_view_superseded`; in the reverse order, Retry reads before the held passive
read completes. All three new real-controller regressions fail before the correction.

`LibraryController.reloadPrimaryScanCatalog` now delegates to the viewport's
`refreshPrimaryScanCatalog`, which composes the existing committed-refresh admission with the raw
reader. Raw reads remain separate for callers that already hold admission; wrapping those would
queue an obligation behind itself. This adds no scheduler state, retry policy, schema, dependency,
source read or media write. Original read exceptions propagate unchanged. ADR 0025 requires a
committed obligation to follow a proven user-query replacement, so the obsolete test expecting
failure after that replacement now proves rejection of the old result and completion on the latest
query. A separately held replacement read must remain `refreshing/reloadPending` before completion.

Focused verification passes 23 production-boundary cases: three initial/Retry/passive interleavings,
15 primary-workflow cases and five query-coordinator cases. These retain true read-error text,
no repeated scan/resume, cancellation, disposal and late-result ownership. Explicit-file formatting
and warnings-fatal Dart analysis pass. Independent review takes nine active minutes and its scoped
recheck one minute; the recheck closes the missing intermediate-state assertion. Physical sizes are
354 controller and 1350 viewport production lines, zero inline-test lines, with the unchanged
130-line query owner; dedicated suites contain 108/390/134 lines respectively. The viewport gains
only admission composition, not a second lifecycle owner.

The separate ignored `r2c-input-retry-scope` diagnostic attributes each read through the actual
controller entrypoint and asynchronous scope, clearing inherited committed context at passive
entry. Four attribution cases pass, including both ordering directions and nested/late context;
three existing keyboard-token rejection/acceptance cases also pass. Preserve the preceding failed
predicate and scope runs, which exposed the product race rather than proving an input failure.
These seven diagnostic tests do not establish delivered native keyboard input. The fresh reserved
graphical lifetime is retained below without delivered input; final-source gates remain pending.
The unresolved C02 local gate is not restarted unchanged. All focused work uses controlled ports
without accessing original media.

## Read-attribution native checkpoint

Product source `935961800893862a2a2d0cf50ce6f31673ce3f12` builds the scoped diagnostic in 29.5 seconds;
formatting, Dart analysis and PowerShell syntax pass. A one-minute independent preparation review
confirms that asynchronous attribution preserves the original calls and input assertions. The
fixture, native picker and process owners remain byte-identical to the preceding graphical method.

Lifetime `f3ad386cb58449399ff9fa39c218fae3` reveals the empty native client and returns a populated
accessibility tree, but screenshot input reports `unknown screenshotId screenshot-0` and indexed
input reports `call get_window_state before using this window`. Fresh selection and one JavaScript
reset preserve the same screenshot-binding failure. A coordinate action without the optional
screenshot ID and a Shift-Tab call return without error, but no native picker or application
keyboard event appears in the trace. Their tool responses are not delivered-input evidence.
The attempt ends before an accepted directory, scan, injected read failure or task Retry.

After PID, parent, executable and start-time revalidation, the owned application is terminated.
Its 327864 ms receipt records exit -1, process exit and Job closure with no cleanup failure;
this is not a normal-close or input pass. Across 1231 samples, peak working set is 342024192 bytes,
sampled private memory 297041920, kernel peak commit 330420224 and minimum system availability
5087272960. All four prepared files and all 10000 frozen files pass full identity, membership,
content and timestamp checks before/after in 71.769/49.600 seconds. The EXE and Rust DLL retain
their earlier hashes; the diagnostic kernel is
`C28AFEF55A2F420046AB971D1F0F07A12F5A3784C040817F3EB37CD714F40FF7`.

Ignored `.build/r2c-input-retry-scope/` and its GUID-owned fixture retain original negative
regressions, passing focused logs, preparation, admission, interface failure and source/process/
memory receipts. Conservatively charge the 60-minute attribution and 90-minute C07 reservations
in full, including their recorded reviews; 3359 minutes is a cumulative reservation, not measured
elapsed work. Stop this graphical binding route until its missing input capability is corrected
or a separately prepared changed method can prove delivery. The C07 focused correction remains
valid, while actual task controls, Release and the separate local complete gates remain open.

## Deferred manual input and current capability check

Lifetime `55dddd3e449344a592f111488e235f10` reuses the preceding EXE, kernel, Rust DLL and picker /
fixture / process owners after exact hash verification. No production-compiled source changes
between `9359618` and the accumulated `9c584a0` candidate; the only Rust change is test-only.
A fresh GUID-owned catalog and four-file source are prepared, and all 10004 generated sources
pass the full integrity check before launch. The window is revealed, but no manual input occurs.

A current Computer Use capability check returns the correct native window, its empty-gallery
screenshot and indexed Import button. Clicking that observed button immediately returns
`element 31 is not available in cached app state for cedarflake_ame.exe`. The application trace
contains no picker opening, accepted source, keyboard event or dispatched scan. This remains an
automation state-binding failure; it does not establish an application input or scan defect.
The failed binding route is not retried and no helper or direct presentation callback replaces it.

The exact PID, parent, executable and process creation time are revalidated before deliberate
retirement. The parent receipt records exit -1 after 306218 ms, process exit and Job closure with
no cleanup failure. This is an incomplete input lifetime, not a normal-close pass. Across 1149
resource samples, peak working set is 357912576 bytes, sampled private memory 302141440, kernel
peak commit 319229952 and minimum available system memory 4721377280; the original bounds hold.
Full before/after source checks pass in 48.720/39.826 seconds for exact membership, content, IDs,
sizes and timestamps of four small files and 10000 frozen files totaling 10921494393 bytes.

Ignored `.build/r2c-input-manual/` and its GUID-owned fixture retain admission, preparation,
process/memory receipts, raw application logs and both full source checks. Charge the 45-minute
handoff reservation conservatively in full. Actual focused-Enter Retry and pre-registration
Pause/Cancel remain unproved. Another native input lifetime needs an available input interface
and a recorded changed method; no unchanged launch or successful-input claim follows this check.

### Renewed interface availability

The explicitly renewed check uses lifetime `2a2d8d4dc200412387295c807981aa69`, fresh isolated storage
and the same verified artifacts. Discovery, screenshot and the indexed Import control are available;
the first click repeats `element 31 is not available in cached app state for cedarflake_ame.exe`.
The trace again contains no picker, source scan or keyboard event. No second input route is tried.
Exact process revalidation precedes deliberate retirement: exit -1 after 94273 ms, process exit
and Job closure, with no cleanup failures. This is a failed availability check, not normal shutdown.

All 10004 sources pass before/after integrity in 47.222/42.615 seconds. Across 349 resource samples,
peak working set is 355926016 bytes, sampled private memory 296370176, kernel peak commitment
317149184 and minimum system availability 4966293504; the original bounds hold. Ignored
`.build/r2c-input-availability/` and its GUID-owned fixture retain the exact admission, separate
tool-response transcription, raw trace, process/resource receipts and source checks. Charge the
30-minute reservation in full. Input remains unavailable; repeat discovery or an unchanged launch
cannot close actual keyboard, task-control or candidate acceptance.

### Tool recovery and delivered input

The 2026-09-23 tool-only diagnosis uses the installed Computer Use plugin `26.915.31029` and
`@oai/sky` runtime `0.7.1`. Session and turn identifiers remain consistent across calls. The
installed host closes its helper when its last pipe connection closes, but no measurement proves
that teardown caused the earlier failure. A native Explorer control works across separate calls:
an indexed View menu opens, Escape closes it, and subsequent observation confirms dismissal.
The first text-only attempt reports `coordinate input geometry is unavailable`; screenshot plus
accessibility resolves that control's geometry. The previously minimized window is restored to
its original minimized state. Calculator approval times out without a delivered action.

Ame lifetime `8c23b1be071145bb80f15b6ecc32a80a` uses a fresh isolated catalog and byte-identical
prepared artifacts. Explicit activation and a fresh screenshot/accessibility observation still
produce `element 907 is not available in cached app state for cedarflake_ame.exe`. The preserved
indexed failure is followed by the admitted screenshot-only comparison, using the actual returned
screenshot identifier and an observed coordinate. That action opens the native picker at 110209 ms.
Its observed confirmation returns the exact small fixture directory at 150721 ms. One real scan
publishes the three expected assets and the controlled post-commit display-read failure occurs
exactly once. That displayed failure is the existing diagnostic injection, not a new tool defect.

Fourteen native key-down records contain six Tab events, four Shift Left events and four unlabeled
events accompanying Shift-Tab. Key-up and pressed-key state are not recorded by this observer;
modifier release cannot be inferred. These records confirm delivery but do not prove
focused task activation: no Enter on `library-retry-button`, admitted retry read, second picker,
Pause or Cancel is recorded. A read-only `ext.flutter.debugDumpFocusTree` snapshot reports the
screen-level Focus as primary rather than a task button. A further read-only keyboard-state
expression cannot compile because this launched debug process has no compilation service; no
expression executes. These observations leave focus progression unexplained and do not establish
whether application focus or injected-key handling owns it. No programmatic focus or retry action
substitutes for native input. Screenshot-based pointer input is usable; indexed Flutter controls
and the complete keyboard/task-control journey are not accepted.

The process is deliberately retired after 702395 ms with exit -1, process exit and Job closure,
without cleanup failures. This is not normal-close acceptance. All 10004 generated sources pass
identity/byte checks before and after in 43.625/42.777 seconds. Across 2647 samples, peak working
set is 446103552 bytes, sampled private memory 331612160, kernel peak commitment 352149504 and
minimum system availability 5270708224. No real-library run, tool-permission change, installed
binary patch or product-code change occurs. The native trace SHA-256 is
`560E0A20939FFF2C1EEFC9FE7DF14726B37E6EC377A0ED28DFD44A8EDC47E4EA`.
Ignored `.build/r2c-input-tool-recovery/` and its GUID-owned fixture retain admission, trace,
resource/process receipts and complete integrity evidence. Charge the 70-minute reservation in
full. Any further focus investigation needs a changed, bounded observation method, preserving the
required focused Enter and pre-registration controls instead of accepting raw key delivery.
That method must distinguish input from observation effects with before/after focus ownership,
physical/logical key identities, key-down/up and pressed modifiers. Repeating Tab without those
observations cannot discriminate a traversal failure from injected-key or activation behavior.

### Complete native input and rendered feedback

The 2026-09-23 continuation uses Computer Use `26.915.31945`, current screenshot-coordinate input
and observed keyboard focus. A separate diagnostic records key-down/up, physical/logical identity,
pressed keys, focus and lifecycle without consuming input or requesting focus. Tab and Shift-Tab
move focus; Enter reaches `library-retry-button`. Immediate accessibility snapshots can lag the
painted focus; a fresh observation confirms the target before activation. No product behavior,
permission, installed tool binary, retry oracle or native cancellation delay is changed. These
observations establish a usable input route, not the cause of earlier indexed-control failures or
proof that a plugin update alone resolved them.

The first lifetime `24fbe4c35a374c7bb67ccd89fa84c4ce` delivers the complete input ordering, but
the 250 ms observer misses cancelling between control dispatch at 191946 ms and the terminal event
at 191992 ms. Its failed result remains under the GUID-owned generated fixture, with exit zero,
225826 ms parent lifetime, retired Job/monitor and no cleanup failure. Full source checks pass
before/after in 45.748/42.488 seconds. This lifetime is not accepted as a complete pass.

The corrected diagnostic observes each rendered frame through a persistent/post-frame callback,
using the existing C05 method. It also requires rendered, nonempty Pause/Cancel label bounds.
Two focused widget cases pass for feedback shorter than 250 ms and retirement before a queued
observation. The single-use frame owner has 23 lines, its dedicated tests 38, and the composing
observation file 216; there are no inline tests or production changes. Formatting and fatal-info
Dart analysis pass; the final Debug build takes 26.6 seconds. The original focused-Enter,
registration barrier, exact native-event ordering, membership and source oracles remain intact.

Lifetime `d96c95f4411b4e7ebb25f7dba2061ea1` then passes the complete selected sequence:

- The actual picker imports exactly the three expected Chinese-named images, including PNG bytes
  under a `.jpg` name, and reports the damaged PNG with `image_decode_invalid`.
- Observed Shift-Tab traversal focuses Retry; Enter admits one committed display read at 119031 ms.
  No source rescan or resume occurs, and the task returns to visible completion.
- The second actual picker selects the frozen 10000-image corpus. Pause and Cancel both report
  not registered; late Started replays only Cancel, then the native stream drains as cancelled.
- Rendered Pause and Cancel labels are recorded at 150964/164787 ms. Ready-to-close at 164870 ms
  verifies exact original membership, no paused/recoverable checkpoint and current baseline pixels.
  Final observation records 11737 frames, 9934 baseline-pixel frames, 4220 cancelled-pixel frames
  and zero thumbnail Retry frames.
- The observed close button exits normally in 326.241 ms. The 195166 ms parent lifetime, process
  exit, Job closure and monitor retirement pass with no failure. Across 730 samples, peak working
  set is 443981824 bytes, sampled private bytes 343273472, kernel peak commitment 357355520 and
  minimum system availability 6164078592 bytes.
- Full source checks before/after pass in 45.387/42.360 seconds: four small files and all 10000
  frozen files (10921494393 bytes) preserve exact membership, bytes, identities and timestamps.

Ignored `.build/r2c-input-focus-frames/` retains the diagnostic sources and immutable admission;
`build/integration-storage-8174d9c28f104ac19f8a5df4d1c5cada/` retains the result, membership, trace,
process, memory and source receipts. SHA-256 identities are:

- Debug kernel: `C7FB879DA04BC19EB8872E082236380FEB3DD48A7332D448EE619FA89621437F`.
- Trace: `81FCC16B55864407F28072CE028E737B733B7C2BFD9B5B1972940E5F3DF71119`.
- Result: `59FB9887B0FB06B7166185821F79E384293D70B70AE4F654606C1E8BAE07DADA`.
- Observer: `077FACC6FECD81B64AC5369E9C0669010CEFD2696F65148DBFA9B1815539328B`.

The executable and Rust DLL retain their earlier recorded identities. The two reservations are
charged in full, bringing the conservative envelope to 3774 minutes. No real source, complete
large-library rescan, Release acceptance or full Daily is implied by this selected input pass.

An independent read-only review confirms the picker, focused-Enter admission, exact native control
ordering, rendered cancellation, observer retirement, membership, source and process receipts.
It finds no blocker within this selected Debug sequence. Keyboard Cancel, complete focus return,
Release and real-library acceptance remain outside this result; the review does not rerun them.
