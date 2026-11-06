# Release gate

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Release gate

The default GitHub release channel is **unsigned**. Manually run
`.github/workflows/release_candidate_windows.yml` from `main` with an existing strict
`v<semantic-version>` tag, choose `release_signing_mode: unsigned`, and leave
`expected_broker_publisher` empty. No certificate, PFX secret or signing Environment is required.

The reusable gate builds the application and broker without signing, stages one immutable
manifest-bound Windows x64 bundle, and independently verifies its complete file list, lengths,
SHA-256 hashes, source commit, tag ancestry, executable architecture and broker protocol.
The release gate runs Daily's eight existing components against that same source commit, alongside
packaged process ownership/restart, bridge smoke and synthetic performance acceptance. All components
and candidate checks must pass before the candidate proceeds to the read-only portable
packager and the no-checkout, write-authorized GitHub Release publisher.

For independent unsigned-candidate validation:

```powershell
./tool/release_verify_unsigned_candidate.ps1 `
  -CandidateRoot "<downloaded candidate directory containing manifest.json and payload>" `
  -Tag "v0.1.0" `
  -SourceCommit "<exact 40-character release source commit>"
```

For an already verified unsigned Windows Release directory, the portable packager accepts no
publisher parameter:

```powershell
./tool/release_package_portable_windows.ps1 `
  -Tag "v0.1.0" `
  -ReleaseRoot "<verified unsigned Windows Release directory>"
```

An optional `signed` channel remains available. It requires an exact
`expected_broker_publisher`, the protected `windows-production-signing` Environment, an
authorized reviewer and production PFX secrets. Its original Authenticode admission and signed
candidate verification stay mandatory **only for that channel**. The local signed gate remains:

```powershell
./tool/release_verify_candidate.ps1 `
  -ExpectedBrokerPublisher "<exact Authenticode signer subject>" `
  -SignedBrokerBinaryPath "<signed x64 cedarflake_ame_journal_broker.exe>" `
  -SignedApplicationBundlePath "<immutable signed Windows Release directory>"
```

The portable archive is written under `build/release-artifacts/` with one `Cedarflake-Ame/`
root. It includes the broker executable but never installs, repairs, starts or removes an SCM
service on extraction. Unsigned and signed clients can use an already installed compatible broker
under the same caller/root-access policy.

Publication remains manual and main-only. The version tag must resolve to the
manifest-bound source, which must be an ancestor of the workflow definition's commit.
The publisher has no checkout or repository-script execution; it rechecks the ZIP SHA-256 and
refuses conflicting or unprovable existing assets. After publication, the reusable verifier
downloads the exact ZIP, validates archive paths and bounded expansion, then rechecks unsigned
executable identity or, in the signed channel, both signatures and the exact publisher. It also
checks the broker protocol and removes extraction scratch storage.

External `release: published` verification defaults to unsigned and can be configured for signed
artifacts through `AME_WINDOWS_RELEASE_SIGNING_MODE=signed` and
`AME_WINDOWS_EXPECTED_PUBLISHER`. The optional signing Environment and publisher variable do
not block unsigned publication.

When a retained real-library catalog needs release qualification, run the separately authorized
read-only acceptance with its two approved roots and explicit authorization token. It does not
silently scan or modify real media.
