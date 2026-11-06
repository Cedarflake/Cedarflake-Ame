# Hosted workflow lifecycle and coverage

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Hosted workflow lifecycle

The hosted Windows gate is implemented once in
`.github/workflows/quality_gate_windows.yml` and called by category-owned trigger workflows:

- `quality_ci.yml` runs Daily plus the synthetic and unsigned-build gates for pushes to `main`, pull
  requests targeting `main`, merge queue checks, and manual dispatches;
- `release_candidate_windows.yml` is manually dispatched from `main` for one existing
  strict-SemVer tag, then publishes the verified portable ZIP;
- `release_verify_published.yml` downloads the exact published ZIP and independently verifies its
  identity and archive structure.

The push gate intentionally targets `main` rather than every feature branch. Feature work is
validated by the pull-request event, avoiding duplicate push and pull-request runs for the same
update. Merge queues receive their own `merge_group` check so the combined merge result cannot
inherit a stale pull-request status.

The main concurrency group preserves its active run while later updates wait. Only the newest
pending update is retained, so successive fixes cannot repeatedly discard the complete Rust
result. Pull-request and merge-group runs still cancel a superseded run in their own group.
Each run remains evidence for its recorded source commit, never a later queued head.

Every hosted job uses `windows-2025`. Daily components and the unsigned-build and release-verifier
jobs install Flutter 3.47.8 and use the repository Rust toolchain; the optional protected signer and
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
`workflow_dispatch`, `refs/heads/main`, and equality between the event SHA
and the workflow-definition SHA. The reusable gate receives that trusted workflow commit. Its
unsigned job checks out only the full `refs/tags/<tag>` ref, proves that the tag exists, that `HEAD`
equals `tag^{commit}`, and that the source is an ancestor of the trusted workflow commit, then builds
the application and journal broker without signing credentials. It uploads a unique, immutable
artifact whose manifest records the source commit and enumerates every payload file by safe relative
path, length, and SHA-256; its signing allowlist is exactly `cedarflake_ame.exe` and
`cedarflake_ame_journal_broker.exe`.

The default `unsigned` release channel skips the signing job entirely. A separate credential-free
verifier proves the tag/source/workflow ancestry, downloads the immutable unsigned candidate,
rechecks its complete manifest (all paths, lengths and SHA-256 hashes), x64 executables, unsigned
application/broker state and broker protocol, then runs the release quality, packaged application
and synthetic performance checks. No publisher variable or signing secret is required.

Only the optional `signed` release channel enters the `windows-production-signing` Environment,
which requires a reviewer and restricts deployment to `main`. Its PFX and password remain
Environment secrets. The signer checks out no source, has no `GITHUB_TOKEN` authority, signs only the
two fixed executables, scrubs ephemeral key material and uploads an immutable signed artifact. Its
independent verifier rechecks the signed manifest and publisher before release.

A separate read-only packager checks out the immutable source SHA, binds the selected channel's
manifest to that SHA and the still-current tag, reverifies the bundle, and uploads the exact ZIP
plus its SHA-256 as another immutable artifact. The sole `contents: write` publisher downloads only
that ZIP, checks out no repository and executes no repository script. Immediately before GitHub
Release mutation it peels the current tag through the GitHub API, requires the resolved commit to
equal the verified source, and rechecks the ZIP hash.

For hosted Daily runs, the shared gate calls `quality_gate_daily_windows.yml` for eight isolated
`windows-2025` jobs: static verification, four Rust partitions, Flutter widget tests, the controlled
Windows scan integration, and the native Windows accessibility integration. It also calls
`quality_gate_synthetic_windows.yml` for five separately
isolated workload jobs and `quality_gate_unsigned_windows.yml` for optimized application/broker
verification. A stable `Windows Gate` aggregation job succeeds only when every Daily, synthetic,
and unsigned-build job succeeds; skipped or cancelled requirements fail. Matrix fail-fast is disabled so
one failure does not hide results from the other components. Jobs do not exchange
compiled artifacts or build directories; only dependency caches may be reused. This reduces
wall-clock latency at the cost of additional hosted runner minutes. Release candidate jobs remain
ordered by immutable artifacts because build, protected signing, credential-free verification, and
publication form distinct trust boundaries.

Release Daily reuses the same eight component jobs after candidate construction, checking out the
candidate's manifest-bound source commit. It runs alongside the independent candidate runtime and
synthetic-performance verification; packaging waits for both to succeed. Hosted candidate verifiers
do not repeat the serial workstation Daily gate inside their 90-minute runtime-verification budget.
Their scan benchmark uses `performance_run_synthetic.ps1 -Case scan`, preserving the optimized
Release build, exact workload admission, process ownership, memory limits and performance assertions
used by ordinary synthetic CI.

Static and each Rust partition retain a 90-minute total budget; the other Daily components retain
60 minutes. Each budget includes its own toolchain setup and compilation. Product-workflow and test
deadlines, workload sizes, assertions and case-level serial execution remain unchanged.
The ordinary-client restart contract queries the native adapter on generated directories before
selecting its source volume. A repository-volume `LiveOnly` response can select the generated
system-temp candidate; transport/query errors remain failures, and absence of any supported journal
fails rather than skips the contract. It neither creates/enables a journal nor alters the original
restart, membership, identity, no-source-call, scan/inventory or per-process time requirements.

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
real-library gate. Ordinary PR CI runs the five synthetic cases without release permission; retained
real-library verification remains a separately prepared workstation action. Release jobs
may still show as skipped in a PR: the independent unsigned quality job supplies compilation evidence
without changing their manual main-only admission.

