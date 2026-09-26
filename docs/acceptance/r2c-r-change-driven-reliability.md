# R2c-R change-driven reliability controlled local checkpoint

Status: non-external controlled local reliability checkpoint; not accepted

Date: 2026-09-01

Last amended: 2026-09-02

Platform: Windows 11 x64, x64 process, ordinary non-administrator user

## Scope

This record covers the repeatable R2c-R evidence that can run without an externally signed bundle,
administrator elevation, Service Control Manager changes, a real broker `FSCTL_READ_USN_JOURNAL`
request, `.agents` data, or authorization for either retained library. The gate creates and removes
its own disposable roots and derived storage. It does not accept a caller-supplied source root,
catalog, path alias, or worker environment.

The controlled path is the production synchronization chain:

`poll_runtime_with_storage` -> watcher-first production coordinator -> retained journal session
contract -> P0/P1/P2 queue -> guarded final-state filesystem revalidation -> atomic SQLite catalog
delta -> visible catalog query.

The journal session and counters used by these tests are `#[cfg(test)]` and crate-private. They call
the existing production constructor and do not bypass root registration, capability or checkpoint
validation, queue leasing, final-state revalidation, publication guards, or SQLite transactions.
They add no public bridge or release behavior.

## Repository gate

Run the controlled local gate serially:

```powershell
./tool/acceptance_run_r2c_change_driven_reliability.ps1
```

The runner requires a Windows 11 x64 client workstation in an ordinary non-administrator process.
It cross-checks the 64-bit HKLM build and installation type, `ProductOptions` workstation type,
native `RtlGetVersion` build, and `GetProductInfo` SKU; Windows 10, Server, unresolved SKU, build
mismatch, x86/Arm64, and elevation fail before Cargo. It resolves an OS-known Local AppData path
rather than trusting `PUBLIC`, `TEMP`, or `TMP`. Loading the common module performs no dynamic
compilation. Explicit initialization first emits a minimal Win32/NT declaration surface in memory,
then binds every existing component of both the logical repository `tool` path and its handle-
resolved physical path. Each component is opened relative to a held parent with no-follow semantics;
reparse points, volume transitions, and logical/physical terminal identity drift fail before a
write. Only the final held physical parent can create the high-entropy bootstrap child with
`NtCreateFile`, after which compiler temporary storage is rebound. Compilation revalidates the held
identities, and cleanup deletes only an empty child reopened relative to that parent with the
expected identity; failure or unknown residue is retained without traversal.

The runner applies the same read-only two-stage binding to the logical `SHGetKnownFolderPath`
LocalApplicationData result and its handle-resolved physical filter/container path before it creates
the runtime root. It retains every non-reparse same-volume component and requires the logical and
physical terminal volume/file identities to agree. Only then does it create one high-entropy root
relative to the final held physical anchor, verify the returned root as its direct non-reparse child,
and keep the root and complete anchor chains replacement-blocked until cleanup begins. Parent-
relative opens use the live per-directory Windows case-sensitivity flag; case-folded path text is
never treated as object identity. The runner never creates or trusts a `Temp` junction or predictable
owner directory. All fixture source, catalog, report, log, and temporary paths remain below that
root.

The runner holds the repository tool lock and forces one Cargo build job and one Rust test thread.
Every case uses a fully qualified test name with `--exact`; the four manual reliability cases also
require `--ignored`. A lightweight validation run proves the complete 19-case matrix, 15 normal / 4
ignored split, unique labels and fully qualified names, current attributed functions, and selector
mapping. Each Cargo child is created suspended, assigned to a private kill-on-close Job Object, then
resumed with a ten-minute per-case and one-hour overall parent wall-clock deadline. Timeout kills
only that owned process tree. The final outer Rust summary must contain exactly one passed test,
zero failed, and zero measured; a filtered-only or zero-test invocation fails. `finally` restores
the process environment, location, lock, reports, logs, and exact nonce fixture.

The lightweight guardrail is:

```powershell
./tool/acceptance_test_r2c_change_driven_reliability_guardrails.ps1
```

`quality_lint.ps1`, and therefore Daily, runs only this guardrail. Daily does not run the four
manual reliability cases.

## Exactly counted scenario set

The fresh public gate passed all 19 cases, each as one exact test:

| # | Scenario | Fully qualified test |
|---:|---|---|
| 1 | Production coordinator injection smoke | `application::library_synchronization::change_driven_reliability_acceptance::r2c_r_controlled_journal_evidence_uses_the_production_coordinator` |
| 2 | Live create/modify/rename/move/replacement/delete and ordinary event storm | `application::library_synchronization::change_driven_reliability_acceptance::r2c_r_live_operations_and_event_storm_remain_p0_bounded` |
| 3 | Closed-process six-operation durable P1 recovery and application restart | `application::library_synchronization::change_driven_reliability_acceptance::r2c_r_closed_process_changes_converge_through_durable_p1` |
| 4 | Watcher overflow with measured P0 while P2 remains active | `application::library_synchronization::change_driven_reliability_acceptance::r2c_r_watcher_overflow_recovers_while_twenty_five_real_p0_changes_remain_fast` |
| 5 | One million unrelated records through production parser pages | `journal_broker::windows::usn::tests::million_unrelated_records_stream_through_bounded_production_parser_pages` |
| 6 | One hundred no-change production observer startups | `application::library_synchronization::production::tests::one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning` |
| 7 | Reserved P0 while target-scale P1 and P2 are active | `application::library_synchronization::production::tests::priority::p0_event_to_visible_p95_stays_below_one_second_with_p1_and_p2_active` |
| 8 | Typed journal reset and trim classification | `application::persistent_journal_continuity::session_reader::tests::session_reader_classifies_reset_and_trim_from_structured_query_fields` |
| 9 | Reset/trim race fails closed in the broker service contract | `journal_broker::service::tests::reset_and_trimming_during_read_fail_closed` |
| 10 | Broker session reconnect | `application::library_synchronization::production::tests::production_journal_factory_reconnects_after_closing_the_previous_session` |
| 11 | Reconnect after isolated close failure | `application::library_synchronization::production::tests::reconnect_isolates_a_close_failure_and_installs_a_fresh_session` |
| 12 | Bounded P0/P1/P2 stop and immediate restart | `application::library_synchronization::production::tests::one_stop_joins_delayed_p0_p1_p2_then_allows_immediate_restart` |
| 13 | Cancellation and restart during blocked filesystem/writer work | `application::library_synchronization::production::tests::stop_and_restart_remain_responsive_during_blocked_filesystem_and_writer_work` |
| 14 | Configured-root replacement opens no enumeration path | `application::incremental_library_changes::tests::replacement_root_uses_only_the_proof_guard_and_enumerates_nothing` |
| 15 | Retained same-volume multi-root request sharing and failure isolation | `application::library_synchronization::change_driven_reliability_acceptance::r2c_r_same_volume_roots_share_one_retained_session_read_and_isolate_failure` |
| 16 | Placeholder evidence opens no media inspector | `application::metadata_inventory::tests::placeholder_evidence_is_staged_and_enqueued_without_media_inspection` |
| 17 | Unchanged Cloud Files placeholder remains unresolved without retry | `application::metadata_inventory::tests::unchanged_reparse_cloud_placeholder_preserves_location_without_retry` |
| 18 | Chinese and Windows long relative paths remain lossless | `application::directory_synchronization::tests::chinese_and_long_relative_paths_remain_lossless_and_normalized` |
| 19 | Missing or ambiguous closed-worker report bindings are rejected | `application::library_synchronization::change_driven_reliability_acceptance::r2c_r_worker_report_fields_reject_missing_or_ambiguous_bindings` |

The ignored child worker is not a twentieth gate case. The closed-process parent launches it with
an exact phase and validates its result before accepting each sample.

## Fresh controlled measurements

### Live changes and ordinary storm

- The fixture contains 4,096 unrelated baseline entries before timing any live change. Six live
  operations measured P50 390 ms, P95 435 ms, maximum 435 ms, and zero over one second.
- The ordinary storm performed three writes for each of 96 paths and converged to one coalesced
  queue row carrying 674 observations.
- Journal reads were zero. Media opens were exactly 99: the three live final states that require
  inspection plus the 96 storm final states.
- Bounded affected-path reads were 115, below the explicit 192 limit and independent of the 4,096
  unrelated entries. Root inventory reads, metadata-inventory runs, P2 rows, and added full-scan
  rows were zero. Both the affected-path and unrelated-baseline post-publication snapshots matched.
- Shutdown closed the controlled retained session exactly once.

### Closed-process durable journal recovery

- Create, modify, rename, same-volume move, same-path replacement, and delete each crossed one
  durable source range and one P1 queue lineage after the second runtime became ready.
- Six samples: P50 359 ms, P95 1,058 ms, maximum 1,058 ms, zero over two seconds.
- Source entry reads and inventory entry reads were zero. Inventory runs and full-scan row changes
  were zero.
- Media content opened only for the three final states that required inspection: create, modify,
  and replacement. Production polls used only metadata availability probes; discovery root handles
  remained zero and guarded publication was exercised.
- Each lease completed through production P1 with no retry or supersession and one atomic mutation.
- The parent launched one `crash-ready` and six `recover` workers. Every worker appended exactly one
  report line, and the parent matched its lowercase run nonce, runner PID, parent PID, actual child
  PID, and phase. Missing or duplicate binding fields are rejected as ambiguous.

### Watcher overflow and active P0

- The overflow fixture performed 768 writes across 256 paths and produced exactly one typed
  `WatcherUncoveredGap` authority and one inventory run.
- The inventory staged at most 257 entries, reached page index 2, and produced exactly 256
  candidates. P2 contained one authority control row plus 256 candidate rows owned by the same run:
  257 total, below the steady 3,072-row partition.
- All 25 measured P0 samples started and became visible while the same authority remained
  unretired and its inventory run remained `running` or `comparing`.
- P0 P50 was 331 ms, P95 346 ms, maximum 371 ms, with zero over one second. Overall overflow
  recovery converged in 23.570 seconds; this recovery duration is not presented as the P0 SLA.
- P0 affected-subtree reads before P2 were 256. P2 then read 256 entries, for 512 total. Journal
  reads were zero and full-scan rows were unchanged.

### Large unrelated journal backlog

- The production native record parser and root-scoping backend consumed 1,000,000 semantic V2
  records in 245 strictly advancing pages. Every full page exercised the production 256 KiB native
  buffer capacity and 4,095 fixed-shape records, rather than a smaller 2,048-record surrogate; the
  complete stream finished in 12.798 seconds.
- The maximum native buffer length/capacity was 262,088/262,144 bytes. Retained record, reference-
  history, and resolved-record capacities were 364,543, 442,336, and 393,120 bytes. The fixture
  reused one identity across 2,048 history entries and resolved paths through the real per-USN
  reference-history path.
- Maximum simultaneous requested capacity was 1,462,263 bytes. The fixture-specific conservative
  allocator estimate was 3,711,278 bytes, below its 3,809,886-byte bound. That estimate explicitly
  counts all simultaneously live container capacities and applies a conservative two-times
  requested-capacity multiplier plus 64 bytes per allocation. It is fixed-record fixture evidence,
  not a claim about every allocator or arbitrary production filename worst case.
- All 1,000,000 records crossed the root-scope predicate and emitted zero candidates; retained
  paging and candidate state therefore remain independent of total backlog length.

### No-change and reserved priority

- One hundred complete production starts opened exactly 100 watcher root handles, registered 100
  nonzero unique client instances, issued and harvested exactly 100 journal queries, reached
  `Current`, and closed exactly 100 sessions. Opening the watcher root handle is an expected system
  operation and is not reported as enumeration. The 200 actual production polls performed exactly
  200 metadata-only `symlink_metadata` availability probes. The filesystem adapter now produces an
  opaque metadata result for a path-free classifier. A `syn` 2.0.119 AST visitor locks the owning
  production entry, probe, and classifier against `read_dir`/walk insertion while correctly ignoring
  strings, raw strings, and comments. Its representative post-brace violation fails. Journal reads,
  enumeration-spool opens, source entry
  reads, source-root entries, inventory runs, media-content opens, discovery-root handles,
  publication-guard root handles, and added full-scan rows were all zero in every cycle and in total.
- Under 2,048 P1 candidates and 10,000 P2 entries, all 25 P0 samples saw both lower lanes active.
  P0 P50 was 79 ms, P95 946 ms, and maximum 1,027 ms; the contractual P95 remained below one
  second. Queue and worker P95 were 39 and 38 ms. P1 completed all 2,048 candidates and P2 reached
  all 10,000 source reads without taking the reserved P0 path.

## Boundedness, persistence, and source safety

- Queue and paging bounds are asserted at their production partitions, not inferred from a short
  fixture. The million-record case iterates every record and never replaces the backlog with a
  constant skip.
