# Quality gates

Status: active repository workflow

Cedarflake Ame separates deterministic development feedback from expensive or authorization-bound
acceptance work. A passing lower gate never claims that a higher gate ran.

| Gate | Entry point | Included evidence | When to run |
| --- | --- | --- | --- |
| Daily | `./tool/verify.ps1` | Format, lint, Rust and Flutter tests, controlled Windows scan integration, bridge hash, tracked diff whitespace | Every material change |
| Performance | `./tool/benchmark_synthetic_library.ps1` | 10,000 temporary images, cold and warm scans, pause and resume, bounded memory | Scan pipeline, persistence, concurrency, or performance changes |
| Real library | `./tool/accept_read_only_library.ps1` and `./tool/verify_read_only_library_catalog.ps1` | Explicitly authorized source scan, source integrity sampling, retained multi-root catalog validation | Only with current authorization and explicit paths |
| Release | `./tool/verify_release.ps1` | Daily gate, Windows Release packaging and bridge smoke, synthetic performance gate, optional retained real-library validation | Before a release candidate |

## Daily gate

```powershell
./tool/verify.ps1
```

The Windows integration fixture contains only controlled temporary files under `build`. It opens the
real directory picker through automation, exercises scanning and preview publication, verifies that
source bytes remain unchanged, and removes its temporary storage.

## Performance gate

```powershell
./tool/benchmark_synthetic_library.ps1
```

The default peak working-set ceiling is 512 MiB. Override it only when an accepted performance
decision defines a different budget:

```powershell
./tool/benchmark_synthetic_library.ps1 -MaxPeakWorkingSetBytes 536870912
```

## Real-library gate

Run the guarded scan command separately for every currently authorized root. The exact command and
safety conditions are documented in [read-only-real-library.md](./read-only-real-library.md).
After the expected roots have been published into one retained catalog, validate the complete
catalog through the production loading API:

```powershell
./tool/verify_read_only_library_catalog.ps1 `
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
./tool/verify_release.ps1
```

When current authorization exists and the retained real-library catalog is applicable to the
release, append its validation explicitly:

```powershell
./tool/verify_release.ps1 `
  -IncludeRealLibrary `
  -AcceptanceStorageRoot "<retained acceptance storage>" `
  -RootA "<first approved root>" `
  -RootB "<second approved root>" `
  -AuthorizationToken "<current authorization token>"
```

This option validates the retained catalog; it does not silently start another full source scan.