Read-only verification of known real-library roots is covered by the repository's standing policy
and does not require a separate confirmation for each run. Record explicit roots, read scope,
duration/resource bounds, isolated derived storage and source/placeholder safeguards before execution.
Existing admission tokens and acknowledgement parameters bind that prepared scope; they do not add
another user-confirmation step. Source mutation, service installation and external publication retain
their separate authorization requirements.

### Hosted coverage boundary

`performance_run_synthetic.ps1 -Case jpeg|scan|usn|media|publication|all` accepts only the fixed workload catalog.
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

`performance_test_synthetic_media.ps1` adds compiler-free rejection fixtures for the seven exact
format records, dimensions, resource bounds, unchanged sources, and warm cache reuse. The media
worker decodes real JPEG, PNG, WebP, GIF, BMP, TIFF, and ICO sources and verifies the resulting
preview pixels. Wrong-extension, non-image, truncated-header/payload, and same-path replacement
cases run separately in ordinary Rust tests; a JPEG-only speedup is not common-format coverage.

The `publication` case stages 50,000 distinct generated physical identities through the catalog port
and publishes while real no-change first-import completion calls run on an independent connection.
It requires actual transaction overlap, one publication call and commit, no partial visible catalog,
and unchanged pending P0 ownership. Its 600-second worker and 512 MiB working-set ceilings include
fixture preparation. `performance_test_synthetic_publication.ps1` rejects incomplete or forged
protocol records in lint. This is catalog-publication evidence, not a filesystem or native UI scan.

The USN case uses generated bytes and an injected backend; it does not access a volume journal or
stand in for the complete Windows 11 R2c-R runner. Old H/M controlled scenarios remain historical and
need ADR 0024 adaptation, not a blanket ignored-test switch. The three R2c-R production scenarios
retain their ordinary-user Windows 11 x64 entrypoint. No matching runner is assumed to exist, and
Server jobs never report client acceptance. Real-root gates remain separate; signing-dependent
deployment qualification applies only to the selected signed distribution channel. Ordinary
parent-owned WAL and small M metadata child tests remain part of Daily.

The unsigned gate has no credential-bearing Environment or publication path. It verifies a fresh
application, broker, and Rust payload and runs the native window lifecycle fixture and isolated
native-channel/Release-DLL smoke;
it never starts the existing retained-catalog smoke. See
[ADR 0026](../../architecture/0026-hosted-synthetic-and-unsigned-build-gates.md) for these boundaries.

The native lifecycle entrypoint holds the repository tool lock, resolves `cmake.exe` from an explicit
`-CMakePath` or the current Flutter build's `CMAKE_COMMAND`, and uses the sibling `ctest.exe`.
Both native suites are mandatory in the ordinary unsigned hosted gate, whose caller already owns
the lock and invokes their internal owners after the fresh application Release build. Compilation
and cases remain serial. The engine-free fixture uses a separate Release build directory and a
15-second deadline per case. It requires a fresh GUID-named, at-most-1-MiB JUnit report containing
`window_lifecycle_control`, `window_lifecycle_startup`, and `window_lifecycle_teardown`, all with
executed status and exact native completion markers.

The engine-retirement fixture uses only the prepared SDK's Debug `windows-x64` engine, matching
engine/Dart/SDK stamps, and existing compiler artifacts; missing or incompatible inputs fail before
assembly rather than downloading replacements. It copies the dependency-free no-op package from
`windows/runner/tests/engine_retirement` into fresh GUID storage under `build`, runs offline package
resolution, requires the package configuration to contain only itself, and assembles
`kernel_snapshot_program`. Only the current target stamp's `app.dill` output is copied as
`kernel_blob.bin`; duplicate entries naming that same output are deduplicated. The SDK's optional
`recorded_uses.json` in the same target directory is admitted as metadata and is never copied as the
kernel. Missing kernels and outputs outside that exact pair are rejected. The application
package and Release/ephemeral engine files are not inputs to this isolated Debug build.

`engine_lifecycle_explicit_destroy` and `engine_lifecycle_scope_exit` select Skia like the production
Windows entrypoint. Each has a 30-second CTest deadline; their parent has a 75-second deadline and
an owned kill-on-close Job Object. A separate
fresh, at-most-1-MiB JUnit report must prove both cases executed, including real engine start, child
destruction notification, completed scope exit, retired HWND, no access violation, no plugin
registration, and COM release after window scope. Native process exit and Job closure must also be
confirmed. Produced build/native stdout and stderr plus native completion metadata remain in the
generated storage, including on failure. Missing, duplicate, failed, disabled, skipped, or stale
results fail both suites; exit zero alone is insufficient.

The unsigned hosted workflow's always-run evidence upload includes the unsigned summary, the
engine-free JUnit reports and CTest `LastTest.log`, and the engine-retirement JUnit, native completion
record, and four build-step plus native stdout/stderr pairs. Only files produced before a failure
are available to upload. Its explicit diagnostic paths exclude generated packages, kernel blobs,
DLLs, and catalogs; retaining a complete local fixture does not upload that fixture to CI artifacts.

The compiler-free runner guardrail includes `integration_test_windows_engine_guardrails.ps1` for
offline inputs, isolated package/stamp authority, exact results, command failure and stderr behavior,
independent cleanup, and lock composition. These controlled suites do not replace full application
or plugin shutdown, failed-engine initialization, accessibility, signed packaging, or real-library
acceptance.
