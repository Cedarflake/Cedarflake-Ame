# R2c closeout execution

Status: **active bounded repair supplement; both verification blockers remain unresolved**.

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
review/recheck are exhausted. The supplement below resumes these blockers; historical failures,
invocation counts and active time remain charged.

### Admitted parallel repair supplement

The current direction admits concurrent diagnosis and causal repair of both named blockers on
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