- Journal reset, trim, reconnect, close failure, cancellation, and shutdown preserve structured
  terminal outcomes. The controlled session count proves register/query/read/close behavior, while
  real named-pipe transport, SCM, and FSCTL remain outside this checkpoint.
- Closed-process work uses durable source ranges and P1 lineages; visible publication follows
  final-state revalidation and the existing atomic SQLite delta. No test directly writes the final
  catalog as a shortcut.
- Catalog, settings, previews, and reports remain in fixture-owned derived storage outside the
  source root. Full-scan row counts remain unchanged across the change-driven cases.
- The six operation fixtures intentionally change source entries and bytes. Source safety compares
  the snapshot after each intended fixture change with the snapshot after synchronization; they
  match, proving Ame made no additional change. The record does not incorrectly claim the source
  was identical before and after the fixture operation.
- Placeholder contract cases open no media inspector and retain unresolved placeholder state. They
  are controlled evidence, not a real retained OneDrive no-hydration acceptance run.
- The gate does not configure, enable, disable, resize, delete, or otherwise modify a journal.
- The current same-volume case enters the retained session-reader and production coordinator with
  two registered roots in one volume request. It performs one shared read, advances the healthy
  checkpoint, marks only the rejected root `RecoveryRequired`, closes once, opens two production
  observer root handles, adds no scan row, and leaves both source snapshots unchanged.

## Red-to-green history

The following failures are retained as evidence rather than omitted:

1. The initial acceptance compile failed with missing source-content counters and the missing
   production-session harness (`E0425` and `E0599`). The narrow repair added only `#[cfg(test)]`,
   crate-private instrumentation and a harness that calls the existing production constructor.
2. A workspace-sandbox run reached the production namespace guard and failed with Win32 5. The
   unchanged test passed in the ordinary Windows user context using its then-current unique Public
   Documents disposable root; the production guard was not weakened and Win32 5 was not swallowed.
3. The first closed-process child reported watcher health before the asynchronous journal query
   counter became visible. The fixture now waits for both facts before simulating the crash.
4. A 256-path three-write storm was initially treated as an ordinary event storm. It produced a
   real watcher overflow and took 28.013 seconds, so the evidence was correctly split into a
   96-path no-overflow storm and a separate typed-overflow scenario.
5. The first overflow assertion expected zero source reads when P2 blocked, but the prior bounded
   P0 subtree reconciliation had already read 256 entries. The test now captures that production
   baseline and measures the P2 delta separately.
6. The next overflow run reached correct behavior but used an incorrect `P2 queue <= 8` assertion.
   The final test proves the exact shape: one control, 256 same-authority owned candidates, and 257
   total, then separately asserts the 3,072-row steady cap.
7. The first PowerShell guardrail enumeration hit a Windows environment-provider duplicate-key
   error. The common file now owns the explicit known alias list, and the guardrail tests every
   alias independently.
8. The first public-runner attempt treated Cargo's normal PowerShell 5 stderr progress as a
   terminating `NativeCommandError`; the second rejected blank lines during summary binding. The
   capture layer now permits those transport details locally while still requiring exit code zero
   and the final exact one-test summary. The representative zero-test summary is rejected by the
   guardrail.
9. The first warnings-denied Clippy run found `bool::then` and a redundant iterator clone only in
   the new acceptance code. The narrow fixes use `then_some` and borrowed records; development- and
   release-profile Clippy then passed without suppressions.
10. The sandboxed `quality_lint.ps1` run reached Dart and failed only because the pinned SDK could
    not write its user-level telemetry session file. The unchanged scoped command passed in the
    ordinary Windows user context, without deleting a lock or terminating an unrelated process.
11. The independent R2c-R audit found that the original runner blacklisted environment names but
    still trusted caller-influenced temporary/public locations. A first trusted-root repair chose
    Common Documents and failed with an ordinary-user ACL denial. The final runner resolves the OS
    Local AppData known folder, physically validates its fixed-NTFS `Temp` descendant and every
    ancestor, creates a nonce-owned subtree, and passes with hostile `PUBLIC`, `TEMP`, and `TMP`.
12. The first 100-start test only initiated asynchronous journal queries and used a healthy test
    observer. The replacement initially found a canonical-path mismatch in the new watcher-handle
    counter. After canonical keying, 100 complete production observer/coordinator cycles harvest
    every query and prove 100 real root handles but zero enumeration or scan work.
13. The first remediated live-storm run reported correct bounded I/O but P95 5.156 seconds because a
    recursive snapshot of all 4,096 unrelated files was inside every measured interval. Moving the
    unchanged-baseline proof outside the timers and retaining targeted affected-path snapshots made
    P95 732 ms without changing the production threshold or path.
14. The first current same-volume test reached the intended healthy/failure split but asserted the
    pre-harvest poll snapshot and looked for a capability failure payload that belongs on the
    checkpoint. The final test polls persisted checkpoint/capability state and proves one shared
    production read with per-root isolation.
15. The million-record evidence had used `len`, a 2,048-record page, and manually constructed
    resolved records. It now exercises the fixed 256 KiB production buffer, capacity accounting,
    real per-USN reference histories, and one million root-scope decisions. Its documented memory
    statement is deliberately limited to the fixed-record allocator model it actually proves.
16. The first process-tree guardrail used a 500 ms deadline and sometimes terminated its fixture
    before the child PID could be reported. A three-second guardrail-only deadline fixed that on the
    original workstation but still included fresh child module loading and native-helper compilation;
    the hosted Windows Server 2025 child did not report readiness before that deadline. The current
    15-second fixture-only deadline remains below the child's 30-second block, then requires Job
    Object disposal to remove both within five seconds while an unrelated side process remains alive.
    Production per-case, total, and default process deadlines were not relaxed.
17. The second independent review found that dot-sourcing the common module invoked `Add-Type`
    before the runner rebound hostile temporary variables. The replacement defers compilation to an
    explicit initializer. A fresh PowerShell child now proves the types are absent before and after
    dot-source, hostile `TEMP` and `TMP` sentinels receive zero new files or directories, and the
    repository bootstrap leaves zero residue on success and on compiler failure.
18. The same review found that the earlier `LocalAppData\Temp` model created a predictable owner
    before physical verification. The first replacement string create correctly failed when the
    Codex execution filter redirected it into package-local storage; an initial native-relative
    attempt also failed closed with Win32 4395 while `OBJ_DONT_REPARSE` rejected the filter. The final
    path uses exclusive `NtCreateFile` relative to the held KnownFolder base, verifies and holds each
    returned physical redirection component with no delete sharing, and deletes the exact empty root
    by an identity-checked parent-relative handle. The first complete rerun then failed case 2 with
    Win32 32 because requesting `DELETE` on the runtime blocker correctly conflicted with the
    production no-delete-share publication guard. The blocker now requests no delete access. After
    all owned children exit, cleanup releases that blocker, reopens the unpredictable name relative
    to the still-held parent, verifies the same file identity and physical path, and deletes it. The
    cleanup transition is not claimed atomic: a same-user racer can force fail-closed residue but
    cannot redirect deletion to a replacement. Fully internal `Temp` and owner junction attacks are
    rejected with zero sentinel writes and zero failed residue; no junction experiment touches a
    real external path.
19. The no-change report had mislabeled the normal per-poll metadata lookup as zero availability
    opens. Production instrumentation now counts the real operation: 200 production polls produce
    exactly 200 metadata-only probes, while source enumeration, inventory, full scans, discovery
    handles, publication guards, and media opens remain zero.
20. The report-binding tamper test existed but was absent from the public matrix. It is now the unique
    nineteenth case; matrix source-attribute and `--exact` checks enforce 19 total, 15 normal, and four
    ignored cases.
21. The final sandboxed format and lint invocation reached Dart after completing Rust formatting,
    then failed with Win32 5 while the SDK tried to write its user-profile telemetry session. A broad
    user-profile retry was rejected. The unchanged gates instead ran with analytics suppressed in a
    fresh nonce workspace-only `APPDATA` profile, which was removed afterward; formatting, lint, and
    Daily then passed without granting telemetry or unrelated profile writes.
22. The third review demonstrated that managed prechecks and `Directory.CreateDirectory` did not
    hold the compiler bootstrap identity across `Add-Type`. The replacement uses in-memory
    `Reflection.Emit` only for the minimal native declarations, holds and verifies the repository
    tool parent before any compiler write, and creates the child relative to that handle. A real
    active rename fails with a sharing violation, a pre-created junction fails exclusive create,
    and a forced compiler error leaves zero safely deletable residue; all sentinels remain unchanged.
23. The same review found a recursive managed cleanup fallback after native cleanup failure. That
    fallback and helper are removed. Real ordinary and junction replacement races move the original
    root after blocker release, place a replacement at the old name, and drive the complete cleanup
    function to its fail-closed retention error. Neither replacement nor junction sentinel is
    deleted or traversed; the error reports the owned leaf and expected file-identity token rather
    than treating the old-path replacement as the owned object. The guardrail later deletes only the
    moved original by its retained identity and each known replacement through a no-follow
    parent-relative handle.
24. The earlier availability fake accepted arbitrary metadata for an unrelated nonexistent path,
    so it could not prevent production from adding enumeration. The new test was first red because
    the opaque evidence type and classifier did not exist. The production refactor separates the
    metadata-only filesystem probe from the path-free classifier, and its source guard rejects a
    representative `std::fs::read_dir` body before the exact test turns green. The 100-start rerun
    still reports exactly 200 polls and probes with every scan, enumeration, and media counter zero.
25. The native relative-leaf check previously rejected only separators. The owned-name contract now
    requires the exact generated prefix and two 32-character lowercase-hex fields; only the internal
    race fixture may add an exact `-moved-<32 lowercase hex>` suffix. Direct guardrails reject
    colon/ADS, NUL and controls, dot components, slash/backslash, trailing dot/space, arbitrary ASCII
    suffixes, uppercase hex, and Unicode, while generated nonce/GUID roots and moved fixtures pass.
26. Matrix presence alone did not execute the report-tamper behavior through lint. The lightweight
    guardrail now runs that one Rust function with `--exact`, a five-minute owned Job deadline, and
    the same strict result parser used by the public runner. Exactly one pass and zero failed,
    ignored, or measured tests are required; filtered-only output remains rejected.
27. The fourth review found that the writable-anchor proofs were terminal-only. The production
    KnownFolder seam could therefore traverse an intermediate junction before the held terminal was
    examined, while the logical/physical filter binding did not retain the complete parent-child
    chain. The replacement binds every logical and handle-resolved physical component read-only,
    parent-relative, no-follow, same-volume, and identity-held before the first create. A fully
    internal intermediate-junction fixture now fails before root creation with zero sentinel change
    and zero root residue. Legitimate filter/container redirection remains supported because the
    physical chain is bound from the terminal handle rather than rejected by path spelling.
28. The same review found string-addressed guardrail teardown and fixture cleanup that could act on
    a replacement after validation. Files now use held delete-on-close streams where possible, and
    each directory teardown requires a captured expected identity on a held-parent no-follow reopen.
    Ordinary and junction fixtures are each swapped a second time after validation. The stale token
    fails and retains the unknown object; only a freshly captured fixture identity permits cleanup.
    A PowerShell AST audit rejects `Remove-Item` aliases and `.Delete` calls in both teardown owners.
29. A fourth-review source fixture put `"}"`, a raw string, and a comment brace before a real
    `std::fs::read_dir`. The handwritten brace scanner incorrectly accepted it, so the exact
    availability source-guard test was red. The replacement parses the complete Rust file with
    `syn` 2.0.119 and visits only the owning function ASTs. The representative violation is now
    rejected, while enumeration names and braces contained only in ordinary/raw strings or comments
    remain legal.
30. The first complete runner after moving process logs to held delete-on-close streams passed case
    one, then its obsolete `Remove-Item` attempted to delete a path already removed on close. The
    path delete was removed, and the guardrail AST audit now prevents its return.
31. The next complete runner passed all 19 exact Rust cases but correctly refused final cleanup
    because three test temporary directories still contained open `catalog.sqlite3` files. The
    root-replacement catalog fixture and both placeholder inventory fixtures declared their
    `TempDir` field before the SQLite owner, so Rust dropped storage while Windows still held the
    database. Reordering the fields makes the catalog drop first. All three focused fixtures then
    passed inside a fresh identity-held root, and the next ordinary-user 19-case runner passed and
    removed its root. The failed run's unknown root remains deliberately retained because its child
    identity tokens were not captured; no path cleanup was used as a fallback.
