# Quality gates

Status: active repository workflow

Cedarflake Ame separates deterministic development feedback from expensive or authorization-bound
acceptance work. A passing lower gate never claims that a higher gate ran.

| Gate | Entry point | Included evidence | When to run |
| --- | --- | --- | --- |
| Hosted CI | `.github/workflows/quality_ci.yml` | Parallel isolated Daily components, three synthetic workloads, unsigned x64 Release build, and committed revision-range whitespace validation | Push to `main`, pull request, merge queue, or manual run |
| Hosted synthetic workloads | `./tool/performance_run_synthetic.ps1` | Exact JPEG, 10,000-image scan, and million-record parser cases, bounded execution and diagnostic evidence | Mandatory hosted CI; explicit serial workstation invocation |
| Unsigned Release build | `./tool/quality_verify_unsigned_windows.ps1` | Fresh x64 application/broker, payload and dependency identity, isolated Release bridge smoke; no catalog or signing | Mandatory hosted CI; explicit local packaging verification |
| Daily | `./tool/quality_verify_daily.ps1` | Format, lint, Rust and Flutter tests, controlled Windows scan and native accessibility integrations, bridge hash plus asynchronous API/wire-mode contracts, tracked diff whitespace | Every material change |
| Performance | `./tool/performance_benchmark_synthetic_library.ps1` | 10,000 temporary images, cold and warm scans, pause and resume, bounded memory | Scan pipeline, persistence, concurrency, or performance changes |
| Retained Profile | `./tool/performance_profile_retained_gallery.ps1` | Frozen-interaction Profile frame, memory, garbage-collection, query, publication, and retained-detail evidence; no source preview materialization | Guarded R2b gallery adaptations on the retained catalog |
| Preview performance acceptance | `./tool/acceptance_run_preview_performance.ps1` | Cold/warm bucket latency, cache growth, reuse, reclamation, regeneration, bounded memory, and sampled source integrity | Explicitly authorized R2b preview closeout only |
| Real library | `./tool/acceptance_run_read_only_library.ps1` and `./tool/acceptance_verify_read_only_catalog.ps1` | Explicitly authorized source scan, source integrity sampling, retained multi-root catalog validation | Only with current authorization and explicit paths |
| R2c reliability | `./tool/acceptance_run_r2c_reliability.ps1` | Real watcher latency and coalescing on a disposable root; isolated retained-catalog catch-up, queue, storage, memory, placeholder, metadata, and source-byte evidence | Explicitly authorized R2c-H closeout only |
| R2c replacement reliability | `./tool/acceptance_run_r2c_replacement_reliability.ps1` | Disposable watcher operation latency, storm and restart recovery; isolated retained-catalog metadata inventory, cached gallery, source metadata, placeholder, full-scan, storage, and memory evidence | Explicitly authorized R2c-M closeout only |
| R2c-R controlled local reliability | `./tool/acceptance_run_r2c_change_driven_reliability.ps1` | Exactly counted production-path live, durable P1, overflow/P2, million-record, no-change, priority, reset/trim, reconnect, cancellation, replacement, placeholder, path, and multi-root evidence using only fixture-owned disposable roots | Manual non-external Windows 11 x64 checkpoint; never retained-library acceptance |
| Journal broker integration | `./tool/integration_test_windows_journal_broker.ps1` | Exactly counted disposable local NTFS tests for proof framing, two listeners, global eight-worker bounds, active-stop drain, listener readiness, case semantics, root replacement, reparse rejection, root-external filtering, bounded progress through unrelated-volume storms, cancellation, must-close backpressure, identity-probe deadline, restart, and installed-versus-portable activation | Broker transport, service, parser, or lifecycle changes |
| Journal broker installer guardrails | `./tool/release_test_journal_broker_installer_guardrails.ps1` | Windows 11 x64, signing publisher, fixed destination, service account/SID/privileges/DACL, source/catalog isolation, no journal mutation, the existing eight install/repair/upgrade crash cases, and transaction-level marker-first uninstall/dead-owner recovery across every operation and final cleanup without changing SCM | Broker installer lifecycle changes |
| Installed journal broker acceptance guardrails | `./tool/acceptance_test_windows_journal_broker_guardrails.ps1` | Explicit token, fresh temporary workspace, externally pre-signed protected bundle/publisher, ValidationOnly non-mutation, protected result channel, limited-client replacement rejection, no-follow cleanup, and no certificate, signing, trust-store, or automatic-elevation path | Installed broker acceptance harness changes |
| Release | `./tool/release_verify_candidate.ps1` | Immutable signed x64 application bundle and broker admission, Daily gate, Windows Release and bridge smoke, synthetic performance gate, optional retained real-library validation | Before a release candidate |
| Portable artifact | `./tool/release_package_portable_windows.ps1` | Versioned Windows x64 ZIP containing the same signed application and broker artifact, followed by archive-structure and extracted signature/publisher/protocol verification; no service installation | After a release build passes |
| Portable publication guardrails | `./tool/release_test_portable_publication.ps1` | Exact inline workflow decision: missing attachment upload, same-digest skip, different or missing digest rejection, pagination, and fail-closed lookup errors; no network or publication | Release publication changes; included in lint |

## Hosted workflow lifecycle

The hosted Windows gate is implemented once in
`.github/workflows/quality_gate_windows.yml` and called by category-owned trigger workflows:

- `quality_ci.yml` runs Daily plus the synthetic and unsigned-build gates for pushes to `main`, pull
  requests targeting `main`, merge queue checks, and manual dispatches;
- `release_candidate_windows.yml` is manually dispatched from protected `main` for one existing
  strict-SemVer tag, then publishes the verified portable ZIP;
- `release_verify_published.yml` downloads the exact published ZIP and independently verifies its
  identity and archive structure.

The push gate intentionally targets `main` rather than every feature branch. Feature work is
validated by the pull-request event, avoiding duplicate push and pull-request runs for the same
update. Merge queues receive their own `merge_group` check so the combined merge result cannot
inherit a stale pull-request status.

Every hosted job uses `windows-2025`. Daily components and the unsigned-build and signed-verifier
release jobs install Flutter 3.44.9 and use the repository Rust toolchain; the protected signer and
published-artifact verifier do not. Static Daily, the unsigned-build job, and the signed-verifier job
download actionlint 1.7.12, verify the official Windows x64 SHA-256, and validate every workflow through
`./tool/quality_lint_workflows.ps1` before restoring project dependencies. External actions are
pinned to complete commit SHAs, checkout credentials are not persisted, and ordinary workflow jobs
have read-only repository contents permission. Only the final portable-publication job receives
`contents: write`; it checks out no repository content and starts only after the read-only candidate
and packaging gates succeed. Pull-request jobs receive no release secrets or write permission. The
publisher preserves existing same-name attachments only when GitHub's uploaded-asset SHA-256 digest
matches the verified ZIP; a missing attachment is uploaded without overwrite, and an unprovable
identity or failed lookup stops publication. The workstation daily gate does not download actionlint
implicitly; contributors may run the same script
with an explicitly supplied executable.

`windows-2025` is a hosted Windows Server build surface. It does not replace the explicitly
authorized Windows 11 x64 client acceptance gates named below.

