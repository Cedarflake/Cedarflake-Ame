# R2c closeout execution

Status: **functional-first continuation active; C01/C02 deferred and candidate remains unresolved**.

The [cycle evidence](../acceptance/r2c-closeout-cycle.md) records the frozen source, selected
variants, execution budgets, findings, and verification. Its status does not accept R2c.

This document supplies execution details for the
[canonical roadmap's closeout queue](../roadmap.md#current-execution-queue), not a second product
stage or independent priority list. Its outcome is one reviewed candidate with evidence for the
fixed workflows below, plus an explicit disposition of every discovered issue. It does not
promise that all possible defects have been found or that R2c's external acceptance is complete.

The first complete Daily on candidate `fa1eff0` fails after the independent review and its scoped
recheck, in the existing R2c-R owned-fixture cleanup guardrail. The cycle record owns the exact
failure and retained evidence. The dispatched hosted run also fails its connection-lifetime control;
the separate production full recovery passes. The original two C01 diagnostic passes and original
review/recheck are exhausted. The supplement below has now reached its own stopping boundary.
The owned-fixture repair is reviewed and passes current-source guardrails and lint; the second
complete local Daily fails C01's full recovery at the unchanged 300-second bound. Current hosted
checks pass, but do not replace that failure or explain the earlier C01/C02 outliers. Both complete
Daily invocations and both supplement review invocations are consumed. Historical failures,
invocation counts and active time remain charged. The explicitly approved P2 follow-up below has
also stopped; no unchanged replay allowance is renewed.

The latest instruction changes execution order: complete directly user-facing functional work
before returning to C01/C02's remaining validation obligations. It supersedes the earlier requirement
that item 2 exit before item 3 starts, and the earlier stop on all work after the P2 experiment.
It does not authorize another C01/C02 experiment or turn their failed gates into passing results.

### Functional-first continuation

Treat the requested highest-priority P0 defects separately from synchronization's P0/P1/P2 lanes.
Source/durable-data harm, wrong results, crashes, blocked normal workflows and unusable browsing
take precedence. C01 retains potential background-recovery impact without a proven current normal-
use incident; C02 currently has automation/evidence failure evidence without a reproduced ordinary
user-input defect. Both remain recorded final-verification obligations. Any newly reproduced direct
impact brings the owning issue forward immediately; deferral is not a severity waiver.

Continue the same 24 variants, using existing passing boundary evidence where source still matches.
The next connected observation is UX-01C/02B: create a paused first import through the real Rust
scanner, retire that process, launch the actual Debug client on the same isolated catalog, verify
paused restoration and usable navigation, and require explicit Continue before publication. Check
real previews, folder/query navigation and viewer actions while using this same fixture. A completed
scan that outruns Pause does not count as interrupted-import evidence.

Admit one additional controlled desktop session of at most 30 minutes, with its prepared seed
process and at most three actual client lifetimes inside the same session deadline. Reuse the
existing owned-process, Debug-only storage marker and production application entrypoint. Use only
generated sources and derived storage outside them; no retained-library or Release storage bypass.
Confirm every owned exit, paused/completed checkpoint, exact counts and fixture-byte integrity.
Keep builds/tests serial and retain raw output. Do not repeat unrelated passing heavy suites.

Preparation, the bounded observation and result inspection stay inside the original remaining
discovery allowance (64 active minutes before this continuation); no phase counter resets. At most
one independent delegate prepares the fixture without running concurrent heavy tools. Repair only
a reproduced functional defect at its owner, with its focused regression and applicable gates.
Do not consume the conditional complete Daily for an unrelated observation; final-source readiness
still requires resolution of C01/C02 and all original acceptance exits. Low-impact and unproved
tool symptoms are registered and deferred rather than stopping independent functional work.

### Representative mixed-media and historical-date discovery

The latest requested coverage adds a representative workload to UX-03A/04A/05A/08A; it does not
create more than 24 journey variants or replace their existing interleaving assertions. The earlier
1x1 PNG session proves only process restoration, explicit continuation and exact publication.
The existing 10000-file benchmark uses 2x2 PNGs; the large JPEG and seven-format cases remain
separate adapter evidence. None establishes mixed large-image desktop usability.

Keep the cumulative 16-hour ceiling. Explicitly reallocate 90 minutes from unspent diagnosis/repair
to discovery: discovery is now at most 330 minutes and diagnosis/repair 390 minutes; triage,
review and recording retain their original ceilings. This requested extension admits one pass of
at most 90 active minutes including preparation, delegated work, observation and result analysis.
It does not renew C01/C02 experiment or Daily allowances. Record actual charges and stop the
affected lane on its fixed resource/deadline boundary; do not retry until green.

Freeze exactly 10000 independently created source files, 9000 JPEG and 1000 PNG. Each row has
10 percent PNG. Use three deterministic textured templates per size/format, recording the 72
templates and repeated-content limitation rather than calling them unique real photographs.
Do not use hardlinks or preload all decoded images. Every file has a manifest identity, dimensions,
encoding, source bytes, hash, effective historical date and metadata provenance.

| Dimensions | Files |
| --- | ---: |
| 640 x 480 | 700 |
| 800 x 1200 | 700 |
| 1024 x 1024 | 600 |
| 3840 x 2160 | 2000 |
| 2160 x 3840 | 1500 |
| 4000 x 3000 | 1500 |
| 7680 x 4320 | 1000 |
| 4320 x 7680 | 1000 |
| 6000 x 4000 | 500 |
| 12000 x 1500 | 150 |
| 1500 x 12000 | 150 |
| 8000 x 8000 | 200 |

Distribute dates across January 2010 through September 2026, with multiple days per month and
dimensions interleaved across dates. Cover EXIF original-capture dates on a JPEG subset and
filesystem-date fallback on other files. ADR 0008 selects creation time before modification time
when capture evidence is absent. During fixture preparation, set non-EXIF creation time and mtime
to the planned historical date; keeping only an old mtime with a new creation time would not
exercise historical default groups. For the EXIF subset, give source files recent copy-time
creation and modification dates distinct from capture time, then verify that capture-time ordering uses the owning metadata
contract. Compute expected year/month/day counts from the fixture manifest before import; current
date, copy order or an observed UI count is not the independent oracle. Unsupported fixture
construction must be reported before execution, never silently replaced with one common date.

Use the existing Debug-only isolated-storage client and owned-process boundary with real Rust
catalog, preview and viewer adapters. No retained roots, user catalog, source hydration, signing,
installed-service changes, new dependency, or Release storage bypass is included. Data stays in a
fresh GUID fixture outside all original source trees. No source mutation is permitted after the
fixture and expected-date manifest are frozen; stimulus files for synchronization are prepared
separately and recorded before they are intentionally introduced.

Resource limits: require at least 64 GiB free on the fixture volume, 3 GiB available physical
memory before generation and 4 GiB before client observation. The initial combined 6 GiB preflight
refuses execution when the workstation has approximately 5 GiB available; no workload runs in that
attempt. Separate the serial phases with stricter process ceilings: generation at most 1 GiB and
the client at most 2 GiB, each preserving the 2 GiB system reserve. This changes no source count,
image size or existing gate. Cap generated source storage at 24 GiB, template storage at 2 GiB,
generation at 900 seconds, and the one desktop session at 30 minutes. Retain the existing owned
Job and repository lock, raw stdout/stderr and exact executable/DLL/kernel hashes. Observe the
client's working set and private memory; stop its owned process on more than 2 GiB, available
system memory below 2 GiB, a crash, or the session deadline. A limit breach is a failed observation,
not authority to shrink the corpus, hide evidence or raise the threshold. Logs remain bounded.

Execute one cold import through the real picker; require exact completed inventory, no unpublished
partial replacement and truthful per-file issues. Observe cold first-screen demand, forward/reverse
scrolling, distant year/month jumps, at least 12 large-image opens spanning every dimension, rapid
viewer direction changes and return anchors, then repeat the observed pages warm. Check root,
search and chronology counts against the manifest. Include one bounded generated-source update
during browsing if the earlier observations remain safe; isolate this stimulus from byte-integrity
proof. Record visible failures, responsiveness, preview convergence and memory, with timestamps.
Debug and computer-use timings are diagnostic observations, not Release frame-rate acceptance.

Exit requires preserved raw failures, exact source-byte integrity, metadata/date oracles, owned
retirement and a disposition for each observation. Bring a reproduced blocked or misleading core
workflow forward for an owning-layer correction. Tool-only, cosmetic and unproved symptoms remain
registered for final work. An incomplete observation remains a coverage gap. Do not call all R2c
accepted, skip the Release/client requirements or replace C01/C02 with this workload.

The 2026-09-10 continuation after the account-quota interruption admits one additional client
lifetime of at most 20 minutes to complete the unfinished observations. Charge its preparation,
observation and analysis to the same 90-minute mixed-media allowance and 330-minute discovery
ceiling; retain the first lifetime and its wall time rather than resetting either counter. The
first catalog subsequently contains an additional non-fixture root and is preserved without
relaunch. Reuse the unchanged frozen 10000 source files with fresh GUID-isolated derived storage;
repeat import only to establish that uncontaminated catalog. Keep all dimensions, counts, date
oracles and memory limits. Record the interrupted action as unexecuted and the additional-root
provenance separately; no real-root reads are admitted by this continuation.

Both mixed-media lifetimes have ended. The first is interrupted by tool quota; the continuation
reaches its fixed parent deadline. Preserve their distinct process receipts and the original
additional-root oracle failure. The resumed exact source/catalog/date oracle passes and actual
viewer observations span all twelve dimensions, but unfinished client observations remain gaps
in the cycle record. The 90-minute mixed-media discovery allowance is consumed. No additional
client lifetime, higher deadline or unchanged final-gate replay is admitted by this result.

### Approved P2 completion follow-up — unresolved result

The 2026-09-09 approval admits the following diagnosis, repair, review and gate executions.
It does not reset prior usage, reopen discovery, accept the candidate or remove C02/client blockers.
The existing branch and original 16-hour total and phase ceilings remain binding.

The capture prerequisite passes, but the one selected differential experiment returns identical
plans, exact values and VM instruction counts for the original query and owner-first alternative
on its controlled relational projection. This rejects that proposed correction on the projection;
it does not explain the original full recovery timeout or rule out other query/production costs.
No causal repair is admitted from this result. Experimental Rust changes are removed after
retaining their source and raw output. No focused full workload, new C02 run or conditional third
Daily is started. Further experimentation requires a new bounded decision; unused time is not
another experiment allowance. The [cycle record](../acceptance/r2c-closeout-cycle.md#approved-p2-follow-up)
owns the exact results, limitations and charged time.

**Question to resolve.** In the retained failed local run, the tail lasts 196185 ms across 6375
iterations: application polling accounts for 113671 ms and the test's separate exact-evidence
queries account for 66329 ms. `recovery_completion.rs` currently reads that evidence after each
poll, followed by a two-millisecond sleep. These non-overlapping tail measurements are distinct
from the nested, whole-fixture counters. They justify testing whether exact-evidence query work
materially delays recovery; they do not prove that cause or excuse a production bottleneck.

1. **Make capture reliable before a workload.** Use the existing owned-process boundary with raw
   stdout/stderr redirected to separate files, retaining child exit and cleanup results. Prove
   complete capture and original failure propagation with a bounded compiler-free output fixture
   larger than the prior tool-output limit. Keep logs in ignored derived storage. No full workload
   is replayed merely to replace a missing transcript.
2. **One diagnostic pass, at most 60 active minutes.** Inspect the existing exact-evidence query,
   its query plan/work counts and P2 comparison/publication owner using controlled catalog data.
   Compare equal input states and exact returned evidence; distinguish query work from worker
   waiting and progress. Admit at most one controlled differential experiment, chosen and recorded
   before execution. Keep production behavior unchanged until it yields a causal counterexample.
   A passing rerun, machine-speed difference or a smaller observation total alone is insufficient.
3. **One causal repair, at most 30 further active minutes.** Correct the proved owner and add its
   boundary regression. A test-oracle correction must preserve the same coherent completion proof
   and establish semantic equivalence; a product defect requires a product-owner correction.
   Do not choose an easier test-only fix without that evidence. Preserve the 25 P0 samples, both
   connection-lifetime arms, 2048 P1 items, 10000 P2 entries, 4095-entry page, two-millisecond tail
   interval, original creation-to-reopen deadline and every final publication/reopen assertion.
   No deadline increase, source-proof weakening, skipped count/result, new schema strategy or
   broad refactor is covered. Run one focused unchanged full-workload validation after correction.
4. **Review and verify once.** Permit one independent review of this correction plus one scoped
   recheck, at most 25 active minutes combined. After causal regression, focused workload and lint
   pass, freeze the source and permit one additional complete Daily plus applicable existing
   hosted/unsigned gates. All local heavy work remains serial. A failed final gate is preserved
   and ends this follow-up; it grants no further replay. Documentation alone adds no heavy gate.

This adds at most 90 diagnosis/repair minutes and 25 review minutes: cumulative phase charges
would be at most 370/480 and 46/120 minutes respectively. Preparation/closeout stays within its
original one-hour allowance. Use the existing 24-variant roster and at most one independent
delegate, without nested delegation. Unknown causality, an exceeded allowance or a materially
different repair ends execution at an unresolved checkpoint. C02's historical UIA/1175 failures,
missing client evidence and external acceptance remain explicit blockers, with no new C02 run
authorized by this P2 follow-up. The full goal remains incomplete until its original exits close.

### Supplement scope and consumed allowances

The preceding consumed supplement covered concurrent diagnosis and causal repair of both named blockers on
`codex/r2c`, replacing the earlier unadmitted 45-minute diagnosis-only proposal. This narrow exception
to the original serial family order covers C01 and C02/C03 file-lifecycle failures only. The complete
outcome is passing unchanged synchronization workload/control and reliable owned-fixture evidence
and teardown, with original failures preserved and applicable final-source checks complete.

- C01: inspect native operations inside the fresh-identity stall, distinguish operation cost from
  workload/harness interference, and repair the proven owner. Preserve both lifetime arms, original
  visible deadlines, 25 samples, full production recovery and all identity safeguards.
- C02/C03: capture the original exception, immediate remaining child identities and owned handle
  retirement. Investigate evidence replacement and fixture teardown separately until shared causality
  is proved. Reuse existing native safety primitives and retain the first unexpected failure.
- One primary implementation lane and one delegated implementation lane may proceed independently;
  neither delegates further. File ownership is disjoint. All local compilation, stress, native
  integration and full quality runs remain serial and coordinated through the repository lock.
- Each lane has at most 90 additional active minutes for diagnosis/repair, charged inside the existing
  eight-hour phase ceiling. Start with distinct written hypotheses and at most two new controlled
  diagnostic experiments per lane; every experiment must produce new evidence. Focused regressions
  and one post-correction workload validation are separate bounded verification. No unchanged replay
  allowance is renewed. Unknown causality at the limit produces an unresolved finding.
- The current hosted recurrence of C02's original eight-second UIA timeout additionally warrants
  one stage-timed execution of the unchanged native gate, within these same active-time ceilings.
  Timing records separate existing stage work from evidence publication and parent wall time;
  they do not relax the deadline, phase roster, complete-result proof or owned-process retirement.
- Admit one independent review of the supplement and one scoped correction recheck, together at most
  60 active minutes inside the existing review ceiling. Earlier invocations stay recorded. Each
  implementation must be reviewed by an executor independent of that change.
- After causal correction and focused checks, use the one remaining complete Daily invocation and
  applicable unsigned/hosted gates on frozen source. No unchanged reruns until green. Another failure
  is retained and assessed against this scope and remaining budgets.
- No new dependency/schema strategy, retry loop, relaxed deadline/workload/assertion, identity or
  deletion-guard weakening, tracing-software installation, real-library access, source mutation or
  hydration is admitted. No main merge or release. An unexpected safety incident stops its lane;
  another independent blocker or exhausted phase requires a documented bounded checkpoint.

Use the existing `codex/r2c` branch for this closeout, including fixes, investigation planning, and
documentation organization. Separate rollback boundaries with commits; do not create another branch
for each follow-up. This scope-specific workflow does not authorize merging into `main`.

The unit of work is an **owning invariant and its complete user workflow**, not an error message,
individual widget, or file. The gray preview, failed gallery refresh, and folder-reload symptoms
remain regression seeds; they do not justify speculative changes to every nearby component.
ADR [0024](../architecture/0024-windows-change-driven-library-continuity.md) owns continuity and source
safety; ADR [0025](../architecture/0025-invariant-owned-workflow-modules.md) owns responsibility
boundaries; ADR [0026](../architecture/0026-hosted-synthetic-and-unsigned-build-gates.md) and the
[quality-gate inventory](../acceptance/quality-gates.md) own verification scope.

**Sequence and required outputs**

1. **Freeze the baseline and coverage before execution.** Record the exact source commit, dirty
   state, Windows/SDK/build mode, executable/DLL identities, isolated storage, fixture counts,
   existing deadlines, resource ceilings, and current issue evidence. Inventory prior results by
   source identity and gate, including failed attempts. The browsing-recovery record is the latest
   starting evidence, not a complete Daily pass. Map each scenario below to existing production
   owners and tests, then select at most three named variants per row: a normal path, its critical
   interleaving, and a relevant failure/restart path. State the expected result and exact observation
   point before running each variant. A missing oracle or executable path is a coverage gap, not a
   passing case. This step exits with a frozen roster of at most 24 variants and an evidence map.
2. **Complete one controlled discovery pass.** Trace the frozen transitions through presentation,
   application, adapter, and publication ownership. Reuse applicable unchanged evidence; run the
   missing controlled paths and reproduce existing blockers with fixed fixtures. Use generated
   sources and isolated derived storage before any retained-library work. Add only the narrow
   deterministic probes or test controls needed to observe the named invariant; keep product
   behavior unchanged during discovery. Supplement with at most two 30-minute scripted desktop
   sessions covering the same roster. Record actual actions, expected/observed states, timing,
   logs, and source identity; screenshots alone do not establish convergence. Do not fix a symptom
   while the remaining roster is still being discovered. A demonstrated source-safety incident
   stops the affected execution immediately and follows the stop rules below.
3. **Triage once and freeze one repair batch.** Consolidate the pass in the existing
   [interleaving evidence ledger](../acceptance/r2c-interleaving-remediation.md), linking the dedicated
   browsing record rather than copying its transcript. Give each finding a stable ID, scenario,
   reproduction or unverified hypothesis, severity, owning invariant, causal evidence, affected
   callers, proposed boundary regression, and blocked queue exit. Group symptoms only when evidence
   supports the shared cause. Select at most three root-cause families; item 2's unresolved
   convergence/latency obligation remains first. Necessary item 3 repairs follow only when item 2
   exits. Nonblocking lower-impact or out-of-roster findings retain a reason and future owner
   without joining this batch. Any known S0/S1 in an existing R2c core workflow still blocks candidate
   readiness, even outside the roster. A fourth blocking family requires replanning before more
   implementation; moving it to a follow-up list cannot remove its blocking status.
4. **Repair each admitted invariant end to end, serially.** First establish a failing behavioral
   regression or a reproducible controlled measurement. Identify the application/persistence/
   platform owner and all affected callers before changing it. If the owner is already carrying
   unrelated state machines, establish the typed boundary required by ADR 0025 as part of that
   repair. Record production, inline-test, and dedicated-test size for large affected owners;
   forwarding fragments and arbitrary line-count reduction do not count as decomposition. Verify
   cancellation, stale completion, rollback, and restart where that invariant owns them. A repair
   is ready for batch verification only when its whole admitted workflow and affected regressions
   pass. Preserve item-specific prerequisites, including item 2's controlled and hosted latency
   evidence, before advancing the queue; final gates do not defer those prerequisites. Routine
   implementation choices inside this batch need no additional product-scope approval.
5. **Review and verify the frozen candidate once.** Use one independent reviewer for the changed
   range, causal explanation, missing transitions, and coverage gaps. Allow one follow-up review of
   the resulting changes; do not commission another whole-repository audit after a green review.
   Complete the final-source gates below after batch freeze, then issue the readiness decision.
   Additional blocking findings or repeated review failure leave the cycle unresolved and require
   a bounded revised plan; they do not automatically create another repair/audit round.

**Fixed workflow matrix**

The rows are risk-selected journeys, not a Cartesian product of every file format, root, UI action,
and failure. Baseline preparation names the exact variants and reused evidence within each row.
Only currently implemented controls are in scope.

| ID | User journey and critical transition | Required observable result and owning boundary |
| --- | --- | --- |
| UX-01 | Launch with an existing catalog; browse while continuity starts; close and reopen during work | Cached content stays usable; no-change startup has zero source enumeration/media opens/inventory creation. Shutdown respects its existing deadline and owned-process retirement. Interrupted first import requires explicit continuation; portable `LiveOnly` stays truthful. Owners: bootstrap, synchronization lifecycle, scan restoration, native window. |
| UX-02 | First import and replacement update; pause/cancel near registration or final publication; continue after restart | Exact published counts and task ownership agree. No false completion, lost cancellation, partial baseline replacement, or implicit cancelled-task restart. First import and replacement-update semantics remain distinct. Owners: primary scan lifecycle, execution registration, scan publication. |
| UX-03 | Browse during catalog publication; change source/folder/query/sort; paginate and retry a failed display reload | One coherent query snapshot; no mixed-revision count/page/timeline, duplicates, gaps, permanent folder error, or stale callback replacing the latest selection. Display retry does not repeat a committed scan/removal. Owners: query snapshot, viewport refresh, folder paging. |
| UX-04 | Load cold/warm previews; modify or replace the requested fixture during decode/publication; corrupt/lock and recover it | Newest proven source generation wins; old pixels cannot publish and a current valid source can recover. Failed/unavailable/pending remain distinct; unrelated catalog writes cannot leave permanent gray cards. Reuse existing actual-format and wrong-extension fixtures. Owners: source admission, reconciliation, preview lease/store. |
| UX-05 | Open an image, navigate quickly, close/reopen; receive source changes or removal while navigation is pending | Correct asset and source version, bounded source leases, retired reads released, no late image/error in the new session, and a valid gallery return anchor. No unchecked-path or placeholder-content fallback. Owners: viewer session, source reader, image stream. |
| UX-06 | Update multiple roots; cancel/remove one while another publishes; disconnect/reconnect a fixture root | Independent progress and truthful task feedback; no mass removal from unavailability, resurrected registration, repeated unregister, or other-root starvation. Exercise cleanup/registration overlap using owned fixtures only. Owners: update scheduling, root removal, retirement, cleanup admission. |
| UX-07 | P0 live changes during P1 backlog and P2 recovery; interruption/reopen and retry exhaustion | Preserve the original 25 P0 samples, P95 at most 1 second, all 2048 P1 and 10000 P2 items, 4095-entry page, and existing deadlines. Require complete P2 results, closing coverage, final publication, retired authority and FULL reopen; progress or the first page is insufficient. Owners: observer ingress, scheduling, catalog session, recovery/finalization. |
| UX-08 | Direct scrollbar/time jump and reversal; open/close menus; use retry/cancel with keyboard and accessibility active | Visible demand begins without extra scrolling; identity/geometry remain stable, controls have feedback, and focus remains usable. Retain the original ten-phase whole-window UIA and native exit checks. Owners: visible-range loading, shared menus, task surfaces, native accessibility. |

Existing query/folder, primary-scan, synchronization, preview, and viewer test files provide boundary
regressions; `integration_test/scan_workflow_test.dart` and the native scan/accessibility runners
provide connected paths. The test name or presence of a runner is not proof that every row is
covered. UX-07 uses the existing production mixed-load fixture. The controlled R2c-R runner supplies
its admitted Windows 11 continuity cases; a fixture broker cannot establish installed-service or
real-journal acceptance. Storage tests in UX-06 cover only interaction with registration/removal;
an audit of all settings, relocations, or storage algorithms is outside this cycle.

Current coverage gaps must remain explicit. The retained-import integration remounts the widget
tree; UX-01/02 require an actual owned EXE exit/relaunch for process recovery evidence. The native
accessibility scenario uses a static preview provider in Debug; retain that gate and separately
exercise real decoding and input in the Windows Release client for UX-03/04/05/08. The unsigned
bridge smoke initializes the DLL and accent channel without opening a catalog. Existing native
multi-root management sequences complete updates before removal, so they do not prove UX-06's
overlap. Retained Profile does not materialize source previews. These are missing evidence, not
newly diagnosed bugs. Add narrowly owned fixture/client verification only for the frozen variants;
do not build a second general automation framework to fill them.

**Severity, admission, and stopping rules**

Use `S0/S1/S2` for this ledger so severity is not confused with synchronization lanes `P0/P1/P2`.

The initial cycle has a **16-hour cumulative active-engineering ceiling**: baseline and discovery
4 hours, triage 1 hour, diagnosis and repair 8 hours, independent review/recheck 2 hours, and closeout
recording 1 hour. Code/document inspection, fixture and probe construction, implementation attempts,
test-result analysis, and the two scripted desktop sessions all count against their phase. These
are ceilings, not estimates or a promise to finish within two days. A failed attempt, new finding,
commit, or context handoff does not reset or silently transfer the budgets. Record bounded tool
execution/wait time separately from active work and report total elapsed time; existing tool
deadlines and the invocation limits below remain binding. Exhausting a phase or total ceiling
produces an unresolved checkpoint and a revised plan, not an automatic extension.

- **S0 — source or durable-data safety:** unintended source mutation/hydration, wrong-file access,
  catalog corruption, or lost durable intent. Stop the affected run, preserve evidence and owned
  resources safely, and prevent further exposure. Repair admission requires a proven containment
  boundary; a destructive experiment on a real root is never a reproduction method.
- **S1 — core workflow blocked or misleading:** valid catalog browsing remains blank/unusable,
  tasks never settle within their existing bounds, current images stay stale, normal recovery
  requires restarting the app, a committed action is repeated, false freshness/completion appears,
  or required latency/convergence/native interaction gates fail. These block candidate readiness.
- **S2 — localized, recoverable impact:** issues that preserve the workflow, authority, and existing
  acceptance invariants. Record and defer cosmetic polish, minor copy, and speculative optimization.
  A mandatory gate failure cannot be relabelled S2 to obtain a pass. Unverified symptoms remain
  explicitly unverified; credible S0/S1 evidence cannot be dismissed for lack of an easy repro.

For each family allow at most two focused diagnostic passes, each with a written hypothesis,
measurement, and a 60-minute active-investigation budget. Existing bounded tool runtimes are
recorded separately and are not shortened to fit that budget. Permit at most one unchanged isolated
replay to distinguish an execution-context failure; preserve both results. If causality is still
unknown, stop work on that family and report the evidence, rejected hypotheses, remaining gap, and
smallest next decision. Elapsed time never converts a defect into a pass or authorizes more rounds.

This cycle contains one discovery roster, one batch of at most three families, one independent
review plus its scoped recheck, and the final verification below. New findings go to the ledger;
only an S0 incident or a proven blocker of the active exit may interrupt order, with the reason
recorded first. More blocking families, an exhausted diagnostic/review budget, a proposed architecture
replacement, or a new dependency/schema strategy that changes the agreed boundary ends automatic
expansion: publish an unresolved checkpoint and revise scope before implementation continues.
Nonblocking evidence collection may continue only inside the already frozen roster and budget.

**Verification and exit**

- Run focused boundary regressions and applicable lint for each admitted repair; do not rebuild
  the application after every small edit. Reuse successful evidence only when its owning source,
  behavior, workload, and environment remain applicable, with that relationship recorded.
- After freeze, run one complete serial `./tool/quality_verify_daily.ps1` on the candidate. Retain
  its complete Rust/Flutter/native/bridge result, including the original local whole-window UIA
  path and process cleanup. Partition results, an isolated mixed-load pass, a screenshot, MSAA,
  or a standard-window control do not replace this result. Performance evidence must distinguish
  current product cost, observer/tool cost, and unexplained historical attribution.
- Preserve the first failed final gate. One further complete invocation is allowed only after a
  documented causal correction within the admitted batch or a verified execution-environment
  remedy; unchanged reruns until green are forbidden. If it still fails, the cycle remains open
  for a revised plan. Never raise deadlines, shrink workloads, remove assertions, skip required
  tests, or treat partial P2 progress as completion to satisfy this budget.
- Use the applicable unsigned Windows build and existing hosted PR gates for the final product
  source, including the five required synthetic workloads. Run heavy local work serially. A
  documentation-only change does not invalidate an unchanged product artifact or require those
  heavy gates. Hosted Windows Server and unsigned bridge smoke do not prove Windows 11 hand-feel,
  retained-library behavior, signing, service installation, or real broker FSCTL.
- Before retained-library client acceptance, prepare a separate concrete run description: logical
  roots, bounded scenarios/sample counts, read types, duration/resource limits, isolated derived
  storage, artifact identity, source/placeholder checks, and stop/cleanup behavior. Obtain the
  existing required current authorization only after that description is reviewable. Prior
  authorization or the local root mapping does not authorize a new run. If unavailable, report
  controlled verification and client acceptance separately; keep the latter pending.

The controlled cycle exits only when every frozen variant has current passing or justified reused
evidence, all admitted root-cause regressions pass, no known S0/S1 remains in existing R2c core
workflows (including findings outside the roster), the required final-source gates pass, and the
reviewer confirms the bounded change and its evidence. An unrun,
blocked, or unexplained required case prevents this exit. Record deferred S2 work, invalidated
evidence, source-safety results, and outstanding external/client gates explicitly. Deferred findings
do not satisfy ADR 0024's separate final R2c audit requirement. Do not advance R3 or call all of R2c
accepted when only this controlled cycle has closed.

**Execution cautions — binding for this cycle**

- Diagnose before changing behavior. No extra retry/debounce loop, broad catch, cache wipe, automatic
  full scan, or status-text change may conceal an unresolved owner invariant. Do not weaken source
  identity, generation, revision, lease, cancellation, or atomic-publication guards for apparent
  responsiveness.
- Preserve original media, durable catalog/user intent, and the last trustworthy baseline. Perform
  source edits, replacements, corruption, root loss, and crash injection only in fixture-owned
  storage. Never reset the user's catalog or clear its cache merely to make a reproduction disappear.
  Record the harness's intended fixture changes separately from before/after integrity observations
  so an intentional stimulus cannot conceal an unintended application write.
- Keep real paths, account labels, private catalogs/media, and raw user logs out of tracked evidence;
  use `local-primary` and `cloud-primary`. Do not hydrate cloud placeholders or change journal,
  service, permissions, certificates, signing, or system memory settings as an incidental test fix.
- Keep one implementation owner and at most one useful delegated reviewer. No nested delegation,
  parallel heavy builds, duplicate full audits, or unrelated process termination. Retain original
  failures; terminate only a verified owned process tree, and preserve scratch when cleanup ownership
  or source identity cannot be proved.
- Stay within implemented R2c workflows. No new features, framework migration, general UI redesign,
  wholesale file splitting, broad SQL tuning, new observability platform, or repository-wide cleanup.
  Necessary owner extraction must support the named regression and has its own rollback boundary.
- A cycle budget is a decision checkpoint, not a quality waiver. Report **implemented**, **focused
  verified**, **final-source verified**, and **client accepted** separately. Document limitations
  and the exact next bounded action instead of opening another indefinite repair loop.