32. Hosted PowerShell 7 exposed that the guardrail parsed human-formatted `throw` text as a machine
    protocol. The negative runner process exited nonzero without timing out, but its rendered error
    did not retain one contiguous expected phrase. The runner now writes and flushes one stable ASCII
    reason token before each expected caller-path, environment, workspace-mode, or execution-context
    refusal, while retaining the original exception. The guardrail requires exactly one matching
    token and reports exit, timeout, and captured lines on mismatch. The same run exposed a second
    contract error: Windows Server 2025 cannot truthfully complete a runner that requires a Windows
    11 client workstation and ordinary-user token. A supported workstation must still complete the
    exact 19/15/4 ValidationOnly path; an unsupported host must instead produce the execution-context
    token, exit nonzero, and produce no success report. No CI variable, simulated client evidence,
    platform bypass, Cargo execution, external root, or source-library access was introduced.
33. The next hosted run proved the stable token worked and exposed a PowerShell 7.5/.NET 9
    environment-semantic difference. Passing ordinary `$null` to the .NET string overload left each
    protected name present with an empty value, whereas the Windows PowerShell 5.1/.NET Framework
    workstation had removed it. The first child therefore correctly rejected all inherited empty
    R2c-R aliases before reaching the workspace-mode check. The guardrail now uses PowerShell's
    `NullString.Value` specifically to pass a real null to .NET, verifies every protected runner name
    is absent before process creation, clears each hostile alias the same way, and restores an
    originally absent value as a real null. The same null-aware restoration covers compiler
    `TEMP`/`TMP`, the exact tamper fixture's `CARGO_BUILD_JOBS`/`RUST_TEST_THREADS`, and every runner
    environment snapshot. The tamper fixture and successful runner both assert post-restore
    nullness and value equality before returning. Empty and absent values remain distinct; the
    runner's default-deny environment policy was not weakened.
34. The following hosted run passed all 19 R2c-R guardrails with the truthful unsupported-host
    result, then exposed a separate named-pipe ACL API split in the broker guardrail. Windows
    PowerShell 5.1/.NET Framework provides the secure nine-argument `NamedPipeServerStream`
    constructor, while PowerShell 7/.NET 9 provides the equivalent ten-argument
    `NamedPipeServerStreamAcl.Create` factory. The shared acceptance module now selects only those
    two security-bearing APIs by runtime capability and fails closed if its selected API fails;
    there is no unsecured constructor or post-create ACL fallback. DACL inspection similarly uses
    the Framework instance method or `PipesAclExtensions.GetAccessControl`. The guardrail now proves
    inheritance is disabled and matches exactly three explicit allow rules: SYSTEM and
    Administrators receive `FullControl`, while the current user receives
    `ReadWrite|Synchronize`. The hosted Windows PowerShell compatibility step executes this broker
    guardrail as well, while the normal PowerShell 7 Static lane retains its existing execution, so
    every pull request exercises both runtime branches.

Two later read-only PowerShell 5 evidence commands also failed before producing a result: the first
used a newer two-argument `String.Contains` overload, and the second treated a null pipeline result
as if it always had `Count`. The corrected command used ordinal `IndexOf` and an explicit array
wrapper, then proved the Release facts below. These were validation-command compatibility errors,
not product or acceptance-run failures, and changed no repository file.

## Guardrail evidence

The fresh guardrail exits zero with `AME_R2C_R_GUARDRAILS status=passed`. It proves that cargo is not
entered for:

- caller-supplied `SourceRoot`, `LocalRoot`, `CloudRoot`, or `SourceCatalogPath`;
- each of the 19 known R2c-R path, catalog, nonce, PID, report, phase, and worker environment
  aliases;
- non-Windows, Windows 10, Server installation/product type, unresolved client SKU, registry/native
  build mismatch, non-x64 Windows, non-x64 process, or administrator execution contexts;
- a zero-test or filtered-only Rust result.

The synthetic platform probes always prove the exact Windows 11 client contract. On a supported
Windows 11 x64 ordinary-user host, the child additionally proves the exact 19/15/4 ValidationOnly
success report and attributed functions. On another Windows host, the same child must fail before
fixture creation with exactly one stable execution-context reason token and no success report. The
guardrail summary therefore reports `platform_contract=windows11-x64` and an honest
`host_validation=passed|rejected`; rejection is not presented as Windows 11 acceptance. A fresh child
proves common-module load performs no compilation, hostile temporary sentinels receive zero writes,
active bootstrap replacement is blocked, a pre-created junction is rejected, forced compiler failure
leaves zero safely deletable residue, and sentinels remain unchanged. The production KnownFolder
seam rejects a fully internal intermediate-ancestor junction before root creation while preserving
the sentinel and leaving no root residue. Workspace-only held fixtures exercise ordinary and
junction replacement cleanup races, then swap each replacement again between validation and
teardown. A stale identity cannot delete the unknown object; cleanup requires its newly captured
identity and a held-parent no-follow reopen. Sentinel and log files remain held delete-on-close, and
the reusable deletion-safety AST audit parses the common module, runner, guardrail, and all four
actual child-command payloads. It rejects module-qualified or aliased deletion, dynamic cleanup
commands, reflection or script-block construction, unknown cleanup calls, and recursive child
`File.Delete`, `Directory.Delete`, or `Remove-Item` content. Post-open native fault injection proves
the anchor-chain and failed-root handles are already closed before control returns; the failed-root
fixture can then be renamed and deleted by its retained identity in the same process. The exact
ASCII/hex owned-leaf contract rejects NT path, ADS, arbitrary suffix, and uppercase/non-ASCII syntax.
The exact report-binding
tamper Rust test actually runs and produces one pass; a blocked owned child tree is terminated while
an unrelated process survives, and its timeout fixture is removed. It reports internal disposable
roots, refusal of external source paths, and the strict native Windows 11 workstation contract
without accessing a real library or external sentinel root.

## Final local verification

### Ninth-remediation SQLite protocol-read reliability

- The eighth-remediation history remains part of the evidence: its first complete ordinary-user
  runner failed closed in watcher overflow on SQLite `FileLockingProtocolFailed` (result code 15),
  cleaned its exact owned root and process tree, and was not counted green. The follow-up review
  reported zero Critical, zero High, one Medium, and zero Low findings. Five new independent exact
  watcher-overflow roots reproduced the exposure as four passes and one code-15 failure at the raw
  `Connection::open` / `query_row` observer. That result is not overwritten by later passes.
- The bundled dependency path is `rusqlite` 0.40.1 with `libsqlite3-sys` 0.38.1 and SQLite 3.53.2,
  in WAL mode. SQLite documents result code 15 as a WAL transaction-start locking-protocol race for
  which a later new transaction may succeed. The old public reader opened and validated a catalog
  for every request and then opened a second connection for its query; open, fast schema validation,
  and query therefore all exposed code 15 as the generic `catalog_database_error`. The acceptance
  observer compounded this by reopening an unconfigured raw connection every 10 ms and panicking on
  its query result. This was not introduced by the eighth remediation, but it was reachable from
  production gallery reads under concurrent WAL writes.
- `SqliteCatalogReadExecutor` is now the adapter-owned retry boundary for idempotent catalog reads
  only. It recognizes only rusqlite `FileLockingProtocolFailed`; `Busy` and `Locked` retain the
  existing five-second SQLite busy-handler behavior, and every other error returns immediately.
  A protocol retry drops the complete prior `SqliteCatalog`, opens a fresh connection, reruns pure
  read-only schema and catalog-identity validation, and then reruns the query. Existing-catalog
  validation uses `SQLITE_OPEN_READ_ONLY` and never creates a directory, creates a database, runs a
  migration, or changes WAL state. Application-owned preparation remains the only migration path.
  Writes, migrations, transactions, preview-touch publication, and root removal are not retried.
- Production policy permits at most five total attempts within a 100 ms monotonic protocol-retry
  admission window, with 1/2/4/8 ms backoff capped at 8 ms. The window is checked after each completed
  SQLite call and before another retry; it is not an interruptible hard wall-clock cap on an active
  SQLite call. In particular, a `Busy`/`Locked` call can first consume its existing five-second busy
  timeout and then returns without protocol retry. Exhaustion returns
  `catalog_read_protocol_retry_exhausted` with operation, attempts, actual elapsed milliseconds, and
  the final structured cause, without a catalog path. A successful uncontended read sleeps zero
  times.
- Gallery snapshot/window, timeline, layout manifest, folders, around-asset, and asset-by-ID public
  reads all route through one application helper and this owner. Preview reconciliation and usage
  touches remain outside the retry closure and execute at most once. The watcher-overflow fixture now
  owns one persistent production-equivalent observer for gap count, inventory count, recovery
  authority, and location observations; it propagates the final structured error into the case
  report instead of using raw SQL, swallowing an error, or panicking.
- Deterministic red controls first failed immediately at injected open and existing-schema validation
  code 15. The green set covers open, validation, and query recovery with exact fresh-connection
  counts; non-protocol, Busy, and Locked no-retry behavior; attempt and monotonic-window exhaustion;
  diagnostic cause preservation; no transaction or write-admission hold across backoff; missing
  catalog no-create/no-migrate; current schema no-migrate; attempt statistics; and the exact
  production policy. It passes 13/13. A real concurrent WAL writer plus the public production
  `load_gallery_timeline` path completes 256 subprocess reads. The first full Daily exposed four
  attempt-count fixtures whose copied 100 ms test budget expired after individual fresh opens took
  101-212 ms under full parallel load; that failure remains recorded. Only those deterministic
  attempt-limit fixtures now use a 30-second test budget, while the dedicated 1 ms deadline control
  and every production path retain the production policy. The next full Daily is green.
- A partial observer change that covered only the original gap-count helper produced four green
  exact runs followed by one nonzero fifth run; the ad-hoc wrapper cleaned the fixture before
  preserving the child log, so no narrower cause is claimed. Audit then found and replaced the
  remaining raw inventory, authority, and location reads. After that complete observer boundary,
  twenty consecutive fresh-nonce, identity-held exact watcher-overflow runs pass. Per-run
  `P95/convergence/read-attempts` values are:
  `307/11701/801`, `308/11610/798`, `308/11854/805`, `308/11772/800`,
  `311/11690/797`, `310/11762/800`, `307/11463/787`, `308/11692/800`,
  `325/11784/799`, `311/11600/794`, `308/11579/786`, `308/11776/802`,
  `309/11872/808`, `325/11850/806`, `309/11825/803`, `327/11800/802`,
  `308/11649/801`, `326/11805/800`, `307/11695/801`, and `308/11846/798`
  (milliseconds/milliseconds/count). Attempts equal operations in every run, protocol retries are
  zero, and maximum attempts are one; P95 spans 307-327 ms, convergence 11.463-11.872 seconds, and
  the maximum individual P0 sample is 420 ms.
- The final ordinary-user runner passes all 19 exact cases with nonce-bound cleanup. Watcher
  overflow reports P95 308 ms, 11.631-second convergence, 795 reads/795 attempts, zero protocol
  retries, and maximum attempt one. It also passes no-change, tamper, root replacement, both Cloud
  Files fixtures, one-million backlog, and priority evidence. The lightweight guardrail passes 19
  cases, `ValidationOnly` reports 19/15/4, and the topology, macro, 17-function/16-support, real
  case-sensitive source, and three PowerShell parse controls pass.
- Formatting, development and Release all-target/all-feature checks, warnings-denied Clippy, and
  `quality_lint.ps1` pass. The complete ordinary-user Daily exits zero: Rust reports 878 passed,
  zero failed, and 17 expected ignored; broker binary integration passes 3/3; all Flutter tests,
  controlled Windows scan 2/2, native accessibility 2/2, bridge hashes, and whitespace pass. The
  current unsigned Windows x64 application builds in 37.6 seconds and the independent broker in
  20.65 seconds. Application, packaged DLL, Cargokit DLL, and broker are PE `0x8664`; all 82 Cargokit
  dependencies exist, `read_retry.rs` is the newest dependency, and packaged/built DLL SHA-256
  values match. ASCII and UTF-16 scans find zero new fault-injection, test-policy, watcher-metric,
  R2c-R, or `test_support` seam in all four binaries. They are explicitly unsigned. The unsigned
  bundle has no externally signed broker, so portable packaging fails closed as incomplete; formal
  Windows Release admission with prevalidated absent signed Application and broker paths fails
  closed before starting a process, SCM, pipe, or FSCTL work.

This remediation remains non-external implementation evidence. It does not accept R2c-R or the
accumulated R2c milestone. R2c-O remains active, and exact publisher/signed-bundle admission,
elevated SCM/service lifecycle, real named-pipe/FSCTL journal evidence, separately authorized
retained-root immutability, real Cloud Files no-hydration, and the final independent accumulated
audit remain open.

### Tenth-remediation typed read and catalog-identity boundary

- The next independent boundary review reported zero Critical, zero High, two Medium, and one Low
  finding. Red controls first proved that a crate-visible generic callback could repeat catalog
  mutation or unrelated side effects, a caller-forged `ScanError.code` could trigger retry, an
  arbitrary absolute path could enter exhaustion text, a post-identity deletion could make the
  default SQLite open create an empty catalog, and the loop constructed its clock directly.
