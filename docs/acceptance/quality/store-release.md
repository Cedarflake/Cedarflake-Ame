# Microsoft Store release procedure

[ADR 0045](../../architecture/0045-microsoft-store-distribution.md) owns identity, signing and
deployment. The [roadmap](../../roadmap.md#microsoft-store-distribution) owns activation and remaining
verification. MSI and portable release publication continue through the existing release gate.

## Prepare and publish the first Store version

After an unsigned main `Release / Candidate Windows` run succeeds, dispatch
`release_store_windows.yml` from `main` with `operation: prepare`, the exact release tag, that run ID
and its immutable `Cedarflake-Ame-<tag>-unsigned-<run>-<attempt>` artifact ID. The workflow verifies
producer provenance and payload before generating the unsigned MSIX. Keep its `store-msix` artifact
and `msix-package.json`; submission and retrieval use those same immutable bytes and receipt.

Complete the first publication in Partner Center, including listings and age ratings. Upload this
MSIX and add `Ame package SHA-256: <sha256 from msix-package.json>` as a separate line in the
certification notes. Retain the submission ID from Partner Center. This binds manual first
publication to the same retrieval checks as subsequent automated submissions. Product-page
experiments are optional listing A/B tests, not the initial submission.

## Subsequent submissions

Associate an Entra tenant and application with Partner Center and grant the required submission
role. Configure these secrets in the existing protected `windows-production-signing` environment:

- `AME_STORE_TENANT_ID`
- `AME_STORE_CLIENT_ID`
- `AME_STORE_CLIENT_SECRET`

The Store identity's MSA application ID is not a substitute for this Entra application. Credentials
are supplied through the environment; never place them in workflow inputs, receipts or source files.

Dispatch `Release / Microsoft Store` with `operation: submit` and the successful preparation run
and `store-msix` artifact IDs. The operation creates a draft, uploads the package and commits it for
publication after certification. It preserves the existing listing and pricing. An existing
unrelated draft stops the operation. Do not edit an API-created draft through Partner Center while
the API is managing it; Microsoft's API requires subsequent updates through the API.

The `store-submission` artifact records the exact ID even if upload or commit fails. Resume the same
candidate with that `submission_id`; an already committed submission is queried without another
upload or commit. If creation succeeds but the package-binding update fails, inspect the recorded
draft before recovery; it is not automatically deleted or adopted. Use `operation: status` with the
same package artifact and submission ID for a read-only status query. `PendingPublication` is not
`Published`; certification errors remain visible in Partner Center.

## Retrieve the Store-signed package

Use PowerShell 7, GitHub CLI, WinGet and Windows SDK 10.0.26100 in a Windows user session with an
eligible Entra account already authenticated through Windows Web Account Manager. Store submission
credentials and WinGet authentication are separate. `--skip-license` skips license retrieval only;
an empty hosted Windows runner cannot use the submission secret as a WinGet login. This repository
does not register a self-hosted runner or silently open an interactive login.

Configure the same three Store API credentials in the process environment and authenticate `gh`
for repository artifact access. Run with the preparation run, its artifact and exact submission ID:

```powershell
./tool/release_retrieve_store.ps1 `
  -PackageRun '<preparation run ID>' -PackageArtifact '<store-msix artifact ID>' `
  -SubmissionId '<Store submission ID>' -OutputDirectory build/store-signed/first `
  -WaitMinutes 30
```

The command waits at most the requested interval (0–45 minutes). Pending publication saves status
without downloading or uploading. Use a new output directory on retry. A published submission is
downloaded through WinGet with silent authentication, without dependencies or an offline license.
The downloaded version must match the candidate exactly; a previous CDN version or a newer Store
version stops publication. WinGet does not guarantee historical Store version selection.

The output contains the unchanged signed package and a digest receipt. Before publishing the first
package, verify install, launch and applicable offline-license behavior in an isolated guest,
including any Windows framework dependencies. A valid signature alone does not establish that a
license-free offline installation is supported for this product.

After that acceptance, repeat with a new output directory and `-Publish` to download, validate and
append the package to the existing matching GitHub Release. The command checks the tag's source
commit and the remote asset digest. It never creates a release, changes a tag, replaces an asset
or re-signs the package. A repeated same-digest upload is a no-op. MSI and portable assets remain
untouched. Keep the original preparation artifact until the Store round trip is complete; hosted
retention is 90 days.

## Evidence boundary

Local contract checks exercise actual package archives and receipt parsing, trusted producer
admission, wrong-version/build rejection, exact submission identity, preservation of unrelated
listing/architecture data, foreign-draft refusal, retry without another commit and same-name asset
conflict handling. Submission entrypoint checks use a controlled HTTP transport; they do not contact
Partner Center. `release_test_store.ps1` belongs to both the release-tooling and Static gates.

No Store submission, WAM download or signed Ame deployment has yet been exercised. Complete those
checks after first publication; do not report a fixture pass as Store certification or deployment.

References: [submission API](https://learn.microsoft.com/en-us/windows/uwp/monetize/manage-app-submissions),
[submission prerequisites](https://learn.microsoft.com/en-us/windows/uwp/monetize/create-and-manage-submissions-using-windows-store-services),
[WinGet download](https://learn.microsoft.com/en-us/windows/package-manager/winget/download),
[WAM authentication implementation](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Authentication/WebAccountManagerAuthenticator.cpp).
