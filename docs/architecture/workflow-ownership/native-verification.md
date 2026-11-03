# Native verification ownership

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

The diagnostic release facade composes physical-path admission with separate journal-broker
owners for native interop, transaction state, write-ahead moves, uninstall progression, interrupted
recovery, service control and application deployment. Each transaction owner retains its complete
validation and cleanup policy. Their dot-sourced functions share the existing script constants;
the extraction adds no new mutable transaction state or alternate entrypoint. Installer guardrails
inspect the complete module roster. These historical diagnostic scripts do not replace ADR 0027's
production MSI lifecycle or grant service-installation authority.

`acceptance_r2c_cargo_invocation.ps1` owns exact Cargo command admission, including the locked,
single-job library test preparation command. Its command policy retains digest validation inside
the existing audited process boundary. `acceptance_r2c_worker_report_guardrail.ps1` owns ordered
build preparation and report-test execution, distinct deadlines, bounded failure output, and
independent environment, fixture and repository-lock retirement. Preparation failure prevents test
execution; it cannot consume or extend the report test's execution deadline. The process execution
owner retains source snapshots, Job Object ownership and cancellation.

`acceptance_r2c_fixture_cleanup.ps1` owns R2c-R fixture retirement, bounded immediate remaining-entry
evidence and first-error preservation. Each known child, metadata handle, stream and root retires independently;
one failure cannot skip a later owned resource. Unknown children are retained without traversal.
Metadata evidence opens at most 64 immediate entries without following reparses or recalling content,
records truncation, and grants no new deletion authority. The native resources owner retains identity
primitives; the common facade includes every owner in the audited source closure. Dedicated fixture-lifetime
tests exercise real handles and rejection paths independently of the full acceptance runner.
The first failure retains its original exception, PowerShell error text and script stack even
when later cleanup also fails; the regressions themselves use that same retirement boundary.

R2c-R compiler bootstrap owns the temporary compiler environment, handle identity and disposal
before native type initialization. Native resources remain lazily compiled in one verified unit.
Process execution and source audit are separate owners; the common facade retains case definitions,
execution-context admission and composition. The audit resolves static imports transitively before
function calls, keeping snapshots and source/depth/byte/AST/scope budgets bound to the same closure.
Module discovery never executes imports. Unresolved loaders, unknown commands, unsafe filesystem
operations and altered native/process digests still fail before process creation. The production
source budget remains nine; the larger test-only self-audit explicitly accounts for its own modules.

`integration_windows_accessibility_cleanup.ps1` owns independent process/job disposal, bounded exit
confirmation, exact environment restoration, and evidence-safe scratch disposition. Failure at one
stage cannot bypass later resource release; unconfirmed exit or failed output capture retains the
owned scratch evidence. The public runner composes acquisition and cleanup, while the evidence
owner persists structured stage failures without replacing the original run error. Cleanup tests
inject exceptional outcomes without starting Flutter or substituting for the native UIA gate.
Only an owned process tree grants termination authority; an unrelated process name or a retained
zero-thread process record does not.
The output owner requires exclusive log access after job closure and uses the remaining cleanup
budget to wait for asynchronous writer-handle release. Persistent sharing violations preserve the
raw output; they cannot be replaced with a partial read while a writer remains active.

Probe completion belongs to the evidence owner. Standalone probes require identity, process exit,
complete status and cleanup. The integrated canary retains one owned MTA probe process after runtime
preparation: each phase uses a fresh sequenced request, result and parent-owned completion event.
The worker signals completion only after the invocation and its cleanup return. The parent retains
the eight-second preparation/phase deadline and kills the owned job on a timeout or invalid result.
The runner persists validated results before acknowledgement, and complete-run success additionally
requires probe-session retirement. This removes repeated cold assembly loading from application
phases without extending their deadlines or weakening native assertions. Successful traversal
metrics must survive scratch cleanup, just as failed traversal evidence does.
The session owner manages process and invocation lifetimes; its protocol owner validates bounded,
sequenced messages and direct-child result names. The worker dispatches only the fixed preparation
and native-probe scripts, keeping native observations and assertions in their existing owner.

`integration_windows_accessibility_timing.ps1` owns monotonic accounting and validation for the fixed
internal probe stages. Evidence records preserve accumulated stage durations and publication time;
the process owner records parent wall time on failure. Stage totals include their progress-write
cost, which is also accumulated separately. The final record cannot include its own write or process
exit, so child timing alone never proves compliance with the parent deadline. Timing is diagnostic
only and does not change phase identity, completion, process retirement or acceptance assertions.

`integration_windows_accessibility_assembly_progress.ps1` owns six fixed cross-process loader
milestones, fresh named-event ownership, partial-acquisition cleanup and observation after confirmed
process exit or a returned session invocation. The loader signals entry, return and contract verification
without disk publication.
The process facade composes that owner with its existing deadline and Job Object; the evidence
owner includes the snapshot without granting it success authority. Missing or incomplete markers
cannot establish which native call stalled and cannot bypass complete-result or cleanup checks.
