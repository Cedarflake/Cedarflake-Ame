# ADR 0015: Versioned Windows x64 portable distribution

- Status: Accepted
- Date: 2026-08-11
- Last amended: 2026-11-07

## Context

Cedarflake Ame needs a distributable Windows build before the later installer, update, rollback,
signing, and supply-chain work is ready. Flutter's Windows Release output is a directory containing
the executable, Flutter and plugin DLLs, the Rust bridge DLL, and runtime data. Shipping only the
executable would produce an unusable application.

The application currently has one primary user and one measured platform. Unsigned distribution
is supported without a certificate or publisher identity. Optional production signing requires a
human-reviewed GitHub Environment restricted to `main`; signing credentials must never
be available to a tag-controlled or arbitrary-ref workflow definition. Both channels require the
complete release-candidate gate before publication.
Original media, local catalogs, caches, machine-specific paths, and identity data must never enter a
release artifact.

## Decision drivers

- create one artifact that can be downloaded and run without installation;
- keep the complete Flutter Windows runtime bundle intact;
- maintain one stable application and artifact identity across later packaging work;
- reject incomplete or path-unsafe archives before publication;
- avoid ARM and installer claims that cannot be tested in the current environment;
- keep write permission out of ordinary quality and candidate-verification jobs.

## Considered options

### Portable ZIP only for the current stage

Package the complete Windows x64 Release directory beneath one `Cedarflake-Ame/` folder. This is
simple to inspect, does not require installer technology, and matches the currently measured
platform. It does not provide Start menu registration, uninstall behavior, or automatic updates.

### Portable ZIP and MSI now

An MSI would provide installation and uninstall semantics, but choosing an installer tool, stable
upgrade codes, cache cleanup behavior, and update interaction before those workflows are designed
would create compatibility commitments without acceptance evidence.

### Single executable

Flutter Windows is not a single-file runtime. Repacking or embedding all runtime dependencies would
add a custom loader and new failure modes without product value.

## Decision

Use these stable identities:

- display name: `Cedarflake Ame`;
- application namespace: `com.cedarflake.ame`;
- Windows executable: `cedarflake_ame.exe`;
- release tag: `v<semantic-version>`;
- portable artifact: `Cedarflake-Ame-<tag>-windows-x64-portable.zip`;
- archive root: `Cedarflake-Ame/`.

The semantic version in the release tag, Flutter application manifest without its build suffix,
and Rust package manifest must match. SemVer 2.0 numeric prerelease identifiers have no leading
zeroes; identifiers such as `-01` and `-rc.01` are rejected at every hosted and local release entry.
The current initial release line is `0.1.0`; the Flutter build suffix remains build metadata and is
not part of the tag or artifact name.

Only Windows 11 x64 is supported and published. ARM packages are not produced or implied. A hosted
`windows-2025` Windows Server runner supplies reproducible build and release checks, but it does not
replace the separately authorized Windows 11 client acceptance evidence.

Every release provides the MSI installer and portable ZIP under the same release tag. Its product
version, embedded model payload and installation lifecycle follow [ADR 0027](0027-windows-installer-lifecycle.md).
An additional installer may reuse the completed candidate's immutable application files and runtime
evidence; reverify that manifest, validate the installer itself and preserve existing release assets.
Installer packaging must not rebuild or substitute the already published application payload.

The portable archive contains the complete repository-built Flutter Release directory. Archive
verification rejects an unexpected filename, entries outside the single archive root, traversal or
absolute paths, duplicate paths, missing executable or core runtime DLLs, missing Rust bridge DLL,
missing ICU or application data, and missing Flutter assets.

The release-candidate workflow is manually dispatched only from `main`; a tag push
never obtains publication or signing credentials. The default `unsigned` channel verifies the
manifest-bound source and binaries without a PFX or expected publisher. Optional `signed` release
uses the `windows-production-signing` Environment, which must require a reviewer and restrict
deployment branches and tags to `main`. Its PFX and password are Environment secrets,
never repository or organization secrets. Branch and tag protection are optional repository
settings, not publication prerequisites. Only after the
selected read-only candidate and package gates succeed may a separate no-checkout job with
repository-content write permission publish or update the corresponding GitHub Release. The
published-release workflow independently verifies the exact attachment in the selected channel.

Release text comes from `docs/releases/<tag>.md` at the immutable trusted workflow commit.
A separate admission step retrieves that file as data through the GitHub API; the publisher still
performs no repository checkout or script execution. It requires nonempty UTF-8 Markdown of at most
64 KiB before any release mutation. Publication uses this text for new releases and synchronizes it
on reruns without replacing verified package attachments. It never falls back to generated notes.

ADR 0026 adds a separate credential-free unsigned build check to ordinary quality CI. It verifies
the application and broker payload plus an isolated bridge smoke, but does not feed artifacts to
this candidate chain or claim signing, installation, publication, or client acceptance.