The release-candidate chain begins with a no-checkout admission job that requires
`workflow_dispatch`, `refs/heads/main`, `github.ref_protected`, and equality between the event SHA
and the workflow-definition SHA. The reusable gate receives that trusted workflow commit. Its
unsigned job checks out only the full `refs/tags/<tag>` ref, proves that the tag exists, that `HEAD`
equals `tag^{commit}`, and that the source is an ancestor of the trusted workflow commit, then builds
the application and journal broker without signing credentials. It uploads a unique, immutable
artifact whose manifest records the source commit and enumerates every payload file by safe relative
path, length, and SHA-256; its signing allowlist is exactly `cedarflake_ame.exe` and
`cedarflake_ame_journal_broker.exe`.

The next job is bound to the `windows-production-signing` GitHub Environment, which must require a
reviewer and restrict deployments to protected `main`. Its PFX and password exist only as
Environment secrets. The signer does not check out or execute repository code, has no
`GITHUB_TOKEN` permission, and does not run Flutter or Cargo; it only downloads and strictly verifies
the artifact, parses the PFX as ephemeral in-process key material, admits one active private-key
certificate with the exact publisher and code-signing usage, signs the two fixed binaries, rewrites
their manifest evidence without changing the source commit, clears credential and certificate
material in `finally`, and uploads a new immutable artifact. It never writes the PFX to disk or
imports it into a certificate store. A third, credential-free job re-proves source, tag, and trusted
workflow ancestry on another runner, downloads the signed artifact, verifies its complete manifest
and both signatures, and only then executes the repository release gate.

A separate read-only packager checks out the immutable source SHA, binds the signed manifest to that
SHA and the still-current tag, reverifies the bundle, and uploads the exact ZIP plus its SHA-256 as
another immutable artifact. The sole `contents: write` publisher downloads only that ZIP, checks out
no repository and executes no repository script. Immediately before GitHub Release mutation it
peels the current tag through the GitHub API, requires the resolved commit to equal the signed
source, and rechecks the ZIP hash. Repository configuration must additionally prevent update or
deletion of admitted version tags; no workflow can make a tag lookup and release mutation atomic.

For hosted Daily runs, the shared gate fans out four isolated `windows-2025` jobs: static and Rust
verification, Flutter widget tests, the controlled Windows scan integration, and the native Windows
accessibility integration. It also calls `quality_gate_synthetic_windows.yml` for three separately
isolated workload jobs and `quality_gate_unsigned_windows.yml` for optimized application/broker
verification. A stable `Windows Gate` aggregation job succeeds only when every Daily, synthetic,
and unsigned-build job succeeds; skipped or cancelled requirements fail. Once repository branch
protection is provisioned, its required status context
must be the exact GitHub check name `Windows Gate / Windows Gate`. Matrix fail-fast is disabled so
one failure does not hide results from the other components. Jobs do not exchange
compiled artifacts or build directories; only dependency caches may be reused. This reduces
wall-clock latency at the cost of additional hosted runner minutes. Release candidate jobs remain
ordered by immutable artifacts because build, protected signing, credential-free verification, and
publication form distinct trust boundaries.

Before any release checkout, inline validation accepts only the repository's strict `v`-prefixed
SemVer 2.0 grammar, including the prohibition on leading zeroes in numeric prerelease identifiers.
Tag checkouts name the full `refs/tags/<tag>` namespace; later jobs check out the already proven
source SHA. Every such job proves exact tag existence and equality among the tag, source, and
`HEAD`; a branch with the same name, an ambiguous revision expression, a moved tag, or a mismatched
checkout therefore fails closed. `./tool/release_validate_version.ps1` then checks that this tag, the `pubspec.yaml`
application version without its build suffix, and the `rust/Cargo.toml` package version are
identical. The daily lint gate runs a passing prerelease fixture and representative rejection cases
for these version, ref-identity, and job-isolation contracts. It also builds a controlled portable
fixture and proves that missing runtime files and unsafe archive paths are rejected.

The published-release workflow resolves the version contract from the published tag and downloads
only `Cedarflake-Ame-<tag>-windows-x64-portable.zip`. It first validates bounded safe archive paths,
then extracts into fresh repository scratch storage solely to revalidate the application and broker
Authenticode signatures, exact publisher, x64 machine, and broker protocol before confirmed cleanup.
Provenance and independent checksums remain deferred supply-chain work rather than claims of the
current gate.

GitHub-hosted workflows never receive real-library paths or authorization tokens and never run the
real-library gate. Ordinary PR CI runs the three synthetic cases without release permission; retained
real-library verification remains a separately authorized workstation action. Protected release jobs
may still show as skipped in a PR: the independent unsigned quality job supplies compilation evidence
without relaxing their protected-main admission.

### Hosted coverage boundary

`performance_run_synthetic.ps1 -Case jpeg|scan|usn|all` accepts only the fixed workload catalog.
It builds the locked optimized Rust test artifact, discovers and executes one exact test per case,
and rejects missing tests, ignored results, nonzero exits, malformed evidence, and exceeded deadlines
or workload working-set ceilings. JPEG output does not impose a speedup threshold. Scan timing/storage
and parser boundedness assertions remain unchanged. Fresh logs and JSON under
`.dart_tool/performance_synthetic/` are retained as short-lived CI artifacts, including on failure.
Build primary-process memory is not complete compiler-tree memory evidence.

`performance_test_synthetic_protocol.ps1` verifies the exact workload and output contract without
creating fixtures, starting child processes, or initializing native helpers. The complete
`performance_test_synthetic_guardrails.ps1` invokes it before its process and resource guardrails.
The scan marker requires both published-catalog and resumed-first-import catalog byte counts;
missing, duplicate, malformed, or unexpected fields fail rather than being silently ignored.

The USN case uses generated bytes and an injected backend; it does not access a volume journal or
stand in for the complete Windows 11 R2c-R runner. Old H/M controlled scenarios remain historical and
need ADR 0024 adaptation, not a blanket ignored-test switch. The three R2c-R production scenarios
retain their ordinary-user Windows 11 x64 entrypoint. No matching runner is assumed to exist, and
Server jobs never report client acceptance. Real-root and signed/installed-service gates remain
separate. Ordinary parent-owned WAL and small M metadata child tests remain part of Daily.

The unsigned gate has no credential-bearing Environment or publication path. It verifies a fresh
application, broker, and Rust payload and runs only the isolated native-channel/Release-DLL smoke;
it never starts the existing retained-catalog smoke. See
[ADR 0026](../architecture/0026-hosted-synthetic-and-unsigned-build-gates.md) for these boundaries.

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

Process cleanup is never based on all newly observed Dart or Flutter processes. The Windows
integration gate holds the repository mutex, snapshots pre-existing runner process IDs, and limits
cleanup to later processes from the exact Debug runner path. Other commands may stop only a process
that their own process tree or an equivalently isolated identity proves they own.

The Windows scan integration fixture contains only controlled temporary files under `build`. It
opens the real directory picker through automation, exercises scanning and preview publication,
verifies that source bytes remain unchanged, and removes its temporary storage.

The Windows accessibility integration runs a synthetic 1,200-item virtual gallery and the populated
application shell in the native Windows runner with semantics enabled. It performs distant gallery
jumps, repeatedly opens and closes on-demand photo menus, operates stable toolbar and source menus,
changes the timeline and viewer Sliders, and returns from the viewer twice. The script retains the
complete engine output under `build`, rejects any `Failed to update ui::AXTree` record even when the
Dart assertions pass, and therefore exercises the platform `AccessibilityBridge` behavior that
widget-test semantics models cannot reproduce. Flutter 3.44.9 exposes Windows device integration
tests through its supported Debug test path; this canary does not claim Profile-mode coverage.

