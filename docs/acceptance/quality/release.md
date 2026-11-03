# Release gate

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Release gate

The standard release gate runs daily verification, Windows Release packaging, the packaged bridge
smoke test, packaged same-user process ownership and replacement-start verification, and the
synthetic performance gate:

```powershell
./tool/release_verify_candidate.ps1 `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>" `
  -SignedBrokerBinaryPath "<signed x64 cedarflake_ame_journal_broker.exe>" `
  -SignedApplicationBundlePath "<immutable signed Windows Release directory>"
```

After that gate has produced a complete Windows Release directory, create the portable artifact:

```powershell
./tool/release_package_portable_windows.ps1 `
  -Tag "v0.1.0" `
  -ReleaseRoot "<immutable signed Windows Release directory>" `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>"
```

The artifact is written to `build/release-artifacts/` with one `Cedarflake-Ame/` archive root. A
manual release request dispatched from protected `main` performs this step only after its candidate
gate passes; a tag push never obtains signing credentials. The accepted identity, x64-only support
boundary, and deferred installer decisions are recorded in
[ADR 0015](../../architecture/0015-windows-release-distribution.md).

The portable ZIP contains the broker executable as installer-compatible payload. Extraction does
not install, repair, update, or remove SCM state. Debug, Release and portable clients can use an
already installed compatible broker through the same caller/root access policy; signing and a fixed
client path are not runtime prerequisites. The hosted signed candidate flow builds once without credentials, transfers a manifest-bound
unsigned artifact to the protected Environment signer, and transfers the resulting immutable signed
artifact to a separate credential-free verifier and read-only packager. The signer checks out no
repository content and signs only the application and broker. The final write-capable publisher
checks out no repository content, executes no repository script, and consumes only the manifest-
bound ZIP after a current tag-to-source comparison and hash recheck. A successful candidate release
invokes the reusable published-verification workflow directly with the admitted tag and publisher;
the `release: published` entry point independently verifies releases created outside that candidate
workflow, and `workflow_dispatch` is the recovery entry point. Published verification downloads the
ZIP, validates safe archive structure, extracts into fresh bounded scratch storage, and revalidates
both signatures, the exact publisher, x64 machine, and broker protocol before cleanup. The external
event entry point requires the repository Actions variable `AME_WINDOWS_EXPECTED_PUBLISHER`. The
repository does not contain production signing credentials or a signed candidate. Until the named
Environment secrets, protected-main deployment restriction, required reviewer, immutable version-
tag ruleset, exact `Windows Gate / Windows Gate` required check, and
`AME_WINDOWS_EXPECTED_PUBLISHER` variable are provisioned, candidate verification and external
publication verification fail closed by design.

When the retained real-library catalog is applicable to the release, append its prepared read-only
validation explicitly:

```powershell
./tool/release_verify_candidate.ps1 `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>" `
  -SignedBrokerBinaryPath "<signed x64 cedarflake_ame_journal_broker.exe>" `
  -SignedApplicationBundlePath "<immutable signed Windows Release directory>" `
  -IncludeRealLibrary `
  -AcceptanceStorageRoot "<retained acceptance storage>" `
  -RootA "<first approved root>" `
  -RootB "<second approved root>" `
  -AuthorizationToken "<current authorization token>"
```

This option validates the retained catalog; it does not silently start another full source scan.