- `SqliteCatalogReadExecutor` now exposes only named read operations to crate callers. The generic
  loop and its `FnMut(&mut SqliteCatalog)` are private to `read_retry`; no caller receives a
  connection, transaction, mutable catalog, or callback slot. A `syn` visibility/type contract
  rejects any crate-visible executor method with a callback or catalog owner in its signature.
  Six public catalog-read families and the gap, inventory, authority, location, and completed-P1
  observers use dedicated methods. Preview reconciliation and usage touches remain outside retry
  and execute at most once.
- Retry classification is an internal attempt result derived from the original rusqlite
  `ErrorCode::FileLockingProtocolFailed` while the attempt scope is active. It never examines a
  public `ScanError` code or message; forged protocol text, Busy, Locked, ReadOnly, Corrupt, and I/O
  errors return immediately. Exhaustion uses a fixed path-free message and an optional typed
  `CatalogReadRetryDetails` containing the closed operation enum, attempt count, actual elapsed
  milliseconds, and sanitized `FileLockingProtocolFailed` cause. Ordinary errors retain
  `retry_details = None`. Canonical Flutter Rust Bridge generation carries the optional field and
  both closed enums into Dart; a real SSE encoder test locks the legacy-none marker and all four
  structured fields.
- Every Windows read attempt first opens a no-follow, no-recall regular-file guard with
  `FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE`, shares read and write but not delete, and
  obtains volume/file identity from that held handle. The identity must equal the validated session
  before SQLite opens with `READ_ONLY | URI | NO_MUTEX`. The connection owns the guard for its full
  lifetime and drops before the guard; failure and backoff release both. Delete, single replacement,
  ABA replacement, opened-guard mismatch, terminal reparse, success cleanup, terminal failure, and
  backoff-release fixtures all fail closed or clean up as required. No new `unsafe` was added; the
  accepted local-file identity adapter remains the sole native identity implementation.
- The production policy remains five attempts, 100 ms, and 1/2/4/8 ms. Production uses
  `Instant` plus thread sleep behind a private clock; only test builds can inject a manual clock.
  Deterministic production-policy tests cover success at attempts two through five independently at
  open, validation, and query, deadline refusal before a later attempt, one 101 ms call reporting
  one attempt with real elapsed semantics, and uncontended zero-sleep success. No 30-second fixture
  is used to prove production deadline behavior.
- Focused green evidence is 32/32 read-retry tests, 62/62 migration tests, six application catalog
  tests plus the 256-read WAL child, canonical bridge generation and Release Rust build, bridge
  serialization, and test-profile compilation. Three PowerShell parses, the lightweight 19-case
  guardrail, exact 19/15/4 `ValidationOnly`, topology/macro/source closure, 100-start no-change,
  report tamper, root replacement, and both disposable Cloud Files fixtures pass. Sandbox Win32 5
  ancestor-pin results were retained as failures; only the identical ordinary-user reruns count
  green.

This is still non-external implementation evidence. R2c-R is not accepted, R2c-O remains active,
and fresh repeated production watcher, complete runner, Daily, unsigned x64 packaging, and all
external signed/service/real-journal/retained-root/real-Cloud boundaries remain closeout work.

### Eleventh-remediation blocking WAL and observer bounds

- Locked SQLite 3.53.2 source explains the retained 10.941-second read: `walTryBeginRead` permits
  `WAL_RETRY_PROTOCOL_LIMIT = 100`, begins sleeping after the first five retries, and uses a
  quadratic delay through retries 10-100 before returning protocol failure. The pre-remediation
  compile-option control failed because bundled SQLite did not report `ENABLE_SETLK_TIMEOUT`.
  `.cargo/config.toml` now sets the supported `SQLITE_ENABLE_SETLK_TIMEOUT=1` bundled build flag;
  the same control then passes against the locked dependency.
- Every production read connection now sets `busy_timeout = 100 ms` and `query_only`, so the
  Windows VFS blocking-WAL path shares the protocol retry-admission budget instead of spending the
  internal approximately-ten-second polling path. The write connection keeps its five-second busy
  timeout. Focused read/write contention tests prove those separate policies, and the read-retry
  group is green 33/33. No retry count or 100 ms owner window was enlarged.
- The watcher recovery observer combines gap, inventory, authority, and bounded location evidence
  into one named typed snapshot; its 256 location check is one bounded bulk read. The pre-aggregation
  exact control failed with 789 operations/attempts. The corrected 20 independent fresh-nonce runs
  pass 20/20: operations and attempts span 474-527, retries total zero, maximum attempt is one,
  sample-P95 P95 is 374 ms with a 403 ms maximum, and convergence P95 is 12.128 seconds with a
  12.941-second maximum. The two earlier watcher failures and their retained roots remain historical
  failure evidence.
- Faster typed observers exposed an acceptance-harness cadence error rather than a production
  availability cache defect. The first complete runner failed closed at 78 metadata probes; exact
  reproduction reported 75/75/77 with one readiness probe per worker and 12-13 follow-up polls.
  Worker readiness, crash, and recovery waits use the then-current production 250 ms synchronization
  cadence.
  Five exact runs pass with 24 total probes each and closed-process P95 values 564/586/565/580/576
  ms. Every actual poll still performs one fresh O(1) metadata probe; the no-change control remains
  exactly 200 polls/200 probes with zero enumeration, inventory, or media reads.
- The final ordinary-user runner passes all 19 exact cases with nonce-bound cleanup. Closed-process
  recovery reports 24 probes and P95 569 ms; watcher overflow reports 501 operations/attempts, zero
  retry, P95 363 ms, and 12.004-second convergence; no-change reports 200/200; reserved priority P95
  is 318 ms. The failed roots remain retained and were not deleted or traversed for cleanup.
- Formatting covers 149 files with zero changes. Development and Release all-target/all-feature
  checks pass, both warnings-denied Clippy modes pass, and `quality_lint.ps1` exits zero. The complete
  serial Daily exits zero with 898 Rust tests passed, zero failed, and 17 expected ignored; broker
  binary integration passes 3/3; all Flutter tests, Windows scan 2/2, native accessibility 2/2,
  bridge compatibility, and whitespace validation pass.
- A current unsigned Windows x64 Flutter Release build completes in 65.9 seconds and the independent
  broker Release build in 20.69 seconds. The application, both Rust DLL copies, the Cargokit broker,
  and the independent broker are PE `0x8664` and Authenticode `NotSigned`. The DLL, Cargokit broker,
  and independent broker dependency graphs contain 82/83/83 existing current files; packaged and
  Cargokit DLL SHA-256 values both equal
  `988D6B692606E3E2A0497697A5C0D85BCACB4C6DF9BBC6465B564083CF7BF151`. Eighteen current
  fault, manual-clock, R2c-R, and `test_support` seams have zero ASCII or UTF-16 match in all five
  binaries and all three dependency records.
- Formal Windows verification with explicitly nonexistent signed bundle/broker inputs fails closed
  in 0.77 seconds at bundle admission. Portable signature verification with an absent exact publisher
  fails closed in 0.72 seconds before archive validation or extraction. Neither negative admission
  reaches packaged-process, SCM, named-pipe, or FSCTL work; neither substitutes for signed evidence.

This completes the current non-external remediation evidence only. R2c-R remains not accepted,
R2c-O remains active, and signed publisher/bundle, elevated service, real journal, separately
authorized retained-root and real Cloud Files, and final accumulated independent audit evidence
remain open.

### Cadence-binding audit remediation

- The independent acceptance review reported zero Critical, zero High, zero Medium, and one Low
  finding. `library_synchronization.dart` owned the default 250 ms cadence and passed
  `pollInterval` to `Timer.periodic`; `main.dart` used the default. The closed-process Rust fixture
  nevertheless owned another 250 ms constant, so a Dart-only change could leave the gate falsely
  green.
- The executable mutation red control changed only the embedded Dart constructor declaration from
  250 ms to 875 ms. The old exact contract still returned 250 ms and failed with `left: 250ms` and
  `right: 875ms`; 915 tests were filtered out. No source library or real catalog was involved.
- The corrected `cfg(test)` support uses compile-time `include_str!` for the authoritative
  synchronization owner and `main.dart`. A closed tokenizer excludes comments and Dart string forms,
  including nested interpolation, before a strict local syntax contract admits exactly one owner
  class, one duration default, one typed field, one `Timer.periodic(pollInterval, ...)` consumption,
  and one production construction without `pollInterval:`. Missing, duplicate, ambiguous,
  unconsumed, and overridden sources all fail closed. The parsed `Duration` is the sole cadence used
  by worker readiness, crash, and recovery waits; the independent Rust constant is deleted.
- Green source-contract evidence is six tests for current-source admission, Dart-value propagation,
  missing/duplicate defaults, ambiguous owner classes, unbound timers, and `main.dart` override,
  plus one source control excluding the former Rust constant/literal. Test-profile warnings-denied
  Clippy passes. A locked Release library build completes, and ASCII/UTF-16 scans of the Release DLL,
  static library, and rlib report zero matches for five parser/source seam tokens. The code and
  embedded Dart text exist only in the already-test-only module and add no production runtime
  dependency.
- The workspace-sandbox guardrail attempt failed at the native compiler ownership-fault expectation
  and is not counted green. The identical ordinary-user guardrail passes all 19 cases; production
  `ValidationOnly` passes the exact 19/15/4 matrix on Windows 11 x64 build 26340. One fresh complete
  ordinary-user runner passes 19/19. Its bound closed-process case reports six samples at
  567/581/581 ms P50/P95/maximum, 24 fresh availability probes, zero source enumeration and inventory
  reads, three bounded content opens, and unchanged scan rows/source snapshots. The three fresh exact
  samples from the independent review remain prior audit evidence; no temporary harness or public
  selector was introduced only to duplicate them.

This remediation closes only the Low validation-drift finding. R2c-R remains not accepted, R2c-O
remains active, and signed publisher/bundle, elevated service, real journal, separately authorized
retained-root and real Cloud Files, and final accumulated acceptance evidence remain open.

### Secondary cadence-contract audit remediation

- The phase-19 independent follow-up audit found two Low proof defects. The Dart contract admitted a
  synchronization constructor moved into an unused top-level helper and a periodic timer moved out
  of `_start` into an unused class method. The Rust regression guard also treated comments and
  strings as numeric cadence code while missing `Duration::from_millis(250_u64)` and other
  independent interval expressions.
- Four executable red controls preceded the fix. The former parser returned success for both dead-
  code moves, so their `is_err()` assertions failed. The former raw containment predicate rejected a
  comment/string decoy, while the first complete-worker wait mutation remained accepted. The wait
  fixture was then corrected to preserve the full worker source for all three occurrence mutations;
  no malformed/truncated AST is accepted as evidence.
- The Dart source contract now admits exactly one top-level `Future<void> main() async` body, one
  direct success `try`, and one zero-argument `RustLibrarySynchronization` construction. That exact
  variable must feed one lifecycle owner and the production provider override; the same lifecycle
  variable must feed the shutdown registration and ordered `startInBackground` call. The owner class
  must contain one real async `_start(int generation, BigInt ownerTicket)` method, and its sole
  `Timer.periodic` call must be inside that method with `pollInterval` as the first argument. Duplicate
  entrypoints/methods, broken links, overrides, dead helpers/methods, and ambiguous identifiers fail
  closed. Comment, string, whitespace, and line-break mutations remain green.
- The closed-worker contract now uses the existing dev-only `syn` parser instead of raw containment.
  It selects one top-level worker, one direct immutable `synchronization_poll_interval` local whose
  initializer is exactly the zero-argument production cadence function, one owner call, and three
  `wait_until_at_interval` calls whose interval expression is the same unqualified identifier. Every
  individual `250_u64` mutation, arithmetic expression, alias, second owner, missing owner, duplicate
  worker, or malformed Rust source fails; comments and string literals do not participate in the
  AST. The parser, visitors, `include_str!` payloads, and fixtures remain in the existing `cfg(test)`
  module, and `syn` remains a dev dependency.
- Fresh green evidence is 14 cadence-contract tests plus the production worker AST contract and
  warnings-denied all-target/all-feature Clippy. The sandbox runner attempt failed before fixture
  creation at the known ancestor-pin Win32 5 boundary and is not counted. The identical ordinary-
  user runner passed all 19 internal-disposable cases on Windows 11 x64 build 26340. Closed-process
  evidence is six samples at 577/592/592 ms P50/P95/maximum, 24 availability probes, zero source or
  inventory reads, three bounded content opens, and unchanged scan rows/source snapshots. The
  no-change case remains 100 starts, 200 polls, 200 probes, and zero enumeration, inventory, or media
  access.