The native probe loads only the fixed, strong-named Windows UI Automation assemblies through
`integration_windows_accessibility_assemblies.ps1`, validates their identity and required types,
and reports type/client loading separately from native traversal. It does not initialize the
`Add-Type` command or fall back to a same-named DLL in the working directory. The assembly guard is
part of `integration_test_windows_accessibility_guardrails.ps1`. The existing eight-second parent
deadline, MTA client, owned process tree, and native positive/negative assertions remain unchanged;
workstation PowerShell evidence does not replace execution in the hosted PowerShell runtime.

## Performance gate

```powershell
./tool/performance_benchmark_synthetic_library.ps1
```

The default peak working-set ceiling is 512 MiB. Override it only when an accepted performance
decision defines a different budget:

```powershell
./tool/performance_benchmark_synthetic_library.ps1 -MaxPeakWorkingSetBytes 536870912
```

For R2b gallery adaptations, compare Profile with Profile against the frozen interaction revision:

```powershell
./tool/performance_profile_retained_gallery.ps1 -Iterations 80
```

This supplementary run uses the retained catalog and derived preview storage while its test
previewer rejects source-media materialization. It records evidence under `build/performance`,
does not replace the synthetic gate, and does not authorize a source scan, cloud-placeholder
hydration, or a real-library acceptance run.

The retained Profile is not a preview-throughput gate. It cannot measure source decode or preview
materialization latency, bucket reuse, materialized cache growth, reclamation duration, or
regeneration churn. Run the dedicated gate only with current authorization for `local-primary`:

```powershell
./tool/acceptance_run_preview_performance.ps1 `
  -RootPath "<authorized local-primary root>" `
  -SourceCatalogPath "<current catalog path>" `
  -StorageRoot "<pre-created empty storage outside every source tree>" `
  -AuthorizationToken "<current preview-performance authorization token>"
```

The entrypoint creates an online SQLite backup in isolated derived storage, resets previews only in
that backup, and samples at most 512 catalogued, locally readable source items. It has explicit item,
time, memory, source-file, and cache limits; rejects cloud roots and storage overlap; never performs
a root scan; exercises 128/256/512 cold and compatible warm bucket requests; naturally fills the
minimum 64 MiB cache toward its pressure boundary; then records reclamation duration, regeneration,
immediate boundary churn, and source-state verification. Its guardrail contract is checked by
`./tool/acceptance_test_preview_performance_guardrails.ps1` and the static quality gate. A passing
tool implementation is not acceptance evidence until this authorization-bound workload itself has
run successfully.

## Real-library gate

Run the guarded scan command separately for every currently authorized root. The exact command and
safety conditions are documented in [read-only-real-library.md](./read-only-real-library.md).
After the expected roots have been published into one retained catalog, validate the complete
catalog through the production loading API:

```powershell
./tool/acceptance_verify_read_only_catalog.ps1 `
  -StorageRoot "<retained acceptance storage>" `
  -RootA "<first approved root>" `
  -RootB "<second approved root>" `
  -AuthorizationToken "<current authorization token>"
```

The presence of a retained catalog or an old token does not authorize a new source scan.

## R2c reliability gate

Run the R2c-H gate only after the synthetic performance gate passes and current authorization names
both logical roots, the retained catalog, and new empty derived storage:

```powershell
./tool/acceptance_run_r2c_reliability.ps1 `
  -LocalRoot "<authorized local-primary root>" `
  -CloudRoot "<authorized cloud-primary root>" `
  -SourceCatalogPath "<retained catalog path>" `
  -StorageRoot "<new empty storage outside every source tree>" `
  -AuthorizationToken "<current R2c-H authorization token>" `
  -AcknowledgeCloudReadOnly
```

The first half uses temporary files only and measures the production Windows observer, event-to-
catalog P50/P95, idle polling, event-storm coalescing, durable queue recovery, database growth, and
bounded shutdown. The second half opens the supplied catalog read-only, creates an online SQLite
backup in isolated storage, and runs only downtime-catch-up discovery against the authorized roots.
It does not publish the resulting authoritative work. Before and after that operation it compares a
bounded metadata snapshot of every source entry, placeholder attributes, and deterministic hashes
of locally available `local-primary` samples. Reparse directories are not followed and files marked
offline or recall-on-access are never opened.

The wrapper enforces a time limit, a peak job-memory limit, physically resolved non-overlapping
paths, pre-created empty isolated storage, an exact token, and an explicit cloud read-only
acknowledgement. Its non-accessing guardrails exercise junction rejection and process-job ownership
through `./tool/acceptance_test_r2c_reliability_guardrails.ps1`. A passing tool implementation or
`-ValidationOnly` result is not real-library evidence; the authorization-bound workload must finish
successfully. See
[r2c-h-large-library-reliability.md](./r2c-h-large-library-reliability.md) for the accepted metrics and
remaining platform limitations.

## R2c replacement reliability gate

Run the R2c-M gate only with current authorization naming both logical roots, the retained catalog,
and new empty derived storage:

```powershell
./tool/acceptance_run_r2c_replacement_reliability.ps1 `
  -LocalRoot "<authorized local-primary root>" `
  -CloudRoot "<authorized cloud-primary root>" `
  -SourceCatalogPath "<retained catalog path>" `
  -StorageRoot "<new empty storage outside every source tree>" `
  -AuthorizationToken "<current R2c-M authorization token>" `
  -AcknowledgeCloudReadOnly
```

The disposable phase measures production create, modify, rename, cross-directory move, same-path
replacement, and delete visibility, requiring event-to-visible P95 no greater than one second. It
also records storm coalescing, restart continuity, bounded shutdown, catalog growth, and unchanged
full-scan rows. The retained phase opens the supplied catalog read-only, creates an online SQLite
backup in isolated storage, proves a cached gallery page is available before continuity work, and
runs the production metadata inventory directly for each authorized root. Each root must complete
within 45 seconds before any complete source walk. This phase enumerates metadata without invoking
media inspection, preview, signature, or hashing paths. A separate repeated-inventory pass compares
complete Windows directory-entry metadata and placeholder attributes before and after; the report
keeps that safety evidence distinct from the cold timing and verifies that no full-scan row was
created.

The wrapper retains the R2c-H Job Object, deadline, memory, physical path, empty storage, token, and
cloud acknowledgement controls. Its non-accessing boundary is exercised through
`./tool/acceptance_test_r2c_replacement_guardrails.ps1`. `-ValidationOnly`, disposable evidence, and
the small retained-catalog fixture do not substitute for an authorized target-scale run. See
[r2c-m-replacement-reliability.md](./r2c-m-replacement-reliability.md) for the recorded evidence and
remaining authorization boundary.

## R2c-R controlled local reliability gate

Run the non-external Windows 11 x64 checkpoint as an ordinary, non-administrator user:

```powershell
./tool/acceptance_run_r2c_change_driven_reliability.ps1
```

