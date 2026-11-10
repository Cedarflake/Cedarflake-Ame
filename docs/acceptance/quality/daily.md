# Daily gate

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Daily gate

```powershell
./tool/quality_verify_daily.ps1
```

The default command above remains the only workstation Daily invocation and runs every component
serially under the repository mutex. The hosted reusable workflow may select one validated
component with `-Component`; those partitions are intended for isolated GitHub-hosted runners, not
parallel local shells.

Repository quality, Flutter test, integration, bridge-generation, and Windows release commands
share one named operating-system mutex. Nested repository gates may reuse it on the same PowerShell
thread, while a second process waits instead of starting another Dart compiler or Flutter tester.
The daily gate expands `test` and executes every widget-test file separately with
`--concurrency=1`, preventing suite-level state leakage and avoiding Flutter's processor-count-based
default concurrency.

Rust's complete all-target/all-feature suite likewise uses `--test-threads=1`. Its controlled
latency and shutdown cases already create concurrent P0, P1, P2, observer, and competing-writer
work inside each fixture. Running unrelated synthetic catalogs alongside those cases measures
test-runner resource contention as well as Ame scheduling. Case-level serialization isolates that
measurement without filtering tests, disabling fixture concurrency, or changing any runtime
deadline or latency assertion. Hosted Daily partitions remain isolated from one another.

`quality_test_rust.ps1` owns this complete Cargo invocation. With no arguments it runs the entire
suite once, serially, under the same repository lock. Only the hosted workflow fans out work onto
separate runners; no local entrypoint starts jobs, parallel shells or concurrent test processes.
Its four hosted partitions separate schema migrations, change queue and organization; remaining
catalog tests; synchronization, scan and classification; and all remaining tests. Each runner first
enumerates the complete compiled all-target/all-feature inventory and requires every item to have
exactly one partition.
New modules enter the remaining partition automatically; duplicate names across targets retain
their multiplicity. Each partition then runs native libtest filters with `--test-threads=1` and
the existing ignore policy. Coverage, argument, matrix and local serial-lock guardrails run in lint.

Hosted Rust partitions use the runner's disposable temporary directory for fixture catalogs and
sources by setting process-local `TEMP` and `TMP` in the workflow. The workstation entrypoint and
its storage selection are unchanged. A bounded preflight records eight durable 1 MiB writes in
both the inherited system temporary directory and runner temporary directory before testing;
it removes only its own probe files. This records storage conditions without relaxing SQLite
durability, workloads, timing assertions or test isolation.
The ordinary native-journal fixtures also retain the inherited system temporary directory as a
candidate through `CEDARFLAKE_AME_ORDINARY_JOURNAL_PARENT`. They create an owned temporary child and
verify its existing journal capability before use; they neither enable a volume journal nor skip
restart coverage when no capable fixture volume is available.

The static bridge gate does not treat the generated content-hash constant as sufficient freshness
evidence. In addition to matching the Rust and Dart hashes, it checks timeline load, root removal,
and catalog-reclamation start, snapshot, and cancellation end to end: the Rust API source must not
declare these functions `sync`, the Dart API and generated interface must return `Future`, generated
Dart must dispatch through `executeNormal`, and generated Rust must use both `wrap_normal` and
`FfiCallMode::Normal`.

Run focused Flutter tests through the same lock-aware entrypoint:

```powershell
./tool/quality_test_flutter.ps1 `
  -TestPath test/features/library/presentation/library_viewer_position_test.dart
```

Process cleanup is never based on all newly observed Dart or Flutter processes. The Windows scan
integration gate holds the repository mutex and an owned process-tree Job Object. A matching runner
path or a process appearing after startup does not establish cleanup authority. Other commands may
stop only a process that their own process tree or an equivalently isolated identity proves they own.

The Windows scan integration fixture contains only controlled temporary files under `build`. It
opens the real directory picker through automation, exercises scanning and preview publication,
verifies that source bytes remain unchanged, and retains nonempty storage as evidence. Cleanup must
prove ownership and cannot recursively remove that storage merely because the run has ended.

The Windows accessibility integration runs a synthetic 1,200-item virtual gallery and the populated
application shell in the native Windows runner with semantics enabled. It performs distant gallery
jumps, repeatedly opens and closes on-demand photo menus, operates stable toolbar and source menus,
changes the timeline and viewer Sliders, and returns from the viewer twice. The script retains the
complete engine output under `build`, rejects any `Failed to update ui::AXTree` record even when the
Dart assertions pass, and therefore exercises the platform `AccessibilityBridge` behavior that
widget-test semantics models cannot reproduce. Flutter 3.47.8 exposes Windows device integration
tests through its supported Debug test path; this canary does not claim Profile-mode coverage.

The native probe loads only the fixed, strong-named Windows UI Automation assemblies through
`integration_windows_accessibility_assemblies.ps1`, validates their identity and required types,
and reports type/client loading separately from native traversal. It does not initialize the
`Add-Type` command or fall back to a same-named DLL in the working directory. The assembly guard is
part of `integration_test_windows_accessibility_guardrails.ps1`. The existing eight-second parent
deadline, MTA client, owned process tree, and native positive/negative assertions remain unchanged;
workstation PowerShell evidence does not replace execution in the hosted PowerShell runtime.

An owned MTA session validates these dependencies before starting Flutter and retains the loaded
assemblies for the ten application phases. Preparation and each requested phase retain their own
eight-second parent deadline within the existing complete-run budget. Fresh sequenced requests,
owned completion events and validated results separate the invocations; the worker signals only
after the phase and its cleanup return. Preparation cannot acknowledge an application phase.
Timeout or invalid evidence retires the whole session, and complete-run success requires session
retirement. `integration_test_windows_accessibility_session.ps1` covers real dependency reuse,
request ownership, incomplete or invalid results and a blocked invocation that already wrote a
complete result file. The same guardrail entrypoint runs these checks.
`integration_test_windows_accessibility_preparation.ps1` covers real child loading, complete assembly
milestones, zero native observations and refusal of an application-phase request. It runs through
the existing accessibility guardrail entrypoint; actual UIA assertions still run in hosted integration.

Six parent-owned manual-reset event markers distinguish each assembly's load entry, load return
and completed contract validation independently of progress-file publication. The parent observes
them only after confirmed probe exit or returned session invocation and retires every handle. They are
diagnostic milestones,
not completion evidence; no marker can replace the owned result, deadline, phase or cleanup checks.
The compiler-free `integration_test_windows_accessibility_assembly_progress.ps1` is included in
the same guardrail and covers actual loading, rejected ownership, partial acquisition, read failure,
and identical persisted stages before versus inside a controlled blocked loader.

Validated successful probe records, including traversal counts and timing, are retained in the
same transcript before acknowledgement; they do not replace the ten ordered phase assertions.
`integration_test_windows_accessibility_probe_evidence.ps1` exercises completion identity, original
failure precedence, and the actual transcript composition without running Flutter, and is included
in the accessibility guardrail entrypoint.

Output capture waits for exclusive access after closing the owned application job, using only the
remainder of the existing five-second cleanup budget. Only Windows sharing/lock violations enter
that wait; other read failures remain immediate. Unreleased writers retain the raw output and fail
cleanup. The output guardrail checks delayed writer completion, bounded failure and unchanged
evidence; a readable partial log cannot substitute for writer release.
