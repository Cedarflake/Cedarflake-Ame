# ADR 0027: Version the application and broker through Windows Installer

- Status: Accepted
- Date: 2026-10-12
- Last amended: 2026-10-13

## Context and decision drivers

The desktop application and existing journal broker form one installed product. Distributing
PowerShell installation commands does not give ordinary users a maintained upgrade, repair and
uninstall workflow. The installer must preserve the protected broker boundary, support unsigned
local builds, retain user data and restore the preceding installation if an upgrade fails.

## Decision

Use an x64, per-machine MSI built with WiX Toolset 7.0.0. Windows Installer owns file components,
product registration, service installation/configuration/control and transactional rollback.
Distribute it in one native WiX Burn executable using `WixInternalUIBootstrapperApplication`.
The standard bootstrapper detects, plans and applies the per-machine chain before displaying the
MSI's internal interface, requesting elevation at the start. Burn owns package caching and bundle
maintenance; the MSI remains the sole owner of installed files and service registration.
The installed product never invokes PowerShell, `sc.exe`, a downloaded helper or an elevated copy
of the desktop application. Native actions validate protected destinations and existing objects,
then own one bounded service-start/readiness operation and its paired rollback stop. They do not
implement a second installation transaction engine.

The upgrade identity is `C284E6B0-1484-4926-A94B-C3EC3840DBBD`. Release MSI versions are exactly
the three numeric components shared by Cargo, Flutter and the release tag. Reject prerelease
suffixes, a fourth numeric component and values outside MSI's 255/255/65535 limits rather than
silently collapsing distinct versions. Increment that version for each distributed change.
The product code is stable for an exact version and changes for an upgrade; component identities
are stable for their installed paths. The package embeds its complete bounded payload.

Schedule major-upgrade removal after `InstallInitialize`, inside the rollback transaction.
Reject downgrades and disabled rollback. Repair reinstalls owned components and revalidates
service configuration and protected ACLs. The package owns only its installed files, service,
product registration and shortcut. It never owns source directories, catalogs, previews, settings,
user decisions or operation history. Removal of unknown files or recursive deletion of the
installation directory is not part of uninstall.

Keep the broker beneath native `FOLDERID_ProgramFilesX64` in fixed `Cedarflake Ame/Journal Broker`
directories. The application defaults to the sibling `Application` directory and may use another
local protected installation directory selected through a native MSI path field and folder browser.
Reject volume roots, overlap with the protected broker, redirected/reparse destinations, untrusted parent
ownership and writable ACLs before mutation. A new destination must be empty; an existing nonempty
destination must belong to the registered application component. Preserve ADR 0024's
LocalSystem own-process demand-start service, restricted SID, exact required privileges and
ordinary-client access. Native MSI ACL tables contain complete descriptors; no installer property
substitution supplies principals or access masks. The broker validates its actual process token
before creating any production pipe, including a start during incomplete MSI service configuration.

Before changing an existing service, bind its fixed path, account, start/type, restricted SID,
privileges and description to the registered product's protected cached MSI and registered broker
component path. An existing broker must match that cached identity. A missing broker remains
repairable through this independently registered ownership; a missing service may be recreated.
After file installation and service configuration, hash the actual protected broker through one
read-shared handle (at most 128 MiB) and require the incoming package identity before starting it.
A new SCM description with old, locked or restart-delayed bytes cannot establish successful upgrade.

WiX warning WIX1149 applies broadly to `ServiceConfig`; its suggested Util replacement configures
failure actions and cannot configure the required SID/privilege fields. The package admits only
native `MsiServiceConfig` types 5 and 6 and no failure-action table. The builder narrowly suppresses
WIX1149 while retaining all other warnings as errors. Mandatory deferred SCM readback verifies the
exact restricted SID and privilege allowlist before service start; any mismatch aborts the MSI
transaction. Actual install/repair/upgrade/rollback fixtures remain necessary for this native path.

Do not use the standard `StartServices` retry loop for readiness. The native operation makes one
SCM start request, reads terminal failures immediately and checks MSI cancellation every 100 ms
during a ten-second readiness window. The synchronous SCM call itself remains subject to Windows
service-manager scheduling. A rollback action stops only the matching protected incoming broker
before MSI restores the previous components. MSI retains stop/removal and registration rollback.
Use `MSIFASTINSTALL=1` and Burn `DisableSystemRestore=yes` to omit this package's restore-point
creation; retain MSI and bundle transactional rollback, file costing and normal progress messages.
These properties do not change machine-wide restore-point or security settings.

The portable ZIP remains supported under ADR 0015 and does not install a service. Existing script
entrypoints remain historical diagnostic/recovery tools, not the user-facing installer or an MSI
custom action. Adoption of a legacy script installation requires a separately proven migration;
an unknown service or unregistered nonempty installation tree is rejected. General automatic
update discovery/download, downgrade and catalog rollback remain outside this decision.

## Dependency, licensing and alternatives