This runner accepts no source-root or catalog arguments and does not trust caller `PUBLIC`, `TEMP`,
or `TMP`. It admits only an ordinary Windows 11 x64 client workstation by cross-checking trusted
registry, native version API, product type, and SKU evidence. Loading its common module performs no
dynamic compilation. Explicit initialization first emits only the minimal Win32/NT declarations in
memory, then performs a read-only logical and physical binding of the repository `tool` path. Every
existing component is opened relative to a held parent with no-follow semantics, and any reparse,
volume transition, or terminal identity mismatch is rejected before the high-entropy bootstrap is
created with `NtCreateFile` relative to the final held physical parent. Only then does it rebind
`TEMP` and `TMP` for compilation. It revalidates the held identities after compilation, restores both
variables, and deletes only an empty bootstrap reopened relative to the held parent with the expected
identity. Compiler failure, unknown children, reparse state, or replacement retains an identifiable
bootstrap without traversal. The runner applies the same pre-write binding to both the logical
`SHGetKnownFolderPath` LocalApplicationData path and its handle-resolved physical filter path. It
holds every component and requires terminal volume/file identity equality before creating one high-
entropy disposable root relative to the final physical handle. The component and root handles remain
open without delete sharing for the complete runner lifecycle until cleanup begins; no `Temp` or
predictable owner directory is trusted or created. Parent-relative opens honor the live per-
directory Windows case-sensitivity flag; case-folded path text is never used as identity evidence.
Every fixture source, catalog, report, and per-case log remains below that
root. The runner validates the exact unique 19-case / 15-normal / 4-ignored matrix against current
attributed Rust functions, including report-binding tamper rejection, runs each fully qualified test
one at a time with `--exact`, and requires the final outer Rust result to report exactly one pass.
The four live/closed/overflow/million-record manual cases are explicitly ignored outside this
wrapper.

Windows share compatibility does not allow the runtime replacement-blocking root handle to request
`DELETE` while the production publication namespace guard also opens the root without delete
sharing. Cleanup therefore starts only after every owned child has exited: it releases the root
blocker, reopens the unpredictable child name relative to the still-held parent with delete access,
rechecks the exact file identity and physical path, then deletes that exact empty handle. This is
not
claimed as an atomic cleanup transition. A same-user racer can make cleanup fail and leave the owned
root for investigation, but cannot redirect deletion to a replacement identity; mismatch fails
closed. Failure evidence records the owned leaf and expected file-identity token because the
original
path may now name an attacker replacement. Cleanup never recurses from the original path after a
native reopen, identity, path, reparse, or emptiness failure. The high-entropy name, held parent
chain, and post-open identity check bound
that residual cleanup-only namespace race.

Each Cargo process is created suspended and assigned to a private kill-on-close Job Object before
it runs. A ten-minute per-case and one-hour total parent wall-clock deadline terminate only the
owned process tree, and `finally` restores environment/location/lock state and removes reports and
the exact nonce fixture. Evidence covers P0 latency and non-O(N) affected-path reads against a
4,096-entry unrelated baseline, durable P1 restart with nonce/PID/phase-bound worker reports, typed
overflow and bounded P2 ownership, a production-maximum-buffer million-record parser stream, 100
complete production observer starts with one metadata-only availability probe per actual poll and
zero source enumeration, target-scale P1/P2 priority, reset/trim, reconnect,
cancellation, replacement, retained-session same-volume multi-root isolation, placeholder
behavior, and Chinese/long paths.

The non-accessing guardrail is:

```powershell
./tool/acceptance_test_r2c_change_driven_reliability_guardrails.ps1
```

It rejects Windows 10/Server, unresolved SKU or inconsistent build evidence, non-Windows, non-x64,
administrator, caller-path, worker-environment, matrix drift, and zero-test inputs before Cargo. A
fresh PowerShell child proves the common module does not preload native types, hostile `TEMP`/`TMP`
sentinels receive zero writes, forced compiler failure leaves zero safely deletable residue, active
bootstrap replacement is blocked, and pre-created terminal plus production KnownFolder intermediate
junctions are rejected before a write without changing their sentinels. Workspace-only held fixtures
release the runtime blocker, move the original root, and put both ordinary and junction replacements
at the old name. After each first validation the replacement is swapped again; the stale expected
identity must retain the unknown object. Cleanup can proceed only through a held-parent no-follow
reopen with the newly captured fixture identity. Sentinel and process-output files use held delete-
on-close streams. One default-deny PowerShell AST source/call-closure audit covers every top-level
and function scope in the common module, runner, guardrail, all recursively reached local helpers,
and every exact dot-source dependency. A dot-source must resolve to a no-follow, same-volume,
identity-held repository-tool script whose exact content is audited; variable or otherwise
unrecoverable loading fails. Path text is never deduplication authority: the ordinal path map is only
an index, every request opens and binds the native snapshot first, and only its held volume/file ID
may return an existing source. Duplicate snapshots close before return, unique snapshots transfer one
owner, and exceptional pre-transfer paths close locally. Source-count admission remains before the
open. A real case-sensitive NTFS control requires `Safe.ps1` and `safe.ps1` to open as independent
identities and rejects a forbidden lower-case source; a host that cannot enable the directory flag
reports an explicit controlled skip. Ordinary duplicate, case-disabled spelling, and exceptional-
close controls also run. The audit fails closed at 8 sources, dot-source depth 8, 256 KiB per source,
512 KiB total source bytes, 32,768 AST nodes, 128 functions, 512 scopes, and a 512-entry queue. AST
nodes are visited without materializing a collection, the root counts once, and the limit-plus-one
visit throws immediately while recording its attempted high-water. Independent empty and literal
shapes exercise limit-minus-one, limit, and limit-plus-one without reusing the production counter.
Every checked-add overflow or exceeded limit identifies its exact budget key, limit, and actual
value. Trusted cmdlets use an explicit leaf/module allowlist, and every unresolved command,
call operator, unknown helper, external executable, alias definition, dynamic script construction,
reflection invocation, or path-addressed mutation fails in every scope. The only process-creation
boundary is an independently digest-locked wrapper and native Job Object suffix. Before launch it
validates one exact Cargo matrix case or recovers and audits the final actual PowerShell `Command`,
canonical UTF-16LE `EncodedCommand`, or native no-follow `File` snapshot and arguments; nested or
unrecoverable execution fails closed. A file-backed child retains the audited terminal and every
parent handle without delete sharing, denies terminal write sharing, and reopens each wrapper and
payload source relative to its held parent to revalidate volume/file identity immediately before
native process transfer. Terminal reparse, post-audit terminal replacement, post-audit parent
replacement, or an exceptional partial-open fails closed and releases only the handles opened by
that attempt. Forty-five adversarial source/runtime fixtures cover module qualification, remove
aliases, alias definition, variable call/dot-source in every scope, `IEX`, script-block and remote
invocation, nested commands, PowerShell/pwsh/cmd `Start-Process`, `System.IO`, `cmd del`/`rmdir`,
`robocopy /MIR`, reflection, unknown helpers, and the exact `Add-Type`, move, and create boundaries
while all real child payloads remain green. Injected faults require the volume handle to close
before retained transfer and the compiler bootstrap owner to close before later initialization,
both in the parent process before controlled native types exist. The failure itself must report the
owned leaf and expected identity. The owned-name contract admits only the exact fixed prefix and two
32-character lowercase-hex fields, plus the exact internal moved-fixture form. Malicious NT leaves,
including ADS, control, separator, dot, trailing dot/space, arbitrary ASCII suffix, uppercase hex,
and non-ASCII syntax, are rejected. The guardrail also proves a blocked owned parent/child tree is
killed without terminating an unrelated process, timeout storage is cleaned, and it actually runs
the report-tamper Rust test with `--exact` and requires one pass, zero failures, zero ignored, and
zero measured tests. The Rust suite additionally locks the availability classifier to opaque
metadata evidence. Its exact `syn` 2.0.119/`quote` 1.0.47 source contract begins at the three
production owners and requires 17 named cfg-qualified function items plus 16 named support items to
match their token digest, local-callee closure, and call/receiver provenance. Any missing, changed,
ambiguous, newly reached, or newly referenced item fails with its readable key and summary. A full-
crate check rejects `Drop` and overloaded side-effect operators only for the six closure-reachable
evidence types. The protected closure contains no macro expression, admits only its locked
attributes and built-in derives, and rejects relevant local, parent, and imported macro-resolution
changes. The same structured contract locks the exact crate, adapters, local, domain, and metadata-
domain module declaration topology, including visibility, cfg attributes, inline/external shape, and
the one exact test-only item macro. File attributes, source-changing `cfg_attr`/`path`, arbitrary or
procedural attributes, `include!` items, extern-crate aliases, and nested/generated modules fail by
source key; the test and non-test production cfgs therefore load the same availability implementation.
Fixtures cover callee and receiver shadowing, unchanged-owner helper I/O, `Drop`, operator overload,
static/lazy side effects, aliases, same-module and parent macros, renamed macro imports, every module-
loading mutation, unknown expressions, and enumeration while the current production AST remains
green. The runner
restores environment state. `quality_lint.ps1` and Daily run only
this lightweight guardrail; they do not run the manual reliability wrapper. Controlled local
evidence does not substitute for an externally signed Release bundle, elevated SCM/service
lifecycle, real broker FSCTL, or separately authorized retained-root immutability and no-hydration
acceptance. See
[r2c-r-change-driven-reliability.md](./r2c-r-change-driven-reliability.md).