### Hosted artifact transport

Admit `actions/upload-artifact` 6.0.0 and `actions/download-artifact` 7.0.0, both MIT-licensed,
pinned to complete commit SHAs in the workflows. Their action manifests select Node.js 24, matching
the hosted runner runtime without relying on forced execution of Node.js 20 actions. They require
Actions Runner 2.327.1 or later. Existing named-artifact downloads retain their direct destination
layout; the intervening single-artifact-by-ID path change does not apply to these workflows.
Manifest/hash verification remains the admission boundary when replacing either action; transport
updates do not change the verified payload, local toolchain or protected publication authority.

### ADR 0024 amendment: service-enabled distribution

The portable ZIP remains a valid Windows x64 artifact. It does not install or update the constrained
journal broker. Under ADR 0024, a portable, Debug or Release client can use an already installed,
compatible broker without a signature or fixed client path. Missing or unverifiable broker service
falls back to explicit `LiveOnly`; runtime continuity is determined by actual journal/root evidence.

ADR 0024 moves the minimum installer work required to install, repair, upgrade, and remove
that broker into R2c. This is a narrow amendment to the stage boundary below, not a replacement of
the portable identity, archive layout, candidate gate, or publication rules in this ADR. General
automatic update, downgrade, catalog rollback, and broader supply-chain maturity remain deferred.

The verified Release directory and portable archive include
`cedarflake_ame_journal_broker.exe`, but extracting or running the portable application never
installs, repairs, upgrades, or removes the service. Any admitted local client may demand-start the
installed broker and negotiate its protocol under ADR 0024's OS caller and root-access rules.
The bundled executable alone grants no service-installation authority.

Broker lifecycle entrypoints are fixed to Windows 11 client x64 and the Ame-owned
`FOLDERID_ProgramFilesX64\Cedarflake Ame\Journal Broker` directory. The lifecycle runs only in a
native 64-bit process and resolves that known folder through `SHGetKnownFolderPath`; the
`ProgramFiles` environment value and a WOW64 redirected path have no authority. Install, repair,
and upgrade reject a non-x64 PE. An optional explicit publisher additionally requires a valid exact
Authenticode identity; omission permits unsigned payloads. Broker-only installation does not
require an Application bundle. Upgrade validates the existing binary against its own protected
manifest before staging the replacement. V2 rollback preserves its original publisher/certificate
checks; new V3 manifests bind protocol, maximum frame and broker image hash. Source inspection is
passive and protocol execution follows protected staging, exact ACL and copied-hash verification.
The transaction stops the service with a bounded wait, records
only paths and service objects created or moved by this invocation in a protected, write-through
persistent marker, renames the old binary to a
rollback name when one exists, and revalidates service and binary ACL state. Every rollback action
runs independently in reverse order and failures are aggregated, including failures after the new
service starts or while deleting the owned backup. A repair with a missing binary creates no fake
backup ownership and never deletes a pre-existing path. Uninstall first performs the same available
binary identity checks, removes only the fixed service through a bounded stop/delete/disappearance
wait, then removes the fixed binary and an empty broker directory; it has no catalog, cache,
source-root, or USN-journal operation.
Application removal requires the explicit `-RemoveApplicationBundle` switch. New schema-3
transactions persist that boolean intent; broker-only uninstall and its recovery do not inspect or
delete an existing Application tree. Interrupted schema-2 uninstall retains its original bundle
removal semantics.

Every lifecycle entry first validates and recovers a prior marker. It refuses a live owner identified
by both PID and process start time, treats PID reuse or a dead owner as interrupted work, and executes
every recovery action independently. The marker preserves whether the Ame parent existed and its
exact descriptor so rollback restores that state or removes a parent created by this invocation.
Owned-tree deletion uses a bounded no-follow walker and rejects any descendant reparse point. A
markerless broker directory may be adopted only when it is empty, has the exact protected descriptor,
and neither the service nor binary exists; a nonempty or differently protected orphan fails closed.

ADR [0027](0027-windows-installer-lifecycle.md) replaces the user-facing script installation route
with a versioned MSI whose native tables own both application and service lifecycle. The scripts
remain diagnostic/recovery tools; the MSI neither invokes them nor inherits their separate
transaction markers. A production signing identity is not implied. Signed release-candidate
verification requires an already signed broker path and the exact expected publisher. An unsigned
release instead requires complete manifest, executable architecture, broker protocol, runtime and
process checks; the absence of signing credentials never blocks that channel.

