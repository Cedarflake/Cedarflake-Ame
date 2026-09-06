# ADR 0015: Versioned Windows x64 portable distribution

- Status: Accepted
- Date: 2026-08-11
- Last amended: 2026-09-06

## Context

Cedarflake Ame needs a distributable Windows build before the later installer, update, rollback,
signing, and supply-chain work is ready. Flutter's Windows Release output is a directory containing
the executable, Flutter and plugin DLLs, the Rust bridge DLL, and runtime data. Shipping only the
executable would produce an unusable application.

The application currently has one primary user and one measured platform. Production signing does
require a human-reviewed GitHub Environment whose deployment policy admits only protected `main`;
the signing credential must never become available to a tag-controlled or arbitrary-ref workflow
definition. No artifact may be published until the complete release-candidate gate has passed.
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

The portable archive contains the complete repository-built Flutter Release directory. Archive
verification rejects an unexpected filename, entries outside the single archive root, traversal or
absolute paths, duplicate paths, missing executable or core runtime DLLs, missing Rust bridge DLL,
missing ICU or application data, and missing Flutter assets.

The release-candidate workflow is manually dispatched only from protected `main`; a tag push does
not run a credential-bearing workflow. The `windows-production-signing` Environment must require a
reviewer and must restrict deployment branches and tags to protected `main` only. Its PFX and
password are Environment secrets, never repository or organization secrets. The repository must
also enforce an immutable release-tag ruleset that prevents update or deletion of admitted
`v<semantic-version>` tags. Only after the read-only candidate and package gates succeed may a
separate no-checkout job with repository-content write permission publish or update the
corresponding GitHub Release. The published-release workflow downloads the exact attachment and
verifies it independently.

### ADR 0024 amendment: service-enabled distribution

The portable ZIP remains a valid Windows x64 artifact, but it cannot establish the protected
installed-client identity or install/update the constrained journal broker required for complete
closed-process continuity. Under ADR 0024 it therefore always exposes only an explicit `LiveOnly`
capability, including on a machine where a broker is installed. It must not claim the complete
synchronization contract or use a signed executable in a user-writable directory as code identity.

ADR 0024 moves the minimum signed installer work required to install, repair, upgrade, and remove
that broker into R2c. This is a narrow amendment to the stage boundary below, not a replacement of
the portable identity, archive layout, candidate gate, or publication rules in this ADR. General
automatic update, downgrade, catalog rollback, and broader supply-chain maturity remain deferred.

The verified Release directory and portable archive include
`cedarflake_ame_journal_broker.exe`, but extracting or running the portable application never
installs, starts, repairs, upgrades, or removes the service. The desktop negotiates the installed
service protocol only from the protected Program Files Application identity. The portable desktop
remains explicitly `LiveOnly`. The bundled executable is installer input, not authority to mutate
SCM state.

Broker lifecycle entrypoints are fixed to Windows 11 client x64 and the Ame-owned
`FOLDERID_ProgramFilesX64\Cedarflake Ame\Journal Broker` directory. The lifecycle runs only in a
native 64-bit process and resolves that known folder through `SHGetKnownFolderPath`; the
`ProgramFiles` environment value and a WOW64 redirected path have no authority. Install, repair,
and upgrade reject a non-x64 PE, an invalid or missing Authenticode signature, an empty expected
publisher, or a valid
signature whose exact signer subject differs from that expected publisher. Upgrade stages the
validated replacement on the destination volume only after the installed binary's exact ACL,
publisher, executable `--protocol-info`, hash, signer-certificate hash, client identity, and
SCM-protected manifest all agree. The transaction stops the service with a bounded wait, records
only paths and service objects created or moved by this invocation in a protected, write-through
persistent marker, renames the old binary to a
rollback name when one exists, and revalidates service and binary ACL state. Every rollback action
runs independently in reverse order and failures are aggregated, including failures after the new
service starts or while deleting the owned backup. A repair with a missing binary creates no fake
backup ownership and never deletes a pre-existing path. Uninstall first performs the same available
binary identity checks, removes only the fixed service through a bounded stop/delete/disappearance
wait, then removes the fixed binary and an empty broker directory; it has no catalog, cache,
source-root, or USN-journal operation.

Every lifecycle entry first validates and recovers a prior marker. It refuses a live owner identified
by both PID and process start time, treats PID reuse or a dead owner as interrupted work, and executes
every recovery action independently. The marker preserves whether the Ame parent existed and its
exact descriptor so rollback restores that state or removes a parent created by this invocation.
Owned-tree deletion uses a bounded no-follow walker and rejects any descendant reparse point. A
markerless broker directory may be adopted only when it is empty, has the exact protected descriptor,
and neither the service nor binary exists; a nonempty or differently protected orphan fails closed.

These repository scripts are the lifecycle foundation consumed by a future signed installer
container; they are not themselves evidence that a production signing identity or installer UI has
been provisioned. Release-candidate verification requires an already signed broker path and the
exact expected publisher. The repository currently owns neither signing credentials nor a signed
candidate, so an unsigned candidate fails closed and cannot be published.

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
control and BUILTIN Users have read/execute only. The signed installed client and broker are then
bound together by the SCM-protected `AMEJBID2` manifest using protocol/max-frame, canonical client
path, both binary SHA-256 values, both signer-certificate SHA-256 values, and the exact publisher
validated by the installer.