The R2c-R SQLite protocol-read remediation adds no new public command or acceptance case. Its
production contract is exercised through the existing exact watcher-overflow case and ordinary
19-case runner. All six idempotent public catalog read families and the persistent acceptance
observer use the adapter-owned `SqliteCatalogReadExecutor`. Only rusqlite
`FileLockingProtocolFailed` is retried, on a new connection with read-only schema/identity
validation; application-owned migration and every write path remain outside the owner. Production
allows five total attempts within a 100 ms monotonic retry-admission window with 1/2/4/8 ms bounded
backoff. The window is checked only after a SQLite call completes and does not claim to interrupt an
active call. Read connections use a 100 ms SQLite busy timeout shared with that admission window;
write connections retain their five-second timeout. Busy and Locked are terminal to the protocol-
retry owner. Exhaustion must retain operation, attempts, actual elapsed time, and final structured
cause without an absolute catalog root.

The ninth-remediation focused gate was the 13-test SQLite read-retry module plus the real concurrent WAL writer/public
timeline regression. Red evidence, the eighth-remediation first failure, and the ninth-review
four-of-five reproduction remain recorded in the owning acceptance document. Closeout additionally
requires twenty consecutive fresh-nonce exact watcher-overflow passes and one complete ordinary-user
runner; a failed sample is retained and resets the conclusion rather than being retried until green.
The completed sequence spans 307-327 ms P95 and 11.463-11.872 seconds convergence; every operation
used one attempt and no protocol retry. Complete Daily, all-target/all-feature development and
Release checks, warnings-denied Clippy, and `quality_lint.ps1` pass. Because production Rust changed,
fresh unsigned x64 app/broker, PE machine, Cargokit dependency/hash, and ASCII/UTF-16 test-seam
checks are also required. These unsigned artifacts are not release candidates. Signed Release and
portable gates must continue to fail closed when the externally signed Application, broker, or exact
publisher is absent.

The tenth-remediation focused gate expands the SQLite read-retry module to 32 tests. It structurally
closes the crate-visible API to named reads, verifies raw typed protocol classification and forged-
code rejection, locks sanitized structured FRB serialization, and covers held Windows catalog
identity across delete, replacement, ABA, reparse, mismatch, success, failure, and backoff. Its
manual clock runs the exact production five-attempt/100 ms/1-2-4-8 ms policy at open, validation,
and query without using a wall-clock fixture. The migration group, current-schema no-migrate, and
256-read public WAL regression remain required. The R2c-R observer must use named typed reads for
gap, inventory, authority, location, and completed-P1 polling; high-frequency raw reopen or `expect`
is a gate failure. Canonical bridge generation, Rust/Dart hash compatibility, three PowerShell
parses, guardrail 19, ValidationOnly 19/15/4, topology/macro/source closure, no-change, tamper, root
replacement, and both disposable Cloud controls are required before the repeated production runner.
These remain non-external gates and cannot accept R2c-R or R2c-O.

The current eleventh-remediation gate is 33/33 read-retry tests. It additionally requires bundled
SQLite to report `ENABLE_SETLK_TIMEOUT`, keeps `busy_timeout = 100 ms` and `query_only` on every
read connection while the writer remains at five seconds, and proves read and write contention
timeouts separately. The exact watcher observer must combine gap, inventory, authority, and bounded
location evidence into one named typed snapshot per observation. Before aggregation, the exact
control failed at 789 typed operations; the admitted repeated sequence is 20/20 with 474-527
operations/attempts, zero protocol retries, maximum attempt one, sample-P95 P95 374 ms, maximum
sample P95 403 ms, convergence P95 12.128 seconds, and maximum convergence 12.941 seconds.

Production and closed-process acceptance waiting derive from one tracked policy byte sequence:
`tool/library_synchronization_poll_interval_ms.txt`. It must contain exactly one positive base-10
integer followed by LF; empty, zero, signs, units, extra whitespace or lines, CRLF, BOM, unsigned
overflow, and values above `9223372036854775` milliseconds fail before output changes. That exact
maximum is `floor(i64::MAX / 1000)` because Dart `Duration` stores signed 64-bit microseconds.
`./tool/quality_generate_library_synchronization_policy.ps1` deterministically writes
`library_synchronization_policy.g.dart`; `-Check` compares exact UTF-8/LF bytes without writing.
`./tool/quality_test_library_synchronization_policy.ps1` proves stale output and the invalid-input
matrix fail closed. `quality_lint.ps1` runs both the guardrail and `-Check` before formatting.

Dart production code imports the generated
`productionLibrarySynchronizationPollInterval`. `RustLibrarySynchronization.production()` takes no
arguments, while dependency injection, alternate clocks, diagnostics controls, retry schedules, and
fast cadence exist only on `@visibleForTesting .testing(...)`; analyzer configuration promotes a
production misuse of that member to an error. A Zone timer behavior test executes a successful
start, intercepts the actual `Timer.periodic` creation, and compares its `Duration` with the generated
constant. A separate behavior test proves an explicitly injected fast testing cadence remains
available. `main.dart` may call only `.production()`.

The common cfg(test) Rust `production_synchronization_cadence` module uses `include_str!` once on the
same tracked text and enforces the identical canonical form and Dart-safe maximum.
`ProductionSynchronizationCadence` is stored by `ProductionSynchronizationTestHarness`. R2c-R
composes it with readiness, crash-ready, and recovery counts without an interval parameter; worker
reports and the parent still require `1/1/0` or `1/0/1`. The value's `Duration` field is private and
neither acceptance exposes a naked interval getter. R2c-M's real helper requires the opaque value
obtained from its harness; R2c-R's counted wrapper invokes the same owned wait method before recording
the phase. A compile-time function signature rejects `Duration` at the R2c-M cadence position. The
former embedded Dart/main sources, handwritten Dart tokenizer and statement cursor, Rust cadence
`syn` visitor, semantic source scan, and mutation matrix remain removed. `syn` remains only for
independent filesystem source-topology tests.