- A read-only scan of the retained Release DLL, static library, rlib, and packaged DLL reports zero
  ASCII or UTF-16LE matches for six new parser/AST seam tokens. No artifact was rebuilt or relabelled,
  because this remediation changes only test support and documentation. No real library, retained
  catalog, external broker, elevation, SCM, named pipe, or FSCTL path was used.

This closes the two Low proof findings only. R2c-R remains a non-external controlled checkpoint and
is not accepted; R2c-O remains active. Signed publisher/bundle, elevated service, real journal,
separately authorized retained-root and real Cloud Files, and final accumulated independent audit
evidence remain open.

### Phase-21 statement-ownership and AST-ancestor remediation

- The phase-21 independent re-review reported zero Critical, zero High, zero Medium, and two Low
  findings. The Dart delimiter-depth proof admitted `if (false) try`, a
  `holder.synchronizationLifecycle` receiver suffix, and the timer under either `if (false)` or an
  uncalled local function. The recursive Rust visitor admitted a wait under `if false`, an uncalled
  closure, or a nested item and missed both a qualified production-helper owner and a
  `use ... as cadence_alias` owner.
- Eight separate mutation fixtures were red against the previous implementation: each of the four
  required Dart mutations and each of the constant-false wait, closure wait, qualified-owner, and
  aliased-owner Rust mutations made its rejection test exit 1 with zero passes and one failure. The
  full Rust worker source remained parseable by `syn`; every Dart fixture remained tokenizable and
  retained the complete owning method or entrypoint. Additional green controls cover nested Dart
  blocks/closures, early `return`/`throw`, and a nested Rust item.
- The current Dart gate uses a minimal statement-owner cursor for the fixed production shape. The
  unique top-level `main` must directly own its `try`; the try success block must directly own the
  exact synchronization declaration, lifecycle declaration, shutdown registration, production
  provider binding, and complete lifecycle start receiver in order. Non-simple statement owners and
  early termination before the start fail closed. The unique `_start` must directly own its retry
  loop; that loop must directly own the succeeded-status `if`; and that success block must directly
  assign `_timer = Timer.periodic(pollInterval, ...)` before the direct `started` return. Merely
  placing the same tokens in a nested block, branch, closure, or local function cannot satisfy it.
- The current Rust gate uses `syn` for both exact statement slots and whole-worker closure. It admits
  one direct production-derived cadence local, a direct ready wait immediately after `runtime`, the
  first direct statement wait in the direct `crash-ready` branch immediately after
  `availability_readiness_probes`, and a direct visible wait between `ready` and
  `ready_to_visible`. A separate visitor requires exactly three wait-call paths, exactly one path
  whose final segment is `production_synchronization_poll_interval`, and zero `UseTree` references
  to that helper. Qualified or aliased owners and calls moved into constant-false branches, closures,
  or nested items fail. This is a strict proof of the current production statement shape, not a
  claim that the fixture decides arbitrary Rust or Dart reachability.
- Fresh focused evidence is 25/25 cadence tests and warnings-denied all-target/all-feature Clippy.
  The sandbox runner failed before fixture creation at the known ancestor-pin Win32 5 boundary and
  is not counted. The identical ordinary-user runner passed 19/19 internal-disposable cases on
  Windows 11 x64 build 26340. Closed-process evidence is six samples at 560/580/580 ms
  P50/P95/maximum, 24 availability probes, zero source or inventory reads, three bounded content
  opens, and unchanged source/scan evidence. No-change remains 100 starts, 200 polls, 200 probes, and
  zero enumeration, inventory, full-scan, or media access.
- A read-only ASCII/UTF-16LE scan found zero matches for eight new statement/visitor seam tokens in
  the retained Release DLL, static library, rlib, and packaged DLL. No Release artifact was rebuilt
  or relabelled. No real library, retained catalog, external broker, elevation, SCM, named pipe, or
  real FSCTL path was used.

This remediates only the two phase-21 local proof findings. R2c-R remains a non-external controlled
checkpoint and is not accepted; R2c-O remains active. Signed publisher/bundle, elevated service,
real journal, separately authorized retained-root and real Cloud Files, and final accumulated
independent acceptance evidence remain open.

### Phase-23 shared synchronization policy remediation

- Phase 23 removes the phase-19 through phase-21 cadence parser/visitor implementation. Those
  reviews usefully exposed drift, dead-code, ownership, alias, and ancestry risks, but the resulting
  handwritten Dart tokenizer/statement cursor plus Rust `syn` visitor remained a growing model of
  two languages rather than cadence authority. The historical findings above remain evidence; their
  implementation is superseded.
- `tool/library_synchronization_poll_interval_ms.txt` now owns the exact `250\n` policy.
  `quality_generate_library_synchronization_policy.ps1` accepts only one positive base-10 integer and
  LF, bounds the Dart value, and deterministically writes the generated Dart constant as UTF-8/LF.
  `-Check` is read-only. Its guardrail proves stale 250 output against a temporary 875 policy and
  rejects empty, zero, leading-zero, signed, unit-bearing, whitespace, CRLF, multiline, missing-LF,
  BOM, unsigned-overflow, and Dart-overflow inputs without changing output. Lint runs the guardrail
  and check before format.
- Dart production now exposes zero-argument `RustLibrarySynchronization.production()` only.
  Alternate calls, clocks, retries, diagnostics, and cadence remain on
  `@visibleForTesting .testing(...)`; analyzer configuration rejects production misuse as an error.
  The controller behavior test starts successfully without passing a cadence, intercepts the actual
  periodic-timer factory, and asserts its duration equals the generated constant. A second test
  retains explicit fast testing cadence.
- Rust `include_str!` reads the same policy directly and a strict parser admits a positive non-zero
  value through the shared cfg(test) `production_synchronization_cadence` module.
  `ProductionSynchronizationCadence` is stored by the production test harness; R2c-R composes it
  with readiness, crash-ready, and recovery counts while exposing no interval argument. Child
  reports record each count and the parent requires `1/1/0` for `crash-ready` or `1/0/1` for
  `recover`. The Dart/main includes, tokenizer, statement cursor, cadence-specific `syn` visitor,
  and more than 25 source-mutation tests are deleted; `syn` remains because the independent
  filesystem source-topology proof still uses it.
- Red evidence is executable: a tracked-policy 875/generated-250 drift fails `-Check`; a real timer
  hardcoded to 875 fails both the generated 250 and explicit 3 ms behavior assertions; `.testing()`
  in `main.dart` fails the fatal analyzer rule; and a recovery call counted as crash readiness fails
  with actual `1/2/0` versus expected `1/1/1`. Restored focused green is the generator and malformed
  matrix, 31 controller tests, three lifecycle-owner tests, two Rust cadence tests, the report-field
  test, and the ordinary-user 19-case lightweight guardrail.
- Warnings-denied Clippy and `quality_lint.ps1` pass. The complete ordinary-user runner passes 19/19;
  closed-process P50/P95/maximum is 566/703/703 ms with 24 availability probes, zero source or
  inventory reads, and three bounded media-content opens. The no-change case reports exactly 100
  starts, 200 polls, 200 availability probes, and zero source enumeration, inventory, full scan,
  media access, discovery, or publication.
- The serial Daily gate passes 900 Rust tests with zero failed and 17 expected ignored, broker binary
  integration 3/3, all Flutter tests, controlled Windows scan 2/2, native Windows accessibility 2/2,
  generated bridge compatibility, and tracked-diff whitespace validation. A fresh local unsigned
  Windows x64 Release build completes in 35.8 seconds. The app, packaged Rust DLL, and broker are PE
  x64; packaged and Cargokit DLL hashes match; policy text and generated Dart are absent from assets
  and dependency manifests; and five Rust Release artifacts contain none of eight deleted cadence-
  source seam tokens.
- The sandbox guardrail, lint, Daily, and complete-runner attempts fail only at the known held-parent
  reopen boundary (`NTSTATUS=0xC0000022`, Win32 5). Identical commands pass as an ordinary user; no
  gate was removed, skipped, or weakened.
- No real library, retained catalog, external broker, elevation, SCM, named pipe, or real FSCTL path
  participates in this focused evidence. Production bridge/API and Flutter assets are unchanged.

This remediation is still a non-external local checkpoint. It does not accept R2c-R or R2c-O and
does not replace signed publisher/bundle, installed-service, real journal, separately authorized
retained-root/Cloud Files, or final accumulated independent audit.

### Phase-24 numeric-boundary and R2c-M cadence remediation

- The phase-24 independent review reported zero Critical, zero High, two Medium, and zero Low
  findings. M1 identified a representation mismatch: Dart `Duration(milliseconds:)` multiplies by
  1,000 into signed 64-bit microseconds, so the shared safe millisecond maximum is
  `9223372036854775`, not signed-i64 or u64 maximum. Before remediation the generator successfully
  wrote `9223372036854776`, and the Rust must-reject control failed with
  `accepted invalid production cadence policy "9223372036854776\n"`.
- PowerShell normal and `-Check` modes now reject anything above `9223372036854775` before output
  changes. The guardrail proves exact maximum generate/check succeeds and that maximum plus one,
  u64 maximum, and u64 overflow fail in both modes without changing the sentinel output. The one
  shared Rust parser applies the same exact boundary; its old u64-maximum success assertion is
  removed. The current tracked cadence remains exact `250\n`.
- M2 identified a second semantic production cadence in R2c-M. With the tracked policy temporarily
  changed to 875, its old assertion failed with `left: 250ms` and `right: 875ms`. Parsing, policy
  inclusion, the Dart-safe bound, and `ProductionSynchronizationCadence` now live once in
  `rust/test_support/production_synchronization_cadence.rs` under the narrow common cfg(test) parent.
  `ProductionSynchronizationTestHarness` stores that value. R2c-M obtains each foreground interval
  from the harness, while R2c-R's counted stage wrapper composes `runtime.production_cadence()`.
- R2c-M and R2c-R behavior contracts each interpret a shared exact `875\n` value. A controlled
  tracked-policy 875 mutation makes the shared module, the normal R2c-M harness, and R2c-R counted
  wrapper pass together before both policy and current-value test are restored to 250.
- The follow-up review reported zero Critical, zero High, zero Medium, and one Low finding. The first
  fix still exposed a naked `Duration` getter and passed that primitive to R2c-M's real `wait_for`.
  Red changed the startup wait to `Duration::from_millis(250_u64)`; both the semantic source scan and
  875 accessor test remained green. Both are deleted. The cadence field is private, the opaque value
  owns waiting, every R2c-M call obtains it from the harness, and R2c-R's counted wrapper calls the
  same method before recording a phase. A compile-time function type locks the R2c-M helper's first
  parameter to `ProductionSynchronizationCadence`; replaying the naked mutation fails with `E0308`,
  expected that type and found `Duration`. The cfg(test)-only shared sleeper capture drives the real
  R2c-M helper once and records exactly 875 ms without sleeping; it is absent from Release.
- Focused generator and Rust policy/consumer tests, the non-accessing R2c-M guardrail,
  warnings-denied Clippy, and complete ordinary-user `quality_lint.ps1` pass. Lint reports the
  ordinary-user 19-case lightweight R2c-R guardrail, 150 formatted files with zero changes, and no
  Dart analyzer issues. The complete ordinary-user R2c-R runner passes 19/19; closed-process
  P50/P95/maximum is 565/578/578 ms for the initial correction and 565/585/585 ms for the follow-up
  opaque API, with 24 probes, zero source or inventory reads, and three bounded content opens.
  No-change remains exactly 100 starts, 200 polls, and 200 probes.
- The sandbox runner still fails only at the held-parent reopen boundary with
  `NTSTATUS=0xC0000022`, Win32 5; the identical ordinary-user command passes. Phase 24 changes no
  production Dart or Release payload, so complete Daily and Release builds were not rerun after the
  immediately preceding phase-23 green. Read-only scans find no policy/test-support path in assets or
  dependency files; a fresh eight-token opaque-cadence scan finds zero matches in five retained
  Release Rust artifacts.
- No R2c-M retained catalog/root acceptance, real library, external broker, elevation, SCM, named
  pipe, real FSCTL, or real Cloud Files path participated. R2c-M's historical accepted evidence and
  runnable entrypoint remain; no new retained-root claim is made.

This remediation remains a non-external local checkpoint. It does not accept R2c-R or R2c-O and
does not replace signed publisher/bundle, installed-service, real journal, separately authorized
retained-root/Cloud Files, or final accumulated independent audit. R2c-O remains active.

### Eighth-remediation fresh verification

- All three reported gaps were first executable red controls against the previous implementation.
  The Rust loader mutations returned `Ok(())` for `cfg_attr(not(test), path = ...)` before topology
  validation. The PowerShell audit opened a real case-sensitive NTFS directory successfully but
  collapsed `Safe.ps1` and `safe.ps1`, so it missed the forbidden lower-case source; an ordinary
  duplicate also opened only once. A one-literal PowerShell AST has an independently known five
  nodes, but the old count reported `budget=ast-nodes limit=5 actual=6` after materializing the full
  `FindAll` result and counting the root twice. The high-entropy fixture files and directory were
  removed after the red run, and no retained-library or external path was used.