Before any lifecycle mutation, every existing component from the local volume root through the
fixed install directory is opened and its normalized final handle path is compared with the
expected path. A reparse directory, a path redirection, an owner other than SYSTEM, Administrators,
or TrustedInstaller, or an applicable untrusted allow ACE granting object replacement, directory
write, `DELETE`, `WRITE_DAC`, `WRITE_OWNER`, or parent `FILE_DELETE_CHILD` fails closed. Inherited-
only ACEs do not authorize the current object, and a newly required Ame directory is created with a
protected SYSTEM/Administrators security descriptor in the same `CreateDirectoryW` operation so no
user-writable intermediate directory exists before ACL hardening. The installed directory and
binary then require exact protected SYSTEM/Administrators full-control and BUILTIN Users
read/execute rules without write, delete, `WRITE_DAC`, or `WRITE_OWNER`; the service process remains
restricted by its SCM service SID and required-privilege contract. The SCM object requires the exact
admitted owner, group, and DACL rather than a required-SID substring check.

The narrow native installer helper owns three resource invariants: the known-folder allocation is
freed with the matching COM allocator, every final-path handle has one `SafeFileHandle` owner, and
the converted security descriptor remains live through synchronous directory creation and is then
released with `LocalFree`. Failure to prove any of these path or ACL facts aborts before an
installer-owned write or delete.

The same installer boundary creates or validates the complete fixed
`FOLDERID_ProgramFilesX64\Cedarflake Ame\Application` bundle. The source bundle is bounded to 4,096
entries and 2 GiB and contains no reparse point. Every installed entry is reopened to prove its
final physical path and receives an exact protected ACL: SYSTEM and Administrators have full
control and BUILTIN Users have read/execute only. The installed client and broker are then
bound together by the SCM-protected `AMEJBID2` manifest using protocol/max-frame, canonical client
path, binary SHA-256 values and the configured optional publisher/certificate policy validated by
the installer.

Hosted Daily checks reuse the eight isolated component jobs at the candidate's manifest-bound source
commit. The release aggregate requires these jobs and the selected candidate verifier to succeed
before packaging. This preserves complete Daily coverage without placing the serial workstation gate
inside a 90-minute candidate-verification job; runtime and synthetic-performance checks stay in that
independent verifier. The local full-release entrypoint remains serial.

Hosted release verification consumes one immutable manifest-bound Application bundle without
rebuilding or copying over it after admission. The main dispatch commit is the trusted
workflow commit. Before checkout, the workflow accepts only a strict `v`-prefixed SemVer tag; checkout
names the complete `refs/tags/<tag>` ref and proves `HEAD` equals `tag^{commit}` and that the source
commit is an ancestor of the trusted workflow commit. The application and broker are built without
signing credentials and transferred as a bounded, exact path/length/SHA-256 manifest artifact whose
`sourceCommit` records that immutable SHA.

The default unsigned channel independently downloads and validates that artifact, including exact
manifest coverage, x64 executable identity, broker protocol, unsigned application/broker and
packaged desktop behavior. Only the optional signed channel invokes the
`windows-production-signing` GitHub Environment. Its signer does not check out or execute repository
content; it admits an ephemeral PFX with exact publisher and code-signing usage, signs only the two
fixed executables, clears secrets, preserves the source identity, and uploads a signed manifest.
The signed verifier independently checks the complete manifest and both signatures before release.

A read-only packaging workflow consumes the verified artifact in the selected channel, binds its
manifest to the same source commit, and produces both ZIP and MSI. The MSI uses pinned WiX tooling
and embeds the independently verified classification models. It exports both names and SHA-256
values with one immutable two-file artifact. The only `contents: write` job checks out no repository
content and executes no repository script. Immediately before publication it resolves lightweight or annotated tag objects through the
GitHub API and requires the resulting commit to equal the manifest-bound source commit, then
rechecks both package hashes. A new release remains a draft until both uploaded assets have matching
API digests. Upload failure cannot expose a new release with only one package. For an existing
release it lists every asset page through the authenticated GitHub API. A same-name asset must be
uploaded and have an API-provided SHA-256 digest matching the corresponding verified package; that
identity is retained without another upload. A different, missing, or malformed digest fails closed.
Only a missing attachment may be uploaded, without overwrite permission.
A release lookup's explicit HTTP 404 permits creation; transport errors and other unsuccessful
release or asset lookups never count as absence. The publisher rechecks the tag immediately before
mutation; GitHub does not make tag resolution and release-asset mutation one transaction. Post-publication
verification downloads and checks both assets against their API digests and the packager's hashes.
Its pair verifier is read from the immutable workflow commit; application verification remains bound
to the release source. ZIP verification enforces safe paths and bounded expansion, extracts into
fresh repository scratch storage, validates x64 payload and broker protocol, and checks both unsigned executable states
or the selected signatures/publisher before cleanup. The candidate workflow calls this reusable
verifier directly after publication. Externally created `release: published` events use the default
unsigned channel unless the repository Actions variable `AME_WINDOWS_RELEASE_SIGNING_MODE` explicitly
selects `signed`, which also requires `AME_WINDOWS_EXPECTED_PUBLISHER`. Manual dispatch can select
either channel. A structural ZIP check alone is not release identity evidence.

