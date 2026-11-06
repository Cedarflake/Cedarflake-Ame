# Repository command reference

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Repository command reference

The Windows CMake application target invokes `./tool/release_prepare_classification_runtime.ps1
-OutputDirectory <repository-build-subdirectory>` to prepare the admitted CPU runtime. It downloads
only the pinned official archive when that build directory is absent, verifies its digest and the
seven selected payload files, and reuses only complete verified output. `-ArchivePath <local-zip>`
supports an offline build using the same archive checks. Models remain outside this build step.
The installer adds its separately admitted model payload with
`release_prepare_classification_models.ps1 -OutputDirectory <repository-build-subdirectory>`;
`-SourceDirectory <verified-model-directory>` supports offline preparation. The installer builder
accepts that source through `-ModelSourceDirectory`, verifies artifacts/notices, and composes them
with the unsigned application manifest. `release_test_classification_models.ps1` covers bounded
preparation, retained notices, omission/tamper rejection and interrupted staging cleanup in Static.
`./tool/release_test_classification_runtime.ps1` supplies compiler-free archive, tamper, omission,
namespace and output-scope checks through Static. The native distribution-contract regression ties
the packaging DLL pins to the worker's compiled pins. Normal unsigned Release verifies the connected
application build; no local full Release run is added by this dependency.

The same target invokes `./tool/release_prepare_classification_vocabulary.ps1
-OutputDirectory <repository-build-subdirectory>` for the admitted lexical resource and its notices.
The build-only converter uses installed Python 3.11 or later, resolved through local `uv` when present.
It downloads only the two pinned source archives into repository build storage, verifies their exact
members and deterministically checks all six output files. `-SourceDirectory <local-archive-directory>`
supports offline preparation; existing output is reused only after complete verification.
`./tool/release_test_classification_vocabulary.ps1` supplies archive, download, conversion and atomic
publication fixtures through Static. Unsigned, portable and installer checks require the resource,
both licenses, both citations and the modification notice. Model execution and media reads are outside
this build step; the normal hosted Release job owns the complete application build.

The application-and-broker MSI is built by `./tool/release_package_installer_windows.ps1` from
an exact passing unsigned Release manifest. Supply the version tag, bundle/evidence paths,
the pinned WiX 7.0.0 SDK, UI-extension and bootstrapper-extension packages with their matching extracted directories,
the installed CMake executable,
and a new output directory below repository `build`. `-AcceptWixBinaryEula` requires prior review
of the build-tool maintenance agreement in ADR 0027; it does not authorize a subscription.
The builder holds the repository lock, verifies every payload digest, builds/tests the native
admission module with warnings denied, verifies the emitted MSI's native service, ACL, file,
upgrade, action-ordering, native options and maintenance-dialog tables, and emits one embedded-payload
MSI plus its receipt, and a Burn setup executable that opens the MSI interface after early elevation.
Supply the bootstrapper inputs through `-WixBootstrapperPackagePath` and `-WixBootstrapperDirectory`.
For a completed release-candidate gate, replace `-BundlePath` and `-UnsignedEvidencePath` with
`-CandidateRoot <manifest-and-payload-directory>` and `-SourceCommit <verified-source-sha>`.
The builder reverifies that immutable candidate before packaging; the input modes cannot be mixed.
The application version must match the tag and include a positive numeric Flutter build suffix.
The MSI upgrade number follows [ADR 0027](../../architecture/0027-windows-installer-lifecycle.md).
It also runs the rejecting database-policy cases against modified in-memory models.
`./tool/release_test_installer_ui.ps1 -PackagePath <msi> -DtfAssemblyPath <pinned-dtf-assembly>`
uses isolated MSI sessions for costing, exact property initialization and native condition evaluation
across all four incoming shortcut states and four checkbox combinations. It never executes the
installation sequence; native dialog interaction and installed shortcuts retain separate checks.
It does not install the package or establish lifecycle acceptance. The compiler-free
`./tool/release_test_installer_payload.ps1` covers version limits, stable product/component
identities, unsafe payload paths, shortcut ownership and complete file/directory ACL generation.
`windows/installer` owns the native path/service admission checks and their bounded CTest fixture.
`./tool/release_verify_installer_windows.ps1 -PackagePath <msi> -ReceiptPath <receipt>
-WixSdkPackagePath <pinned-sdk-package> -WixSdkDirectory <matching-sdk-directory>` repeats the
read-only package/digest/table checks without building, executing custom actions or installing.

