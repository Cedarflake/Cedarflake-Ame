# Gate selection and common admission

Status: binding requirements under [quality gates](../quality-gates.md).

Cedarflake Ame separates deterministic development feedback from expensive or authorization-bound
acceptance work. A passing lower gate never claims that a higher gate ran.

R2c engineering acceptance and merge require the unsigned quality and applicable functional gates.
They do not require signing credentials, publisher identity, signed bundles, protected signing jobs
or the signing-dependent installed-service acceptance runner. Signed release and deployment commands
below belong to the separately selected signed distribution channel; their absent-input refusals do
not fail R2c closeout. Their production signature checks remain intact. Real journal, broker security,
retained-library and Cloud Files assertions still require applicable evidence, with any unexecuted
deployed-service behavior disclosed accurately. See
[ADR 0026](../../architecture/0026-hosted-synthetic-and-unsigned-build-gates.md#decision).

`./tool/quality_verify_bridge_contracts.ps1` is the read-only bridge contract entrypoint used by
Daily's static component. It checks matching content hashes and exact Rust, Dart API, generated
Dart, and Rust-wire method boundaries for catalog reads, retained cancellation, storage commands,
and original-image source acquisition and lease methods. Comments, literals, and another method's
normal execution cannot satisfy a contract. `./tool/quality_test_bridge_contracts.ps1` runs the
compiler-free in-memory rejection fixtures in lint. Neither command generates bridge files or
replaces the native DLL integration and Release smoke.

`./tool/integration_test_windows.ps1` runs the controlled scan workflow inside an owned Windows
Job Object with a 900-second parent deadline by default. Each invocation uses fresh GUID-named
storage under `build`; stdout, stderr, and `completion.json` retain the original run failure and
separate cleanup failures. The runner restores its environment and releases its tool lock without
killing another same-path application instance. Nonempty fixture storage is retained as evidence,
not recursively deleted without identity-held cleanup authority. Hosted scan jobs upload only the
three diagnostic files, never catalogs or media. `./tool/integration_test_windows_scan_guardrails.ps1`
checks the run, deadline, failure-precedence, and environment-restoration protocol without Flutter.

| Gate | Entry point | Included evidence | When to run |
| --- | --- | --- | --- |
| Hosted CI | `.github/workflows/quality_ci.yml` | Parallel isolated Daily components, five synthetic workloads, unsigned x64 Release build, and committed revision-range whitespace validation | Push to `main`, pull request, merge queue, or manual run |
| Hosted synthetic workloads | `./tool/performance_run_synthetic.ps1` | Exact JPEG, 10,000-image scan, million-record parser, seven-format cold/warm preview, and 50,000-identity concurrent publication cases | Mandatory hosted CI; explicit serial workstation invocation |
| Unsigned Release build | `./tool/quality_verify_unsigned_windows.ps1` | Fresh x64 application/broker, payload and dependency identity, native window lifecycle, isolated Release bridge smoke; no catalog or signing | Mandatory hosted CI; explicit local packaging verification |
| Native window lifecycle | `./tool/integration_test_windows_runner.ps1` | Three engine-free hidden-HWND cases plus two real Debug-engine explicit/scope retirement cases using the production window owner | Native runner lifecycle changes; mandatory in unsigned hosted verification |
| Daily | `./tool/quality_verify_daily.ps1` | Format, lint, Rust and Flutter tests, controlled Windows scan and native accessibility integrations, bridge hash plus asynchronous API/wire-mode contracts, tracked diff whitespace | Every material change |
| Performance | `./tool/performance_benchmark_synthetic_library.ps1` | 10,000 temporary images, cold and warm scans, pause and resume, bounded memory | Scan pipeline, persistence, concurrency, or performance changes |
| Retained Profile | `./tool/performance_profile_retained_gallery.ps1` | Frozen-interaction Profile frame, memory, garbage-collection, query, publication, and retained-detail evidence; no source preview materialization | Guarded R2b gallery adaptations on the retained catalog |
| Preview performance acceptance | `./tool/acceptance_run_preview_performance.ps1` | Cold/warm bucket latency, cache growth, reuse, reclamation, regeneration, bounded memory, and sampled source integrity | Prepared read-only R2b preview closeout |
| Real library | `./tool/acceptance_run_read_only_library.ps1` and `./tool/acceptance_verify_read_only_catalog.ps1` | Bounded read-only source scan, source integrity sampling, retained multi-root catalog validation | Known roots, explicit paths, isolated storage; no separate per-run approval |
| R2c replacement reliability | `./tool/acceptance_run_r2c_replacement_reliability.ps1` | Disposable watcher operation latency, storm and restart recovery; isolated retained-catalog metadata inventory, cached gallery, source metadata, placeholder, full-scan, storage, and memory evidence | Prepared read-only R2c-M closeout |
| R2c-R controlled local reliability | `./tool/acceptance_run_r2c_change_driven_reliability.ps1` | Exactly counted production-path live, durable P1, overflow/P2, million-record, no-change, priority, reset/trim, reconnect, cancellation, replacement, placeholder, path, and multi-root evidence using only fixture-owned disposable roots | Manual non-external Windows 11 x64 checkpoint; never retained-library acceptance |
| Journal broker integration | `./tool/integration_test_windows_journal_broker.ps1` | Exactly counted disposable local NTFS tests for proof framing, two listeners, global eight-worker bounds, active-stop drain, listener readiness, read-only identity grants, idle retirement and admission races, verified service handover, case semantics, root replacement, reparse rejection, caller lifetime, revoked directory access, root-external filtering, bounded progress through unrelated-volume storms, cancellation, must-close backpressure, identity-probe deadline, restart, and unsigned watcher-first activation | Broker transport, service, parser, or lifecycle changes |
| Journal broker installer guardrails | `./tool/release_test_journal_broker_installer_guardrails.ps1` | Windows 11 x64, optional signing policy, V2/V3 identity and rollback compatibility, fixed destination, service account/SID/privileges/DACL, source/catalog isolation, no journal mutation, the existing eight install/repair/upgrade crash cases, and transaction-level marker-first uninstall/dead-owner recovery across every operation and final cleanup without changing SCM | Broker installer lifecycle changes |
| Journal broker privilege profile | `./tool/release_test_journal_broker_privileges.ps1` | Exact journal-access and caller-impersonation privileges in either order; missing, duplicate and extra privileges are rejected without changing SCM | Broker privilege configuration changes; also part of lint |
| Installed journal broker acceptance guardrails | `./tool/acceptance_test_windows_journal_broker_guardrails.ps1` | Explicit token, fresh temporary workspace, externally pre-signed protected bundle/publisher, ValidationOnly non-mutation, protected result channel, limited-client replacement rejection, no-follow cleanup, and no certificate, signing, trust-store, or automatic-elevation path | Installed broker acceptance harness changes |
| Release | `.github/workflows/release_candidate_windows.yml` | Unsigned by default: protected tag/source admission, complete candidate manifest verification, Daily gate, packaged process/bridge smoke, synthetic performance and bounded portable packaging; optional signed channel retains Authenticode checks | Manual release from protected `main` |
| Portable artifact | `./tool/release_package_portable_windows.ps1` | Versioned Windows x64 ZIP; unsigned mode checks original identities and broker protocol, while optional signed mode verifies expected Authenticode publisher; no service installation | After the selected release candidate passes |
| Portable publication guardrails | `./tool/release_test_portable_publication.ps1` | Exact inline workflow decision: missing attachment upload, same-digest skip, different or missing digest rejection, pagination, and fail-closed lookup errors; no network or publication | Release publication changes; included in lint |
