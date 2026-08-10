# Quality gates

Status: active repository workflow

Cedarflake Ame separates deterministic development feedback from expensive or authorization-bound
acceptance work. A passing lower gate never claims that a higher gate ran.

| Gate | Entry point | Included evidence | When to run |
| --- | --- | --- | --- |
| Hosted CI | `.github/workflows/quality_ci.yml` | Daily gate plus committed revision-range whitespace validation on a pinned Windows toolchain | Push to `main`, pull request, merge queue, or manual run |
| Daily | `./tool/quality_verify_daily.ps1` | Format, lint, Rust and Flutter tests, controlled Windows scan integration, bridge hash, tracked diff whitespace | Every material change |
| Performance | `./tool/performance_benchmark_synthetic_library.ps1` | 10,000 temporary images, cold and warm scans, pause and resume, bounded memory | Scan pipeline, persistence, concurrency, or performance changes |
| Real library | `./tool/acceptance_run_read_only_library.ps1` and `./tool/acceptance_verify_read_only_catalog.ps1` | Explicitly authorized source scan, source integrity sampling, retained multi-root catalog validation | Only with current authorization and explicit paths |
| Release | `./tool/release_verify_candidate.ps1` | Daily gate, Windows Release packaging and bridge smoke, synthetic performance gate, optional retained real-library validation | Before a release candidate |

## Hosted workflow lifecycle

The hosted Windows gate is implemented once in
`.github/workflows/quality_gate_windows.yml` and called by category-owned trigger workflows:

- `quality_ci.yml` runs the daily gate for pushes to `main`, pull requests targeting `main`, merge
  queue checks, and manual dispatches;
- `release_candidate_windows.yml` runs the release-candidate gate for an existing `v*.*.*` tag or
  an explicitly named tag selected by manual dispatch;
- `release_verify_published.yml` reruns the release-candidate gate against the published release tag.

The push gate intentionally targets `main` rather than every feature branch. Feature work is
validated by the pull-request event, avoiding duplicate push and pull-request runs for the same
update. Merge queues receive their own `merge_group` check so the combined merge result cannot
inherit a stale pull-request status.

Every hosted workflow uses `windows-2025`, Flutter 3.44.9, and the repository Rust toolchain. Before
restoring project dependencies, the shared gate downloads actionlint 1.7.12, verifies the official
Windows x64 SHA-256, and validates every workflow through
`./tool/quality_lint_workflows.ps1`. External actions are pinned to complete commit SHAs, checkout
credentials are not persisted, and the workflow token has read-only repository contents permission.
Pull-request jobs receive no release secrets or write permission. The workstation daily gate does
not download actionlint implicitly; contributors may run the same script with an explicitly supplied
executable.

Before either release workflow executes the release candidate gate,
`./tool/release_validate_version.ps1` checks that the `v`-prefixed tag, `pubspec.yaml` application
version without its build suffix, and `rust/Cargo.toml` package version are identical. The daily
lint gate runs a passing prerelease fixture and representative rejection cases for this contract.

The published-release workflow currently revalidates the immutable source tag and release build. It
does not yet claim to verify downloadable release attachments; checksum and attachment verification
becomes required when repository-owned artifact publication is introduced.

GitHub-hosted workflows never receive real-library paths or authorization tokens and never run the
real-library gate. A version-tag gate may run the synthetic performance benchmark, but retained real
library verification remains a separately authorized workstation action.

## Daily gate

```powershell
./tool/quality_verify_daily.ps1
```

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

The Windows integration fixture contains only controlled temporary files under `build`. It opens the
real directory picker through automation, exercises scanning and preview publication, verifies that
source bytes remain unchanged, and removes its temporary storage.

## Performance gate

```powershell
./tool/performance_benchmark_synthetic_library.ps1
```

The default peak working-set ceiling is 512 MiB. Override it only when an accepted performance
decision defines a different budget:

```powershell
./tool/performance_benchmark_synthetic_library.ps1 -MaxPeakWorkingSetBytes 536870912
```

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

## Release gate

The standard release gate runs daily verification, Windows Release packaging, the packaged bridge
smoke test, and the synthetic performance gate:

```powershell
./tool/release_verify_candidate.ps1
```

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
