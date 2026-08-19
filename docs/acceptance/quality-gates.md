# Quality gates

Status: active repository workflow

Cedarflake Ame separates deterministic development feedback from expensive or authorization-bound
acceptance work. A passing lower gate never claims that a higher gate ran.

| Gate | Entry point | Included evidence | When to run |
| --- | --- | --- | --- |
| Hosted CI | `.github/workflows/quality_ci.yml` | Parallel isolated Daily components plus committed revision-range whitespace validation on pinned Windows toolchains | Push to `main`, pull request, merge queue, or manual run |
| Daily | `./tool/quality_verify_daily.ps1` | Format, lint, Rust and Flutter tests, controlled Windows scan and native accessibility integrations, bridge hash, tracked diff whitespace | Every material change |
| Performance | `./tool/performance_benchmark_synthetic_library.ps1` | 10,000 temporary images, cold and warm scans, pause and resume, bounded memory | Scan pipeline, persistence, concurrency, or performance changes |
| Retained Profile | `./tool/performance_profile_retained_gallery.ps1` | Frozen-interaction Profile frame, memory, garbage-collection, query, publication, and retained-detail evidence; no source preview materialization | Guarded R2b gallery adaptations on the retained catalog |
| Preview performance acceptance | `./tool/acceptance_run_preview_performance.ps1` | Cold/warm bucket latency, cache growth, reuse, reclamation, regeneration, bounded memory, and sampled source integrity | Explicitly authorized R2b preview closeout only |
| Real library | `./tool/acceptance_run_read_only_library.ps1` and `./tool/acceptance_verify_read_only_catalog.ps1` | Explicitly authorized source scan, source integrity sampling, retained multi-root catalog validation | Only with current authorization and explicit paths |
| R2c reliability | `./tool/acceptance_run_r2c_reliability.ps1` | Real watcher latency and coalescing on a disposable root; isolated retained-catalog catch-up, queue, storage, memory, placeholder, metadata, and source-byte evidence | Explicitly authorized R2c-H closeout only |
| Release | `./tool/release_verify_candidate.ps1` | Daily gate, Windows Release and bridge smoke, synthetic performance gate, optional retained real-library validation | Before a release candidate |
| Portable artifact | `./tool/release_package_portable_windows.ps1` | Versioned Windows x64 ZIP plus archive-structure verification | After a release build passes |

## Hosted workflow lifecycle

The hosted Windows gate is implemented once in
`.github/workflows/quality_gate_windows.yml` and called by category-owned trigger workflows:

- `quality_ci.yml` runs the daily gate for pushes to `main`, pull requests targeting `main`, merge
  queue checks, and manual dispatches;
- `release_candidate_windows.yml` runs the release-candidate gate for an existing `v*.*.*` tag or
  an explicitly named tag selected by manual dispatch, then publishes the verified portable ZIP;
- `release_verify_published.yml` downloads the exact published ZIP and independently verifies its
  identity and archive structure.

The push gate intentionally targets `main` rather than every feature branch. Feature work is
validated by the pull-request event, avoiding duplicate push and pull-request runs for the same
update. Merge queues receive their own `merge_group` check so the combined merge result cannot
inherit a stale pull-request status.

Every hosted workflow uses `windows-2025`, Flutter 3.44.9, and the repository Rust toolchain. Before
the static Daily component or serial Release gate restores project dependencies, it downloads
actionlint 1.7.12, verifies the official Windows x64 SHA-256, and validates every workflow through
`./tool/quality_lint_workflows.ps1`. External actions are pinned to complete commit SHAs, checkout
credentials are not persisted, and ordinary workflow jobs have read-only repository contents
permission. Only the portable-publication job receives `contents: write`, and it starts only after
the read-only candidate gate succeeds. Pull-request jobs receive no release secrets or write
permission. The workstation daily gate does not download actionlint implicitly; contributors may
run the same script with an explicitly supplied executable.

For hosted Daily runs, the shared gate fans out four isolated `windows-2025` jobs: static and Rust
verification, Flutter widget tests, the controlled Windows scan integration, and the native Windows
accessibility integration. A stable `Windows Gate` aggregation job succeeds only when every
component succeeds, so branch protection keeps one durable required-check name. Matrix fail-fast
is disabled so one failure does not hide results from the other components. Jobs do not exchange
compiled artifacts or build directories; only dependency caches may be reused. This reduces
wall-clock latency at the cost of additional hosted runner minutes. Release candidates remain
serial because validation, release build, performance evidence, and packaging have an ordered
dependency chain.

Before the release-candidate workflow executes its gate,
`./tool/release_validate_version.ps1` checks that the `v`-prefixed tag, `pubspec.yaml` application
version without its build suffix, and `rust/Cargo.toml` package version are identical. The daily
lint gate runs a passing prerelease fixture and representative rejection cases for this contract.
It also builds a controlled portable fixture and proves that missing runtime files and unsafe
archive paths are rejected.

The published-release workflow resolves the version contract from the published tag, downloads only
`Cedarflake-Ame-<tag>-windows-x64-portable.zip`, and validates that attachment without extracting or
executing it. Signing, provenance, and checksums remain deferred supply-chain work rather than
claims of the current gate.

GitHub-hosted workflows never receive real-library paths or authorization tokens and never run the
real-library gate. A version-tag gate may run the synthetic performance benchmark, but retained real
library verification remains a separately authorized workstation action.

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

## Release gate

The standard release gate runs daily verification, Windows Release packaging, the packaged bridge
smoke test, and the synthetic performance gate:

```powershell
./tool/release_verify_candidate.ps1
```

After that gate has produced a complete Windows Release directory, create the portable artifact:

```powershell
./tool/release_package_portable_windows.ps1 -Tag "v0.1.0"
```

The artifact is written to `build/release-artifacts/` with one `Cedarflake-Ame/` archive root. A tag
push performs this step automatically only after its candidate gate passes. The accepted identity,
x64-only support boundary, and deferred installer decisions are recorded in
[ADR 0015](../architecture/0015-windows-release-distribution.md).

When current authorization exists and the retained real-library catalog is applicable to the
release, append its validation explicitly:

```powershell
./tool/release_verify_candidate.ps1 `
  -IncludeRealLibrary `
  -AcceptanceStorageRoot "<retained acceptance storage>" `
  -RootA "<first approved root>" `
  -RootB "<second approved root>" `
  -AuthorizationToken "<current authorization token>"
```

This option validates the retained catalog; it does not silently start another full source scan.