Run quality commands from PowerShell. The scripts resolve Cargo from `PATH` or the user's Rustup
installation and resolve Flutter and Dart from `PATH` or the pinned local SDK location in [the repository contract](../../../AGENTS.md#101-local-flutter-toolchain).

Tool scripts use `snake_case` filenames beginning with an ownership category. Do not name a script
only after its immediate action, such as `format.ps1`, `lint.ps1`, `verify.ps1`, or `generate.ps1`.
Use these prefixes:

- `quality_*` for formatting, linting, daily verification, and shared quality implementation;
- `integration_*` for device-backed integration workflows;
- `acceptance_*` for authorization-bound acceptance checks and their guardrail tests;
- `performance_*` for explicit benchmarks and performance evidence;
- `release_*` for packaging and release-candidate verification;
- `bridge_*` for generated bridge maintenance.

GitHub Actions workflow files under `.github/workflows` follow the same `snake_case` ownership
prefixes. Do not add generic names such as `ci.yml`, `build.yml`, `test.yml`, or `release.yml`.
Workflow display names and job IDs must retain the same ownership boundary so branch-protection
checks remain understandable and stable. Reusable workflows are named for the gate they own, not
only for the fact that they are reusable.

Repository workflows must use the narrowest required `GITHUB_TOKEN` permissions, must not use
`pull_request_target` to execute untrusted pull-request code, and must pin external actions to full
commit SHAs. Normal push and pull-request CI may use dependency caches but must not restore compiled
application or library output as trusted build evidence. Authorization-bound real-library paths,
tokens, catalogs, and scans never enter GitHub-hosted workflows.

Choose the narrowest existing category before introducing another one. When a public script is
added or renamed, update its callers, repository instructions, and documentation in the same
change. Do not retain an undocumented alias that creates two canonical entrypoints.

- `./tool/quality_format.ps1` applies `rustfmt` and `dart format` to repository-owned Rust and Dart trees.
- `./tool/quality_format.ps1 -Check` is the non-mutating formatting gate.
- `./tool/quality_generate_library_synchronization_policy.ps1` reads the canonical positive-decimal
  poll interval from `tool/library_synchronization_poll_interval_ms.txt` and deterministically writes
  the Dart policy source. Values above `9223372036854775` milliseconds fail before output changes so
  Dart's signed 64-bit microsecond `Duration` representation cannot overflow. Its `-Check` mode
  compares exact UTF-8/LF bytes without writing.
- `./tool/quality_test_library_synchronization_policy.ps1` proves policy drift and malformed input
  plus the exact Dart microsecond boundary fail closed without changing generated output.
- `./tool/quality_test_windows_powershell_compatibility.ps1` loads the compiler-free R2c-R common
  surface and proves exact boolean platform binding under Windows PowerShell 5.1 and PowerShell 7.
  It does not replace the complete R2c-R guardrail or initialize its native compiler bootstrap.
- `./tool/quality_lint.ps1` validates repository PowerShell and JSON configuration, runs the policy
  generator guardrail and non-mutating generation check before the formatting gate, runs Clippy for
  all targets and features with warnings denied, and runs the pinned Dart analyzer with warnings and
  informational lints treated as failures.
- `./tool/quality_verify_dependencies.ps1` parses handwritten Dart imports, exports, conditional
  branches and parts, checks resolved layer/cross-feature edges and cycles, and rejects changed,
  unknown or stale legacy exceptions. Its exact bridge roster excludes reviewed generated
  output only; new handwritten files cannot gain an exemption by entering that directory. Static
  runs this command after formatting. Flutter owns its positive/negative boundary regressions.
  The existing locked `analyzer` package is a direct development dependency for this parser;
  no resolved package version or runtime dependency changes.
- `./tool/quality_lint_workflows.ps1 -ActionlintPath <path>` validates all hosted workflows with a
  caller-provided `actionlint` executable. Hosted CI supplies a fixed version with a verified
  checksum; the daily workstation gate does not silently download tools.
- `./tool/quality_test_flutter.ps1` expands the requested widget-test paths and runs each test file
  in its own `flutter test --concurrency=1` process while holding the repository tool lock.
- `./tool/quality_test_rust.ps1` runs every locked Cargo target and feature under the repository
  lock with single-threaded case execution. Its optional `-Partition` selects one hosted owner
  after full compiled-inventory validation; it never starts parallel local workers.
  `quality_rust_test_partitions.ps1` owns partition membership and libtest arguments, and
  `quality_test_rust_partition_contract.ps1` covers complete, overlapping and missing membership.
  The `other` partition includes `dependency_boundaries`: parsed domain/port/application/API
  references, exact named composition entries, and new/stale dependency rejection against the
  immutable native baseline. Its read-only inventory example shares the parser; neither path can
  regenerate accepted exceptions.
- `./tool/quality_verify_daily.ps1` is the daily gate. It runs the lint gate, Rust tests, Flutter tests, the
  controlled Windows scan integration, the native Windows accessibility integration, generated
  bridge compatibility, and whitespace validation for the complete tracked diff. With no arguments
  it remains serial for workstation safety. The shared hosted workflow may invoke its validated
  `-Component` partitions on isolated runners so the same evidence is collected in parallel without
  sharing Flutter, Cargo, or build state.
- `./tool/integration_test_windows_accessibility.ps1` runs a semantics-enabled virtual-gallery
  stress sequence in the native Windows runner and fails when engine stderr reports an invalid
  `ui::AXTree` update. Run and cleanup failures persist current Flutter output, verified native
  phases, and failure metadata before returning the original run error, replacing prior evidence.
- `./tool/integration_test_windows.ps1` runs controlled scan interactions in an owned process tree
  with a parent deadline. GUID-isolated logs and completion evidence survive failures; nonempty
  fixture storage is retained rather than recursively deleted without cleanup authority.
  `./tool/integration_test_windows_scan_guardrails.ps1` verifies its compiler-free lifecycle protocol
  in lint, including original-failure precedence and independent environment restoration.
- `./tool/integration_test_native_directory_picker.ps1` verifies the native picker action lifetime
  used by the Windows scan integration: ready-button admission, observed dialog closure, bounded
  delivery and cancellation, and original-failure retention. Static runs these controlled protocol
  cases; Windows Scan retains the real picker and complete import assertions.
- `./tool/integration_test_classification.ps1 -ModelDirectory <verified-models>
  -RuntimeDirectory <verified-runtime> -PythonPath <uv-python>` runs the explicit local Windows
  classification client workflow with existing admitted artifacts and an existing three-file,
  two-content fixture under `build/test` (`-FixturePath` overrides `build/test/metadata`). It uses
  the repository lock, an owned process tree and fresh isolated Debug storage. Its bounded Python
  observer reads only that derived storage; source bytes are compared before and after. Its native
  power probe validates its invoking Debug client and sends suspend/resume messages only to that
  process's runner HWND, without changing the computer's power state. It observes owned worker
  retirement and verifies the framework memory-pressure cooldown. Logs and catalogs remain outside
  the fixture. This opt-in model workflow adds no hosted model download.
- `./tool/integration_test_windows_runner.ps1` builds and executes three engine-free native window
  cases and two real Debug-engine retirement cases, using an explicit `-CMakePath` or the current
  Flutter CMake cache. The latter require prepared, matching SDK artifacts and assemble only an
  isolated dependency-free no-op Dart package offline; they never use the application package or
  Release engine payload. Their CTest cases have 30-second deadlines inside a 75-second owned Job
  lifetime. Fresh exact three-case and two-case JUnit reports are required; the engine suite also
  requires native exit and Job closure evidence and retains its logs. The unsigned Windows gate must
  invoke both internal owners after its fresh Release build under its existing lock.
  `./tool/integration_test_windows_runner_guardrails.ps1`
  includes the compiler-free engine guardrail and checks inputs, exact execution evidence, fresh
  results, command failure propagation, cleanup, and lock composition without building an engine.
- `./tool/quality_verify_git_range.ps1` checks committed whitespace over an explicit Git revision
  range so a clean hosted checkout does not turn `git diff HEAD --check` into an empty gate.
- `./tool/quality_verify_bridge_contracts.ps1` checks generated bridge hash identity and exact
  asynchronous API and wire method bodies without building, including coherent query and semantic
  date snapshots. `./tool/quality_test_bridge_contracts.ps1`
  runs compiler-free negative fixtures against that boundary, including cross-method false matches.
- `./tool/performance_benchmark_synthetic_library.ps1` is the explicit performance gate. It creates 10,000
  temporary images and records cold, warm, pause, resume, memory, and storage evidence.
  The scan case completes the production post-scan reclamation worker before measuring settled
  catalog storage, with a ten-second maintenance deadline. It records allocated bytes before and
  after reclamation separately; the final 64-MiB assertion does not describe peak allocation.
- `./tool/performance_run_synthetic.ps1` runs the fixed JPEG, 10,000-image scan, million-record
  parser, seven-format cold/warm preview, and 50,000-identity concurrent publication catalog with
  exact execution and bounded resource evidence. Hosted CI runs its cases on isolated workers;
  workstation use remains explicit and serial. Its focused guardrail runs in lint.
  `./tool/performance_test_synthetic_media.ps1` verifies the exact per-format resource, source, and
  cache evidence without decoding media; the synthetic protocol guardrail invokes it in lint.
  `./tool/performance_test_synthetic_publication.ps1` rejects incomplete scale, overlap, atomic
  publication, and pending-change evidence through the same compiler-free protocol gate.
- `./tool/quality_verify_unsigned_windows.ps1` builds and verifies the credential-free x64 application
  and broker plus an isolated Release-DLL/native-channel smoke. It never loads a retained catalog or
  replaces signed candidate, installation, or Windows 11 client acceptance. Payload guardrails run
  in lint; hosted CI requires the complete unsigned job.
- `./tool/acceptance_run_read_only_library.ps1` and `./tool/acceptance_verify_read_only_catalog.ps1` are the
  real-library gate. They require explicit known roots, bounded read-only scope and isolated storage
  outside source trees, without separate per-run approval; they never join unattended Daily verification.
- `./tool/acceptance_run_r2c_replacement_reliability.ps1` is the R2c-M replacement closeout gate.
  It measures production watcher operations against a disposable root, then backs up the retained
  catalog into isolated storage and measures per-root metadata-only continuity without opening
  media content or publishing a full scan. The retained-root phase follows the standing scoped
  read-only verification policy.
- `./tool/acceptance_test_r2c_replacement_guardrails.ps1` verifies the R2c-M authorization token,
  cloud acknowledgement, physical path separation, fresh storage, process ownership and final
  resource-limit samples without accessing a real library.
- `./tool/acceptance_run_r2c_change_driven_reliability.ps1` is the R2c-R non-external controlled
  local reliability gate for Windows 11 x64 client workstations. Its common module performs no
  dynamic compilation while being loaded; explicit initialization uses an in-memory native identity
  surface to bind and retain every existing logical and physical repository-tool path component,
  rejecting reparse or volume transitions before it creates the compiler bootstrap relative to the
  final physical handle. It cleans only an empty identity-matching bootstrap and retains unknown or
  replaced state without traversal. The runner executes
  exactly counted production-path tests as an ordinary user inside one fixed-NTFS KnownFolder tree.
  Before any root write, both the logical `SHGetKnownFolderPath` result and its physical filter-
  redirected path are bound component by component through no-follow, same-volume, identity-held
  handles. A high-entropy root is then created relative to the final physical LocalApplicationData
  handle, verified as its non-reparse same-volume direct child, and kept replacement-blocked for the
  complete runner lifecycle until cleanup begins. Parent-relative opens honor the live per-directory
  Windows case-sensitivity flag; object identity is never inferred from case-folded path text. The
  gate refuses caller-supplied source paths, catalogs, worker environment aliases, elevation,
  reparse/volume escape, and filtered-only results, and applies parent wall-clock deadlines through
  an owned process-tree Job Object.
- `./tool/acceptance_test_r2c_change_driven_reliability_guardrails.ps1` verifies the R2c-R platform,
  privilege, fresh-process hostile-temporary bootstrap, compiler failure, active bootstrap and
  two-stage cleanup replacement races, held physical storage, intermediate and terminal internal
  junction/sentinel rejection, identity-bound teardown, process-timeout, malicious NT leaves,
  cleanup retention, path/environment-alias, exact-matrix, an actually executed exact report-tamper
  test, default-deny source/call-closure coverage of every common/runner/guardrail scope and exact
  dot-source under fixed source/depth/byte/AST/function/scope/queue budgets with bounded non-
  materializing AST-node traversal, one digest-locked process boundary that audits each final
  Command/EncodedCommand/File payload before launch, no-follow same-volume parent/terminal identity
  retention, post-open volume/file-ID source deduplication, immediate pre-transfer revalidation for
  every file source, immediate pre-initialization native handle ownership, and the macro- and module-
  topology-closed exact item/call-closure Rust availability source contract without running the manual
  reliability scenarios or accessing a real library. Expected runner refusals use flushed stable
  reason tokens rather than PowerShell's formatted exception text. On a supported Windows 11 x64
  ordinary-user host, the guardrail requires the exact ValidationOnly success report; on another
  Windows host, it requires the unchanged production execution-context gate to reject that report.
  Cross-version environment deletion uses PowerShell `NullString.Value` and proves every protected
  name is absent before creating a runner process; an empty value is not treated as deletion.
  The lint and Daily paths run only this lightweight guardrail, not the R2c-R acceptance runner.
  Its worker-report check prepares the locked debug library test target with one Cargo job under
  a separate fifteen-minute owned-process deadline, then executes the unchanged exact test under
  its five-minute deadline. Build failure prevents execution; both phases retain source auditing
  and process-tree ownership. Failure output includes at most 24 lines of 512 characters each
  before fixture cleanup. `./tool/acceptance_test_r2c_worker_report_guardrail.ps1` checks command
  admission, phase ordering, deadlines, result rejection, output bounds, environment restoration
  and independent cleanup with failure preservation; lint runs it before the executable guardrail.
- `./tool/release_verify_windows.ps1` checks a verified Windows bundle and its release bridge;
  `-SigningMode unsigned` selects unsigned verification; its local default remains signed.
  It also proves that a packaged process rejects a same-user duplicate before runtime
  initialization and that a replacement starts after the original process exits. Run it when
  desktop integration, native packaging, generated bridge loading, or release behavior changes.
- `./tool/release_verify_candidate.ps1` retains the signed local release-candidate orchestrator.
  The unsigned CI channel calls Daily, `release_verify_windows.ps1 -SigningMode unsigned`, and
  synthetic performance on a separately manifest-verified immutable bundle. Retained real-library
  validation requires separate explicit authorization.
- `./tool/release_validate_version.ps1` requires a `v`-prefixed semantic version and verifies that
  the tag, Flutter application version, and Rust package version agree before a release gate runs.
- `./tool/release_package_portable_windows.ps1` packages the complete Windows x64 Release directory
  as a versioned portable ZIP after release verification.
- `./tool/release_verify_portable_archive.ps1` verifies the portable ZIP filename, single-root
  layout, safe entry paths, and required Flutter and Rust runtime payload without extracting it.
- `./tool/release_test_portable_publication.ps1` tests the workflow-owned attachment identity
  decision using controlled API fixtures: missing attachments may upload, matching SHA-256 digests
  skip publication, and conflicting, unprovable, or failed lookups fail closed. It runs in lint
  without network access or publication permission.
- `./tool/release_verify_unsigned_candidate.ps1` checks the immutable unsigned artifact manifest,
  tag/source commit, exact bounded payload, file hashes, unsigned x64 binaries and broker protocol.
- `./tool/release_verify_portable_unsigned.ps1` validates an extracted unsigned portable ZIP,
  including x64 application and broker protocol, then confirms scratch cleanup.
- `./tool/release_verify_portable_signatures.ps1` is the optional signed-channel verifier and
  requires exact publisher, signature, x64 machine, broker protocol and scratch cleanup.

Hosted workflow ownership is:

- `.github/workflows/quality_ci.yml` for pushes to `main`, pull requests, merge queues, and manual
  daily-gate runs;
- `.github/workflows/quality_gate_windows.yml` for the shared Windows daily or release gate;
- `.github/workflows/quality_gate_synthetic_windows.yml` and
  `.github/workflows/quality_gate_unsigned_windows.yml` for mandatory isolated workload and unsigned
  build evidence in ordinary quality CI, without publication or signing authority;
- `.github/workflows/release_candidate_windows.yml` for version-tag and manual release candidates,
  followed by portable ZIP publication;
- `.github/workflows/release_verify_published.yml` for post-publication attachment verification.

The gate sections above own environment requirements, complete procedures, and acceptance boundaries.

Do not replace these entrypoints with remembered command fragments in normal work. A focused test
may be run directly for fast feedback, but it does not replace the applicable repository gate.