Focused red evidence changes the tracked policy to 875 while leaving generated Dart at 250, mutates
the real periodic timer to 875, attempts `.testing()` from `main.dart`, and misrecords recovery as a
crash-ready wait. The corresponding generation check, timer assertions, fatal analyzer rule, and
Rust invocation-count assertion all fail. Restored focused green evidence includes the generator
drift/invalid matrix, 31 synchronization controller tests, three lifecycle-owner tests, two Rust
policy tests, the report-field contract, and the ordinary-user 19-case lightweight R2c-R guardrail.
Warnings-denied Clippy and the complete lint gate pass. The full ordinary-user runner passes 19/19
with closed-process P50/P95/maximum 566/703/703 ms, 24 availability probes, zero source or inventory
reads, and three bounded content opens; no-change remains exactly 100 starts, 200 polls, and 200
probes without enumeration or publication. Serial Daily passes 900 Rust tests with zero failed and
17 expected ignored, broker 3/3, all Flutter tests, and both controlled Windows integrations 2/2.
A fresh local unsigned Windows x64 Release build proves x64 PE payloads, identical packaged/Cargokit
DLL hashes, no policy source in assets or dependency manifests, and no deleted cadence-source seam
in five Rust Release artifacts. Sandbox variants stop at the expected held-parent reopen denial; the
identical ordinary-user gates pass without weakened checks. These are non-external controls and do
not accept R2c-R or R2c-O.

Phase-24 red controls prove the former numeric and consumer gaps. The generator wrote the first
Dart-unsafe `9223372036854776` value, and Rust reported that same value as an accepted invalid policy.
With tracked policy 875, R2c-M failed with actual 250 ms versus expected 875 ms. Restored controls
prove exact maximum success plus maximum-plus-one/u64-maximum/u64-overflow rejection in PowerShell
normal mode, PowerShell `-Check`, and the one Rust parser. Both the normal R2c-M harness and R2c-R
counted wrapper follow a controlled tracked 875 value before restoration to 250; their permanent
contracts also consume a synthetic exact `875\n` value through the shared object.

The phase-24 follow-up red replaces one real R2c-M wait argument with
`Duration::from_millis(250_u64)`. The former raw semantic scan and accessor-only 875 test both stay
green, demonstrating their gap. After moving the wait loop onto the opaque cadence and deleting the
naked getter, the identical mutation fails compilation with `E0308`: expected
`ProductionSynchronizationCadence`, found `Duration`. A cfg(test)-only shared sleeper capture executes
the real R2c-M helper without wall-clock delay and records exactly one 875 ms sleep request; R2c-R's
same-object wrapper records three 875 ms requests and retains `1/1/1` stage counts in its focused
contract. The capture seam is not compiled into Release.

Focused shared-policy tests, the R2c-M non-accessing guardrail, warnings-denied Clippy, and complete
ordinary-user lint pass. The complete ordinary-user R2c-R runner passes 19/19 with closed-process
P50/P95/maximum 565/578/578 ms for the initial correction and 565/585/585 ms for the follow-up opaque
API, 24 probes, zero source or inventory reads, and three bounded content opens; no-change remains
exactly 100 starts, 200 polls, and 200 probes. The sandbox denial remains
the known held-parent `C0000022`/Win32 5 boundary. Production Dart and Release payloads are unchanged,
so Daily and Release were not rebuilt for phase 24; read-only scans find no phase-24 policy/test seam
in assets or dependency files, and a fresh eight-token opaque-cadence scan finds zero matches in five
retained Release Rust artifacts. No R2c-M retained-root phase ran. These remain non-external controls
and do not accept R2c-R or R2c-O.

Each actual poll still performs one fresh O(1) availability metadata probe: caching, skipped
availability validation, source enumeration, inventory, and media reads are forbidden. The retained
red results were 78 probes in the complete runner and 75/75/77 in exact reproduction. Five corrected
exact runs each report 24 probes; the pre-binding final runner reported closed-process P95 569 ms,
and the fresh bound runner reports 24 probes with P95 581 ms. The separate no-change case continues
to require exactly 200 polls and 200 probes.

The phase-26 live-gap ownership gate adds no public script or acceptance case. Focused Rust tests
must exercise the production coordinator for native `need_rescan`, observer-ingress drop,
offline-to-available recovery, expired-lease restart, continuous journal coverage, uncovered journal
recovery, and P2 capacity rollback. A convergence test passes only when the root is `Synchronized`,
the relevant queue is empty, no automatic full scan was requested, and lane, origin, scope, intent,
lineage, and the persisted consumer all match ADR 0024. Pending journal ownership must exist before
P0 supersession. Uncovered recovery must create the P2 control, allowlisted authority, and lineage
transfer in the same transaction. Capacity failure must retain the original P0 as retryable without
a naked P1 root marker or partial P2 ownership.

Schema v30 migration coverage is mandatory for fresh catalogs, v29 upgrades, repeated current-
schema opening, conflicting destination rows, malformed partial v30 objects, and naked historical
`StartupCatchUp` P1 root gaps both with and without root publication identity. Root-generation and
publication-namespace identity are not event provenance, so every ambiguous naked row records
explicit recovery ownership and fails closed. Migration may not restore such a row to live P0 or
invent journal ranges, recovery authority, or source-media evidence. These focused checks remain
part of normal Rust and Daily verification; the unchanged ordinary-user 19-case R2c-R runner
supplies the controlled production-path regression.

The phase-27 manual-recovery gate adds no public script or R2c-R acceptance case. Focused Rust tests
must prove transactional explicit-claim begin, publish, abandon, crash/reopen recovery, pending-
journal isolation, and fail-closed reopen of malformed explicit, active foreground, and consumed
foreground states. Queue metrics and the production snapshot must report the typed block before a
manual scan, clear it only after successful publication, and restore it after abandon or crash.
Generated-bridge compatibility must remain exact; because the bridge DTO already owns
`recovery_blocked`, no regeneration is justified unless its Rust shape changes. Dart mapping and
Widget tests must preserve the field, show explicit manual-update guidance, expose a keyboard and
screen-reader operable `更新图库` action, and prove generic persistence failures do not expose that
action. Complete Daily, Release, and the 19-case runner may be deferred only to the named accumulated
closeout; a focused pass does not accept R2c-R or R2c-O.

The completed phase-27 capacity gate drives one real P0 live gap through more than
`max_attempts` promotion deferrals while P1/P2 capacity is full. Every deferral must use the typed
capacity state, refund the lease attempt, retain a bounded non-null retry deadline, and survive
restart plus lease expiry. Releasing exactly one matching slot must wake or make the gap ready within
the bounded deadline and allow exact P2 authority/claim acquisition without manual action. A ready
precise P0 event must remain separately leasable while that root gap waits, and a genuine
non-capacity failure must still exhaust at the unchanged limit. This gate does not require a schema
migration because schema v30 already persists the necessary attempt, failure-code, lease, and retry
state.

When multiple P2 authorities overlap for one root, retained inventory-source ownership must be
keyed by immutable change ID. A newer authority must not consume another run's frontier. Focused
tests must cover exact-owner resumption, independent-owner admission, pruning of an authority that
is missing, retired, or run-mismatched, catalog-validation failure before transfer, and restoration
after a deterministic worker-spawn failure.

