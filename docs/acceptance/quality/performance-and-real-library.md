# Performance and real-library gates

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

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
regeneration churn. Prepare the dedicated read-only gate with the known `local-primary` root:

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
safety conditions are documented in [read-only-real-library.md](../read-only-real-library.md).
After the expected roots have been published into one retained catalog, validate the complete
catalog through the production loading API:

```powershell
./tool/acceptance_verify_read_only_catalog.ps1 `
  -StorageRoot "<retained acceptance storage>" `
  -RootA "<first approved root>" `
  -RootB "<second approved root>" `
  -AuthorizationToken "<current authorization token>"
```

Each source scan needs a current scope binding; a retained catalog or old token does not replace it.
Preparing that binding under the standing read-only policy does not require another confirmation.

## Historical R2c-H reliability evidence

The direct-desktop USN implementation and R2c-H execution entrypoint are retired. Their
[recorded results](../r2c-h-large-library-reliability.md) remain historical; they do not validate the
ADR 0024 runtime. Current broker/identity continuity uses the R2c-R entrypoint below. Metadata-only
inventory checks retain the R2c-M entrypoint and its source-isolation/process guardrails.

## R2c replacement reliability gate

Prepare the R2c-M gate with both logical roots, the retained catalog and new empty derived storage
explicitly bound to its read-only scope:

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

The wrapper owns its Job Object, deadline, memory, physical path, empty storage, token, and
cloud acknowledgement controls. Its non-accessing boundary is exercised through
`./tool/acceptance_test_r2c_replacement_guardrails.ps1`. `-ValidationOnly`, disposable evidence, and
the small retained-catalog fixture do not substitute for an authorized target-scale run. See
[r2c-m-replacement-reliability.md](../r2c-m-replacement-reliability.md) for the recorded evidence and
remaining authorization boundary.
