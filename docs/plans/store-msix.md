# Microsoft Store MSIX distribution

The [roadmap](../roadmap.md#microsoft-store-distribution) owns activation and disposition.
[ADR 0045](../architecture/0045-microsoft-store-distribution.md) owns package identity, runtime
selection, storage and update behavior.

## Outcome and scope

Produce an MSIX package for the reserved Cedarflake-Ame Store product and exercise its desktop
workflows in an isolated Windows environment. Preserve MSI and portable delivery and the separate
R10 Beta candidate work. Store submission, release publication and merging require separate authority.

The Store build uses Windows-managed updates and ordinary-user journal synchronization. It does
not install the retained MSI service. MSI, portable and MSIX share per-user catalog, settings and
cache storage. MSIX uninstall clears the shared application data; MSI uninstall gains an explicit,
initially unchecked data-removal choice for the current account. Deleting portable program files
leaves the data in place. No new backup/restore feature is included. Existing organization export
remains. Use the unified directories directly without importing earlier defaults; old directories
remain untouched. Custom storage selection remains available. Normal future upgrades preserve
unified data, and original photos are outside every uninstall cleanup scope.

## Implementation order

1. Identify packaged execution before selecting update engines. Expose Store-managed delivery through
   the typed bridge and use the existing About setting to open the reserved Store product.
2. Package admitted release payloads with the Windows SDK, the exact Partner Center identity,
   existing brand assets and a monotonic Store version. Keep package construction separate from
   signing and submission; local test certificates remain outside tracked files.
3. Use common default storage beneath the reserved package family's LocalState folder. Do not read,
   move or import earlier default directories. Preserve support for explicitly selected external
   locations.
   Verify clean installation and launch, packaged data locations, native worker startup, a disposable
   image library, offline change catch-up, file actions, cross-distribution sharing, upgrade
   preservation and uninstall behavior
   in an isolated Windows guest. Exercise the Store action and all supported UI languages.
4. Implement the MSI uninstall data choice with source-safe current-user cleanup and verify both
   retain and remove paths, including cancellation, repair and upgrade exclusion.
5. Add boundary regressions for delivery selection and package admission, run the applicable hosted
   gates, and commit/push implementation and evidence together.

## Store-signed GitHub Release asset

The selected follow-up distribution flow publishes the MSIX asset only after the Store has signed
and published that version. Keep MSI and portable publication on their existing path; attach the
retrieved Store package to the same version's GitHub Release when it becomes available.

1. Record the admitted source commit, application build, mapped Store package version and target
   Release tag before submitting that candidate. Complete the first Store publication in Partner
   Center; configure subsequent submission credentials through the existing protected CI boundary.
2. Wait for successful Store publication and availability of the matching downloadable package.
   Retrieve it through the supported WinGet Store download path with offline-download eligibility.
3. Validate the Windows package signature and integrity, reserved package identity, architecture,
   exact Store version and application build against that candidate. Downloading the latest package
   alone is insufficient to select an older Release. Keep the signed package unchanged and record
   its own digest; signing changes the archive digest from the unsigned input.
4. Verify the first retrieved Ame package's installation and applicable offline-license behavior in
   an isolated environment, then attach the validated signed MSIX or bundle to its matching Release.
   Existing same-name assets must match the admitted digest; retries must not replace another build.

The implementation uses the [Store release procedure](../acceptance/quality/store-release.md).
Submission runs in hosted CI; signed-package retrieval runs in an authenticated Windows user
session because WinGet requires WAM authentication even with `--skip-license`. No dedicated runner
or account is provisioned by this implementation. First publication, offline-download eligibility,
submission credentials and actual signed-package deployment remain external prerequisites.
Submission and external asset upload remain explicit release actions. The
[WinGet download contract](https://learn.microsoft.com/en-us/windows/package-manager/winget/download)
owns package and license retrieval; downloaded packages retain their actual Store format.

## Verification boundary

Use disposable guest-owned media and app data. Do not install the test certificate or package into
the user's desktop account. Keep temporary signing material in ignored storage and remove trust
from the guest after testing. Reuse existing source-safe fixtures and release payload validation.

Prove that Store execution cannot run the direct updater, download GitHub update packages, consume
direct-installation update results or launch a replacement helper. Package probes must execute with
the package identity, not merely beside an unpacked manifest. Record Store activation separately
from a real Store-delivered update, which requires publication.

Keep the existing Flutter graphics qualification limit visible. MSIX admission does not replace
R10-05's renderer or release acceptance, and an unsigned upload artifact is not Store certification.

## Current implementation boundary

Store update selection and package construction are implemented. Isolated package deployment has
verified the shared-directory lifecycle for both administrator and ordinary-user execution; the
ordinary-user result is the applicable permission evidence. The direct-directory client passed
startup, custom-catalog selection, fixture import and uninstall with external files retained.
The MSI lifecycle matrix and Setup's visible data-removal choice have passed, including repair,
major upgrade, cancellation, alternate-account rejection and external-file retention. Hosted checks
are running for the implementation commit. The
[MSIX acceptance record](../acceptance/store-msix.md) owns measurements and probe limitations.