- The exact Rust proof still locks 17 cfg-qualified function items and 16 support items, and now also
  locks module-loading topology in the local, adapters-parent, crate-parent, domain, and metadata-
  domain source scopes. Visibility, attributes, module names, and inline/external shape must match.
  File attributes, `cfg_attr(path)`, direct `path`, arbitrary/procedural attributes, `include!` item
  macros, extern-crate aliases, and unexpected nested/generated modules fail structurally. The one
  current top-level `thread_local!` item is test-only and digest/cfg locked. The unconditional private
  crate-to-adapters-to-local and crate-to-domain-to-metadata chains prove test and non-test production
  cfg load the same implementation. Same-module macro, parent macro, renamed import, and current exact
  availability controls all pass; production availability code remains unchanged and O(1).
- File-source deduplication is now post-open and identity authoritative. The path dictionary uses
  `StringComparer.Ordinal` only as an index. Every request first consumes source-count admission and
  creates a no-follow snapshot; the held volume/file ID selects any existing source. A duplicate
  snapshot closes before return, a unique snapshot transfers one owner, and every exceptional pre-
  transfer path closes locally. The ordinary duplicate reports two opens, one immediate close, one
  retained owner, and two total closes after state shutdown. The case-disabled alternate spelling
  opens twice and resolves to one identity while retaining both ordinal keys. Mismatched supplied text
  leaves zero held streams and closes its one open.
- The real Windows case-sensitive control passes rather than skipping on this workstation. It creates
  `Safe.ps1` and `safe.ps1` under an identity-held high-entropy tool child. Both receive independent
  snapshots; the forbidden `[System.IO.File]::Delete` in the lower-case file makes the closure fail
  closed. Setup uses the held-parent native case flag, and cleanup removes only the two known fixture
  files and the exact empty identity-matching child. Unsupported or access-denied platforms emit an
  explicit `status=skipped` with the native error instead of reporting the control as passed.
- AST-node admission is a bounded traversal with an always-false predicate, so it forms no node
  collection. The root is counted once; each visit updates a separate high-water; limit-plus-one
  throws immediately and retains the attempted actual. Independent fixtures assert two nodes for an
  empty script and five for a literal, then pass at limit-minus-one and limit and fail at limit-plus-
  one with `budget=ast-nodes limit=4 actual=5`. Source, depth, per-source bytes, total bytes, function,
  scope, and queue controls remain green. The measured current maximum is 3 sources, depth 1, 162,853
  bytes per source, 251,983 total bytes, 18,901 nodes, 62 functions, 56 scopes, and queue high-water
  49.
- All three PowerShell files parse. The lightweight guardrail reports
  `AME_R2C_R_CASE_SENSITIVE_SOURCE status=passed` and then its exact 19-case pass. Production
  `-ValidationOnly -GuardrailWorkspaceAnchor` reports 19/15/4 on Windows 11 x64 build 26340 as an
  ordinary user. Rust topology, macro and exact source controls, 100-start no-change, report tamper,
  root replacement, and both Cloud Files cases pass. The sandboxed first Cloud attempt failed with
  Win32 5 and was not counted green; the identical ordinary-user command passed outside the workspace
  sandbox.
- Formatting, development and Release all-target/all-feature checks, warnings-denied Clippy, and
  `quality_lint.ps1` pass. Lint reruns the real case-sensitive control and exact 19-case guardrail,
  reports 149 formatted files with zero changes, and completes Dart analysis with no issue. The
  remediation changes acceptance PowerShell and test-only Rust proof only; it changes no production
  ABI, bridge, package graph, or runtime code, so no new unsigned or signed Release is required or
  claimed. Prior Release images remain historical packaging evidence, not current-tree binaries.
- The first complete ordinary-user controlled runner failed closed in watcher overflow with SQLite
  `FileLockingProtocolFailed`; it returned nonzero and removed its exact high-entropy root and process
  tree. It was not counted green. Two complete serial reruns then each emitted
  `AME_R2C_R_REPORT status=passed cases=19`, including watcher overflow, one-million backlog, 100-start
  no-change, priority, tamper, replacement, and both placeholder cases. Both rerun roots were removed.
- The complete serial Daily exits zero. Rust reports 863 passed, zero failed, and 16 expected ignored;
  broker binary integration passes 3/3; all 309 Flutter unit/widget tests pass; controlled Windows
  scan and native Windows accessibility pass 2/2 each; generated bridge and tracked-diff whitespace
  stages complete. Final process inspection finds no matching Cargo, Rust, broker, Flutter, tester, or
  Dart process. Residue inspection still finds exactly the five empty compiler-bootstrap directories
  and one three-entry failed-run root created on 2026-08-31. No eighth-remediation command added a
  residue, and those historical objects were left untouched because this run has no historical
  identity authorization to delete them.

### Seventh-remediation fresh verification

- Red evidence preceded the fixes. The exact 17-function/16-support Rust digest accepted a real
  same-module `macro_rules! vec` environment without noticing that the protected `vec!` could now
  enumerate; the new test failed at that acceptance. A 300,002-byte in-memory PowerShell source
  also passed the old unbounded audit. A real child PowerShell probe showed that an anonymous
  `EncodedCommand` wrapper preserves runner argument binding but resets both `$PSScriptRoot` and
  `$PSCommandPath` to empty, so it cannot preserve the three-source runner semantics without
  rewriting those sources and creating a second loader.
- The corrected Rust control removes `vec!` from production and rejects every macro expression in
  the protected closure. It also validates protected attributes/derives and the relevant local,
  domain, metadata-domain, adapters-parent, and crate-parent macro environments. Same-module macro,
  parent macro, and renamed-import fixtures all fail with the exact source key; the current O(1)
  availability closure and its exact callee/receiver contract pass.
- File-backed PowerShell audit sources now use `AmeR2cRAuditedScriptSnapshot`, not an ordinary
  followed-path stream. It opens the tool-root chain, each nested parent, and terminal through
  parent-relative no-follow native calls, retains volume/file identity and no-delete handles,
  rejects terminal reparse/directory state, and denies terminal write sharing. Both the payload
  state and the digest-locked wrapper state are revalidated by held-parent relative reopen
  immediately before process transfer. The native Job suffix and audited-script snapshot have
  independent digests. Ordinary identity, terminal junction, terminal replacement, parent
  replacement, and injected exceptional-open controls pass without touching a retained library.
- Closure construction is bounded at 8 sources, depth 8, 256 KiB per source, 512 KiB total,
  32,768 AST nodes, 128 functions, 512 scopes, and queue high-water 512. Checked-add errors identify
  `budget`, `limit`, and `actual` or `overflow`; limit-minus-one, limit, limit-plus-one, concrete
  source/byte/depth/node/function/scope/queue, multi-source, and overflow controls all pass. The
  post-remediation real peak is 3 sources, depth 1, 158,839 bytes per source, 239,967 total bytes,
  17,943 nodes, 60 functions, 54 scopes, and queue high-water 47.
- All three PowerShell files parse. The lightweight guardrail exits zero and reports
  `script_identity=no-follow-held-revalidated audit_budget=bounded-checked`; production
  `-ValidationOnly -GuardrailWorkspaceAnchor` reports the exact 19/15/4 matrix on Windows 11 x64
  build 26340 as an ordinary user. These are non-external controls and do not accept R2c-R.
- Fresh seventh-remediation verification passes the four-test Rust availability contract, exact
  no-change, report-tamper, root-replacement, and both Cloud Files cases, format, development and
  Release all-target/all-feature checks, warnings-denied Clippy, and `quality_lint.ps1`. The complete
  ordinary-user runner passes all 19 cases from one physical KnownFolder fixture. The serial Daily
  gate reports 862 Rust library tests passed, zero failed, and 16 expected ignored; broker binary
  integration 3/3; Flutter unit/widget tests 309/309; controlled Windows scan 2/2; native Windows
  accessibility 2/2; generated bridge compatibility; and tracked-diff whitespace validation.
- The only production Rust edit is an O(1)-equivalent standard-library `Vec` construction; it does
  not change an ABI, bridge, package graph, or runtime policy. No new unsigned or signed Windows
  Release is claimed. The fifth-remediation Release remains prior packaging evidence and is not
  relabelled as a current-tree build. An ASCII/UTF-16LE scan finds seven new seventh-remediation
  macro, identity, fault, and budget tokens absent from the retained application, packaged DLL,
  Cargokit DLL, and independent broker images. That negative seam check does not make those images
  current or supply a signature.

### Sixth-remediation fresh verification

- The adversarial controls were added before the corrections. The old Rust guard went red by
  accepting the local-callee-shadow fixture, and the old PowerShell audit went red on the new
  sixth-review bypass matrix before the default-deny closure existed. That matrix includes the
  reported top/helper/owner call-operator, variable dot-source, `cmd rmdir`, and unregistered
  `EncodedCommand` entries. Native fault injection exposed the remaining volume and bootstrap
  acquisition windows before ownership was reordered. The completed controls retain every review
  category rather than treating those red results as environmental failures.
- The corrected Rust exact source test passes 1/1. It binds 17 named cfg-qualified function items
  and 16 named support items with a readable purpose, normalized-token digest, exact local-callee
  closure, and call/receiver evidence in every mismatch. It also scans the complete crate for
  `Drop` and side-effecting operator traits on the six reachable availability types while leaving
  unrelated module implementations outside the policy. Callee-local shadow, receiver shadow,
  unchanged-owner/helper `read_dir`, `Drop`, `BitOr`, and referenced static/lazy fixtures are all
  rejected, and the current production AST is the green control. Exact `quote` 1.0.47 remains a
  dev dependency; the production availability path is unchanged and O(1).
- All three PowerShell files parse and their actual source closures pass. The audit holds exact
  source snapshots, recursively resolves real local helpers and exact repository-tool dot-sources,
  uses a static cmdlet/module allowlist, and rejects every unresolved command, call operator,
  variable dot-source, unknown helper, external executable, dynamic script, reflection invocation,
  or path-addressed mutation in every scope. The sole process wrapper and native Job Object suffix
  are digest-locked; before launch it validates exact Cargo arguments or audits the final actual
  `Command`, canonical `EncodedCommand`, or held `File` source. Its 45 adversarial source/runtime
  fixtures cover the reported seven plus module-qualified and aliased removal, alias definition,
  `IEX`, `ScriptBlock.Create`, `Invoke-Command`, nested command/encoding, PowerShell/pwsh/cmd
  `Start-Process`, `System.IO` delete/move/replace, `cmd del/rmdir`, `robocopy /MIR`, reflection,
  unknown helpers, `Add-Type`, `Move-Item`, forceful `New-Item`, and wrong-module qualification.
  Every actual legal common/runner/guardrail child payload passes the same boundary.
- The ordered native fault control runs in the parent process before controlled native-type
  initialization. `volume-post-open-pre-transfer` and
  `bootstrap-post-create-pre-initialization` both report their owner closed; neither leaves partial
  native types, and all later fresh children continue through the same verified wrapper and owned
  Job Object. No direct process fallback was added.
- The ordinary-user lightweight guardrail passes all 19 cases. Production `-ValidationOnly` passes
  the exact 19/15/4 matrix. The no-change exact test passes 1/1 in 6.71 seconds with 100 starts,
  exactly 200 polls and 200 metadata probes, and zero enumeration, inventory, full-scan, media,
  discovery, or publication work. Exact report tamper, root replacement, and both Cloud Files cases
  pass. Where the workspace sandbox denied the held KnownFolder ancestor with Win32 5, the same
  scoped command was rerun as the ordinary user outside that sandbox and passed; no blocked result
  is recorded as green.
- `cargo fmt --check`, all-target/all-feature check, and warnings-denied Clippy pass; the latter two
  complete in 14.46 and 17.89 seconds. `quality_lint.ps1` passes as the ordinary user with the
  19-case R2c-R guardrail, 149 files requiring zero formatting changes, warnings-denied Clippy,
  clean Dart analysis, and the other repository guardrails green.
- The complete ordinary-user controlled runner emits its exact final pass report for all 19 cases
  on Windows 11 x64 build 26340 (`26H2`). Representative case durations are 22.72 seconds live,
  3.30 seconds closed-process, 18.67 seconds overflow, 10.75 seconds million-record, and 40.01
  seconds reserved-priority; the priority visible P95 is 69 ms. The no-change counters remain zero
  for every prohibited normal-path operation.
- The complete serial Daily gate exits zero. Rust reports 861 passed, zero failed, and 16 ignored;
  broker integration passes 3/3; Flutter unit/widget tests pass 309/309; controlled Windows scan
  passes 2/2; native Windows accessibility passes 2/2; and the bridge and tracked-diff whitespace
  stages complete without error.