WiX 7.0.0 is pinned to upstream source commit
`b8977d6f88e7b68e000bac226a2814f236770570`. The official `WixToolset.Sdk` NuGet package SHA-256 is
`AF8F72FB2550E9C2CF00B6EAF5E3ED811514AA2B3F344DFA342540AF39676979`.
The matching `WixToolset.UI.wixext` package is pinned to SHA-256
`1716C2CBED4613A7291B5775630CF4D0DCA597568BA1128CEC5FDA1903F05BD1` at the same source commit.
Compose its stock localized dialogs for installation, repair and removal; do not display its
placeholder product license or invent an Ame license agreement. The first interactive installation
page contains a `PathEdit`, a stock folder browser and two native `CheckBox` controls; no feature
tree, welcome page or launch splash precedes these choices. These standard Windows Installer
controls retain native keyboard and accessibility behavior. The stock feature tree hides ordinary
directory and shortcut choices behind component-state menus; the composed options page addresses
that product gap without replacing the install engine or folder picker.

A mandatory core feature owns the application and broker. Independent optional features own
Start Menu and desktop shortcuts; Start Menu is initially selected and desktop is initially
unselected. Initialize the checkboxes from costed feature states after `MigrateFeatureStates`;
publish explicit `AddLocal` or `Remove` decisions before leaving the options page. Returning from
confirmation preserves the selected values, and UI initialization never runs in the execute
sequence. Windows Installer migrates feature choices across upgrades. A product-owned HKLM setting
restores the selected application directory before costing; the cached MSI and registered executable
component independently bind that location for repair and upgrade admission. No UI extension DLL is
installed with the product.
The matching `WixToolset.Bal.wixext` package is pinned to SHA-256
`0F30C29F9E53A6C6ACDEA2CE0D1F0833153883FA3D16ED8DFF75005EDD7FE876` at the same source commit.
Its native internal-UI bootstrapper requires no additional runtime and owns the repair payload cache.
Its .NET Framework tool runs with the existing Windows build toolchain; the application and
installed product have no WiX or .NET SDK runtime dependency.

WiX source uses MS-RL. Its precompiled binaries additionally have the Open Source Maintenance Fee
agreement: revenue-generating use with annual gross revenue of at least USD 10,000 requires the
upstream maintenance arrangement. This personal, non-revenue development workflow does not
authorize a subscription or payment. A future commercial build operator must assess that condition
or build WiX from source under MS-RL; the package builder requires explicit binary-EULA acceptance
instead of silently accepting it. These build-tool terms do not add an Ame end-user license or
require users of the generated MSI to install or subscribe to WiX.

MSIX would add a signing/deployment prerequisite to the present unsigned acceptance path. NSIS and
Inno Setup would require more application-owned service and failed-upgrade restoration logic.
Direct MSI database generation would duplicate mature compiler and validation capabilities.
Wrapping the former scripts in an executable would preserve their separate transaction owner and
does not satisfy the installed-product lifecycle.

## Validation and consequences

Inspect actual MSI tables and reject unsafe actions, directory changes, identities, versions and
sequence drift. Exercise a fresh install, two different product versions, repair, uninstall and
injected upgrade failure after old-product removal and after service configuration. Compare exact
files, service description, process token and ACLs after success and rollback. Verify early service
start fails closed and user data remains untouched. Then verify the ordinary client can use the
installed broker and reopen its retained profile. Compilation alone proves none of these outcomes.

The fixed upgrade identity and component paths become compatibility commitments. An unsigned MSI
can trigger Windows publisher warnings; signing remains a separate distribution qualification.
Stop/restart deadlines and locked files must fail visibly rather than report a completed upgrade.
Replacing WiX later may retain the same MSI identities and standard tables, subject to the same
lifecycle fixtures. Reverting a failed package uses MSI rollback, not source or catalog rollback.

## References

- [WiX releases](https://docs.firegiant.com/wix/whatsnew/releasenotes/)
- [WiX maintenance agreement](https://docs.firegiant.com/wix/osmf/)
- [WiX major-upgrade scheduling](https://docs.firegiant.com/wix/schema/wxs/majorupgrade/)
- [Windows Installer service configuration](https://learn.microsoft.com/en-us/windows/win32/msi/msiserviceconfig-table)
- [Windows Installer exact ACLs](https://learn.microsoft.com/en-us/windows/win32/msi/msilockpermissionsex-table)
- [SCM required-privilege behavior](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/ns-winsvc-service_required_privileges_infow)
- [Named-pipe caller impersonation requirements](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-impersonatenamedpipeclient)
- [WiX internal-UI bootstrapper](https://docs.firegiant.com/wix/schema/bal/wixinternaluibootstrapperapplication/)
- [MSI optional restore-point creation](https://learn.microsoft.com/en-us/windows/win32/msi/msifastinstall)
- [Burn chain rollback and restore-point policy](https://docs.firegiant.com/wix/schema/wxs/chain/)
- [Windows Installer native checkbox behavior](https://learn.microsoft.com/en-us/windows/win32/msi/checkbox-control)
- [Windows Installer path control](https://learn.microsoft.com/en-us/windows/win32/msi/pathedit-control)
