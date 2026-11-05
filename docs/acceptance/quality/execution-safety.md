# Execution scope, storage and local toolchain

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

### Execution scope and storage

Before selecting, changing, or running verification, read the applicable definitions and
[command reference](commands.md#repository-command-reference). That document
owns tool parameters, guardrails, environment requirements, and hosted workflow ownership.
Use its canonical PowerShell entrypoints and ownership-prefixed script/workflow names; update
callers and documentation together when an entrypoint changes. Do not create a second canonical alias.

Keep selected local heavy work serial under the repository lock. Hosted CI owns the applicable
automated Daily, native, performance, and release checks; focused diagnosis does not remove those
requirements. Client and real-library acceptance that requires the workstation stays local, scoped
to the affected user workflow rather than replaying hosted CI. Read-only real-library verification
of known roots is included in project work and does not require separate per-run approval. Define the
roots, read scope, resource limits, and isolated derived storage before each run; preserve source
media and original catalogs, and never hydrate placeholders. Real-library runs remain separate
from unattended Daily verification. Source mutation, service installation, and external publication
retain their applicable explicit-authorization requirements.
Hosted workflows use minimal token permissions, full-SHA action pins, no untrusted
`pull_request_target` execution, and no restored compiled output as trusted build evidence.
Real-library paths, tokens, catalogs, and scans never enter hosted workflows.

For local gallery workflows, reuse the established library in `build/test` instead of generating
another library per run. Cover varying image dimensions, dates, encoded sizes and non-uniform
content. Keep the reusable corpus, original catalogs and user data; after a run ends, clean up
disposable test storage, caches and outputs. Before cleanup, verify process retirement, profile
restoration, exact workspace-contained targets and absence of reparse traversal. Do not delete
active outputs or unresolved restoration backups. Hosted and unit fixtures retain isolated storage.

Disposable local acceptance copies use `Invoke-AmeAcceptanceScratch` from
`tool/acceptance_scratch_storage.ps1`. Its named directory under `build/test/derived/temporary`
is leased for the run and removed after success or failure. Keep the reusable baseline, model
artifacts and small diagnostic reports outside that directory; do not archive a complete profile
for each failed attempt. Join owned child processes before the action returns. Busy files and
reparse points reject cleanup; an unmarked directory is never adopted. A subsequent invocation
can reclaim a marked, inactive directory left by interruption.

Native-DLL diagnostics using `CEDARFLAKE_AME_TEST_STORAGE_ROOT` require an optimized acceptance
build with Rust debug assertions enabled; an ordinary Release library ignores that override.
Before starting the diagnostic child, verify a build receipt bound to the loaded DLL's hash and
the isolated-storage configuration. A missing or mismatched receipt stops before loading the library.

The repository Cargo configuration disables incremental compilation to prevent repeated local
feature/build variants from accumulating compiler working directories. Normal dependency and
compiled-artifact reuse remains enabled. This trades some edited-crate rebuild speed for lower
storage use; it does not change debug symbols, assertions or release optimization.
`rust/cargokit.yaml` enables Rust optimization level 2 for Flutter Debug builds so media decoding
and analysis remain usable during `flutter run`. Debug symbols, assertions and Dart hot reload stay
enabled. Ordinary Cargo test/dev profiles and Flutter Profile/Release builds keep their own settings.
The scratch lifecycle and a dependency-free Cargo probe run in the hosted Static gate through
`tool/acceptance_test_scratch_storage.ps1`.

### Local Flutter toolchain

- The installed Flutter SDK root on this workstation is resolved from
  `$env:USERPROFILE\develop\flutter`; do not commit the expanded user-specific path.
- PowerShell does not currently expose `flutter` or `dart` through `PATH`. Invoke the verified
  executables explicitly instead of searching for, downloading, or installing another SDK:
  - Flutter: `& "$env:USERPROFILE\develop\flutter\bin\flutter.bat"`
  - Dart: `& "$env:USERPROFILE\develop\flutter\bin\cache\dart-sdk\bin\dart.exe"`
- Select local Flutter work under [verification ownership](verification-workflow.md#verification-ownership); these toolchain
  rules do not add a local CI stage.
  Run selected formatting, diagnostic commands, and client-verification builds serially on this
  workstation. If a command hangs or leaves a tester process behind, stop and inspect that process
  before starting another Flutter command.
- Use the repository's lock-aware entrypoints for selected local Flutter work. When a focused widget
  test is needed for a specific diagnosis or requested local verification, use
  `./tool/quality_test_flutter.ps1 -TestPath <path>` instead of launching a parallel `flutter test`
  process. Never terminate Dart or Flutter processes merely because they appeared after a command
  began; cleanup must be limited to descendants or executables proven to belong to that command.
- In a workspace-only agent sandbox, `flutter.bat` must still update its SDK-owned
  `bin/cache/flutter.bat.lock`. If the batch process loops before creating a Dart child, rerun the
  same scoped repository command with the required sandbox approval. Do not delete the SDK lock
  file or infer that an unrelated Dart process owns it without an exclusive-open check.