- Final read-only residue inspection finds the same five empty compiler-bootstrap directories and
  one three-entry failed-run disposable root created on 2026-08-31. No sixth-remediation run added a
  residue. They remain intentionally untouched because this review has no complete historical
  identity token authorizing their deletion.
- No new Release build is claimed because this remediation changes only acceptance PowerShell,
  `#[cfg(test)]` Rust source proof, and exact dev dependencies. The fifth-remediation fresh unsigned
  Release therefore remains the applicable product artifact. A new ASCII/UTF-16 check over the
  application, packaged and Cargokit Rust DLLs, and independent broker finds zero occurrences of
  either sixth-remediation native-fault token. This reuse is not signed Release, SCM, real
  named-pipe/FSCTL, retained-root, or Cloud Files evidence.

### Fifth-remediation fresh verification

- Red evidence reproduced each fifth-review bypass before the correction: the prior Rust
  callable-name guard admitted an aliased enumeration call; the prior PowerShell check reported zero
  violations for module-qualified, aliased, dynamic, and recursively encoded deletion; and both
  injected native post-open handles remained open, with the failed-root rename blocked in the same
  process. The corrected exact Rust structural-allowlist test passes 1/1 in 0.26 seconds after a
  20.875-second fresh wrapper run. All three PowerShell owners parse with zero errors; the reusable
  AST audit accepts the three current files plus one legal fixture and rejects all four malicious
  fixture categories in 0.533 seconds. The native fault fixture then reports both handles closed,
  no retained root handle, and successful same-process rename plus identity-bound deletion in 0.736
  seconds.
- The ordinary-user lightweight guardrail passes all 19 cases in 10.343 seconds, including actual
  child-payload recursion and exact tamper execution. Production `-ValidationOnly` passes the exact
  19/15/4 matrix in 1.031 seconds. The 100-start no-change exact test passes in 9.040 seconds with
  exactly 200 production polls, 200 metadata-only probes, 100 watcher root handles, 100 journal
  register/query/close cycles, and zero journal reads, enumeration, inventory, full-scan rows,
  media opens, discovery handles, or publication-guard handles. The exact report-tamper test passes
  1/1 in 0.940 seconds.
- `cargo fmt --check`, development and release all-target/all-feature checks, and warnings-denied
  Clippy pass in 2.657, 8.981, 18.026, and 15.723 seconds. `quality_lint.ps1` passes in the ordinary-
  user context in 46.297 seconds with the 19-case guardrail, 149 files requiring zero formatting
  changes, warnings-denied Clippy, and clean Dart analysis.
- The complete ordinary-user controlled runner passes all 19 exact cases in 136.624 seconds. Live
  P50/P95/maximum is 389/439/439 ms; closed-process recovery is 353/369/369 ms; overflow P0 is
  343/363/375 ms with 19.269-second convergence; the one-million stream completes 245 pages in
  13.459 seconds with a 3,711,278-byte maximum allocator estimate below its 3,809,886-byte fixture
  bound; and reserved-priority P0 is 82/98/100 ms. The same-volume case retains two roots, performs
  one shared read, and isolates one root failure.
- The complete serial Daily gate passes in 711.104 seconds. Rust reports 861 passed, zero failed,
  and 16 expected ignored tests in 192.52 seconds; broker integration passes 3/3; Flutter unit/widget
  tests pass 309/309; Windows scan and native Windows accessibility pass 2/2 each; and bridge plus
  tracked-diff whitespace validation complete without error.
- The internal unsigned Windows x64 application builds in 38.5 seconds, and the independent broker
  Release target builds in 75 seconds. The application, both Rust DLL copies, and both inspected
  broker copies are PE machine `0x8664`. The 81-file DLL graph and both 82-file broker graphs contain
  no missing or newer dependency and no named fifth-remediation seam. Packaged and Cargokit DLL
  SHA-256 values both equal
  `09DECC5B7B8DD80B955B518A67BECC5FBC74EAD2269320181F697BDE04017ED7`. Boundary-qualified
  ASCII/UTF-16 scans of the application, packaged DLL, Cargokit broker, and independent broker find
  zero matches across 17 R2c-R fixture, structural-guard, counter-report, or native-fault tokens.
  Authenticode reports `NotSigned`, as expected for this internal build. Missing publisher, broker,
  or Application input each fails formal Release admission closed with zero new process; separately
  nonexistent signed-artifact paths fail at immutable bundle admission in 0.333 seconds. None of
  this is signed Release, SCM, or real broker-FSCTL evidence.

### Fourth-remediation fresh verification

- All three R2c-R PowerShell scripts parse. The ordinary-user guardrail passes in 11.757 seconds
  with 19 cases, Reflection.Emit bootstrap, compiler-failure fail-closed, two-stage replacement
  retention, held KnownFolder boundary, internal-junction rejection, process-tree timeout, and exact
  tamper execution. Production `-ValidationOnly` passes in 1.20 seconds with the exact 19/15/4
  matrix.
- The representative Rust enumeration source fixture is first red against the removed handwritten
  brace extractor, then passes with the `syn` visitor in 0.19 seconds. `cargo fmt --check`, all-
  target/all-feature development check in 18.05 seconds, and warnings-denied Clippy in 20.86 seconds
  pass. Cargo metadata and tree evidence show exact `syn` 2.0.119 as a direct dev dependency under
  its existing `MIT OR Apache-2.0` license, with no added non-dev release edge.
- The 100-start no-change exact test passes in 10.50 seconds with 100 starts, 200 polls, 200 metadata
  probes, and every enumeration, inventory, full-scan, media, discovery, and publication count at
  zero. The three SQLite fixture drop-order tests pass 3/3 inside a fresh identity-held
  LocalApplicationData root and remove that root safely.
- `quality_lint.ps1` passes in the ordinary-user context in approximately 89 seconds through a fresh
  repository-local `APPDATA`/`LOCALAPPDATA` profile with analytics suppressed. It reports all R2c-H,
  R2c-M, and R2c-R guardrails green, 149 formatted files with zero changes, warnings-denied Clippy
  green, and no Dart analyzer issue. The isolated profile is removed afterward.
- The resource-bounded complete Daily gate passes. Rust reports 861 passed, zero failed, and 16
  expected ignored tests in 495.51 seconds; broker binary integration passes 3/3 in 2.38 seconds;
  Flutter unit/widget tests pass 309/309; controlled Windows scan and native Windows accessibility
  integrations pass 2/2 each; and bridge plus tracked-diff whitespace validation complete without
  error.
- The fresh ordinary-user controlled runner passes all 19 exact cases and emits one exact final
  report after safe root cleanup. Live P50/P95/maximum is 390/435/435 ms; closed-process recovery is
  359/1,058/1,058 ms; overflow P0 is 331/346/371 ms with 23.570-second convergence; the one-million
  stream completes 245 pages in 12.798 seconds; and reserved-priority P0 is 79/946/1,027 ms with
  queue/worker P95 of 39/38 ms. The last maximum is above one second, but the specified P95 contract
  remains below one second and passes.
- Release-profile all-target/all-feature check and warnings-denied Clippy pass in 24.88 and 26.68
  seconds. The unsigned Windows x64 application builds in 181.82 seconds without signing or external
  service access. The application, packaged Rust DLL, and Cargokit Rust DLL are PE machine `0x8664`.
  All 81 Cargokit dependency files exist and are no newer than the DLL; packaged and built DLL
  SHA-256 values match. Dependency evidence and ASCII/UTF-16 scans of both Release binaries contain
  zero R2c-R fixture, `test_support`, environment, counter, or source-guard seams. Explicitly absent
  signed Application and broker inputs fail `release_verify_windows.ps1` closed in 0.51 seconds with
  `The Ame application bundle was not found`, before packaged-process, SCM, or broker FSCTL entry.
- Five empty compiler-bootstrap directories from earlier interrupted development runs and one
  failed-run disposable root containing three SQLite temporary directories remain intentionally
  retained because their complete creation-time identity tokens are unavailable. The fourth-
  remediation focused rerun and final 19-case runner created no new residue. No path-addressed
  deletion was used to conceal the retained forensic state.

### Earlier third-remediation evidence

- The third-remediation scripts parse. The final hostile-bootstrap, compiler-failure, active
  bootstrap race, ordinary/junction cleanup race, malicious-leaf, timeout, and exact tamper
  guardrail
  passes in 12.39 seconds; workspace-held `-ValidationOnly` reports the exact 19/15/4 matrix in 0.73
  seconds. The opaque availability and executable source-guard tests pass 3/3; the final recompiling
  run completes in 25.06 seconds. The
  100-start no-change production test passes in 9.45 seconds with exactly 200 polls, 200 metadata
  probes, and zero enumeration, inventory, full-scan, media-open, discovery, or publication work.
- The third-remediation `quality_lint.ps1` run passes in 84.63 seconds through a fresh verified
  workspace-only `APPDATA`/`LOCALAPPDATA` profile with analytics suppressed. Its R2c-R guardrail
  actually executes the exact tamper test, reports 149 formatted files with zero changes, passes
  warnings-denied Clippy, and reaches a clean Dart analysis; the profile is removed afterward.
  Rust formatting, all-target/all-feature development and release checks, and warnings-denied Clippy
  also pass independently with one build job.
- The current Cargokit Windows x64 Release Rust target rebuilds in 128.01 seconds without invoking
  signing or external service paths. CMake installation refreshes the repository-local bundle; the
  application and Rust DLL are PE machine `0x8664`, all 81 dependency files are present and no newer
  than the DLL, the packaged and built DLL SHA-256 values match, and dependency plus ASCII/UTF-16
  scans contain zero R2c-R test seams. Explicitly absent signed Application and broker inputs still
  fail closed in 0.47 seconds with `The Ame application bundle was not found`, before
  packaged-process, SCM, or broker FSCTL execution.
- A one-off third-remediation sandbox diagnostic temporarily admitted the runner matrix to an
  internal workspace anchor; the final runner again limits that anchor to `-ValidationOnly`. The
  diagnostic passed the first exact production-coordinator case, then the live-storm case reached
  the
  unchanged production namespace guard and failed with Win32 5 while the workspace sandbox denied
  the user-profile ancestor pin. Cleanup retained rather than traversed the non-empty owned root;
  the single expected case log, direct-child identity, and non-reparse state were verified before
  the test fixture was removed without recursion. The production guard was not
  weakened, and this sandbox run is not recorded as a fresh 19/19 result. A complete Daily attempt
  likewise passed lint and the new focused tests but was not green because multiple pre-existing
  filesystem tests reached the same sandbox ancestor-pin denial. The earlier ordinary-user full gate
  evidence below remains historical evidence, not a substitute for rerunning outside this sandbox.

The following bullets record the earlier complete second-remediation ordinary-user gate set.

- All three R2c-R PowerShell scripts parse. The fresh guardrail passes in 7.70 seconds and
  `-ValidationOnly` passes in 0.93 seconds. Explicit Rust 2024 formatting passes for the four Rust
  files changed by the second remediation. Development- and release-profile Cargo checks and
  warnings-denied Clippy pass with one Cargo build job.
- `quality_lint.ps1` passes in the ordinary Windows user context. It reports all R2c-H, R2c-M, and
  R2c-R guardrails green, 149 formatted files with zero changes, Rust Clippy green, and no Dart
  analyzer issue.
- The resource-bounded complete Daily gate passes with one Cargo build job and one Rust test
  thread. Rust reports 860 passed, zero failed, and 16 expected ignored tests in 219.75 seconds;
  broker binary integration passes 3/3 in 2.07 seconds. Flutter unit and widget tests pass 309/309,
  the controlled Windows scan integration passes 2/2, and native Windows accessibility passes 2/2.
- The internal unsigned Windows x64 Release application builds in 67.3 seconds. The repository PE
  parser verifies machine `0x8664` for both the application and Rust DLL. All 81 Cargokit dependency
  files exist and are no newer than the built DLL, and the packaged DLL hash matches the Cargokit
  output. Dependency evidence contains zero R2c-R fixture or `test_support` references; ASCII and
  UTF-16 scans of the Release application and Rust DLL find zero fixture, harness, environment, or
  counter-name matches. The `#[cfg(test)]` evidence seams therefore do not enter the Release payload.
- Formal `release_verify_windows.ps1` admission receives explicitly prevalidated nonexistent
  signed-application and signed-broker paths and fails closed with `The Ame application bundle was
  not found`. No packaged process starts, and it does not enter SCM or broker FSCTL. This negative
  admission is not a substitute for a real externally signed bundle.

## Phase-26 live-gap ownership remediation