## Deferred decisions

Later release-maturity work outside ADR 0024 owns the following work:

- the general installed-product and update experience beyond ADR 0024's required broker lifecycle;
- stable package and upgrade identities not already required by that broker installer;
- manual and in-application application-update workflows;
- database migration and compatibility policy;
- application and database rollback behavior;
- optional code-signing infrastructure and credential custody, provenance, checksums,
  and broader supply-chain hardening.

The future uninstaller must remove Ame-owned cache, thumbnail, and temporary files. It must never
delete or alter original media. Durable catalogs, user decisions, and operation history require a
separate explicit retention decision before installer implementation; this ADR does not classify
them as disposable cache.

## Consequences and risks

- Users extract one directory and run `cedarflake_ame.exe` from that directory.
- Moving the directory is supported; removing it does not clean application data.
- The ZIP is larger than the executable because it intentionally includes all runtime dependencies.
- Application and broker executables are unsigned by default; optional signed releases verify
  the exact configured publisher. The portable artifact has no installer registration, automatic
  update, or rollback.
- The portable payload may contain the broker executable but never grants service lifecycle
  authority or protected client identity, so the portable application remains `LiveOnly`.
- GitHub Release publication has write permission only after the read-only candidate and packaging
  gates succeed, and that job has no repository checkout.
- A workflow rerun retains existing attachments only after proving the same immutable source
  commit and both package identities; it never replaces an attachment.

## Validation evidence

- `tool/release_package_portable_windows.ps1` copies and archives the complete Release directory.
- `tool/release_verify_portable_archive.ps1` validates identity, structure, and required runtime
  entries without extracting the archive.
- `tool/release_verify_windows_candidate.ps1` checks the exact candidate manifest, file
  lengths/hashes, required runtime, x64 executables and broker protocol; the selected channel
  requires unsigned executables or valid signatures from the expected publisher.
- `tool/release_verify_portable_unsigned.ps1` safely extracts and verifies unsigned application
  and broker identity, while `tool/release_verify_portable_signatures.ps1` retains the corresponding
  publisher and Authenticode checks for an explicitly signed release.
- `tool/release_test_portable_archive.ps1` proves a valid fixture passes and incomplete or
  traversal-bearing archives fail, including a broker-missing archive.
- `tool/release_test_windows_publication.ps1` exercises the workflow-owned publication transaction
  with missing, matching, conflicting, digest-less, paginated, failed-lookup and partial-upload fixtures.
  It performs no remote requests and supplies no executable artifact to the write-permission job.
- `tool/release_test_journal_broker_installer_guardrails.ps1` proves the Windows/x64, publisher,
  service account, service SID, privilege, DACL, fixed-destination, interrupted-transaction,
  rollback isolation, no-follow cleanup, source/catalog isolation, and no-journal-mutation
  constraints without changing SCM state.
- `tool/release_install_journal_broker.ps1`, `release_repair_journal_broker.ps1`,
  `release_upgrade_journal_broker.ps1`, `release_stop_journal_broker.ps1`, and
  `release_uninstall_journal_broker.ps1` own the explicit elevated lifecycle. Their existence does
  not satisfy the pending signed-candidate or real-SCM acceptance gate.
- `.github/workflows/release_candidate_windows.yml` gates publication behind release verification.
- `.github/workflows/release_verify_published.yml` downloads and validates the published attachment.

## References

- [GitHub: Get a release by tag name](https://docs.github.com/en/rest/releases/releases#get-a-release-by-tag-name)
- [GitHub: List release assets and SHA-256 digest](https://docs.github.com/en/rest/releases/assets#list-release-assets)
- [Microsoft: KNOWNFOLDERID](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid)
- [Microsoft: SHGetKnownFolderPath](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath)
- [Microsoft: GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew)
- [Microsoft: File security and access rights](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights)
- [Microsoft: File access rights constants](https://learn.microsoft.com/en-us/windows/win32/fileio/file-access-rights-constants)
- [Microsoft: ConvertStringSecurityDescriptorToSecurityDescriptorW](https://learn.microsoft.com/en-us/windows/win32/api/sddl/nf-sddl-convertstringsecuritydescriptortosecuritydescriptorw)
- [Microsoft: CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw)

## Replacement and rollback strategy

The ZIP packager is independent of installer technology. ADR 0027's MSI and native bootstrapper
consume the same verified Release directory without changing the application core or portable
artifact contract.
If automated publication is unsafe, disable the publishing job while retaining candidate and local
archive verification; do not publish an unverified artifact manually as a substitute.