At least one phase-26 P2 production matrix must use an explicitly published non-empty disposable
source baseline. The matrix then covers both an addition and a deletion through a real P2 consumer
and requires exact new-asset visibility, exact removed-location absence, unchanged control-source
bytes and hashes, `Synchronized`, zero queue/claim/authority residue, and no additional automatic
full-scan run. Metadata inventory and media-open counts must remain inside the ADR 0024 boundary. A
rollback-only publication mutation must demonstrate that the old empty-fixture/queue-only contract
could miss a visibility defect. Direct database fabrication is not acceptance evidence.

Accumulated closeout evidence passes the focused capacity, restart, lease-expiry, fairness,
real-error, ownership-transfer, and visibility cases plus the complete 83-test production module.
The first complete Daily run exposed a test-observation race: after valid P0-to-P2 promotion, the
helper selected the newest P2 `metadata_inventory` row instead of the retained P0
`live_notification` row. The corrected helper selects the exact P0 origin and lane; absence of that
row still times out and fails. The exact case, complete production module, and subsequent concurrent
Daily run pass.

`quality_lint.ps1` passes, the ordinary-user internal-disposable runner passes 19/19, and serial
Daily passes 927 Rust library tests with zero failed and 17 expected ignored, broker integration
3/3, all Flutter tests, and controlled Windows scan plus native accessibility 2/2 each. Generated
bridge hashes match at `941711727`. Release-profile warnings-denied Clippy passes, and a fresh local
unsigned Windows x64 build completes in 58.2 seconds. The application, packaged Rust DLL, Cargokit
Rust DLL, and independent broker are PE `0x8664` and `NotSigned`; the broker, rlib, and DLL dependency
graphs contain 83/82/82 present and current files; packaged and Cargokit DLL SHA-256 hashes match.
Seven ownership/fault test-seam tokens are absent under ASCII and UTF-16 scans across six Release
artifacts, and all ten Flutter assets contain neither the policy sources nor those test seams. This
is non-external unsigned evidence and does not accept R2c-R or R2c-O.

### Phase-31 migration-integrity gate

The phase-31 focused gate keeps schema v30 ownership migration, validation, and crash repair on one
exact contract:

- v29 running and paused foreground scans with a valid authoritative scan ID must preserve that scan
  ID, queue-row provenance, foreground owner, and publication binding through migration, then resume,
  publish, and reopen without an explicit recovery claim;
- truly naked v29 P1 root fallbacks both with and without root publication identity must still become
  conservative `explicit_recovery_required` claims, while existing conflict and malformed partial-v30
  fixtures fail closed and roll back;
- active and consumed `foreground_scan` claims must require an exact associated
  `scan_owner = 'foreground'`; wrong-owner reopen must return the typed live-gap contract error without
  terminating the scan, restoring the claim, changing the queue row, or exposing a configured path;
- current-v30 interrupted foreground cleanup and normal scan cleanup must invoke the same
  transaction-local orphan predicate. Assets owned by either handoff table remain, an asset with no
  location or handoff owner is deleted, no handoff reference dangles, cleanup failure rolls the whole
  recovery transaction back, and reopen retry is idempotent.

Run the six exact regressions first, then the complete SQLite migration and catalog modules and the
scan-library module serially. Scan tests that exercise Windows publication guards must be rerun by the
same ordinary user outside a workspace sandbox when the sandbox returns Win32 5; the denied run is not
green. The focused closeout also requires all-target/all-feature Clippy with warnings denied,
`quality_lint.ps1`, the explicit-recovery Flutter application and presentation files through
`quality_test_flutter.ps1`, exact Rust/Dart bridge-hash equality, formatting, whitespace validation,
and read-only absent-signed-bundle Release admission. It does not run Daily, the Release build, the
complete R2c-R 19-case runner, a retained root, real Cloud Files, SCM, named-pipe/FSCTL, or signing;
those remain accumulated or external closeout boundaries.

### Phase-32 leased capacity-gap and reserved-code gate

The phase-32 queue gate requires a due exact typed capacity-deferred P0 root gap to be exercised in
both unleased and leased states while precise live path work arrives. The gap must retain its lease,
ownership, retry state, and P0 admission protection; the precise path must become independently
leasable and visible before P2 capacity is available. Duplicate precise evidence must coalesce, and
ordinary P0 rows must retain the existing coalescing and fairness rules. The production control must
hold P2 capacity full, pause the real live worker after leasing the gap, publish a real created image
through the independent path row, and prove no automatic full scan occurred.

The failure-code gate treats `live_gap_p2_capacity_` as a reserved namespace. Generic retry must
reject both the current code and a future namespace member with a path-free typed error and an
unchanged lease/queue transaction state. Lease-expiry refund, maximum-attempt exemption,
metrics, and wake-up require the exact typed lane, origin, intent, root/path shape, status, code, and
absent recovery claim. Wrong lane, scope, status, or claim state must not receive an exemption, while
a genuine non-capacity failure must still become terminal at the configured limit. These checks use
schema v30; a new migration is not part of this gate.

Run the exact red/green controls, phase-27 retry-budget and phase-26 visibility tests, phase-29
retained-owner tests, then the complete queue, production, persistent-journal, migration, and scan
groups serially. Closeout additionally requires all-target/all-feature development and Release
Clippy with warnings denied, the 56 focused Flutter application/presentation tests through
`quality_test_flutter.ps1`, exact bridge-hash equality, `quality_lint.ps1`, the ordinary-user 19-case
runner, canonical Daily, a fresh unsigned Windows x64 Release, PE machine and `NotSigned` checks,
dependency freshness, packaged/Cargokit DLL hash equality, Release ASCII/UTF-16LE test-seam scans,
Flutter asset scans, formatting, and tracked-diff whitespace validation. A sandbox denial or a
failed first run is recorded and is not green; any rerun uses the identical ordinary-user command.

The accumulated phase-32 evidence passes 82 queue, 84 production, 42 persistent-journal, 65
migration, and 36 runnable scan tests with two expected ignores; focused Flutter passes 56/56,
bridge hash is `941711727`, and the internal-disposable runner passes 19/19. The final canonical
Daily passes 940 runnable Rust tests with 17 expected ignores, broker integration 3/3, all Flutter
tests, and both controlled Windows integrations 2/2. The fresh unsigned Release builds in 119.75
seconds; four PE images are `0x8664` and `NotSigned`, dependency graphs are current at 83/82/82,
packaged and Cargokit DLL hashes match, six Release artifacts contain none of twenty executable
test-seam strings, and ten Flutter assets contain none of twenty-four policy/test tokens.

This is non-external evidence only. Retained roots, real Cloud Files, SCM/service lifecycle, real
named-pipe/FSCTL journal access, signing, and the final independent full-range audit remain outside
this gate. R2c-R remains not accepted and R2c-O remains active.

## Installed journal broker acceptance

The real SCM and `FSCTL_QUERY_USN_JOURNAL` / `FSCTL_READ_USN_JOURNAL` path is a separate,
authorization-bound Windows 11 x64 acceptance. It never targets `local-primary` or `cloud-primary`.
Obtain V1 and V2 pre-signed acceptance bundles from the protected production signing pipeline,
pre-create one empty workspace directly below the current temporary directory, then launch an
already elevated PowerShell and pass every disposable path explicitly:

