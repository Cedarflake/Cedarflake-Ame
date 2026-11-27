# ADR 0045: Microsoft Store owns packaged deployment and updates

- Status: Accepted
- Date: 2026-11-27

## Context

Ame's reserved Store product needs a package identity and Windows-managed lifecycle alongside its
existing MSI and portable distributions. AppData virtualization and read-only package installation
paths make direct package replacement inappropriate. The ordinary-token journal adapter is already
the production synchronization path under ADR 0024; the MSI service is not a package prerequisite.

## Decision

Use the Windows SDK's MakeAppx tool to build a full-trust `packagedClassicApp` MSIX from the same
verified release payload boundary used by existing packagers. No community packaging runtime is
added. The package's identity is `Cedarflake.Cedarflake-Ame`, publisher
`CN=4756B427-4333-4200-A66E-9407CC4BB239`, publisher display name `Cedarflake`, and package family
`Cedarflake.Cedarflake-Ame_t341beqjcwv0y`. The reserved Store product is `9NRXBZ70WGVX`.

Request `runFullTrust` for the existing desktop executable and its isolated native workers. Do not
declare packaged services, LocalSystem services, broad filesystem access or AppData virtualization
exemptions.
The application retains ordinary user file permissions and existing source authorization rules.
Use the ordinary-token journal adapter and verify it under package identity. MSI service delivery
remains unchanged.

### Update ownership

Query the current process package family through a narrow Windows adapter before selecting an
update source. An un-packaged process retains ADR 0043's existing engine selection. The exact Ame
family selects a Store-only source; an unknown family or failed identity probe cannot fall through
to the direct updater. The early portable startup hook also skips all packaged execution.

Expose `Unavailable`, `Direct` and `MicrosoftStore` delivery as an Ame-owned typed value. The Store
source never reads direct update results, performs release discovery or supplies a downloadable
package. About opens the fixed `ms-windows-store://pdp/?productid=9NRXBZ70WGVX` destination; direct
channel and automatic-download controls are absent in this mode. Windows owns Store update timing,
download and replacement. Existing direct installation controls retain their behavior.

The identity adapter's unsafe calls borrow initialized output lengths and a bounded UTF-16 buffer
for the duration of `GetCurrentPackageFamilyName`. Inspect data only after successful return and
validate the terminator and reported length. No native pointer escapes. The Store launcher passes
only a fixed NUL-terminated URI and verb to `ShellExecuteW`; it accepts no executable, parameter or
URI supplied by release metadata. Native errors remain structured diagnostics.

### Data lifecycle

MSI, portable and MSIX share a default data root at
`%LOCALAPPDATA%\Packages\Cedarflake.Cedarflake-Ame_t341beqjcwv0y\LocalState\Ame`.
Retain distinct catalog, configuration, preview, model and preference storage beneath that root.
Unpackaged clients use direct filesystem access; packaged clients use the same physical location.
Use this common layout directly. Earlier default directories remain untouched and are not
automatically loaded, moved or imported. This Alpha transition provides no legacy-directory
migration or fallback. Custom catalog and preview locations remain supported; locations outside
the shared default root remain outside uninstall cleanup. Subsequent normal upgrades preserve data
already stored in the unified layout and its configured external locations.
An explicitly authorized, one-time operator migration may be performed outside the product; it
does not add automatic startup migration or a persistent compatibility mechanism.

Package upgrade preserves shared data. MSIX uninstall clears the package-managed shared root,
including data subsequently used by MSI and portable clients. Deleting portable program files
leaves shared data in place. MSI provides an opt-in data-removal checkbox, initially unchecked;
its cleanup must operate only
on the current account's admitted app data and never on original photos or another account.
Repair and major upgrades never inherit this deletion choice. ADR 0027 owns the MSI implementation.
No new backup/restore feature or external default data folder is included.

### Default-data preparation

Resolve Windows LocalAppData with `SHGetKnownFolderPath` and
`KF_FLAG_NO_PACKAGE_REDIRECTION`; environment variables and package virtualization do not select
the physical defaults. The shared root contains `data`, `config`, `cache` and `preferences`.
Existing custom catalog and preview locations remain available. Locations outside the shared
default root stay outside uninstall cleanup.

The directory adapter is a pure path query: it creates no directories and requires no preparation
gate. The existing settings, catalog and preferences owners create their needed directories.
Bootstrap initializes Rust, queries the preferences directory and captures initialization failure.
Retain the debug-only isolated storage override. On bootstrap failure, diagnostics remain in memory
for export and the failure window does not persist placement.

Startup does not inspect earlier settings or catalogs or rewrite stored paths. Reinstallation starts
from the current unified data state and never imports earlier defaults. Selecting a custom catalog
or preview location continues to follow ADR 0005's storage controls and source-overlap protections.

MSI cleanup pins admitted ancestors without delete sharing and rejects reparse points under
ADR 0027. Known-folder allocations are copied into owned paths and freed exactly once. Original media
is outside default-data preparation and uninstall cleanup; file actions retain their separate
explicit authorization and recovery requirements.

### Packaging and signing

Use the exact reserved identity in a generated manifest and existing Ame artwork for package logos.
Include the same pinned classification models and license notices as the MSI and portable packages;
reuse the existing model admission and preparation owner before packaging the Store archive.
Store upload artifacts are unsigned; Microsoft signs accepted Store packages. Local sideload
verification uses a temporary certificate trusted only in the isolated guest. Private keys and
generated packages never enter Git history. Package version mapping must be monotonic and retain
the Store-required zero fourth component. Map application `major.minor.patch+build` to
`(major + 1).minor.build.0`, retaining the existing monotonic build requirement within a major/minor
line. The offset supports pre-1.0 Ame releases while satisfying the Store's nonzero first component.
Version and architecture admission belong to packaging.

## Consequences and validation

MSIX installation does not need the MSI service's administrative setup. Installed and portable
packages remain independent supported distributions. Store review may require the desktop capability
justification; no submission or approval is implied by local package verification.

Verify engine exclusion at startup and through the application port, package identity and payload
admission, fresh install, packaged native workers, ordinary-token offline catch-up, source-safe file
workflows, shared data and single-instance behavior across distributions, retained data on upgrade
and the selected uninstall data policy. Preserve MSI/portable update
regressions. Record exact package/runtime evidence in the scoped acceptance record.

## Alternatives and rollback

Running the direct updater inside WindowsApps conflicts with Windows package ownership. Shipping the
retained service adds unnecessary capabilities to the selected journal path. Separate private data
conflicts with the requested shared library. Exempting existing AppData folders would preserve them
on MSIX uninstall, contrary to the selected removal behavior. A new packaging
framework adds no needed runtime capability beyond the existing verified payload and Windows SDK.

The MSIX packager and Store update source can be retired independently while preserving MSI and
portable distribution. Apply only the selected uninstall data policy; original media is always
outside cleanup. Future package-identity changes require a separately reviewed transition that
preserves existing unified user data.

## References

- [Manual MSIX packaging](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion)
- [Packaged desktop behavior](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes)
- [Package capabilities](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations)
- [Scoped virtualization exclusions](https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization)
- [Store identity](https://learn.microsoft.com/en-us/windows/apps/publish/view-app-identity-details)
- [Store readiness](https://learn.microsoft.com/en-us/windows/apps/publish/store-readiness-checklist)