Hosted release verification consumes one immutable signed Application bundle rather than rebuilding
or copying over it after admission. The protected-main dispatch commit is the trusted workflow
commit. Before checkout, the workflow accepts only a strict `v`-prefixed SemVer tag; checkout names
the complete `refs/tags/<tag>` ref and then proves `HEAD` equals `tag^{commit}` and that this source
commit is an ancestor of the trusted workflow commit. The application and broker are built without
signing credentials on one runner and transferred as a bounded, exact path/length/SHA-256 manifest
artifact whose `sourceCommit` records that immutable SHA. A second runner, bound to the
`windows-production-signing` GitHub Environment, does not check out or execute any repository
content. It admits one ephemeral PFX certificate with the exact publisher and code-signing usage,
signs only the two fixed executable paths, clears the secret material, preserves the tag and source
commit in a new immutable manifest artifact, and exposes neither secret outside that step. A third
credential-free runner checks out the same source commit, proves the current tag and `HEAD` still
resolve to it, revalidates the complete signed manifest, Authenticode publisher, x64 machine, and
broker protocol, and gates publication.

A read-only packaging job consumes that verified signer artifact, binds its signed manifest to the
same source commit, packages it, and exports the ZIP name and SHA-256 through an immutable workflow
artifact. The only `contents: write` job checks out no repository content and executes no repository
script. Immediately before publication it resolves lightweight or annotated tag objects through the
GitHub API and requires the resulting commit to equal the manifest-bound source commit, then
rechecks the ZIP hash. For an existing release it lists every asset page through the authenticated
GitHub API. A same-name asset must be uploaded and have an API-provided SHA-256 digest matching the
verified ZIP; that identity is retained without another upload. A different, missing, or malformed
digest fails closed. Only a missing attachment may be uploaded, without overwrite permission.
A release lookup's explicit HTTP 404 permits creation; transport errors and other unsuccessful
release or asset lookups never count as absence. The immutable tag ruleset is a required enforcement
layer because a workflow
cannot make tag resolution and release-asset mutation one GitHub transaction. Post-publication
verification first enforces safe ZIP paths and bounded expansion, then extracts to fresh repository
scratch storage, revalidates both embedded signatures and the broker protocol, and removes the
scratch directory. The candidate workflow calls that reusable verifier directly after publication;
the `release: published` entry point covers externally created releases and requires the repository
Actions variable `AME_WINDOWS_EXPECTED_PUBLISHER`, while `workflow_dispatch` remains the recovery
entry point. Repository protection must require the exact `Windows Gate / Windows Gate` status
context. A structural ZIP check alone is not release identity evidence.

## Deferred decisions

Later release-maturity work outside ADR 0024 owns the following work:

- the general installed-product and update experience beyond ADR 0024's required broker lifecycle;
- stable package and upgrade identities not already required by that broker installer;
- manual and in-application application-update workflows;
- database migration and compatibility policy;
- application and database rollback behavior;
- signing infrastructure and credential custody beyond the required broker Authenticode gate,
  provenance, checksums, and broader supply-chain hardening.

The future uninstaller must remove Ame-owned cache, thumbnail, and temporary files. It must never
delete or alter original media. Durable catalogs, user decisions, and operation history require a
separate explicit retention decision before installer implementation; this ADR does not classify
them as disposable cache.

## Consequences and risks

- Users extract one directory and run `cedarflake_ame.exe` from that directory.
- Moving the directory is supported; removing it does not clean application data.
- The ZIP is larger than the executable because it intentionally includes all runtime dependencies.
- The application and broker are Authenticode-signed; the portable artifact still has no installer
  registration, automatic update, or rollback.
- The portable payload may contain the broker executable but never grants service lifecycle
  authority or protected client identity, so the portable application remains `LiveOnly`.
- GitHub Release publication has write permission only after the read-only candidate and packaging
  gates succeed, and that job has no repository checkout.
- A workflow rerun retains an existing same-name attachment only after proving the same immutable
  source commit and ZIP identity; it never replaces an attachment. Tag update and deletion protection
  is a repository-configuration prerequisite.

## Validation evidence

- `tool/release_package_portable_windows.ps1` copies and archives the complete Release directory.
- `tool/release_verify_portable_archive.ps1` validates identity, structure, and required runtime
  entries without extracting the archive.
- `tool/release_verify_portable_signatures.ps1` safely extracts a structurally valid archive into
  fresh bounded scratch storage and revalidates the application and broker signature, publisher,
  x64 machine, and broker protocol before confirmed cleanup.
- `tool/release_test_portable_archive.ps1` proves a valid fixture passes and incomplete or
  traversal-bearing archives fail, including a broker-missing archive.
- `tool/release_test_portable_publication.ps1` exercises the exact workflow-owned publication
  decision with missing, matching, conflicting, digest-less, paginated, and failed-lookup fixtures.
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

The ZIP packager is independent of installer technology. R9 may add an MSI produced from the same
verified Release directory without changing the application core or portable artifact contract.
If automated publication is unsafe, disable the publishing job while retaining candidate and local
archive verification; do not publish an unverified artifact manually as a substitute.