The phase-25 cumulative audit's sole High found that production could turn a running-time or
availability `EvidenceGap` into an unowned `StartupCatchUp` P1 root marker. Phase 26 keeps those gaps
as durable `LiveNotification` P0 work and requires one persisted consumer before supersession:
either exact P1 source-range ownership when the journal is continuous, or an independent P2 control
with `watcher_uncovered_gap` authority and transferred lineage when it is not. If P2 capacity is
unavailable, the transaction rolls back and the original P0 remains retryable.

Focused ordinary-user production fixtures cover native `LiveOnly` `need_rescan`, observer-ingress
drop, offline-to-available recovery, expired-lease restart recovery, continuous journal ownership,
uncovered journal recovery, and P2 capacity rollback. Every convergence case requires
`Synchronized`, zero relevant queued work, zero automatic full scans, and exact lane, origin, scope,
intent, lineage, and consumer evidence. Schema v30 adds the durable claim contract. Its v29 forward
migration treats every naked historical fallback as ambiguous because publication namespace proves
root identity, not event provenance; each row remains fail closed with explicit recovery ownership.
Migration evidence includes fresh, legacy, idempotent, conflict, and malformed-partial-schema cases.

No retained root, real Cloud Files placeholder, SCM, real named-pipe/FSCTL journal, signing, or
external release admission is used by this remediation. The existing public R2c-R commands and
19-case matrix remain stable.

## Phase-27 explicit manual recovery and provenance remediation

Focused schema and application fixtures now cover the user-authorized recovery path without adding
a public R2c-R command or acceptance case. Foreground begin changes only an
`explicit_recovery_required` claim into a typed `foreground_scan` consumer and binds the scan ID,
root, and generation in one transaction. The generic scan high-watermark, lease, completion, and
abandon paths exclude every claim-owned gap, so a pending-journal claim retains its original owner.
Foreground publish atomically records the catalog revision, terminal gap, consumed claim, and scan
linkage. Abandon and interrupted reopen restore the original explicit claim as `retry_wait` with a
null retry deadline and the typed failure code.

The current-schema validator rejects inexact explicit, active foreground, and consumed foreground
relations. The v29 migration fixture includes the earlier legal pending P1 root fallback with no
lineage and a valid root publication identity, proving that identity cannot be used as event
provenance. The row and root identity remain intact; no journal evidence or recovery authority is
created. Because schema v30 has not shipped, the correction evolves the current v30 DDL and v29
migration in place. Partial prerelease v30 objects continue to fail closed rather than being silently
upgraded or relabeled as v31.

Repository metrics and the production synchronization snapshot expose a nonzero explicit-claim
count as `NeedsReconciliation`, `Blocked`, `recoveryBlocked=true`, and
`live_gap_v30_explicit_recovery_required`. The existing generated bridge field is unchanged; Dart
now preserves it, and the notification supplies explicit guidance plus an accessible `更新图库`
button. Successful publication clears the count and permits `Synchronized`; abandon or crash
restores the block. Generic persistence failures remain actionless.

This phase uses only disposable SQLite, production-runtime fixtures, and Flutter tests. It does not
run the 19-case wrapper, Daily, Release, retained-root, real Cloud Files, SCM, real named-pipe/FSCTL,
or signing gates; those remain accumulated closeout or external evidence. R2c-R remains a
non-external checkpoint and is not accepted. R2c-O remains active.

## Phase-27 capacity deferral and real-visibility completion

The remaining capacity path now distinguishes backpressure from processing failure. An exact leased
P0 root live gap uses the typed `MetadataInventoryLane` deferral, refunds the lease attempt, retains
a bounded non-null retry deadline, and survives lease expiry or restart without reaching terminal
exhaustion. P1/P2 completion and authority finalization wake the matching wait in their transaction.
A ready precise P0 path remains separately leasable, while a genuine non-capacity failure still
becomes terminal at the existing maximum-attempt limit. These invariants fit the current schema-v30
queue fields and require no v31 DDL evolution.

The production capacity fixture holds every P2 slot, drives a real live gap beyond
`max_attempts + 1` promotion attempts, releases one slot, and observes automatic P2 authority and
claim acquisition followed by `Synchronized`. Focused restart, lease-expiry, fairness, and
real-error fixtures cover the remaining boundaries without a user action or an automatic full
scan.

The production rotation fixture also proves that retained metadata-inventory sources are owned by
immutable change ID rather than root ID. It admits overlapping containment and watcher-gap
authorities for one root, keeps each run's source frontier independent, and prunes only a missing,
retired, or run-mismatched authority. Deterministic failure cases prove that catalog-session
validation never removes the source and that an injected worker-spawn failure restores it to the
same change ID before returning the start error.

The phase-26 native `LiveOnly` `need_rescan` production matrix now publishes a controlled two-PNG
foreground baseline before deleting one asset and adding another. Its real P2 metadata-inventory
consumer must publish the new asset with exact dimensions and byte size, publish exact absence for
the removed location, retain the unchanged asset, leave source bytes and hashes unchanged, drain
queue, claim, and authority state, reach `Synchronized`, and keep the full-scan run count unchanged.
The instrumentation boundary is two metadata-entry reads, one spool content open, and one terminal
media open. A rollback-only publication mutation proves that the former empty fixture and
queue/lineage assertions could remain green while the added asset was absent.

The accumulated ordinary-user closeout passes the exact visibility case and all 83 production
synchronization tests. Its first complete concurrent Daily run exposed a test-only observation
race: after valid promotion, the helper selected the newer P2 `metadata_inventory` row rather than
the retained P0 `live_notification` lineage owner. The helper now selects the exact
`live_notification`/`p0_live` row and still fails closed when that P0 does not exist. The corrected
Daily passes 927 Rust tests with zero failed and 17 expected ignored, broker integration 3/3, every
Flutter test, and both controlled Windows integrations 2/2. `quality_lint.ps1`, Release-profile
warnings-denied Clippy, bridge hash `941711727`, and the ordinary-user 19/19 runner also pass.

The fresh unsigned Windows x64 build completes in 58.2 seconds. Four application/DLL/broker PE
images are `0x8664` and `NotSigned`; 83/82/82 broker, rlib, and DLL dependencies are present and
current; packaged and Cargokit DLL hashes match. ASCII and UTF-16 scans find none of seven
ownership/fault test seams across six Release artifacts, and all ten Flutter assets exclude the
policy sources and test seams.

Only disposable source/catalog storage is involved. No retained root, Cloud Files placeholder,
SCM, real named-pipe/FSCTL journal, signing, source mutation, or automatic full scan is admitted.
R2c-R remains a non-external checkpoint and is not accepted; R2c-O remains active.

## Phase-31 migration-integrity remediation

- Red controls first proved all three phase-30 findings. Both running and paused v29 foreground
  fixtures lost the `authoritative_scan_id` join after upgrade because the migration cleared the
  real owner and created an explicit claim. An active foreground claim linked to an
  `authoritative_recovery` scan was accepted and then rewritten by reopen recovery; the corresponding
  consumed claim was accepted unchanged. Current-v30 crash cleanup deleted both a catch-up-handoff
  asset and a scan-handoff asset, leaving two dangling asset references while also deleting the true
  orphan.
- The v29 naked selector now requires a null authoritative scan ID in addition to the existing
  absence of catch-up, journal, recovery, baseline, supersession, and candidate ownership. Running
  and paused foreground fixtures preserve scan and row provenance, resume, publish, reopen, and
  satisfy the zero-work/zero-explicit-block conditions for `Synchronized`. Existing fixtures still
  convert truly naked rows both with and without root publication identity to conservative explicit
  recovery, and partial-v30 rollback remains exact.
- Active and consumed foreground relations require the associated scan owner to be exactly
  `foreground`; current-v30 interrupted-claim selection and terminalization use that same predicate.
  The two wrong-owner fixtures fail closed with
  `catalog_live_gap_recovery_contract_unverifiable`, disclose no configured path, and leave the scan,
  claim, and gap unchanged.
- Current-v30 crash repair now reuses the normal scan transaction's handoff-aware orphan cleanup.
  Both handoff-owned assets survive, the real orphan is deleted, handoff references remain resolvable,
  an injected delete failure rolls the complete recovery transaction back, and retry plus a second
  reopen are idempotent.
- Focused ordinary-user evidence passes the six new exact controls, 65/65 migration tests, 88/88
  SQLite catalog tests, and 36/36 runnable scan tests with two expected ignored acceptance cases.
  The scan suite's sandbox run failed at the Windows no-delete-sharing ancestor pin and was not
  counted; the identical ordinary-user command passed. Warnings-denied all-target/all-feature
  Clippy, ordinary-user `quality_lint.ps1`, 56/56 focused Flutter tests, bridge hash `941711727`,
  formatting, and the read-only absent-signed-bundle Release admission pass or fail closed as
  required.

No Daily, Release build, complete 19-case runner, retained library, real Cloud Files, SCM, real
named-pipe/FSCTL journal, signing, or source-media mutation was used. Those remain phase-32 or
external closeout evidence. R2c-R remains not accepted and R2c-O remains active.

## Phase-32 leased capacity-gap and reserved-code remediation

- The first leased-gap red held an exact typed P0 root gap after its retry deadline and leased it
  before background P2 promotion completed. Enqueuing a concurrent precise live path then let the
  covering root absorb that path and reported `superseded_count = 1`. The production red reproduced
  the same window with P2 capacity full. The admitted coalescer excludes only an exact
  capacity-deferred gap in `retry_wait` or `leased` from covering and absorption for incoming precise
  live path work. The gap retains its lease and owner; the path receives an independent leasable P0
  row, repeated path evidence coalesces normally, and no ordinary root row gains this protection.
- The reserved-code red passed `live_gap_p2_capacity_deferred` through generic retry and then
  observed lease-expiry attempt refund plus nonterminal retry. Separate forged controls showed the
  old code-only exemption for wrong scope, recovery-claimed, and wrong-lane rows. Generic retry now
  rejects the complete `live_gap_p2_capacity_` namespace before mutation. Refund, maximum-attempt
  exemption, exhausted metrics, and wake-up all require the exact typed lane, origin, intent, scope,
  path, status, failure code, and absent-claim relation. True non-capacity failures still terminate
  at the unchanged budget, and the typed capacity path retains restart, wake, and fairness behavior.
- Focused ordinary-user evidence passes the new leased/unleased/duplicate coalescing controls, the
  generic current/future reserved-namespace rejection with transaction invariance, wrong-shape
  expiry and terminal controls, the real-error terminal control, the phase-29 retained-owner five,
  phase-27 retry-budget survival, and phase-26 native `need_rescan` visibility. The complete queue,
  production, persistent-journal, migration, and scan groups pass 82/82, 84/84, 42/42, 65/65, and
  36/36 runnable tests respectively, with two expected scan ignores. Development warnings-denied
  Clippy, 56/56 focused Flutter tests, bridge hash `941711727`, and `quality_lint.ps1` pass.
- The ordinary-user internal-disposable runner passes 19/19. The first canonical Daily attempt had
  one existing lifecycle stop-deadline timeout after 939 Rust passes; the exact test immediately
  passed 1/1, and an unchanged canonical rerun passed 940 runnable Rust tests with 17 expected
  ignores, broker integration 3/3, every Flutter test, Windows scan 2/2, and native accessibility
  2/2. No timeout, concurrency, or assertion threshold was changed.
- Release-profile warnings-denied Clippy passes. After deleting only the verified repository
  `build/windows/x64` output, a fresh unsigned x64 Release builds in 119.75 seconds. The application,
  packaged and Cargokit Rust DLLs, and broker are PE `0x8664` and `NotSigned`; broker, rlib, and DLL
  graphs contain 83/82/82 present current dependencies; packaged and Cargokit DLL SHA-256 values
  match. Twenty executable test-seam strings have zero ASCII or UTF-16LE matches across six Release
  artifacts, and all ten Flutter assets have zero matches across twenty-four policy/test tokens.

This phase uses no retained library, real Cloud Files, SCM, named-pipe/FSCTL journal, signed input,
or source-media mutation, and schedules no automatic full scan. Those external gates and the final
independent full-range audit remain open. R2c-R remains not accepted and R2c-O remains active.

## Acceptance boundary

This is not R2c-R acceptance and does not accept the accumulated R2c milestone. It is specifically
a non-external controlled local reliability checkpoint.

Still open:

- an immutable externally signed Application and broker bundle with exact publisher admission;
- elevated installed-service and SCM lifecycle evidence;
- real broker named-pipe and FSCTL journal evidence on Windows 11 x64;
- separately authorized, serial, read-only retained-library runs for both logical roots;
- retained-root source immutability and real Cloud Files no-hydration acceptance;
- the final independent full-range R2c architecture, code, security, migration, performance, and
  source-safety audit after every applicable external gate is available.

R2c-O remains the active acceptance slice. R2c-P and R2c-Q remain implementation checkpoints, even
though the fourteenth R2c-Q independent re-audit closed its implementation/audit checkpoint with
zero findings.
