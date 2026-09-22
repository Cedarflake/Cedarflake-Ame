# R2c process lifecycle verification

Status: **Pending native start/poll close verified; real-input restore and continuation incomplete**.

This record covers the 2026-09-22 generated-only method in the
[active execution plan](../plans/r2c-closeout.md#remaining-process-lifecycle-and-input-method).
Production source was `2004bafa56eec9c58c7fe6e3fd4f4f876cde5db6`. It adds no product behavior,
changes no acceptance threshold, and does not establish complete UX-01/02/08 or R2c acceptance.

## Boundary verification and preparation

The current shutdown, synchronization lifecycle, scan-control and retained-task interaction suites
passed 63 tests: 3 lifecycle-owner, 5 window-action, 15 scan-control, 32 synchronization and 8 retained
interaction cases. Those controlled races include late success/failure and pre-registration control;
they do not prove actual operating-system input or a complete process restart workflow.

The native entry uses the real bridge, real native start tickets and stop fences, the unchanged
250-millisecond poll policy, existing window close action and six-second shutdown bound. In-memory
preferences and marked Debug-only derived storage isolate it from the user catalog. An owned Job,
900-second parent deadline, 2 GiB client ceiling and 2 GiB minimum available system memory bound each
lifetime. The planned two interactive lifetimes also share a 30-minute deadline.

The first seed run `4c74a0d34be64a58a21241a6a961363c` failed with
`first_import_continuity_preflight_timeout`: the diagnostic invoked scanning before starting the
production synchronization runtime. The native first-import owner correctly returned not-ready
without a runtime owner and preserved its 15-second preflight limit. The process exited 1 after
16988 ms; its Job closed and cleanup had no failures. Keep its directory, executable identities,
bootstrap error, 58 memory samples and source receipts. It supplies no successful lifecycle evidence.

The corrected seed first awaits the real synchronization lifecycle's successful start. It uses a
fresh derived store and generated baseline while retaining the failed stores. Production source,
preflight, polling and scan/publication guards remain unchanged.

## Native results

| Phase | Run identity | PID | Result |
| --- | --- | --- | --- |
| Seed | `9163c521a30f4abeb26c94513e75da77` | 33956 | Passed; 12 published baseline images and a durable paused first-import checkpoint with 163 visited entries, 151 accepted images and zero issues; normal close 459.082 ms |
| StartClose | `7c42595bbecc4722aff225d98df6c844` | 30608 | Passed; close dispatched while the real start future was pending; same checkpoint and baseline retained; normal close 1307.317 ms |
| PollClose | `9a34a81942fe4ac5b154e262784d40ba` | 36912 | Passed; close dispatched while the real poll future was pending; same checkpoint and baseline retained; normal close 338.960 ms |
| Restore | `7f547c9c7e254ecb9b8dd4730d4223e6` | 3752 | Incomplete; paused task and baseline pixels observed, but desktop input could not be delivered reliably; owned process deliberately retired with exit -1 |
| Continue | No admission | None | Not run after the failed interactive phase |

Both pending-call targets naturally completed before native stop completed in these executions.
Their real overlap proves the close path while a call is outstanding, not a native result arriving
after stop. Controlled delayed-result tests retain that separate race obligation. All three passing
phases exited normally with no Job/monitor cleanup failures. Forced retirement of Restore is never
counted as normal-close evidence.

The observation review found two potential false passes. Memory records now use create-new run
identity and verify run/PID/parent ownership, including distinct PIDs across phases. Parent
verification now reads the complete trace after process exit and rejects late diagnostic failures,
running publication after stop, missing target settlement, and missing application-state observation.
A next-event-turn read observes the actual state after the application processes the bridge result;
this also detects a silent state change after the watch stream has closed. It delays neither the
original result nor shutdown. An unchanged initial stopped state need not emit another notification.
The evidence guardrail passed one positive and ten rejecting fixtures, including silent late state
mutation. Independent review and two changed-boundary rechecks consumed about 7 minutes 50 seconds;
the last scoped review found the previously reported observation gap closed.

There was one recorded diagnostic-only kernel transition after the successful seed, before
StartClose. Every native executable/DLL and production source stayed unchanged; subsequent phases
used the same new kernel, SHA-256
`6C2145B9D352E249AE915BCCBCF96EF99ADF6D9ABE38C3117A85762CBA60D839`.
Preparation, transition, admissions, raw traces and receipts remain under the ignored
`.build/r2c-process-lifecycle` and `.build/r2c-process-lifecycle-ready` diagnostics and their named
GUID-derived stores. Their Dart analysis and Debug builds passed. Initial diagnostic import/bracing
lint failures were corrected before native admission; they are not product verification failures.

## Source and resource evidence

The frozen corpus contains 10000 generated mixed-size/historical images, 10921494393 bytes; the
fresh baseline contains 12 images, 34549397 bytes. Full read-only checks cover exact membership,
content hashes, file identity, frozen size, creation time and modification time. The initial check
passed in 45.170 seconds, the failed-seed post-check in 41.110 seconds, and the corrected-set post-check
in 57.103 seconds. Baseline records prove identity, bytes and stable modification time during each
read; they do not independently freeze its original creation/modification times as the large oracle
does. No source mutation, hydration or real-library access was part of these lifetimes.

| Phase | Samples | Peak working set bytes | Sampled peak private bytes | Kernel peak commit bytes | Minimum system available bytes |
| --- | --- | --- | --- | --- | --- |
| Seed | 61 | 434163712 | 384307200 | 451358720 | 8801300480 |
| StartClose | 6 | 381964288 | 321007616 | 357277696 | 8173383680 |
| PollClose | 6 | 385294336 | 329330688 | 451608576 | 8172642304 |
| Restore | 1138 | 489881600 | 462151680 | 468815872 | 7592394752 |

## Remaining input and final-gate limits

The desktop tool returned `unknown screenshotId screenshot-0` twice, including after fresh window
selection, and `coordinate input geometry is unavailable` for the accessibility-targeted click.
A Tab operation returned without error but the client recorded no keyboard event. Focus subsequently
belonged to another application. Input was stopped; only the verified test executable, parent PID
and admission-time process was retired. The test process and monitor exited, its Job closed, and
cleanup reported no failures. No further input was sent to other applications.

Restore cannot claim settings/menu/viewer keyboard interaction, an unchanged checkpoint after a
normal interactive close, or explicit continuation. Continue, actual OS picker/pre-registration
actions, the full local UIA path and Release input remain open. Demonstrate a corrected driver on a
harmless surface before another admitted interactive lifetime; do not replay this set unchanged.

The [preview contract correction](r2c-browsing-diagnosis.md#preview-contract-ci-correction) retains
the concurrent hosted failure and focused follow-up. Its test-only changes do not invalidate these
native production artifacts. The next current-head hosted gate, complete local Daily, C01/C02 and
externally gated acceptance remain separate obligations.