```powershell
./tool/acceptance_run_windows_journal_broker.ps1 `
  -PreSignedBundleV1Path "<protected-path>\AmeBrokerAcceptanceBundleV1" `
  -PreSignedBundleV2Path "<protected-path>\AmeBrokerAcceptanceBundleV2" `
  -ExpectedPublisher "<exact production publisher subject>" `
  -AcceptanceWorkspace "<pre-created empty directory below the current temp directory>" `
  -DisposableRoot "<workspace>\root" `
  -ExternalSiblingRoot "<workspace>\external" `
  -AuthorizationToken "CEDARFLAKE_AME_WINDOWS_JOURNAL_BROKER_ACCEPTANCE_V1" `
  -AcknowledgeDisposableSystemChanges
```

Both bundles must already contain `Application\cedarflake_ame.exe` and
`Broker\cedarflake_ame_journal_broker.exe`, all signed by the exact expected publisher, and must be
rooted in a physical tree whose owner and ACL prevent an untrusted user from replacing any ancestor
or payload. The broker SHA-256 must differ between V1 and V2; the current broker-only upgrade gate
requires the client SHA-256 and compatible protocol contract to match. The runner installs and
repairs V1, upgrades to V2, then verifies the installed hash, protected identity manifest, protocol,
service restart, and limited V2-compatible client path. The repository runner has no certificate
creation, trust-store mutation, private-key, or signing path. A dedicated VM may prepare such a
bundle as an offline prerequisite, but that
workflow is outside this runner and must use independently protected signing input.

The harness does not auto-elevate. Before system mutation, it requires the exactly counted broker
integration matrix and installer rollback guardrail to pass. It rejects a pre-existing fixed
service, installs through the normal installer verifier, exercises missing-binary repair and a
same-publisher upgrade, then uses an exact one-shot Task Scheduler entry with InteractiveToken
and RunLevel Limited to run the production-pipe client without administrator elevation. The client
itself rejects an elevated token. The parent receives the bounded result only through its own
one-instance named pipe with a protected DACL, and validates a random nonce, task instance, pipe-
observed client PID, process creation time, fixed protected client path/hash/signer/protocol,
limited token, and current Task Scheduler result before acknowledging success. No user-writable
result file is acceptance evidence. The client exercises Query/Read/cancel and incompatible
protocol, restarts SCM and repeats the positive path, compares SHA-256, length, last-write time, and
entry counts for the disposable root
and an external sibling, then independently removes and confirms absence of the exact scheduled
task, service, installed Application bundle, and fixture paths. The external pre-signed bundle is
read-only input and is never removed by the harness. During system-bound work, the harness pins the
workspace ancestor chain against rename, replaces the workspace DACL with SYSTEM/Administrators
full control and limited-user read/execute only, deletes fixture entries through no-follow handles,
then restores the original workspace descriptor in `finally`.
`./tool/acceptance_test_windows_journal_broker_guardrails.ps1` exercises `-ValidationOnly` without
invoking Task Scheduler or SCM mutation. Without an externally pre-signed protected bundle,
production admission remains unexecuted and the guardrail is the only permitted local path.
The guardrail runs under both hosted Windows PowerShell 5.1 and PowerShell 7. It proves the
runtime-specific secure pipe constructor or ACL factory and ACL reader yield one protected DACL
with exactly SYSTEM/Administrators `FullControl` and current-user `ReadWrite|Synchronize`; it never
falls back to an unsecured pipe or repairs access control after creation.

The deterministic prerequisite matrix proves that a portable production identity returns explicit
`LiveOnly` before constructing or connecting the broker factory, while the fixed installed identity
may connect exactly once. It also drives volume-wide root-unrelated records through bounded partial
pages until the captured end. The installer guardrail retains its eight install/repair/upgrade crash
cases. Its transaction-level matrix invokes the complete uninstall, injects crashes before and after
each of seven mutations and after each completion write, and resumes the persisted marker through
dead-owner recovery. Both present and initially absent install trees, committed recovery, marker
deletion before and after its boundary, exact parent-prestate restoration, and contradictory
phase/pending-operation failure are covered. Completed postconditions are re-verified unless a
strict persisted later operation specifically supersedes them. These checks do not invoke real SCM
or FSCTL operations and do not substitute for the elevated acceptance above.

## Release gate

The standard release gate runs daily verification, Windows Release packaging, the packaged bridge
smoke test, packaged same-user process ownership and replacement-start verification, and the
synthetic performance gate:

```powershell
./tool/release_verify_candidate.ps1 `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>" `
  -SignedBrokerBinaryPath "<signed x64 cedarflake_ame_journal_broker.exe>" `
  -SignedApplicationBundlePath "<immutable signed Windows Release directory>"
```

After that gate has produced a complete Windows Release directory, create the portable artifact:

```powershell
./tool/release_package_portable_windows.ps1 `
  -Tag "v0.1.0" `
  -ReleaseRoot "<immutable signed Windows Release directory>" `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>"
```

The artifact is written to `build/release-artifacts/` with one `Cedarflake-Ame/` archive root. A
manual release request dispatched from protected `main` performs this step only after its candidate
gate passes; a tag push never obtains signing credentials. The accepted identity, x64-only support
boundary, and deferred installer decisions are recorded in
[ADR 0015](../architecture/0015-windows-release-distribution.md).

The portable ZIP contains the broker executable only as signed installer-compatible payload. It
does not install, start, repair, update, or remove SCM state and always remains `LiveOnly`; a broker
already installed for the protected Program Files client does not authorize a user-writable
portable executable. Product admission rejects that portable identity before constructing or
connecting the broker factory, and transport defense rejects it before any SCM or named-pipe side
effect. The hosted candidate flow builds once without credentials, transfers a manifest-bound
unsigned artifact to the protected Environment signer, and transfers the resulting immutable signed
artifact to a separate credential-free verifier and read-only packager. The signer checks out no
repository content and signs only the application and broker. The final write-capable publisher
checks out no repository content, executes no repository script, and consumes only the manifest-
bound ZIP after a current tag-to-source comparison and hash recheck. A successful candidate release
invokes the reusable published-verification workflow directly with the admitted tag and publisher;
the `release: published` entry point independently verifies releases created outside that candidate
workflow, and `workflow_dispatch` is the recovery entry point. Published verification downloads the
ZIP, validates safe archive structure, extracts into fresh bounded scratch storage, and revalidates
both signatures, the exact publisher, x64 machine, and broker protocol before cleanup. The external
event entry point requires the repository Actions variable `AME_WINDOWS_EXPECTED_PUBLISHER`. The
repository does not contain production signing credentials or a signed candidate. Until the named
Environment secrets, protected-main deployment restriction, required reviewer, immutable version-
tag ruleset, exact `Windows Gate / Windows Gate` required check, and
`AME_WINDOWS_EXPECTED_PUBLISHER` variable are provisioned, candidate verification and external
publication verification fail closed by design.

When current authorization exists and the retained real-library catalog is applicable to the
release, append its validation explicitly:

```powershell
./tool/release_verify_candidate.ps1 `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>" `
  -SignedBrokerBinaryPath "<signed x64 cedarflake_ame_journal_broker.exe>" `
  -SignedApplicationBundlePath "<immutable signed Windows Release directory>" `
  -IncludeRealLibrary `
  -AcceptanceStorageRoot "<retained acceptance storage>" `
  -RootA "<first approved root>" `
  -RootB "<second approved root>" `
  -AuthorizationToken "<current authorization token>"
```

This option validates the retained catalog; it does not silently start another full source scan.
